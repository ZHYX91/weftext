use super::*;

struct Fixture {
    core: Core,
    _temp: tempfile::TempDir,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        Self {
            core,
            _temp: temp,
            root,
        }
    }
    fn node(&mut self, parent: &Path, name: &str) -> OpenedDocument {
        self.core
            .create_note_fixture(&self.root, parent, name)
            .unwrap()
    }
    fn owner(&self, doc: &OpenedDocument) -> weftext_knowledge::NodeRef {
        weftext_knowledge::NodeRef {
            workspace_id: self
                .core
                .require_workspace(&self.root)
                .unwrap()
                .workspace_id,
            node_id: doc.node_id.clone().unwrap(),
        }
    }
    fn reference(&self, doc: &OpenedDocument) -> String {
        let o = self.owner(doc);
        format!("weftext-node://{}/{}", o.workspace_id, o.node_id)
    }
    fn directory(&mut self, path: &Path) -> String {
        let op = fresh();
        let p = self
            .core
            .prepare_directory_admission(&self.root, path, &op)
            .unwrap();
        self.core
            .execute_operation(&op, &p.preview.plan_id)
            .unwrap();
        p.target_ref
    }
    fn prepare(&mut self, intent: OperationIntent) -> OperationPreview {
        self.core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&self.root),
                intent,
            })
            .unwrap()
    }
    fn execute(&mut self, preview: &OperationPreview) {
        self.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    }
}
#[test]
fn query_scope_exact_admitted_root_node_and_directory_without_ambient_reads() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let node = f.node(&root, "Shared");
    let reference = f.reference(&node);
    let owner = f.owner(&node);
    let secret = f.node(&root, "Private");
    let guard = ParentGuard::acquire(
        &crate::BackendContext::local().unwrap(),
        Path::new(&secret.path),
    )
    .unwrap();
    let _unreadable = guard.open_file(false, false).unwrap();
    let root_ref = f.directory(&root);
    let folder = root.join("Ordinary");
    fs::create_dir(&folder).unwrap();
    assert!(
        f.core
            .qualified_query_scope_at(&root, &folder, "weftext-directory://invalid/id", "live")
            .is_err()
    );
    let dir_ref = f.directory(&folder);
    let scope = f
        .core
        .qualified_query_scope_at(&root, &root.join("Shared"), &reference, "live")
        .unwrap();
    let leaf = f
        .core
        .qualified_query_node_at(&root, Path::new(&node.path), &owner, "live")
        .unwrap();
    assert!(leaf.coordinate.starts_with(&scope.directory));
    assert!(scope.witness.retained_bytes() >= node.source.len());
    f.core
        .validate_query_scope_witness(&root, &scope.witness)
        .unwrap();
    f.core
        .validate_query_node_witness(&root, &leaf.witness)
        .unwrap();
    f.core
        .qualified_query_scope_at(&root, &root, &root_ref, "live")
        .unwrap();
    f.core
        .qualified_query_scope_at(&root, &folder, &dir_ref, "live")
        .unwrap();
    assert!(
        f.core
            .qualified_query_scope_at(
                &root,
                &root,
                &format!("weftext-directory://{}/{}", owner.workspace_id, fresh()),
                "live"
            )
            .is_err()
    );
}
#[test]
fn query_scope_handle_coordinates_preserve_non_ascii_and_mixed_case_slashes() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let node = f.node(&root, "Shared中文😀");
    let reference = f.reference(&node);
    let owner = f.owner(&node);
    let alias = root.join("sHaReD中文😀");
    let scope = f
        .core
        .qualified_query_scope_at(&root, &alias, &reference, "live")
        .unwrap();
    let source = PathBuf::from(display(&alias.join("Shared中文😀.typ")).replace('\\', "/"));
    let leaf = f
        .core
        .qualified_query_node_at(&root, &source, &owner, "live")
        .unwrap();
    assert!(leaf.coordinate.starts_with(&scope.directory));
    assert!(display(&scope.directory).ends_with("Shared中文😀"));
    // The writer alias cannot make Core double-open its share-zero source.
    let mut pool = vec![];
    Core::merge_query_scope_witness(&mut pool, &scope.witness).unwrap();
    let refs = pool.iter().collect::<Vec<_>>();
    f.core
        .with_guarded_snapshot_witnesses(&root, &refs, Path::new(&node.path), |core| {
            core.require_workspace(&root)?;
            core.save(SaveRequest {
                session_id: node.session_id.clone(),
                base_id: node.base_id.clone(),
                source: node.source.clone() + "\nSaved\n",
                operation_id: fresh(),
            })
        })
        .unwrap();
}
#[test]
fn query_scope_rejects_copied_marker_replaced_source_and_changed_declaration() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let n = f.node(&root, "Note");
    let owner = f.owner(&n);
    let reference = f.reference(&n);
    let dir = root.join("Original");
    fs::create_dir(&dir).unwrap();
    let dir_ref = f.directory(&dir);
    let copy = root.join("Copy");
    fs::create_dir(&copy).unwrap();
    fs::copy(
        dir.join("weftext.directory.json"),
        copy.join("weftext.directory.json"),
    )
    .unwrap();
    assert_eq!(
        f.core
            .qualified_query_scope_at(&root, &copy, &dir_ref, "live")
            .err()
            .unwrap()
            .code,
        "identityPending"
    );
    let witness = f
        .core
        .qualified_query_scope_at(&root, &root.join("Note"), &reference, "live")
        .unwrap()
        .witness;
    fs::rename(&n.path, root.join("retained.typ")).unwrap();
    fs::write(&n.path, &n.source).unwrap();
    assert!(
        f.core
            .qualified_query_node_at(&root, Path::new(&n.path), &owner, "live")
            .is_err()
    );
    assert!(
        f.core
            .validate_query_scope_witness(&root, &witness)
            .is_err()
    );
    fs::remove_file(&n.path).unwrap();
    fs::rename(root.join("retained.typ"), &n.path).unwrap();
    fs::write(&n.path, n.source.replace(&owner.node_id, &fresh())).unwrap();
    assert!(
        f.core
            .qualified_query_node_at(&root, Path::new(&n.path), &owner, "live")
            .is_err()
    );
}
#[test]
fn query_scope_parent_move_proves_members_without_freezing_late_contents_or_children() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let parent = f.node(&root, "A");
    let child = f.node(&root.join("A"), "Child");
    let owner = f.owner(&child);
    let reference = f.reference(&parent);
    let folder = root.join("A").join("Folder");
    fs::create_dir(&folder).unwrap();
    let dir_ref = f.directory(&folder);
    let move_plan = f.prepare(OperationIntent::RenameNode {
        source: display(&root.join("A")),
        name: "C".into(),
    });
    fs::write(&child.path, child.source.clone() + "\nlate body change\n").unwrap();
    fs::create_dir(root.join("A/Late")).unwrap();
    fs::write(root.join("A/Late/raw.txt"), "late child").unwrap();
    f.execute(&move_plan);
    let now = root.join("C/Child/Child.typ");
    f.core
        .qualified_query_node_at(&root, &now, &owner, "live")
        .unwrap();
    f.core
        .qualified_query_scope_at(&root, &root.join("C"), &reference, "live")
        .unwrap();
    f.core
        .qualified_query_scope_at(&root, &root.join("C/Folder"), &dir_ref, "live")
        .unwrap();
    assert!(root.join("C/Late/raw.txt").exists());
    assert!(
        fs::read_to_string(now)
            .unwrap()
            .contains("late body change")
    );
}
#[test]
fn query_scope_unrelated_reoccupant_ancestor_move_cannot_lend_child_placement() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let _a = f.node(&root, "A");
    let child = f.node(&root.join("A"), "Child");
    let owner = f.owner(&child);
    let folder = root.join("A/Folder");
    fs::create_dir(&folder).unwrap();
    let dir_ref = f.directory(&folder);
    fs::rename(root.join("A"), root.join("Held")).unwrap();
    let _replacement = f.node(&root, "A");
    let p = f.prepare(OperationIntent::RenameNode {
        source: display(&root.join("A")),
        name: "C".into(),
    });
    f.execute(&p);
    fs::rename(root.join("Held/Child"), root.join("C/Child")).unwrap();
    fs::rename(root.join("Held/Folder"), root.join("C/Folder")).unwrap();
    assert_eq!(
        f.core
            .qualified_query_scope_at(&root, &root.join("C/Folder"), &dir_ref, "live")
            .err()
            .unwrap()
            .code,
        "identityPending"
    );
    assert_eq!(
        f.core
            .qualified_query_node_at(&root, &root.join("C/Child/Child.typ"), &owner, "live")
            .err()
            .unwrap()
            .code,
        "identityPending"
    );
}
#[test]
fn query_scope_trash_keeps_original_main_leaf_and_lifecycle_and_restores_exact_members() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let parent = f.node(&root, "A");
    let child = f.node(&root.join("A"), "Child");
    let parent_owner = f.owner(&parent);
    let child_owner = f.owner(&child);
    let reference = f.reference(&parent);
    let p = f.prepare(OperationIntent::TrashNode {
        source: display(&root.join("A")),
    });
    f.execute(&p);
    let tree = root.join(".weftext-trash").join(&p.operation_id);
    let main = tree.join("A.typ");
    let scope = f
        .core
        .qualified_query_scope_at(&root, &tree, &reference, "trash")
        .unwrap();
    let leaf = f
        .core
        .qualified_query_node_at(&root, &main, &parent_owner, "trash")
        .unwrap();
    assert!(leaf.coordinate.starts_with(&scope.directory));
    f.core
        .qualified_query_node_at(&root, &tree.join("Child/Child.typ"), &child_owner, "trash")
        .unwrap();
    let alias = root
        .join(".WEFTEXT-TRASH")
        .join(&p.operation_id)
        .join("A.typ");
    let root_alias = root.with_file_name("lIbRaRy");
    let root_alias_node = f
        .core
        .qualified_query_node_at(&root_alias, &alias, &parent_owner, "trash")
        .unwrap();
    f.core
        .validate_query_node_witness(&root_alias, &root_alias_node.witness)
        .unwrap();
    let trash_doc = f.core.open(&alias, Some(root.clone())).unwrap();
    assert_eq!(
        f.core
            .save(SaveRequest {
                session_id: trash_doc.session_id,
                base_id: trash_doc.base_id,
                source: trash_doc.source + "\nforbidden write\n",
                operation_id: fresh()
            })
            .unwrap_err()
            .code,
        "trashReadOnly"
    );
    assert_eq!(
        f.core
            .qualified_query_node_at(&root, &main, &parent_owner, "live")
            .err()
            .unwrap()
            .code,
        "lifecycleMismatch"
    );
    assert!(
        f.core
            .qualified_query_node_at(
                &root,
                &tree.join(format!("{}.typ", p.operation_id)),
                &parent_owner,
                "trash"
            )
            .is_err()
    );
    let restore = f.prepare(OperationIntent::RestoreNode {
        trash_operation_id: p.operation_id,
        node_id: None,
        parent: display(&root),
        name: "Back".into(),
    });
    f.execute(&restore);
    f.core
        .qualified_query_node_at(
            &root,
            &root.join("Back/Child/Child.typ"),
            &child_owner,
            "live",
        )
        .unwrap();
    assert!(
        f.core
            .validate_query_scope_witness(&root, &scope.witness)
            .is_err()
    );
}
#[test]
fn query_scope_rules_and_parent_are_held_through_save_with_controls_reentrant() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let node = f.node(&root, "Owner");
    let owner = f.owner(&node);
    let q = f
        .core
        .qualified_query_node_at(&root, Path::new(&node.path), &owner, "live")
        .unwrap();
    let mut pool = vec![];
    Core::merge_query_node_witness(&mut pool, &q.witness).unwrap();
    assert!(
        pool.iter()
            .map(SnapshotWitness::retained_bytes)
            .sum::<usize>()
            > node.source.len()
    );
    f.core
        .with_guarded_snapshot_witnesses(
            &root,
            &pool.iter().collect::<Vec<_>>(),
            Path::new(&node.path),
            |core| {
                core.require_workspace(&root)?;
                core.check_discovery_boundary(&root, &root.join("Owner"))?;
                assert!(fs::write(root.join(".weftext-rules"), b"{}").is_err());
                assert!(fs::rename(root.join("Owner"), root.join("Replaced")).is_err());
                core.save(SaveRequest {
                    session_id: node.session_id.clone(),
                    base_id: node.base_id.clone(),
                    source: node.source.clone() + "\nscope save\n",
                    operation_id: fresh(),
                })
            },
        )
        .unwrap();
}
#[test]
fn query_scope_committed_rules_missing_or_invalid_are_not_legacy_defaults() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let node = f.node(&root, "Note");
    let owner = f.owner(&node);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(root.join(".weftext-rules")).unwrap()
        )
        .unwrap(),
        serde_json::json!({"version":1,"ignored":[],"unmanaged":[]})
    );
    let q = f
        .core
        .qualified_query_node_at(&root, Path::new(&node.path), &owner, "live")
        .unwrap();
    fs::write(root.join(".weftext-rules"), b"bad rules").unwrap();
    assert!(
        f.core
            .validate_query_node_witness(&root, &q.witness)
            .is_err()
    );
    assert_eq!(
        f.core
            .qualified_query_node_at(&root, Path::new(&node.path), &owner, "live")
            .err()
            .unwrap()
            .code,
        "unsupportedRules"
    );
    fs::remove_file(root.join(".weftext-rules")).unwrap();
    assert_eq!(
        f.core
            .qualified_query_node_at(&root, Path::new(&node.path), &owner, "live")
            .err()
            .unwrap()
            .code,
        "rulesUnavailable"
    );
    let alias = root.with_file_name("lIbRaRy");
    assert_eq!(
        f.core
            .qualified_query_node_at(&alias, Path::new(&node.path), &owner, "live")
            .err()
            .unwrap()
            .code,
        "rulesUnavailable"
    );
}
#[test]
fn query_scope_default_rules_have_no_native_loss_but_nonempty_rules_still_report_omission() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let _node = f.node(&root, "Note");
    let empty = weftext_export::exchange::read_native_package(
        &f.core.export_native_package(&root).unwrap(),
    )
    .unwrap();
    assert!(
        !empty
            .manifest
            .losses
            .iter()
            .any(|loss| loss.path == ".weftext-rules")
    );
    fs::write(
        root.join(".weftext-rules"),
        br#"{"version":1,"ignored":["Excluded"],"unmanaged":[]}"#,
    )
    .unwrap();
    let nonempty = weftext_export::exchange::read_native_package(
        &f.core.export_native_package(&root).unwrap(),
    )
    .unwrap();
    assert!(
        nonempty
            .manifest
            .losses
            .iter()
            .any(|loss| loss.path == ".weftext-rules" && loss.code == "unregisteredFileOmitted")
    );
    fs::write(
        root.join(".weftext-rules"),
        br#"{"version":1,"ignored":[],"unmanaged":[],"unknown":true}"#,
    )
    .unwrap();
    assert!(f.core.export_native_package(&root).is_err());
}
#[test]
fn query_scope_genuine_legacy_absence_is_readable_but_not_a_guarded_save_basis() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("OldLibrary");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let operation = fresh();
    let workspace = fresh();
    // Construct the actual pre-rules create-only birth manifest, not a current
    // library with its committed rules moved away.
    let config = WorkspaceConfig {
        version: 1,
        workspace_id: workspace.clone(),
        storage_profile: STORAGE_PROFILE.into(),
        schema_support: "notImplemented".into(),
        sync_support: "notImplemented".into(),
    };
    let mut p = journal::Plan::file(
        operation.clone(),
        hash(
            &serde_json::to_vec(&serde_json::json!({"kind":"createLibrary","path":display(&root)}))
                .unwrap(),
        ),
        &root,
        None,
        vec![],
        None,
        "createLibrary",
    );
    p.workspace_id = Some(workspace.clone());
    p.files=vec![
        (".weftext-workspace.json".into(),serde_json::to_vec(&config).unwrap()),
        (".weftext-format".into(),b"weftext.typst.v1\n".to_vec()),
        (".weftext-mvp-commit.json".into(),serde_json::to_vec(&serde_json::json!({"workspaceId":workspace,"operationId":operation,"storageProfile":STORAGE_PROFILE})).unwrap()),
    ];
    core.stage(p.clone()).unwrap();
    let guard = ParentGuard::acquire(&crate::BackendContext::local().unwrap(), &root).unwrap();
    let directory = guard.create_directory().unwrap();
    for (name, bytes) in &p.files {
        let mut file = ParentGuard::child_file(&directory, name).unwrap();
        write_source(&mut file, bytes).unwrap();
    }
    core.commit(&p).unwrap();
    drop(directory);
    drop(guard);
    let n = core.create_note_fixture(&root, &root, "OldNote").unwrap();
    let owner = weftext_knowledge::NodeRef {
        workspace_id: workspace,
        node_id: n.node_id.clone().unwrap(),
    };
    let q = core
        .qualified_query_node_at(&root, Path::new(&n.path), &owner, "live")
        .unwrap();
    core.validate_query_node_witness(&root, &q.witness).unwrap();
    let original = fs::read(&n.path).unwrap();
    let mut pool = vec![];
    assert_eq!(
        Core::merge_query_node_witness(&mut pool, &q.witness)
            .unwrap_err()
            .code,
        "basisRequired"
    );
    assert!(pool.is_empty());
    assert!(!root.join(".weftext-rules").exists());
    assert_eq!(fs::read(&n.path).unwrap(), original);
}
#[test]
fn query_scope_relocation_member_recovery_keeps_exact_identity_and_allows_late_body() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let _a = f.node(&root, "A");
    let child = f.node(&root.join("A"), "Child");
    let owner = f.owner(&child);
    let p = f.prepare(OperationIntent::RenameNode {
        source: display(&root.join("A")),
        name: "C".into(),
    });
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &f.core.journal_path(&p.operation_id, "install"),
        b"{}",
    )
    .unwrap();
    fs::write(&child.path, child.source.clone() + "\nlate recovery body\n").unwrap();
    fs::rename(root.join("A/A.typ"), root.join("A/C.typ")).unwrap();
    fs::rename(root.join("A"), root.join("C")).unwrap();
    let data = f.core.data_dir.clone();
    drop(f.core);
    f.core = Core::new(data).unwrap();
    // Completing the original decision proves member placement, never its
    // frozen body, and is still subject to original parent/object bindings.
    f.core
        .resolve_recovery(&p.operation_id, "adoptAfterImages")
        .unwrap();
    f.core
        .qualified_query_node_at(&root, &root.join("C/Child/Child.typ"), &owner, "live")
        .unwrap();
}
#[test]
fn query_scope_relocation_recovery_rejects_same_bytes_replacement_or_moved_out_member() {
    for replace in [false, true] {
        let mut f = Fixture::new();
        let root = f.root.clone();
        let _a = f.node(&root, "A");
        let child = f.node(&root.join("A"), "Child");
        let p = f.prepare(OperationIntent::RenameNode {
            source: display(&root.join("A")),
            name: "C".into(),
        });
        journal::private_create(
            &crate::BackendContext::local().unwrap(),
            &f.core.journal_path(&p.operation_id, "install"),
            b"{}",
        )
        .unwrap();
        fs::rename(root.join("A/A.typ"), root.join("A/C.typ")).unwrap();
        fs::rename(root.join("A"), root.join("C")).unwrap();
        let path = root.join("C/Child/Child.typ");
        fs::rename(&path, root.join("retained-original.typ")).unwrap();
        if replace {
            fs::write(&path, &child.source).unwrap();
        }
        let data = f.core.data_dir.clone();
        drop(f.core);
        f.core = Core::new(data).unwrap();
        assert!(
            f.core
                .resolve_recovery(&p.operation_id, "adoptAfterImages")
                .is_err()
        );
        assert!(!f.core.committed(&p.operation_id).unwrap());
    }
}

#[test]
fn query_scope_definition_reserve_and_explicit_first_admission_share_original_save_guard() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let created = f.node(&root, "Owner");
    let reference = f.reference(&created);
    let schema_bytes =
        include_bytes!("../../../docs/design/examples/schema.authoring.json").to_vec();
    let schema =
        weftext_data::Schema::from_author_json(std::str::from_utf8(&schema_bytes).unwrap())
            .unwrap();
    let operation = fresh();
    let config_plan = f
        .core
        .prepare_config_save(&root, ConfigRole::Schema, None, schema_bytes, &operation)
        .unwrap();
    f.core
        .execute_operation(&operation, &config_plan.plan_id)
        .unwrap();
    let config = f.core.read_config(&root, ConfigRole::Schema).unwrap();
    let binding = weftext_data::SavedQueryBinding {
        schema: config.role_ref,
        revision: config.revision.unwrap(),
        language: "weftext.query.cel.v1".into(),
    };
    let actor = "hosted:scope:definition";
    let doc = f
        .core
        .with_actor(actor, |core| core.open(&created.path, Some(root.clone())))
        .unwrap();
    let q = f
        .core
        .qualified_query_scope_at(&root, &root.join("Owner"), &reference, "live")
        .unwrap();
    let mut pool = vec![];
    Core::merge_query_scope_witness(&mut pool, &q.witness).unwrap();
    let proposal_id = fresh();
    let mut plan = weftext_data::QueryPlan::default();
    plan.scope.directory = Some(reference);
    let request = HostedDefinitionReservationRequest {
        session_id: doc.session_id.clone(),
        base_id: doc.base_id.clone(),
        operation_id: proposal_id.clone(),
        intent_fingerprint: "a".repeat(64),
        binding,
    };
    let proposal = f
        .core
        .with_actor(actor, |core| {
            core.with_guarded_snapshot_witnesses(
                &root,
                &pool.iter().collect::<Vec<_>>(),
                Path::new(&doc.path),
                |core| {
                    core.actor_definition_reserve("Owner", &root, request, |key, binding| {
                        weftext_data::upsert_saved_query(
                            &doc.source,
                            None,
                            key,
                            "weftext-query-scope",
                            binding,
                            &plan,
                            &schema,
                        )
                        .map_err(|e| CoreError::from_data("invalidQueryDefinition", e))
                    })
                },
            )
        })
        .unwrap();
    assert_eq!(fs::read_to_string(&doc.path).unwrap(), doc.source);
    let source = proposal.patch.unwrap().source;
    let key = proposal.status.key.unwrap();
    let status = f
        .core
        .with_actor(actor, |core| {
            core.with_guarded_snapshot_witnesses(
                &root,
                &pool.iter().collect::<Vec<_>>(),
                Path::new(&doc.path),
                |core| {
                    core.actor_save_with_definition_reservations(
                        "Owner",
                        &root,
                        HostedSaveRequest {
                            input_revision: None,
                            session_id: doc.session_id.clone(),
                            base_id: doc.base_id.clone(),
                            source,
                            operation_id: fresh(),
                        },
                        &[proposal_id],
                    )
                },
            )
        })
        .unwrap();
    assert_eq!(status.status, "committed");
    assert!(
        f.core
            .is_definition_key_admitted(&root, doc.node_id.as_ref().unwrap(), &key)
            .unwrap()
    );
}
