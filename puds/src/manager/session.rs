// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Manager Session

use crate::{
    manager::message::{Connect, Disconnect},
    server::Server,
    utils::parse_websocat_ping,
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Handler,
    Running, StreamHandler, WrapFuture,
};
use actix_http::ws::Item;
use actix_web::web::Bytes;
use actix_web_actors::ws::{Message, ProtocolError, WebsocketContext};
use bincode::serialize;
use pudlib::Manager;
use std::time::{Duration, Instant};
use tracing::{debug, error};
use typed_builder::TypedBuilder;
use uuid::Uuid;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(TypedBuilder)]
pub(crate) struct Session {
    // unique session id
    id: Uuid,
    // mux worker must send ping at least once per CLIENT_TIMEOUT
    // otherwise we drop connection.
    hb: Instant,
    /// mux server
    addr: Addr<Server>,
    /// the session ip
    ip: String,
    /// the session name
    name: String,
    /// continuation bytes
    #[builder(default = Vec::new())]
    cont_bytes: Vec<u8>,
}

impl Session {
    // Heartbeat that sends ping to the manager every HEARTBEAT_INTERVAL seconds (5)
    // Also check for activity from the manager in the past CLIENT_TIMEOUT seconds (10)
    #[allow(clippy::unused_self)]
    fn hb(&self, ctx: &mut WebsocketContext<Self>) {
        debug!("Starting manager session heartbeat");
        let _ = ctx.run_interval(HEARTBEAT_INTERVAL, move |act, ctx| {
            debug!("checking heartbeat");
            // check heartbeat
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                error!("heartbeat timed out, disconnecting!");

                // send disconnect to server
                act.addr.do_send(Disconnect::builder().id(act.id).build());

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }
            debug!("sending heartbeat ping");
            ctx.ping(b"heartbeat");
        });
    }
}

impl Actor for Session {
    type Context = WebsocketContext<Self>;

    // Method is called on actor start.
    // We register mux worker session with the mux server
    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("worker session started");
        // start the heartbeat
        self.hb(ctx);

        // Get our address and send a connect worker
        // message to the server.  After registration
        // our id has been set
        debug!("registering with the server");
        let addr = ctx.address();
        self.addr
            .send(
                Connect::builder()
                    .addr(addr.recipient())
                    .ip(self.ip.clone())
                    .name(self.name.clone())
                    .build(),
            )
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
        debug!("server registration complete: {}", self.id);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        debug!("session stopping, sending Disconnect to server");
        self.addr.do_send(Disconnect::builder().id(self.id).build());
        Running::Stop
    }
}

// Handle messages from server, we simply send it to peer websocket
impl Handler<Manager> for Session {
    type Result = ();

    fn handle(&mut self, msg: Manager, ctx: &mut Self::Context) {
        debug!("message received from server, sending on to manager");
        if let Ok(wm_bytes) = serialize(&msg) {
            ctx.binary(wm_bytes);
        } else {
            error!("error serializing message");
            ctx.binary(Bytes::from_static(b"error serializing message"));
        }
    }
}

// WebSocket message handler
impl StreamHandler<Result<Message, ProtocolError>> for Session {
    fn handle(&mut self, msg_res: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg_res {
            match msg {
                Message::Ping(bytes) => {
                    debug!("received ping message from manager, sending pong");
                    parse_websocat_ping(&bytes);
                    self.hb = Instant::now();
                    ctx.pong(&bytes);
                }
                Message::Pong(bytes) => {
                    debug!("received pong message from manager, resetting heartbeat");
                    debug!("pong: {}", String::from_utf8_lossy(&bytes));
                    self.hb = Instant::now();
                }
                Message::Text(text) => error!("unexpected text message: {}", text),
                Message::Binary(bytes) => {
                    self.hb = Instant::now();
                    let _bytes_vec = bytes.to_vec();
                    // if let Ok(mut cmd) = deserialize::<Command>(&bytes) {
                    //     let _ = cmd.set_manager_id(self.id);
                    //     self.addr.do_send(cmd);
                    // } else if let Ok(mut workers) = deserialize::<Workers>(&bytes) {
                    //     let _ = workers.set_manager_id(self.id);
                    //     self.addr.do_send(workers);
                    // }
                }
                Message::Close(reason) => {
                    ctx.close(reason);
                    ctx.stop();
                }
                Message::Continuation(item) => {
                    debug!("received continuation message from manager");
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
                Message::Nop => (),
            }
        } else {
            ctx.stop();
        }
    }
}
