// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Utility functions

use actix::Actor;
use actix_http::ws::Item;
use actix_web::web::Bytes;
use actix_web_actors::ws::{Message, WebsocketContext};
use bincode::{config::standard, encode_to_vec, serde::Compat};
use serde::Serialize;
use tracing::{debug, error};

pub(crate) fn handle_server_to_client<T, U>(msg: T, ctx: &mut WebsocketContext<U>)
where
    T: Serialize,
    U: Actor<Context = WebsocketContext<U>>,
{
    debug!("handling message from server actor to manager client");
    let bincode_compat = Compat(msg);
    if let Ok(wm_bytes) = encode_to_vec(&bincode_compat, standard()) {
        if wm_bytes.len() > 65_536 {
            let chunks = wm_bytes.chunks(65_536);
            let (_lower, upper_opt) = chunks.size_hint();
            if let Some(upper) = upper_opt {
                for (idx, chunk) in wm_bytes.chunks(65_536).enumerate() {
                    debug!("chunk length: {}", chunk.len());
                    if idx == 0 {
                        ctx.write_raw(Message::Continuation(Item::FirstBinary(
                            Bytes::copy_from_slice(chunk),
                        )));
                    } else if idx == (upper - 1) {
                        ctx.write_raw(Message::Continuation(Item::Last(Bytes::copy_from_slice(
                            chunk,
                        ))));
                    } else {
                        ctx.write_raw(Message::Continuation(Item::Continue(
                            Bytes::copy_from_slice(chunk),
                        )));
                    }
                }
            }
        } else {
            ctx.binary(wm_bytes);
        }
    } else {
        error!("error serializing message");
        ctx.binary(Bytes::from_static(b"error serializing message"));
    }
}
