// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Utilities

use actix_web::web::Bytes;
use std::time::Duration;
use tracing::debug;

pub(crate) fn parse_websocat_ping(bytes: &Bytes) {
    if bytes.len() == 12 {
        let secs_bytes = <[u8; 8]>::try_from(&bytes[0..8]).unwrap_or([0; 8]);
        let nanos_bytes = <[u8; 4]>::try_from(&bytes[8..12]).unwrap_or([0; 4]);
        let secs = u64::from_be_bytes(secs_bytes);
        let nanos = u32::from_be_bytes(nanos_bytes);
        let dur = Duration::new(secs, nanos);
        debug!("ping duration: {}s", dur.as_secs_f64());
    }
}
