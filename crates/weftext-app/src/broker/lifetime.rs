//! One owner lifetime, not a second Session or operation ledger. Authenticated
//! complete CLI ingress and the sole actual UI coordinator take over a temporary
//! bootstrap independently. Bootstrap failure cannot revoke either owner.
use std::sync::atomic::{AtomicU64, Ordering};

const CLI: u64 = 1;
const CLOSING: u64 = 2;
const UI: u64 = 4;
const FLAGS: u64 = CLI | CLOSING | UI;
const EPOCH_SHIFT: u32 = 3;
const MAX_EPOCH: u64 = u64::MAX >> EPOCH_SHIFT;

pub(crate) struct ServiceContinuation(AtomicU64);

/// The actual STA owns this monotonic visibility fence. A supervisor's sampled
/// exposed count cannot seal it: only its serial window inventory or the final
/// author-proof cut may do so, before publishing ExitPermitted.
#[derive(Default)]
pub(crate) struct VisibilityFence {
    sealed: bool,
}
impl VisibilityFence {
    pub fn accepts_exposure(&self) -> bool {
        !self.sealed
    }
    pub fn confirm_exit(&mut self, any_exposed: bool, all_proved: bool) -> bool {
        if self.sealed || !any_exposed || all_proved {
            self.sealed = true;
            true
        } else {
            false
        }
    }
}
impl ServiceContinuation {
    pub fn new(headless_first: bool) -> Self {
        Self(AtomicU64::new(if headless_first { CLI } else { 0 }))
    }
    /// Call only after peer authentication, full framing and fixed operation /
    /// secret validation. It linearizes against a last-window idle exit.
    pub fn admit_cli(&self) -> bool {
        self.admit(CLI)
    }
    /// The sole owner calls this before it starts/activates an actual desktop.
    /// Bootstrap expiry cannot retire an already admitted author surface.
    pub fn admit_ui(&self) -> Option<u64> {
        let mut previous = self.0.load(Ordering::Acquire);
        loop {
            if previous & CLOSING != 0 {
                return None;
            }
            let epoch = (previous >> EPOCH_SHIFT).checked_add(1)?;
            if epoch > MAX_EPOCH {
                return None;
            }
            let next = (epoch << EPOCH_SHIFT) | (previous & FLAGS) | UI;
            match self
                .0
                .compare_exchange_weak(previous, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Some(epoch),
                Err(current) => previous = current,
            }
        }
    }
    fn admit(&self, kind: u64) -> bool {
        let mut previous = self.0.load(Ordering::Acquire);
        loop {
            if previous & CLOSING != 0 {
                return false;
            }
            match self.0.compare_exchange_weak(
                previous,
                previous | kind,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(current) => previous = current,
            }
        }
    }
    pub fn cli_active(&self) -> bool {
        self.0.load(Ordering::Acquire) & (CLI | CLOSING) == CLI
    }
    pub fn has_takeover(&self) -> bool {
        self.0.load(Ordering::Acquire) & (CLI | UI) != 0
    }
    pub fn is_closing(&self) -> bool {
        self.0.load(Ordering::Acquire) & CLOSING != 0
    }
    /// Only the serial owner activation path calls this after Desktop::start
    /// failed before creating a UI thread. No actual or concurrent UI owner is
    /// retired; a CLI call admitted during that await keeps its separate bit.
    pub fn failed_ui_before_creation(&self, epoch: u64) {
        let mut previous = self.0.load(Ordering::Acquire);
        loop {
            if previous >> EPOCH_SHIFT != epoch || previous & CLOSING != 0 {
                return;
            }
            match self.0.compare_exchange_weak(
                previous,
                previous & !UI,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(current) => previous = current,
            }
        }
    }
    /// A temporary cold-toast bootstrap has no authority to revoke later CLI
    /// or UI admission. Only no active flags may retire; the epoch never resets.
    pub fn end_unclaimed_bootstrap(&self) -> bool {
        let mut previous = self.0.load(Ordering::Acquire);
        loop {
            if previous & FLAGS != 0 {
                return false;
            }
            match self.0.compare_exchange_weak(
                previous,
                previous | CLOSING,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(current) => previous = current,
            }
        }
    }
    /// Only an actual STA zero-window cut supplies empty_epoch. A later UI
    /// admission invalidates that cut even before its queued Open is consumed.
    /// The same CAS also excludes a concurrent accepted CLI continuation.
    pub fn exit_after_last_window(&self, resident: bool, empty_epoch: u64) -> bool {
        if resident || empty_epoch == 0 {
            return false;
        }
        let mut previous = self.0.load(Ordering::Acquire);
        loop {
            if previous & (CLI | CLOSING) != 0 || previous >> EPOCH_SHIFT != empty_epoch {
                return false;
            }
            match self.0.compare_exchange_weak(
                previous,
                (previous | CLOSING) & !UI,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(current) => previous = current,
            }
        }
    }
    /// Only an approved global exit, typed owner shutdown, or an actual host
    /// failure retires service admission. A blocked close must never call this.
    /// Accepted handlers and worker tails still require their real joins.
    pub fn close(&self) {
        let mut previous = self.0.load(Ordering::Acquire);
        loop {
            match self.0.compare_exchange_weak(
                previous,
                (previous | CLOSING) & !(CLI | UI),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(current) => previous = current,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[test]
    fn cli_first_and_gui_first_keep_the_same_continuation_after_current_last_close() {
        for headless in [false, true] {
            let owner = ServiceContinuation::new(headless);
            let cut = owner.admit_ui().unwrap();
            assert!(owner.admit_cli());
            assert!(!owner.exit_after_last_window(false, cut));
            assert!(owner.admit_cli());
            assert!(owner.cli_active());
            owner.close();
            assert!(!owner.admit_cli());
            assert!(owner.admit_ui().is_none());
            assert!(!owner.cli_active());
        }
    }
    #[test]
    fn failed_or_incomplete_ingress_does_not_take_over_bootstrap() {
        let owner = ServiceContinuation::new(false);
        assert!(!owner.has_takeover());
        assert!(owner.end_unclaimed_bootstrap());
        assert!(!owner.admit_cli());
        assert!(owner.admit_ui().is_none());
    }
    #[test]
    fn blocked_global_close_keeps_admission_and_resident_is_independent_of_cli() {
        let owner = ServiceContinuation::new(false);
        let cut = owner.admit_ui().unwrap();
        assert!(!owner.exit_after_last_window(true, cut));
        assert!(!owner.cli_active());
        assert!(!owner.end_unclaimed_bootstrap());
        assert!(owner.admit_cli());
        assert!(!owner.exit_after_last_window(false, cut));
        assert!(owner.cli_active());
    }
    #[test]
    fn last_close_and_complete_cli_admission_have_one_atomic_winner() {
        for _ in 0..64 {
            let owner = Arc::new(ServiceContinuation::new(false));
            let cut = owner.admit_ui().unwrap();
            let start = Arc::new(Barrier::new(2));
            let candidate = owner.clone();
            let together = start.clone();
            let caller = std::thread::spawn(move || {
                together.wait();
                candidate.admit_cli()
            });
            start.wait();
            let exiting = owner.exit_after_last_window(false, cut);
            let admitted = caller.join().unwrap();
            assert_ne!(exiting, admitted);
            assert_eq!(owner.cli_active(), admitted);
        }
    }
    #[test]
    fn cold_bootstrap_expiry_cannot_retire_an_admitted_author_surface() {
        let owner = ServiceContinuation::new(false);
        let cut = owner.admit_ui().unwrap();
        assert!(owner.has_takeover());
        assert!(!owner.end_unclaimed_bootstrap());
        assert!(!owner.is_closing());
        // Only the later actual STA empty cut may use the last-window path.
        assert!(owner.exit_after_last_window(false, cut));
    }
    #[test]
    fn original_toast_timeout_rejection_or_late_reply_does_not_revoke_other_owners() {
        for ui_first in [false, true] {
            let owner = ServiceContinuation::new(false);
            let cut = if ui_first {
                owner.admit_ui().unwrap()
            } else {
                0
            };
            assert!(owner.admit_cli());
            let cut = if ui_first {
                cut
            } else {
                owner.admit_ui().unwrap()
            };
            for _original_result in ["timeout", "rejected", "late"] {
                assert!(!owner.end_unclaimed_bootstrap());
                assert!(!owner.is_closing());
                assert!(owner.admit_cli());
                assert!(!owner.exit_after_last_window(false, cut));
            }
        }
    }
    #[test]
    fn unclaimed_bootstrap_can_retire_and_refuse_late_activation() {
        let owner = ServiceContinuation::new(false);
        assert!(owner.end_unclaimed_bootstrap());
        assert!(owner.is_closing());
        assert!(owner.admit_ui().is_none());
        assert!(!owner.admit_cli());
    }
    #[test]
    fn failed_ui_before_creation_preserves_concurrent_cli_and_monotonic_epoch() {
        for cli in [false, true] {
            let owner = ServiceContinuation::new(false);
            let failed = owner.admit_ui().unwrap();
            if cli {
                assert!(owner.admit_cli());
            }
            owner.failed_ui_before_creation(failed);
            assert_eq!(owner.cli_active(), cli);
            assert_eq!(owner.has_takeover(), cli);
            assert_eq!(owner.end_unclaimed_bootstrap(), !cli);
        }
    }
    #[test]
    fn actual_ui_admission_and_bootstrap_expiry_have_one_atomic_winner() {
        for _ in 0..64 {
            let owner = Arc::new(ServiceContinuation::new(false));
            let start = Arc::new(Barrier::new(2));
            let candidate = owner.clone();
            let together = start.clone();
            let caller = std::thread::spawn(move || {
                together.wait();
                candidate.admit_ui()
            });
            start.wait();
            let expired = owner.end_unclaimed_bootstrap();
            let admitted = caller.join().unwrap();
            assert_ne!(expired, admitted.is_some());
            assert_eq!(owner.is_closing(), expired);
            assert_eq!(owner.has_takeover(), admitted.is_some());
        }
    }
    #[test]
    fn old_empty_cannot_retire_new_admission_even_before_its_open_is_consumed() {
        let owner = ServiceContinuation::new(false);
        let old_empty = owner.admit_ui().unwrap();
        let queued_open = owner.admit_ui().unwrap();
        assert!(queued_open > old_empty);
        assert!(!owner.exit_after_last_window(false, old_empty));
        assert!(!owner.is_closing());
        assert!(owner.exit_after_last_window(false, queued_open));
    }
    #[test]
    fn a_b_a_and_duplicate_empty_never_reuse_a_prior_cut() {
        let owner = ServiceContinuation::new(false);
        let first_a = owner.admit_ui().unwrap();
        let b = owner.admit_ui().unwrap();
        let next_a = owner.admit_ui().unwrap();
        assert!(first_a < b && b < next_a);
        for stale in [first_a, b, first_a] {
            assert!(!owner.exit_after_last_window(false, stale));
        }
        assert!(owner.exit_after_last_window(false, next_a));
        assert!(!owner.exit_after_last_window(false, next_a));
    }
    #[test]
    fn old_failed_start_cleanup_cannot_clear_new_ui_admission() {
        let owner = ServiceContinuation::new(false);
        let old = owner.admit_ui().unwrap();
        let new = owner.admit_ui().unwrap();
        owner.failed_ui_before_creation(old);
        assert!(owner.has_takeover());
        assert!(!owner.end_unclaimed_bootstrap());
        assert!(!owner.exit_after_last_window(false, old));
        assert!(owner.exit_after_last_window(false, new));
    }
    #[test]
    fn explicit_global_exit_retires_both_admissions_without_reusing_idle_proof() {
        let owner = ServiceContinuation::new(false);
        let cut = owner.admit_ui().unwrap();
        assert!(owner.admit_cli());
        owner.close();
        assert!(owner.is_closing());
        assert!(!owner.cli_active());
        assert!(owner.admit_ui().is_none());
        assert!(!owner.admit_cli());
        assert!(!owner.exit_after_last_window(false, cut));
    }
    #[test]
    fn ui_epoch_never_wraps_or_accepts_an_unobserved_zero_empty() {
        let owner = ServiceContinuation(AtomicU64::new((MAX_EPOCH << EPOCH_SHIFT) | UI));
        assert!(owner.admit_ui().is_none());
        assert!(!owner.exit_after_last_window(false, 0));
        assert!(!owner.end_unclaimed_bootstrap());
        assert!(owner.exit_after_last_window(false, MAX_EPOCH));
    }
    #[test]
    fn old_empty_and_concurrent_reactivation_have_one_atomic_winner() {
        for _ in 0..64 {
            let owner = Arc::new(ServiceContinuation::new(false));
            let old = owner.admit_ui().unwrap();
            let start = Arc::new(Barrier::new(2));
            let candidate = owner.clone();
            let together = start.clone();
            let caller = std::thread::spawn(move || {
                together.wait();
                candidate.admit_ui()
            });
            start.wait();
            let exited = owner.exit_after_last_window(false, old);
            let new = caller.join().unwrap();
            assert_ne!(exited, new.is_some());
            if let Some(new) = new {
                assert!(new > old);
            }
        }
    }
    #[test]
    fn supervisor_zero_sample_cannot_retire_a_later_actual_ready_surface() {
        let mut sta = VisibilityFence::default();
        let sampled_exposed = false;
        assert!(!sampled_exposed);
        // The actual STA consumes Ready between the owner's sample and its
        // BeginExit command. The real inventory, not that sample, requires proof.
        assert!(sta.accepts_exposure());
        let actual_exposed = true;
        assert!(!sta.confirm_exit(actual_exposed, false));
        assert!(sta.accepts_exposure());
        assert!(sta.confirm_exit(actual_exposed, true));
        assert!(!sta.accepts_exposure());
    }
    #[test]
    fn sta_no_exposure_confirmation_prevents_all_late_ready_or_open() {
        let mut sta = VisibilityFence::default();
        assert!(sta.confirm_exit(false, false));
        // The kernel/ordinary queues can still contain Ready/controller/Open;
        // after this serial cut none may expose an author surface.
        for _late in ["ready", "controller", "repeatOpen", "postActivate"] {
            assert!(!sta.accepts_exposure());
        }
        assert!(sta.confirm_exit(false, false));
    }
    #[test]
    fn failed_begin_exit_delivery_never_becomes_a_visibility_confirmation() {
        let mut sta = VisibilityFence::default();
        let (send, queue) = std::sync::mpsc::sync_channel(1);
        send.try_send("oldWork").unwrap();
        assert!(send.try_send("beginExit").is_err());
        assert!(sta.accepts_exposure());
        assert_eq!(queue.recv().unwrap(), "oldWork");
        // A later exposed author still needs its own actual proof, even if a
        // supervisor records an unrelated host failure in the meantime.
        assert!(!sta.confirm_exit(true, false));
    }
}
