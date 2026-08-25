use serde::{Deserialize, Serialize};

use crate::{ImportError, ImportErrorCode};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportLimits {
    pub max_source_bytes: u64,
    pub max_probe_bytes: u64,
    pub max_pages: u32,
    pub max_container_entries: u32,
    pub max_ir_nodes: u32,
    pub max_ir_depth: u16,
    pub max_text_bytes: u64,
    pub max_resource_count: u32,
    pub max_resource_bytes: u64,
    pub max_total_output_bytes: u64,
    pub max_diagnostics: u32,
    pub max_agent_selected_nodes: u32,
    pub max_agent_operations: u32,
    pub max_agent_output_bytes: u64,
    pub worker_memory_bytes: u64,
    pub worker_timeout_ms: u64,
    pub cancellation_grace_ms: u64,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 64 * 1024 * 1024,
            max_probe_bytes: 64 * 1024,
            max_pages: 1_000,
            max_container_entries: 10_000,
            max_ir_nodes: 100_000,
            max_ir_depth: 64,
            max_text_bytes: 32 * 1024 * 1024,
            max_resource_count: 5_000,
            max_resource_bytes: 64 * 1024 * 1024,
            max_total_output_bytes: 512 * 1024 * 1024,
            max_diagnostics: 10_000,
            max_agent_selected_nodes: 1_000,
            max_agent_operations: 1_000,
            max_agent_output_bytes: 4 * 1024 * 1024,
            worker_memory_bytes: 2 * 1024 * 1024 * 1024,
            worker_timeout_ms: 5 * 60 * 1_000,
            cancellation_grace_ms: 1_000,
        }
    }
}

impl ImportLimits {
    /// Validates relationships between every configured resource limit.
    ///
    /// # Errors
    ///
    /// Returns an error for zero or internally contradictory limits.
    pub fn validate(&self) -> Result<(), ImportError> {
        let nonzero = [
            self.max_source_bytes,
            self.max_probe_bytes,
            u64::from(self.max_pages),
            u64::from(self.max_container_entries),
            u64::from(self.max_ir_nodes),
            u64::from(self.max_ir_depth),
            self.max_text_bytes,
            u64::from(self.max_resource_count),
            self.max_resource_bytes,
            self.max_total_output_bytes,
            u64::from(self.max_diagnostics),
            u64::from(self.max_agent_selected_nodes),
            u64::from(self.max_agent_operations),
            self.max_agent_output_bytes,
            self.worker_memory_bytes,
            self.worker_timeout_ms,
            self.cancellation_grace_ms,
        ];
        if nonzero.contains(&0) {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "all import limits must be non-zero",
            ));
        }
        if self.max_probe_bytes > self.max_source_bytes
            || self.max_resource_bytes > self.max_total_output_bytes
            || self.max_agent_output_bytes > self.max_total_output_bytes
        {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "dependent import limits must not exceed their enclosing byte limit",
            ));
        }
        Ok(())
    }

    // A method keeps each check explicitly attached to the validated limit
    // snapshot at call sites even though the selected maximum is passed in.
    #[allow(clippy::unused_self)]
    pub(crate) fn check(&self, label: &str, actual: u64, maximum: u64) -> Result<(), ImportError> {
        if actual > maximum {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                format!("{label} is {actual}, exceeding the limit of {maximum}"),
            ));
        }
        Ok(())
    }
}
