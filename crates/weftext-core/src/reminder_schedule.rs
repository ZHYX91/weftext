//! Pure bounded admission policy shared by the Core reminder guard and summary
//! preparation. This module owns no files, task data, clock or external effect.
use crate::{CoreError, Result};
use std::collections::BTreeSet;
use weftext_data::ExactInstant;
use weftext_messages::UiMessage;

pub(crate) const MAX_INPUT_PATHS: usize = 1024;
pub(crate) const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const SUMMARY_COOLDOWN_SECONDS: i64 = 900;

fn temporal(error: weftext_data::DataError) -> CoreError {
    CoreError::new(
        "reminderState",
        UiMessage::error_reminder_temporal_value_invalid(),
    )
    .caused_by(CoreError::from_data("data", error))
}

pub(crate) fn summary_deadline(cutoff: &ExactInstant) -> Result<ExactInstant> {
    let utc_seconds = cutoff
        .utc_seconds
        .checked_add(SUMMARY_COOLDOWN_SECONDS)
        .ok_or_else(|| {
            CoreError::new(
                "reminderState",
                UiMessage::error_reminder_summary_cooldown_invalid(),
            )
        })?;
    let deadline = ExactInstant {
        utc_seconds,
        fraction: cutoff.fraction.clone(),
    };
    deadline.canonical().map_err(temporal)?;
    Ok(deadline)
}

pub(crate) fn validate_window(
    cutoff: &str,
    next_allowed_at: &str,
) -> Result<(ExactInstant, ExactInstant)> {
    let cutoff = ExactInstant::parse(cutoff).map_err(temporal)?;
    let deadline = ExactInstant::parse(next_allowed_at).map_err(temporal)?;
    if !summary_deadline(&cutoff)?.compare(&deadline).is_eq() {
        return Err(CoreError::new(
            "reminderState",
            UiMessage::error_reminder_summary_cooldown_invalid(),
        ));
    }
    Ok((cutoff, deadline))
}

/// No caller-provided now enters this function from a public producer. The
/// mutable owner supplies its trusted clock; persisted future cutoffs remain
/// restrictive after a clock correction rather than unlocking another batch.
pub(crate) fn cooling_down<'a>(
    at: &ExactInstant,
    windows: impl Iterator<Item = (&'a str, &'a str)>,
) -> Result<Option<String>> {
    let mut latest: Option<ExactInstant> = None;
    for (cutoff, deadline) in windows {
        let (cutoff, deadline) = validate_window(cutoff, deadline)?;
        if at.compare(&cutoff).is_lt() {
            return Err(CoreError::new(
                "reminderClockRollback",
                UiMessage::error_reminder_summary_clock_rollback(),
            ));
        }
        if latest
            .as_ref()
            .is_none_or(|old| old.compare(&deadline).is_lt())
        {
            latest = Some(deadline);
        }
    }
    match latest {
        Some(deadline) if at.compare(&deadline).is_lt() => {
            Ok(Some(deadline.canonical().map_err(temporal)?))
        }
        _ => Ok(None),
    }
}

#[derive(Default)]
pub(crate) struct InputBudget {
    paths: BTreeSet<String>,
    bytes: usize,
}
impl InputBudget {
    pub(crate) fn include(&mut self, path: &str, bytes: usize) -> Result<()> {
        if self.paths.contains(path) {
            return Ok(());
        }
        if self.paths.len() == MAX_INPUT_PATHS {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_reminder_input_guard_count_budget(),
            ));
        }
        let total = self.bytes.checked_add(bytes).ok_or_else(|| {
            CoreError::new("budget", UiMessage::error_reminder_input_byte_overflow())
        })?;
        if total > MAX_INPUT_BYTES {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_reminder_input_guard_byte_budget(),
            ));
        }
        self.paths.insert(path.into());
        self.bytes = total;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn instant(text: &str) -> ExactInstant {
        ExactInstant::parse(text).unwrap()
    }
    #[test]
    fn exact_cooldown_boundary_and_clock_rollback_survive_serialization() {
        let at = instant("2026-10-23T10:00:00.123456789123Z");
        let deadline = summary_deadline(&at).unwrap().canonical().unwrap();
        assert_eq!(deadline, "2026-10-23T10:15:00.123456789123Z");
        let cutoff = at.canonical().unwrap();
        assert_eq!(
            cooling_down(&at, [(cutoff.as_str(), deadline.as_str())].into_iter()).unwrap(),
            Some(deadline.clone())
        );
        assert!(
            cooling_down(
                &instant(&deadline),
                [(cutoff.as_str(), deadline.as_str())].into_iter()
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            cooling_down(
                &instant("2026-10-23T10:00:00.123456789122Z"),
                [(cutoff.as_str(), deadline.as_str())].into_iter()
            )
            .unwrap_err()
            .code,
            "reminderClockRollback"
        );
        assert!(validate_window(&cutoff, "2026-10-23T10:15:00.123456789124Z").is_err());
    }
    #[test]
    fn whole_input_budget_is_exact_and_duplicate_paths_do_not_count_twice() {
        let mut budget = InputBudget::default();
        budget.include("source", MAX_INPUT_BYTES).unwrap();
        budget.include("source", MAX_INPUT_BYTES).unwrap();
        assert!(budget.include("another", 1).is_err());
        let mut budget = InputBudget::default();
        for index in 0..MAX_INPUT_PATHS {
            budget.include(&index.to_string(), 0).unwrap();
        }
        assert!(budget.include("overflow", 0).is_err());
        assert_eq!(budget.paths.len(), MAX_INPUT_PATHS);
    }
}
