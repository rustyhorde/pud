// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// shared server code

use bincode::Encode;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub(crate) mod message;

/// A command to run on a worker
#[derive(Clone, Debug, Default, Deserialize, Encode, Eq, Getters, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Encode, Eq, PartialEq, Serialize)]
pub enum Schedule {
    /// A monotonic schedule
    Monotonic {
        /// Seconds after the worker clients starts to run the first command
        on_boot_sec: Duration,
        /// Seconds after the first run to run the command again
        on_unit_active_sec: Duration,
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

#[cfg(test)]
mod test {
    use super::{Schedule, Schedules};
    use anyhow::Result;
    use toml::from_str;

    const SCHEDULES: &str = r#"schedules = [ 
    { Realtime = { on_calendar = "*-*-* 4:00:00", persistent = false, cmds = ["python"] } },
    { Realtime = { on_calendar = "*-*-* 4:30:00", persistent = false, cmds = ["tmux"] } },
    { Monotonic = { on_boot_sec = { secs = 1, nanos = 0 }, on_unit_active_sec = { secs = 1, nanos = 0 }, cmds = ["updall"] } } 
]"#;

    #[test]
    fn deserialize_schedule() -> Result<()> {
        let schedules: Schedules = from_str(SCHEDULES)?;
        let realtime = schedules
            .schedules()
            .iter()
            .filter(|x| match x {
                Schedule::Realtime {
                    on_calendar: _,
                    persistent: _,
                    cmds: _,
                } => true,
                Schedule::Monotonic {
                    on_boot_sec: _,
                    on_unit_active_sec: _,
                    cmds: _,
                } => false,
            })
            .cloned();
        let monotonic = schedules
            .schedules()
            .iter()
            .filter(|x| match x {
                Schedule::Monotonic {
                    on_boot_sec: _,
                    on_unit_active_sec: _,
                    cmds: _,
                } => true,
                Schedule::Realtime {
                    on_calendar: _,
                    persistent: _,
                    cmds: _,
                } => false,
            })
            .cloned();
        assert_eq!(3, schedules.schedules().len());
        assert_eq!(2, realtime.count());
        assert_eq!(1, monotonic.count());
        Ok(())
    }
}
