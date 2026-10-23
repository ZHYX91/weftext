use super::*;
use weftext_core::CaptureText;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn fixture() -> (tempfile::TempDir, HostedState, Auth) {
    let (temp, state, auth) = super::super::tests::fixture();
    state
        .inner
        .lock()
        .unwrap()
        .policy
        .users
        .get_mut("alice")
        .unwrap()
        .structure
        .destinations
        .insert(
            "Created".into(),
            DestinationRights {
                create: true,
                receive: false,
                members: super::super::super::Rights::default(),
            },
        );
    (temp, state, auth)
}
async fn draft(state: &HostedState, auth: &Auth, draft_id: &str) -> Value {
    let mut input = CaptureInput::default();
    input.body = CaptureText {
        utf16: vec![65, 0xd800, 0, 0xdc00],
    };
    input.author_texts = weftext_core::CaptureAuthorTexts {
        body: vec![weftext_core::CaptureRawAuthor {
            text: input.body.clone(),
            provenance: weftext_core::CaptureRawProvenance {
                producer_id: id(),
                input_id: "a".repeat(32),
                revision: 1,
            },
        }],
        ..Default::default()
    };
    create(
        State(state.clone()),
        Extension(auth.clone()),
        Json(Create {
            draft_id: draft_id.into(),
            mutation_id: id(),
            slot: "Created".into(),
            input,
        }),
    )
    .await
    .unwrap()
    .0
}
#[tokio::test]
async fn revoked_owner_recovers_and_persists_only_own_raw_across_real_core_restart() {
    let (temp, state, auth) = fixture();
    let draft_id = id();
    let first = draft(&state, &auth, &draft_id).await;
    assert!(first["active"].is_object());
    {
        let mut inner = state.inner.lock().unwrap();
        inner.policy.users.get_mut("alice").unwrap().structure = Rights::default();
    }
    let recovery = read(
        State(state.clone()),
        Extension(auth.clone()),
        Json(DraftId {
            draft_id: draft_id.clone(),
        }),
    )
    .await
    .unwrap()
    .0;
    assert!(recovery["active"].is_null());
    assert_eq!(
        recovery["recovery"]["texts"]["body"][0]["text"]["utf16"],
        json!([65, 55296, 0, 56320])
    );
    let text = serde_json::to_string(&recovery).unwrap();
    for forbidden in [
        "rootHint",
        "pathHint",
        "formBasis",
        "templateId",
        "\"fields\"",
        "wireDigest",
        "\"wire\"",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    let mut input = CaptureAuthorInput {
        texts: weftext_core::CaptureAuthorTexts {
            body: vec![weftext_core::CaptureRawAuthor {
                text: CaptureText::from_text("last key"),
                provenance: weftext_core::CaptureRawProvenance {
                    producer_id: id(),
                    input_id: "b".repeat(32),
                    revision: 1,
                },
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    input.texts.body[0].text.utf16.push(0xd800);
    let mutation_id = id();
    let expected_revision = recovery["recovery"]["revision"].as_u64().unwrap();
    let saved = author_input(
        State(state.clone()),
        Extension(auth.clone()),
        Json(AuthorInput {
            draft_id: draft_id.clone(),
            mutation_id: mutation_id.clone(),
            expected_revision,
            input: input.clone(),
        }),
    )
    .await
    .unwrap()
    .0;
    let lookup = author_mutation(
        State(state.clone()),
        Extension(auth.clone()),
        Json(AuthorInput {
            draft_id: draft_id.clone(),
            mutation_id: mutation_id.clone(),
            expected_revision,
            input: input.clone(),
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(lookup["status"], "applied");
    input.texts.body[0].text.utf16.push(66);
    let conflict = author_mutation(
        State(state.clone()),
        Extension(auth.clone()),
        Json(AuthorInput {
            draft_id: draft_id.clone(),
            mutation_id,
            expected_revision,
            input,
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(conflict["status"], "differentCommand");
    {
        let mut inner = state.inner.lock().unwrap();
        let placeholder = Core::new(temp.path().join("placeholder")).unwrap();
        drop(std::mem::replace(&mut inner.core, placeholder));
        inner.core = Core::new(temp.path().join("state")).unwrap();
    }
    let reopened = read(
        State(state.clone()),
        Extension(auth.clone()),
        Json(DraftId {
            draft_id: draft_id.clone(),
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(reopened["recovery"], saved["recovery"]);
    let other = Auth {
        digest: digest(b"bob"),
    };
    // The HTTP authentication boundary supplies actor identity; the DTO has no
    // actor, owner path or resource capability for a caller to substitute.
    assert!(
        read(
            State(state.clone()),
            Extension(other),
            Json(DraftId {
                draft_id: draft_id.clone()
            })
        )
        .await
        .is_err()
    );
    let mut forged = json!({"draftId":draft_id});
    forged["actor"] = json!("bob");
    assert!(serde_json::from_value::<DraftId>(forged).is_err());
}
#[tokio::test]
async fn pending_wire_is_never_returned_and_unknown_original_is_settled_before_advance() {
    let (_temp, state, auth) = fixture();
    let draft_id = id();
    let first = draft(&state, &auth, &draft_id).await;
    let operation_id = id();
    let request = Prepare {
        operation_id: operation_id.clone(),
        capture: Some(Capture {
            body: "own body".into(),
            ..Default::default()
        }),
        ..ordinary("Created")
    };
    let wire = serde_json::to_string(&request).unwrap();
    let frozen = freeze(
        State(state.clone()),
        Extension(auth.clone()),
        Json(Freeze {
            draft_id: draft_id.clone(),
            expected_revision: first["recovery"]["revision"].as_u64().unwrap(),
            mutation_id: id(),
            request: FrozenCaptureRequest {
                operation_id: operation_id.clone(),
                request_kind: "hostedStructure".into(),
                wire_digest: digest(wire.as_bytes()),
                wire,
                input_revision: first["recovery"]["inputRevision"].as_u64().unwrap(),
            },
        }),
    )
    .await
    .unwrap()
    .0;
    let text = serde_json::to_string(&frozen).unwrap();
    assert!(!text.contains("\"wire\""));
    assert!(!text.contains("wireDigest"));
    assert_eq!(
        operation(
            State(state.clone()),
            Extension(auth.clone()),
            Json(DraftId {
                draft_id: draft_id.clone()
            })
        )
        .await
        .unwrap()
        .0["status"],
        "unknown"
    );
    let expected_revision = frozen["recovery"]["revision"].as_u64().unwrap();
    assert!(
        advance(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Advance {
                draft_id: draft_id.clone(),
                expected_revision,
                mutation_id: id(),
                operation_id: operation_id.clone(),
                reset_input: false
            })
        )
        .await
        .is_err()
    );
    assert_eq!(
        settle(
            State(state.clone()),
            Extension(auth.clone()),
            Json(DraftId {
                draft_id: draft_id.clone()
            })
        )
        .await
        .unwrap()
        .0["status"],
        "cancelled"
    );
    let next = advance(
        State(state.clone()),
        Extension(auth.clone()),
        Json(Advance {
            draft_id: draft_id.clone(),
            expected_revision,
            mutation_id: id(),
            operation_id,
            reset_input: true,
        }),
    )
    .await
    .unwrap()
    .0;
    assert!(next["recovery"]["pending"].is_null());
    assert_eq!(next["recovery"]["texts"], first["recovery"]["texts"]);
    assert!(
        !state
            .inner
            .lock()
            .unwrap()
            .config
            .root
            .join("Created")
            .exists()
    );
}
