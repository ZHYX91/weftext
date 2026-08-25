use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::CalendarDate;

/// A literal task-node calendar date or explicit-offset RFC 3339 instant.
///
/// Serde represents both variants as their canonical source string so the wire shape follows the
/// task-node v1 decoded-field schema while Rust callers retain the date/instant distinction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum TaskNodeTemporal {
    Date(String),
    Instant(String),
}

impl TaskNodeTemporal {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Date(value) | Self::Instant(value) => value,
        }
    }

    /// Parses one exact, timezone-independent task-node temporal value.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid calendar dates, instants without an explicit offset, lowercase
    /// time separators, leap seconds, or any non-canonical spelling.
    pub fn parse(value: &str) -> Result<Self, TaskNodeTemporalError> {
        if valid_calendar_date(value) {
            return Ok(Self::Date(value.to_owned()));
        }
        if valid_rfc3339_instant(value) {
            return Ok(Self::Instant(value.to_owned()));
        }
        Err(TaskNodeTemporalError)
    }
}

impl FromStr for TaskNodeTemporal {
    type Err = TaskNodeTemporalError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<String> for TaskNodeTemporal {
    type Error = TaskNodeTemporalError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<TaskNodeTemporal> for String {
    fn from(value: TaskNodeTemporal) -> Self {
        match value {
            TaskNodeTemporal::Date(value) | TaskNodeTemporal::Instant(value) => value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskNodeTemporalError;

impl fmt::Display for TaskNodeTemporalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "task-node temporal value must be an ISO date or explicit-offset RFC 3339 instant",
        )
    }
}

impl std::error::Error for TaskNodeTemporalError {}

fn valid_calendar_date(value: &str) -> bool {
    if value.len() != 10
        || !value.is_ascii()
        || value.as_bytes()[4] != b'-'
        || value.as_bytes()[7] != b'-'
    {
        return false;
    }
    let Ok(year) = value[0..4].parse::<i32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    CalendarDate::new(year, month, day).is_ok()
}

fn valid_rfc3339_instant(value: &str) -> bool {
    if value.len() < 20
        || !value.is_ascii()
        || !valid_calendar_date(&value[..10])
        || value.as_bytes()[10] != b'T'
    {
        return false;
    }
    let time = &value[11..];
    if time.len() < 9 || time.as_bytes()[2] != b':' || time.as_bytes()[5] != b':' {
        return false;
    }
    let Ok(hour) = time[0..2].parse::<u8>() else {
        return false;
    };
    let Ok(minute) = time[3..5].parse::<u8>() else {
        return false;
    };
    let Ok(second) = time[6..8].parse::<u8>() else {
        return false;
    };
    if hour > 23 || minute > 59 || second > 59 {
        return false;
    }

    let mut offset_start = 8;
    if time.as_bytes().get(offset_start) == Some(&b'.') {
        offset_start += 1;
        let fraction_start = offset_start;
        while time
            .as_bytes()
            .get(offset_start)
            .is_some_and(u8::is_ascii_digit)
        {
            offset_start += 1;
        }
        if offset_start == fraction_start {
            return false;
        }
    }
    if time.get(offset_start..) == Some("Z") {
        return true;
    }
    let Some(offset) = time.get(offset_start..) else {
        return false;
    };
    if offset.len() != 6
        || !matches!(offset.as_bytes()[0], b'+' | b'-')
        || offset.as_bytes()[3] != b':'
    {
        return false;
    }
    offset[1..3]
        .parse::<u8>()
        .is_ok_and(|offset_hour| offset_hour <= 23)
        && offset[4..6]
            .parse::<u8>()
            .is_ok_and(|offset_minute| offset_minute <= 59)
}
