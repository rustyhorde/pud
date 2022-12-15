// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Errors

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("There is no valid config directory")]
    ConfigDir,
    #[error("invalid calendar string: '{}'", calendar)]
    InvalidCalendar { calendar: String },
    #[error("invalid date string: '{}'", date)]
    InvalidDate { date: String },
    #[error("invalid time string: '{}'", time)]
    InvalidTime { time: String },
    #[error("invalid first capture")]
    InvalidFirstCapture,
    #[error("invalid second capture")]
    InvalidSecondCapture,
    #[error("no valid captures")]
    NoValidCaptures,
    #[error("invalid range: '{}'", range)]
    InvalidRange { range: String },
}
