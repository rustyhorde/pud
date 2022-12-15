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
    Actor, ActorContext, AsyncContext, Context, Handler, SpawnHandle, StreamHandler, System,
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
    parse_calendar, parse_ts_ping, send_ts_ping, Command, Schedule, ServerToWorkerClient,
    WorkerClientToWorkerSession,
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;
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
    // the sender for stdout from running commands
    tx_stdout: UnboundedSender<WorkerClientToWorkerSession>,
    // the sender for stderr from running commands
    tx_stderr: UnboundedSender<WorkerClientToWorkerSession>,
    // the sender for a command status
    tx_status: UnboundedSender<WorkerClientToWorkerSession>,
    // handle to the stdout queue future
    #[builder(default = Arc::new(Mutex::new(None)))]
    stdout_handle: Arc<Mutex<Option<SpawnHandle>>>,
    #[builder(default = VecDeque::new())]
    // the stdout queue
    stdout_queue: VecDeque<Vec<u8>>,
    // the last instant a queue message was drained
    #[builder(default = Instant::now())]
    stdout_last: Instant,
    // the last instant a stdout message was received
    #[builder(default = AtomicBool::new(false))]
    queue_running: AtomicBool,
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
        let _ = ctx.run_interval(Duration::from_secs(2), move |act, ctx| {
            if !act.stdout_queue.is_empty() && !act.queue_running.load(Ordering::SeqCst) {
                info!("Starting stdout queue drain");
                act.queue_running.store(true, Ordering::SeqCst);
                let handle = Worker::start_stdout_drain(ctx);
                let mut sh_opt = match act.stdout_handle.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                *sh_opt = Some(handle);
            } else if act.stdout_queue.is_empty()
                && act.queue_running.load(Ordering::SeqCst)
                && Instant::now().duration_since(act.stdout_last) > Duration::from_secs(30)
            {
                let mut sh_opt = match act.stdout_handle.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                if let Some(sh) = *sh_opt {
                    info!("Stopping stdout queue drain");
                    if !ctx.cancel_future(sh) {
                        error!("Unable to kill stdout_queue");
                    }
                    act.queue_running.store(false, Ordering::SeqCst);
                    *sh_opt = None;
                }
            }
        });
    }

    fn start_stdout_drain(ctx: &mut Context<Self>) -> SpawnHandle {
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

    fn start_schedules(&mut self, ctx: &mut Context<Self>) {
        let schedules_c = self.schedules.clone();
        for schedule in &schedules_c {
            match schedule {
                Schedule::Monotonic {
                    on_boot_sec,
                    on_unit_active_sec,
                    cmds,
                } => self.launch_monotonic(ctx, *on_boot_sec, *on_unit_active_sec, cmds),
                Schedule::Realtime {
                    on_calendar,
                    persistent,
                    cmds,
                } => self.launch_realtime(ctx, on_calendar, *persistent, cmds),
            }
        }
    }

    fn launch_monotonic(
        &mut self,
        ctx: &mut Context<Worker>,
        on_boot_sec: Duration,
        on_unit_active_sec: Duration,
        cmds: &[String],
    ) {
        info!(
            "launching monotonic schedule in {}s, re-running every {}s",
            on_boot_sec.as_secs_f64(),
            on_unit_active_sec.as_secs_f64(),
        );
        // clone everything to move into the initial run later future
        let cmds_later = cmds.to_owned();
        let tx_stdout_later = self.tx_stdout.clone();
        let tx_stderr_later = self.tx_stderr.clone();
        let tx_status_later = self.tx_status.clone();

        let _ = ctx.run_later(on_boot_sec, move |_act, ctx| {
            let cmds_interval = cmds_later.clone();
            let tx_stdout_interval = tx_stdout_later.clone();
            let _tx_stderr_interval = tx_stderr_later.clone();
            let _tx_status_interval = tx_status_later.clone();

            let _ = ctx.run_interval(on_unit_active_sec, move |_act, _ctx| {
                let cmds_thread = cmds_interval.clone();
                let tx_stdout_thread = tx_stdout_interval.clone();

                // Run the long running commands in a separate thread
                let _b = thread::spawn(move || {
                    // Run the commands sequentially
                    for _cmd in &cmds_thread {
                        // Simulating a command running
                        info!("Running 'interval' command");
                        thread::sleep(Duration::from_secs(5));
                        let msg = WorkerClientToWorkerSession::into_stdout("test");
                        if let Err(e) = tx_stdout_thread.send(msg) {
                            error!("Error running command: {e}");
                        }
                    }
                });
            });

            // Run the commands sequentially
            for _cmd in &cmds_later {
                // Simulating a command running
                info!("Running 'later' command");
                thread::sleep(Duration::from_secs(5));
                let msg = WorkerClientToWorkerSession::into_stdout("test");
                if let Err(e) = tx_stdout_later.send(msg) {
                    error!("Error running command: {e}");
                }
            }
        });
    }

    #[allow(clippy::unused_self)]
    fn launch_realtime(
        &mut self,
        _ctx: &mut Context<Worker>,
        on_calendar: &str,
        _persistent: bool,
        _cmds: &[String],
    ) {
        info!("launching realtime schedule for calendar '{on_calendar}'");
        match parse_calendar(on_calendar) {
            Ok(_rt) => {}
            Err(e) => error!("{e}"),
        }
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

impl Handler<WorkerClientToWorkerSession> for Worker {
    type Result = ();

    fn handle(&mut self, msg: WorkerClientToWorkerSession, _ctx: &mut Context<Self>) {
        match serialize(&msg) {
            Ok(msg_bytes) => self.stdout_queue.push_back(msg_bytes),
            Err(e) => error!("{e}"),
        }
    }
}
