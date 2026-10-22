//! Windowless out-of-process COM activation. Only a bounded original opaque ID
//! is accepted; a callback/task keeps its process lease through the real reply.
use super::{error, qualify_gui};
use crate::launch::ServeError;
use std::{
    ffi::c_void,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use windows::{
    Win32::{
        Foundation::{
            CLASS_E_NOAGGREGATION, E_ACCESSDENIED, E_FAIL, E_INVALIDARG, E_POINTER, E_UNEXPECTED,
            HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        System::{
            Com::*,
            Threading::{CreateEventW, SetEvent, WaitForSingleObject},
        },
        UI::Notifications::{
            INotificationActivationCallback, INotificationActivationCallback_Impl,
            NOTIFICATION_USER_INPUT_DATA,
        },
    },
    core::{BOOL, Error, GUID, IUnknown, Interface, PCWSTR, Ref, Result, implement},
};

struct State {
    done: OwnedHandle,
    bootstrap: AtomicBool,
    closed: AtomicBool,
    task_failed: AtomicBool,
    locks: AtomicUsize,
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}
impl State {
    fn reap_finished(&self, tasks: &mut Vec<tokio::task::JoinHandle<()>>) {
        use std::{
            future::Future,
            pin::Pin,
            task::{Context, Poll, Waker},
        };
        let mut context = Context::from_waker(Waker::noop());
        let mut index = 0;
        while index < tasks.len() {
            match Pin::new(&mut tasks[index]).poll(&mut context) {
                Poll::Ready(result) => {
                    if result.is_err() {
                        self.task_failed.store(true, Ordering::Release);
                    }
                    drop(tasks.swap_remove(index));
                }
                Poll::Pending => index += 1,
            }
        }
    }
    fn release(&self) {
        if unsafe { CoReleaseServerProcess() } == 0 {
            // COM automatically suspends class objects at zero. Only the
            // registration thread revokes them and balances its apartment.
            self.closed.store(true, Ordering::Release);
            let _ = unsafe { SetEvent(HANDLE(self.done.as_raw_handle())) };
        }
    }
    fn release_bootstrap(&self) {
        if self.bootstrap.swap(false, Ordering::AcqRel) {
            self.release();
        }
    }
}
struct Lease(Arc<State>);
impl Lease {
    fn new(state: &Arc<State>) -> Self {
        unsafe { CoAddRefServerProcess() };
        Self(state.clone())
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.release();
    }
}

#[implement(INotificationActivationCallback)]
struct Activator {
    lease: Lease,
    runtime: tokio::runtime::Handle,
}
impl INotificationActivationCallback_Impl for Activator_Impl {
    fn Activate(
        &self,
        app_id: &PCWSTR,
        args: &PCWSTR,
        _data: *const NOTIFICATION_USER_INPUT_DATA,
        count: u32,
    ) -> Result<()> {
        if count != 0 || self.lease.0.closed.load(Ordering::Acquire) {
            return Err(Error::from(E_ACCESSDENIED));
        }
        let app_id = unsafe { bounded(*app_id, 128) }?;
        let opaque = unsafe { bounded(*args, 36) }?;
        if !crate::broker::canonical_uuid(&opaque) {
            return Err(Error::from(E_INVALIDARG));
        }
        let mut tasks = self
            .lease
            .0
            .tasks
            .lock()
            .map_err(|_| Error::from(E_UNEXPECTED))?;
        self.lease.0.reap_finished(&mut tasks);
        if tasks.len() >= 32 || self.lease.0.task_failed.load(Ordering::Acquire) {
            return Err(Error::from(E_FAIL));
        }
        let lease = Lease::new(&self.lease.0);
        let (send, received) = std::sync::mpsc::sync_channel(1);
        tasks.push(self.runtime.spawn(async move {
            let _lease = lease;
            let success = super::activate_original_toast(&opaque, &app_id)
                .await
                .is_ok();
            let _ = send.send(success);
        }));
        drop(tasks);
        // The transport owns its connect/handshake/original-call budgets.
        // A second, shorter callback timer must not manufacture an unknown
        // result while that one bounded request is still legitimately running.
        // Caller-side COM cancellation likewise does not cancel this task.
        match received.recv() {
            Ok(true) => Ok(()),
            _ => Err(Error::from(E_FAIL)),
        }
    }
}

#[implement(IClassFactory)]
struct Factory {
    state: Arc<State>,
    runtime: tokio::runtime::Handle,
}
impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        iid: *const GUID,
        output: *mut *mut c_void,
    ) -> Result<()> {
        if output.is_null() {
            return Err(Error::from(E_POINTER));
        }
        unsafe {
            *output = std::ptr::null_mut();
        }
        if iid.is_null() {
            return Err(Error::from(E_POINTER));
        }
        if outer.as_ref().is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }
        if self.state.closed.load(Ordering::Acquire) {
            return Err(Error::from(E_UNEXPECTED));
        }
        let callback: INotificationActivationCallback = Activator {
            lease: Lease::new(&self.state),
            runtime: self.runtime.clone(),
        }
        .into();
        unsafe { callback.query(iid, output) }.ok()?;
        // A real returned object already has its own process ref. Releasing
        // startup ownership here cannot suspend a not-yet-arrived first call.
        self.state.release_bootstrap();
        Ok(())
    }
    fn LockServer(&self, lock: BOOL) -> Result<()> {
        if lock.as_bool() {
            if self.state.closed.load(Ordering::Acquire) {
                return Err(Error::from(E_UNEXPECTED));
            }
            unsafe {
                CoAddRefServerProcess();
            }
            self.state.locks.fetch_add(1, Ordering::AcqRel);
        } else {
            self.state
                .locks
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                    count.checked_sub(1)
                })
                .map_err(|_| Error::from(E_UNEXPECTED))?;
            self.state.release();
        }
        Ok(())
    }
}

/// COM supplies a valid LPCWSTR. Its content is nevertheless bounded before
/// allocation; no unbounded to_string/scan or caller-owned pointer is retained.
unsafe fn bounded(value: PCWSTR, limit: usize) -> Result<String> {
    if value.is_null() {
        return Err(Error::from(E_POINTER));
    }
    for length in 0..=limit {
        if unsafe { *value.0.add(length) } == 0 {
            if length == 0 {
                return Err(Error::from(E_INVALIDARG));
            }
            return String::from_utf16(unsafe { std::slice::from_raw_parts(value.0, length) })
                .map_err(|_| Error::from(E_INVALIDARG));
        }
    }
    Err(Error::from(E_INVALIDARG))
}

struct Apartment {
    state: Option<Arc<State>>,
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self
            .state
            .as_ref()
            .is_none_or(|state| state.closed.load(Ordering::Acquire))
        {
            unsafe { CoUninitialize() };
        }
    }
}
struct Registration(Option<u32>);
impl Registration {
    fn revoke(&mut self) -> Result<()> {
        match self.0.take() {
            Some(cookie) => unsafe { CoRevokeClassObject(cookie) },
            None => Ok(()),
        }
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        let _ = self.revoke();
    }
}

pub(super) async fn run() -> std::result::Result<(), ServeError> {
    qualify_gui()?;
    crate::artifact_catalog::Catalog::get().map_err(|_| {
        error(
            "artifactQualification",
            "The notification server is not a qualified complete build",
        )
    })?;
    let packaged =
        super::winrt::call(|| super::package::current().map(|identity| identity.is_some()))
            .await
            .map_err(|_| {
                error(
                    "toastIdentityUnavailable",
                    "The actual notification package identity is unavailable",
                )
            })?;
    if !packaged {
        return Err(error(
            "toastUnavailable",
            "This portable delivery has no system notification registration",
        ));
    }
    let contract = crate::host_contract::get()
        .map_err(|_| error("hostContract", "The Windows host contract is invalid"))?;
    let clsid = GUID::from_u128(
        uuid::Uuid::parse_str(&contract.activation.toast_clsid)
            .map_err(|_| error("hostContract", "The notification class is invalid"))?
            .as_u128(),
    );
    let runtime = tokio::runtime::Handle::current();
    crate::work::spawn_blocking(move || server(clsid, runtime))
        .await
        .map_err(|_| {
            error(
                "toastServerUnknown",
                "The original notification server thread did not finish",
            )
        })?
        .map_err(|_| {
            error(
                "toastServerFailed",
                "The notification server did not retire cleanly",
            )
        })
}

fn server(clsid: GUID, runtime: tokio::runtime::Handle) -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok()?;
    let mut apartment = Apartment { state: None };
    let event = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }?;
    let state = Arc::new(State {
        done: unsafe { OwnedHandle::from_raw_handle(event.0) },
        bootstrap: AtomicBool::new(true),
        closed: AtomicBool::new(false),
        task_failed: AtomicBool::new(false),
        locks: AtomicUsize::new(0),
        tasks: Mutex::new(Vec::new()),
    });
    unsafe {
        CoAddRefServerProcess();
    }
    apartment.state = Some(state.clone());
    let factory: IClassFactory = Factory {
        state: state.clone(),
        runtime: runtime.clone(),
    }
    .into();
    let mut registration = match unsafe {
        CoRegisterClassObject(
            &clsid,
            &factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE | REGCLS_SUSPENDED,
        )
    } {
        Ok(cookie) => Registration(Some(cookie)),
        Err(error) => {
            state.release_bootstrap();
            return Err(error);
        }
    };
    let mut failure = unsafe { CoResumeClassObjects() }.err();
    let wait = if failure.is_none() {
        unsafe { WaitForSingleObject(event, 30_000) }
    } else {
        WAIT_TIMEOUT
    };
    if wait != WAIT_OBJECT_0 && wait != WAIT_TIMEOUT {
        failure.get_or_insert(Error::from(E_FAIL));
    }
    if failure.is_some() || (wait == WAIT_TIMEOUT && state.bootstrap.load(Ordering::Acquire)) {
        // Stop registration before releasing startup ownership. Existing
        // factory locks/objects/jobs remain counted through their real tail.
        if let Err(error) = unsafe { CoSuspendClassObjects() } {
            failure.get_or_insert(error);
        }
        if let Err(error) = registration.revoke() {
            failure.get_or_insert(error);
        }
        state.release_bootstrap();
    }
    // After the first object was returned this server has no idle deadline:
    // it waits for actual object, LockServer and callback-task reference zero.
    if !state.closed.load(Ordering::Acquire)
        && unsafe { WaitForSingleObject(event, u32::MAX) } != WAIT_OBJECT_0
    {
        failure.get_or_insert(Error::from(E_FAIL));
        // A broken wake handle cannot turn live COM callbacks into a completed
        // teardown. The reference-zero flag is independently published only by
        // the real final CoReleaseServerProcess result.
        while !state.closed.load(Ordering::Acquire) {
            std::thread::park_timeout(Duration::from_millis(100));
        }
    }
    if let Err(error) = registration.revoke() {
        failure.get_or_insert(error);
    }
    let tasks = std::mem::take(&mut *state.tasks.lock().map_err(|_| Error::from(E_UNEXPECTED))?);
    if let Err(error) = runtime.block_on(async {
        let mut failed = false;
        for task in tasks {
            if task.await.is_err() {
                failed = true;
            }
        }
        if failed {
            Err(Error::from(E_FAIL))
        } else {
            Ok(())
        }
    }) {
        failure.get_or_insert(error);
    }
    drop(factory);
    if state.task_failed.load(Ordering::Acquire) {
        failure.get_or_insert(Error::from(E_FAIL));
    }
    failure.map_or(Ok(()), Err)
}
