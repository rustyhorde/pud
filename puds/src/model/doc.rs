// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! job results document

use getset::{Getters, MutGetters, Setters};
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug, Eq, Getters, MutGetters, PartialEq, Serialize, Setters)]
#[getset(get = "pub(crate)", set = "pub(crate)")]
pub(crate) struct Job {
    worker_id: Uuid,
    worker_name: String,
    job_id: Uuid,
    job_name: String,
    #[serde(with = "time::serde::iso8601")]
    start_time: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    end_time: OffsetDateTime,
    #[getset(get_mut = "pub(crate)")]
    stdout: Vec<String>,
    #[getset(get_mut = "pub(crate)")]
    stderr: Vec<String>,
    status: i32,
}

impl Job {
    pub(crate) fn new<T>(worker_id: Uuid, worker_name: T, job_id: Uuid, job_name: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            worker_id,
            worker_name: worker_name.into(),
            start_time: OffsetDateTime::now_utc(),
            end_time: OffsetDateTime::now_utc(),
            job_id,
            job_name: job_name.into(),
            stdout: vec![],
            stderr: vec![],
            status: i32::default(),
        }
    }
}
