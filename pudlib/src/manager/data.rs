// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Database document structs

use getset::Getters;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Job document
#[derive(Clone, Debug, Deserialize, Getters, Serialize)]
#[getset(get = "pub")]
pub struct JobDoc {
    /// The start time of the job
    #[serde(with = "time::serde::iso8601")]
    start_time: OffsetDateTime,
    /// The end time of the job
    #[serde(with = "time::serde::iso8601")]
    end_time: OffsetDateTime,
    /// The stdout lines of the job
    stdout: Vec<String>,
    /// The stderr lines of the job
    stderr: Vec<String>,
    /// The status code of the job
    status: i32,
}
