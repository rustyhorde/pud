// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// realtime yyyy-mm-dd helpers

use super::{parse_time_chunk, All, RANGE_RE};
use crate::error::Error::InvalidDate;
use anyhow::{anyhow, Result};
use rand::Rng;

const MONTHS_PER_YEAR: u8 = 12;
// TODO: Fix this
const DAYS_PER_MONTH: u8 = 31;

/// The year for a realtime schedule
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Year {
    /// Every year
    All,
    /// A range of years
    Range(i32, i32),
    /// A repetition of years
    ///
    /// This is a sequence of years: start, start + rep, start + 2*rep
    /// up to the optional end year.
    Repetition {
        /// The year to start
        start: i32,
        /// An optional end year
        end: Option<i32>,
        /// The repetition value
        rep: u8,
    },
    /// Specific years
    Year(i32),
}

impl Year {
    pub(crate) fn matches(&self, given: i32) -> bool {
        match self {
            Year::All => true,
            Year::Range(lo, hi) => *lo <= given && given <= *hi,
            Year::Repetition { start, end, rep } => if let Some(end) = end {
                *start..=*end
            } else {
                *start..=9999
            }
            .step_by(usize::from(*rep))
            .any(|x| x == given),
            Year::Year(year) => *year == given,
        }
    }
}

/// The month for a realtime schedule
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Month {
    /// Every month
    All,
    /// Specific months
    Months(Vec<u8>),
}

impl Month {
    pub(crate) fn matches(&self, given: u8) -> bool {
        match self {
            Month::All => true,
            Month::Months(months) => months.contains(&given),
        }
    }
}

impl All for Month {
    fn all() -> Self {
        Self::All
    }

    fn rand() -> Self {
        let mut rng = rand::rng();
        let rand_in_range = rng.random_range(1..13);
        Month::Months(vec![rand_in_range])
    }
}

impl From<Vec<u8>> for Month {
    fn from(value: Vec<u8>) -> Self {
        Month::Months(value)
    }
}

impl From<u8> for Month {
    fn from(value: u8) -> Self {
        Month::Months(vec![value])
    }
}

/// The date for a realtime schedule
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Day {
    /// Every day
    All,
    /// Specific days
    Days(Vec<u8>),
}

impl Day {
    pub(crate) fn matches(&self, given: u8) -> bool {
        match self {
            Day::All => true,
            Day::Days(days) => days.contains(&given),
        }
    }
}

impl All for Day {
    fn all() -> Self {
        Self::All
    }

    fn rand() -> Self {
        let mut rng = rand::rng();
        let rand_in_range = rng.random_range(1..29);
        Day::Days(vec![rand_in_range])
    }
}

impl From<Vec<u8>> for Day {
    fn from(value: Vec<u8>) -> Self {
        Day::Days(value)
    }
}

impl From<u8> for Day {
    fn from(value: u8) -> Self {
        Day::Days(vec![value])
    }
}

pub(crate) fn parse_date(ymd: &str) -> Result<(Year, Month, Day)> {
    let date_parts: Vec<&str> = ymd.split('-').collect();
    if date_parts.len() == 3 {
        let year = parse_year(date_parts[0])?;
        let month = parse_time_chunk::<Month>(date_parts[1], MONTHS_PER_YEAR, true)?;
        let day = parse_time_chunk::<Day>(date_parts[2], DAYS_PER_MONTH, true)?;
        Ok((year, month, day))
    } else {
        Err(InvalidDate {
            date: ymd.to_string(),
        }
        .into())
    }
}

fn parse_year(yearish: &str) -> Result<Year> {
    Ok(if yearish == "*" {
        Year::All
    } else if RANGE_RE.is_match(yearish) {
        let caps = RANGE_RE.captures(yearish).ok_or_else(|| anyhow!(""))?;
        let first = caps
            .get(1)
            .ok_or_else(|| anyhow!(""))?
            .as_str()
            .parse::<i32>()?;
        let second = caps
            .get(2)
            .ok_or_else(|| anyhow!(""))?
            .as_str()
            .parse::<i32>()?;
        Year::Range(first, second)
    } else {
        Year::Year(yearish.parse::<i32>()?)
    })
}

#[cfg(test)]
mod test {
    use super::{parse_date, Day, Month, Year, DAYS_PER_MONTH, MONTHS_PER_YEAR};
    use anyhow::Result;

    #[test]
    fn simple() -> Result<()> {
        let (year, month, day) = parse_date("1976-03-22")?;
        assert_eq!(year, Year::Year(1976));
        assert_eq!(month, Month::Months(vec![3]));
        assert_eq!(day, Day::Days(vec![22]));
        Ok(())
    }

    #[test]
    fn range() -> Result<()> {
        let (year, month, day) = parse_date("1976-03..07-10..20")?;
        assert_eq!(year, Year::Year(1976));
        assert_eq!(month, Month::Months((3..=7).collect()));
        assert_eq!(day, Day::Days((10..=20).collect()));
        Ok(())
    }

    #[test]
    fn simple_repetition() -> Result<()> {
        let (year, month, day) = parse_date("1976-01/2-01/3")?;
        assert_eq!(year, Year::Year(1976));
        assert_eq!(
            month,
            Month::Months((1..MONTHS_PER_YEAR).step_by(2).collect())
        );
        assert_eq!(day, Day::Days((1..DAYS_PER_MONTH).step_by(3).collect()));
        Ok(())
    }

    #[test]
    fn range_repetition() -> Result<()> {
        let (year, month, day) = parse_date("1976-03..09/2-10..20/3")?;
        assert_eq!(year, Year::Year(1976));
        assert_eq!(month, Month::Months((3..=9).step_by(2).collect()));
        assert_eq!(day, Day::Days((10..=20).step_by(3).collect()));
        Ok(())
    }

    #[test]
    fn funky() -> Result<()> {
        let (year, month, day) = parse_date("1976-01,03..09/2,10..12-10..20/3")?;
        assert_eq!(year, Year::Year(1976));
        assert_eq!(month, Month::Months(vec![1, 3, 5, 7, 9, 10, 11, 12]));
        assert_eq!(day, Day::Days((10..=20).step_by(3).collect()));
        Ok(())
    }

    #[test]
    fn year_matching_works() {
        let years = Year::Range(2022, 2024);
        assert!(!years.matches(2021));
        assert!(years.matches(2022));
        assert!(years.matches(2023));
        assert!(years.matches(2024));
        assert!(!years.matches(2025));
    }

    #[test]
    fn month_matching_works() {
        let months = Month::Months(vec![1, 3, 7]);
        assert!(months.matches(1));
        assert!(!months.matches(2));
        assert!(months.matches(3));
        assert!(!months.matches(4));
        assert!(!months.matches(5));
        assert!(!months.matches(6));
        assert!(months.matches(7));
        assert!(!months.matches(8));
        assert!(!months.matches(9));
        assert!(!months.matches(10));
        assert!(!months.matches(11));
        assert!(!months.matches(12));
    }

    #[test]
    fn day_matching_works() {
        let days = Day::Days(vec![10, 11, 12]);
        assert!(!days.matches(9));
        assert!(days.matches(10));
        assert!(days.matches(11));
        assert!(days.matches(12));
        assert!(!days.matches(13));
    }
}
