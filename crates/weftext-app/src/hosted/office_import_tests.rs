use super::*;
use std::collections::BTreeMap;
use std::os::windows::process::CommandExt;

fn fixture() -> (tempfile::TempDir, HostedState, Auth) {
    let (temp, state, auth, _, _, _) = history_api::test_fixture();
    {
        let mut inner = state.inner.lock().unwrap();
        inner.config.files.insert(
            "Import".into(),
            FileEntry {
                path: "Imported/document/document.typ".into(),
                title: "Import slot".into(),
                lifecycle: "live".into(),
                expected_node: None,
                editable: true,
                importable: true,
                inputs: vec![],
            },
        );
        inner.policy.users.get_mut("alice").unwrap().grants.insert(
            "Import".into(),
            Rights {
                read: true,
                write: true,
                ..Default::default()
            },
        );
    }
    (temp, state, auth)
}
fn document(temp: &Path) -> Vec<u8> {
    let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
    let render = engine
        .render("Imported exact body", &temp.join("producer.typ"), temp)
        .unwrap();
    assert!(render.ok);
    weftext_export::docx::export_docx(weftext_export::docx::DocxInput {
        render: &render,
        template: None,
        bindings: &BTreeMap::new(),
        options: Default::default(),
    })
    .unwrap()
    .bytes
}
fn request(bytes: &[u8]) -> Prepare {
    Prepare {
        resource: "Import".into(),
        operation_id: uuid::Uuid::new_v4().to_string(),
        bytes_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        format: "docx".into(),
        legacy_backend: None,
        docx_options: None,
    }
}
#[tokio::test]
async fn docx_inspect_and_options_require_current_target_and_do_not_grant_legacy_formats() {
    let (temp, state, auth) = fixture();
    let bytes = document(temp.path());
    assert!(
        inspect(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Inspect {
                resource: "Import".into(),
                format: "docx".into(),
                bytes_base64: base64::engine::general_purpose::STANDARD.encode(&bytes)
            })
        )
        .await
        .is_err()
    );
    {
        let mut inner = state.inner.lock().unwrap();
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Import")
            .unwrap()
            .office
            .r#import = true;
    }
    let Json(analysis) = inspect(
        State(state.clone()),
        Extension(auth.clone()),
        Json(Inspect {
            resource: "Import".into(),
            format: "docx".into(),
            bytes_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        }),
    )
    .await
    .unwrap();
    assert_eq!(analysis["contentHash"], digest(&bytes));
    let Json(targets) = targets(State(state.clone()), Extension(auth.clone()))
        .await
        .unwrap();
    assert_eq!(targets["targets"][0]["formats"], json!(["docx"]));
    let mut valid = request(&bytes);
    valid.docx_options = Some(weftext_export::docx::DocxImportOptions {
        content_hash: analysis["contentHash"].as_str().unwrap().into(),
        caption_pairings: vec![],
    });
    let Json(plan) = prepare(
        State(state.clone()),
        Extension(auth.clone()),
        Json(valid.clone()),
    )
    .await
    .unwrap();
    assert_eq!(plan["status"], "prepared");
    assert!(!temp.path().join("library/Imported").exists());
    {
        let mut inner = state.inner.lock().unwrap();
        let rights = &mut inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Import")
            .unwrap()
            .office;
        rights.r#import = false;
        rights.doc.r#import = true;
    }
    assert!(
        settle(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                resource: "Import".into(),
                operation_id: valid.operation_id.clone(),
                format: Some("docx".into()),
            })
        )
        .await
        .is_err()
    );
    {
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let actor_scope = scope(&inner, "alice");
        let original = inner
            .core
            .with_actor_for_root(&actor_scope, &root, |core| {
                core.actor_feature_status("Import", &valid.operation_id)
            })
            .unwrap()
            .unwrap();
        assert_eq!(original.status, "prepared");
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Import")
            .unwrap()
            .office
            .r#import = true;
    }
    let mut legacy = valid.clone();
    legacy.operation_id = uuid::Uuid::new_v4().to_string();
    legacy.format = "doc".into();
    assert!(
        prepare(State(state.clone()), Extension(auth.clone()), Json(legacy))
            .await
            .is_err()
    );
    let mut invalid = valid;
    invalid.operation_id = uuid::Uuid::new_v4().to_string();
    invalid.docx_options.as_mut().unwrap().content_hash = "stale".into();
    assert!(
        prepare(State(state), Extension(auth), Json(invalid))
            .await
            .is_err()
    );
}
#[test]
fn import_fingerprint_keeps_old_default_and_freezes_format_backend_and_caption_options() {
    let original = request(b"uploaded");
    let hash = digest(b"uploaded");
    let old =
        digest(&serde_json::to_vec(&json!({"resource":"Import","uploadDigest":hash})).unwrap());
    assert_eq!(fingerprint(&original, &hash).unwrap(), old);
    let mut changed = original.clone();
    changed.format = "doc".into();
    assert_ne!(fingerprint(&changed, &hash).unwrap(), old);
    let first = fingerprint(&changed, &hash).unwrap();
    changed.legacy_backend = Some("configured".into());
    assert_ne!(fingerprint(&changed, &hash).unwrap(), first);
    changed = original;
    changed.docx_options = Some(weftext_export::docx::DocxImportOptions {
        content_hash: hash.clone(),
        caption_pairings: vec![],
    });
    assert_ne!(fingerprint(&changed, &hash).unwrap(), old);
}
#[tokio::test]
async fn external_import_cancel_is_actor_scoped_and_revocation_stops_owned_run() {
    let (temp, state, auth) = fixture();
    let flag = Arc::new(AtomicBool::new(false));
    let id = uuid::Uuid::new_v4().to_string();
    {
        let mut inner = state.inner.lock().unwrap();
        inner.office = office::Store::with_legacy_root(temp.path().join("legacy-office"));
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Import")
            .unwrap()
            .office
            .doc
            .r#import = true;
        let file = inner.file("Import").unwrap();
        let root = legacy_root(&inner, "alice").unwrap();
        retain_claim(
            inner.office.legacy_root.as_deref().unwrap(),
            &root,
            ImportClaim {
                schema_version: 1,
                fingerprint: "fingerprint".into(),
                decision: Decision {
                    actor_epoch: inner.actor_generation("alice"),
                    expires_at_unix_ms: now_ms() + 100000,
                    config_hash: config_hash(&inner, &file).unwrap(),
                    upload_digest: "upload".into(),
                    format: "doc".into(),
                },
                resource: "Import".into(),
                operation_id: id.clone(),
                format: "doc".into(),
                legacy_backend: Some("libreoffice".into()),
                docx_options: None,
                conversion_layout: None,
            },
        )
        .unwrap();
        let epoch = inner.actor_generation("alice");
        inner.office.import_active.insert(
            ("alice".into(), id.clone()),
            office::LegacyImportRun {
                actor_epoch: epoch,
                cancel: flag.clone(),
            },
        );
    }
    let Json(pending) = operation(
        State(state.clone()),
        Extension(auth.clone()),
        Json(Operation {
            resource: "Import".into(),
            operation_id: id.clone(),
            format: None,
        }),
    )
    .await
    .unwrap();
    assert_eq!(pending["status"], "pending");
    assert_eq!(pending["preview"], json!({}));
    let Json(cancelled) = cancel(
        State(state.clone()),
        Extension(auth),
        Json(Operation {
            resource: "Import".into(),
            operation_id: id.clone(),
            format: None,
        }),
    )
    .await
    .unwrap();
    assert_eq!(cancelled["status"], "cancelling");
    assert!(flag.load(Ordering::Acquire));
    {
        let mut inner = state.inner.lock().unwrap();
        inner.policy.users.get_mut("alice").unwrap().epoch += 1;
        office::expire_revoked(&mut inner);
        assert!(
            !inner
                .office
                .import_active
                .contains_key(&("alice".into(), id))
        );
    }
}
#[test]
fn persistent_import_claim_requires_exact_sealed_original_and_never_accepts_partial_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("imports");
    let id = uuid::Uuid::new_v4().to_string();
    let original = ImportClaim {
        schema_version: 1,
        fingerprint: "exact".into(),
        decision: Decision {
            actor_epoch: 3,
            expires_at_unix_ms: 1000,
            config_hash: "target".into(),
            upload_digest: "original".into(),
            format: "doc".into(),
        },
        resource: "Import".into(),
        operation_id: id.clone(),
        format: "doc".into(),
        legacy_backend: Some("libreoffice".into()),
        docx_options: None,
        conversion_layout: None,
    };
    retain_claim(temp.path(), &root, original.clone()).unwrap();
    assert_eq!(
        read_claim(temp.path(), &root, &id)
            .unwrap()
            .unwrap()
            .fingerprint,
        "exact"
    );
    let mut different = original;
    different.fingerprint = "different".into();
    assert!(retain_claim(temp.path(), &root, different).is_err());
    fs::write(claim_path(&root, &id), b"partial").unwrap();
    assert_eq!(
        read_claim(temp.path(), &root, &id).unwrap_err().code,
        "resultUnknown"
    );
}

fn enable_legacy_import(state: &HostedState, temp: &Path) -> PathBuf {
    let mut inner = state.inner.lock().unwrap();
    inner.office = office::Store::with_legacy_root(temp.join("legacy-office"));
    inner
        .policy
        .users
        .get_mut("alice")
        .unwrap()
        .grants
        .get_mut("Import")
        .unwrap()
        .office
        .doc
        .r#import = true;
    legacy_root(&inner, "alice").unwrap()
}

#[test]
fn conversion_layout_keeps_original_claims_and_namespaces_new_physical_runs() {
    let (temp, state, _) = fixture();
    let old_root = enable_legacy_import(&state, temp.path());
    let inner = state.inner.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let old = conversion_location(&inner, "alice", &id, 3, None).unwrap();
    assert_eq!(old.root, old_root);
    assert_eq!(old.request_id, id);
    let flat = conversion_location(&inner, "alice", &id, 3, Some("flatV1")).unwrap();
    assert_eq!(
        flat.root,
        inner.office.legacy_root.as_ref().unwrap().join("runs")
    );
    assert_ne!(flat.request_id, id);
    assert_eq!(
        flat.request_id,
        conversion_location(&inner, "alice", &id, 3, Some("flatV1"))
            .unwrap()
            .request_id
    );
    assert_ne!(
        flat.request_id,
        conversion_location(&inner, "alice", &id, 4, Some("flatV1"))
            .unwrap()
            .request_id
    );
    assert_ne!(
        flat.request_id,
        conversion_location(&inner, "bob", &id, 3, Some("flatV1"))
            .unwrap()
            .request_id
    );
    assert!(conversion_location(&inner, "alice", &id, 3, Some("future")).is_err());
}

fn frozen_claim(inner: &HostedInner, req: &Prepare, layout: Option<&str>) -> ImportClaim {
    let file = inner.file(&req.resource).unwrap();
    let bytes = upload(&req.bytes_base64).unwrap();
    ImportClaim {
        schema_version: 1,
        fingerprint: fingerprint(req, &digest(&bytes)).unwrap(),
        decision: Decision {
            actor_epoch: inner.actor_generation("alice"),
            expires_at_unix_ms: now_ms() + 100000,
            config_hash: config_hash(inner, &file).unwrap(),
            upload_digest: digest(&bytes),
            format: req.format.clone(),
        },
        resource: req.resource.clone(),
        operation_id: req.operation_id.clone(),
        format: req.format.clone(),
        legacy_backend: req.legacy_backend.clone(),
        docx_options: req.docx_options.clone(),
        conversion_layout: layout.map(str::to_owned),
    }
}

#[tokio::test]
async fn retained_import_claim_with_missing_wrapper_only_recovers_and_never_starts_again() {
    let (temp, state, auth) = fixture();
    let root = enable_legacy_import(&state, temp.path());
    for layout in [None, Some("flatV1")] {
        let mut req = request(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]);
        req.format = "doc".into();
        // A regression must not invoke external software even if the
        // read-only recovery check is accidentally removed.
        req.legacy_backend = Some("invalid-test-backend".into());
        let location = {
            let inner = state.inner.lock().unwrap();
            let base = inner.office.legacy_root.as_deref().unwrap();
            let claim = frozen_claim(&inner, &req, layout);
            let location = conversion_location(
                &inner,
                "alice",
                &req.operation_id,
                claim.decision.actor_epoch,
                layout,
            )
            .unwrap();
            retain_claim(base, &root, claim).unwrap();
            office::ensure_owned(base, &location.root, true).unwrap();
            location
        };
        let error = prepare(State(state.clone()), Extension(auth.clone()), Json(req))
            .await
            .unwrap_err();
        assert_eq!(error.code, "resultUnknown");
        assert!(!location.root.join(location.request_id).exists());
        assert!(state.inner.lock().unwrap().office.import_active.is_empty());
    }
}

#[tokio::test]
async fn import_claim_and_flat_execution_reject_ancestor_junctions_without_outside_effects() {
    for redirect_claim in [true, false] {
        let (temp, state, auth) = fixture();
        let root = enable_legacy_import(&state, temp.path());
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        let mut req = request(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]);
        req.format = "doc".into();
        req.legacy_backend = Some("invalid-test-backend".into());
        let base = state
            .inner
            .lock()
            .unwrap()
            .office
            .legacy_root
            .clone()
            .unwrap();
        fs::create_dir_all(&base).unwrap();
        let junction = if redirect_claim {
            root.parent().unwrap().to_owned()
        } else {
            base.join("runs")
        };
        let status = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x0800_0000)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(
            prepare(
                State(state.clone()),
                Extension(auth.clone()),
                Json(req.clone())
            )
            .await
            .is_err()
        );
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        assert!(state.inner.lock().unwrap().office.import_active.is_empty());
        // The flat-root case failed only after retaining a valid claim. Make
        // that claim unreadable to exercise the same unknown-claim fallback
        // as the redirected empty claims ancestor, without testing a known
        // claim's separately authorized explicit termination semantics.
        if !redirect_claim {
            fs::write(claim_path(&root, &req.operation_id), b"partial claim").unwrap();
        }
        let error = settle(
            State(state.clone()),
            Extension(auth),
            Json(Operation {
                resource: req.resource.clone(),
                operation_id: req.operation_id.clone(),
                format: Some("doc".into()),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "resultUnknown");
        {
            let mut inner = state.inner.lock().unwrap();
            let actor_scope = scope(&inner, "alice");
            let library = inner.config.root.clone();
            assert!(
                inner
                    .core
                    .with_actor_for_root(&actor_scope, &library, |core| core
                        .actor_feature_status(&req.resource, &req.operation_id))
                    .unwrap()
                    .is_none()
            );
        }
        if redirect_claim {
            assert!(read_claim(&base, &root, &req.operation_id).is_err());
            let inner = state.inner.lock().unwrap();
            let claim = frozen_claim(&inner, &req, Some("flatV1"));
            let bytes = serde_json::to_vec(&claim).unwrap();
            let redirected = outside.join("imports/claims");
            fs::create_dir_all(&redirected).unwrap();
            fs::write(
                redirected.join(format!("{}.json", req.operation_id)),
                &bytes,
            )
            .unwrap();
            fs::write(
                redirected.join(format!("{}.sha256", req.operation_id)),
                digest(&bytes),
            )
            .unwrap();
            assert!(read_claim(&base, &root, &req.operation_id).is_err());
        }
    }
}

#[tokio::test]
async fn dropped_blocking_registration_result_cancels_and_removes_only_its_owned_import() {
    let (_temp, state, _) = fixture();
    let id = uuid::Uuid::new_v4().to_string();
    let other = uuid::Uuid::new_v4().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    let other_cancel = Arc::new(AtomicBool::new(false));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let task_state = state.clone();
    let guard_state = state.clone();
    let task_id = id.clone();
    let flag = cancel.clone();
    {
        let mut inner = state.inner.lock().unwrap();
        inner.office.import_active.insert(
            ("alice".into(), other.clone()),
            office::LegacyImportRun {
                actor_epoch: 1,
                cancel: other_cancel.clone(),
            },
        );
    }
    let task = tokio::spawn(async move {
        locked(&task_state, move |inner| {
            let key = ("alice".into(), task_id);
            let epoch = inner.actor_generation("alice");
            inner.office.import_active.insert(
                key.clone(),
                office::LegacyImportRun {
                    actor_epoch: epoch,
                    cancel: flag.clone(),
                },
            );
            let guard = ImportGuard {
                state: guard_state,
                key,
                cancel: flag,
            };
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(guard)
        })
        .await
    });
    entered_rx.await.unwrap();
    task.abort();
    release_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if cancel.load(Ordering::Acquire)
                && !state
                    .inner
                    .lock()
                    .unwrap()
                    .office
                    .import_active
                    .contains_key(&("alice".into(), id.clone()))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        state
            .inner
            .lock()
            .unwrap()
            .office
            .import_active
            .contains_key(&("alice".into(), other))
    );
    assert!(!other_cancel.load(Ordering::Acquire));
}

#[tokio::test]
async fn legacy_claim_capacity_and_uuid_rejections_do_not_write_new_recovery_files() {
    let (temp, state, auth) = fixture();
    let root = enable_legacy_import(&state, temp.path());
    {
        let mut inner = state.inner.lock().unwrap();
        let epoch = inner.actor_generation("alice");
        for _ in 0..8 {
            inner.office.import_active.insert(
                ("alice".into(), uuid::Uuid::new_v4().to_string()),
                office::LegacyImportRun {
                    actor_epoch: epoch,
                    cancel: Arc::new(AtomicBool::new(false)),
                },
            );
        }
    }
    let mut req = request(b"original DOC upload");
    req.format = "doc".into();
    let id = req.operation_id.clone();
    assert!(
        prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(req.clone())
        )
        .await
        .is_err()
    );
    assert!(!claim_path(&root, &id).exists());
    assert!(!root.exists());
    state.inner.lock().unwrap().office.import_active.clear();
    req.operation_id = uuid::Uuid::nil().to_string();
    assert!(
        prepare(State(state), Extension(auth), Json(req))
            .await
            .is_err()
    );
    assert!(!root.exists());
}

#[tokio::test]
async fn damaged_unstarted_claim_can_be_fenced_but_external_evidence_stays_unknown() {
    let (temp, state, auth) = fixture();
    let root = enable_legacy_import(&state, temp.path());
    fs::create_dir_all(root.join("claims")).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let path = claim_path(&root, &id);
    fs::write(&path, b"partial before external startup").unwrap();
    let Json(ended) = settle(
        State(state.clone()),
        Extension(auth.clone()),
        Json(Operation {
            resource: "Import".into(),
            operation_id: id.clone(),
            format: Some("doc".into()),
        }),
    )
    .await
    .unwrap();
    assert_eq!(ended["status"], "cancelled");
    assert_eq!(fs::read(&path).unwrap(), b"partial before external startup");
    assert!(!root.join(&id).exists());
    let Json(recovered) = operation(
        State(state.clone()),
        Extension(auth.clone()),
        Json(Operation {
            resource: "Import".into(),
            operation_id: id,
            format: None,
        }),
    )
    .await
    .unwrap();
    assert_eq!(recovered["status"], "cancelled");

    let uncertain = uuid::Uuid::new_v4().to_string();
    fs::write(claim_path(&root, &uncertain), b"partial claim").unwrap();
    fs::create_dir(root.join(&uncertain)).unwrap();
    let error = settle(
        State(state.clone()),
        Extension(auth),
        Json(Operation {
            resource: "Import".into(),
            operation_id: uncertain.clone(),
            format: Some("doc".into()),
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, "resultUnknown");
    let mut inner = state.inner.lock().unwrap();
    let actor_scope = scope(&inner, "alice");
    let library = inner.config.root.clone();
    assert!(
        inner
            .core
            .with_actor_for_root(&actor_scope, &library, |core| core
                .actor_feature_status("Import", &uncertain))
            .unwrap()
            .is_none()
    );
}
