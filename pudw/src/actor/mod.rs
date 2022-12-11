// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// The worker actix actor

use actix::{
    io::{SinkWrite, WriteHandler},
    Actor, ActorContext, AsyncContext, Context, SpawnHandle, StreamHandler, System,
};
use actix_codec::Framed;
use actix_http::ws::{CloseReason, Item};
use awc::{
    error::WsProtocolError,
    ws::{Codec, Frame, Message},
    BoxedSocket,
};
use bincode::{deserialize, serialize};
use bytes::{Bytes, BytesMut};
use futures::stream::SplitSink;
use pudlib::{
    parse_ts_ping, send_ts_ping, Command, Schedule, ServerToWorkerClient,
    WorkerClientToWorkerSession,
};
use std::{
    collections::{BTreeMap, VecDeque},
    time::{Duration, Instant},
};
use tracing::{debug, error, info};
use typed_builder::TypedBuilder;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(TypedBuilder)]
pub(crate) struct Worker {
    // current heartbeat instant
    #[builder(default = Instant::now())]
    hb: Instant,
    // The addr used to send messages back to the worker session
    addr: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    // tx_stdout: UnboundedSender<Stdout>,
    // tx_stderr: UnboundedSender<Stderr>,
    // tx_status: UnboundedSender<Status>,
    // handle to the stdout queue future
    #[builder(default = None)]
    stdout_handle: Option<SpawnHandle>,
    #[builder(default = VecDeque::new())]
    // the stdout queue
    stdout_queue: VecDeque<Vec<u8>>,
    // the last instant a stdout message was received
    #[builder(default = Instant::now())]
    stdout_last: Instant,
    // Is there currently a command running?
    #[builder(default = false)]
    running: bool,
    // continuation bytes
    #[builder(default = BytesMut::new())]
    cont_bytes: BytesMut,
    // The start instant of this session
    #[builder(default = Instant::now())]
    origin: Instant,
    // The commands loaded in this worker
    #[builder(default = BTreeMap::new())]
    commands: BTreeMap<String, Command>,
    // The schedules for the commands
    #[builder(default = Vec::new())]
    schedules: Vec<Schedule>,
}

impl Worker {
    // Heartbeat that sends ping to the server every HEARTBEAT_INTERVAL seconds (5)
    // Also check for activity from the worker in the past CLIENT_TIMEOUT seconds (10)
    #[allow(clippy::unused_self)]
    fn hb(&self, ctx: &mut Context<Self>) {
        debug!("Starting worker session heartbeat");
        let origin_c = self.origin;
        let _ = ctx.run_interval(HEARTBEAT_INTERVAL, move |act, ctx| {
            debug!("checking heartbeat");
            // check heartbeat
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                error!("heartbeat timed out, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }
            debug!("sending heartbeat ping");
            let bytes = send_ts_ping(origin_c);
            if let Err(e) = act
                .addr
                .write(Message::Ping(Bytes::copy_from_slice(&bytes)))
            {
                error!("unable to send ping: {e:?}");
            }
        });
    }

    #[allow(clippy::unused_self)]
    fn queue_monitor(&self, ctx: &mut Context<Self>) {
        let _ = ctx.run_interval(Duration::from_secs(10), move |act, ctx| {
            if !act.running
                && Instant::now().duration_since(act.stdout_last) > Duration::from_secs(30)
            {
                if let Some(sh) = act.stdout_handle {
                    info!("Cancelling stdout_queue");
                    if !ctx.cancel_future(sh) {
                        error!("Unable to kill stdout_queue");
                    }
                    act.stdout_handle = None;
                }
            }
        });
    }

    #[allow(clippy::unused_self)]
    fn stdout_queue(&self, ctx: &mut Context<Self>) -> SpawnHandle {
        ctx.run_interval(Duration::from_millis(1), |act, _ctx| {
            if let Some(front) = act.stdout_queue.pop_front() {
                act.stdout_last = Instant::now();
                if let Err(e) = act.addr.write(Message::Binary(Bytes::from(front))) {
                    error!("Unable to write command: {e:?}");
                }
            }
        })
    }

    #[allow(clippy::unused_self)]
    fn handle_text(&mut self, bytes: &Bytes) {
        debug!("handling text message");
        error!("invalid text received: {}", String::from_utf8_lossy(bytes));
    }

    fn handle_binary(&mut self, ctx: &mut Context<Self>, bytes: &Bytes) {
        if self.stdout_handle.is_none() {
            self.stdout_handle = Some(self.stdout_queue(ctx));
        }
        if let Ok(msg) = deserialize::<ServerToWorkerClient>(bytes) {
            match msg {
                ServerToWorkerClient::Status(status) => info!("Status: {status}"),
                ServerToWorkerClient::Initialize(commands, schedules) => {
                    self.commands = commands;
                    self.schedules = schedules;
                    info!("worker loaded {} commands", self.commands.len());
                    info!("worker loaded {} schedules", self.schedules.len());
                    info!("worker initialization complete");
                    self.start_schedules(ctx);
                }
            }
        }
    }

    fn handle_ping(&mut self, bytes: Bytes) {
        debug!("handling ping message");
        if let Some(dur) = parse_ts_ping(&bytes) {
            debug!("ping duration: {}s", dur.as_secs_f64());
        }
        self.hb = Instant::now();
        if let Err(e) = self.addr.write(Message::Pong(bytes)) {
            error!("unable to send pong: {e:?}");
        }
    }

    fn handle_pong(&mut self, bytes: &Bytes) {
        debug!("handling pong message");
        if let Some(dur) = parse_ts_ping(bytes) {
            debug!("pong duration: {}s", dur.as_secs_f64());
        }
        self.hb = Instant::now();
    }

    #[allow(clippy::unused_self)]
    fn handle_close(&mut self, ctx: &mut Context<Self>, reason: Option<CloseReason>) {
        debug!("handling close message");
        if let Some(reason) = reason {
            info!("close reason: {reason:?}");
        }
        ctx.stop();
    }

    fn handle_continuation(&mut self, ctx: &mut Context<Self>, item: Item) {
        debug!("handling continuation message");
        match item {
            Item::FirstText(_bytes) => error!("unexpected text continuation"),
            Item::FirstBinary(bytes) | Item::Continue(bytes) => {
                self.cont_bytes.extend_from_slice(&bytes);
            }
            Item::Last(bytes) => {
                self.cont_bytes.extend_from_slice(&bytes);
                self.handle_binary(ctx, &bytes);
                self.cont_bytes.clear();
            }
        }
    }

    fn start_schedules(&self, ctx: &mut Context<Self>) {
        for schedule in &self.schedules {
            match schedule {
                Schedule::Monotonic {
                    on_boot_sec,
                    on_unit_active_sec,
                    cmds,
                } => self.launch_monotonic(ctx, *on_boot_sec, *on_unit_active_sec, cmds),
                Schedule::Realtime {
                    on_calendar: _,
                    persistent: _,
                    cmds: _,
                } => {}
            }
        }
    }

    #[allow(clippy::unused_self)]
    fn launch_monotonic(
        &self,
        ctx: &mut Context<Worker>,
        on_boot_sec: Duration,
        on_unit_active_sec: Duration,
        cmds: &[String],
    ) {
        info!(
            "launching monotonic schedule in {}s, re-running every {}",
            on_boot_sec.as_secs_f64(),
            on_unit_active_sec.as_secs_f64(),
        );
        let cmds_c = cmds.to_owned();
        let _ = ctx.run_later(on_boot_sec, move |_, ctx| {
            let inner_cmds = cmds_c.clone();
            let _ = ctx.run_interval(on_unit_active_sec, move |_act, _ctx| {
                for cmd in &inner_cmds {
                    info!("running {cmd}");
                }
            });
            for cmd in &cmds_c {
                info!("running {cmd}");
            }
        });
    }
}

impl Actor for Worker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("worker actor started");
        // start heartbeat otherwise server will disconnect after 10 seconds
        self.hb(ctx);
        // initialze the queue monitor
        self.queue_monitor(ctx);
        // request initialization from the server
        if let Ok(init) = serialize(&WorkerClientToWorkerSession::Initialize) {
            if let Err(_e) = self.addr.write(Message::Binary(Bytes::from(init))) {
                error!("Unable to send initialize message");
            }
        } else {
            error!("Unable to serialize initialize message");
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        info!("worker actor stopped");
        // Stop application on disconnect
        System::current().stop();
    }
}

/// Handle server websocket messages
impl StreamHandler<Result<Frame, WsProtocolError>> for Worker {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Self::Context) {
        if let Ok(message) = msg {
            match message {
                Frame::Binary(bytes) => self.handle_binary(ctx, &bytes),
                Frame::Text(bytes) => self.handle_text(&bytes),
                Frame::Ping(bytes) => self.handle_ping(bytes),
                Frame::Pong(bytes) => self.handle_pong(&bytes),
                Frame::Close(reason) => self.handle_close(ctx, reason),
                Frame::Continuation(item) => self.handle_continuation(ctx, item),
            }
        }
    }

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("worker stream handler started");
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        info!("worker stream handler finished");
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for Worker {}
