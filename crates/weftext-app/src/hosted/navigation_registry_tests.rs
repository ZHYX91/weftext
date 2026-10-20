use super::*;

#[tokio::test]
async fn atomic_tag_assignment_uses_real_narrow_basis_and_portable_configured_path_after_restart() {
    let (temp, state, auth, _doc, _version, _original) = history_api::test_fixture();
    let original_source = {
        let mut inner = state.inner.lock().unwrap();
        let sessions = inner.sessions.keys().cloned().collect::<Vec<_>>();
        for session in sessions {
            inner.release_session(&session).unwrap();
        }
        let root = inner.config.root.clone();
        let before = inner
            .core
            .read_config(&root, weftext_core::ConfigRole::Schema)
            .unwrap();
        let schema = inner
            .core
            .prepare_config_save(
                &root,
                weftext_core::ConfigRole::Schema,
                before.bytes,
                include_bytes!("../../../../docs/design/examples/schema.authoring.json").to_vec(),
                &uuid::Uuid::new_v4().to_string(),
            )
            .unwrap();
        inner
            .core
            .execute_operation(&schema.operation_id, &schema.plan_id)
            .unwrap();
        let opened = inner
            .core
            .open(root.join("Shared").join("Shared.typ"), Some(root.clone()))
            .unwrap();
        let source = opened.source.replace(
            "types: ()",
            "types: (\"task\",), status: \"open\", privateNote: \"retained private field\"",
        ) + "\nPrivate body stays intact.\n";
        inner
            .core
            .save(weftext_core::SaveRequest {
                session_id: opened.session_id.clone(),
                base_id: opened.base_id,
                source: source.clone(),
                operation_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        inner.core.close_session(&opened.session_id).unwrap();
        allow(&mut inner, "alice");
        let grant = inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap();
        grant.read = false;
        grant.write = false;
        grant.history = false;
        grant.fields.insert(
            "core/tags".into(),
            FieldRights {
                read: true,
                write: true,
            },
        );
        // Operator configuration deliberately uses portable separators. This
        // must retain the exact source identity without requiring its spelling.
        assert_eq!(inner.config.files["Shared"].path, "Shared/Shared.typ");
        source
    };
    let field =
        fields::test_read_assignment_field(state.clone(), auth.clone(), "Shared", "core/tags")
            .await
            .unwrap()
            .0;
    assert!(field["basis"].is_string(), "{field:?}");
    let request = {
        let mut inner = state.inner.lock().unwrap();
        let (registry, bytes) = tag_cut(&mut inner).unwrap();
        let basis = retain(&mut inner, "alice", registry, bytes).unwrap();
        Prepare {
            operation_id: uuid::Uuid::new_v4().to_string(),
            intent: Intent::CreateTag {
                basis,
                display_name: "Created and assigned once".into(),
                parent_ref: None,
                assign_to: Some(
                    serde_json::from_value(
                        json!({"resource":"Shared","fieldId":"core/tags","basis":field["basis"]}),
                    )
                    .unwrap(),
                ),
            },
        }
    };
    let prepared = prepare(
        State(state.clone()),
        Extension(auth.clone()),
        Json(request.clone()),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(prepared["commitAllowed"], true, "{prepared:?}");
    {
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        assert_eq!(
            std::fs::read_to_string(root.join("Shared/Shared.typ")).unwrap(),
            original_source
        );
        assert!(!root.join(".weftext-tags.json").exists());
        let serialized = serde_json::to_string(&prepared).unwrap();
        assert!(
            !serialized.contains("Private body") && !serialized.contains("retained private field")
        );
        assert!(!serialized.contains(root.to_str().unwrap()));
        let placeholder = Core::new(temp.path().join("restart-placeholder")).unwrap();
        let previous = std::mem::replace(&mut inner.core, placeholder);
        drop(previous);
        inner.core = Core::new(temp.path().join("state")).unwrap();
        inner.fields = Default::default();
        inner.navigation_registry = Default::default();
    }
    let committed = commit(
        State(state.clone()),
        Extension(auth.clone()),
        Json(Commit {
            operation_id: request.operation_id.clone(),
            plan_id: prepared["planId"].as_str().unwrap().into(),
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(committed["status"], "committed", "{committed:?}");
    let retried = prepare(State(state.clone()), Extension(auth.clone()), Json(request))
        .await
        .unwrap()
        .0;
    assert_eq!(retried, committed);
    let mut inner = state.inner.lock().unwrap();
    let source = std::fs::read_to_string(inner.config.root.join("Shared/Shared.typ")).unwrap();
    assert!(source.contains(committed["navigationRef"].as_str().unwrap()));
    assert!(
        source.contains("retained private field") && source.contains("Private body stays intact.")
    );
    let root = inner.config.root.clone();
    assert_eq!(inner.core.read_tag_registry(&root).unwrap().tags.len(), 1);
    assert!(!inner.policy.users["alice"].grants["Shared"].read);
}

#[test]
fn explicit_empty_path_admits_library_root_and_root_read_does_not_grant_source_or_query_scope() {
    let (_temp, state, _, _, _, _) = history_api::test_fixture();
    let mut inner = state.inner.lock().unwrap();
    inner.config.navigation.directory_admissions.insert(
        "RootAdmission".into(),
        Admission {
            path: String::new(),
            title: "Library root".into(),
        },
    );
    inner
        .policy
        .users
        .get_mut("alice")
        .unwrap()
        .navigation
        .admit_directories
        .insert("RootAdmission".into());
    assert!(valid_config(&inner.config));
    assert_eq!(
        directory_path(&inner, "").to_string_lossy(),
        inner.config.root.to_string_lossy()
    );
    let operation = uuid::Uuid::new_v4().to_string();
    let prepared = build(
        &mut inner,
        "alice",
        Prepare {
            operation_id: operation.clone(),
            intent: Intent::AdmitDirectory {
                slot: "RootAdmission".into(),
            },
        },
    )
    .unwrap();
    let actor_scope = scope(&inner, "alice");
    let committed = inner
        .core
        .with_actor(&actor_scope, |c| {
            c.actor_feature_commit(RESOURCE, &operation, prepared["planId"].as_str().unwrap())
        })
        .unwrap();
    let d = decision(&inner, "alice", &committed).unwrap();
    assert_eq!(
        receipt_target(&mut inner, "alice", &committed, &d).unwrap()["title"],
        "Library root"
    );
    inner.config.directories.insert(
        "RootDirectory".into(),
        query_scopes::Directory {
            path: String::new(),
            title: "Root read".into(),
            reference: committed.navigation_ref.unwrap(),
            lifecycle: "live".into(),
        },
    );
    let bob = inner.policy.users.get_mut("bob").unwrap();
    bob.grants.clear();
    bob.navigation.directories.insert("RootDirectory".into());
    assert_eq!(
        configured_directory(&inner, "bob", "RootDirectory").unwrap()["title"],
        "Root read"
    );
    assert!(
        inner.policy.users["bob"]
            .query_scopes
            .directories
            .is_empty()
    );
    let mut plan = serde_json::from_value::<weftext_data::QueryPlan>(
        json!({"scope":{"directory":"RootDirectory"}}),
    )
    .unwrap();
    assert!(query_scopes::canonicalize(&inner, "bob", &mut plan).is_err());
    assert!(inner.open("bob", "Shared").is_err());
    inner
        .config
        .navigation
        .directory_admissions
        .get_mut("RootAdmission")
        .unwrap()
        .path = "..".into();
    assert!(!valid_config(&inner.config));
}

fn allow(inner: &mut HostedInner, actor: &str) {
    inner.config.navigation.directory_admissions.insert(
        "PublicAdmission".into(),
        Admission {
            path: "PublicContainer".into(),
            title: "Visible container".into(),
        },
    );
    inner.policy.users.get_mut(actor).unwrap().navigation = Rights {
        directories: BTreeSet::new(),
        admit_directories: BTreeSet::from(["PublicAdmission".into()]),
        tags: TagRights {
            read: true,
            create: true,
            rename: true,
            move_tag: false,
        },
    };
}
fn admit(operation: &str) -> Prepare {
    Prepare {
        operation_id: operation.into(),
        intent: Intent::AdmitDirectory {
            slot: "PublicAdmission".into(),
        },
    }
}
fn original(inner: &mut HostedInner, actor: &str, operation: &str) -> ActorFeatureStatus {
    let actor_scope = scope(inner, actor);
    inner
        .core
        .with_actor(&actor_scope, |c| {
            c.actor_feature_status(RESOURCE, operation)
        })
        .unwrap()
        .unwrap()
}

#[test]
fn tag_read_is_required_for_writing_and_directory_grants_do_not_grant_query_scopes_or_tags() {
    let (_temp, state, _, _, _, _) = history_api::test_fixture();
    let mut inner = state.inner.lock().unwrap();
    let invalid = Rights {
        tags: TagRights {
            create: true,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!valid_rights(&inner.config, &invalid));
    // A denied role is not read even when its physical file is malformed.
    std::fs::write(
        inner.config.root.join(".weftext-tags.json"),
        b"not-a-registry",
    )
    .unwrap();
    assert!(tag_target(&mut inner, "alice", "weftext-tag://unknown/unknown").is_err());
    let owner = inner
        .core
        .qualified_node_at(
            &inner.config.root,
            &inner.config.root.join("Shared/Shared.typ"),
        )
        .unwrap()
        .owner;
    inner.config.directories.insert(
        "VisibleFolder".into(),
        query_scopes::Directory {
            path: "Shared".into(),
            title: "Shared directory".into(),
            reference: format!("weftext-node://{}/{}", owner.workspace_id, owner.node_id),
            lifecycle: "live".into(),
        },
    );
    inner
        .policy
        .users
        .get_mut("alice")
        .unwrap()
        .query_scopes
        .directories
        .insert("VisibleFolder".into());
    assert!(configured_directory(&inner, "alice", "VisibleFolder").is_err());
    inner
        .policy
        .users
        .get_mut("alice")
        .unwrap()
        .query_scopes
        .directories
        .clear();
    inner
        .policy
        .users
        .get_mut("alice")
        .unwrap()
        .navigation
        .directories
        .insert("VisibleFolder".into());
    assert_eq!(
        configured_directory(&inner, "alice", "VisibleFolder").unwrap()["title"],
        "Shared directory"
    );
    assert!(
        inner.policy.users["alice"]
            .query_scopes
            .directories
            .is_empty()
    );
}

#[test]
fn directory_receipt_is_actor_scoped_and_configured_slot_identity_never_follows_a_namesake() {
    let (_temp, state, _, _, _, _) = history_api::test_fixture();
    let mut inner = state.inner.lock().unwrap();
    std::fs::create_dir(inner.config.root.join("PublicContainer")).unwrap();
    allow(&mut inner, "alice");
    allow(&mut inner, "bob");
    let operation = uuid::Uuid::new_v4().to_string();
    let prepared = build(&mut inner, "alice", admit(&operation)).unwrap();
    assert_eq!(prepared["commitAllowed"], true);
    let plan = prepared["planId"].as_str().unwrap();
    let actor_scope = scope(&inner, "alice");
    let committed = inner
        .core
        .with_actor(&actor_scope, |c| {
            c.actor_feature_commit(RESOURCE, &operation, plan)
        })
        .unwrap();
    assert_eq!(
        committed.navigation_ref.as_deref(),
        prepared["navigationRef"].as_str()
    );
    let bob_scope = scope(&inner, "bob");
    assert!(
        inner
            .core
            .with_actor(&bob_scope, |c| c.actor_feature_status(RESOURCE, &operation))
            .unwrap()
            .is_none()
    );
    let d = decision(&inner, "alice", &committed).unwrap();
    assert_eq!(
        receipt_target(&mut inner, "alice", &committed, &d).unwrap()["canResolve"],
        true
    );
    let root = inner.config.root.clone();
    std::fs::rename(root.join("PublicContainer"), root.join("OriginalContainer")).unwrap();
    std::fs::create_dir(root.join("PublicContainer")).unwrap();
    std::fs::copy(
        root.join("OriginalContainer/weftext.directory.json"),
        root.join("PublicContainer/weftext.directory.json"),
    )
    .unwrap();
    assert!(receipt_target(&mut inner, "alice", &committed, &d).is_err());
    let safe = safe_status(&mut inner, "alice", committed).unwrap();
    assert!(safe["result"].is_null());
    assert!(
        !serde_json::to_string(&safe)
            .unwrap()
            .contains(root.to_str().unwrap())
    );
}

#[test]
fn epoch_config_binding_and_unknown_fence_do_not_regenerate_original_directory_allocation() {
    let (_temp, state, _, _, _, _) = history_api::test_fixture();
    let mut inner = state.inner.lock().unwrap();
    std::fs::create_dir(inner.config.root.join("PublicContainer")).unwrap();
    allow(&mut inner, "alice");
    let operation = uuid::Uuid::new_v4().to_string();
    let first = build(&mut inner, "alice", admit(&operation)).unwrap();
    assert_eq!(
        build(&mut inner, "alice", admit(&operation)).unwrap(),
        first
    );
    inner.policy.users.get_mut("alice").unwrap().epoch += 1;
    let old = original(&mut inner, "alice", &operation);
    let hidden = safe_status(&mut inner, "alice", old).unwrap();
    assert_eq!(hidden["status"], "prepared");
    assert_eq!(hidden["commitAllowed"], false);
    assert!(hidden["planId"].is_null() && hidden["navigationRef"].is_null());
    inner
        .config
        .navigation
        .directory_admissions
        .get_mut("PublicAdmission")
        .unwrap()
        .path = "ReboundContainer".into();
    let old = original(&mut inner, "alice", &operation);
    assert!(decision(&inner, "alice", &old).is_none());
    let unknown = uuid::Uuid::new_v4().to_string();
    let actor_scope = scope(&inner, "alice");
    inner
        .core
        .with_actor(&actor_scope, |c| c.actor_feature_settle(RESOURCE, &unknown))
        .unwrap();
    let fenced = build(&mut inner, "alice", admit(&unknown)).unwrap();
    assert_eq!(fenced["status"], "cancelled");
    assert!(fenced["navigationRef"].is_null());
}

#[test]
fn tag_basis_is_exact_actor_epoch_and_registry_object_and_retry_never_reallocates() {
    let (_temp, state, _, _, _, _) = history_api::test_fixture();
    let mut inner = state.inner.lock().unwrap();
    allow(&mut inner, "alice");
    allow(&mut inner, "bob");
    let (registry, bytes) = tag_cut(&mut inner).unwrap();
    let basis = retain(&mut inner, "alice", registry, bytes).unwrap();
    let operation = uuid::Uuid::new_v4().to_string();
    let req = Prepare {
        operation_id: operation.clone(),
        intent: Intent::CreateTag {
            parent_ref: None,
            assign_to: None,
            basis,
            display_name: "Same name".into(),
        },
    };
    assert!(build(&mut inner, "bob", req.clone()).is_err());
    let first = build(&mut inner, "alice", req.clone()).unwrap();
    assert_eq!(build(&mut inner, "alice", req.clone()).unwrap(), first);
    let actor_scope = scope(&inner, "alice");
    let committed = inner
        .core
        .with_actor(&actor_scope, |c| {
            c.actor_feature_commit(RESOURCE, &operation, first["planId"].as_str().unwrap())
        })
        .unwrap();
    assert!(
        tag_target(
            &mut inner,
            "alice",
            committed.navigation_ref.as_deref().unwrap()
        )
        .is_ok()
    );
    let (stale, bytes) = tag_cut(&mut inner).unwrap();
    let stale_basis = retain(&mut inner, "alice", stale, bytes).unwrap();
    let path = inner.config.root.join(".weftext-tags.json");
    let raw = std::fs::read(&path).unwrap();
    std::fs::rename(&path, path.with_extension("previous")).unwrap();
    std::fs::write(&path, raw).unwrap();
    let second = Prepare {
        operation_id: uuid::Uuid::new_v4().to_string(),
        intent: Intent::CreateTag {
            parent_ref: None,
            assign_to: None,
            basis: stale_basis,
            display_name: "Copy must not bind".into(),
        },
    };
    assert!(build(&mut inner, "alice", second).is_err());
    assert!(
        tag_target(
            &mut inner,
            "alice",
            committed.navigation_ref.as_deref().unwrap()
        )
        .is_err()
    );
    let replay = build(&mut inner, "alice", req).unwrap();
    assert_eq!(replay["status"], "committed");
    assert_eq!(replay["navigationRef"], first["navigationRef"]);
    assert!(replay["result"].is_null());
}
