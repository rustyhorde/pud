// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Worker

use self::message::Connect;
use actix::Recipient;
use getset::Getters;
use pudlib::ServerToWorkerClient;

pub(crate) mod message;
pub(crate) mod session;

// Worker information stored with server on connect
#[derive(Clone, Debug, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct Worker {
    name: String,
    ip: String,
    addr: Recipient<ServerToWorkerClient>,
}

impl From<Connect> for Worker {
    fn from(value: Connect) -> Self {
        let (addr, ip, name) = value.take();
        Worker { name, ip, addr }
    }
}
