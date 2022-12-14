// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Manager Messages

use actix::{Message, Recipient};
use getset::CopyGetters;
use pudlib::ServerToManagerClient as ManagerMessage;
use typed_builder::TypedBuilder;
use uuid::Uuid;

// Message received when a `Manager` has connected
#[derive(Clone, Debug, Message, TypedBuilder)]
#[rtype(result = "Uuid")]
pub(crate) struct Connect {
    addr: Recipient<ManagerMessage>,
    ip: String,
    name: String,
}

impl Connect {
    pub(crate) fn take(self) -> (Recipient<ManagerMessage>, String, String) {
        (self.addr, self.ip, self.name)
    }
}

// Message received when a `Manager` has disconnected
#[derive(Clone, CopyGetters, Debug, Message, TypedBuilder)]
#[rtype(result = "()")]
pub(crate) struct Disconnect {
    #[getset(get_copy = "pub(crate)")]
    id: Uuid,
}
