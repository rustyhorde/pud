// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Worker Actix Message

use actix::Message;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A message from a worker client to a worker session
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum WorkerClientToWorkerSession {
    /// A text message for a server
    Text(String),
    /// A stdout line from a command
    Stdout {
        /// The command id associated with this line
        id: Uuid,
        /// The stdout line
        line: String,
    },
    /// A stderr line from a command
    Stderr {
        /// The command id associated with this line
        id: Uuid,
        /// The stderr line
        line: String,
    },
    /// A status from a command
    Status {
        /// The command id associated with this status
        id: Uuid,
        /// The status code
        code: i32,
    },
    /// An initialization request from a worker
    Initialize,
}

impl WorkerClientToWorkerSession {
    /// Convert a value into a `WorkerClientToWorkerSession::Stdout` message
    pub fn into_stdout<T>(value: T) -> Self
    where
        T: Into<String>,
    {
        Self::Stdout {
            id: Uuid::new_v4(),
            line: value.into(),
        }
    }

    /// Convert a value into a `WorkerClientToWorkerSession::Stderr` message
    pub fn into_stderr<T>(value: T) -> Self
    where
        T: Into<String>,
    {
        Self::Stderr {
            id: Uuid::new_v4(),
            line: value.into(),
        }
    }
}
