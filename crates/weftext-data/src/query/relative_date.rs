use super::{DataError, Result, UiMessage};
use chrono::{DateTime, Datelike, Days, Months, NaiveDate, SecondsFormat, TimeZone, Utc};
use chrono_tz::{GapInfo, Tz};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryRelativeDate {
    pub field: String,
    pub date_kind: QueryDateKind,
    pub time_zone: String,
    pub period: QueryDatePeriod,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueryDateKind {
    Date,
    Instant,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueryPeriodUnit {
    Week,
    Month,
    Quarter,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum QueryDatePeriod {
    Today,
    Yesterday,
    Current { unit: QueryPeriodUnit },
    Previous { unit: QueryPeriodUnit },
    Past { days: u16 },
    Future { days: u16 },
    Between { start: String, end: String },
}

fn invalid_period() -> DataError {
    DataError::new(UiMessage::error_data_query_relative_date_period())
}

impl QueryRelativeDate {
    pub(super) fn cel(&self, reference: DateTime<Utc>) -> Result<String> {
        if self.field.is_empty() || self.field.len() > 256 || self.time_zone.len() > 128 {
            return Err(invalid_period());
        }
        let zone: Tz = self
            .time_zone
            .parse()
            .map_err(|_| DataError::new(UiMessage::error_data_query_relative_date_zone()))?;
        let today = reference.with_timezone(&zone).date_naive();
        let (start, end) = self.period.range(today)?;
        let literal = |date: NaiveDate| -> Result<String> {
            if !(1..=9999).contains(&date.year()) {
                return Err(invalid_period());
            }
            Ok(match self.date_kind {
                QueryDateKind::Date => format!("date(\"{date}\")"),
                QueryDateKind::Instant => {
                    let midnight = date.and_hms_opt(0, 0, 0).ok_or_else(invalid_period)?;
                    let boundary = zone
                        .from_local_datetime(&midnight)
                        .earliest()
                        .or_else(|| GapInfo::new(&midnight, &zone)?.end)
                        .ok_or_else(invalid_period)?
                        .with_timezone(&Utc)
                        .to_rfc3339_opts(SecondsFormat::Secs, true);
                    format!("instant(\"{boundary}\")")
                }
            })
        };
        let field = format!("field({})", serde_json::to_string(&self.field)?);
        Ok(format!(
            "!isAbsent({field}) && !isNull({field}) && {field} >= {} && {field} < {}",
            literal(start)?,
            literal(end)?
        ))
    }
}

impl QueryDatePeriod {
    fn range(&self, today: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
        let offset = |date: NaiveDate, days: i32| {
            let amount = Days::new(days.unsigned_abs().into());
            if days < 0 {
                date.checked_sub_days(amount)
            } else {
                date.checked_add_days(amount)
            }
            .ok_or_else(invalid_period)
        };
        match self {
            Self::Today => Ok((today, offset(today, 1)?)),
            Self::Yesterday => Ok((offset(today, -1)?, today)),
            Self::Past { days } if (1..=3660).contains(days) => {
                Ok((offset(today, -i32::from(*days))?, today))
            }
            Self::Future { days } if (1..=3660).contains(days) => {
                Ok((offset(today, 1)?, offset(today, i32::from(*days) + 1)?))
            }
            Self::Current { unit } | Self::Previous { unit } => {
                let previous = matches!(self, Self::Previous { .. });
                let months = match unit {
                    QueryPeriodUnit::Week => {
                        let monday =
                            offset(today, -(today.weekday().num_days_from_monday() as i32))?;
                        return if previous {
                            Ok((offset(monday, -7)?, monday))
                        } else {
                            Ok((monday, offset(monday, 7)?))
                        };
                    }
                    QueryPeriodUnit::Month => 1,
                    QueryPeriodUnit::Quarter => 3,
                    QueryPeriodUnit::Year => 12,
                };
                let month = today.month0() / months * months + 1;
                let start =
                    NaiveDate::from_ymd_opt(today.year(), month, 1).ok_or_else(invalid_period)?;
                let span = Months::new(months);
                if previous {
                    Ok((
                        start.checked_sub_months(span).ok_or_else(invalid_period)?,
                        start,
                    ))
                } else {
                    Ok((
                        start,
                        start.checked_add_months(span).ok_or_else(invalid_period)?,
                    ))
                }
            }
            Self::Between { start, end } => {
                let iso = |value: &str| {
                    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
                        .map_err(|_| invalid_period())?;
                    if date.to_string() != value {
                        return Err(invalid_period());
                    }
                    Ok(date)
                };
                let start = iso(start)?;
                let end = iso(end)?;
                if start > end {
                    return Err(invalid_period());
                }
                Ok((start, offset(end, 1)?))
            }
            _ => Err(invalid_period()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_day_ranges_use_the_actual_midnights_across_clock_changes() {
        let condition = QueryRelativeDate {
            field: "events/instant".into(),
            date_kind: QueryDateKind::Instant,
            time_zone: "America/New_York".into(),
            period: QueryDatePeriod::Today,
        };
        for (reference, start, end) in [
            (
                "2024-03-10T12:00:00Z",
                "2024-03-10T05:00:00Z",
                "2024-03-11T04:00:00Z",
            ),
            (
                "2024-11-03T12:00:00Z",
                "2024-11-03T04:00:00Z",
                "2024-11-04T05:00:00Z",
            ),
        ] {
            let expression = condition.cel(reference.parse().unwrap()).unwrap();
            assert!(expression.contains(start));
            assert!(expression.contains(end));
        }
    }
}
