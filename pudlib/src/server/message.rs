// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Actix messages for a server

use actix::Message;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A message from a worker client to a server
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum WorkerClientToServer {
    /// A text message for a server
    Text(String),
    /// An initialization request from a worker
    Initialize,
}

/// A message from a worker session to the server actor
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum WorkerSessionToServer {
    /// An initialization request from the worker client
    Initialize {
        /// The id of the worker client
        id: Uuid,
        /// The name of the worker client
        name: String,
    },
}
