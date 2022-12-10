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
use actix_http::ws::Item;
use awc::{
    error::WsProtocolError,
    ws::{Codec, Frame, Message},
    BoxedSocket,
};
use bincode::serialize;
use bytes::Bytes;
use futures::stream::SplitSink;
use pudlib::{parse_ts_ping, send_ts_ping, Server};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use tracing::{debug, error, info};
use typed_builder::TypedBuilder;

use crate::model::config::Config;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(TypedBuilder)]
pub(crate) struct Worker {
    #[builder(default = Instant::now())]
    hb: Instant,
    addr: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    config: Config,
    // tx_stdout: UnboundedSender<Stdout>,
    // tx_stderr: UnboundedSender<Stderr>,
    // tx_status: UnboundedSender<Status>,
    #[builder(default = None)]
    stdout_handle: Option<SpawnHandle>,
    #[builder(default = VecDeque::new())]
    stdout_queue: VecDeque<Vec<u8>>,
    #[builder(default = Instant::now())]
    stdout_last: Instant,
    running: bool,
    /// continuation bytes
    #[builder(default = Vec::new())]
    cont_bytes: Vec<u8>,
    /// The start instant of this session
    #[builder(default = Instant::now())]
    origin: Instant,
}

impl Worker {
    // Heartbeat that sends ping to the worker every HEARTBEAT_INTERVAL seconds (5)
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
}

impl Actor for Worker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // start heartbeat otherwise server will disconnect after 10 seconds
        self.hb(ctx);
        // initialze the queue monitor
        self.queue_monitor(ctx);
        // request initialization from the server
        let message = Server::Initialize(self.config.name().clone());
        if let Ok(init) = serialize(&message) {
            if let Err(_e) = self.addr.write(Message::Binary(Bytes::from(init))) {
                error!("Unable to send initialize message");
            }
        } else {
            error!("Unable to serialize initialize message");
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        // Stop application on disconnect
        System::current().stop();
    }
}

/// Handle server websocket messages
impl StreamHandler<Result<Frame, WsProtocolError>> for Worker {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Self::Context) {
        if let Ok(message) = msg {
            match message {
                Frame::Binary(_bytes) => {
                    if self.stdout_handle.is_none() {
                        self.stdout_handle = Some(self.stdout_queue(ctx));
                    }
                }
                Frame::Text(_bytes) => {}
                Frame::Ping(bytes) => {
                    debug!("received ping message from server, sending pong");
                    if let Some(dur) = parse_ts_ping(&bytes) {
                        debug!("ping duration: {}s", dur.as_secs_f64());
                    }
                    self.hb = Instant::now();
                    if let Err(e) = self.addr.write(Message::Pong(bytes)) {
                        error!("unable to send pong: {e:?}");
                    }
                }
                Frame::Pong(bytes) => {
                    debug!("received pong message from server, resetting heartbeat");
                    if let Some(dur) = parse_ts_ping(&bytes) {
                        debug!("pong duration: {}s", dur.as_secs_f64());
                    }
                    self.hb = Instant::now();
                }
                Frame::Close(reason) => {
                    debug!("received close message from worker");
                    if let Some(reason) = reason {
                        info!("close reason: {reason:?}");
                    }
                    ctx.stop();
                }
                Frame::Continuation(item) => {
                    debug!("received continuation message from worker");
                    match item {
                        Item::FirstText(_bytes) => error!("unexpected text continuation"),
                        Item::FirstBinary(bytes) | Item::Continue(bytes) => {
                            self.cont_bytes.append(&mut bytes.to_vec());
                        }
                        Item::Last(bytes) => {
                            self.cont_bytes.append(&mut bytes.to_vec());
                            // TODO: Deserialize the bytes here
                            self.cont_bytes.clear();
                        }
                    }
                }
            }
        }
    }

    fn started(&mut self, _ctx: &mut Self::Context) {}

    fn finished(&mut self, ctx: &mut Self::Context) {
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for Worker {}
