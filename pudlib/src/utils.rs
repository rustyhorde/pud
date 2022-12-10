// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Utilities

use bytes::Bytes;
use std::time::{Duration, Instant};

/// Parse a received timestamp ping
pub fn parse_ts_ping(bytes: &Bytes) -> Option<Duration> {
    if bytes.len() == 12 {
        let secs_bytes = <[u8; 8]>::try_from(&bytes[0..8]).unwrap_or([0; 8]);
        let nanos_bytes = <[u8; 4]>::try_from(&bytes[8..12]).unwrap_or([0; 4]);
        let secs = u64::from_be_bytes(secs_bytes);
        let nanos = u32::from_be_bytes(nanos_bytes);
        Some(Duration::new(secs, nanos))
    } else {
        None
    }
}

/// Send a timestamp ping
#[must_use]
pub fn send_ts_ping(origin: Instant) -> [u8; 12] {
    let ts = Instant::now().duration_since(origin);
    let (ts1, ts2) = (ts.as_secs(), ts.subsec_nanos());
    let mut ts = [0; 12];
    ts[0..8].copy_from_slice(&ts1.to_be_bytes());
    ts[8..12].copy_from_slice(&ts2.to_be_bytes());
    ts
}
