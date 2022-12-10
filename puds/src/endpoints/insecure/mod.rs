// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Insecure Endpoints

use actix_web::web::{get, ServiceConfig};
use serde::Deserialize;

mod health;
mod info;
mod manager;
mod worker;

#[derive(Deserialize)]
pub(crate) struct Name {
    name: Option<String>,
}

pub(crate) fn insecure_config(cfg: &mut ServiceConfig) {
    let _ = cfg
        .route("/health", get().to(health::health))
        .route("/info", get().to(info::info))
        .route("/ws/worker", get().to(worker::worker))
        .route("/ws/manager", get().to(manager::manager));
}
