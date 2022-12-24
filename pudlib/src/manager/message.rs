// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Manager Actix Message

use actix::Message;
use serde::{Deserialize, Serialize};

/// A message from a manger client to a manager session
#[derive(Clone, Debug, Deserialize, Message, Serialize)]
#[rtype(result = "()")]
pub enum ManagerClientToManagerSession {
    /// An initialization request from a manager
    Initialize,
    /// A reload request from a manager
    Reload,
    /// List the connected workers
    ListWorkers,
    /// List the schedules for the given worker
    Schedules(String),
}
