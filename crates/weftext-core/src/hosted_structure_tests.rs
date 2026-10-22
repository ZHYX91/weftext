use super::*;
use std::os::windows::fs::OpenOptionsExt;

const ACTOR: &str = "deployment:alice:structure";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_FP: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
struct Fixture {
    _temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    core: Core,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        Self {
            _temp: temp,
            data,
            root,
            core,
        }
    }
    fn restart(&mut self) {
        let placeholder = Core::new(self._temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, placeholder));
        self.core = Core::new(&self.data).unwrap();
    }
    fn request(&self, intent: OperationIntent) -> OperationRequest {
        OperationRequest {
            operation_id: fresh(),
            root: display(&self.root),
            intent,
        }
    }
    fn prepare(&mut self, resource: &str, op: &str, intent: OperationIntent) -> ActorFeatureStatus {
        let destination = match &intent {
            OperationIntent::TrashNode { .. } => None,
            OperationIntent::NewNode { name, .. }
            | OperationIntent::RenameNode { name, .. }
            | OperationIntent::RestoreNode { name, .. } => Some(name.clone()),
            _ => Some(resource.into()),
        };
        let request = self.request(intent);
        self.core
            .with_actor(ACTOR, |c| {
                c.actor_feature_prepare_structure(
                    resource,
                    op,
                    FP,
                    request,
                    serde_json::json!({"authorPreview":"original"}),
                    &[],
                    &[],
                    &[],
                    destination.as_deref(),
                    None,
                )
            })
            .unwrap()
    }
    fn commit(
        &mut self,
        resource: &str,
        op: &str,
        plan: &ActorFeatureStatus,
    ) -> ActorFeatureStatus {
        self.core
            .with_actor(ACTOR, |c| {
                c.actor_feature_commit_with_losses(
                    resource,
                    op,
                    plan.plan_id.as_deref().unwrap(),
                    true,
                )
            })
            .unwrap()
    }
    fn node(&mut self, name: &str) -> weftext_knowledge::NodeRef {
        let doc = self
            .core
            .create_note_fixture(&self.root, &self.root, name)
            .unwrap();
        self.core
            .qualified_node_at(&self.root, Path::new(&doc.path))
            .unwrap()
            .owner
    }
    fn copy(&mut self, op: &str, name: &str, witness: &FeatureTreeWitness) -> ActorFeatureStatus {
        let request = self.request(OperationIntent::CopyNode {
            source: display(&self.root.join("Source")),
            parent: display(&self.root),
            name: name.into(),
            allow_losses: false,
        });
        self.core
            .with_actor(ACTOR, |c| {
                c.actor_feature_prepare_structure(
                    "Destination",
                    op,
                    FP,
                    request,
                    serde_json::json!({"name":name}),
                    &[],
                    &[],
                    &[],
                    Some("Destination"),
                    Some(witness),
                )
            })
            .unwrap()
    }
}

#[test]
fn hosted_structure_original_ids_refs_drafts_and_trash_receipts_survive_restart() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let new = fresh();
    let p = f.prepare(
        "Created",
        &new,
        OperationIntent::NewNode {
            parent: display(&root),
            name: "Created".into(),
            source: None,
        },
    );
    assert_eq!(p.intent_kind, "newNode");
    assert_eq!(p.node_refs.len(), 1);
    let original = p.node_refs[0].clone();
    assert!(!root.join("Created").exists());
    f.restart();
    let retry = f
        .core
        .with_actor(ACTOR, |c| c.actor_feature_retry("Created", &new, FP))
        .unwrap()
        .unwrap();
    assert_eq!(retry.plan_id, p.plan_id);
    assert_eq!(retry.node_refs, p.node_refs);
    assert_eq!(f.commit("Created", &new, &p).status, "committed");
    let path = root.join("Created/Created.typ");
    let doc = f
        .core
        .with_actor(ACTOR, |c| c.open(&path, Some(root.clone())))
        .unwrap();
    let private = format!("{}\nACTOR_PRIVATE_DRAFT", doc.source);
    f.core
        .with_actor(ACTOR, |c| c.persist_draft(&doc.session_id, &private))
        .unwrap();
    let rename = fresh();
    let p = f.prepare(
        "Created",
        &rename,
        OperationIntent::RenameNode {
            source: display(&root.join("Created")),
            name: "Renamed".into(),
        },
    );
    assert_eq!(p.node_refs, vec![original.clone()]);
    f.commit("Created", &rename, &p);
    let renamed = root.join("Renamed/Renamed.typ");
    assert_eq!(
        f.core
            .qualified_structure_slot_at(&root, &renamed, "Renamed")
            .unwrap()
            .owner,
        original
    );
    assert!(
        f.core
            .qualified_structure_slot_at(&root, &renamed, "Created")
            .is_err()
    );
    f.core
        .with_actor(ACTOR, |c| {
            c.actor_feature_structure_result_at("Created", &root, &renamed, &rename, &original)
        })
        .unwrap();
    fs::create_dir(root.join("Parent")).unwrap();
    let moved = fresh();
    let p = f.prepare(
        "Created",
        &moved,
        OperationIntent::MoveNode {
            source: display(&root.join("Renamed")),
            parent: display(&root.join("Parent")),
        },
    );
    f.commit("Created", &moved, &p);
    let trash = fresh();
    let p = f.prepare(
        "Created",
        &trash,
        OperationIntent::TrashNode {
            source: display(&root.join("Parent/Renamed")),
        },
    );
    f.commit("Created", &trash, &p);
    f.restart();
    let receipt = f
        .core
        .with_actor(ACTOR, |c| {
            c.actor_feature_structure_receipt("Created", &trash)
        })
        .unwrap()
        .unwrap();
    let internal = receipt.trash_operation_id.clone().unwrap();
    assert_ne!(internal, trash);
    assert_eq!(receipt.nodes[0].node_ref, original);
    assert_eq!(receipt.nodes[0].lifecycle, "trash");
    assert_eq!(receipt.nodes[0].source.file_name().unwrap(), "Renamed.typ");
    f.core
        .with_actor(ACTOR, |c| {
            c.actor_feature_structure_result_at(
                "Created",
                &root,
                &receipt.nodes[0].source,
                &trash,
                &original,
            )
        })
        .unwrap();
    let restore = fresh();
    let p = f.prepare(
        "Restored",
        &restore,
        OperationIntent::RestoreNode {
            trash_operation_id: internal,
            node_id: None,
            parent: display(&root),
            name: "Restored".into(),
        },
    );
    f.commit("Restored", &restore, &p);
    let restored = root.join("Restored/Restored.typ");
    assert_eq!(
        f.core
            .qualified_structure_slot_at(&root, &restored, "Restored")
            .unwrap()
            .owner,
        original
    );
    let reopened = f
        .core
        .with_actor(ACTOR, |c| c.open(&restored, Some(root.clone())))
        .unwrap();
    assert_eq!(reopened.recovered_draft.as_deref(), Some(private.as_str()));
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_settle("Created", &trash))
            .unwrap()
            .status,
        "committed"
    );
    assert!(
        f.core
            .with_actor("deployment:bob", |c| c
                .actor_feature_structure_receipt("Created", &trash))
            .unwrap()
            .is_none()
    );
}

#[test]
fn hosted_structure_copy_uses_only_original_authorized_tree_and_preserves_map() {
    let mut f = Fixture::new();
    let owner = f.node("Source");
    f.node("Private");
    let _hidden = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(f.root.join("Private/Private.typ"))
        .unwrap();
    fs::write(f.root.join("Source/plain.txt"), b"RAW_AUTHORIZED_MEMBER").unwrap();
    fs::write(f.root.join("Source/Source.txt"), b"SAME_STEM_RAW_MEMBER").unwrap();
    let tree = f
        .core
        .capture_feature_tree(&f.root, &f.root.join("Source"), &owner)
        .unwrap();
    assert_eq!(tree.file_count(), 3);
    assert!(tree.retained_bytes() > 0);
    let op = fresh();
    let p = f.copy(&op, "Copied", &tree);
    assert_eq!(p.node_refs.len(), 1);
    assert_ne!(p.node_refs[0], owner);
    assert_eq!(p.preview["losses"], serde_json::json!([]));
    f.restart();
    let replay = f
        .core
        .with_actor(ACTOR, |c| c.actor_feature_retry("Destination", &op, FP))
        .unwrap()
        .unwrap();
    assert_eq!(replay.node_refs, p.node_refs);
    f.commit("Destination", &op, &p);
    assert_eq!(
        fs::read(f.root.join("Copied/plain.txt")).unwrap(),
        b"RAW_AUTHORIZED_MEMBER"
    );
    assert_eq!(
        fs::read(f.root.join("Copied/Source.txt")).unwrap(),
        b"SAME_STEM_RAW_MEMBER"
    );
    let receipt = f
        .core
        .with_actor(ACTOR, |c| {
            c.actor_feature_structure_receipt("Destination", &op)
        })
        .unwrap()
        .unwrap();
    assert_eq!(receipt.nodes[0].node_ref, p.node_refs[0]);
    let path = f.root.join("Copied/Copied.typ");
    f.core
        .with_actor(ACTOR, |c| {
            c.actor_feature_structure_result_at("Destination", &f.root, &path, &op, &p.node_refs[0])
        })
        .unwrap();
    assert_eq!(
        f.core
            .qualified_structure_slot_at(&f.root, &path, "Destination")
            .unwrap()
            .owner,
        p.node_refs[0]
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_retry(
                "Destination",
                &op,
                OTHER_FP
            ))
            .unwrap_err()
            .code,
        "operationConflict"
    );
}

#[test]
fn hosted_structure_empty_directory_frontier_has_an_early_finite_budget() {
    let mut f = Fixture::new();
    let owner = f.node("Source");
    // Each individual directory is below its entry limit; the combined empty
    // frontier must still fail before it can grow without a retained-byte cut.
    for branch in ["A", "B"] {
        let path = f.root.join("Source").join(branch);
        fs::create_dir(&path).unwrap();
        for n in 0..2500 {
            fs::create_dir(path.join(format!("{n:04}"))).unwrap();
        }
    }
    assert_eq!(
        f.core
            .capture_feature_tree(&f.root, &f.root.join("Source"), &owner)
            .err()
            .unwrap()
            .code,
        "budget"
    );
    assert!(!f.root.join("Destination").exists());
}

#[test]
fn hosted_structure_namespace_parent_is_not_a_transfer_source_and_explicit_context_stays_frozen() {
    let mut f = Fixture::new();
    let source_owner = f.node("Source");
    let parent_owner = f.node("PrivateParent");
    let root = f.root.clone();
    let parent_path = root.join("PrivateParent/PrivateParent.typ");
    let target_url = crate::operations::file_url(&parent_path).unwrap();
    let source_path = root.join("Source/Source.typ");
    let source = format!(
        "{}\n#link({})[Original link]\n",
        fs::read_to_string(&source_path).unwrap(),
        serde_json::to_string(&target_url).unwrap()
    );
    fs::write(&source_path, source).unwrap();
    let tree = f
        .core
        .capture_feature_tree(&root, &root.join("Source"), &source_owner)
        .unwrap();
    let parent_ref = format!(
        "weftext-node://{}/{}",
        parent_owner.workspace_id, parent_owner.node_id
    );
    let scope = f
        .core
        .qualified_query_scope_at(&root, &root.join("PrivateParent"), &parent_ref, "live")
        .unwrap();
    let mut namespace = vec![];
    Core::merge_query_scope_witness(&mut namespace, &scope.witness).unwrap();
    let namespace_refs = namespace.iter().collect::<Vec<_>>();
    let request = f.request(OperationIntent::CopyNode {
        source: display(&root.join("Source")),
        parent: display(&root.join("PrivateParent")),
        name: "Unresolved".into(),
        allow_losses: false,
    });
    let op = fresh();
    let prepared = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare_structure(
                "Unresolved",
                &op,
                FP,
                request,
                serde_json::json!({}),
                &[],
                &namespace_refs,
                &[],
                Some("Unresolved"),
                Some(&tree),
            )
        })
        .unwrap();
    assert!(!prepared.preview["losses"].as_array().unwrap().is_empty());
    f.commit("Unresolved", &op, &prepared);
    let copied = fs::read_to_string(root.join("PrivateParent/Unresolved/Unresolved.typ")).unwrap();
    assert!(copied.contains(&target_url));
    assert!(!copied.contains(&parent_ref));

    // Source.read was independently obtained for this exact additional input.
    // The namespace proof above does not stand in for this capability.
    let original = f
        .core
        .read_snapshot(&root, &[display(&parent_path)])
        .unwrap()
        .files
        .remove(0);
    let transfer = f.core.take_snapshot_witness(&original.base_id).unwrap();
    let request = f.request(OperationIntent::CopyNode {
        source: display(&root.join("Source")),
        parent: display(&root),
        name: "Explicit".into(),
        allow_losses: false,
    });
    let op = fresh();
    let prepared = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare_structure(
                "Explicit",
                &op,
                FP,
                request,
                serde_json::json!({}),
                &[],
                &[],
                &[&transfer],
                Some("Explicit"),
                Some(&tree),
            )
        })
        .unwrap();
    assert!(prepared.preview["losses"].as_array().unwrap().is_empty());
    let actor_record = f
        .core
        .with_actor(ACTOR, |core| core.actor_feature_record("Explicit", &op))
        .unwrap()
        .unwrap();
    let original_plan = &f.core.operations[&actor_record.core_operation_id];
    let Mutation::CreateTree { files, .. } = &original_plan.transaction.as_ref().unwrap().mutation
    else {
        panic!("original copy plan required")
    };
    assert!(
        files
            .iter()
            .any(|file| decode(&file.after).is_ok_and(|after| after.contains(&parent_ref)))
    );
    fs::write(
        &parent_path,
        format!(
            "{}\nLate authorized context",
            decode(original.bytes.as_ref().unwrap()).unwrap()
        ),
    )
    .unwrap();
    f.restart();
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_commit_with_losses(
                "Explicit",
                &op,
                prepared.plan_id.as_deref().unwrap(),
                true
            ))
            .is_err()
    );
    assert!(!root.join("Explicit").exists());
}

#[test]
fn hosted_structure_tree_is_required_and_changed_member_or_schema_never_commits() {
    let mut f = Fixture::new();
    let owner = f.node("Source");
    let op = fresh();
    let request = f.request(OperationIntent::CopyNode {
        source: display(&f.root.join("Source")),
        parent: display(&f.root),
        name: "Denied".into(),
        allow_losses: true,
    });
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_prepare(
                "Destination",
                &op,
                FP,
                request,
                serde_json::json!({}),
                &[]
            ))
            .unwrap_err()
            .code,
        "basisRequired"
    );
    assert!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_status("Destination", &op))
            .unwrap()
            .is_none()
    );
    let tree = f
        .core
        .capture_feature_tree(&f.root, &f.root.join("Source"), &owner)
        .unwrap();
    let prepared = f.copy(&fresh(), "Stale", &tree);
    fs::write(f.root.join("Source/late.txt"), b"LATE_MEMBER").unwrap();
    assert!(
        f.core
            .validate_feature_tree_witness(&f.root, &tree)
            .is_err()
    );
    f.restart();
    assert!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_commit(
                "Destination",
                &prepared.operation_id,
                prepared.plan_id.as_deref().unwrap()
            ))
            .is_err()
    );
    assert!(!f.root.join("Stale").exists());
    let tree = f
        .core
        .capture_feature_tree(&f.root, &f.root.join("Source"), &owner)
        .unwrap();
    let op = fresh();
    let p = f.copy(&op, "SchemaChanged", &tree);
    fs::write(f.root.join(".weftext-schema.json"), b"{}\n").unwrap();
    assert!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_commit(
                "Destination",
                &op,
                p.plan_id.as_deref().unwrap()
            ))
            .is_err()
    );
    assert!(!f.root.join("SchemaChanged").exists());
}

#[test]
fn hosted_structure_template_freezes_original_source_and_instantiates_fresh_identity() {
    let mut f = Fixture::new();
    let owner = f.node("Source");
    let path = f.root.join("Source/Source.typ");
    let base = f
        .core
        .read_snapshot(&f.root, &[display(&path)])
        .unwrap()
        .files
        .remove(0);
    let tree = f
        .core
        .capture_feature_tree(&f.root, &f.root.join("Source"), &owner)
        .unwrap();
    let request = f.request(OperationIntent::TemplateNode {
        source: display(&path),
        parent: display(&f.root),
        name: "Instance".into(),
        instantiated_source: format!(
            "{}\nInstantiated body",
            decode(base.bytes.as_ref().unwrap()).unwrap()
        ),
        source_base_id: base.base_id,
    });
    let op = fresh();
    let p = f
        .core
        .with_actor(ACTOR, |c| {
            c.actor_feature_prepare_structure(
                "Instance",
                &op,
                FP,
                request,
                serde_json::json!({}),
                &[],
                &[],
                &[],
                Some("Instance"),
                Some(&tree),
            )
        })
        .unwrap();
    assert_eq!(p.intent_kind, "templateNode");
    assert_ne!(p.node_refs[0], owner);
    f.commit("Instance", &op, &p);
    assert!(
        fs::read_to_string(f.root.join("Instance/Instance.typ"))
            .unwrap()
            .contains("Instantiated body")
    );
    assert!(
        !fs::read_to_string(path)
            .unwrap()
            .contains("Instantiated body")
    );
}

#[test]
fn hosted_structure_copy_losses_are_safe_original_and_require_explicit_acceptance() {
    let mut f = Fixture::new();
    let owner = f.node("Source");
    let path = f.root.join("Source/Source.typ");
    let source = format!(
        "{}\n#link(\"file:///C:/PRIVATE_UNGRANTED/hidden.typ\")[Visible label]\n",
        fs::read_to_string(&path).unwrap()
    );
    fs::write(&path, source).unwrap();
    let tree = f
        .core
        .capture_feature_tree(&f.root, &f.root.join("Source"), &owner)
        .unwrap();
    let op = fresh();
    let p = f.copy(&op, "Lossy", &tree);
    assert!(!p.preview["losses"].as_array().unwrap().is_empty());
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains("PRIVATE_UNGRANTED"));
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_commit(
                "Destination",
                &op,
                p.plan_id.as_deref().unwrap()
            ))
            .unwrap_err()
            .code,
        "copyLoss"
    );
    assert!(!f.root.join("Lossy").exists());
    f.commit("Destination", &op, &p);
    assert!(
        fs::read_to_string(f.root.join("Lossy/Lossy.typ"))
            .unwrap()
            .contains("PRIVATE_UNGRANTED")
    );
}

#[test]
fn hosted_structure_category_fence_and_installed_status_are_not_regenerated() {
    let mut f = Fixture::new();
    let op = fresh();
    f.core
        .with_actor(ACTOR, |c| c.actor_feature_settle("Created", &op))
        .unwrap();
    f.restart();
    let root = f.root.clone();
    assert_eq!(
        f.prepare(
            "Created",
            &op,
            OperationIntent::NewNode {
                parent: display(&root),
                name: "Created".into(),
                source: None
            }
        )
        .status,
        "cancelled"
    );
    assert!(!root.join("Created").exists());
    let op = fresh();
    let p = f.prepare(
        "Created",
        &op,
        OperationIntent::NewNode {
            parent: display(&root),
            name: "Created".into(),
            source: None,
        },
    );
    let other = f.request(OperationIntent::TrashNode {
        source: display(&root.join("Created")),
    });
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_prepare(
                "Created",
                &op,
                FP,
                other,
                serde_json::json!({}),
                &[]
            ))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    let internal = f
        .core
        .with_actor(ACTOR, |c| {
            Ok(c.actor_feature_record("Created", &op)?
                .unwrap()
                .core_operation_id)
        })
        .unwrap();
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &f.core.journal_path(&internal, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    f.restart();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_settle("Created", &op))
            .unwrap()
            .status,
        "recoveryPending"
    );
    assert_eq!(f.commit("Created", &op, &p).status, "recoveryPending");
    assert!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_structure_receipt("Created", &op))
            .unwrap()
            .is_none()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_status("WrongOwner", &op))
            .is_err()
    );
}

#[test]
fn hosted_structure_slot_and_receipt_reject_same_byte_physical_replacement() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let op = fresh();
    let p = f.prepare(
        "Created",
        &op,
        OperationIntent::NewNode {
            parent: display(&root),
            name: "Created".into(),
            source: None,
        },
    );
    f.commit("Created", &op, &p);
    let path = root.join("Created/Created.typ");
    let bytes = fs::read(&path).unwrap();
    f.core
        .qualified_structure_slot_at(&root, &path, "Created")
        .unwrap();
    fs::rename(&path, root.join("held-original.typ")).unwrap();
    fs::write(&path, &bytes).unwrap();
    assert!(
        f.core
            .qualified_structure_slot_at(&root, &path, "Created")
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_structure_result_at(
                "Created",
                &root,
                &path,
                &op,
                &p.node_refs[0]
            ))
            .is_err()
    );
    let other = f.node("Unrelated");
    assert!(
        f.core
            .qualified_structure_slot_at(&root, &root.join("Unrelated/Unrelated.typ"), "Unrelated")
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |c| c.actor_feature_structure_result_at(
                "Created",
                &root,
                &root.join("Unrelated/Unrelated.typ"),
                &op,
                &other
            ))
            .is_err()
    );
}
