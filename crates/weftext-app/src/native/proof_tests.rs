use super::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use weftext_core::{Core, DocumentDraftEvidence, DocumentDraftFlush, DraftInputRequest};

#[tokio::test]
async fn actual_native_verify_and_retire_reprove_exact_input_while_other_producers_change() {
    for saved_view in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        std::fs::write(&path, "B").unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let a = core.open(&path, None).unwrap();
        let b = core.open(&path, None).unwrap();
        let input_id = if saved_view {
            a.view_input_id.clone()
        } else {
            uuid::Uuid::new_v4().to_string()
        };
        let source = if saved_view { "B" } else { "A" };
        core.persist_draft_ordered(DraftInputRequest {
            session_id: a.session_id.clone(),
            source: source.into(),
            input_revision: 1,
            input_id: input_id.clone(),
            base_id: Some(a.base_id.clone()),
            retained_base_source: None,
        })
        .unwrap();
        let flush = || DocumentDraftFlush {
            session_id: a.session_id.clone(),
            base_id: a.base_id.clone(),
            source_digest: format!("{:x}", Sha256::digest(source.as_bytes())),
            input_id: Some(input_id.clone()),
        };
        let receipt = core.flush_document_draft(flush()).unwrap();
        let raw = serde_json::json!({"kind":"document","ownerId":"real-body","ownerGeneration":uuid::Uuid::new_v4().to_string(),
            "sessionId":a.session_id,"baseId":a.base_id,"sourceDigest":flush().source_digest,"inputId":input_id,
            "evidence":receipt.evidence,"recordDigest":receipt.record_digest});
        let proof: Proof = serde_json::from_value(raw.clone()).unwrap();
        assert!(proof.valid(SurfaceKind::Library));
        let mut missing = raw.clone();
        missing.as_object_mut().unwrap().remove("inputId");
        assert!(serde_json::from_value::<Proof>(missing).is_err());
        let state = crate::AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from("test-token"),
            port: 0,
            locations: Arc::new(Mutex::new(HashMap::new())),
            renders: Arc::new(Mutex::new(HashMap::new())),
            editor_versions: Arc::new(Mutex::new(HashMap::new())),
            exports: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            native_status: Default::default(),
            sync_remote: Default::default(),
            initial: None,
        };
        assert!(verify(&state, vec![proof.clone()]).await);
        for revision in [1, 2] {
            state
                .core
                .lock()
                .unwrap()
                .persist_draft_ordered(DraftInputRequest {
                    session_id: b.session_id.clone(),
                    source: format!("B producer {revision}"),
                    input_revision: revision,
                    input_id: uuid::Uuid::new_v4().to_string(),
                    base_id: Some(b.base_id.clone()),
                    retained_base_source: None,
                })
                .unwrap();
            let current = state
                .core
                .lock()
                .unwrap()
                .flush_document_draft(flush())
                .unwrap();
            if !saved_view {
                assert_ne!(current.record_digest, receipt.record_digest);
            }
            assert!(verify(&state, vec![proof.clone()]).await);
        }
        let mut wrong = raw.clone();
        wrong["inputId"] = serde_json::json!(uuid::Uuid::new_v4().to_string());
        let wrong: Proof = serde_json::from_value(wrong).unwrap();
        assert!(!verify(&state, vec![wrong.clone()]).await);
        assert!(!retire(&state, vec![wrong], vec![a.session_id.clone()]).await);
        assert!(
            state
                .core
                .lock()
                .unwrap()
                .flush_document_draft(flush())
                .is_ok()
        );
        // A real same-byte new author generation must invalidate the former
        // proof, even while every source/Base/producer byte is unchanged.
        let next_input = uuid::Uuid::new_v4().to_string();
        state
            .core
            .lock()
            .unwrap()
            .persist_draft_ordered(DraftInputRequest {
                session_id: a.session_id.clone(),
                source: source.into(),
                input_revision: 2,
                input_id: next_input.clone(),
                base_id: Some(a.base_id.clone()),
                retained_base_source: None,
            })
            .unwrap();
        assert!(!verify(&state, vec![proof.clone()]).await);
        assert!(!retire(&state, vec![proof], vec![a.session_id.clone()]).await);
        let receipt = state
            .core
            .lock()
            .unwrap()
            .flush_document_draft(DocumentDraftFlush {
                input_id: Some(next_input.clone()),
                ..flush()
            })
            .unwrap();
        let mut renewed = raw;
        renewed["inputId"] = serde_json::json!(next_input);
        renewed["evidence"] = serde_json::json!(receipt.evidence);
        renewed["recordDigest"] = serde_json::json!(receipt.record_digest);
        let proof: Proof = serde_json::from_value(renewed).unwrap();
        assert!(verify(&state, vec![proof.clone()]).await);
        assert!(retire(&state, vec![proof], vec![a.session_id.clone()]).await);
        assert_eq!(
            state
                .core
                .lock()
                .unwrap()
                .flush_document_draft(flush())
                .unwrap_err()
                .code,
            "sessionMissing"
        );
        assert_eq!(
            state
                .core
                .lock()
                .unwrap()
                .flush_document_draft(DocumentDraftFlush {
                    session_id: b.session_id,
                    base_id: b.base_id,
                    source_digest: format!("{:x}", Sha256::digest(b"B producer 2")),
                    input_id: None
                })
                .unwrap()
                .evidence,
            DocumentDraftEvidence::AuthorDraft
        );
    }
}
