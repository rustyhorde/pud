// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Actix messages for a server

use crate::{Command, Schedule};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

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

/// A message from a server to a worker client
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum ServerToWorkerClient {
    /// A status message for a worker
    Status(String),
    /// initialize response for a worker
    Initialize(BTreeMap<String, Command>, Vec<Schedule>),
}

impl From<String> for ServerToWorkerClient {
    fn from(value: String) -> Self {
        Self::Status(value)
    }
}

/// A message from a manager session to the server actor
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum ManagerSessionToServer {
    /// An initialization request from the manager client
    Initialize {
        /// The id of the worker client
        id: Uuid,
        /// The name of the worker client
        name: String,
    },
    /// Reload the server configuration
    Reload,
}

/// A message for a manager
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum ServerToManagerClient {
    /// A status message for a manager
    Status(String),
    /// initialize response for a manager
    Initialize,
}

impl From<String> for ServerToManagerClient {
    fn from(value: String) -> Self {
        Self::Status(value)
    }
}
