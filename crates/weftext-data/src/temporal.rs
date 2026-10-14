use crate::*;
use chrono::{Datelike, Days, NaiveDate, Weekday};
use std::cmp::Ordering;

fn member_text<'a>(v: &'a NativeValue, key: &str) -> Result<&'a str> {
    v.member(key)
        .and_then(NativeValue::text)
        .ok_or_else(|| DataError(format!("{key}: required text")))
}
fn date_string(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}
fn native_range(start: NaiveDate, end: NaiveDate) -> NativeValue {
    NativeValue::Record(vec![
        NativeMember {
            key: "start".into(),
            value: NativeValue::Date(date_string(start)),
        },
        NativeMember {
            key: "end_exclusive".into(),
            value: NativeValue::Date(date_string(end)),
        },
    ])
}
fn civil(year: i32, month: u32, day: u32) -> Result<NaiveDate> {
    if !(1..=9999).contains(&year) {
        return Err(DataError("Gregorian year outside installed bounds".into()));
    }
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| DataError("Invalid Gregorian civil date".into()))
}
fn plus_days(date: NaiveDate, n: u64) -> Result<NaiveDate> {
    let d = date
        .checked_add_days(Days::new(n))
        .ok_or_else(|| DataError("Civil date overflow".into()))?;
    if d.year() > 9999 {
        return Err(DataError(
            "Period endpoint outside installed Gregorian coverage".into(),
        ));
    }
    Ok(d)
}
/// Registered Gregorian ISO 8601 periods have civil-date half-open bounds.
/// Series/calendar/zone versions come exclusively from installed configuration.
pub fn period_bounds(period: &NativeValue, schema: &Schema) -> Result<NativeValue> {
    let (start, end) = period_dates(period, schema)?;
    Ok(native_range(start, end))
}
/// Central admission outcome used by the schema and callable service. Provider
/// absence/version/coverage is unavailable; malformed authored keys are invalid.
pub fn validate_period(period: &NativeValue, schema: &Schema) -> Cell {
    for key in [
        "series_key",
        "kind",
        "calendar",
        "calendar_version",
        "rule",
        "key",
        "zone",
        "zone_version",
    ] {
        if period
            .member(key)
            .and_then(NativeValue::text)
            .is_none_or(str::is_empty)
        {
            return invalid(format!("{key}: required nonempty text"));
        }
    }
    match period_dates(period, schema) {
        Ok(_) => Cell::value(NativeValue::Bool(true)),
        Err(e) if e.0.contains("unavailable") || e.0.contains("coverage") => issue(e.0),
        Err(e) => invalid(e.0),
    }
}
pub fn period_step(period: &NativeValue, steps: i32, schema: &Schema) -> Result<NativeValue> {
    let (start, _) = period_dates(period, schema)?;
    let (policy, calendar, _) = period_providers(period, schema)?;
    let kind = member_text(period, "kind")?;
    let model = CalendarModel::new(calendar)?;
    let date = model.step_period(start, kind, steps)?;
    period_for_date(
        &policy.key,
        kind,
        &NativeValue::Date(date_string(date)),
        schema,
    )
}
fn period_providers<'a>(
    period: &NativeValue,
    schema: &'a Schema,
) -> Result<(&'a SeriesPolicy, &'a Provider, &'a Provider)> {
    let series = member_text(period, "series_key")?;
    let policy = schema
        .series_policies
        .iter()
        .find(|p| p.key == series)
        .ok_or_else(|| DataError("Series policy unavailable".into()))?;
    let kind = member_text(period, "kind")?;
    if !policy.kinds.iter().any(|k| k == kind) || member_text(period, "rule")? != policy.rule {
        return Err(DataError(
            "Period kind/rule conflicts with series policy".into(),
        ));
    }
    let calendar = schema
        .providers
        .iter()
        .find(|p| p.id == policy.calendar)
        .ok_or_else(|| DataError("Series calendar provider unavailable".into()))?;
    let zone = schema
        .providers
        .iter()
        .find(|p| p.id == policy.zone)
        .ok_or_else(|| DataError("Series zone provider unavailable".into()))?;
    if calendar.data["key"].as_str() != Some(member_text(period, "calendar")?)
        || zone.data["key"].as_str() != Some(member_text(period, "zone")?)
    {
        return Err(DataError(
            "Period calendar/zone keys conflict with series policy".into(),
        ));
    }
    if calendar.data["version"].as_str() != Some(member_text(period, "calendar_version")?)
        || zone.data["version"].as_str() != Some(member_text(period, "zone_version")?)
    {
        return Err(DataError(
            "Period frozen provider versions unavailable under series policy".into(),
        ));
    }
    Ok((policy, calendar, zone))
}
fn period_dates(period: &NativeValue, schema: &Schema) -> Result<(NaiveDate, NaiveDate)> {
    let (policy, calendar, _) = period_providers(period, schema)?;
    let kind = member_text(period, "kind")?;
    if calendar.implementation.id == "weftext.calendar.table.v2"
        && policy.rule == "table-ordinal-v1"
    {
        return CalendarModel::new(calendar)?.table_period(kind, member_text(period, "key")?);
    }
    if calendar.implementation.id != "weftext.calendar.gregorian.v2" || policy.rule != "iso8601" {
        return Err(DataError(
            "Selected calendar period executor unavailable".into(),
        ));
    }
    let key = member_text(period, "key")?;
    if !key.is_ascii() {
        return Err(DataError(
            "Gregorian period key must be canonical ASCII".into(),
        ));
    }
    let bounds = match kind {
        "day" => {
            let d = NaiveDate::parse_from_str(key, "%Y-%m-%d")
                .map_err(|_| DataError("Invalid day period key".into()))?;
            if date_string(d) != key {
                return Err(DataError("Noncanonical day key".into()));
            }
            (civil(d.year(), d.month(), d.day())?, plus_days(d, 1)?)
        }
        "week" => {
            let (y, w) = key
                .split_once("-W")
                .ok_or_else(|| DataError("Expected YYYY-Www".into()))?;
            let y = y
                .parse::<i32>()
                .map_err(|_| DataError("Invalid week year".into()))?;
            let w = w
                .parse::<u32>()
                .map_err(|_| DataError("Invalid ISO week".into()))?;
            if key != format!("{y:04}-W{w:02}") || !(1..=9999).contains(&y) {
                return Err(DataError("Noncanonical ISO week key".into()));
            }
            let start = NaiveDate::from_isoywd_opt(y, w, Weekday::Mon)
                .ok_or_else(|| DataError("Invalid ISO week including week-53".into()))?;
            (
                civil(start.year(), start.month(), start.day())?,
                plus_days(start, 7)?,
            )
        }
        "month" => {
            let (y, m) = key
                .split_once('-')
                .ok_or_else(|| DataError("Expected YYYY-MM".into()))?;
            let y = y
                .parse::<i32>()
                .map_err(|_| DataError("Invalid year".into()))?;
            let m = m
                .parse::<u32>()
                .map_err(|_| DataError("Invalid month".into()))?;
            if key != format!("{y:04}-{m:02}") {
                return Err(DataError("Noncanonical month key".into()));
            }
            let start = civil(y, m, 1)?;
            let end = if m == 12 {
                civil(y + 1, 1, 1)?
            } else {
                civil(y, m + 1, 1)?
            };
            (start, end)
        }
        "quarter" => {
            let (y, q) = key
                .split_once("-Q")
                .ok_or_else(|| DataError("Expected YYYY-Qq".into()))?;
            let y = y
                .parse::<i32>()
                .map_err(|_| DataError("Invalid year".into()))?;
            let q = q
                .parse::<u32>()
                .map_err(|_| DataError("Invalid quarter".into()))?;
            if key != format!("{y:04}-Q{q}") || !(1..=4).contains(&q) {
                return Err(DataError("Noncanonical quarter key".into()));
            }
            let start = civil(y, (q - 1) * 3 + 1, 1)?;
            let end = if q == 4 {
                civil(y + 1, 1, 1)?
            } else {
                civil(y, q * 3 + 1, 1)?
            };
            (start, end)
        }
        "year" => {
            let y = key
                .parse::<i32>()
                .map_err(|_| DataError("Invalid year key".into()))?;
            if key != format!("{y:04}") {
                return Err(DataError("Noncanonical year key".into()));
            }
            (civil(y, 1, 1)?, civil(y + 1, 1, 1)?)
        }
        _ => return Err(DataError("Unknown period kind".into())),
    };
    let min = calendar.data["min_year"].as_i64().unwrap_or(1);
    let max = calendar.data["max_year"].as_i64().unwrap_or(9999);
    if i64::from(bounds.0.year()) < min || i64::from(bounds.1.year()) > max {
        return Err(DataError(
            "Period crosses installed calendar coverage".into(),
        ));
    }
    // Civil bounds retain dates; the zone is a dependency, never an implicit
    // conversion to instants or the machine's local timezone.
    Ok(bounds)
}

/// Exact RFC3339 comparison with arbitrary fractional-second precision.
#[derive(Debug, Clone)]
pub struct ExactInstant {
    pub utc_seconds: i64,
    pub fraction: ExactDecimal,
}
impl ExactInstant {
    pub fn canonical(&self) -> Result<String> {
        let dt = chrono::DateTime::from_timestamp(self.utc_seconds, 0)
            .ok_or_else(|| DataError("Instant outside supported bounds".into()))?;
        if !(1..=9999).contains(&dt.year()) {
            return Err(DataError("Instant outside supported bounds".into()));
        }
        let fraction = self.fraction.spelling();
        let tail = fraction
            .split_once('.')
            .map_or("", |(_, f)| f)
            .trim_end_matches('0');
        Ok(format!(
            "{}{}Z",
            dt.format("%Y-%m-%dT%H:%M:%S"),
            if tail.is_empty() {
                String::new()
            } else {
                format!(".{tail}")
            }
        ))
    }
    pub(crate) fn shifted(&self, seconds: i64, fraction: &ExactDecimal) -> Result<Self> {
        let zero = ExactDecimal::parse("0")?;
        let one = ExactDecimal::parse("1")?;
        let mut utc_seconds = self
            .utc_seconds
            .checked_add(seconds)
            .ok_or_else(|| DataError("Instant arithmetic overflow".into()))?;
        let mut value = self.fraction.add(fraction);
        if value.compare(&zero).is_lt() {
            utc_seconds = utc_seconds
                .checked_sub(1)
                .ok_or_else(|| DataError("Instant arithmetic overflow".into()))?;
            value = value.add(&one);
        } else if !value.compare(&one).is_lt() {
            utc_seconds = utc_seconds
                .checked_add(1)
                .ok_or_else(|| DataError("Instant arithmetic overflow".into()))?;
            value = value.add(&ExactDecimal::parse("-1")?);
        }
        let result = Self {
            utc_seconds,
            fraction: value,
        };
        result.canonical()?;
        Ok(result)
    }
    pub fn parse(s: &str) -> Result<Self> {
        if !s.is_ascii() || s.len() < 20 || s.ends_with("-00:00") {
            return Err(DataError(
                "Instant requires a known explicit RFC3339 offset".into(),
            ));
        }
        let (date, time) = s
            .split_once('T')
            .ok_or_else(|| DataError("Instant needs RFC3339 T".into()))?;
        let date_text = date;
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| DataError("Invalid instant civil date".into()))?;
        if !(1..=9999).contains(&date.year()) || date_string(date) != date_text {
            return Err(DataError(
                "Instant date outside supported Gregorian bounds".into(),
            ));
        }
        let (time, offset) = if let Some(t) = time.strip_suffix('Z') {
            (t, 0i64)
        } else {
            if time.len() < 14 {
                return Err(DataError("Missing instant offset".into()));
            }
            let at = time.len() - 6;
            let (t, z) = time.split_at(at);
            let sign = match z.as_bytes()[0] {
                b'+' => 1,
                b'-' => -1,
                _ => return Err(DataError("Invalid instant offset".into())),
            };
            if z.as_bytes()[3] != b':'
                || !z[1..3].bytes().all(|b| b.is_ascii_digit())
                || !z[4..6].bytes().all(|b| b.is_ascii_digit())
            {
                return Err(DataError("Invalid instant offset".into()));
            }
            let h = z[1..3]
                .parse::<i64>()
                .map_err(|_| DataError("Invalid offset hour".into()))?;
            let m = z[4..6]
                .parse::<i64>()
                .map_err(|_| DataError("Invalid offset minute".into()))?;
            if h > 23 || m > 59 {
                return Err(DataError("Invalid instant offset".into()));
            }
            (t, sign * (h * 3600 + m * 60))
        };
        let (clock, fraction) = if let Some((clock, fraction)) = time.split_once('.') {
            if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
                return Err(DataError("Invalid fractional seconds".into()));
            }
            (clock, ExactDecimal::parse(&format!("0.{fraction}"))?)
        } else {
            (time, ExactDecimal::parse("0")?)
        };
        if clock.len() != 8 || clock.as_bytes()[2] != b':' || clock.as_bytes()[5] != b':' {
            return Err(DataError("Invalid instant clock".into()));
        }
        let time = chrono::NaiveTime::parse_from_str(clock, "%H:%M:%S")
            .map_err(|_| DataError("Invalid instant clock or leap second".into()))?;
        use chrono::Timelike;
        if time.second() > 59 || time.nanosecond() != 0 {
            return Err(DataError("Leap seconds unsupported".into()));
        }
        let utc_seconds = date
            .and_time(time)
            .and_utc()
            .timestamp()
            .checked_sub(offset)
            .ok_or_else(|| DataError("Instant overflow".into()))?;
        Ok(Self {
            utc_seconds,
            fraction,
        })
    }
    pub fn compare(&self, other: &Self) -> Ordering {
        self.utc_seconds
            .cmp(&other.utc_seconds)
            .then_with(|| self.fraction.compare(&other.fraction))
    }
}

#[derive(Debug, Clone)]
enum Endpoint {
    Civil {
        calendar: String,
        version: String,
        precision: String,
        ordinal: i64,
        lo: i64,
        hi: i64,
    },
    Instant(ExactInstant),
}
fn issue(message: impl Into<String>) -> Cell {
    Cell::Unavailable {
        reason: message.into(),
    }
}
fn invalid(message: impl Into<String>) -> Cell {
    Cell::Invalid {
        diagnostics: vec![message.into()],
        value: None,
    }
}
fn endpoint(value: &NativeValue, schema: &Schema) -> std::result::Result<Endpoint, Cell> {
    match value {
        NativeValue::Date(s) => {
            let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| invalid("Invalid civil date"))?;
            if !(1..=9999).contains(&d.year()) || date_string(d) != *s {
                return Err(invalid("Civil date outside Gregorian bounds"));
            }
            let ordinal = i64::from(d.num_days_from_ce());
            Ok(Endpoint::Civil {
                calendar: "gregorian".into(),
                version: "proleptic-gregorian.v1".into(),
                precision: "day".into(),
                ordinal,
                lo: ordinal,
                hi: ordinal,
            })
        }
        NativeValue::Text(s) => ExactInstant::parse(s)
            .map(Endpoint::Instant)
            .map_err(|e| invalid(e.0)),
        NativeValue::Record(_) => {
            let calendar = value
                .member("calendar")
                .and_then(NativeValue::text)
                .ok_or_else(|| invalid("CalendarDate calendar absent"))?;
            let version = value
                .member("version")
                .and_then(NativeValue::text)
                .ok_or_else(|| invalid("CalendarDate version absent"))?;
            let precision = value
                .member("precision")
                .and_then(NativeValue::text)
                .ok_or_else(|| invalid("CalendarDate precision absent"))?;
            let year = value
                .member("year")
                .and_then(NativeValue::integer)
                .ok_or_else(|| invalid("CalendarDate year absent"))?;
            let month = value.member("month").and_then(NativeValue::integer);
            let day = value.member("day").and_then(NativeValue::integer);
            if value.member("month").is_some() && month.is_none()
                || value.member("day").is_some() && day.is_none()
                || value
                    .member("leap_month")
                    .is_some_and(|v| !matches!(v, NativeValue::Bool(_)))
            {
                return Err(invalid("Precision date component type mismatch"));
            }
            let provider = schema
                .providers
                .iter()
                .find(|p| {
                    matches!(
                        p.data["kind"].as_str(),
                        Some("gregorian" | "calendar-table")
                    ) && p.data["key"].as_str() == Some(calendar)
                        && p.data["version"].as_str() == Some(version)
                })
                .ok_or_else(|| issue("Exact calendar provider unavailable"))?;
            if !matches!(precision, "year" | "month" | "day")
                || precision == "year" && (month.is_some() || day.is_some())
                || precision == "month" && (month.is_none() || day.is_some())
                || precision == "day" && (month.is_none() || day.is_none())
            {
                return Err(invalid("Precision/member mismatch"));
            }
            let (ordinal, lo, hi) = if provider.implementation.id == "weftext.calendar.gregorian.v2"
            {
                let year = i32::try_from(year).map_err(|_| invalid("Calendar year overflow"))?;
                let min = provider.data["min_year"].as_i64().unwrap_or(1);
                let max = provider.data["max_year"].as_i64().unwrap_or(9999);
                if i64::from(year) < min || i64::from(year) > max {
                    return Err(issue("Calendar coverage unavailable"));
                }
                if value.member("leap_month") == Some(&NativeValue::Bool(true)) {
                    return Err(invalid("Gregorian has no leap-month branch"));
                }
                let (lo, hi, ordinal) = match precision {
                    "year" => (
                        civil(year, 1, 1).map_err(|e| invalid(e.0))?,
                        civil(year, 12, 31).map_err(|e| invalid(e.0))?,
                        i64::from(year),
                    ),
                    "month" => {
                        let m =
                            u32::try_from(month.unwrap()).map_err(|_| invalid("Invalid month"))?;
                        let first = civil(year, m, 1).map_err(|e| invalid(e.0))?;
                        let last = if m == 12 {
                            civil(year, 12, 31).map_err(|e| invalid(e.0))?
                        } else {
                            civil(year, m + 1, 1)
                                .map_err(|e| invalid(e.0))?
                                .pred_opt()
                                .unwrap()
                        };
                        (first, last, (i64::from(year) - 1) * 12 + i64::from(m) - 1)
                    }
                    _ => {
                        let month =
                            u32::try_from(month.unwrap()).map_err(|_| invalid("Invalid month"))?;
                        let day =
                            u32::try_from(day.unwrap()).map_err(|_| invalid("Invalid day"))?;
                        let d = civil(year, month, day).map_err(|e| invalid(e.0))?;
                        (d, d, i64::from(d.num_days_from_ce()))
                    }
                };
                (
                    ordinal,
                    i64::from(lo.num_days_from_ce()),
                    i64::from(hi.num_days_from_ce()),
                )
            } else if provider.implementation.id == "weftext.calendar.table.v2" {
                let epoch = provider.data["epoch_date"]
                    .as_str()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                    .ok_or_else(|| issue("Calendar table epoch unavailable"))?;
                let years = provider.data["years"]
                    .as_array()
                    .ok_or_else(|| issue("Calendar table unavailable"))?;
                let mut offset = 0i64;
                let mut month_ordinal = 0i64;
                let mut result = None;
                for y in years {
                    let months = y["months"]
                        .as_array()
                        .ok_or_else(|| invalid("Invalid calendar table months"))?;
                    let year_start = offset;
                    let year_length = months
                        .iter()
                        .try_fold(0i64, |sum, m| sum.checked_add(m["days"].as_i64()?))
                        .ok_or_else(|| invalid("Calendar table day overflow"))?;
                    if y["year"].as_i64() == Some(year) && precision == "year" {
                        result = Some((
                            year,
                            i64::from(epoch.num_days_from_ce()) + year_start,
                            i64::from(epoch.num_days_from_ce()) + year_start + year_length - 1,
                        ));
                    }
                    for m in months {
                        let length = m["days"]
                            .as_i64()
                            .filter(|n| *n > 0)
                            .ok_or_else(|| invalid("Invalid calendar month length"))?;
                        if y["year"].as_i64() == Some(year)
                            && month == m["number"].as_i64()
                            && value.member("leap_month").map_or(Some(false), |v| {
                                if let NativeValue::Bool(b) = v {
                                    Some(*b)
                                } else {
                                    None
                                }
                            }) == m["leap"].as_bool()
                        {
                            let lo = i64::from(epoch.num_days_from_ce()) + offset;
                            result = if precision == "month" {
                                Some((month_ordinal, lo, lo + length - 1))
                            } else if precision == "day" {
                                let d = day.unwrap();
                                if !(1..=length).contains(&d) {
                                    return Err(invalid("Day outside calendar month"));
                                }
                                Some((lo + d - 1, lo + d - 1, lo + d - 1))
                            } else {
                                result
                            };
                        }
                        offset = offset
                            .checked_add(length)
                            .ok_or_else(|| invalid("Calendar coverage overflow"))?;
                        month_ordinal += 1;
                    }
                }
                result.ok_or_else(|| issue("Date outside finite calendar coverage"))?
            } else {
                return Err(issue("Calendar executor unavailable"));
            };
            Ok(Endpoint::Civil {
                calendar: calendar.into(),
                version: version.into(),
                precision: precision.into(),
                ordinal,
                lo,
                hi,
            })
        }
        _ => Err(invalid(
            "TemporalEndpoint requires civil date, precision date or offset instant",
        )),
    }
}
fn common_basis(endpoints: &[&Endpoint]) -> std::result::Result<(), Cell> {
    let instant = endpoints.iter().any(|v| matches!(v, Endpoint::Instant(_)));
    let civil = endpoints
        .iter()
        .any(|v| matches!(v, Endpoint::Civil { .. }));
    if instant && civil {
        return Err(invalid("Civil/instant mixing needs explicit conversion"));
    }
    if civil {
        let mut bases = std::collections::BTreeSet::new();
        let mut all_day = true;
        for e in endpoints {
            if let Endpoint::Civil {
                calendar,
                version,
                precision,
                ..
            } = e
            {
                bases.insert((calendar, version));
                all_day &= precision == "day";
            }
        }
        if bases.len() > 1 && !all_day {
            return Err(issue(
                "Coarse cross-calendar/version endpoints have no common day basis",
            ));
        }
    }
    Ok(())
}
fn order(a: &Endpoint, b: &Endpoint, strict: bool) -> Option<bool> {
    match (a, b) {
        (Endpoint::Instant(a), Endpoint::Instant(b)) => Some(if strict {
            a.compare(b).is_lt()
        } else {
            !a.compare(b).is_gt()
        }),
        (
            Endpoint::Civil {
                calendar: ac,
                version: av,
                precision: ap,
                ordinal: ao,
                lo: al,
                hi: ah,
            },
            Endpoint::Civil {
                calendar: bc,
                version: bv,
                precision: bp,
                ordinal: bo,
                lo: bl,
                hi: bh,
            },
        ) => {
            if ac == bc && av == bv && ap == bp {
                return Some(if strict { ao < bo } else { ao <= bo });
            }
            if strict {
                if ah < bl {
                    Some(true)
                } else if al >= bh {
                    Some(false)
                } else {
                    None
                }
            } else if ah <= bl {
                Some(true)
            } else if al > bh {
                Some(false)
            } else {
                None
            }
        }
        _ => None,
    }
}
pub fn compare_temporal(a: &NativeValue, b: &NativeValue, schema: &Schema) -> Cell {
    let (a, b) = match (endpoint(a, schema), endpoint(b, schema)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return e,
    };
    if let Err(e) = common_basis(&[&a, &b]) {
        return e;
    }
    let result = match (&a, &b) {
        (Endpoint::Instant(a), Endpoint::Instant(b)) => Some(a.compare(b)),
        (
            Endpoint::Civil {
                calendar: ac,
                version: av,
                precision: ap,
                ordinal: ao,
                lo: al,
                hi: ah,
            },
            Endpoint::Civil {
                calendar: bc,
                version: bv,
                precision: bp,
                ordinal: bo,
                lo: bl,
                hi: bh,
            },
        ) => {
            if ac == bc && av == bv && ap == bp {
                Some(ao.cmp(bo))
            } else if ap == "day" && bp == "day" {
                Some(al.cmp(bl))
            } else if ah < bl {
                Some(Ordering::Less)
            } else if al > bh {
                Some(Ordering::Greater)
            } else {
                None
            }
        }
        _ => None,
    };
    result
        .map(|v| {
            Cell::value(NativeValue::Int(
                match v {
                    Ordering::Less => "-1",
                    Ordering::Equal => "0",
                    Ordering::Greater => "1",
                }
                .into(),
            ))
        })
        .unwrap_or_else(|| issue("Mixed-precision temporal order is undecided"))
}
pub fn validate_calendar_endpoint(value: &NativeValue, schema: &Schema) -> Cell {
    match endpoint(value, schema) {
        Ok(_) => Cell::value(NativeValue::Bool(true)),
        Err(e) => e,
    }
}
struct Interval {
    start: Option<Endpoint>,
    end: Option<Endpoint>,
}
fn interval(
    value: &NativeValue,
    schema: &Schema,
    closed: bool,
) -> std::result::Result<Interval, Cell> {
    if value.record().is_none() {
        return Err(invalid("Interval must be a record"));
    }
    let start = value.member("start");
    let end = value.member(if closed { "end" } else { "end_exclusive" });
    if closed && (start.is_none() || end.is_none()) {
        return Err(issue("During absent endpoints mean unknown"));
    }
    if !closed
        && (start.is_none() && end.is_none()
            || matches!(start, Some(NativeValue::None))
            || matches!(end, Some(NativeValue::None)))
    {
        return Err(invalid("Range needs a finite endpoint; none is invalid"));
    }
    let read = |v: Option<&NativeValue>| -> std::result::Result<Option<Endpoint>, Cell> {
        match v {
            None | Some(NativeValue::None) => Ok(None),
            Some(v) => endpoint(v, schema).map(Some),
        }
    };
    let start = read(start)?;
    let end = read(end)?;
    let values = start.iter().chain(end.iter()).collect::<Vec<_>>();
    common_basis(&values)?;
    if let (Some(a), Some(b)) = (&start, &end) {
        match order(a, b, !closed) {
            Some(true) => {}
            Some(false) => return Err(invalid("Reversed or empty temporal interval")),
            None => return Err(issue("Interval endpoint order is undecided")),
        }
    }
    Ok(Interval { start, end })
}
pub fn validate_interval(value: &NativeValue, schema: &Schema, closed: bool) -> Cell {
    match interval(value, schema, closed) {
        Ok(_) => Cell::value(NativeValue::Bool(true)),
        Err(e) => e,
    }
}
fn conjunction(a: Option<bool>, b: Option<bool>) -> Cell {
    if a == Some(false) || b == Some(false) {
        Cell::value(NativeValue::Bool(false))
    } else if a == Some(true) && b == Some(true) {
        Cell::value(NativeValue::Bool(true))
    } else {
        issue("Temporal boundary proof is undecided")
    }
}
pub fn contains_date(during: &NativeValue, point: &NativeValue, schema: &Schema) -> Cell {
    let interval = match interval(during, schema, true) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let point = match endpoint(point, schema) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mut values = interval
        .start
        .iter()
        .chain(interval.end.iter())
        .collect::<Vec<_>>();
    values.push(&point);
    if let Err(e) = common_basis(&values) {
        return e;
    }
    conjunction(
        interval
            .start
            .as_ref()
            .map_or(Some(true), |s| order(s, &point, false)),
        interval
            .end
            .as_ref()
            .map_or(Some(true), |e| order(&point, e, false)),
    )
}
pub fn range_overlap(a: &NativeValue, b: &NativeValue, schema: &Schema) -> Cell {
    let (a, b) = match (interval(a, schema, false), interval(b, schema, false)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return e,
    };
    let values = a
        .start
        .iter()
        .chain(a.end.iter())
        .chain(b.start.iter())
        .chain(b.end.iter())
        .collect::<Vec<_>>();
    if let Err(e) = common_basis(&values) {
        return e;
    }
    conjunction(
        match (&a.start, &b.end) {
            (Some(a), Some(b)) => order(a, b, true),
            _ => Some(true),
        },
        match (&b.start, &a.end) {
            (Some(a), Some(b)) => order(a, b, true),
            _ => Some(true),
        },
    )
}
fn duration_interval(value: &NativeValue, schema: &Schema, closed: bool) -> Cell {
    let interval = match interval(value, schema, closed) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let (Some(a), Some(b)) = (interval.start, interval.end) else {
        return issue("Open or unknown duration has no implicit now");
    };
    match (a, b) {
        (Endpoint::Instant(a), Endpoint::Instant(b)) => {
            let seconds =
                ExactDecimal::parse(&(b.utc_seconds - a.utc_seconds).to_string()).unwrap();
            let negative = ExactDecimal {
                coefficient: -a.fraction.coefficient,
                scale: a.fraction.scale,
            };
            let value = seconds.add(&b.fraction).add(&negative);
            Cell::value(NativeValue::Record(vec![
                NativeMember {
                    key: "value".into(),
                    value: NativeValue::Text(value.spelling()),
                },
                NativeMember {
                    key: "unit".into(),
                    value: NativeValue::Text("seconds".into()),
                },
            ]))
        }
        (
            Endpoint::Civil {
                calendar: ac,
                version: av,
                precision: ap,
                ordinal: ao,
                lo: al,
                ..
            },
            Endpoint::Civil {
                calendar: bc,
                version: bv,
                precision: bp,
                ordinal: bo,
                lo: bl,
                ..
            },
        ) => {
            let (n, unit, calendar, version) = if ac == bc && av == bv && ap == bp {
                (
                    bo - ao + if closed { 1 } else { 0 },
                    match ap.as_str() {
                        "year" => "years",
                        "month" => "months",
                        _ => "days",
                    },
                    ac,
                    av,
                )
            } else if ap == "day" && bp == "day" {
                (
                    bl - al + if closed { 1 } else { 0 },
                    "days",
                    "gregorian".into(),
                    "proleptic-gregorian.v1".into(),
                )
            } else {
                return issue("Duration needs equal grain or exact full-day common basis");
            };
            Cell::value(NativeValue::Record(vec![
                NativeMember {
                    key: "value".into(),
                    value: NativeValue::Int(n.to_string()),
                },
                NativeMember {
                    key: "unit".into(),
                    value: NativeValue::Text(unit.into()),
                },
                NativeMember {
                    key: "calendar".into(),
                    value: NativeValue::Text(calendar),
                },
                NativeMember {
                    key: "version".into(),
                    value: NativeValue::Text(version),
                },
            ]))
        }
        _ => invalid("Temporal duration domains disagree"),
    }
}
pub fn range_duration(value: &NativeValue, schema: &Schema) -> Cell {
    duration_interval(value, schema, false)
}
pub fn during_duration(value: &NativeValue, schema: &Schema) -> Cell {
    duration_interval(value, schema, true)
}
pub fn next_civil_day(value: &NativeValue) -> Cell {
    match value {
        NativeValue::Date(s) => match NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .filter(|d| date_string(*d) == *s)
            .and_then(|d| plus_days(d, 1).ok())
        {
            Some(d) => Cell::value(NativeValue::Date(date_string(d))),
            None => invalid("Civil day overflow/invalid date"),
        },
        _ => invalid("nextCivilDay needs a complete civil date"),
    }
}

/// Explicit date selection creates a Period value under one installed series policy.
pub fn period_for_date(
    series_key: &str,
    kind: &str,
    date: &NativeValue,
    schema: &Schema,
) -> Result<NativeValue> {
    let policy = schema
        .series_policies
        .iter()
        .find(|p| p.key == series_key)
        .ok_or_else(|| DataError("Series policy unavailable".into()))?;
    if !policy.kinds.iter().any(|k| k == kind) {
        return Err(DataError("Period kind conflicts with series policy".into()));
    }
    let calendar = schema
        .providers
        .iter()
        .find(|p| p.id == policy.calendar)
        .ok_or_else(|| DataError("Series calendar provider unavailable".into()))?;
    let zone = schema
        .providers
        .iter()
        .find(|p| p.id == policy.zone)
        .ok_or_else(|| DataError("Series zone provider unavailable".into()))?;
    let NativeValue::Date(value) = date else {
        return Err(DataError(
            "Period selection needs explicit full Gregorian date".into(),
        ));
    };
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| DataError("Invalid Period selection date".into()))?;
    if date_string(date) != *value {
        return Err(DataError("Noncanonical Period selection date".into()));
    }
    let model = CalendarModel::new(calendar)?;
    let key = model.period_key(date, kind)?;
    let pairs = [
        ("series_key", series_key),
        ("kind", kind),
        ("calendar", calendar.data["key"].as_str().unwrap_or("")),
        (
            "calendar_version",
            calendar.data["version"].as_str().unwrap_or(""),
        ),
        ("rule", policy.rule.as_str()),
        ("key", key.as_str()),
        ("zone", zone.data["key"].as_str().unwrap_or("")),
        ("zone_version", zone.data["version"].as_str().unwrap_or("")),
    ];
    let result = NativeValue::Record(
        pairs
            .into_iter()
            .map(|(key, value)| NativeMember {
                key: key.into(),
                value: NativeValue::Text(value.into()),
            })
            .collect(),
    );
    period_bounds(&result, schema)?;
    Ok(result)
}

#[derive(Debug, Clone)]
pub(crate) struct CalendarMonth {
    pub year: i64,
    pub ordinal: i64,
    pub in_year: usize,
    pub number: i64,
    pub leap: bool,
    pub start: NaiveDate,
    pub end: NaiveDate,
}
#[derive(Debug, Clone)]
pub(crate) struct CalendarParts {
    pub year: i64,
    pub month_ordinal: i64,
    pub month_number: i64,
    pub leap: bool,
    pub day: i64,
    pub month_days: i64,
}
pub(crate) struct CalendarModel {
    gregorian: bool,
    min_year: i64,
    max_year: i64,
    pub months: Vec<CalendarMonth>,
    pub start: NaiveDate,
    pub end: NaiveDate,
}
impl CalendarModel {
    pub(crate) fn new(provider: &Provider) -> Result<Self> {
        if provider.implementation.id == "weftext.calendar.gregorian.v2" {
            let min = provider.data["min_year"]
                .as_i64()
                .ok_or_else(|| DataError("Calendar coverage unavailable".into()))?;
            let max = provider.data["max_year"]
                .as_i64()
                .ok_or_else(|| DataError("Calendar coverage unavailable".into()))?;
            return Ok(Self {
                gregorian: true,
                min_year: min,
                max_year: max,
                months: vec![],
                start: civil(min as i32, 1, 1)?,
                end: civil(max as i32, 12, 31)?,
            });
        }
        if provider.implementation.id != "weftext.calendar.table.v2" {
            return Err(DataError("Calendar executor unavailable".into()));
        }
        let epoch = provider.data["epoch_date"]
            .as_str()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .ok_or_else(|| DataError("Calendar table epoch unavailable".into()))?;
        let years = provider.data["years"]
            .as_array()
            .ok_or_else(|| DataError("Calendar table unavailable".into()))?;
        let mut months = vec![];
        let mut at = epoch;
        for year in years {
            let y = year["year"]
                .as_i64()
                .ok_or_else(|| DataError("Invalid calendar year".into()))?;
            for (i, m) in year["months"]
                .as_array()
                .ok_or_else(|| DataError("Calendar months unavailable".into()))?
                .iter()
                .enumerate()
            {
                let days = m["days"]
                    .as_u64()
                    .filter(|d| *d > 0)
                    .ok_or_else(|| DataError("Invalid calendar month length".into()))?;
                let end = plus_days(at, days)?;
                months.push(CalendarMonth {
                    year: y,
                    ordinal: months.len() as i64,
                    in_year: i + 1,
                    number: m["number"]
                        .as_i64()
                        .ok_or_else(|| DataError("Invalid calendar month number".into()))?,
                    leap: m["leap"]
                        .as_bool()
                        .ok_or_else(|| DataError("Invalid calendar month leap flag".into()))?,
                    start: at,
                    end,
                });
                at = end;
            }
        }
        let first = months
            .first()
            .ok_or_else(|| DataError("Calendar table unavailable".into()))?;
        let min_year = first.year;
        let max_year = months.last().unwrap().year;
        Ok(Self {
            gregorian: false,
            min_year,
            max_year,
            start: epoch,
            end: at,
            months,
        })
    }
    pub(crate) fn parts(&self, date: NaiveDate) -> Result<CalendarParts> {
        if self.gregorian {
            if i64::from(date.year()) < self.min_year || i64::from(date.year()) > self.max_year {
                return Err(DataError("Calendar coverage unavailable".into()));
            }
            let first = civil(date.year(), date.month(), 1)?;
            let days = if date.month() == 12 {
                31
            } else {
                (civil(date.year(), date.month() + 1, 1)? - first).num_days()
            };
            Ok(CalendarParts {
                year: i64::from(date.year()),
                month_ordinal: (i64::from(date.year()) - 1) * 12 + i64::from(date.month0()),
                month_number: i64::from(date.month()),
                leap: false,
                day: i64::from(date.day()),
                month_days: days,
            })
        } else {
            let m = self
                .months
                .iter()
                .find(|m| m.start <= date && date < m.end)
                .ok_or_else(|| DataError("Calendar table coverage unavailable".into()))?;
            Ok(CalendarParts {
                year: m.year,
                month_ordinal: m.ordinal,
                month_number: m.number,
                leap: m.leap,
                day: (date - m.start).num_days() + 1,
                month_days: (m.end - m.start).num_days(),
            })
        }
    }
    fn period_key(&self, date: NaiveDate, kind: &str) -> Result<String> {
        let parts = self.parts(date)?;
        if self.gregorian {
            Ok(match kind {
                "day" => date_string(date),
                "week" => format!(
                    "{:04}-W{:02}",
                    date.iso_week().year(),
                    date.iso_week().week()
                ),
                "month" => format!("{:04}-{:02}", date.year(), date.month()),
                "quarter" => format!("{:04}-Q{}", date.year(), date.month0() / 3 + 1),
                "year" => format!("{:04}", date.year()),
                _ => return Err(DataError("Unknown period kind".into())),
            })
        } else {
            let m = &self.months[parts.month_ordinal as usize];
            Ok(match kind {
                "day" => format!("Y{}-M{}-D{}", m.year, m.in_year, parts.day),
                "week" => format!("W{}", (date - self.week_epoch()).num_days().div_euclid(7)),
                "month" => format!("Y{}-M{}", m.year, m.in_year),
                "quarter" => format!("Y{}-Q{}", m.year, (m.in_year - 1) / 3 + 1),
                "year" => format!("Y{}", m.year),
                _ => return Err(DataError("Unknown period kind".into())),
            })
        }
    }
    fn week_epoch(&self) -> NaiveDate {
        self.start - chrono::Duration::days(i64::from(self.start.weekday().num_days_from_monday()))
    }
    fn canonical_int(value: &str) -> Result<i64> {
        let n = value
            .parse::<i64>()
            .map_err(|_| DataError("Invalid table period key".into()))?;
        if n.to_string() != value {
            return Err(DataError("Noncanonical table period key".into()));
        }
        Ok(n)
    }
    fn table_period(&self, kind: &str, key: &str) -> Result<(NaiveDate, NaiveDate)> {
        let bounds = if kind == "week" {
            let n = Self::canonical_int(
                key.strip_prefix('W')
                    .ok_or_else(|| DataError("Expected W<integer>".into()))?,
            )?;
            let days = n
                .checked_mul(7)
                .ok_or_else(|| DataError("Period key overflow".into()))?;
            let start = self
                .week_epoch()
                .checked_add_signed(
                    chrono::Duration::try_days(days)
                        .ok_or_else(|| DataError("Period key overflow".into()))?,
                )
                .ok_or_else(|| DataError("Period key overflow".into()))?;
            (start, plus_days(start, 7)?)
        } else {
            let key = key
                .strip_prefix('Y')
                .ok_or_else(|| DataError("Expected Y<integer>".into()))?;
            let (year, suffix) = if let Some((year, suffix)) = key.split_once("-M") {
                (Self::canonical_int(year)?, Some(("M", suffix)))
            } else if let Some((year, suffix)) = key.split_once("-Q") {
                (Self::canonical_int(year)?, Some(("Q", suffix)))
            } else {
                (Self::canonical_int(key)?, None)
            };
            let months: Vec<_> = self.months.iter().filter(|m| m.year == year).collect();
            if months.is_empty() {
                return Err(DataError("Calendar table coverage unavailable".into()));
            }
            match (kind, suffix) {
                ("year", None) => (months[0].start, months.last().unwrap().end),
                ("quarter", Some(("Q", n))) => {
                    let q = Self::canonical_int(n)?;
                    if q < 1 {
                        return Err(DataError("Invalid table quarter".into()));
                    }
                    let i = usize::try_from(q - 1)
                        .ok()
                        .and_then(|q| q.checked_mul(3))
                        .ok_or_else(|| DataError("Quarter key overflow".into()))?;
                    let m = months
                        .get(i)
                        .ok_or_else(|| DataError("Invalid table quarter".into()))?;
                    (m.start, months[(i + 2).min(months.len() - 1)].end)
                }
                ("month", Some(("M", n))) | ("day", Some(("M", n))) => {
                    let (n, day) = n.split_once("-D").map_or((n, None), |(n, d)| (n, Some(d)));
                    let i = Self::canonical_int(n)?;
                    if i < 1 {
                        return Err(DataError("Invalid table month ordinal".into()));
                    }
                    let m = usize::try_from(i - 1)
                        .ok()
                        .and_then(|i| months.get(i))
                        .ok_or_else(|| DataError("Invalid table month ordinal".into()))?;
                    if kind == "month" && day.is_none() {
                        (m.start, m.end)
                    } else if kind == "day" && day.is_some() {
                        let d = Self::canonical_int(day.unwrap())?;
                        if d < 1 || d > (m.end - m.start).num_days() {
                            return Err(DataError("Invalid table day".into()));
                        }
                        let start = plus_days(m.start, (d - 1) as u64)?;
                        (start, plus_days(start, 1)?)
                    } else {
                        return Err(DataError("Table key/kind mismatch".into()));
                    }
                }
                _ => return Err(DataError("Table key/kind mismatch".into())),
            }
        };
        if bounds.0 < self.start || bounds.1 > self.end {
            return Err(DataError(
                "Period crosses calendar table coverage unavailable".into(),
            ));
        }
        Ok(bounds)
    }
    fn step_period(&self, start: NaiveDate, kind: &str, steps: i32) -> Result<NaiveDate> {
        let parts = self.parts(start)?;
        let date = match kind {
            "day" | "week" => start
                .checked_add_signed(chrono::Duration::days(
                    i64::from(steps) * if kind == "week" { 7 } else { 1 },
                ))
                .ok_or_else(|| DataError("Period step overflow".into()))?,
            "month" | "quarter" if self.gregorian => {
                let ordinal =
                    parts.month_ordinal + i64::from(steps) * if kind == "quarter" { 3 } else { 1 };
                civil(
                    i32::try_from(ordinal.div_euclid(12) + 1)
                        .map_err(|_| DataError("Period step overflow".into()))?,
                    ordinal.rem_euclid(12) as u32 + 1,
                    1,
                )?
            }
            "month" => {
                self.months
                    .get(
                        usize::try_from(parts.month_ordinal + i64::from(steps))
                            .map_err(|_| DataError("Period step coverage unavailable".into()))?,
                    )
                    .ok_or_else(|| DataError("Period step coverage unavailable".into()))?
                    .start
            }
            "quarter" => {
                let starts: Vec<_> = self
                    .months
                    .iter()
                    .filter(|m| (m.in_year - 1) % 3 == 0)
                    .collect();
                let ordinal = starts
                    .iter()
                    .position(|m| m.start == start)
                    .ok_or_else(|| DataError("Invalid table quarter start".into()))?
                    as i64;
                starts
                    .get(
                        usize::try_from(ordinal + i64::from(steps))
                            .map_err(|_| DataError("Period step coverage unavailable".into()))?,
                    )
                    .ok_or_else(|| DataError("Period step coverage unavailable".into()))?
                    .start
            }
            "year" => {
                let year = parts
                    .year
                    .checked_add(i64::from(steps))
                    .ok_or_else(|| DataError("Period step overflow".into()))?;
                if self.gregorian {
                    civil(
                        i32::try_from(year)
                            .map_err(|_| DataError("Period step overflow".into()))?,
                        1,
                        1,
                    )?
                } else {
                    self.months
                        .iter()
                        .find(|m| m.year == year)
                        .ok_or_else(|| DataError("Period step coverage unavailable".into()))?
                        .start
                }
            }
            _ => return Err(DataError("Unknown period kind".into())),
        };
        self.parts(date)?;
        Ok(date)
    }
}

pub(crate) struct FrozenZone {
    from: ExactInstant,
    until: ExactInstant,
    segments: Vec<(ExactInstant, ExactInstant, i64)>,
    pub max_offset: i64,
    pub min_offset: i64,
}
impl FrozenZone {
    pub(crate) fn new(provider: &Provider) -> Result<Self> {
        if provider.implementation.id != "weftext.zone.table.v1" {
            return Err(DataError("Zone executor unavailable".into()));
        }
        let parse = |key: &str| {
            provider.data[key]
                .as_str()
                .ok_or_else(|| DataError("Zone table coverage unavailable".into()))
                .and_then(ExactInstant::parse)
        };
        let from = parse("from")?;
        let until = parse("until")?;
        let mut at = from.clone();
        let mut offset = provider.data["initial_offset_seconds"]
            .as_i64()
            .ok_or_else(|| DataError("Zone offset unavailable".into()))?;
        let mut segments = vec![];
        for transition in provider.data["transitions"]
            .as_array()
            .ok_or_else(|| DataError("Zone table transitions unavailable".into()))?
        {
            let next = ExactInstant::parse(
                transition["at"]
                    .as_str()
                    .ok_or_else(|| DataError("Invalid zone transition".into()))?,
            )?;
            segments.push((at, next.clone(), offset));
            at = next;
            offset = transition["offset_seconds"]
                .as_i64()
                .ok_or_else(|| DataError("Invalid zone offset".into()))?;
        }
        segments.push((at, until.clone(), offset));
        let max_offset = segments.iter().map(|s| s.2).max().unwrap();
        let min_offset = segments.iter().map(|s| s.2).min().unwrap();
        Ok(Self {
            from,
            until,
            segments,
            max_offset,
            min_offset,
        })
    }
    pub(crate) fn civil(&self, instant: &ExactInstant) -> Result<chrono::NaiveDateTime> {
        let offset = self
            .segments
            .iter()
            .find(|(start, end, _)| !instant.compare(start).is_lt() && instant.compare(end).is_lt())
            .ok_or_else(|| DataError("Zone coverage unavailable".into()))?
            .2;
        let local = chrono::DateTime::from_timestamp(
            instant
                .utc_seconds
                .checked_add(offset)
                .ok_or_else(|| DataError("Zone conversion overflow".into()))?,
            0,
        )
        .map(|d| d.naive_utc())
        .ok_or_else(|| DataError("Zone conversion overflow".into()))?;
        if !(1..=9999).contains(&local.year()) {
            return Err(DataError("Local Gregorian coverage unavailable".into()));
        }
        Ok(local)
    }
    pub(crate) fn instant(
        &self,
        local: chrono::NaiveDateTime,
        fraction: &ExactDecimal,
        fold: &str,
    ) -> Result<Option<ExactInstant>> {
        if !matches!(fold, "earlier" | "later") {
            return Err(DataError("Explicit earlier/later fold required".into()));
        }
        let seconds = local.and_utc().timestamp();
        let earliest = ExactInstant {
            utc_seconds: seconds - self.max_offset,
            fraction: fraction.clone(),
        };
        let latest = ExactInstant {
            utc_seconds: seconds - self.min_offset,
            fraction: fraction.clone(),
        };
        if earliest.compare(&self.from).is_lt() || !latest.compare(&self.until).is_lt() {
            return Err(DataError(
                "Zone coverage unavailable for complete local-time proof".into(),
            ));
        }
        let mut candidates = self
            .segments
            .iter()
            .filter_map(|(start, end, offset)| {
                let candidate = ExactInstant {
                    utc_seconds: seconds - offset,
                    fraction: fraction.clone(),
                };
                (!candidate.compare(start).is_lt() && candidate.compare(end).is_lt())
                    .then_some(candidate)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(ExactInstant::compare);
        candidates.dedup_by(|a, b| a.compare(b).is_eq());
        Ok(if fold == "earlier" {
            candidates.first()
        } else {
            candidates.last()
        }
        .cloned())
    }
}
/// Converts civil Period bounds only with the exact series zone. A skipped gap
/// cannot be presented as a valid complete instant range.
pub fn period_zoned_bounds(period: &NativeValue, fold: &str, schema: &Schema) -> Cell {
    let result = (|| -> Result<NativeValue> {
        let (start, end) = period_dates(period, schema)?;
        let (_, _, provider) = period_providers(period, schema)?;
        let zone = FrozenZone::new(provider)?;
        let zero = ExactDecimal::parse("0")?;
        let mut members = vec![];
        for (key, date) in [("start", start), ("end_exclusive", end)] {
            let instant = zone
                .instant(date.and_hms_opt(0, 0, 0).unwrap(), &zero, fold)?
                .ok_or_else(|| {
                    DataError("Period midnight falls in zone gap; bounds unavailable".into())
                })?;
            members.push(NativeMember {
                key: key.into(),
                value: NativeValue::Text(instant.canonical()?),
            });
        }
        Ok(NativeValue::Record(members))
    })();
    match result {
        Ok(v) => Cell::value(v),
        Err(e) if e.0.contains("unavailable") || e.0.contains("coverage") => issue(e.0),
        Err(e) => invalid(e.0),
    }
}

fn frozen_zone(schema: &Schema, key: &str, version: &str) -> Result<FrozenZone> {
    let provider = schema
        .providers
        .iter()
        .find(|p| {
            p.data["kind"].as_str() == Some("zone-table")
                && p.data["key"].as_str() == Some(key)
                && p.data["version"].as_str() == Some(version)
        })
        .ok_or_else(|| DataError("Exact zone provider/version unavailable".into()))?;
    FrozenZone::new(provider)
}
/// Pure frozen-zone primitive. Native Typst local datetimes have whole seconds;
/// a fractional instant requires the exact recurrence context and is not rounded.
pub fn zone_to_civil(instant: &NativeValue, key: &str, version: &str, schema: &Schema) -> Cell {
    let result = (|| -> Result<NativeValue> {
        let instant = ExactInstant::parse(
            instant
                .text()
                .ok_or_else(|| DataError("toCivil needs an exact instant".into()))?,
        )?;
        let zone = frozen_zone(schema, key, version)?;
        let local = zone.civil(&instant)?;
        if !instant.fraction.compare(&ExactDecimal::parse("0")?).is_eq() {
            return Err(DataError("Fractional local time unavailable in native whole-second local_datetime; exact recurrence context preserves it".into()));
        }
        Ok(NativeValue::LocalDatetime(
            local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        ))
    })();
    match result {
        Ok(value) => Cell::value(value),
        Err(e) if e.0.contains("unavailable") || e.0.contains("coverage") => issue(e.0),
        Err(e) => invalid(e.0),
    }
}
/// A skipped gap returns absent; ambiguity always requires earlier/later.
pub fn zone_to_instant(
    local: &NativeValue,
    key: &str,
    version: &str,
    fold: &str,
    gap: &str,
    schema: &Schema,
) -> Cell {
    let result = (|| -> Result<Option<NativeValue>> {
        if gap != "skip" {
            return Err(DataError("Explicit gap=skip required".into()));
        }
        let NativeValue::LocalDatetime(value) = local else {
            return Err(DataError("toInstant needs native local_datetime".into()));
        };
        let local = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
            .map_err(|_| DataError("Invalid local_datetime".into()))?;
        if local.format("%Y-%m-%dT%H:%M:%S").to_string() != *value
            || !(1..=9999).contains(&local.year())
        {
            return Err(DataError("Noncanonical local_datetime".into()));
        }
        let zone = frozen_zone(schema, key, version)?;
        zone.instant(local, &ExactDecimal::parse("0")?, fold)?
            .map(|i| i.canonical().map(NativeValue::Text))
            .transpose()
    })();
    match result {
        Ok(Some(value)) => Cell::value(value),
        Ok(None) => Cell::Absent,
        Err(e) if e.0.contains("unavailable") || e.0.contains("coverage") => issue(e.0),
        Err(e) => invalid(e.0),
    }
}
