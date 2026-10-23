//! Process-owned work tracking. Dropping a request/JoinHandle does not make an
//! accepted Core write, conversion or child process disappear.
use std::{
    future::Future,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tokio::{sync::Notify, task::JoinHandle};

#[derive(Clone, Default)]
pub(crate) struct Registry(Arc<Inner>);

#[derive(Default)]
struct Inner {
    active: AtomicUsize,
    failed: AtomicBool,
    changed: Notify,
}

pub(crate) struct Permit(Registry);
impl Drop for Permit {
    fn drop(&mut self) {
        if self.0.0.active.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.0.0.changed.notify_waiters();
        }
    }
}

impl Registry {
    pub(crate) fn enter(&self) -> Permit {
        self.0.active.fetch_add(1, Ordering::AcqRel);
        Permit(self.clone())
    }

    /// Called only after request/scheduler admission is closed and accepted HTTP
    /// handlers have drained. Nested work from those handlers is still tracked;
    /// closing admission never rejects the tail of an already accepted write.
    pub(crate) async fn drain(&self) -> std::io::Result<()> {
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.0.active.load(Ordering::Acquire) == 0 {
                return if self.0.failed.load(Ordering::Acquire) {
                    Err(std::io::Error::other(
                        "Accepted worker quiescence was not confirmed",
                    ))
                } else {
                    Ok(())
                };
            }
            changed.await;
        }
    }
}

pub(crate) fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::default)
}

pub(crate) fn fault() {
    registry().0.failed.store(true, Ordering::Release);
    registry().0.changed.notify_waiters();
}

pub(crate) fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let permit = registry().enter();
    tokio::spawn(async move {
        let _permit = permit;
        future.await
    })
}

pub(crate) fn spawn_blocking<F, R>(operation: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let permit = registry().enter();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        operation()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn detached_accepted_work_remains_visible_until_actual_completion() {
        let registry = Registry::default();
        let permit = registry.enter();
        let (release, released) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _permit = permit;
            let _ = released.await;
        });
        drop(task);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), registry.drain())
                .await
                .is_err()
        );
        release.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), registry.drain())
            .await
            .unwrap()
            .unwrap();
    }
}
