// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// The cli actix actor

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use actix::{
    io::{SinkWrite, WriteHandler},
    Actor, ActorContext, AsyncContext, Context, Handler, SpawnHandle, StreamHandler, System,
};
use actix_codec::Framed;
use actix_http::ws::Item;
use awc::{
    error::WsProtocolError,
    ws::{CloseReason, Codec, Frame, Message},
    BoxedSocket,
};
use bincode::{deserialize, serialize};
use bytes::{Bytes, BytesMut};
use futures::stream::SplitSink;
use pudlib::{parse_ts_ping, send_ts_ping, ManagerClientToManagerSession, ServerToManagerClient};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};
use typed_builder::TypedBuilder;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(TypedBuilder)]
pub(crate) struct CommandLine {
    // current heartbeat instant
    #[builder(default = Instant::now())]
    hb: Instant,
    // The addr used to send messages back to the worker session
    addr: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    // the sender for manager client to manager session
    #[allow(dead_code)]
    tx: UnboundedSender<ManagerClientToManagerSession>,
    // the command to run on the server
    command_to_run: ManagerClientToManagerSession,
    #[builder(default = VecDeque::new())]
    // the stdout queue
    stdout_queue: VecDeque<Vec<u8>>,
    // continuation bytes
    #[builder(default = BytesMut::new())]
    cont_bytes: BytesMut,
    // The start instant of this session
    #[builder(default = Instant::now())]
    origin: Instant,
    // Current futures handles
    #[builder(default = Vec::new())]
    fut_handles: Vec<SpawnHandle>,
}

impl CommandLine {
    // Heartbeat that sends ping to the server every HEARTBEAT_INTERVAL seconds (5)
    // Also check for activity from the worker in the past CLIENT_TIMEOUT seconds (10)
    fn hb(&mut self, ctx: &mut Context<Self>) {
        debug!("Starting worker session heartbeat");
        let origin_c = self.origin;
        let hb_handle = ctx.run_interval(HEARTBEAT_INTERVAL, move |act, ctx| {
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
        self.fut_handles.push(hb_handle);
    }

    #[allow(clippy::unused_self)]
    fn handle_text(&mut self, bytes: &Bytes) {
        debug!("handling text message");
        error!("invalid text received: {}", String::from_utf8_lossy(bytes));
    }

    #[allow(clippy::unused_self)]
    fn handle_binary(&mut self, ctx: &mut Context<Self>, bytes: &Bytes) {
        if let Ok(msg) = deserialize::<ServerToManagerClient>(bytes) {
            match msg {
                ServerToManagerClient::Status(status) => info!("Status: {status}"),
                ServerToManagerClient::Initialize => {
                    info!("command line initialization complete");
                    // request reload from the server
                    if let Ok(init) = serialize(&self.command_to_run) {
                        if let Err(_e) = self.addr.write(Message::Binary(Bytes::from(init))) {
                            error!("Unable to send message");
                        }
                    } else {
                        error!("Unable to serialize message");
                    }
                }
                ServerToManagerClient::Reload(result) => {
                    error!(
                        "reload was a {}",
                        if result { "success" } else { "failure" }
                    );
                    ctx.stop();
                }
                ServerToManagerClient::WorkersList(workers) => {
                    let count = workers.len();
                    let max_ip_len = workers
                        .iter()
                        .map(|x| (x.1).0.len())
                        .max_by(Ord::cmp)
                        .unwrap_or(20);
                    let max_name_len = workers
                        .iter()
                        .map(|x| (x.1).1.len())
                        .max_by(Ord::cmp)
                        .unwrap_or(20);
                    error!("{count} worker(s) connected");
                    let mut lines = vec![];

                    for (id, (ip, name)) in &workers {
                        lines.push(format!("{name:max_name_len$} - {ip:max_ip_len$} ({id})"));
                    }

                    lines.sort();

                    for line in &lines {
                        error!("{line}");
                    }
                    ctx.stop();
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
}

impl Actor for CommandLine {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("command line actor started");
        // start heartbeat otherwise server will disconnect after 10 seconds
        self.hb(ctx);
        // request initialization from the server
        if let Ok(init) = serialize(&ManagerClientToManagerSession::Initialize) {
            if let Err(_e) = self.addr.write(Message::Binary(Bytes::from(init))) {
                error!("Unable to send initialize message");
            }
        } else {
            error!("Unable to serialize initialize message");
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        info!("command line actor stopped");
        // Stop application on disconnect
        System::current().stop();
    }
}

/// Handle server websocket messages
impl StreamHandler<Result<Frame, WsProtocolError>> for CommandLine {
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

impl WriteHandler<WsProtocolError> for CommandLine {}

impl Handler<ManagerClientToManagerSession> for CommandLine {
    type Result = ();

    fn handle(&mut self, msg: ManagerClientToManagerSession, _ctx: &mut Context<Self>) {
        match serialize(&msg) {
            Ok(msg_bytes) => self.stdout_queue.push_back(msg_bytes),
            Err(e) => error!("{e}"),
        }
    }
}
