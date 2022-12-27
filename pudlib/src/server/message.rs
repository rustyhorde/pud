// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Actix messages for a server

use crate::{Command, JobDoc, Schedule};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use time::OffsetDateTime;
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
    /// A schedules request has been fulfilled
    Schedules {
        /// The manager id that originated the request
        manager_id: Uuid,
        /// The name of the worker
        name: String,
        /// The currently loaded schedules
        schedules: Vec<Schedule>,
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
    /// A reload has been requested, worker should re-initialize
    Reload,
    /// A request for the current loaded schedules
    Schedules(Uuid),
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
    Reload(Uuid),
    /// List the connected workers
    ListWorkers(Uuid),
    /// Schedules for the given worker
    Schedules {
        /// The id of the manager
        id: Uuid,
        /// The name of the worker to fetch schedules from
        name: String,
    },
    /// Job output for the given worker
    Query {
        /// The id of the manager
        id: Uuid,
        /// The job output
        output: Vec<JobDoc>,
    },
}

/// A message for a manager
#[allow(variant_size_differences)]
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum ServerToManagerClient {
    /// A status message for a manager
    Status(String),
    /// initialize response for a manager
    Initialize,
    /// Reload status
    Reload(bool),
    /// Connected Workers
    WorkersList(HashMap<Uuid, (String, String)>),
    /// Schedules for the given worker
    Schedules {
        /// The name of the worker
        name: String,
        /// The schedules currently loaded on the worker
        schedules: Vec<Schedule>,
    },
    /// Job details
    QueryReturn {
        /// The stdout from a job
        stdout: Vec<String>,
        /// The stderr from a job
        stderr: Vec<String>,
        /// The job status
        status: i32,
        /// The start time of a job
        start_time: OffsetDateTime,
        /// The end time of a job
        end_time: OffsetDateTime,
        /// Are there any more messages coming?
        done: bool,
    },
}

impl From<String> for ServerToManagerClient {
    fn from(value: String) -> Self {
        Self::Status(value)
    }
}
