//! The synchronous portion of a cancellable Query read. The caller installs
//! its own flag only while running that read on the current thread.
use crate::{DataError, UiMessage};
use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

thread_local! {
    static STOP: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

pub fn with_query_stop<T>(flag: Arc<AtomicBool>, read: impl FnOnce() -> T) -> T {
    in_query_stop(Some(flag), read)
}

pub fn without_query_stop<T>(operation: impl FnOnce() -> T) -> T {
    in_query_stop(None, operation)
}

fn in_query_stop<T>(flag: Option<Arc<AtomicBool>>, read: impl FnOnce() -> T) -> T {
    struct Restore(Option<Arc<AtomicBool>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            STOP.replace(self.0.take());
        }
    }
    let _restore = Restore(STOP.replace(flag));
    read()
}

pub fn query_checkpoint() -> crate::Result<()> {
    if STOP.with_borrow(|flag| flag.as_ref().is_some_and(|f| f.load(Ordering::Acquire))) {
        Err(DataError::unavailable(
            UiMessage::error_api_data_query_cancelled(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_draft_leaves_the_enclosing_query_cancelled() {
        with_query_stop(Arc::new(AtomicBool::new(true)), || {
            assert!(query_checkpoint().is_err());
            without_query_stop(|| assert!(query_checkpoint().is_ok()));
            assert!(query_checkpoint().is_err());
        });
        assert!(query_checkpoint().is_ok());
    }
}
