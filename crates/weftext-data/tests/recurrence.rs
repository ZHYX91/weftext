use serde_json::json;
use weftext_data::*;
mod support;
use support::assert_data_error;
fn date(s: &str) -> NativeValue {
    NativeValue::Date(s.into())
}
fn instant(s: &str) -> NativeValue {
    NativeValue::Text(s.into())
}
fn record(pairs: Vec<(&str, NativeValue)>) -> NativeValue {
    NativeValue::Record(
        pairs
            .into_iter()
            .map(|(key, value)| NativeMember {
                key: key.into(),
                value,
            })
            .collect(),
    )
}
fn set(value: &mut NativeValue, key: &str, new: NativeValue) {
    let NativeValue::Record(members) = value else {
        panic!("record")
    };
    if let Some(member) = members.iter_mut().find(|m| m.key == key) {
        member.value = new;
    } else {
        members.push(NativeMember {
            key: key.into(),
            value: new,
        });
    }
}
fn range(a: NativeValue, b: NativeValue) -> NativeValue {
    record(vec![("start", a), ("end_exclusive", b)])
}
fn rule(frequency: &str, count: Option<i64>) -> NativeValue {
    let mut r = record(vec![
        ("frequency", instant(frequency)),
        ("interval", NativeValue::Int("1".into())),
        ("calendar", instant("gregorian")),
        ("calendar_version", instant("proleptic-gregorian.v1")),
        ("zone", instant("UTC")),
        ("zone_version", instant("fixed-utc.v1")),
        ("fold", instant("earlier")),
        ("gap", instant("skip")),
    ]);
    if let Some(n) = count {
        set(&mut r, "count", NativeValue::Int(n.to_string()));
    }
    r
}
fn dates(a: &[&str]) -> NativeValue {
    NativeValue::Array(a.iter().map(|s| date(s)).collect())
}
fn ints(a: &[i64]) -> NativeValue {
    NativeValue::Array(a.iter().map(|n| NativeValue::Int(n.to_string())).collect())
}
fn run(
    a: &str,
    b: &str,
    rule: &NativeValue,
    x: &str,
    y: &str,
    schema: &Schema,
) -> RecurrenceExpansion {
    expand_recurrence(
        &range(date(a), date(b)),
        rule,
        &range(date(x), date(y)),
        schema,
        ExpansionBudget::default(),
    )
}
fn originals(result: &RecurrenceExpansion) -> Vec<String> {
    result
        .occurrences
        .iter()
        .map(|o| match &o.original_onset {
            NativeValue::Date(s) | NativeValue::Text(s) => s.clone(),
            _ => panic!(),
        })
        .collect()
}
fn complete(result: &RecurrenceExpansion) {
    assert_eq!(
        result.status,
        ExpansionStatus::Complete,
        "{:?}",
        result.diagnostics
    );
}
fn exception(onset: NativeValue, action: &str, new_range: Option<NativeValue>) -> NativeValue {
    let mut e = record(vec![("onset", onset), ("action", instant(action))]);
    if let Some(r) = new_range {
        set(&mut e, "range", r);
    }
    e
}
fn zoned_schema(from: &str, until: &str, initial: i64, transitions: serde_json::Value) -> Schema {
    let mut s = Schema::bundled().unwrap();
    let zone = s
        .providers
        .iter_mut()
        .find(|p| p.id == "zones/utc")
        .unwrap();
    zone.data = json!({"kind":"zone-table","key":"Test/Eastern","version":"synthetic-dst.v1","from":from,"until":until,"initial_offset_seconds":initial,"transitions":transitions});
    Schema::from_author_json(&serde_json::to_string(&s).unwrap()).unwrap()
}
fn zone_rule(frequency: &str, count: i64, fold: &str) -> NativeValue {
    let mut r = rule(frequency, Some(count));
    set(&mut r, "zone", instant("Test/Eastern"));
    set(&mut r, "zone_version", instant("synthetic-dst.v1"));
    set(&mut r, "fold", instant(fold));
    r
}

#[test]
fn count_precedes_exclusions_and_rdate_does_not_consume_it() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("daily", Some(3));
    set(&mut r, "exDates", dates(&["2026-01-02", "2026-12-01"]));
    set(
        &mut r,
        "rDates",
        dates(&["2025-12-30", "2026-01-10", "2026-01-10"]),
    );
    let result = run(
        "2026-01-01",
        "2026-01-02",
        &r,
        "2025-12-01",
        "2026-02-01",
        &schema,
    );
    complete(&result);
    assert_eq!(
        originals(&result),
        ["2025-12-30", "2026-01-01", "2026-01-03", "2026-01-10"]
    );
    assert!(!result.providers.is_empty());
}
#[test]
fn monthly_and_yearly_invalid_dates_skip_without_clamp_or_count() {
    let schema = Schema::bundled().unwrap();
    let result = run(
        "2026-01-31",
        "2026-02-01",
        &rule("monthly", Some(3)),
        "2026-01-01",
        "2026-07-01",
        &schema,
    );
    complete(&result);
    assert_eq!(
        originals(&result),
        ["2026-01-31", "2026-03-31", "2026-05-31"]
    );
    let result = run(
        "2024-02-29",
        "2024-03-01",
        &rule("yearly", Some(2)),
        "2024-01-01",
        "2030-01-01",
        &schema,
    );
    complete(&result);
    assert_eq!(originals(&result), ["2024-02-29", "2028-02-29"]);
}
#[test]
fn anchor_survives_explicit_empty_selector_and_ordinal_refinement_is_month_relative() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("daily", Some(8));
    set(&mut r, "byMonth", ints(&[]));
    let result = run(
        "2026-01-01",
        "2026-01-02",
        &r,
        "2026-01-01",
        "2027-01-01",
        &schema,
    );
    complete(&result);
    assert_eq!(originals(&result), ["2026-01-01"]);
    let mut r = rule("monthly", Some(4));
    set(
        &mut r,
        "byWeekday",
        NativeValue::Array(vec![record(vec![
            ("day", instant("MO")),
            ("ordinal", NativeValue::Int("-1".into())),
        ])]),
    );
    let result = run(
        "2026-01-01",
        "2026-01-02",
        &r,
        "2026-01-01",
        "2026-04-01",
        &schema,
    );
    complete(&result);
    assert_eq!(
        originals(&result),
        ["2026-01-01", "2026-01-26", "2026-02-23", "2026-03-30"]
    );
}
#[test]
fn weekly_phase_uses_declared_week_start_and_dimensions_intersect() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("weekly", Some(3));
    set(&mut r, "interval", NativeValue::Int("2".into()));
    set(&mut r, "weekStart", instant("SU"));
    set(
        &mut r,
        "byWeekday",
        NativeValue::Array(vec![record(vec![("day", instant("MO"))])]),
    );
    let result = run(
        "2026-01-03",
        "2026-01-04",
        &r,
        "2026-01-01",
        "2026-02-01",
        &schema,
    );
    complete(&result);
    assert_eq!(
        originals(&result),
        ["2026-01-03", "2026-01-12", "2026-01-26"]
    );
    set(&mut r, "byMonthDay", ints(&[-1]));
    let result = run(
        "2026-01-03",
        "2026-01-04",
        &r,
        "2026-01-01",
        "2026-02-01",
        &schema,
    );
    complete(&result);
    assert_eq!(originals(&result), ["2026-01-03"]);
}
#[test]
fn inclusive_until_applies_to_base_and_window_is_half_open() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("daily", None);
    set(&mut r, "until", date("2026-01-03"));
    set(&mut r, "rDates", dates(&["2026-01-10"]));
    let result = run(
        "2026-01-01",
        "2026-01-02",
        &r,
        "2026-01-02",
        "2026-01-10",
        &schema,
    );
    complete(&result);
    assert_eq!(originals(&result), ["2026-01-02", "2026-01-03"]);
    set(&mut r, "until", date("2025-12-31"));
    assert_eq!(
        validate_event(
            &range(date("2026-01-01"), date("2026-01-02")),
            Some(&r),
            &schema
        )
        .state(),
        "invalid"
    );
}
#[test]
fn moved_in_replacement_wins_exdate_and_retains_original_key() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("daily", Some(31));
    set(&mut r, "exDates", dates(&["2026-01-15"]));
    set(
        &mut r,
        "exceptions",
        NativeValue::Array(vec![exception(
            date("2026-01-15"),
            "replace",
            Some(range(date("2026-03-05"), date("2026-03-06"))),
        )]),
    );
    let result = run(
        "2026-01-01",
        "2026-01-02",
        &r,
        "2026-03-01",
        "2026-03-10",
        &schema,
    );
    complete(&result);
    assert_eq!(originals(&result), ["2026-01-15"]);
    assert!(result.occurrences[0].replacement);
    assert_eq!(
        result.occurrences[0].range.member("start"),
        Some(&date("2026-03-05"))
    );
    assert_eq!(
        validate_event(
            &range(date("2026-01-01"), date("2026-01-02")),
            Some(&r),
            &schema
        )
        .state(),
        "value"
    );
}
#[test]
fn moved_out_replacement_does_not_leave_ghost_original_and_cancel_wins() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("daily", Some(10));
    set(
        &mut r,
        "exceptions",
        NativeValue::Array(vec![
            exception(
                date("2026-01-05"),
                "replace",
                Some(range(date("2026-03-05"), date("2026-03-06"))),
            ),
            exception(date("2026-01-06"), "cancel", None),
        ]),
    );
    let result = run(
        "2026-01-01",
        "2026-01-02",
        &r,
        "2026-01-01",
        "2026-01-10",
        &schema,
    );
    complete(&result);
    assert_eq!(result.occurrences.len(), 7);
    assert!(!originals(&result).contains(&"2026-01-05".into()));
    assert!(!originals(&result).contains(&"2026-01-06".into()));
}
#[test]
fn title_only_replacement_inherits_range_and_is_not_removed_by_exdate() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("daily", Some(2));
    let mut e = exception(date("2026-01-02"), "replace", None);
    set(&mut e, "title", instant("Moved title"));
    set(&mut r, "exceptions", NativeValue::Array(vec![e]));
    set(&mut r, "exDates", dates(&["2026-01-02"]));
    let result = run(
        "2026-01-01",
        "2026-01-03",
        &r,
        "2026-01-01",
        "2026-01-10",
        &schema,
    );
    complete(&result);
    assert_eq!(result.occurrences[1].title, Some(instant("Moved title")));
    assert_eq!(
        result.occurrences[1].range,
        range(date("2026-01-02"), date("2026-01-04"))
    );
}
#[test]
fn ghost_exceptions_duplicate_exact_keys_and_cancel_data_are_invalid() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("monthly", Some(3));
    set(
        &mut r,
        "exceptions",
        NativeValue::Array(vec![exception(date("2026-02-01"), "cancel", None)]),
    );
    assert_eq!(
        run(
            "2026-01-31",
            "2026-02-01",
            &r,
            "2026-01-01",
            "2026-07-01",
            &schema
        )
        .status,
        ExpansionStatus::Invalid
    );
    let mut r = rule("daily", Some(2));
    set(
        &mut r,
        "exceptions",
        NativeValue::Array(vec![
            exception(instant("2026-01-01T00:00:00Z"), "cancel", None),
            exception(
                instant("2026-01-01T08:00:00+08:00"),
                "replace",
                Some(range(
                    instant("2026-01-04T00:00:00Z"),
                    instant("2026-01-04T01:00:00Z"),
                )),
            ),
        ]),
    );
    assert_eq!(validate_recurrence(&r, &schema).state(), "invalid");
    let e = exception(
        date("2026-01-01"),
        "cancel",
        Some(range(date("2026-01-02"), date("2026-01-03"))),
    );
    assert_eq!(validate_exception(&e, &schema).state(), "invalid");
}
#[test]
fn selectors_interval_count_and_coarse_or_mixed_domains_reject_without_materialization() {
    let schema = Schema::bundled().unwrap();
    for (key, value) in [
        ("interval", NativeValue::Int("65536".into())),
        ("count", NativeValue::Int("2147483648".into())),
        ("byMonthDay", ints(&[0])),
        ("byMonth", ints(&[13])),
    ] {
        let mut r = rule("daily", Some(3));
        set(&mut r, key, value);
        assert_eq!(validate_recurrence(&r, &schema).state(), "invalid");
    }
    let mut r = rule("daily", Some(3));
    set(&mut r, "until", date("2026-01-05"));
    assert_eq!(validate_recurrence(&r, &schema).state(), "invalid");
    let mut r = rule("daily", Some(3));
    set(
        &mut r,
        "byWeekday",
        NativeValue::Array(vec![record(vec![
            ("day", instant("MO")),
            ("ordinal", NativeValue::Int("1".into())),
        ])]),
    );
    assert_eq!(validate_recurrence(&r, &schema).state(), "invalid");
    let coarse = decode_native_expression("(calendar: \"gregorian\", version: \"proleptic-gregorian.v1\", precision: \"month\", year: 2026, month: 1)").unwrap();
    let coarse_range = range(coarse, date("2026-03-01"));
    assert_eq!(
        validate_event(&coarse_range, None, &schema).state(),
        "value"
    );
    assert_eq!(
        validate_event(&coarse_range, Some(&rule("daily", Some(1))), &schema).state(),
        "invalid"
    );
    let result = expand_recurrence(
        &range(date("2026-01-01"), date("2026-01-02")),
        &rule("daily", Some(1)),
        &range(
            instant("2026-01-01T00:00:00Z"),
            instant("2026-01-02T00:00:00Z"),
        ),
        &schema,
        ExpansionBudget::default(),
    );
    assert_eq!(result.status, ExpansionStatus::Invalid);
}
#[test]
fn provider_versions_missing_tables_and_budget_never_become_complete_zero() {
    let schema = Schema::bundled().unwrap();
    let mut r = rule("daily", Some(3));
    set(&mut r, "calendar_version", instant("uninstalled"));
    assert_eq!(
        run(
            "2026-01-01",
            "2026-01-02",
            &r,
            "2026-01-01",
            "2026-02-01",
            &schema
        )
        .status,
        ExpansionStatus::Unavailable
    );
    let mut s = schema.clone();
    s.providers.retain(|p| p.id != "zones/utc");
    assert_eq!(
        validate_recurrence(&rule("daily", Some(3)), &s).state(),
        "unavailable"
    );
    let result = expand_recurrence(
        &range(date("2026-01-01"), date("2026-01-02")),
        &rule("daily", Some(300)),
        &range(date("2026-01-01"), date("2026-12-01")),
        &schema,
        ExpansionBudget {
            max_steps: 2,
            ..Default::default()
        },
    );
    assert_eq!(result.status, ExpansionStatus::Partial);
    assert!(result.occurrences.is_empty());
    let mut s = schema.clone();
    s.providers
        .iter_mut()
        .find(|p| p.id == "temporal/rules")
        .unwrap()
        .limits
        .max_steps = 2;
    assert_eq!(
        validate_event(
            &range(date("2026-01-01"), date("2026-01-02")),
            Some(&rule("daily", Some(3))),
            &s
        )
        .state(),
        "unavailable"
    );
}
#[test]
fn dst_gap_skips_without_consuming_count_and_fold_choice_is_explicit() {
    let s = zoned_schema(
        "2026-03-01T00:00:00Z",
        "2026-03-20T00:00:00Z",
        -18000,
        json!([{"at":"2026-03-08T07:00:00Z","offset_seconds":-14400}]),
    );
    let result = expand_recurrence(
        &range(
            instant("2026-03-07T07:30:00Z"),
            instant("2026-03-07T08:30:00Z"),
        ),
        &zone_rule("daily", 3, "earlier"),
        &range(
            instant("2026-03-01T00:00:00Z"),
            instant("2026-03-15T00:00:00Z"),
        ),
        &s,
        ExpansionBudget::default(),
    );
    complete(&result);
    assert_eq!(
        originals(&result),
        [
            "2026-03-07T07:30:00Z",
            "2026-03-09T06:30:00Z",
            "2026-03-10T06:30:00Z"
        ]
    );
    let s = zoned_schema(
        "2026-10-01T00:00:00Z",
        "2026-11-20T00:00:00Z",
        -14400,
        json!([{"at":"2026-11-01T06:00:00Z","offset_seconds":-18000}]),
    );
    for (fold, expected) in [
        ("earlier", "2026-11-01T05:30:00Z"),
        ("later", "2026-11-01T06:30:00Z"),
    ] {
        let result = expand_recurrence(
            &range(
                instant("2026-10-31T05:30:00Z"),
                instant("2026-10-31T06:30:00Z"),
            ),
            &zone_rule("daily", 3, fold),
            &range(
                instant("2026-10-30T00:00:00Z"),
                instant("2026-11-05T00:00:00Z"),
            ),
            &s,
            ExpansionBudget::default(),
        );
        complete(&result);
        assert_eq!(originals(&result)[1], expected);
    }
}
#[test]
fn exact_fraction_elapsed_duration_and_offset_spelling_deduplicate() {
    let s = Schema::bundled().unwrap();
    let mut r = rule("daily", Some(2));
    set(
        &mut r,
        "rDates",
        NativeValue::Array(vec![instant("2026-01-01T08:00:00.00000000000000001+08:00")]),
    );
    let result = expand_recurrence(
        &range(
            instant("2026-01-01T00:00:00.00000000000000001Z"),
            instant("2026-01-01T00:00:01.00000000000000002Z"),
        ),
        &r,
        &range(
            instant("2026-01-01T00:00:00Z"),
            instant("2026-01-03T00:00:00Z"),
        ),
        &s,
        ExpansionBudget::default(),
    );
    complete(&result);
    assert_eq!(result.occurrences.len(), 2);
    assert_eq!(
        result.occurrences[1].range.member("end_exclusive"),
        Some(&instant("2026-01-02T00:00:01.00000000000000002Z"))
    );
}
#[test]
fn missing_zone_coverage_is_partial_and_never_host_timezone_fallback() {
    let s = zoned_schema(
        "2026-03-01T00:00:00Z",
        "2026-03-09T00:00:00Z",
        -18000,
        json!([{"at":"2026-03-08T07:00:00Z","offset_seconds":-14400}]),
    );
    let result = expand_recurrence(
        &range(
            instant("2026-03-07T07:30:00Z"),
            instant("2026-03-07T08:30:00Z"),
        ),
        &zone_rule("daily", 3, "earlier"),
        &range(
            instant("2026-03-07T00:00:00Z"),
            instant("2026-03-08T23:00:00Z"),
        ),
        &s,
        ExpansionBudget::default(),
    );
    assert_eq!(result.status, ExpansionStatus::Complete);
    assert_eq!(result.occurrences.len(), 1);
    let result = expand_recurrence(
        &range(
            instant("2026-03-07T07:30:00Z"),
            instant("2026-03-07T08:30:00Z"),
        ),
        &zone_rule("daily", 3, "earlier"),
        &range(
            instant("2026-03-07T00:00:00Z"),
            instant("2026-03-12T00:00:00Z"),
        ),
        &s,
        ExpansionBudget::default(),
    );
    assert_eq!(result.status, ExpansionStatus::Partial);
    assert!(result.occurrences.is_empty());
}
fn table_schema() -> Schema {
    let mut s = Schema::bundled().unwrap();
    s.providers
        .iter_mut()
        .find(|p| p.id == "calendars/test")
        .unwrap()
        .data = json!({"kind":"calendar-table","key":"test-calendar","version":"test-calendar.v1","epoch_date":"2026-01-05","years":[{"year":1,"months":[{"number":1,"leap":false,"days":30},{"number":1,"leap":true,"days":29},{"number":2,"leap":false,"days":30},{"number":3,"leap":false,"days":28}]},{"year":2,"months":[{"number":1,"leap":false,"days":30},{"number":2,"leap":false,"days":30}]}]});
    s.series_policies.push(SeriesPolicy {
        id: "calendar/table".into(),
        key: "table".into(),
        calendar: "calendars/test".into(),
        zone: "zones/utc".into(),
        kinds: vec!["day", "week", "month", "quarter", "year"]
            .into_iter()
            .map(String::from)
            .collect(),
        rule: "table-ordinal-v1".into(),
        scope: json!({"kind":"library"}),
        multiplicity: "many".into(),
    });
    Schema::from_author_json(&serde_json::to_string(&s).unwrap()).unwrap()
}
#[test]
fn finite_table_period_keys_bounds_and_all_five_steps_keep_leap_month_ordinals() {
    let s = table_schema();
    for (kind, input, key, start, end) in [
        ("day", "2026-02-04", "Y1-M2-D1", "2026-02-04", "2026-02-05"),
        ("week", "2026-01-05", "W0", "2026-01-05", "2026-01-12"),
        ("month", "2026-02-04", "Y1-M2", "2026-02-04", "2026-03-05"),
        ("quarter", "2026-04-04", "Y1-Q2", "2026-04-04", "2026-05-02"),
        ("year", "2026-04-04", "Y1", "2026-01-05", "2026-05-02"),
    ] {
        let p = period_for_date("table", kind, &date(input), &s).unwrap();
        assert_eq!(p.member("key"), Some(&instant(key)));
        assert_eq!(
            period_bounds(&p, &s).unwrap(),
            range(date(start), date(end))
        );
    }
    for (kind, key) in [
        ("day", "Y1-M1-D2"),
        ("week", "W1"),
        ("month", "Y1-M2"),
        ("quarter", "Y1-Q2"),
        ("year", "Y2"),
    ] {
        let p = period_for_date("table", kind, &date("2026-01-05"), &s).unwrap();
        let q = period_step(&p, 1, &s).unwrap();
        assert_eq!(q.member("key"), Some(&instant(key)));
    }
    let p = period_for_date("table", "month", &date("2026-02-04"), &s).unwrap();
    let q = period_step(&p, -1, &s).unwrap();
    assert_eq!(q.member("key"), Some(&instant("Y1-M1")));
}
#[test]
fn table_bounds_noncanonical_keys_versions_missing_pack_and_coverage_reject_explicitly() {
    let s = table_schema();
    let p = period_for_date("table", "day", &date("2026-01-05"), &s).unwrap();
    for key in ["Y01-M1-D1", "Y1-M0-D1", "Y1-M1-D31", "Y1-M1-D01"] {
        let mut p = p.clone();
        set(&mut p, "key", instant(key));
        assert!(period_bounds(&p, &s).is_err());
    }
    let mut wrong = p.clone();
    set(&mut wrong, "calendar_version", instant("missing"));
    assert_data_error(
        &period_bounds(&wrong, &s).unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_temporal_period_provider_versions_unavailable_under_policy(),
    );
    // Civil arithmetic reaches a date outside the finite calendar table.
    for kind in ["day", "week"] {
        let first = period_for_date("table", kind, &date("2026-01-05"), &s).unwrap();
        assert_data_error(
            &period_step(&first, -1, &s).unwrap_err(),
            DataErrorKind::Coverage,
            UiMessage::error_data_temporal_calendar_table_coverage_unavailable(),
        );
    }
    // Ordinal arithmetic instead fails to select an installed table period.
    for kind in ["month", "quarter", "year"] {
        let first = period_for_date("table", kind, &date("2026-01-05"), &s).unwrap();
        assert_data_error(
            &period_step(&first, -1, &s).unwrap_err(),
            DataErrorKind::Coverage,
            UiMessage::error_data_temporal_period_step_coverage_unavailable(),
        );
    }
    assert_data_error(
        &period_for_date("table", "day", &date("2026-01-04"), &s).unwrap_err(),
        DataErrorKind::Coverage,
        UiMessage::error_data_temporal_calendar_table_coverage_unavailable(),
    );
    let mut missing = s.clone();
    missing.providers.retain(|p| p.id != "calendars/test");
    assert_data_error(
        &period_bounds(&p, &missing).unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_temporal_series_calendar_provider_unavailable(),
    );
    let mut s = table_schema();
    s.providers
        .iter_mut()
        .find(|p| p.id == "calendars/test")
        .unwrap()
        .data["epoch_date"] = json!("2026-01-06");
    let p = period_for_date("table", "week", &date("2026-01-06"), &s).unwrap_err();
    assert_data_error(
        &p,
        DataErrorKind::Coverage,
        UiMessage::error_data_temporal_period_outside_calendar_table_coverage(),
    );
}
#[test]
fn period_zoned_bounds_use_exact_series_zone_and_explicit_fold() {
    let s = zoned_schema(
        "2026-01-01T00:00:00Z",
        "2027-01-01T00:00:00Z",
        -18000,
        json!([{"at":"2026-03-08T07:00:00Z","offset_seconds":-14400},{"at":"2026-11-01T06:00:00Z","offset_seconds":-18000}]),
    );
    let p = period_for_date("main", "day", &date("2026-07-01"), &s).unwrap();
    assert_eq!(
        period_zoned_bounds(&p, "earlier", &s).native(),
        Some(&range(
            instant("2026-07-01T04:00:00Z"),
            instant("2026-07-02T04:00:00Z")
        ))
    );
    assert_eq!(period_zoned_bounds(&p, "guess", &s).state(), "invalid");
}
#[test]
fn table_recurrence_runs_on_month_ordinals_not_gregorian_month_numbers() {
    let s = table_schema();
    let mut r = rule("monthly", Some(3));
    set(&mut r, "calendar", instant("test-calendar"));
    set(&mut r, "calendar_version", instant("test-calendar.v1"));
    let result = run(
        "2026-01-05",
        "2026-01-06",
        &r,
        "2026-01-05",
        "2026-04-01",
        &s,
    );
    complete(&result);
    assert_eq!(
        originals(&result),
        ["2026-01-05", "2026-02-04", "2026-03-05"]
    );
}

#[test]
fn utc_sort_and_future_lower_bound_precede_count_and_until() {
    // Deliberate synthetic offset jumps make civil Jan2 map after civil Jan3.
    // A first over-until candidate is therefore insufficient termination proof.
    let s = zoned_schema(
        "2025-12-29T00:00:00Z",
        "2026-01-08T00:00:00Z",
        86400,
        json!([{"at":"2026-01-01T00:00:00Z","offset_seconds":-86400},
        {"at":"2026-01-02T00:00:00Z","offset_seconds":86400},
        {"at":"2026-01-02T12:00:00Z","offset_seconds":-86400}]),
    );
    let template = range(
        instant("2025-12-31T00:00:00Z"),
        instant("2025-12-31T01:00:00Z"),
    );
    let window = range(
        instant("2025-12-30T00:00:00Z"),
        instant("2026-01-07T00:00:00Z"),
    );
    let result = expand_recurrence(
        &template,
        &zone_rule("daily", 3, "earlier"),
        &window,
        &s,
        ExpansionBudget::default(),
    );
    complete(&result);
    assert_eq!(
        originals(&result),
        [
            "2025-12-31T00:00:00Z",
            "2026-01-02T00:00:00Z",
            "2026-01-03T00:00:00Z"
        ]
    );
    let mut r = zone_rule("daily", 3, "earlier");
    let NativeValue::Record(members) = &mut r else {
        panic!()
    };
    members.retain(|m| m.key != "count");
    set(&mut r, "until", instant("2026-01-02T12:00:00Z"));
    let result = expand_recurrence(&template, &r, &window, &s, ExpansionBudget::default());
    complete(&result);
    assert_eq!(
        originals(&result),
        ["2025-12-31T00:00:00Z", "2026-01-02T00:00:00Z"]
    );
}
#[test]
fn closed_event_abi_and_zone_primitives_preserve_inputs_and_gap_ambiguity() {
    let schema = Schema::bundled().unwrap();
    let budget_native = record(vec![
        ("max_steps", NativeValue::Int("1000".into())),
        ("max_occurrences", NativeValue::Int("100".into())),
        ("max_bytes", NativeValue::Int("100000".into())),
    ]);
    let budget = ExpansionBudget::from_native(&budget_native).unwrap();
    let event = record(vec![
        ("range", range(date("2026-01-01"), date("2026-01-02"))),
        ("recurrence", rule("daily", Some(2))),
        ("title", instant("Event title")),
        ("event_status", instant("confirmed")),
    ]);
    let result = expand_event(
        &event,
        &range(date("2026-01-01"), date("2026-01-10")),
        &schema,
        budget,
    );
    complete(&result);
    assert_eq!(result.occurrences[0].title, Some(instant("Event title")));
    assert_eq!(result.occurrences[0].status, Some(instant("confirmed")));
    assert_eq!(result.native().member("status"), Some(&instant("complete")));
    let mut bad = budget_native;
    set(&mut bad, "execute", instant("script"));
    assert!(ExpansionBudget::from_native(&bad).is_err());
    let s = zoned_schema(
        "2026-03-01T00:00:00Z",
        "2026-03-20T00:00:00Z",
        -18000,
        json!([{"at":"2026-03-08T07:00:00Z","offset_seconds":-14400}]),
    );
    let gap = NativeValue::LocalDatetime("2026-03-08T02:30:00".into());
    assert_eq!(
        zone_to_instant(
            &gap,
            "Test/Eastern",
            "synthetic-dst.v1",
            "earlier",
            "skip",
            &s
        )
        .state(),
        "absent"
    );
    assert_eq!(
        zone_to_instant(
            &gap,
            "Test/Eastern",
            "synthetic-dst.v1",
            "guess",
            "skip",
            &s
        )
        .state(),
        "invalid"
    );
    let value = zone_to_civil(
        &instant("2026-03-09T06:30:00Z"),
        "Test/Eastern",
        "synthetic-dst.v1",
        &s,
    );
    assert_eq!(
        value.native(),
        Some(&NativeValue::LocalDatetime("2026-03-09T02:30:00".into()))
    );
    assert_eq!(
        zone_to_civil(
            &instant("2026-03-09T06:30:00.00000000000001Z"),
            "Test/Eastern",
            "synthetic-dst.v1",
            &s
        )
        .state(),
        "unavailable"
    );
    assert!(ExactInstant::parse("2026-01-01T00:00:00+-1:00").is_err());
    let s = table_schema();
    let mut p = period_for_date("table", "month", &date("2026-01-05"), &s).unwrap();
    set(&mut p, "key", instant("Y1-M-9223372036854775808"));
    assert!(period_bounds(&p, &s).is_err());
}

#[test]
fn actual_native_event_field_validation_proves_anchor_membership_and_preserves_raw_invalid_source()
{
    let schema = Schema::bundled().unwrap();
    let range_value = range(date("2026-01-31"), date("2026-02-01"));
    let source = |recurrence: &NativeValue| {
        format!(
            "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"event\",), range: {}, recurrence: {})) <weftext>\n原始正文",
            range_value.to_typst().unwrap(),
            recurrence.to_typst().unwrap()
        )
    };
    let mut recurrence = rule("monthly", Some(3));
    let original = source(&recurrence);
    let projected = project(&original, &schema);
    assert_eq!(projected.qualification, "qualified");
    assert_eq!(
        projected
            .fields
            .iter()
            .find(|f| f.field_id == "calendar/recurrence")
            .unwrap()
            .cell
            .state(),
        "value"
    );
    assert!(validate_note(&original, &schema, &ValidationContext::default()).is_valid());
    set(
        &mut recurrence,
        "exceptions",
        NativeValue::Array(vec![exception(date("2026-02-01"), "cancel", None)]),
    );
    let invalid_source = source(&recurrence);
    let invalid = validate_note(&invalid_source, &schema, &ValidationContext::default());
    assert!(!invalid.is_valid());
    assert!(invalid.diagnostics.iter().any(|d| d.explanation
        == UiMessage::error_data_recurrence_exception_onset_outside_base_and_rdate()));
    let projected = project(&invalid_source, &schema);
    let raw = projected
        .fields
        .iter()
        .find(|f| f.field_id == "calendar/recurrence")
        .unwrap()
        .raw
        .as_ref()
        .unwrap();
    assert!(raw.contains("exceptions"));
    assert!(invalid_source.ends_with("原始正文"));
    set(&mut recurrence, "calendar_version", instant("uninstalled"));
    let missing = validate_note(&source(&recurrence), &schema, &ValidationContext::default());
    assert!(!missing.unavailable.is_empty());
}

#[test]
fn period_admission_classifies_missing_versions_and_coverage_without_invalidating_source() {
    let schema = table_schema();
    let p = period_for_date("table", "day", &date("2026-01-05"), &schema).unwrap();
    assert_eq!(validate_period(&p, &schema).state(), "value");
    for key in ["calendar_version", "zone_version", "series_key"] {
        let mut missing = p.clone();
        set(&mut missing, key, instant("uninstalled"));
        assert_eq!(validate_period(&missing, &schema).state(), "unavailable");
    }
    let mut wrong = p.clone();
    set(&mut wrong, "key", instant("Y1-M0-D1"));
    assert_eq!(validate_period(&wrong, &schema).state(), "invalid");
    let mut wrong = p.clone();
    set(&mut wrong, "rule", instant("unregistered"));
    assert_eq!(validate_period(&wrong, &schema).state(), "invalid");
    let mut missing = schema.clone();
    missing.providers.retain(|p| p.id != "calendars/test");
    assert_eq!(validate_period(&p, &missing).state(), "unavailable");
}
