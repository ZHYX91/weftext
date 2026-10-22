//! The supervisor's final Core result must not compete with ordinary UI work.
//! Publication, acknowledgement and actual thread retirement share one atomic
//! word. No channel capacity, HWND post or sender Drop is evidence of release.
use std::{
    io,
    sync::atomic::{AtomicU8, Ordering},
};

const QUIET: u8 = 1;
const FAILED: u8 = 2;
const RESULT: u8 = QUIET | FAILED;
const ACK_SHIFT: u32 = 2;
const STOPPED: u8 = 1 << 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Quiescent,
    ReleaseFailed,
    Contradictory,
}
impl Outcome {
    fn bits(self) -> u8 {
        match self {
            Self::Quiescent => QUIET,
            Self::ReleaseFailed => FAILED,
            Self::Contradictory => RESULT,
        }
    }
    fn from_bits(bits: u8) -> Option<Self> {
        match bits & RESULT {
            QUIET => Some(Self::Quiescent),
            FAILED => Some(Self::ReleaseFailed),
            RESULT => Some(Self::Contradictory),
            _ => None,
        }
    }
}

#[derive(Default)]
pub(super) struct Terminal(AtomicU8);
impl Terminal {
    /// A duplicate identical result is idempotent. Conflicting publications are
    /// retained as a permanent failure, including a conflict after an ACK.
    pub fn publish(&self, outcome: Outcome) -> io::Result<()> {
        let mut previous = self.0.load(Ordering::Acquire);
        loop {
            if previous & STOPPED != 0 {
                return Err(io::Error::other("The native STA has already returned"));
            }
            let next = previous | outcome.bits();
            match self
                .0
                .compare_exchange_weak(previous, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) if next & RESULT == RESULT => {
                    return Err(io::Error::other(
                        "The final Core release results contradict each other",
                    ));
                }
                Ok(_) => return Ok(()),
                Err(current) => previous = current,
            }
        }
    }
    pub fn outcome(&self) -> Option<Outcome> {
        Outcome::from_bits(self.0.load(Ordering::Acquire))
    }
    pub fn accepts_commands(&self) -> bool {
        self.0.load(Ordering::Acquire) & (RESULT | STOPPED) == 0
    }
    /// Only the STA reducer calls this after applying this exact result. A
    /// concurrent contradictory publication cannot inherit an earlier ACK.
    pub fn acknowledge(&self, observed: Outcome) -> bool {
        let mut previous = self.0.load(Ordering::Acquire);
        loop {
            if previous & RESULT != observed.bits() || previous & STOPPED != 0 {
                return false;
            }
            let next = (previous & !(RESULT << ACK_SHIFT)) | (observed.bits() << ACK_SHIFT);
            match self
                .0
                .compare_exchange_weak(previous, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return true,
                Err(current) => previous = current,
            }
        }
    }
    /// Executed by the exact STA thread's return guard, also on unwinding.
    pub fn stopped(&self) {
        self.0.fetch_or(STOPPED, Ordering::AcqRel);
    }
    /// The caller must first join the actual thread handle. A published result
    /// alone, even an acknowledged one, is not thread or callback retirement.
    pub fn confirm_joined(&self) -> io::Result<()> {
        let state = self.0.load(Ordering::Acquire);
        if state & STOPPED == 0 || state & RESULT != QUIET || (state >> ACK_SHIFT) & RESULT != QUIET
        {
            return Err(io::Error::other(
                "The native STA did not acknowledge a consistent successful Core release",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, mpsc};

    fn full_queue(outcome: Outcome, failed_wake: bool) {
        let terminal = Arc::new(Terminal::default());
        let (ordinary, queued) = mpsc::sync_channel(64);
        for n in 0..64 {
            ordinary.try_send(n).unwrap();
        }
        assert!(ordinary.try_send(64).is_err());
        let (resume, paused) = mpsc::channel();
        let (applied, received) = mpsc::channel();
        let reducer = terminal.clone();
        let actual = std::thread::spawn(move || {
            paused.recv().unwrap();
            let result = reducer.outcome().unwrap();
            assert!(!reducer.accepts_commands());
            // The same retained slot is readable if OS signalling fails; the
            // real pump also has a bounded poll, independent of event delivery.
            if failed_wake {
                assert_eq!(reducer.outcome(), Some(result));
            }
            assert!(reducer.acknowledge(result));
            applied.send(result).unwrap();
            drop(queued);
            reducer.stopped();
        });
        terminal.publish(outcome).unwrap();
        assert!(!terminal.accepts_commands());
        assert!(terminal.confirm_joined().is_err());
        resume.send(()).unwrap();
        assert_eq!(received.recv().unwrap(), outcome);
        actual.join().unwrap();
        assert_eq!(
            terminal.confirm_joined().is_ok(),
            outcome == Outcome::Quiescent
        );
    }
    #[test]
    fn full_normal_queue_does_not_lose_success() {
        full_queue(Outcome::Quiescent, false);
    }
    #[test]
    fn full_normal_queue_does_not_lose_failure() {
        full_queue(Outcome::ReleaseFailed, false);
    }
    #[test]
    fn failed_wake_does_not_erase_terminal() {
        full_queue(Outcome::Quiescent, true);
        full_queue(Outcome::ReleaseFailed, true);
    }
    #[test]
    fn repeated_identical_is_idempotent_but_conflict_invalidates_ack() {
        for (first, second) in [
            (Outcome::Quiescent, Outcome::ReleaseFailed),
            (Outcome::ReleaseFailed, Outcome::Quiescent),
        ] {
            let terminal = Terminal::default();
            terminal.publish(first).unwrap();
            terminal.publish(first).unwrap();
            assert!(terminal.acknowledge(first));
            assert!(terminal.publish(second).is_err());
            assert_eq!(terminal.outcome(), Some(Outcome::Contradictory));
            assert!(!terminal.acknowledge(first));
            assert!(terminal.acknowledge(Outcome::Contradictory));
            terminal.stopped();
            assert!(terminal.confirm_joined().is_err());
        }
    }
    #[test]
    fn retired_ui_rejects_late_publication_and_ordinary_work() {
        let terminal = Terminal::default();
        terminal.stopped();
        assert!(terminal.publish(Outcome::Quiescent).is_err());
        assert!(!terminal.accepts_commands());
        assert_eq!(terminal.outcome(), None);
        assert!(terminal.confirm_joined().is_err());
    }
    #[test]
    fn callback_completion_is_not_core_ack_or_thread_retirement() {
        let terminal = Terminal::default();
        terminal.publish(Outcome::Quiescent).unwrap();
        assert!(terminal.confirm_joined().is_err());
        assert!(terminal.acknowledge(Outcome::Quiescent));
        assert!(terminal.confirm_joined().is_err());
        terminal.stopped();
        terminal.confirm_joined().unwrap();
        assert!(terminal.publish(Outcome::Quiescent).is_err());
    }
}
