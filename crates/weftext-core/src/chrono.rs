use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl CalendarDate {
    /// Construct a validated Gregorian calendar date.
    ///
    /// # Errors
    ///
    /// Returns [`CalendarDateError`] when the year, month, or day is outside the
    /// Gregorian calendar range represented by this type.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, CalendarDateError> {
        if year < 1 || !(1..=12).contains(&month) {
            return Err(CalendarDateError);
        }
        let maximum = days_in_month(year, month);
        if day == 0 || day > maximum {
            return Err(CalendarDateError);
        }
        Ok(Self { year, month, day })
    }

    #[must_use]
    pub fn quarter(self) -> u8 {
        (self.month - 1) / 3 + 1
    }

    #[must_use]
    pub fn iso_week(self) -> (i32, u8) {
        let ordinal = ordinal_day(self.year, self.month, self.day);
        let weekday = weekday_monday_one(self.year, self.month, self.day);
        let mut week = (i32::from(ordinal) - i32::from(weekday) + 10) / 7;
        let mut week_year = self.year;
        if week < 1 {
            week_year -= 1;
            week = i32::from(weeks_in_iso_year(week_year));
        } else if week > i32::from(weeks_in_iso_year(self.year)) {
            week_year += 1;
            week = 1;
        }
        let week = u8::try_from(week).unwrap_or(1);
        (week_year, week)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalendarDateError;

impl fmt::Display for CalendarDateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid Gregorian calendar date")
    }
}

impl std::error::Error for CalendarDateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChronoPeriod {
    Year,
    Quarter,
    Month,
    Week,
    Day,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChronoNodePlan {
    pub period: ChronoPeriod,
    pub name: String,
    pub relative_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChronoPlan {
    pub date: CalendarDate,
    pub nodes: Vec<ChronoNodePlan>,
}

impl ChronoPlan {
    #[must_use]
    pub fn build(date: CalendarDate, enabled: &[ChronoPeriod]) -> Self {
        let year = format!("{:04}", date.year);
        let (week_year, week) = date.iso_week();
        let mut nodes = Vec::new();
        for period in enabled {
            let name = match period {
                ChronoPeriod::Year => year.clone(),
                ChronoPeriod::Quarter => format!("{}-Q{}", date.year, date.quarter()),
                ChronoPeriod::Month => format!("{}-{:02}", date.year, date.month),
                ChronoPeriod::Week => format!("{week_year:04}-W{week:02}"),
                ChronoPeriod::Day => format!("{}-{:02}-{:02}", date.year, date.month, date.day),
            };
            let relative_path = if *period == ChronoPeriod::Year {
                PathBuf::from(&name)
            } else {
                PathBuf::from(&year).join(&name)
            };
            nodes.push(ChronoNodePlan {
                period: *period,
                name,
                relative_path,
            });
        }
        Self { date, nodes }
    }
}

const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn ordinal_day(year: i32, month: u8, day: u8) -> u16 {
    let before = (1..month)
        .map(|candidate| u16::from(days_in_month(year, candidate)))
        .sum::<u16>();
    before + u16::from(day)
}

fn weekday_monday_one(year: i32, month: u8, day: u8) -> u8 {
    let mut adjusted_year = year;
    if month < 3 {
        adjusted_year -= 1;
    }
    let table = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let sunday_zero = (adjusted_year + adjusted_year / 4 - adjusted_year / 100
        + adjusted_year / 400
        + table[usize::from(month - 1)]
        + i32::from(day))
    .rem_euclid(7);
    u8::try_from((sunday_zero + 6) % 7 + 1).expect("weekday is between 1 and 7")
}

fn weeks_in_iso_year(year: i32) -> u8 {
    let january_first = weekday_monday_one(year, 1, 1);
    if january_first == 4 || (january_first == 3 && is_leap_year(year)) {
        53
    } else {
        52
    }
}
