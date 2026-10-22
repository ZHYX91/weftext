//! The HTTP completion signal may precede destruction of its connection's
//! Router. Keep actual service and request-future tails until their inner
//! authority has been dropped, independently of response/connection signals.
use crate::work::{Permit, Registry};
use axum::{Router, ServiceExt, extract::Request, routing::IntoMakeService};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

pub(crate) fn track(router: Router) -> (IntoMakeService<Tracked<Router>>, Registry) {
    let registry = Registry::default();
    let service = Tracked::new(router, registry.clone());
    (ServiceExt::<Request>::into_make_service(service), registry)
}

pub(crate) struct Tracked<S> {
    // Rust drops fields in declaration order. All inner authority must be
    // destroyed before the scope can observe this lease's completion.
    inner: S,
    _permit: Permit,
    registry: Registry,
}
impl<S> Tracked<S> {
    fn new(inner: S, registry: Registry) -> Self {
        Self {
            inner,
            _permit: registry.enter(),
            registry,
        }
    }
}
impl<S: Clone> Clone for Tracked<S> {
    fn clone(&self) -> Self {
        Self::new(self.inner.clone(), self.registry.clone())
    }
}
impl<S, R> Service<R> for Tracked<S>
where
    S: Service<R>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = TrackedFuture<S::Future>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: R) -> Self::Future {
        TrackedFuture {
            inner: Box::pin(self.inner.call(request)),
            _permit: self.registry.enter(),
        }
    }
}

pub(crate) struct TrackedFuture<F> {
    inner: Pin<Box<F>>,
    _permit: Permit,
}
impl<F: Future> Future for TrackedFuture<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().inner.as_mut().poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        convert::Infallible,
        sync::{Arc, Condvar, Mutex},
        time::Duration,
    };

    struct HeldDrop {
        entered: Option<tokio::sync::oneshot::Sender<()>>,
        gate: Arc<(Mutex<bool>, Condvar)>,
        _authority: Arc<()>,
    }
    impl Drop for HeldDrop {
        fn drop(&mut self) {
            self.entered.take().unwrap().send(()).unwrap();
            let (lock, changed) = &*self.gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = changed.wait(released).unwrap();
            }
        }
    }
    fn release(gate: &Arc<(Mutex<bool>, Condvar)>) {
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn service_scope_waits_for_actual_inner_authority_destruction() {
        let authority = Arc::new(());
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let (entered, inside) = tokio::sync::oneshot::channel();
        let registry = Registry::default();
        let service = Tracked::new(
            HeldDrop {
                entered: Some(entered),
                gate: gate.clone(),
                _authority: authority.clone(),
            },
            registry.clone(),
        );
        let dropping = tokio::task::spawn_blocking(move || drop(service));
        inside.await.unwrap();
        let premature = tokio::time::timeout(Duration::from_millis(20), registry.drain()).await;
        let retained = Arc::strong_count(&authority);
        release(&gate);
        dropping.await.unwrap();
        assert!(premature.is_err());
        assert_eq!(retained, 2);
        registry.drain().await.unwrap();
        assert!(Arc::try_unwrap(authority).is_ok());
    }

    struct ReadyHolding(HeldDrop);
    impl Future for ReadyHolding {
        type Output = Result<(), Infallible>;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            let _ = &self.0;
            Poll::Ready(Ok(()))
        }
    }
    struct OneRequest(Option<ReadyHolding>);
    impl Service<()> for OneRequest {
        type Response = ();
        type Error = Infallible;
        type Future = ReadyHolding;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, _: ()) -> Self::Future {
            self.0.take().unwrap()
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ready_request_scope_waits_for_actual_future_authority_destruction() {
        for ready in [false, true] {
            let authority = Arc::new(());
            let gate = Arc::new((Mutex::new(false), Condvar::new()));
            let (entered, inside) = tokio::sync::oneshot::channel();
            let registry = Registry::default();
            let mut service = Tracked::new(
                OneRequest(Some(ReadyHolding(HeldDrop {
                    entered: Some(entered),
                    gate: gate.clone(),
                    _authority: authority.clone(),
                }))),
                registry.clone(),
            );
            let mut future = service.call(());
            if ready {
                std::future::poll_fn(|cx| Pin::new(&mut future).poll(cx))
                    .await
                    .unwrap();
            }
            drop(service);
            let dropping = tokio::task::spawn_blocking(move || drop(future));
            inside.await.unwrap();
            let premature = tokio::time::timeout(Duration::from_millis(20), registry.drain()).await;
            let retained = Arc::strong_count(&authority);
            release(&gate);
            dropping.await.unwrap();
            assert!(premature.is_err());
            assert_eq!(retained, 2);
            registry.drain().await.unwrap();
            assert!(Arc::try_unwrap(authority).is_ok());
        }
    }
}
