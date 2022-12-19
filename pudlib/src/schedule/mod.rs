// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Realtime schedule structs

use self::{
    dow::{parse_day_of_week, DayOfWeek},
    hms::{parse_hms, Hour, Minute, Second},
    ymd::{parse_date, Day, Month, Year},
};
use crate::{
    error::Error::{
        InvalidCalendar, InvalidFirstCapture, InvalidRange, InvalidSecondCapture, InvalidTime,
        NoValidCaptures,
    },
    utils::until_err,
};
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;
use time::OffsetDateTime;
use typed_builder::TypedBuilder;

pub(crate) mod dow;
pub(crate) mod hms;
pub(crate) mod ymd;

lazy_static! {
    static ref RANGE_RE: Regex =
        Regex::new(r#"(\d{1,2})\.\.(\d{1,2})"#).expect("invalid range regex");
    static ref REP_RE: Regex =
        Regex::new(r#"(\d{1,2})(\.\.(\d{1,2}))?/(\d{1,2})"#).expect("invalid repetition regex");
}

const MINUTELY: &str = "minutely";
const HOURLY: &str = "hourly";
const DAILY: &str = "daily";
const WEEKLY: &str = "weekly";
const MONTHLY: &str = "monthly";
const QUARTERLY: &str = "quarterly";
const SEMIANUALLY: &str = "semiannually";
const YEARLY: &str = "yearly";

trait All {
    fn all() -> Self;
    fn rand() -> Self;
}

/// A realtime schedule
#[derive(Clone, Debug, Eq, Hash, PartialEq, TypedBuilder)]
pub struct Realtime {
    /// The day(s) of the week to run
    #[builder(default = DayOfWeek::All, setter(into))]
    day_of_week: DayOfWeek,
    /// The year(s) to run
    #[builder(default = Year::All)]
    year: Year,
    /// The month(s) to run
    #[builder(default = Month::All, setter(into))]
    month: Month,
    /// The day(s) of the month
    #[builder(default = Day::All, setter(into))]
    day: Day,
    /// The hour(s) to run
    #[builder(default = Hour::All, setter(into))]
    hour: Hour,
    /// The minutes(s) to run
    #[builder(default = Minute::All, setter(into))]
    minute: Minute,
    /// The second(s) to run
    #[builder(default = Second::All, setter(into))]
    second: Second,
}

impl Default for Realtime {
    fn default() -> Self {
        Self {
            day_of_week: DayOfWeek::All,
            year: Year::All,
            month: Month::All,
            day: Day::All,
            hour: Hour::All,
            minute: Minute::All,
            second: Second::All,
        }
    }
}

impl Realtime {
    /// Should this schedule run at this time
    #[must_use]
    pub fn should_run(&self, now: OffsetDateTime) -> bool {
        self.day_of_week.matches(now.weekday())
            && self.year.matches(now.year())
            && self.month.matches(now.month().into())
            && self.day.matches(now.day())
            && self.hour.matches(now.hour())
            && self.minute.matches(now.minute())
            && self.second.matches(now.second())
    }
}

/// parse the given calendar string
///
/// # Errors
///
pub fn parse_calendar(calendar: &str) -> Result<Realtime> {
    let parts: Vec<&str> = calendar.split_whitespace().collect();

    let (day_of_week, date, hms) = if parts.len() == 3 {
        // has day of week
        (parts[0], parts[1], parts[2])
    } else if parts.len() == 2 {
        // no day of week
        ("*", parts[0], parts[1])
    } else if parts.len() == 1 {
        // no day of week, or date
        if parts[0] == MINUTELY {
            return Ok(Realtime::builder().second(0).build());
        } else if parts[0] == HOURLY {
            return Ok(Realtime::builder().minute(0).second(0).build());
        } else if parts[0] == DAILY {
            return Ok(Realtime::builder().hour(0).minute(0).second(0).build());
        } else if parts[0] == WEEKLY {
            return Ok(Realtime::builder()
                .day_of_week(1)
                .hour(0)
                .minute(0)
                .second(0)
                .build());
        } else if parts[0] == MONTHLY {
            return Ok(Realtime::builder()
                .day(1)
                .hour(0)
                .minute(0)
                .second(0)
                .build());
        } else if parts[0] == QUARTERLY {
            return Ok(Realtime::builder()
                .month(vec![1, 4, 7, 10])
                .day(1)
                .hour(0)
                .minute(0)
                .second(0)
                .build());
        } else if parts[0] == SEMIANUALLY {
            return Ok(Realtime::builder()
                .month(vec![1, 7])
                .day(1)
                .hour(0)
                .minute(0)
                .second(0)
                .build());
        } else if parts[0] == YEARLY {
            return Ok(Realtime::builder()
                .month(1)
                .day(1)
                .hour(0)
                .minute(0)
                .second(0)
                .build());
        }
        ("*", "*", parts[0])
    } else {
        return Err(InvalidCalendar {
            calendar: calendar.to_string(),
        }
        .into());
    };

    let dow = parse_day_of_week(day_of_week)?;
    let (year, month, day) = parse_date(date)?;
    let (hour, minute, second) = parse_hms(hms)?;
    Ok(Realtime::builder()
        .day_of_week(dow)
        .year(year)
        .month(month)
        .day(day)
        .hour(hour)
        .minute(minute)
        .second(second)
        .build())
}

fn parse_time_chunk<T>(part: &str, max: u8, one_based: bool) -> Result<T>
where
    T: All + From<Vec<u8>>,
{
    if part == "*" {
        Ok(T::all())
    } else if part == "R" {
        Ok(T::rand())
    } else {
        let mut err = Ok(());
        let prrv_fn = |hour: &str| -> Result<Vec<u8>> { parse_rep_range_val(hour, max, one_based) };
        let mut time_v: Vec<u8> = part
            .split(',')
            .map(prrv_fn)
            .scan(&mut err, until_err)
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        err?;
        time_v.sort_unstable();
        Ok(T::from(time_v))
    }
}

fn parse_rep_range_val(val: &str, max: u8, one_based: bool) -> Result<Vec<u8>> {
    if REP_RE.is_match(val) {
        parse_repetition(val, max)
    } else if RANGE_RE.is_match(val) {
        parse_range(val, max, one_based)
    } else {
        parse_value(val)
    }
}

fn parse_range(range: &str, max: u8, one_based: bool) -> Result<Vec<u8>> {
    let caps = RANGE_RE.captures(range).ok_or(NoValidCaptures)?;
    let first = caps
        .get(1)
        .ok_or(InvalidFirstCapture)?
        .as_str()
        .parse::<u8>()?;
    let second = caps
        .get(2)
        .ok_or(InvalidSecondCapture)?
        .as_str()
        .parse::<u8>()?;
    if second < first
        || (one_based && first == 0)
        || ((one_based && second > max) || (!one_based && second >= max))
    {
        Err(InvalidRange {
            range: range.to_string(),
        }
        .into())
    } else {
        Ok((first..=second).collect())
    }
}

fn parse_repetition(rep: &str, max: u8) -> Result<Vec<u8>> {
    let caps = REP_RE.captures(rep).ok_or(NoValidCaptures)?;

    if caps.len() == 5 {
        let start = caps
            .get(1)
            .ok_or(InvalidFirstCapture)?
            .as_str()
            .parse::<u8>()?;
        let rep = caps
            .get(4)
            .ok_or(InvalidSecondCapture)?
            .as_str()
            .parse::<usize>()?;
        if let Some(end) = caps.get(3) {
            let end = end.as_str().parse::<u8>()?;
            if end < start || end >= max {
                Err(InvalidRange {
                    range: format!("{start}..{end}"),
                }
                .into())
            } else {
                Ok((start..=end).step_by(rep).collect())
            }
        } else {
            Ok((start..max).step_by(rep).collect())
        }
    } else {
        Err(InvalidTime {
            time: rep.to_string(),
        }
        .into())
    }
}

fn parse_value(value: &str) -> Result<Vec<u8>> {
    Ok(vec![value.parse::<u8>()?])
}

#[cfg(test)]
mod test {
    use super::{
        parse_calendar, Realtime, DAILY, HOURLY, MINUTELY, MONTHLY, QUARTERLY, SEMIANUALLY, WEEKLY,
        YEARLY,
    };
    use anyhow::{anyhow, Result};
    use time::OffsetDateTime;

    #[test]
    fn invalid_calendar() -> Result<()> {
        match parse_calendar("this is a bad calendar") {
            Ok(_) => Err(anyhow!("this should be a bad calendar")),
            Err(e) => {
                assert_eq!(
                    format!("{e}"),
                    "invalid calendar string: 'this is a bad calendar'"
                );
                Ok(())
            }
        }
    }

    #[test]
    fn minutely() -> Result<()> {
        let res = parse_calendar(MINUTELY)?;
        let expected = Realtime::builder().second(0).build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn hourly() -> Result<()> {
        let res = parse_calendar(HOURLY)?;
        let expected = Realtime::builder().minute(0).second(0).build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn daily() -> Result<()> {
        let res = parse_calendar(DAILY)?;
        let expected = Realtime::builder().hour(0).minute(0).second(0).build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn weekly() -> Result<()> {
        let res = parse_calendar(WEEKLY)?;
        let expected = Realtime::builder()
            .day_of_week(1)
            .hour(0)
            .minute(0)
            .second(0)
            .build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn monthly() -> Result<()> {
        let res = parse_calendar(MONTHLY)?;
        let expected = Realtime::builder()
            .day(1)
            .hour(0)
            .minute(0)
            .second(0)
            .build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn quarterly() -> Result<()> {
        let res = parse_calendar(QUARTERLY)?;
        let expected = Realtime::builder()
            .month(vec![1, 4, 7, 10])
            .day(1)
            .hour(0)
            .minute(0)
            .second(0)
            .build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn semiannually() -> Result<()> {
        let res = parse_calendar(SEMIANUALLY)?;
        let expected = Realtime::builder()
            .month(vec![1, 7])
            .day(1)
            .hour(0)
            .minute(0)
            .second(0)
            .build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn yearly() -> Result<()> {
        let res = parse_calendar(YEARLY)?;
        let expected = Realtime::builder()
            .month(1)
            .day(1)
            .hour(0)
            .minute(0)
            .second(0)
            .build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn no_day_of_week() -> Result<()> {
        let res = parse_calendar("*-*-* 3:00:00")?;
        let expected = Realtime::builder().hour(3).minute(0).second(0).build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn full_calendar() -> Result<()> {
        let res = parse_calendar("Mon..Fri *-*-* 3:22:17")?;
        let expected = Realtime::builder()
            .day_of_week((1..=5).collect::<Vec<u8>>())
            .hour(3)
            .minute(22)
            .second(17)
            .build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn funky() -> Result<()> {
        let res = parse_calendar("Mon..Thu,Sun,Sat *-*-* 3..7,10,0,14..18/2:22:17")?;
        let expected = Realtime::builder()
            .day_of_week(vec![0, 1, 2, 3, 4, 6])
            .hour(vec![0, 3, 4, 5, 6, 7, 10, 14, 16, 18])
            .minute(22)
            .second(17)
            .build();
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn invalid_date() -> Result<()> {
        match parse_calendar("*-* 3:11:17") {
            Ok(_) => Err(anyhow!("this should be a bad calendar")),
            Err(e) => {
                assert_eq!(format!("{e}"), "invalid date string: '*-*'");
                Ok(())
            }
        }
    }

    #[test]
    fn invalid_time() -> Result<()> {
        match parse_calendar("*-*-* 12:00") {
            Ok(_) => Err(anyhow!("this should be a bad calendar")),
            Err(e) => {
                assert_eq!(format!("{e}"), "invalid time string: '12:00'");
                Ok(())
            }
        }
    }

    #[test]
    fn should_run() -> Result<()> {
        let rt = Realtime::builder().hour(4).minute(37).second(0).build();
        let odt = OffsetDateTime::now_utc();
        let odt = odt.replace_year(2023)?;
        let odt = odt.replace_month(time::Month::February)?;
        let odt = odt.replace_hour(4)?;
        let odt = odt.replace_minute(37)?;
        let odt = odt.replace_second(0)?;
        assert!(rt.should_run(odt));
        Ok(())
    }
}
