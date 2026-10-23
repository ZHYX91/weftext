//! A single saved task reminder intent. Pure frozen input; no clock or I/O.
use crate::temporal::{FrozenZone, reminder_civil_day};
use crate::*;
use chrono::{NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskReminderState {
    Ready,
    Absent,
    Invalid,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskReminderResolution {
    pub state: TaskReminderState,
    pub reminder_id: Option<String>,
    pub trigger_instant: Option<String>,
    /// Exclusive end of the target civil minute, derived without reducing the
    /// saved trigger's seconds/fraction or consulting a host clock/timezone.
    pub normal_delivery_until: Option<String>,
    pub definition_digest: Option<String>,
    pub providers: BTreeMap<String, String>,
    #[serde(deserialize_with = "required_failure_kind")]
    pub failure_kind: Option<DataErrorKind>,
    pub diagnostics: Vec<ErrorDescription>,
}
impl TaskReminderResolution {
    fn empty(state: TaskReminderState) -> Self {
        Self {
            state,
            reminder_id: None,
            trigger_instant: None,
            normal_delivery_until: None,
            definition_digest: None,
            providers: BTreeMap::new(),
            failure_kind: None,
            diagnostics: vec![],
        }
    }
    pub fn validation(&self) -> Cell {
        match (self.state, self.failure_kind) {
            (TaskReminderState::Ready | TaskReminderState::Absent, None)
                if self.diagnostics.is_empty() =>
            {
                Cell::value(NativeValue::Bool(true))
            }
            (TaskReminderState::Invalid, Some(DataErrorKind::Invalid))
                if !self.diagnostics.is_empty() =>
            {
                Cell::Invalid {
                    diagnostics: self.diagnostics.clone(),
                    value: None,
                }
            }
            (TaskReminderState::Unavailable, Some(kind))
                if kind != DataErrorKind::Invalid && !self.diagnostics.is_empty() =>
            {
                Cell::unavailable_group(
                    kind,
                    UiMessage::error_data_task_reminder_validation_unavailable(),
                    self.diagnostics.clone(),
                )
            }
            _ => {
                let mut diagnostics =
                    vec![UiMessage::error_data_task_reminder_resolution_invalid().into()];
                diagnostics.extend(self.diagnostics.iter().cloned());
                Cell::Invalid {
                    diagnostics,
                    value: None,
                }
            }
        }
    }
}
// A present nullable classification is part of the sole result wire. Missing
// legacy fields are rejected; validation separately rejects inconsistent states.
fn required_failure_kind<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<DataErrorKind>, D::Error> {
    Option::<DataErrorKind>::deserialize(deserializer)
}
fn bad(message: UiMessage) -> DataError {
    DataError::new(message)
}
fn civil_day_error(cell: Cell) -> DataError {
    match cell {
        Cell::Unavailable {
            kind,
            reason,
            diagnostics,
        } => DataError::with_kind(
            kind,
            UiMessage::error_data_task_reminder_civil_due_unavailable(),
        )
        .with_diagnostics(std::iter::once(reason).chain(diagnostics).collect()),
        Cell::Invalid { diagnostics, .. } => {
            bad(UiMessage::error_data_task_reminder_civil_due_invalid())
                .with_diagnostics(diagnostics)
        }
        _ => DataError::unavailable(UiMessage::error_data_task_reminder_civil_due_unavailable()),
    }
}
fn text<'a>(value: &'a NativeValue, key: &str) -> Result<&'a str> {
    value
        .member(key)
        .and_then(NativeValue::text)
        .ok_or_else(|| {
            bad(UiMessage::error_data_task_reminder_member_text_required(
                weftext_messages::params::ErrorDataTaskReminderMemberTextRequired {
                    member: key.into(),
                },
            ))
        })
}
fn closed(value: &NativeValue, required: &[&str], optional: &[&str]) -> Result<()> {
    let members = value
        .record()
        .ok_or_else(|| bad(UiMessage::error_data_task_reminder_direct_record_required()))?;
    let mut names = BTreeSet::new();
    for member in members {
        if !names.insert(member.key.as_str())
            || !required.contains(&member.key.as_str()) && !optional.contains(&member.key.as_str())
        {
            return Err(bad(UiMessage::error_data_task_reminder_members_not_closed()));
        }
    }
    if required.iter().any(|name| !names.contains(name)) {
        return Err(bad(
            UiMessage::error_data_task_reminder_required_member_missing(),
        ));
    }
    Ok(())
}
fn decimal(value: &str) -> Result<(ExactDecimal, String)> {
    let number = ExactDecimal::parse(value)?;
    let mut canonical = number.spelling();
    if canonical.contains('.') {
        canonical = canonical.trim_end_matches('0').trim_end_matches('.').into();
    }
    if canonical == "-0" {
        canonical = "0".into();
    }
    Ok((number, canonical))
}
/// The caller has already frozen/authorized these exact schema providers.
/// Full provider fingerprints are qualification dependencies, not the whole
/// note revision or unrelated schema in the effect definition digest.
pub fn resolve_task_reminder(
    reminder: Option<&NativeValue>,
    due: Option<&NativeValue>,
    schema: &Schema,
) -> TaskReminderResolution {
    let Some(reminder) = reminder else {
        return TaskReminderResolution::empty(TaskReminderState::Absent);
    };
    let mut result = TaskReminderResolution::empty(TaskReminderState::Ready);
    let resolved = (|| -> Result<(String, String)> {
        if serde_json::to_vec(reminder)?.len() > 16 * 1024 {
            return Err(bad(UiMessage::error_data_task_reminder_intent_byte_budget()));
        }
        let id = text(reminder, "id")?;
        if !uuid_v4(id) {
            return Err(bad(UiMessage::error_data_task_reminder_identity_invalid()));
        }
        result.reminder_id = Some(id.into());
        let kind = text(reminder, "kind")?;
        let (instant, meaning, minute_second) = match kind {
            "absolute" => {
                closed(reminder, &["id", "kind", "at"], &[])?;
                let parsed = ExactInstant::parse(text(reminder, "at")?)?;
                let instant = parsed.canonical()?;
                (
                    instant.clone(),
                    serde_json::json!({"kind":kind,"at":instant}),
                    parsed.utc_seconds.rem_euclid(60),
                )
            }
            "due_before" => {
                closed(reminder, &["id", "kind", "before"], &[])?;
                let due = due.and_then(NativeValue::text).ok_or_else(|| {
                    bad(UiMessage::error_data_task_reminder_instant_due_required())
                })?;
                let due = ExactInstant::parse(due)?;
                let (before, canonical) = decimal(text(reminder, "before")?)?;
                if before.compare(&ExactDecimal::parse("0")?).is_lt()
                    || before
                        .compare(&ExactDecimal::parse("128849018820")?)
                        .is_gt()
                {
                    return Err(bad(
                        UiMessage::error_data_task_reminder_advance_out_of_range(),
                    ));
                }
                let (whole, fraction) = canonical
                    .split_once('.')
                    .unwrap_or((canonical.as_str(), ""));
                let whole = whole.parse::<i64>().map_err(|error| {
                    bad(UiMessage::error_data_task_reminder_advance_overflow())
                        .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
                })?;
                let mut fraction = ExactDecimal::parse(&format!(
                    "0.{}",
                    if fraction.is_empty() { "0" } else { fraction }
                ))?;
                // Negating an admitted value is arithmetic, not a new authored
                // spelling. An extra sign must not exhaust the input codec's
                // bound for a valid maximum-precision advance.
                fraction.coefficient = -fraction.coefficient;
                let parsed = due.shifted(-whole, &fraction)?;
                let trigger = parsed.canonical()?;
                (
                    trigger,
                    serde_json::json!({"kind":kind,"before":canonical,"due":due.canonical()?}),
                    parsed.utc_seconds.rem_euclid(60),
                )
            }
            "due_day" => {
                closed(
                    reminder,
                    &["id", "kind", "clock", "zone", "zone_version"],
                    &["fold"],
                )?;
                let due = due.ok_or_else(|| {
                    bad(UiMessage::error_data_task_reminder_due_day_input_required())
                })?;
                let day = reminder_civil_day(due, schema).map_err(civil_day_error)?;
                if let Some(calendar) = due.member("calendar").and_then(NativeValue::text) {
                    let version = text(due, "version")?;
                    let provider = schema
                        .providers
                        .iter()
                        .find(|p| {
                            p.data["key"].as_str() == Some(calendar)
                                && p.data["version"].as_str() == Some(version)
                                && matches!(
                                    p.data["kind"].as_str(),
                                    Some("gregorian" | "calendar-table")
                                )
                        })
                        .ok_or_else(|| {
                            DataError::unavailable(
                                UiMessage::error_data_task_reminder_calendar_provider_unavailable(),
                            )
                        })?;
                    result
                        .providers
                        .insert(provider.id.clone(), provider_fingerprint(provider)?);
                }
                let clock = reminder
                    .member("clock")
                    .ok_or_else(|| bad(UiMessage::error_data_task_reminder_clock_missing()))?;
                closed(clock, &["time"], &["fraction"])?;
                let Some(NativeValue::Time(clock_text)) = clock.member("time") else {
                    return Err(bad(
                        UiMessage::error_data_task_reminder_native_time_required(),
                    ));
                };
                let time = NaiveTime::parse_from_str(clock_text, "%H:%M:%S").map_err(|error| {
                    bad(UiMessage::error_data_task_reminder_clock_invalid())
                        .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
                })?;
                if time.format("%H:%M:%S").to_string() != *clock_text {
                    return Err(bad(
                        UiMessage::error_data_task_reminder_clock_not_canonical(),
                    ));
                }
                if time.nanosecond() != 0 || time.second() > 59 {
                    return Err(bad(
                        UiMessage::error_data_task_reminder_clock_leap_second_unsupported(),
                    ));
                }
                let (fraction, fraction_text) = decimal(
                    clock
                        .member("fraction")
                        .map(|_| text(clock, "fraction"))
                        .transpose()?
                        .unwrap_or("0"),
                )?;
                if fraction.compare(&ExactDecimal::parse("0")?).is_lt()
                    || !fraction.compare(&ExactDecimal::parse("1")?).is_lt()
                {
                    return Err(bad(
                        UiMessage::error_data_task_reminder_fraction_out_of_range(),
                    ));
                }
                let zone_key = text(reminder, "zone")?;
                let zone_version = text(reminder, "zone_version")?;
                let provider = schema
                    .providers
                    .iter()
                    .find(|p| {
                        p.data["kind"].as_str() == Some("zone-table")
                            && p.data["key"].as_str() == Some(zone_key)
                            && p.data["version"].as_str() == Some(zone_version)
                    })
                    .ok_or_else(|| {
                        DataError::unavailable(
                            UiMessage::error_data_task_reminder_zone_provider_unavailable(),
                        )
                    })?;
                result
                    .providers
                    .insert(provider.id.clone(), provider_fingerprint(provider)?);
                let zone = FrozenZone::new(provider)?;
                let candidates = zone.instant_candidates(day.and_time(time), &fraction)?;
                let fold = reminder
                    .member("fold")
                    .map(|_| text(reminder, "fold"))
                    .transpose()?;
                if fold.is_some_and(|f| !matches!(f, "earlier" | "later")) {
                    return Err(bad(UiMessage::error_data_task_reminder_fold_invalid()));
                }
                let selected = match (candidates.len(), fold) {
                    (0, _) => {
                        return Err(bad(UiMessage::error_data_task_reminder_civil_time_in_gap()));
                    }
                    (1, _) => &candidates[0],
                    (_, Some("earlier")) => &candidates[0],
                    (_, Some("later")) => candidates.last().unwrap(),
                    _ => {
                        return Err(bad(
                            UiMessage::error_data_task_reminder_civil_time_ambiguous(),
                        ));
                    }
                };
                // A fold choice which makes no difference does not produce a
                // new effect. Rule and calendar dependencies still requalify.
                let trigger = selected.canonical()?;
                result.normal_delivery_until = Some(
                    zone.reminder_minute_end(selected, day.and_time(time))?
                        .canonical()?,
                );
                (
                    trigger.clone(),
                    serde_json::json!({"kind":kind,"day":day.format("%Y-%m-%d").to_string(),"clock":clock_text,"fraction":fraction_text,"zone":zone_key,"zoneVersion":zone_version,"trigger":trigger}),
                    i64::from(time.second()),
                )
            }
            _ => return Err(bad(UiMessage::error_data_task_reminder_kind_unknown())),
        };
        let trigger = ExactInstant::parse(&instant)?;
        if result.normal_delivery_until.is_none() {
            result.normal_delivery_until = Some(
                ExactInstant {
                    utc_seconds: trigger
                        .utc_seconds
                        .checked_add(60 - minute_second)
                        .ok_or_else(
                            || bad(UiMessage::error_data_task_reminder_minute_overflow()),
                        )?,
                    fraction: ExactDecimal::parse("0")?,
                }
                .canonical()?,
            );
        }
        let bytes = serde_json::to_vec(
            &serde_json::json!({"profile":"weftext.task-reminder.v1","meaning":meaning}),
        )?;
        Ok((instant, format!("{:x}", Sha256::digest(bytes))))
    })();
    match resolved {
        Ok((instant, digest)) => {
            result.trigger_instant = Some(instant);
            result.definition_digest = Some(digest);
        }
        Err(error) => {
            result.normal_delivery_until = None;
            result.failure_kind = Some(error.kind());
            result.state = if error.is_unavailable() {
                TaskReminderState::Unavailable
            } else {
                TaskReminderState::Invalid
            };
            result.diagnostics = error.into_diagnostics();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(value: &str) -> NativeValue {
        NativeValue::Text(value.into())
    }
    fn r(members: &[(&str, NativeValue)]) -> NativeValue {
        NativeValue::Record(
            members
                .iter()
                .map(|(key, value)| NativeMember {
                    key: (*key).into(),
                    value: value.clone(),
                })
                .collect(),
        )
    }
    fn id() -> NativeValue {
        t("a0358e3e-5784-4a8b-8e5f-146029b70dbd")
    }
    fn absolute(at: &str) -> NativeValue {
        r(&[("id", id()), ("kind", t("absolute")), ("at", t(at))])
    }
    fn day(
        clock: &str,
        fraction: &str,
        zone: &str,
        version: &str,
        fold: Option<&str>,
    ) -> NativeValue {
        let mut value = r(&[
            ("id", id()),
            ("kind", t("due_day")),
            (
                "clock",
                r(&[
                    ("time", NativeValue::Time(clock.into())),
                    ("fraction", t(fraction)),
                ]),
            ),
            ("zone", t(zone)),
            ("zone_version", t(version)),
        ]);
        if let Some(fold) = fold
            && let NativeValue::Record(m) = &mut value
        {
            m.push(NativeMember {
                key: "fold".into(),
                value: t(fold),
            });
        }
        value
    }
    #[test]
    fn absolute_is_exact_and_independent_of_due_and_unrelated_schema() {
        let schema = Schema::bundled().unwrap();
        let reminder = absolute("2026-10-23T09:00:12.123456789123+08:00");
        let a = resolve_task_reminder(Some(&reminder), None, &schema);
        assert_eq!(a.state, TaskReminderState::Ready);
        assert_eq!(
            a.trigger_instant.as_deref(),
            Some("2026-10-23T01:00:12.123456789123Z")
        );
        assert_eq!(
            a.normal_delivery_until.as_deref(),
            Some("2026-10-23T01:01:00Z")
        );
        let b = resolve_task_reminder(
            Some(&absolute("2026-10-23T01:00:12.1234567891230Z")),
            Some(&NativeValue::Date("2026-10-25".into())),
            &schema,
        );
        assert_eq!(a.definition_digest, b.definition_digest);
        assert!(a.providers.is_empty());
        assert_eq!(
            resolve_task_reminder(None, None, &schema).state,
            TaskReminderState::Absent
        );
    }
    #[test]
    fn relative_preserves_subnanosecond_due_and_advance() {
        let schema = Schema::bundled().unwrap();
        let reminder = r(&[
            ("id", id()),
            ("kind", t("due_before")),
            ("before", t("1800.000000000001")),
        ]);
        let a = resolve_task_reminder(
            Some(&reminder),
            Some(&t("2026-10-23T17:00:12.123456789123+08:00")),
            &schema,
        );
        assert_eq!(
            a.trigger_instant.as_deref(),
            Some("2026-10-23T08:30:12.123456789122Z")
        );
        assert_eq!(
            resolve_task_reminder(
                Some(&reminder),
                Some(&NativeValue::Date("2026-10-23".into())),
                &schema
            )
            .state,
            TaskReminderState::Invalid
        );
        let negative = r(&[("id", id()), ("kind", t("due_before")), ("before", t("-1"))]);
        assert_eq!(
            resolve_task_reminder(Some(&negative), Some(&t("2026-10-23T17:00:00Z")), &schema).state,
            TaskReminderState::Invalid
        );
    }
    #[test]
    fn relative_maximum_admitted_precision_is_not_rejected_by_internal_negation() {
        let schema = Schema::bundled().unwrap();
        let before = format!("0.{}1", "0".repeat(4093));
        assert_eq!(before.len(), 4096);
        ExactDecimal::parse(&before).unwrap();
        let reminder = r(&[
            ("id", id()),
            ("kind", t("due_before")),
            ("before", t(&before)),
        ]);
        let resolution =
            resolve_task_reminder(Some(&reminder), Some(&t("2026-10-23T09:00:00Z")), &schema);
        assert_eq!(resolution.state, TaskReminderState::Ready);
        assert_eq!(
            resolution.trigger_instant,
            Some(format!("2026-10-23T08:59:59.{}Z", "9".repeat(4094)))
        );
        assert_eq!(
            resolution.normal_delivery_until.as_deref(),
            Some("2026-10-23T09:00:00Z")
        );
    }
    #[test]
    fn complete_day_clock_and_fraction_use_exact_installed_zone() {
        let schema = Schema::bundled().unwrap();
        let reminder = day("09:00:12", "0.123456789123", "UTC", "fixed-utc.v1", None);
        let due = NativeValue::Date("2026-10-23".into());
        let a = resolve_task_reminder(Some(&reminder), Some(&due), &schema);
        assert_eq!(
            a.trigger_instant.as_deref(),
            Some("2026-10-23T09:00:12.123456789123Z")
        );
        assert_eq!(a.providers.len(), 1);
        assert_eq!(
            resolve_task_reminder(
                Some(&day(
                    "09:00:00",
                    "0",
                    "Asia/Shanghai",
                    "not-installed",
                    None
                )),
                Some(&due),
                &schema
            )
            .state,
            TaskReminderState::Unavailable
        );
        let month = r(&[
            ("calendar", t("gregorian")),
            ("version", t("proleptic-gregorian.v1")),
            ("precision", t("month")),
            ("year", NativeValue::Int("2026".into())),
            ("month", NativeValue::Int("10".into())),
        ]);
        assert_eq!(
            resolve_task_reminder(Some(&reminder), Some(&month), &schema).state,
            TaskReminderState::Invalid
        );
    }
    #[test]
    fn dst_gap_is_invalid_and_ambiguous_fold_needs_explicit_choice() {
        let mut schema = Schema::bundled().unwrap();
        let mut zone = schema
            .providers
            .iter()
            .find(|p| p.id == "zones/utc")
            .unwrap()
            .clone();
        zone.id = "zones/new-york-test".into();
        zone.data = serde_json::json!({"kind":"zone-table","key":"America/New_York","version":"test-2026","from":"2026-01-01T00:00:00Z","until":"2027-01-01T00:00:00Z","initial_offset_seconds":-18000,"transitions":[{"at":"2026-03-08T07:00:00Z","offset_seconds":-14400},{"at":"2026-11-01T06:00:00Z","offset_seconds":-18000}]});
        schema.providers.push(zone);
        let resolve = |clock: &str, date: &str, fold| {
            resolve_task_reminder(
                Some(&day(clock, "0", "America/New_York", "test-2026", fold)),
                Some(&NativeValue::Date(date.into())),
                &schema,
            )
        };
        assert_eq!(
            resolve("02:30:00", "2026-03-08", None).state,
            TaskReminderState::Invalid
        );
        assert_eq!(
            resolve("01:30:00", "2026-11-01", None).state,
            TaskReminderState::Invalid
        );
        assert_eq!(
            resolve("01:30:00", "2026-11-01", Some("earlier"))
                .trigger_instant
                .as_deref(),
            Some("2026-11-01T05:30:00Z")
        );
        assert_eq!(
            resolve("01:30:00", "2026-11-01", Some("later"))
                .trigger_instant
                .as_deref(),
            Some("2026-11-01T06:30:00Z")
        );
        assert_eq!(
            resolve("09:00:00", "2028-11-01", None).state,
            TaskReminderState::Unavailable
        );
    }
    #[test]
    fn unknown_duplicate_and_malformed_intents_never_execute() {
        let schema = Schema::bundled().unwrap();
        let mut bad = absolute("2026-10-23T09:00:00Z");
        if let NativeValue::Record(m) = &mut bad {
            m.push(NativeMember {
                key: "channel".into(),
                value: t("shell"),
            });
        }
        assert_eq!(
            resolve_task_reminder(Some(&bad), None, &schema).state,
            TaskReminderState::Invalid
        );
        assert_eq!(
            resolve_task_reminder(Some(&absolute("2026-10-23T09:00:00-00:00")), None, &schema)
                .state,
            TaskReminderState::Invalid
        );
        if let NativeValue::Record(m) = &mut bad {
            m.pop();
            m.push(NativeMember {
                key: "at".into(),
                value: t("2026-10-23T09:00:00Z"),
            });
        }
        assert_eq!(
            resolve_task_reminder(Some(&bad), None, &schema).state,
            TaskReminderState::Invalid
        );
    }
    #[test]
    fn target_minute_follows_second_level_zone_transitions_and_coverage() {
        let mut schema = Schema::bundled().unwrap();
        let mut zone = schema
            .providers
            .iter()
            .find(|p| p.id == "zones/utc")
            .unwrap()
            .clone();
        zone.id = "zones/second-transition-test".into();
        zone.data = serde_json::json!({"kind":"zone-table","key":"test/seconds","version":"v1","from":"2026-01-01T00:00:00Z","until":"2027-01-01T00:00:00Z","initial_offset_seconds":0,"transitions":[{"at":"2026-10-23T09:00:45Z","offset_seconds":30}]});
        schema.providers.push(zone.clone());
        let resolve = |schema: &Schema, fold| {
            resolve_task_reminder(
                Some(&day("09:00:30", "0", "test/seconds", "v1", fold)),
                Some(&NativeValue::Date("2026-10-23".into())),
                schema,
            )
        };
        assert_eq!(
            resolve(&schema, None).normal_delivery_until.as_deref(),
            Some("2026-10-23T09:00:45Z")
        );
        schema.providers.last_mut().unwrap().data["transitions"] =
            serde_json::json!([{"at":"2026-10-23T09:01:00Z","offset_seconds":-30}]);
        assert_eq!(
            resolve(&schema, Some("earlier"))
                .normal_delivery_until
                .as_deref(),
            Some("2026-10-23T09:01:30Z")
        );
        schema.providers.last_mut().unwrap().data["transitions"] = serde_json::json!([]);
        schema.providers.last_mut().unwrap().data["until"] = "2026-10-23T09:00:45Z".into();
        assert_eq!(resolve(&schema, None).state, TaskReminderState::Unavailable);
    }
    #[test]
    fn leap_clock_and_fraction_outside_one_second_are_rejected() {
        let schema = Schema::bundled().unwrap();
        let due = NativeValue::Date("2026-10-23".into());
        for (clock, fraction) in [("23:59:60", "0"), ("09:00:00", "1"), ("09:00:00", "-0.1")] {
            assert_eq!(
                resolve_task_reminder(
                    Some(&day(clock, fraction, "UTC", "fixed-utc.v1", None)),
                    Some(&due),
                    &schema
                )
                .state,
                TaskReminderState::Invalid
            );
        }
    }
    #[test]
    fn real_task_projection_keeps_reminder_id_and_exact_author_values() {
        let schema = Schema::bundled().unwrap();
        let intent = r(&[
            ("id", id()),
            ("kind", t("due_before")),
            ("before", t("60.000000000001")),
        ]);
        let fields = BTreeMap::from([
            ("tasks/status".into(), t("open")),
            (
                "tasks/due".into(),
                t("2026-10-23T17:00:12.123456789123+08:00"),
            ),
            ("tasks/reminder".into(), intent.clone()),
        ]);
        let source = crate::capture_source(
            "6fc0e4b4-8a30-4dfe-b093-0d334f24d8ad",
            &["task".into()],
            &fields,
            "正文😀",
            &schema,
        )
        .unwrap();
        let projected = crate::project(&source, &schema);
        assert_eq!(projected.qualification, "qualified");
        let reminder = projected
            .fields
            .iter()
            .find(|f| f.field_id == "tasks/reminder")
            .unwrap();
        assert!(matches!(&reminder.cell,Cell::Value{value}if value==&intent));
        let valid = crate::validate_note(&source, &schema, &crate::ValidationContext::default());
        assert!(valid.is_valid(), "{valid:?}");
        let date_only = source.replace("2026-10-23T17:00:12.123456789123+08:00", "2026-10-23");
        assert!(
            !crate::validate_field_change(
                &source,
                &date_only,
                &schema,
                "tasks/due",
                &crate::ValidationContext::default()
            )
            .is_valid()
        );
        let unrelated = source.replace("正文😀", "只改正文，原提醒不变");
        let p = crate::project(&unrelated, &schema);
        assert_eq!(
            projected
                .fields
                .iter()
                .find(|f| f.field_id == "tasks/reminder")
                .unwrap()
                .cell,
            p.fields
                .iter()
                .find(|f| f.field_id == "tasks/reminder")
                .unwrap()
                .cell
        );
    }

    #[test]
    fn reminder_failure_facts_and_parallel_diagnostics_do_not_depend_on_message_text_or_locale() {
        let mut schema = Schema::bundled().unwrap();
        let zone = schema
            .providers
            .iter_mut()
            .find(|p| p.id == "zones/utc")
            .unwrap();
        zone.data["from"] = "2026-01-01T00:00:00Z".into();
        zone.data["until"] = "2026-10-23T09:00:45Z".into();
        let intent = day("09:00:30", "0", "UTC", "fixed-utc.v1", None);
        let original = serde_json::to_value(&intent).unwrap();
        let resolution = resolve_task_reminder(
            Some(&intent),
            Some(&NativeValue::Date("2026-10-23".into())),
            &schema,
        );
        assert_eq!(resolution.state, TaskReminderState::Unavailable);
        assert_eq!(resolution.failure_kind, Some(DataErrorKind::Coverage));
        assert_eq!(
            resolution.diagnostics[0].explanation,
            UiMessage::error_data_task_reminder_minute_coverage_unavailable()
        );
        let Cell::Unavailable {
            kind, diagnostics, ..
        } = resolution.validation()
        else {
            panic!("incomplete minute coverage remains unavailable")
        };
        assert_eq!(kind, DataErrorKind::Coverage);
        assert_eq!(diagnostics, resolution.diagnostics);
        let wire = serde_json::to_value(&resolution).unwrap();
        for diagnostic in &resolution.diagnostics {
            let _ = diagnostic
                .explanation
                .render(weftext_messages::UiLocale::ZhCn);
            let _ = diagnostic
                .explanation
                .render(weftext_messages::UiLocale::EnUs);
        }
        assert_eq!(serde_json::to_value(&resolution).unwrap(), wire);
        assert_eq!(serde_json::to_value(&intent).unwrap(), original);

        let mut oversized = absolute("2026-10-23T09:00:00Z");
        if let NativeValue::Record(members) = &mut oversized {
            members.push(NativeMember {
                key: "authorRaw".into(),
                value: t(&"作者原值".repeat(6000)),
            });
        }
        let original = serde_json::to_value(&oversized).unwrap();
        let result = resolve_task_reminder(Some(&oversized), None, &schema);
        assert_eq!(result.state, TaskReminderState::Invalid);
        assert_eq!(result.failure_kind, Some(DataErrorKind::Invalid));
        assert_eq!(
            result.diagnostics[0].explanation,
            UiMessage::error_data_task_reminder_intent_byte_budget()
        );
        assert_eq!(serde_json::to_value(&oversized).unwrap(), original);
    }

    #[test]
    fn reminder_result_wire_is_typed_required_and_inconsistent_results_fail_closed() {
        let schema = Schema::bundled().unwrap();
        let resolution = resolve_task_reminder(
            Some(&day("09:00:00", "0", "missing", "missing", None)),
            Some(&NativeValue::Date("2026-10-23".into())),
            &schema,
        );
        assert_eq!(resolution.failure_kind, Some(DataErrorKind::Unavailable));
        let wire = serde_json::to_value(&resolution).unwrap();
        let decoded: TaskReminderResolution = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), wire);
        let mut missing = wire.clone();
        missing.as_object_mut().unwrap().remove("failureKind");
        assert!(serde_json::from_value::<TaskReminderResolution>(missing).is_err());
        let mut legacy = wire.clone();
        legacy["diagnostics"] = serde_json::json!(["legacy product text"]);
        assert!(serde_json::from_value::<TaskReminderResolution>(legacy).is_err());
        let mut inconsistent = wire;
        inconsistent["failureKind"] = serde_json::Value::Null;
        let inconsistent: TaskReminderResolution = serde_json::from_value(inconsistent).unwrap();
        let Cell::Invalid { diagnostics, .. } = inconsistent.validation() else {
            panic!("an inconsistent derived result must reject instead of guessing a category")
        };
        assert_eq!(
            diagnostics[0].explanation,
            UiMessage::error_data_task_reminder_resolution_invalid()
        );
    }

    #[test]
    fn reminder_original_clock_diagnostic_and_civil_day_parallel_failures_remain_structured() {
        let schema = Schema::bundled().unwrap();
        let original = NaiveTime::parse_from_str("not a clock", "%H:%M:%S")
            .unwrap_err()
            .to_string();
        let resolution = resolve_task_reminder(
            Some(&day("not a clock", "0", "UTC", "fixed-utc.v1", None)),
            Some(&NativeValue::Date("2026-10-23".into())),
            &schema,
        );
        assert_eq!(resolution.state, TaskReminderState::Invalid);
        assert_eq!(
            resolution.diagnostics[0].explanation,
            UiMessage::error_data_task_reminder_clock_invalid()
        );
        assert_eq!(
            resolution.diagnostics[0].diagnostics,
            vec![RawDiagnostic {
                origin: DiagnosticOrigin::Dependency,
                text: original
            }]
        );
        let parallel = vec![resolution.diagnostics[0].clone(); 96];
        let error = civil_day_error(Cell::unavailable_group(
            DataErrorKind::Coverage,
            UiMessage::error_data_task_reminder_minute_coverage_unavailable(),
            parallel.clone(),
        ));
        assert_eq!(error.kind(), DataErrorKind::Coverage);
        let (_, _, Some(DataErrorDetails::Diagnostics { diagnostics })) = error.into_parts() else {
            panic!("parallel source evidence must be retained")
        };
        assert_eq!(diagnostics.len(), 97);
        assert_eq!(diagnostics[1..], parallel);
        assert!(
            diagnostics
                .iter()
                .all(|description| description.causes.as_slice().is_empty())
        );
    }
}
