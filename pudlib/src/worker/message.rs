// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Worker Actix Message

use crate::server::Command;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A message from a server to a worker client
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum ServerToWorkerClient {
    /// A text message for a worker
    Text(String),
    /// initialize response for a worker
    Initialize(BTreeMap<String, Command>),
}
