// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// realtime day of week helpers

use crate::{
    error::Error::{InvalidFirstCapture, InvalidRange, InvalidSecondCapture, NoValidCaptures},
    utils::until_err,
};
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;
use time::Weekday;

lazy_static! {
    static ref DOW_RANGE_RE: Regex =
        Regex::new(r#"([a-zA-Z]{3,})\.\.([a-zA-Z]{3,})"#).expect("invalid day of week range regex");
}

/// The day of the week for a realtime schedule
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DayOfWeek {
    /// Every day of the week
    All,
    /// Specific days of the week
    Days(Vec<u8>),
}

impl From<u8> for DayOfWeek {
    fn from(value: u8) -> Self {
        DayOfWeek::Days(vec![value])
    }
}

impl From<Vec<u8>> for DayOfWeek {
    fn from(value: Vec<u8>) -> Self {
        DayOfWeek::Days(value)
    }
}

impl DayOfWeek {
    pub(crate) fn matches(&self, given: Weekday) -> bool {
        match self {
            DayOfWeek::All => true,
            DayOfWeek::Days(days) => {
                let given_u = match given {
                    Weekday::Monday => 1,
                    Weekday::Tuesday => 2,
                    Weekday::Wednesday => 3,
                    Weekday::Thursday => 4,
                    Weekday::Friday => 5,
                    Weekday::Saturday => 6,
                    Weekday::Sunday => 0,
                };
                days.contains(&given_u)
            }
        }
    }
}

pub(crate) fn parse_day_of_week(dowish: &str) -> Result<DayOfWeek> {
    if dowish == "*" {
        Ok(DayOfWeek::All)
    } else {
        let mut err = Ok(());
        let mut dows: Vec<u8> = dowish
            .split(',')
            .map(parse_range_or_dow)
            .scan(&mut err, until_err)
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        err?;
        dows.sort_unstable();
        Ok(DayOfWeek::Days(dows))
    }
}

fn parse_range_or_dow(dow_str: &str) -> Result<Vec<u8>> {
    if DOW_RANGE_RE.is_match(dow_str) {
        parse_dow_range(dow_str)
    } else {
        parse_dow_v(dow_str)
    }
}

fn parse_dow_range(dow_range: &str) -> Result<Vec<u8>> {
    let caps = DOW_RANGE_RE.captures(dow_range).ok_or(NoValidCaptures)?;
    let first = parse_dow(caps.get(1).ok_or(InvalidFirstCapture)?.as_str())?;
    let second = parse_dow(caps.get(2).ok_or(InvalidSecondCapture)?.as_str())?;
    if second < first {
        Err(InvalidRange {
            range: dow_range.to_string(),
        }
        .into())
    } else {
        Ok((first..=second).collect())
    }
}

fn parse_dow_v(dow: &str) -> Result<Vec<u8>> {
    parse_dow(dow).map(|x| vec![x])
}

fn parse_dow(dow: &str) -> Result<u8> {
    let dow_l = dow.to_ascii_lowercase();

    Ok(if &dow_l == "sun" || &dow_l == "sunday" {
        0
    } else if &dow_l == "mon" || &dow_l == "monday" {
        1
    } else if &dow_l == "tue" || &dow_l == "tuesday" {
        2
    } else if &dow_l == "wed" || &dow_l == "wednesday" {
        3
    } else if &dow_l == "thu" || &dow_l == "thursday" {
        4
    } else if &dow_l == "fri" || &dow_l == "friday" {
        5
    } else if &dow_l == "sat" || &dow_l == "saturday" {
        6
    } else {
        return Err(anyhow!("invalid day of week: {dow}"));
    })
}

#[cfg(test)]
mod test {
    use super::{parse_day_of_week, DayOfWeek};
    use anyhow::{anyhow, Result};

    #[test]
    fn simple() -> Result<()> {
        assert_eq!(DayOfWeek::Days(vec![0]), parse_day_of_week("Sun")?);
        assert_eq!(DayOfWeek::Days(vec![0]), parse_day_of_week("Sunday")?);
        Ok(())
    }

    #[test]
    fn range() -> Result<()> {
        assert_eq!(
            DayOfWeek::Days(vec![1, 2, 3, 4, 5]),
            parse_day_of_week("Mon..Fri")?
        );
        assert_eq!(
            DayOfWeek::Days(vec![1, 2, 3, 4, 5]),
            parse_day_of_week("Monday..Friday")?
        );
        Ok(())
    }

    #[test]
    fn all() -> Result<()> {
        assert_eq!(DayOfWeek::All, parse_day_of_week("*")?);
        Ok(())
    }

    #[test]
    fn multiple() -> Result<()> {
        assert_eq!(
            DayOfWeek::Days(vec![0, 2, 4, 6]),
            parse_day_of_week("Sun,Tue,Thu,Sat")?
        );
        assert_eq!(
            DayOfWeek::Days(vec![0, 2, 4, 6]),
            parse_day_of_week("Sunday,Tuesday,Thursday,Saturday")?
        );
        Ok(())
    }

    #[test]
    fn day_already_in_range() -> Result<()> {
        assert_eq!(
            DayOfWeek::Days(vec![1, 2, 3, 4, 5]),
            parse_day_of_week("Mon..Fri,Tue")?
        );
        assert_eq!(
            DayOfWeek::Days(vec![1, 2, 3, 4, 5]),
            parse_day_of_week("Monday..Friday,Tuesday")?
        );
        Ok(())
    }

    #[test]
    fn one_day_range() -> Result<()> {
        assert_eq!(
            DayOfWeek::Days(vec![1, 5]),
            parse_day_of_week("Mon..Mon,Fri..Fri")?
        );
        assert_eq!(
            DayOfWeek::Days(vec![1, 5]),
            parse_day_of_week("Monday..Monday,Friday..Friday")?
        );
        Ok(())
    }

    #[test]
    fn funky() -> Result<()> {
        assert_eq!(
            DayOfWeek::Days(vec![0, 1, 2, 3, 4, 6]),
            parse_day_of_week("Mon..Thu,Sat,Sun")?
        );
        assert_eq!(
            DayOfWeek::Days(vec![0, 1, 2, 3, 4, 6]),
            parse_day_of_week("Monday..Thursday,Saturday,Sunday")?
        );
        assert_eq!(
            DayOfWeek::Days(vec![0, 1, 2, 3, 4, 6]),
            parse_day_of_week("Mon..Thursday,SAt,SuNdaY")?
        );
        Ok(())
    }

    #[test]
    fn invalid() -> Result<()> {
        match parse_day_of_week("Hogwash,Wed") {
            Ok(_) => Err(anyhow!("this day of week should be invalid")),
            Err(e) => {
                assert_eq!(format!("{e}"), "invalid day of week: Hogwash");
                Ok(())
            }
        }
    }

    #[test]
    fn invalid_range() -> Result<()> {
        match parse_day_of_week("Mon..Hogwash,Wed") {
            Ok(_) => Err(anyhow!("this day of week should be invalid")),
            Err(e) => {
                assert_eq!(format!("{e}"), "invalid day of week: Hogwash");
                Ok(())
            }
        }
    }

    #[test]
    fn invalid_range_order() -> Result<()> {
        match parse_day_of_week("Fri..Mon") {
            Ok(_) => Err(anyhow!("this day of week should be invalid")),
            Err(e) => {
                assert_eq!(format!("{e}"), "invalid range: 'Fri..Mon'");
                Ok(())
            }
        }
    }
}
