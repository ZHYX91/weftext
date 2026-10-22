use super::*;
use std::os::windows::fs::OpenOptionsExt;

const ACTOR: &str = "deployment:alice:library";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct Fixture {
    temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    source: PathBuf,
    owner: weftext_knowledge::NodeRef,
    core: Core,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("Library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        Self::finish(temp, data, root, core)
    }
    fn legacy() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("Legacy");
        let mut core = Core::new(&data).unwrap();
        let operation = fresh();
        let workspace = fresh();
        let config = WorkspaceConfig {
            version: 1,
            workspace_id: workspace.clone(),
            storage_profile: STORAGE_PROFILE.into(),
            schema_support: "notImplemented".into(),
            sync_support: "notImplemented".into(),
        };
        let mut plan = journal::Plan::file(
            operation.clone(),
            hash(
                &serde_json::to_vec(
                    &serde_json::json!({"kind":"createLibrary","path":display(&root)}),
                )
                .unwrap(),
            ),
            &root,
            None,
            vec![],
            None,
            "createLibrary",
        );
        plan.workspace_id = Some(workspace.clone());
        plan.files=vec![(".weftext-workspace.json".into(),serde_json::to_vec(&config).unwrap()),(".weftext-format".into(),b"weftext.typst.v1\n".to_vec()),(".weftext-mvp-commit.json".into(),serde_json::to_vec(&serde_json::json!({"workspaceId":workspace,"operationId":operation,"storageProfile":STORAGE_PROFILE})).unwrap())];
        core.stage(plan.clone()).unwrap();
        let guard = ParentGuard::acquire(&crate::BackendContext::local().unwrap(), &root).unwrap();
        let directory = guard.create_directory().unwrap();
        for (name, bytes) in &plan.files {
            let mut file = ParentGuard::child_file(&directory, name).unwrap();
            write_source(&mut file, bytes).unwrap();
        }
        core.commit(&plan).unwrap();
        drop(directory);
        drop(guard);
        Self::finish(temp, data, root, core)
    }
    fn finish(temp: tempfile::TempDir, data: PathBuf, root: PathBuf, mut core: Core) -> Self {
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let owner = weftext_knowledge::NodeRef {
            workspace_id: core.require_workspace(&root).unwrap().workspace_id,
            node_id: note.node_id.unwrap(),
        };
        Self {
            temp,
            data,
            root,
            source: PathBuf::from(note.path),
            owner,
            core,
        }
    }
    fn restart(&mut self) {
        let placeholder = Core::new(self.temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, placeholder));
        self.core = Core::new(&self.data).unwrap();
    }
    fn import(&mut self, name: &str, bytes: &[u8]) -> String {
        let p = self
            .core
            .prepare_import_resource(&self.root, &self.source, name, bytes.to_vec(), &fresh())
            .unwrap();
        self.core
            .execute_operation(&p.operation_id, &p.plan_id)
            .unwrap();
        p.resource_map.values().next().unwrap().clone()
    }
    fn resource(
        &self,
        reference: &str,
        name: &str,
        version: Option<&str>,
    ) -> Result<QualifiedResource> {
        self.core.qualified_resource_at(
            &self.root,
            &self.source,
            &self.owner,
            reference,
            &self.source.parent().unwrap().join(name),
            "live",
            version,
        )
    }
    fn execute(&mut self, intent: OperationIntent) -> OperationPreview {
        let p = self
            .core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&self.root),
                intent,
            })
            .unwrap();
        self.core
            .execute_operation(&p.operation_id, &p.plan_id)
            .unwrap();
        p
    }
    fn install_style(&mut self) {
        let s = self.core.read_library_style(&self.root).unwrap();
        let p = self
            .core
            .prepare_library_style(&self.root, &s.base_id, &s.source, true, &fresh())
            .unwrap();
        self.core
            .execute_operation(&p.operation_id, &p.plan_id)
            .unwrap();
        self.core.discard_library_style_basis(&s.base_id).unwrap();
    }
    fn readonly_review(
        &mut self,
        operation: &str,
        witnesses: &[&SnapshotWitness],
    ) -> ActorFeatureStatus {
        let source = fs::read(&self.source).unwrap();
        let s = self
            .core
            .read_snapshot(&self.root, &[display(&self.source)])
            .unwrap()
            .files
            .remove(0);
        let mut after = source.clone();
        after.extend_from_slice(b"\nReviewed original source.");
        let request = OperationRequest {
            operation_id: fresh(),
            root: display(&self.root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: display(&self.source),
                    base_id: Some(s.base_id),
                    before: Some(source),
                    after,
                }],
            },
        };
        self.core
            .with_actor(ACTOR, |core| {
                core.actor_feature_prepare_with_witnesses(
                    "Note",
                    operation,
                    FP,
                    request,
                    serde_json::json!({"review":"proofread"}),
                    &[],
                    witnesses,
                )
            })
            .unwrap()
    }
}

#[test]
fn exact_resource_reads_only_selected_leaf_and_never_registers_writable_bases() {
    let mut f = Fixture::new();
    let reference = f.import("owned.bin", b"selected original bytes");
    f.import("hidden.bin", b"private sibling");
    let manifest = f.source.parent().unwrap().join("weftext.resources.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    value["resources"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"resourceId":fresh(),"path":{"damaged":"sibling descriptor"}}));
    fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();
    let hidden = f.source.parent().unwrap().join("hidden.bin");
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&hidden)
        .unwrap();
    let before = f.core.snapshots.len();
    let q = f.resource(&reference, "owned.bin", None).unwrap();
    assert_eq!(q.bytes(), b"selected original bytes");
    assert!(q.revision.is_some());
    assert!(q.witness.retained_bytes() >= q.bytes().len());
    f.core
        .validate_resource_witness(&f.root, &q.witness)
        .unwrap();
    assert_eq!(f.core.snapshots.len(), before);
    assert!(
        f.core
            .managed_resource_snapshot(&f.root, &f.source)
            .is_err()
    );
    drop(held);
    fs::remove_file(hidden).unwrap();
    assert_eq!(
        f.resource(&reference, "owned.bin", q.revision.as_deref())
            .unwrap()
            .content_hash,
        q.content_hash
    );
    assert_eq!(
        f.resource(&reference, "hidden.bin", None)
            .err()
            .unwrap()
            .code,
        "resourcePath"
    );
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
    assert!(
        f.core
            .qualified_resource_at(
                &f.root,
                Path::new(&other.path),
                &f.owner,
                &reference,
                &f.source.parent().unwrap().join("owned.bin"),
                "live",
                None
            )
            .is_err()
    );
}

#[test]
fn exact_resource_rejects_copy_same_bytes_and_original_version_does_not_fallback() {
    let mut f = Fixture::new();
    let reference = f.import("owned.bin", b"original");
    let q = f.resource(&reference, "owned.bin", None).unwrap();
    let revision = q.revision.clone().unwrap();
    let leaf = f.source.parent().unwrap().join("owned.bin");
    let original = f.temp.path().join("original-retained.bin");
    fs::rename(&leaf, &original).unwrap();
    fs::copy(&original, &leaf).unwrap();
    assert_eq!(
        f.resource(&reference, "owned.bin", Some(&revision))
            .err()
            .unwrap()
            .code,
        "resourceUnadmitted"
    );
    assert!(
        f.core
            .validate_resource_witness(&f.root, &q.witness)
            .is_err()
    );
    fs::remove_file(&leaf).unwrap();
    fs::rename(&original, &leaf).unwrap();
    fs::write(&leaf, b"external current bytes").unwrap();
    let current = f.resource(&reference, "owned.bin", None).unwrap();
    assert!(current.revision.is_none());
    assert_eq!(current.bytes(), b"external current bytes");
    assert_eq!(
        f.resource(&reference, "owned.bin", Some(&revision))
            .err()
            .unwrap()
            .code,
        "resourceVersion"
    );
    let fake = format!("weftext-resource://{}/{}", f.owner.workspace_id, fresh());
    assert_eq!(
        f.resource(&fake, "owned.bin", None).err().unwrap().code,
        "resourceMissing"
    );
}

#[test]
fn exact_resource_follows_proven_owner_move_and_trash_original_leaf() {
    let mut f = Fixture::new();
    let reference = f.import("owned.bin", b"moving bytes");
    let old = f.source.clone();
    f.execute(OperationIntent::RenameNode {
        source: display(old.parent().unwrap()),
        name: "Renamed".into(),
    });
    f.source = f.root.join("Renamed/Renamed.typ");
    assert_eq!(
        f.resource(&reference, "owned.bin", None).unwrap().bytes(),
        b"moving bytes"
    );
    assert!(
        f.core
            .qualified_resource_at(
                &f.root,
                &old,
                &f.owner,
                &reference,
                &old.parent().unwrap().join("owned.bin"),
                "live",
                None
            )
            .is_err()
    );
    let p = f.execute(OperationIntent::TrashNode {
        source: display(f.source.parent().unwrap()),
    });
    let tree = f.root.join(".weftext-trash").join(&p.operation_id);
    let source = tree.join("Renamed.typ");
    let trash = f
        .core
        .qualified_resource_at(
            &f.root,
            &source,
            &f.owner,
            &reference,
            &tree.join("owned.bin"),
            "trash",
            None,
        )
        .unwrap();
    assert_eq!(trash.bytes(), b"moving bytes");
    f.core
        .validate_resource_witness(&f.root, &trash.witness)
        .unwrap();
    assert!(
        f.core
            .qualified_resource_at(
                &f.root,
                &source,
                &f.owner,
                &reference,
                &tree.join("owned.bin"),
                "live",
                None
            )
            .is_err()
    );
    assert!(
        f.core
            .qualified_resource_at(
                &f.root,
                &tree.join(format!("{}.typ", p.operation_id)),
                &f.owner,
                &reference,
                &tree.join("owned.bin"),
                "trash",
                None
            )
            .is_err()
    );
    f.execute(OperationIntent::RestoreNode {
        trash_operation_id: p.operation_id,
        node_id: None,
        parent: display(&f.root),
        name: "Restored".into(),
    });
    f.source = f.root.join("Restored/Restored.typ");
    assert_eq!(
        f.resource(&reference, "owned.bin", None).unwrap().bytes(),
        b"moving bytes"
    );
    assert!(
        f.core
            .validate_resource_witness(&f.root, &trash.witness)
            .is_err()
    );
}

#[test]
fn exact_resource_missing_rules_distinguishes_committed_loss_and_legacy() {
    let mut f = Fixture::new();
    let reference = f.import("owned.bin", b"rules protected");
    fs::remove_file(f.root.join(".weftext-rules")).unwrap();
    assert_eq!(
        f.resource(&reference, "owned.bin", None)
            .err()
            .unwrap()
            .code,
        "rulesUnavailable"
    );
    let mut legacy = Fixture::legacy();
    let reference = legacy.import("owned.bin", b"legacy bytes");
    let q = legacy.resource(&reference, "owned.bin", None).unwrap();
    let mut pool = vec![];
    assert_eq!(
        Core::merge_resource_witness(&mut pool, &q.witness)
            .unwrap_err()
            .code,
        "basisRequired"
    );
    assert!(pool.is_empty());
    Core::merge_resource_feature_witness(&mut pool, &q.witness).unwrap();
    assert!(pool.iter().any(|witness| {
        Path::new(&witness.binding.path) == legacy.root.join(".weftext-rules")
            && witness.binding.captured.is_none()
    }));
    for witness in &pool {
        legacy
            .core
            .validate_snapshot_witness(&legacy.root, &witness.binding.path, witness)
            .unwrap();
    }
    legacy
        .core
        .validate_resource_witness(&legacy.root, &q.witness)
        .unwrap();
    assert!(!legacy.root.join(".weftext-rules").exists());
    let config = legacy
        .core
        .read_config(&legacy.root, ConfigRole::Rules)
        .unwrap();
    assert!(config.bytes.is_none());
    assert!(config.role_ref.ends_with("/rules"));
    let p = legacy
        .core
        .prepare_config_save(
            &legacy.root,
            ConfigRole::Rules,
            None,
            br#"{"version":1,"ignored":[],"unmanaged":[]}"#.to_vec(),
            &fresh(),
        )
        .unwrap();
    assert!(!legacy.root.join(".weftext-rules").exists());
    legacy
        .core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    assert!(
        legacy
            .core
            .validate_resource_witness(&legacy.root, &q.witness)
            .is_err()
    );
    let negative = pool
        .iter()
        .find(|witness| {
            Path::new(&witness.binding.path) == legacy.root.join(".weftext-rules")
                && witness.binding.captured.is_none()
        })
        .unwrap();
    assert!(
        legacy
            .core
            .validate_snapshot_witness(&legacy.root, &negative.binding.path, negative)
            .is_err()
    );
    assert!(
        legacy
            .core
            .read_config(&legacy.root, ConfigRole::Rules)
            .unwrap()
            .revision
            .is_some()
    );
    fs::remove_file(legacy.root.join(".weftext-rules")).unwrap();
    assert_eq!(
        legacy
            .core
            .read_config(&legacy.root, ConfigRole::Rules)
            .unwrap_err()
            .code,
        "rulesUnavailable"
    );
}

#[test]
fn rules_writes_use_original_strict_parser_and_leave_original_on_invalid_input() {
    let mut f = Fixture::new();
    let before = fs::read(f.root.join(".weftext-rules")).unwrap();
    for invalid in [
        br#"{"version":2,"ignored":[],"unmanaged":[]}"#.as_slice(),
        br#"{"version":1,"ignored":[],"unmanaged":[],"unknown":true}"#,
        br#"{"version":1,"ignored":["../escape"],"unmanaged":[]}"#,
        br#"{"version":1,"ignored":["*.typ"],"unmanaged":[]}"#,
        br#"{"version":1,"ignored":[],"unmanaged":"bad"}"#,
    ] {
        assert!(validate_library_rules(invalid).is_err());
        assert!(
            f.core
                .prepare_config_save(
                    &f.root,
                    ConfigRole::Rules,
                    Some(before.clone()),
                    invalid.to_vec(),
                    &fresh()
                )
                .is_err()
        );
        assert_eq!(fs::read(f.root.join(".weftext-rules")).unwrap(), before);
    }
    let after =
        br#"{"version":1,"ignored":["Hidden/Deep"],"unmanaged":["Notes/Plain.typ"]}"#.to_vec();
    validate_library_rules(&after).unwrap();
    let p = f
        .core
        .prepare_config_save(
            &f.root,
            ConfigRole::Rules,
            Some(before),
            after.clone(),
            &fresh(),
        )
        .unwrap();
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let config = f.core.read_config(&f.root, ConfigRole::Rules).unwrap();
    assert_eq!(config.bytes.as_ref(), Some(&after));
    assert_eq!(
        f.core
            .read_config_revision(
                &f.root,
                ConfigRole::Rules,
                config.revision.as_deref().unwrap()
            )
            .unwrap(),
        after
    );
}

#[test]
fn owned_style_read_success_and_failure_release_temporary_tokens() {
    let mut f = Fixture::new();
    let tokens = f.core.snapshots.len();
    let bases = f.core.style_bases.len();
    for _ in 0..40 {
        let doc = f.core.read_library_style(&f.root).unwrap();
        let expected = f
            .core
            .library_style_basis_retained_bytes(&doc.base_id)
            .unwrap();
        let w = f.core.take_library_style_witness(&doc.base_id).unwrap();
        assert!(w.retained_bytes() <= expected);
        f.core.validate_library_style_witness(&f.root, &w).unwrap();
        assert_eq!(f.core.style_bases.len(), bases);
        assert_eq!(f.core.snapshots.len(), tokens);
        assert!(
            f.core
                .prepare_library_style(&f.root, &doc.base_id, &doc.source, true, &fresh())
                .is_err()
        );
    }
    let doc = f.core.read_library_style(&f.root).unwrap();
    f.core.discard_library_style_basis(&doc.base_id).unwrap();
    f.core.discard_library_style_basis(&doc.base_id).unwrap();
    f.install_style();
    let tokens = f.core.snapshots.len();
    let module = f.root.join("Styles/library-style.typ");
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&module)
        .unwrap();
    assert!(f.core.read_library_style(&f.root).is_err());
    assert_eq!(f.core.snapshots.len(), tokens);
    drop(held);
    let manifest = f.root.join("Styles/weftext.resources.json");
    fs::write(manifest, b"invalid manifest").unwrap();
    assert!(f.core.read_library_style(&f.root).is_err());
    assert_eq!(f.core.snapshots.len(), tokens);
}

#[test]
fn readonly_style_plan_dependencies_survive_restart_and_reject_changed_enabled_module() {
    let mut f = Fixture::new();
    f.install_style();
    let style = f.core.read_library_style(&f.root).unwrap();
    let w = f.core.take_library_style_witness(&style.base_id).unwrap();
    let mut pool = vec![];
    Core::merge_library_style_witness(&mut pool, &w).unwrap();
    assert_eq!(pool.len(), 4);
    let original = fs::read(&f.source).unwrap();
    let op = fresh();
    let prepared = f.readonly_review(&op, &pool.iter().collect::<Vec<_>>());
    // Only the writable review's source basis remains; the four read-only
    // style bindings were frozen into the Plan without registration.
    assert_eq!(f.core.snapshots.len(), 1);
    f.restart();
    fs::write(
        f.root.join("Styles/library-style.typ"),
        style.source.replace("size: 11pt", "size: 18pt"),
    )
    .unwrap();
    let error = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_commit("Note", &op, prepared.plan_id.as_deref().unwrap())
        })
        .unwrap_err();
    assert_eq!(error.code, "conflict");
    assert_eq!(fs::read(&f.source).unwrap(), original);
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_status("Note", &op))
            .unwrap()
            .unwrap()
            .status,
        "prepared"
    );
}

#[test]
fn readonly_absent_style_plan_remains_usable_but_detects_later_settings_creation() {
    for change in [false, true] {
        let mut f = Fixture::new();
        let style = f.core.read_library_style(&f.root).unwrap();
        let w = f.core.take_library_style_witness(&style.base_id).unwrap();
        let mut pool = vec![];
        Core::merge_library_style_witness(&mut pool, &w).unwrap();
        assert_eq!(pool.len(), 1);
        assert!(pool[0].binding.captured.is_none());
        let original = fs::read(&f.source).unwrap();
        let op = fresh();
        let prepared = f.readonly_review(&op, &pool.iter().collect::<Vec<_>>());
        f.restart();
        if change {
            f.install_style();
            assert_eq!(
                f.core
                    .with_actor(ACTOR, |core| core.actor_feature_commit(
                        "Note",
                        &op,
                        prepared.plan_id.as_deref().unwrap()
                    ))
                    .unwrap_err()
                    .code,
                "scopeChanged"
            );
            assert_eq!(fs::read(&f.source).unwrap(), original);
        } else {
            assert_eq!(
                f.core
                    .with_actor(ACTOR, |core| core.actor_feature_commit(
                        "Note",
                        &op,
                        prepared.plan_id.as_deref().unwrap()
                    ))
                    .unwrap()
                    .status,
                "committed"
            );
            assert!(!f.root.join(".weftext-settings.json").exists());
        }
    }
}

#[test]
fn hosted_resource_original_ref_and_plan_survive_restart_retry_and_settle() {
    let mut f = Fixture::new();
    let doc = f
        .core
        .with_actor(ACTOR, |core| core.open(&f.source, Some(f.root.clone())))
        .unwrap();
    f.core
        .with_actor(ACTOR, |core| {
            core.persist_draft(&doc.session_id, &(doc.source.clone() + "\nPRIVATE_DRAFT"))
        })
        .unwrap();
    let leaf = f.source.parent().unwrap().join("hosted.bin");
    let manifest = f.source.parent().unwrap().join("weftext.resources.json");
    let cut = f
        .core
        .read_snapshot(&f.root, &[display(&manifest), display(&leaf)])
        .unwrap();
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::ImportResource {
            node: display(&f.source),
            name: "hosted.bin".into(),
            bytes: b"original import".to_vec(),
            manifest_base_id: cut.files[0].base_id.clone(),
            resource_base_id: cut.files[1].base_id.clone(),
        },
    };
    let op = fresh();
    let status = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request.clone(),
                serde_json::json!({"kind":"resourceImport"}),
                &[],
            )
        })
        .unwrap();
    assert_eq!(status.intent_kind, "importResource");
    assert_eq!(status.status, "prepared");
    assert_eq!(status.resource_refs.len(), 1);
    assert!(!leaf.exists());
    let safe = serde_json::to_string(&status).unwrap();
    assert!(!safe.contains(&display(&f.root)));
    assert!(!safe.contains("hosted.bin"));
    f.restart();
    let original = f
        .core
        .with_actor(ACTOR, |core| core.actor_feature_retry("Note", &op, FP))
        .unwrap()
        .unwrap();
    assert_eq!(original.resource_refs, status.resource_refs);
    assert_eq!(original.plan_id, status.plan_id);
    let committed = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_commit("Note", &op, status.plan_id.as_deref().unwrap())
        })
        .unwrap();
    assert_eq!(committed.resource_refs, status.resource_refs);
    assert_eq!(committed.status, "committed");
    f.restart();
    let retry = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request,
                serde_json::json!({"replacement":true}),
                &[],
            )
        })
        .unwrap();
    assert_eq!(retry.resource_refs, status.resource_refs);
    assert_eq!(retry.preview, status.preview);
    assert_eq!(retry.status, "committed");
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_settle("Note", &op))
            .unwrap()
            .resource_refs,
        status.resource_refs
    );
    assert_eq!(fs::read(&leaf).unwrap(), b"original import");
    assert_eq!(
        f.resource(&status.resource_refs[0], "hosted.bin", None)
            .unwrap()
            .bytes(),
        b"original import"
    );
    assert!(
        f.core
            .with_actor("deployment:bob:library", |core| core.actor_feature_commit(
                "Note",
                &op,
                status.plan_id.as_deref().unwrap()
            ))
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_status("Other", &op))
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_save_status(&op))
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.open(&f.source, Some(f.root.clone())))
            .unwrap()
            .recovered_draft
            .unwrap()
            .contains("PRIVATE_DRAFT")
    );
}

#[test]
fn hosted_style_original_typed_category_and_unknown_fence_do_not_allocate_again() {
    let mut f = Fixture::new();
    let style = f.core.read_library_style(&f.root).unwrap();
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::ConfigureStyle {
            base_id: style.base_id.clone(),
            source: style.source.clone(),
            enabled: true,
        },
    };
    let op = fresh();
    let prepared = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare(
                "library.style",
                &op,
                FP,
                request.clone(),
                serde_json::json!({"kind":"style"}),
                &[],
            )
        })
        .unwrap();
    assert_eq!(prepared.intent_kind, "configureStyle");
    assert_eq!(prepared.resource_refs.len(), 1);
    assert!(!f.root.join("Styles").exists());
    let wrong = OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::UpdateFiles { changes: vec![] },
    };
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "library.style",
                &op,
                FP,
                wrong,
                serde_json::json!({}),
                &[]
            ))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    f.restart();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "library.style",
                &op,
                FP,
                request,
                serde_json::json!({"new":true}),
                &[]
            ))
            .unwrap()
            .resource_refs,
        prepared.resource_refs
    );
    f.core
        .with_actor(ACTOR, |core| {
            core.actor_feature_commit("library.style", &op, prepared.plan_id.as_deref().unwrap())
        })
        .unwrap();
    let installed = f.core.read_library_style(&f.root).unwrap();
    assert!(installed.enabled);
    assert_eq!(
        installed.resource_ref.as_ref(),
        prepared.resource_refs.first()
    );
    let unknown = fresh();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core
                .actor_feature_settle("library.style", &unknown))
            .unwrap()
            .status,
        "cancelled"
    );
    let late = OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::ConfigureStyle {
            base_id: installed.base_id,
            source: installed.source,
            enabled: false,
        },
    };
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "library.style",
                &unknown,
                FP,
                late,
                serde_json::json!({"late":true}),
                &[]
            ))
            .unwrap()
            .status,
        "cancelled"
    );
    assert!(f.core.read_library_style(&f.root).unwrap().enabled);
}

#[test]
fn hosted_resource_failed_original_plan_is_fenced_after_restart_and_cannot_reallocate() {
    let mut f = Fixture::new();
    let op = fresh();
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::ImportResource {
            node: display(&f.source),
            name: "failed.bin".into(),
            bytes: b"not installed".to_vec(),
            manifest_base_id: fresh(),
            resource_base_id: fresh(),
        },
    };
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request.clone(),
                serde_json::json!({"original":true}),
                &[]
            ))
            .unwrap_err()
            .code,
        "basisRequired"
    );
    f.restart();
    let retry = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request.clone(),
                serde_json::json!({"new":true}),
                &[],
            )
        })
        .unwrap();
    assert_eq!(retry.status, "notCommitted");
    assert!(retry.resource_refs.is_empty());
    assert!(retry.plan_id.is_none());
    assert_eq!(retry.preview, serde_json::json!({"original":true}));
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_settle("Note", &op))
            .unwrap()
            .status,
        "cancelled"
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request,
                serde_json::json!({}),
                &[]
            ))
            .unwrap()
            .status,
        "cancelled"
    );
    assert!(!f.source.parent().unwrap().join("failed.bin").exists());
}
