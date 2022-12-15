// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// realtime HH:MM:SS helpers

use super::{parse_time_chunk, All};
use crate::error::Error::InvalidTime;
use anyhow::Result;

const HOURS_PER_DAY: u8 = 24;
const MINUTES_PER_HOUR: u8 = 60;
const SECONDS_PER_MINUTE: u8 = 60;

/// The hour for a realtime schedule
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Hour {
    /// Every hour
    All,
    /// Specific hours
    Hours(Vec<u8>),
}

impl Hour {
    pub(crate) fn matches(&self, given: u8) -> bool {
        match self {
            Hour::All => true,
            Hour::Hours(hours) => hours.contains(&given),
        }
    }
}

impl All for Hour {
    fn all() -> Self {
        Self::All
    }
}

impl From<Vec<u8>> for Hour {
    fn from(value: Vec<u8>) -> Self {
        Hour::Hours(value)
    }
}

impl From<u8> for Hour {
    fn from(value: u8) -> Self {
        Hour::Hours(vec![value])
    }
}

/// The minute for a realtime schedule
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Minute {
    /// Every minute
    All,
    /// Specific minutes
    Minutes(Vec<u8>),
}

impl Minute {
    pub(crate) fn matches(&self, given: u8) -> bool {
        match self {
            Minute::All => true,
            Minute::Minutes(minutes) => minutes.contains(&given),
        }
    }
}

impl All for Minute {
    fn all() -> Self {
        Self::All
    }
}

impl From<Vec<u8>> for Minute {
    fn from(value: Vec<u8>) -> Self {
        Minute::Minutes(value)
    }
}

impl From<u8> for Minute {
    fn from(value: u8) -> Self {
        Minute::Minutes(vec![value])
    }
}

/// The seconds for a realtime schedule
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Second {
    /// Every second
    All,
    /// Specific seconds
    Seconds(Vec<u8>),
}

impl Second {
    pub(crate) fn matches(&self, given: u8) -> bool {
        match self {
            Second::All => true,
            Second::Seconds(seconds) => seconds.contains(&given),
        }
    }
}

impl All for Second {
    fn all() -> Self {
        Self::All
    }
}

impl From<Vec<u8>> for Second {
    fn from(value: Vec<u8>) -> Self {
        Second::Seconds(value)
    }
}

impl From<u8> for Second {
    fn from(value: u8) -> Self {
        Second::Seconds(vec![value])
    }
}

pub(crate) fn parse_hms(hms: &str) -> Result<(Hour, Minute, Second)> {
    let hms_parts: Vec<&str> = hms.split(':').collect();
    if hms_parts.len() == 3 {
        let hour = parse_time_chunk::<Hour>(hms_parts[0], HOURS_PER_DAY, false)?;
        let minute = parse_time_chunk::<Minute>(hms_parts[1], MINUTES_PER_HOUR, false)?;
        let second = parse_time_chunk::<Second>(hms_parts[2], SECONDS_PER_MINUTE, false)?;
        Ok((hour, minute, second))
    } else {
        Err(InvalidTime {
            time: hms.to_string(),
        }
        .into())
    }
}

#[cfg(test)]
mod test {
    use super::{
        parse_hms, Hour, Minute, Second, HOURS_PER_DAY, MINUTES_PER_HOUR, SECONDS_PER_MINUTE,
    };
    use anyhow::{anyhow, Result};

    #[test]
    fn simple() -> Result<()> {
        let (hour, minute, second) = parse_hms("10:00:00")?;
        assert_eq!(hour, Hour::Hours(vec![10]));
        assert_eq!(minute, Minute::Minutes(vec![0]));
        assert_eq!(second, Second::Seconds(vec![0]));
        Ok(())
    }

    #[test]
    fn range() -> Result<()> {
        let (hour, minute, second) = parse_hms("9..17:15..45:20..50")?;
        assert_eq!(hour, Hour::Hours((9..=17).collect()));
        assert_eq!(minute, Minute::Minutes((15..=45).collect()));
        assert_eq!(second, Second::Seconds((20..=50).collect()));
        Ok(())
    }

    #[test]
    fn simple_repetition() -> Result<()> {
        let (hour, minute, second) = parse_hms("0/2:0/3:0/4")?;
        assert_eq!(hour, Hour::Hours((0..HOURS_PER_DAY).step_by(2).collect()));
        assert_eq!(
            minute,
            Minute::Minutes((0..MINUTES_PER_HOUR).step_by(3).collect())
        );
        assert_eq!(
            second,
            Second::Seconds((0..SECONDS_PER_MINUTE).step_by(4).collect())
        );
        Ok(())
    }

    #[test]
    fn range_repetition() -> Result<()> {
        let (hour, minute, second) = parse_hms("9..17/2:12..44/4:20..50/4")?;
        assert_eq!(hour, Hour::Hours((9..=17).step_by(2).collect()));
        assert_eq!(minute, Minute::Minutes((12..=44).step_by(4).collect()));
        assert_eq!(second, Second::Seconds((20..=50).step_by(4).collect()));
        Ok(())
    }

    #[test]
    fn invalid_hour_range() -> Result<()> {
        match parse_hms("17..9:00:00") {
            Ok(_) => Err(anyhow!("this time should be invalid")),
            Err(e) => {
                assert_eq!(format!("{e}"), "invalid range: '17..9'");
                Ok(())
            }
        }
    }
}
