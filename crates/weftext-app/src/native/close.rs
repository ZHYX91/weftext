//! Surface/incarnation/input fencing is distinct from a durable Core proof.
//! The reducer verifies both; a bool from JavaScript is never a close permit.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum SurfaceKind {
    Library,
    Capture,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Request {
    pub surface_id: String,
    pub surface_epoch: String,
    pub close_attempt_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum Message {
    DraftCall {
        surface_id: String,
        surface_epoch: String,
        request_id: String,
        request: DraftRequest,
    },
    SurfaceReady {
        kind: SurfaceKind,
    },
    SurfaceUnready {
        surface_id: String,
        surface_epoch: String,
    },
    SurfaceEdited {
        surface_id: String,
        surface_epoch: String,
        surface_edit_revision: u64,
    },
    ClosePrepared {
        surface_id: String,
        surface_epoch: String,
        close_attempt_id: String,
        surface_edit_revision: u64,
        proofs: Vec<Proof>,
    },
    CloseBlocked {
        surface_id: String,
        surface_epoch: String,
        close_attempt_id: String,
        reason: String,
    },
    CloseFrozen {
        surface_id: String,
        surface_epoch: String,
        close_attempt_id: String,
        surface_edit_revision: u64,
    },
    LocaleChanged {
        surface_id: String,
        surface_epoch: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum DraftRequest {
    Persist {
        body: DraftWrite,
    },
    Resume {
        body: weftext_core::ResumeDraftRequest,
    },
    Flush {
        body: weftext_core::DocumentDraftFlush,
    },
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DraftWrite {
    pub session_id: String,
    pub source: String,
}
impl DraftRequest {
    pub fn session_id(&self) -> &str {
        match self {
            Self::Persist { body } => &body.session_id,
            Self::Resume { body } => &body.session_id,
            Self::Flush { body } => &body.session_id,
        }
    }
    pub fn acknowledges_input(&self) -> bool {
        matches!(self, Self::Persist { .. } | Self::Resume { .. })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum Proof {
    Document {
        owner_id: String,
        owner_generation: String,
        session_id: String,
        base_id: String,
        source_digest: String,
        record_digest: String,
        pending_operation_id: Option<String>,
    },
    Capture {
        owner_id: String,
        owner_generation: String,
        draft_id: String,
        record_revision: u64,
        input_revision: u64,
        last_mutation_id: String,
        record_digest: String,
        pending_operation_id: Option<String>,
    },
}
impl Proof {
    pub fn valid(&self, kind: SurfaceKind) -> bool {
        let digest = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        let owner = |id: &str, generation: &str| {
            !id.is_empty() && id.len() <= 4096 && crate::broker::canonical_uuid(generation)
        };
        match self {
            Self::Document {
                owner_id,
                owner_generation,
                session_id,
                source_digest,
                record_digest,
                pending_operation_id,
                base_id,
            } => {
                kind == SurfaceKind::Library
                    && owner(owner_id, owner_generation)
                    && crate::broker::canonical_uuid(session_id)
                    && digest(source_digest)
                    && digest(record_digest)
                    && !base_id.is_empty()
                    && base_id.len() <= 256
                    && pending_operation_id.is_none()
            }
            Self::Capture {
                owner_id,
                owner_generation,
                draft_id,
                last_mutation_id,
                record_digest,
                pending_operation_id,
                ..
            } => {
                kind == SurfaceKind::Capture
                    && owner(owner_id, owner_generation)
                    && crate::broker::canonical_uuid(draft_id)
                    && crate::broker::canonical_uuid(last_mutation_id)
                    && digest(record_digest)
                    && pending_operation_id
                        .as_deref()
                        .is_none_or(crate::broker::canonical_uuid)
            }
        }
    }
    pub fn owner_key(&self) -> (&str, &str) {
        match self {
            Self::Document {
                owner_id,
                owner_generation,
                ..
            }
            | Self::Capture {
                owner_id,
                owner_generation,
                ..
            } => (owner_id, owner_generation),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Preparing,
    Verifying,
    Freezing,
    FinalVerifying,
    Permitted,
}
struct Attempt {
    request: Request,
    serial: u64,
    stage: Stage,
    proofs: Vec<Proof>,
}

pub(super) struct Gate {
    pub id: String,
    pub epoch: String,
    pub kind: SurfaceKind,
    pub ready: bool,
    serial: u64,
    attempt: Option<Attempt>,
}
impl Gate {
    pub fn new(kind: SurfaceKind) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            epoch: uuid::Uuid::new_v4().to_string(),
            kind,
            ready: false,
            serial: 0,
            attempt: None,
        }
    }
    pub fn begin(&mut self) -> Option<Request> {
        if !self.ready || self.attempt.is_some() {
            return None;
        }
        let request = Request {
            surface_id: self.id.clone(),
            surface_epoch: self.epoch.clone(),
            close_attempt_id: uuid::Uuid::new_v4().to_string(),
        };
        self.attempt = Some(Attempt {
            request: request.clone(),
            serial: self.serial,
            stage: Stage::Preparing,
            proofs: Vec::new(),
        });
        Some(request)
    }
    fn matches(&self, request: &Request, serial: u64) -> bool {
        request.surface_id == self.id
            && request.surface_epoch == self.epoch
            && serial == self.serial
            && self.attempt.as_ref().is_some_and(|attempt| {
                attempt.request.close_attempt_id == request.close_attempt_id
                    && attempt.serial == serial
            })
    }
    pub fn edited(&mut self, id: &str, epoch: &str, serial: u64) -> bool {
        if id != self.id
            || epoch != self.epoch
            || serial > 9_007_199_254_740_991
            || serial < self.serial
        {
            return false;
        }
        if self.serial != serial {
            self.serial = serial;
            self.attempt = None;
        }
        true
    }
    pub fn prepared(
        &mut self,
        request: &Request,
        serial: u64,
        proofs: Vec<Proof>,
    ) -> Option<Vec<Proof>> {
        if !self.matches(request, serial)
            || proofs.len() > 256
            || proofs.iter().any(|proof| !proof.valid(self.kind))
        {
            return None;
        }
        if self.kind == SurfaceKind::Capture && proofs.len() != 1 {
            return None;
        }
        let mut owners = std::collections::HashSet::new();
        if proofs.iter().any(|proof| !owners.insert(proof.owner_key())) {
            return None;
        }
        let attempt = self.attempt.as_mut()?;
        if attempt.stage != Stage::Preparing {
            return None;
        }
        attempt.stage = Stage::Verifying;
        attempt.proofs = proofs.clone();
        Some(proofs)
    }
    pub fn verified(&mut self, request: &Request, serial: u64) -> bool {
        if !self.matches(request, serial) {
            return false;
        }
        let Some(attempt) = self.attempt.as_mut() else {
            return false;
        };
        if attempt.stage != Stage::Verifying {
            return false;
        }
        attempt.stage = Stage::Freezing;
        true
    }
    pub fn frozen(&mut self, request: &Request, serial: u64) -> Option<Vec<Proof>> {
        if !self.matches(request, serial) {
            return None;
        }
        let attempt = self.attempt.as_mut()?;
        if attempt.stage != Stage::Freezing {
            return None;
        }
        attempt.stage = Stage::FinalVerifying;
        Some(attempt.proofs.clone())
    }
    pub fn final_verified(&mut self, request: &Request, serial: u64) -> bool {
        if !self.matches(request, serial) {
            return false;
        }
        let Some(attempt) = self.attempt.as_mut() else {
            return false;
        };
        if attempt.stage != Stage::FinalVerifying {
            return false;
        }
        attempt.stage = Stage::Permitted;
        true
    }
    pub fn permitted(&self) -> bool {
        self.attempt.as_ref().is_some_and(|attempt| {
            attempt.stage == Stage::Permitted && attempt.serial == self.serial
        })
    }
    pub fn cancel(&mut self) {
        self.attempt = None;
    }
    /// An explicit reactivation supersedes this window's prior close intent
    /// even if its source serial did not change. Late verification cannot then
    /// destroy the reactivated surface under the old attempt.
    pub fn reactivate(&mut self) -> bool {
        self.attempt.take().is_some()
    }
    /// Prepare may itself persist/flush the live input. A later newly accepted
    /// call invalidates every prepared/frozen proof before Core can process it.
    pub fn draft_call(&mut self) -> bool {
        if self
            .attempt
            .as_ref()
            .is_some_and(|attempt| attempt.stage != Stage::Preparing)
        {
            self.attempt = None;
            true
        } else {
            false
        }
    }
    pub fn proofs(&self) -> Vec<Proof> {
        self.attempt
            .as_ref()
            .map(|attempt| attempt.proofs.clone())
            .unwrap_or_default()
    }
    pub fn owns_attempt(&self, request: &Request) -> bool {
        self.attempt
            .as_ref()
            .is_some_and(|attempt| attempt.request.close_attempt_id == request.close_attempt_id)
            && request.surface_id == self.id
            && request.surface_epoch == self.epoch
    }
    pub fn closing(&self) -> bool {
        self.attempt.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reactivation_supersedes_a_frozen_attempt_without_reusing_its_late_core_result() {
        let mut gate = Gate::new(SurfaceKind::Library);
        gate.ready = true;
        let old = gate.begin().unwrap();
        assert!(gate.prepared(&old, 0, vec![]).is_some());
        assert!(gate.verified(&old, 0));
        assert!(gate.frozen(&old, 0).is_some());
        assert!(gate.reactivate());
        assert!(!gate.owns_attempt(&old));
        assert!(!gate.final_verified(&old, 0));
        let new = gate.begin().unwrap();
        assert_ne!(new.close_attempt_id, old.close_attempt_id);
        assert!(!gate.final_verified(&old, 0));
        assert!(gate.owns_attempt(&new));
    }
    #[test]
    fn late_core_ack_and_reentrant_close_never_permit_new_input() {
        let mut gate = Gate::new(SurfaceKind::Library);
        gate.ready = true;
        let request = gate.begin().unwrap();
        assert!(gate.begin().is_none());
        assert!(gate.prepared(&request, 0, vec![]).is_some());
        assert!(gate.edited(&gate.id.clone(), &gate.epoch.clone(), 1));
        assert!(!gate.verified(&request, 0));
        assert!(!gate.permitted());
        let request = gate.begin().unwrap();
        assert!(gate.prepared(&request, 1, vec![]).is_some());
        assert!(gate.verified(&request, 1));
        assert!(gate.frozen(&request, 1).is_some());
        assert!(!gate.permitted());
        assert!(gate.final_verified(&request, 1));
        assert!(gate.permitted());
    }
    #[test]
    fn capture_requires_its_actual_draft_proof_and_rejects_old_surface_epoch() {
        let mut gate = Gate::new(SurfaceKind::Capture);
        gate.ready = true;
        let request = gate.begin().unwrap();
        assert!(gate.prepared(&request, 0, vec![]).is_none());
        let mut old = request.clone();
        old.surface_epoch = uuid::Uuid::new_v4().to_string();
        assert!(!gate.verified(&old, 0));
        assert!(!gate.permitted());
    }
}
