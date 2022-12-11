// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// shared server code

use getset::Getters;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub(crate) mod message;

/// A command to run on a worker
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub")]
pub struct Command {
    /// The command to run
    cmd: String,
}

/// The schedule to run commands on a given worker client
#[derive(Clone, Debug, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub")]
pub struct Schedules {
    /// All of the schedules for a worker client
    schedules: Vec<Schedule>,
}

impl Schedules {
    /// Take the schedules from the struct
    #[must_use]
    pub fn take(self) -> Vec<Schedule> {
        self.schedules
    }
}

/// The schedule to run commands on a given worker client
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Schedule {
    /// A monotonic schedule
    Monotonic {
        /// Seconds after the worker clients starts to run the first command
        on_boot_sec: Duration,
        /// Seconds after the first run to run the command again
        on_unit_activ_sec: Duration,
        /// The commands to run
        cmds: Vec<String>,
    },
    /// A realtime schedule
    Realtime {
        /// A calendar string similar to cron format
        on_calendar: String,
        /// Should this job be run if a time was missed
        persistent: bool,
        /// The commands to run
        cmds: Vec<String>,
    },
}
