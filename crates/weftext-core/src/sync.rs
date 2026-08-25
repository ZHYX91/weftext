use serde::{Deserialize, Serialize};

use crate::{InventoryIssueCode, WorkspaceInventory};

/// Backend-owned evidence about whether a node-local annotation sidecar can be
/// treated as present or absent. This value is supplied by the workspace
/// backend, never by an annotation request body.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationReplicaCompleteness {
    /// The selected local workspace is the complete authority observed by the
    /// Desktop, CLI, backup, or another local backend.
    CompleteLocalWorkspace,
    /// Weftext Server owns and serializes the complete hosted workspace.
    CompleteHostedWorkspace,
    /// A synchronizer has explicitly reported that more node-local files may
    /// still arrive.
    PartialReplica,
    /// The backend cannot prove whether its replica is complete.
    Unknown,
}

impl AnnotationReplicaCompleteness {
    pub(crate) fn is_complete(self) -> bool {
        matches!(
            self,
            Self::CompleteLocalWorkspace | Self::CompleteHostedWorkspace
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyncDisposition {
    Ready,
    WaitForMoreFiles { issues: Vec<InventoryIssueCode> },
    NeedsUserResolution { issues: Vec<InventoryIssueCode> },
}

#[must_use]
pub fn classify_sync_state(inventory: &WorkspaceInventory) -> SyncDisposition {
    if inventory.is_valid() {
        return SyncDisposition::Ready;
    }
    let issues = inventory
        .issues
        .iter()
        .map(|issue| issue.code)
        .collect::<Vec<_>>();
    let incomplete_only = issues.iter().all(|code| {
        matches!(
            code,
            InventoryIssueCode::MissingNodeDocument
                | InventoryIssueCode::MissingIdentity
                | InventoryIssueCode::DocumentUnreadable
        )
    });
    if incomplete_only {
        SyncDisposition::WaitForMoreFiles { issues }
    } else {
        SyncDisposition::NeedsUserResolution { issues }
    }
}
