//! Invocation-owned temporary Core evidence for App read adapters.
use crate::{ApiError, AppState, core_call};
use std::{
    path::Path,
    sync::{Arc, Mutex, TryLockError},
};
use weftext_core::{
    AdmittedCompileSnapshot, AdmittedSnapshot, ConfigDocument, ConfigRole, Core, CoreError,
    LibraryStyleDocument, ManagedResourceSnapshotBatch, SnapshotSet,
};

/// These tokens belong to one adapter invocation. They are retained through its
/// final physical checks; prepared Plans and owned witnesses have their own
/// evidence and are unaffected by releasing this temporary read ownership.
#[derive(Default)]
struct ReadonlyTokens {
    cuts: Vec<String>,
    bases: Vec<String>,
    styles: Vec<String>,
}

impl ReadonlyTokens {
    fn release(self, core: &mut Core) {
        for cut in self.cuts {
            core.release_admitted_cut(&cut);
        }
        core.release_snapshot_bases(&self.bases);
        for style in self.styles {
            // All IDs came directly from a successful Core capture.
            let _ = core.discard_library_style_basis(&style);
        }
    }
}

pub(crate) struct ReadonlyEvidence {
    core: Arc<Mutex<Core>>,
    tokens: ReadonlyTokens,
}

impl Drop for ReadonlyEvidence {
    fn drop(&mut self) {
        let tokens = std::mem::take(&mut self.tokens);
        match self.core.try_lock() {
            Ok(mut core) => tokens.release(&mut core),
            Err(TryLockError::Poisoned(error)) => tokens.release(&mut error.into_inner()),
            Err(TryLockError::WouldBlock) => {
                let core = self.core.clone();
                let release = move || {
                    let mut core = core.lock().unwrap_or_else(|error| error.into_inner());
                    tokens.release(&mut core);
                };
                if tokio::runtime::Handle::try_current().is_ok() {
                    // Request retirement must not synchronously queue a Tokio
                    // thread behind an unrelated Core operation. The existing
                    // process work registry also drains this cleanup at exit.
                    crate::work::spawn_blocking(release);
                } else {
                    release();
                }
            }
        }
    }
}

/// Failed captures release tokens through the existing mutable Core borrow,
/// without reacquiring its mutex. Only a successful capture transfers ownership
/// to an independently droppable result, before the request can be retired.
pub(crate) struct ReadonlyCapture<'a> {
    pub(crate) core: &'a mut Core,
    tokens: ReadonlyTokens,
}

impl ReadonlyCapture<'_> {
    pub(crate) fn read_config(
        &mut self,
        root: &Path,
        role: ConfigRole,
    ) -> Result<ConfigDocument, CoreError> {
        let document = self.core.read_config(root, role)?;
        self.tokens.bases.push(document.base_id.clone());
        Ok(document)
    }

    pub(crate) fn read_library_style(
        &mut self,
        root: &Path,
    ) -> Result<LibraryStyleDocument, CoreError> {
        let document = self.core.read_library_style(root)?;
        self.tokens.styles.push(document.base_id.clone());
        Ok(document)
    }

    pub(crate) fn admitted_snapshot(&mut self, root: &Path) -> Result<AdmittedSnapshot, CoreError> {
        let cut = self.core.admitted_snapshot(root)?;
        self.tokens.cuts.push(cut.cut_id.clone());
        Ok(cut)
    }

    pub(crate) fn admitted_compile_snapshot(
        &mut self,
        root: &Path,
    ) -> Result<AdmittedCompileSnapshot, CoreError> {
        let cut = self.core.admitted_compile_snapshot(root, "live")?;
        self.tokens.cuts.push(cut.snapshot.cut_id.clone());
        Ok(cut)
    }

    pub(crate) fn read_snapshot(
        &mut self,
        root: &Path,
        paths: &[String],
    ) -> Result<SnapshotSet, CoreError> {
        let snapshot = self.core.read_snapshot(root, paths)?;
        self.tokens
            .bases
            .extend(snapshot.files.iter().map(|file| file.base_id.clone()));
        Ok(snapshot)
    }

    pub(crate) fn managed_resources(
        &mut self,
        root: &Path,
        paths: &[String],
    ) -> Result<ManagedResourceSnapshotBatch, CoreError> {
        let batch = self.core.managed_resource_snapshots(root, paths)?;
        for (_, resources) in &batch.entries {
            if let Ok(resources) = resources {
                for resource in resources {
                    self.tokens.bases.push(resource.base_id.clone());
                    self.tokens.bases.push(resource.manifest_base_id.clone());
                }
            }
        }
        Ok(batch)
    }

    fn finish<T>(mut self, value: T, core: Arc<Mutex<Core>>) -> (T, Arc<ReadonlyEvidence>) {
        let tokens = std::mem::take(&mut self.tokens);
        (value, Arc::new(ReadonlyEvidence { core, tokens }))
    }
}

impl Drop for ReadonlyCapture<'_> {
    fn drop(&mut self) {
        std::mem::take(&mut self.tokens).release(self.core);
    }
}

pub(crate) async fn capture_readonly<T, F>(
    state: &AppState,
    capture: F,
) -> Result<(T, Arc<ReadonlyEvidence>), ApiError>
where
    T: Send + 'static,
    F: FnOnce(&mut ReadonlyCapture<'_>) -> Result<T, CoreError> + Send + 'static,
{
    let owner = state.core.clone();
    core_call(state, move |core| {
        let mut scope = ReadonlyCapture {
            core,
            tokens: ReadonlyTokens::default(),
        };
        let value = capture(&mut scope)?;
        Ok(scope.finish(value, owner))
    })
    .await
}
