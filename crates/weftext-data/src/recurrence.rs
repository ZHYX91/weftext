//! Frozen, pure recurrence admission and bounded expansion. Core owns snapshots
//! and persistence; derived occurrences never create or edit notes here.
use crate::temporal::{CalendarModel, CalendarParts, FrozenZone};
use crate::*;
use chrono::{Datelike, Days, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpansionBudget {
    pub max_steps: u64,
    pub max_occurrences: u64,
    pub max_bytes: u64,
}
impl Default for ExpansionBudget {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_occurrences: 10_000,
            max_bytes: 8_388_608,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionStatus {
    Complete,
    Partial,
    Invalid,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceOccurrence {
    pub original_onset: NativeValue,
    pub range: NativeValue,
    pub replacement: bool,
    pub title: Option<NativeValue>,
    pub status: Option<NativeValue>,
    pub note: Option<NativeValue>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceExpansion {
    pub status: ExpansionStatus,
    pub occurrences: Vec<RecurrenceOccurrence>,
    pub diagnostics: Vec<String>,
    pub steps: u64,
    /// Exact installed provider identities and data versions consumed by this call.
    pub providers: Vec<String>,
}
fn failure(message: impl Into<String>) -> DataError {
    DataError(message.into())
}
fn as_cell(result: Result<()>) -> Cell {
    match result {
        Ok(()) => Cell::value(NativeValue::Bool(true)),
        Err(e)
            if e.0.contains("unavailable")
                || e.0.contains("coverage")
                || e.0.contains("budget") =>
        {
            Cell::Unavailable { reason: e.0 }
        }
        Err(e) => Cell::Invalid {
            diagnostics: vec![e.0],
            value: None,
        },
    }
}
fn text<'a>(value: &'a NativeValue, key: &str) -> Result<&'a str> {
    value
        .member(key)
        .and_then(NativeValue::text)
        .ok_or_else(|| failure(format!("{key}: required text")))
}
fn integer(value: &NativeValue, key: &str) -> Result<i64> {
    value
        .member(key)
        .and_then(NativeValue::integer)
        .ok_or_else(|| failure(format!("{key}: required integer")))
}
fn array<'a>(value: &'a NativeValue, key: &str) -> Result<Option<&'a [NativeValue]>> {
    value
        .member(key)
        .map(|v| {
            v.array()
                .ok_or_else(|| failure(format!("{key}: expected array")))
        })
        .transpose()
}
fn weekday(value: &str) -> Result<i64> {
    ["MO", "TU", "WE", "TH", "FR", "SA", "SU"]
        .iter()
        .position(|d| *d == value)
        .map(|n| n as i64)
        .ok_or_else(|| failure("Invalid weekday"))
}
fn exact_provider<'a>(
    schema: &'a Schema,
    key: &str,
    version: &str,
    family: &str,
) -> Result<&'a Provider> {
    schema
        .providers
        .iter()
        .find(|p| {
            p.data["kind"].as_str().is_some_and(|k| {
                if family == "calendar" {
                    matches!(k, "gregorian" | "calendar-table")
                } else {
                    k == "zone-table"
                }
            }) && p.data["key"].as_str() == Some(key)
                && p.data["version"].as_str() == Some(version)
        })
        .ok_or_else(|| failure(format!("Exact {family} provider/version unavailable")))
}
#[derive(Debug, Clone)]
enum Onset {
    Date(NaiveDate),
    Instant(ExactInstant),
}
impl Onset {
    fn parse(value: &NativeValue) -> Result<Self> {
        match value {
            NativeValue::Date(s) => {
                let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map_err(|_| failure("Invalid occurrence date"))?;
                if !(1..=9999).contains(&date.year()) || date.format("%Y-%m-%d").to_string() != *s {
                    return Err(failure("Noncanonical occurrence date"));
                }
                Ok(Self::Date(date))
            }
            NativeValue::Text(s) => Ok(Self::Instant(ExactInstant::parse(s)?)),
            _ => Err(failure(
                "Recurrence requires complete DateOrInstant; precision cannot be filled",
            )),
        }
    }
    fn same_domain(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Date(_), Self::Date(_)) | (Self::Instant(_), Self::Instant(_))
        )
    }
    fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Date(a), Self::Date(b)) => a.cmp(b),
            (Self::Instant(a), Self::Instant(b)) => a.compare(b),
            _ => unreachable!("domain preflight"),
        }
    }
    fn native(&self) -> Result<NativeValue> {
        Ok(match self {
            Self::Date(d) => NativeValue::Date(d.format("%Y-%m-%d").to_string()),
            Self::Instant(i) => NativeValue::Text(i.canonical()?),
        })
    }
    fn key(&self) -> Result<String> {
        self.native().map(|v| match v {
            NativeValue::Date(d) | NativeValue::Text(d) => d,
            _ => unreachable!(),
        })
    }
    fn civil(&self, zone: &FrozenZone) -> Result<NaiveDateTime> {
        match self {
            Self::Date(d) => Ok(d.and_hms_opt(0, 0, 0).unwrap()),
            Self::Instant(i) => zone.civil(i),
        }
    }
}
struct Rule<'a> {
    native: &'a NativeValue,
    frequency: &'a str,
    interval: i64,
    week_start: i64,
    months: Option<Vec<i64>>,
    days: Option<Vec<i64>>,
    weekdays: Option<Vec<(i64, Option<i64>)>>,
    count: Option<usize>,
    until: Option<Onset>,
    rdates: Vec<Onset>,
    exdates: Vec<Onset>,
    exceptions: Vec<&'a NativeValue>,
    calendar_provider: &'a Provider,
    zone_provider: &'a Provider,
    calendar: CalendarModel,
    zone: FrozenZone,
}
impl<'a> Rule<'a> {
    fn parse(value: &'a NativeValue, schema: &'a Schema) -> Result<Self> {
        if value.record().is_none() {
            return Err(failure("Recurrence must be a record"));
        }
        let frequency = text(value, "frequency")?;
        if !matches!(frequency, "daily" | "weekly" | "monthly" | "yearly") {
            return Err(failure("Invalid recurrence frequency"));
        }
        let interval = integer(value, "interval")?;
        if !(1..=65535).contains(&interval) {
            return Err(failure("Recurrence interval outside 1..65535"));
        }
        let count = value
            .member("count")
            .map(|v| {
                v.integer()
                    .filter(|n| (1..=2_147_483_647).contains(n))
                    .map(|n| n as usize)
                    .ok_or_else(|| failure("Recurrence count outside 1..2147483647"))
            })
            .transpose()?;
        if count.is_some() && value.member("until").is_some() {
            return Err(failure("count and until are mutually exclusive"));
        }
        let until = value.member("until").map(Onset::parse).transpose()?;
        let selector = |key: &str, min: i64, max: i64| -> Result<Option<Vec<i64>>> {
            array(value, key)?
                .map(|a| {
                    a.iter()
                        .map(|v| {
                            v.integer()
                                .filter(|n| *n >= min && *n <= max && *n != 0)
                                .ok_or_else(|| failure(format!("Invalid {key} selector")))
                        })
                        .collect()
                })
                .transpose()
        };
        let months = selector("byMonth", 1, 12)?;
        let days = selector("byMonthDay", -31, 31)?;
        let weekdays = array(value, "byWeekday")?
            .map(|a| {
                a.iter()
                    .map(|v| {
                        let d = weekday(text(v, "day")?)?;
                        let ordinal = v
                            .member("ordinal")
                            .map(|v| {
                                v.integer()
                                    .filter(|n| (-5..=5).contains(n) && *n != 0)
                                    .ok_or_else(|| failure("Invalid weekday ordinal"))
                            })
                            .transpose()?;
                        if ordinal.is_some() && !matches!(frequency, "monthly" | "yearly") {
                            return Err(failure(
                                "Weekday ordinal requires monthly/yearly frequency",
                            ));
                        }
                        Ok((d, ordinal))
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?;
        let week_start = value
            .member("weekStart")
            .map(|v| {
                v.text()
                    .ok_or_else(|| failure("Invalid weekStart"))
                    .and_then(weekday)
            })
            .transpose()?
            .unwrap_or(0);
        if !matches!(text(value, "fold")?, "earlier" | "later") || text(value, "gap")? != "skip" {
            return Err(failure(
                "Recurrence requires explicit fold earlier/later and gap skip",
            ));
        }
        let calendar_provider = exact_provider(
            schema,
            text(value, "calendar")?,
            text(value, "calendar_version")?,
            "calendar",
        )?;
        let zone_provider = exact_provider(
            schema,
            text(value, "zone")?,
            text(value, "zone_version")?,
            "zone",
        )?;
        let calendar = CalendarModel::new(calendar_provider)?;
        let zone = FrozenZone::new(zone_provider)?;
        let onsets = |key: &str| -> Result<Vec<Onset>> {
            array(value, key)?
                .unwrap_or(&[])
                .iter()
                .map(Onset::parse)
                .collect()
        };
        let rdates = onsets("rDates")?;
        let exdates = onsets("exDates")?;
        let exceptions = array(value, "exceptions")?
            .unwrap_or(&[])
            .iter()
            .collect::<Vec<_>>();
        let mut exception_keys = BTreeSet::new();
        for exception in &exceptions {
            validate_exception_inner(exception, schema)?;
            let key = Onset::parse(
                exception
                    .member("onset")
                    .ok_or_else(|| failure("Exception onset required"))?,
            )?
            .key()?;
            if !exception_keys.insert(key) {
                return Err(failure("Duplicate exceptions for one exact original onset"));
            }
        }
        let rule = Self {
            native: value,
            frequency,
            interval,
            week_start,
            months,
            days,
            weekdays,
            count,
            until,
            rdates,
            exdates,
            exceptions,
            calendar_provider,
            zone_provider,
            calendar,
            zone,
        };
        // Even locally valid fields cannot acquire an invented cross-domain basis.
        let mut all = rule
            .rdates
            .iter()
            .chain(rule.exdates.iter())
            .chain(rule.until.iter())
            .cloned()
            .collect::<Vec<_>>();
        for e in &rule.exceptions {
            all.push(Onset::parse(e.member("onset").unwrap())?);
            if let Some(range) = e.member("range") {
                all.extend(exact_range(range)?);
            }
        }
        if let Some(first) = all.first() {
            for onset in &all {
                if !first.same_domain(onset) {
                    return Err(failure("Recurrence date/instant domains disagree"));
                }
                rule.coverage(onset)?;
            }
        }
        Ok(rule)
    }
    fn coverage(&self, onset: &Onset) -> Result<()> {
        self.calendar.parts(onset.civil(&self.zone)?.date())?;
        Ok(())
    }
    fn empty_selectors(&self) -> bool {
        self.months.as_ref().is_some_and(Vec::is_empty)
            || self.days.as_ref().is_some_and(Vec::is_empty)
            || self.weekdays.as_ref().is_some_and(Vec::is_empty)
    }
    fn matches(
        &self,
        date: NaiveDate,
        parts: &CalendarParts,
        anchor_date: NaiveDate,
        anchor: &CalendarParts,
    ) -> bool {
        let phase = match self.frequency {
            "daily" => (date - anchor_date).num_days(),
            "weekly" => {
                let bucket = |d: NaiveDate| {
                    i64::from(d.num_days_from_ce())
                        - (i64::from(d.weekday().num_days_from_monday()) - self.week_start)
                            .rem_euclid(7)
                };
                (bucket(date) - bucket(anchor_date)) / 7
            }
            "monthly" => parts.month_ordinal - anchor.month_ordinal,
            "yearly" => parts.year - anchor.year,
            _ => unreachable!(),
        };
        if phase < 0 || phase % self.interval != 0 {
            return false;
        }
        if self
            .months
            .as_ref()
            .is_some_and(|m| !m.contains(&parts.month_number))
        {
            return false;
        }
        if self.frequency == "yearly"
            && self.months.is_none()
            && self.days.is_none()
            && self.weekdays.is_none()
            && (parts.month_number != anchor.month_number || parts.leap != anchor.leap)
        {
            return false;
        }
        if self.days.as_ref().is_some_and(|ds| {
            !ds.iter().any(|d| {
                if *d > 0 {
                    parts.day == *d
                } else {
                    parts.day == parts.month_days + d + 1
                }
            })
        }) {
            return false;
        }
        let day = i64::from(date.weekday().num_days_from_monday());
        if let Some(weekdays) = &self.weekdays {
            if !weekdays.iter().any(|(d, ordinal)| {
                *d == day
                    && ordinal.is_none_or(|n| {
                        if n > 0 {
                            (parts.day - 1) / 7 + 1 == n
                        } else {
                            -((parts.month_days - parts.day) / 7 + 1) == n
                        }
                    })
            }) {
                return false;
            }
        } else if self.frequency == "weekly" && date.weekday() != anchor_date.weekday() {
            return false;
        }
        if matches!(self.frequency, "monthly" | "yearly")
            && self.days.is_none()
            && self.weekdays.is_none()
            && parts.day != anchor.day
        {
            return false;
        }
        true
    }
    fn budget(&self, supplied: ExpansionBudget, schema: &Schema) -> ExpansionBudget {
        let mut result = supplied;
        for p in [
            Some(self.calendar_provider),
            Some(self.zone_provider),
            schema
                .providers
                .iter()
                .find(|p| p.implementation.id == "weftext.temporal.rules.v2"),
        ]
        .into_iter()
        .flatten()
        {
            result.max_steps = result.max_steps.min(p.limits.max_steps);
            result.max_bytes = result.max_bytes.min(p.limits.max_bytes);
            result.max_occurrences = result.max_occurrences.min(p.limits.max_expansion);
        }
        result
    }
}
fn exact_range(range: &NativeValue) -> Result<[Onset; 2]> {
    let start = Onset::parse(
        range
            .member("start")
            .ok_or_else(|| failure("Recurrence range needs bounded start"))?,
    )?;
    let end = Onset::parse(
        range
            .member("end_exclusive")
            .ok_or_else(|| failure("Recurrence range needs bounded end_exclusive"))?,
    )?;
    if !start.same_domain(&end) {
        return Err(failure("Recurrence range date/instant domains disagree"));
    }
    if !start.compare(&end).is_lt() {
        return Err(failure("Recurrence range is empty or reversed"));
    }
    Ok([start, end])
}
fn validate_exception_inner(exception: &NativeValue, _schema: &Schema) -> Result<()> {
    if exception.record().is_none() {
        return Err(failure("Exception must be a record"));
    }
    let onset = Onset::parse(
        exception
            .member("onset")
            .ok_or_else(|| failure("Exception onset required"))?,
    )?;
    let changed = ["range", "title", "status", "note"]
        .iter()
        .any(|k| exception.member(k).is_some());
    match text(exception, "action")? {
        "cancel" if changed => return Err(failure("Cancel forbids replacement data")),
        "replace" if !changed => return Err(failure("Replace requires an actual change")),
        "cancel" | "replace" => {}
        _ => return Err(failure("Invalid exception action")),
    }
    if let Some(range) = exception.member("range")
        && exact_range(range)?
            .iter()
            .any(|end| !onset.same_domain(end))
    {
        return Err(failure("Exception onset/replacement domains disagree"));
    }
    for key in ["title", "note"] {
        if exception.member(key).is_some_and(|v| v.text().is_none()) {
            return Err(failure("Replacement title/note must be text"));
        }
    }
    if exception
        .member("status")
        .is_some_and(|v| !matches!(v.text(), Some("tentative" | "confirmed" | "cancelled")))
    {
        return Err(failure("Invalid replacement status"));
    }
    Ok(())
}
pub fn validate_exception(exception: &NativeValue, schema: &Schema) -> Cell {
    as_cell(validate_exception_inner(exception, schema))
}
pub fn validate_recurrence(recurrence: &NativeValue, schema: &Schema) -> Cell {
    as_cell(Rule::parse(recurrence, schema).map(|_| ()))
}
/// Anchor and exception membership are checked using the same expansion engine.
pub fn validate_event(
    range: &NativeValue,
    recurrence: Option<&NativeValue>,
    schema: &Schema,
) -> Cell {
    let Some(recurrence) = recurrence else {
        return validate_interval(range, schema, false);
    };
    let expansion = expand_recurrence(range, recurrence, range, schema, ExpansionBudget::default());
    match expansion.status {
        ExpansionStatus::Complete => Cell::value(NativeValue::Bool(true)),
        ExpansionStatus::Invalid => Cell::Invalid {
            diagnostics: expansion.diagnostics,
            value: None,
        },
        _ => Cell::Unavailable {
            reason: expansion.diagnostics.join("; "),
        },
    }
}
fn native_range(start: &Onset, end: &Onset) -> Result<NativeValue> {
    Ok(NativeValue::Record(vec![
        NativeMember {
            key: "start".into(),
            value: start.native()?,
        },
        NativeMember {
            key: "end_exclusive".into(),
            value: end.native()?,
        },
    ]))
}
fn template_range(onset: &Onset, template: &[Onset; 2]) -> Result<NativeValue> {
    let end = match (onset, &template[0], &template[1]) {
        (Onset::Date(d), Onset::Date(a), Onset::Date(b)) => Onset::Date(
            d.checked_add_days(Days::new((*b - *a).num_days() as u64))
                .ok_or_else(|| failure("Occurrence range coverage unavailable"))?,
        ),
        (Onset::Instant(i), Onset::Instant(a), Onset::Instant(b)) => {
            let negative = ExactDecimal {
                coefficient: -a.fraction.coefficient.clone(),
                scale: a.fraction.scale,
            };
            Onset::Instant(i.shifted(b.utc_seconds - a.utc_seconds, &b.fraction.add(&negative))?)
        }
        _ => return Err(failure("Occurrence/template domains disagree")),
    };
    native_range(onset, &end)
}
/// Complete means all original-key membership and moved-in replacement proofs
/// succeeded within frozen coverage and the explicit/provider-minimum budgets.
pub fn expand_recurrence(
    template: &NativeValue,
    recurrence: &NativeValue,
    window: &NativeValue,
    schema: &Schema,
    budget: ExpansionBudget,
) -> RecurrenceExpansion {
    let mut output = RecurrenceExpansion {
        status: ExpansionStatus::Complete,
        occurrences: vec![],
        diagnostics: vec![],
        steps: 0,
        providers: vec![],
    };
    if let Err(error) = expand_inner(template, recurrence, window, schema, budget, &mut output) {
        output.status = if error.0.contains("budget") || error.0.contains("coverage") {
            ExpansionStatus::Partial
        } else if error.0.contains("unavailable") {
            ExpansionStatus::Unavailable
        } else {
            ExpansionStatus::Invalid
        };
        output.diagnostics.push(error.0);
        // Incomplete admission cannot expose unproven replacement/negative results.
        output.occurrences.clear();
    }
    output
}
fn expand_inner(
    template_native: &NativeValue,
    recurrence: &NativeValue,
    window: &NativeValue,
    schema: &Schema,
    supplied: ExpansionBudget,
    output: &mut RecurrenceExpansion,
) -> Result<()> {
    let input_bytes = serde_json::to_vec(&(template_native, recurrence, window))?.len() as u64;
    if input_bytes > supplied.max_bytes {
        return Err(failure("Recurrence input byte budget exhausted"));
    }
    let template = exact_range(template_native)?;
    let rule = Rule::parse(recurrence, schema)?;
    let budget = rule.budget(supplied, schema);
    if input_bytes > budget.max_bytes {
        return Err(failure("Recurrence input byte budget exhausted"));
    }
    if budget.max_steps == 0 || budget.max_occurrences == 0 || budget.max_bytes == 0 {
        return Err(failure("Recurrence budget exhausted"));
    }
    output.providers = [rule.calendar_provider, rule.zone_provider]
        .iter()
        .map(|p| format!("{}@{}", p.id, p.data["version"].as_str().unwrap_or("")))
        .collect();
    if let Some(p) = schema
        .providers
        .iter()
        .find(|p| p.implementation.id == "weftext.temporal.rules.v2")
    {
        output
            .providers
            .push(format!("{}@{}", p.id, p.implementation.id));
    }
    // Qualify all finite window/template/provider endpoints before overlap.
    for onset in &template {
        rule.coverage(onset)?;
    }
    let window_start = window.member("start").map(Onset::parse).transpose()?;
    let window_end = window
        .member("end_exclusive")
        .map(Onset::parse)
        .transpose()?;
    if window_start.is_none() && window_end.is_none() {
        return Err(failure("Window needs at least one finite endpoint"));
    }
    let mut domain_values = window_start
        .iter()
        .chain(window_end.iter())
        .chain(rule.until.iter())
        .chain(rule.rdates.iter())
        .chain(rule.exdates.iter())
        .collect::<Vec<_>>();
    let exception_onsets = rule
        .exceptions
        .iter()
        .map(|e| Onset::parse(e.member("onset").unwrap()))
        .collect::<Result<Vec<_>>>()?;
    domain_values.extend(exception_onsets.iter());
    for onset in domain_values {
        if !template[0].same_domain(onset) {
            return Err(failure(
                "Recurrence template/window/date/exception domains disagree",
            ));
        }
        rule.coverage(onset)?;
    }
    for e in &rule.exceptions {
        if let Some(range) = e.member("range") {
            for onset in exact_range(range)? {
                if !template[0].same_domain(&onset) {
                    return Err(failure("Replacement domain disagrees with template"));
                }
                rule.coverage(&onset)?;
            }
        }
    }
    if let (Some(a), Some(b)) = (&window_start, &window_end)
        && !a.compare(b).is_lt()
    {
        return Err(failure("Window reversed or empty"));
    }
    if rule
        .until
        .as_ref()
        .is_some_and(|u| u.compare(&template[0]).is_lt())
    {
        return Err(failure("until cannot precede derived anchor"));
    }
    if window_end.is_none()
        && rule.count.is_none()
        && rule.until.is_none()
        && !rule.empty_selectors()
    {
        return Err(failure(
            "Unbounded recurrence window needs a finite termination bound; unavailable",
        ));
    }
    let local = template[0].civil(&rule.zone)?;
    let anchor_date = local.date();
    let anchor = rule.calendar.parts(anchor_date)?;
    let fraction = match &template[0] {
        Onset::Date(_) => ExactDecimal::parse("0")?,
        Onset::Instant(i) => i.fraction.clone(),
    };
    let mut target = window_end.clone();
    for onset in &exception_onsets {
        if target.as_ref().is_none_or(|t| t.compare(onset).is_lt()) {
            target = Some(onset.clone());
        }
    }
    let mut base = vec![template[0].clone()];
    let mut date = anchor_date;
    let mut generated_bytes = template[0].key()?.len() as u64;
    loop {
        output.steps += 1;
        if output.steps > budget.max_steps {
            return Err(failure("Recurrence proof budget exhausted"));
        }
        let next = date
            .checked_add_days(Days::new(1))
            .ok_or_else(|| failure("Calendar coverage exhausted"))?;
        let future = match &template[0] {
            Onset::Date(_) => Onset::Date(next),
            Onset::Instant(_) => Onset::Instant(ExactInstant {
                utc_seconds: next.and_time(local.time()).and_utc().timestamp()
                    - rule.zone.max_offset,
                fraction: fraction.clone(),
            }),
        };
        let count_done = rule
            .count
            .is_some_and(|n| base.len() >= n && future.compare(&base[n - 1]).is_gt());
        let until_done = rule
            .until
            .as_ref()
            .is_some_and(|u| future.compare(u).is_gt());
        let window_done = target.as_ref().is_some_and(|t| future.compare(t).is_gt());
        if count_done || until_done || window_done || rule.empty_selectors() {
            break;
        }
        date = next;
        let parts = rule.calendar.parts(date)?;
        if rule.matches(date, &parts, anchor_date, &anchor) {
            let onset = match &template[0] {
                Onset::Date(_) => Some(Onset::Date(date)),
                Onset::Instant(_) => rule
                    .zone
                    .instant(
                        date.and_time(local.time()),
                        &fraction,
                        text(rule.native, "fold")?,
                    )?
                    .map(Onset::Instant),
            };
            if let Some(onset) = onset {
                if onset.compare(&template[0]).is_lt() {
                    continue;
                }
                generated_bytes = generated_bytes
                    .checked_add(onset.key()?.len() as u64)
                    .ok_or_else(|| failure("Recurrence byte budget exhausted"))?;
                if generated_bytes > budget.max_bytes {
                    return Err(failure("Recurrence byte budget exhausted"));
                }
                if let Err(index) = base.binary_search_by(|existing| existing.compare(&onset)) {
                    base.insert(index, onset);
                }
            }
        }
    }
    if let Some(until) = &rule.until {
        base.retain(|o| !o.compare(until).is_gt());
    }
    if let Some(count) = rule.count {
        base.truncate(count);
    }
    let mut originals = BTreeMap::new();
    for onset in base.into_iter().chain(rule.rdates.iter().cloned()) {
        originals.insert(onset.key()?, onset);
    }
    let mut exceptions = BTreeMap::new();
    for (exception, onset) in rule.exceptions.iter().zip(&exception_onsets) {
        let key = onset.key()?;
        if !originals.contains_key(&key) {
            return Err(failure(
                "Exception onset is not a member of base union RDATE",
            ));
        }
        exceptions.insert(key, *exception);
    }
    let excluded = rule
        .exdates
        .iter()
        .map(Onset::key)
        .collect::<Result<BTreeSet<_>>>()?;
    for (key, onset) in originals {
        output.steps += 1;
        if output.steps > budget.max_steps {
            return Err(failure("Recurrence decision budget exhausted"));
        }
        let exception = exceptions.get(&key);
        let replacement = exception
            .is_some_and(|e| e.member("action").and_then(NativeValue::text) == Some("replace"));
        if exception
            .is_some_and(|e| e.member("action").and_then(NativeValue::text) == Some("cancel"))
            || !replacement && excluded.contains(&key)
        {
            continue;
        }
        let actual = if let Some(range) = exception.and_then(|e| e.member("range")) {
            range.clone()
        } else {
            template_range(&onset, &template)?
        };
        for value in exact_range(&actual)? {
            rule.coverage(&value)?;
        }
        match range_overlap(&actual, window, schema) {
            Cell::Value {
                value: NativeValue::Bool(true),
            } => {}
            Cell::Value {
                value: NativeValue::Bool(false),
            } => continue,
            Cell::Invalid { diagnostics, .. } => return Err(failure(diagnostics.join("; "))),
            Cell::Unavailable { reason } => {
                return Err(failure(format!("Overlap unavailable: {reason}")));
            }
            _ => return Err(failure("Overlap unavailable")),
        }
        let field = |key: &str| exception.and_then(|e| e.member(key)).cloned();
        output.occurrences.push(RecurrenceOccurrence {
            original_onset: onset.native()?,
            range: actual,
            replacement,
            title: field("title"),
            status: field("status"),
            note: field("note"),
        });
        if output.occurrences.len() as u64 > budget.max_occurrences {
            return Err(failure("Recurrence occurrence budget exhausted"));
        }
    }
    let size = serde_json::to_vec(&output.occurrences)?.len() as u64;
    if size > budget.max_bytes {
        return Err(failure("Recurrence output byte budget exhausted"));
    }
    output.occurrences.sort_by(|a, b| {
        let aa = exact_range(&a.range).unwrap();
        let bb = exact_range(&b.range).unwrap();
        aa[0]
            .compare(&bb[0])
            .then_with(|| aa[1].compare(&bb[1]))
            .then_with(|| {
                Onset::parse(&a.original_onset)
                    .unwrap()
                    .compare(&Onset::parse(&b.original_onset).unwrap())
            })
    });
    Ok(())
}

impl ExpansionBudget {
    /// Closed native budget for the trusted expandRecurrence export. Ordinary
    /// records cannot override provider limits or carry executable instructions.
    pub fn from_native(value: &NativeValue) -> Result<Self> {
        let members = value
            .record()
            .ok_or_else(|| failure("Recurrence budget must be a record"))?;
        let expected = ["max_steps", "max_occurrences", "max_bytes"];
        if members.len() != 3 || members.iter().any(|m| !expected.contains(&m.key.as_str())) {
            return Err(failure("Unknown/missing recurrence budget members"));
        }
        let read = |key| {
            integer(value, key)
                .and_then(|n| u64::try_from(n).map_err(|_| failure("Negative recurrence budget")))
        };
        Ok(Self {
            max_steps: read("max_steps")?,
            max_occurrences: read("max_occurrences")?,
            max_bytes: read("max_bytes")?,
        })
    }
}
impl RecurrenceExpansion {
    pub fn native(&self) -> NativeValue {
        let record = |pairs: Vec<(&str, NativeValue)>| {
            NativeValue::Record(
                pairs
                    .into_iter()
                    .map(|(key, value)| NativeMember {
                        key: key.into(),
                        value,
                    })
                    .collect(),
            )
        };
        let occurrences = self
            .occurrences
            .iter()
            .map(|o| {
                let mut pairs = vec![
                    ("original_onset", o.original_onset.clone()),
                    ("range", o.range.clone()),
                    ("replacement", NativeValue::Bool(o.replacement)),
                ];
                for (key, value) in [
                    ("title", &o.title),
                    ("status", &o.status),
                    ("note", &o.note),
                ] {
                    if let Some(value) = value {
                        pairs.push((key, value.clone()));
                    }
                }
                record(pairs)
            })
            .collect();
        let status = match self.status {
            ExpansionStatus::Complete => "complete",
            ExpansionStatus::Partial => "partial",
            ExpansionStatus::Invalid => "invalid",
            ExpansionStatus::Unavailable => "unavailable",
        };
        let texts = |values: &[String]| {
            NativeValue::Array(
                values
                    .iter()
                    .map(|s| NativeValue::Text(s.clone()))
                    .collect(),
            )
        };
        record(vec![
            ("status", NativeValue::Text(status.into())),
            ("occurrences", NativeValue::Array(occurrences)),
            ("diagnostics", texts(&self.diagnostics)),
            ("steps", NativeValue::Int(self.steps.to_string())),
            ("providers", texts(&self.providers)),
        ])
    }
}
/// The trusted three-argument ABI adapter receives one same-cut Event record.
/// Overrides inherit uncovered title/status/note from the explicit event input.
pub fn expand_event(
    event: &NativeValue,
    window: &NativeValue,
    schema: &Schema,
    mut budget: ExpansionBudget,
) -> RecurrenceExpansion {
    if let Some(provider) = schema
        .providers
        .iter()
        .find(|p| p.implementation.id == "weftext.temporal.rules.v2")
    {
        budget.max_steps = budget.max_steps.min(provider.limits.max_steps);
        budget.max_occurrences = budget.max_occurrences.min(provider.limits.max_expansion);
        budget.max_bytes = budget.max_bytes.min(provider.limits.max_bytes);
    }
    let reject = |status, message: &str| RecurrenceExpansion {
        status,
        occurrences: vec![],
        diagnostics: vec![message.into()],
        steps: 0,
        providers: vec![],
    };
    if serde_json::to_vec(&(event, window)).map_or(true, |v| v.len() as u64 > budget.max_bytes) {
        return reject(
            ExpansionStatus::Partial,
            "Event input byte budget exhausted",
        );
    }
    if ["title", "note"]
        .iter()
        .any(|key| event.member(key).is_some_and(|v| v.text().is_none()))
        || event
            .member("event_status")
            .is_some_and(|v| !matches!(v.text(), Some("tentative" | "confirmed" | "cancelled")))
    {
        return reject(
            ExpansionStatus::Invalid,
            "Event title/note/status input is invalid",
        );
    }
    let Some(range) = event.member("range") else {
        return RecurrenceExpansion {
            status: ExpansionStatus::Invalid,
            occurrences: vec![],
            diagnostics: vec!["Event range required".into()],
            steps: 0,
            providers: vec![],
        };
    };
    let mut result = if let Some(recurrence) = event.member("recurrence") {
        expand_recurrence(range, recurrence, window, schema, budget)
    } else {
        let result = (|| -> Result<Vec<RecurrenceOccurrence>> {
            if budget.max_steps == 0 || budget.max_occurrences == 0 {
                return Err(failure("Event expansion budget exhausted"));
            }
            let endpoints = exact_range(range)?;
            match range_overlap(range, window, schema) {
                Cell::Value {
                    value: NativeValue::Bool(false),
                } => Ok(vec![]),
                Cell::Value {
                    value: NativeValue::Bool(true),
                } => Ok(vec![RecurrenceOccurrence {
                    original_onset: endpoints[0].native()?,
                    range: range.clone(),
                    replacement: false,
                    title: None,
                    status: None,
                    note: None,
                }]),
                Cell::Invalid { diagnostics, .. } => Err(failure(diagnostics.join("; "))),
                Cell::Unavailable { reason } => {
                    Err(failure(format!("Event overlap unavailable: {reason}")))
                }
                _ => Err(failure("Event overlap unavailable")),
            }
        })();
        match result {
            Ok(occurrences) => RecurrenceExpansion {
                status: ExpansionStatus::Complete,
                occurrences,
                diagnostics: vec![],
                steps: 1,
                providers: vec![],
            },
            Err(e) => RecurrenceExpansion {
                status: if e.0.contains("budget") {
                    ExpansionStatus::Partial
                } else if e.0.contains("unavailable") {
                    ExpansionStatus::Unavailable
                } else {
                    ExpansionStatus::Invalid
                },
                occurrences: vec![],
                diagnostics: vec![e.0],
                steps: 0,
                providers: vec![],
            },
        }
    };
    for occurrence in &mut result.occurrences {
        if occurrence.title.is_none() {
            occurrence.title = event.member("title").cloned();
        }
        if occurrence.status.is_none() {
            occurrence.status = event.member("event_status").cloned();
        }
        if occurrence.note.is_none() {
            occurrence.note = event.member("note").cloned();
        }
    }
    if serde_json::to_vec(&result.occurrences).map_or(true, |v| v.len() as u64 > budget.max_bytes) {
        result.status = ExpansionStatus::Partial;
        result
            .diagnostics
            .push("Event output byte budget exhausted".into());
        result.occurrences.clear();
    }
    result
}
