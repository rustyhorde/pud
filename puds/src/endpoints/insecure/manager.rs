// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Insecure Manager websocket endpoint

use super::Name;
use crate::{error::Error::Actix, manager::session::Session, server::Server};
use actix::Addr;
use actix_web::{
    web::{Data, Json, Payload, Query},
    HttpRequest, HttpResponse,
};
use actix_web_actors::ws::start;
use std::time::Instant;
use tracing::{debug, error, info};
use uuid::Uuid;

// do websocket handshake and start a manager session
#[allow(clippy::unused_async)]
pub(crate) async fn manager(
    request: HttpRequest,
    stream: Payload,
    name: Query<Name>,
    srv: Data<Addr<Server>>,
) -> HttpResponse {
    info!("manager connecting...");
    let unknown = String::from("Unknown");
    let conn_info = request.connection_info();
    let ip = conn_info
        .realip_remote_addr()
        .map_or(unknown.clone(), ToString::to_string);
    let name = name.name.as_deref().map_or(unknown, ToString::to_string);
    info!("Name: {name}, Ip: {ip}");
    let response = start(
        Session::builder()
            .id(Uuid::new_v4())
            .addr(srv.as_ref().clone())
            .name(name)
            .ip(ip)
            .hb(Instant::now())
            .origin(Instant::now())
            .build(),
        &request,
        stream,
    );
    debug!("{response:?}");
    match response {
        Ok(res) => res,
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().json(Json(Actix {
                msg: format!("{e}"),
            }))
        }
    }
}
