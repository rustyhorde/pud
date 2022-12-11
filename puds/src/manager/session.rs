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
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Handler,
    Running, StreamHandler, WrapFuture,
};
use actix_http::ws::{CloseReason, Item};
use actix_web::web::{Bytes, BytesMut};
use actix_web_actors::ws::{Message, ProtocolError, WebsocketContext};
use bincode::serialize;
use bytestring::ByteString;
use pudlib::{parse_ts_ping, send_ts_ping, ServerToManagerClient};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};
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
    #[builder(default = BytesMut::new())]
    cont_bytes: BytesMut,
    /// The start instant of this session
    origin: Instant,
}

impl Session {
    // Heartbeat that sends ping to the manager every HEARTBEAT_INTERVAL seconds (5)
    // Also check for activity from the manager in the past CLIENT_TIMEOUT seconds (10)
    #[allow(clippy::unused_self)]
    fn hb(&self, ctx: &mut WebsocketContext<Self>) {
        debug!("Starting manager session heartbeat");
        let origin_c = self.origin;
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
            ctx.ping(&send_ts_ping(origin_c));
        });
    }

    #[allow(clippy::unused_self)]
    fn handle_text(&mut self, byte_string: &ByteString) {
        debug!("handling text message");
        error!("invalid text received: {byte_string}");
    }

    fn handle_ping(&mut self, ctx: &mut WebsocketContext<Self>, bytes: &Bytes) {
        debug!("handling ping message");
        if let Some(dur) = parse_ts_ping(bytes) {
            debug!("ping duration: {}s", dur.as_secs_f64());
        }
        self.hb = Instant::now();
        ctx.pong(bytes);
    }

    fn handle_pong(&mut self, bytes: &Bytes) {
        debug!("handling pong message");
        if let Some(dur) = parse_ts_ping(bytes) {
            debug!("pong duration: {}s", dur.as_secs_f64());
        }
        self.hb = Instant::now();
    }

    fn handle_binary(&mut self, bytes: &Bytes) {
        debug!("handling binary message");
        self.hb = Instant::now();
        let _bytes_vec = bytes.to_vec();
    }

    #[allow(clippy::unused_self)]
    fn handle_close(&mut self, ctx: &mut WebsocketContext<Self>, reason: Option<CloseReason>) {
        debug!("handling close message");
        ctx.close(reason);
        ctx.stop();
    }

    fn handle_continuation(&mut self, item: Item) {
        debug!("handling continuation message");
        match item {
            Item::FirstText(_bytes) => error!("unexpected text continuation"),
            Item::FirstBinary(bytes) | Item::Continue(bytes) => {
                self.cont_bytes.extend_from_slice(&bytes);
            }
            Item::Last(bytes) => {
                self.cont_bytes.extend_from_slice(&bytes);
                self.handle_binary(&bytes);
                self.cont_bytes.clear();
            }
        }
    }

    #[allow(clippy::unused_self)]
    fn handle_no_op(&mut self) {
        debug!("handling no op message");
    }
}

impl Actor for Session {
    type Context = WebsocketContext<Self>;

    // Method is called on actor start.
    // We register manager session with the server
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("manager session started");
        // start the heartbeat
        self.hb(ctx);

        // Get our address and send a connect manager
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
        info!("manager session stopping");
        self.addr.do_send(Disconnect::builder().id(self.id).build());
        Running::Stop
    }
}

// Handle messages from server, we simply send it to peer websocket
impl Handler<ServerToManagerClient> for Session {
    type Result = ();

    fn handle(&mut self, msg: ServerToManagerClient, ctx: &mut Self::Context) {
        debug!("handling message from server actor to manager client");
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
                Message::Ping(bytes) => self.handle_ping(ctx, &bytes),
                Message::Pong(bytes) => self.handle_pong(&bytes),
                Message::Text(byte_string) => self.handle_text(&byte_string),
                Message::Binary(bytes) => self.handle_binary(&bytes),
                Message::Close(reason) => self.handle_close(ctx, reason),
                Message::Continuation(item) => self.handle_continuation(item),
                Message::Nop => self.handle_no_op(),
            }
        } else {
            ctx.stop();
        }
    }

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("manager session stream handler started");
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        info!("manager session stream handler finished");
        ctx.stop();
    }
}
