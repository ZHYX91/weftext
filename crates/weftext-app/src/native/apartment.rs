use std::{cell::Cell, rc::Rc};
use windows::{
    Win32::{
        Foundation::E_UNEXPECTED,
        System::Com::{
            COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
        },
    },
    core::{Error, Result},
};

/// The non-Send guard belongs to the only WebView thread. S_FALSE is balanced
/// exactly once as well. Normal finish requires every completion to retire.
pub(super) struct Sta {
    pending: Rc<Cell<usize>>,
    finished: bool,
}
impl Sta {
    pub fn enter(pending: Rc<Cell<usize>>) -> Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }.ok()?;
        Ok(Self {
            pending,
            finished: false,
        })
    }
    pub fn finish(mut self) -> Result<()> {
        if self.pending.get() != 0 {
            return Err(Error::from(E_UNEXPECTED));
        }
        unsafe {
            CoUninitialize();
        }
        self.finished = true;
        Ok(())
    }
}
impl Drop for Sta {
    fn drop(&mut self) {
        if !self.finished && self.pending.get() == 0 {
            unsafe {
                CoUninitialize();
            }
        }
        // A fatal pump failure is not a normal COM teardown. The host must first
        // quiesce its independent Core/worker supervisor and return a failure;
        // never uninitialize an apartment with callbacks still outstanding.
    }
}
