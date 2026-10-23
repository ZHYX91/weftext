//! Original public producers plus the exact retained install-only checkpoint.
use super::*;

fn prepared_config(core: &mut Core, root: &Path, value: u8) -> OperationPreview {
    let before = core.read_config(root, ConfigRole::Schema).unwrap();
    assert!(before.bytes.is_none());
    core.prepare_config_save(
        root,
        ConfigRole::Schema,
        None,
        serde_json::to_vec(&serde_json::json!({"version":value})).unwrap(),
        &fresh(),
    )
    .unwrap()
}

fn prepared_node(core: &mut Core, root: &Path) -> OperationPreview {
    core.prepare_operation(OperationRequest {
        operation_id: fresh(),
        root: display(root),
        intent: OperationIntent::NewNode {
            parent: display(root),
            name: "AbsentParent".into(),
            source: None,
        },
    })
    .unwrap()
}

fn pending_create_only_refuses(node: bool, recovery: bool) {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let first = if node {
        prepared_node(&mut core, &root)
    } else {
        prepared_config(&mut core, &root, 1)
    };
    let other = if node {
        prepared_node(&mut core, &root)
    } else {
        prepared_config(&mut core, &root, 2)
    };
    let before = fs::read(root.join(".weftext-rules")).unwrap();
    core.install_record(&core.operations[&other.operation_id])
        .unwrap();
    if recovery {
        // Prior code allowed both original install markers to coexist while
        // every create-only slot was still absent. Replay uses those records.
        core.install_record(&core.operations[&first.operation_id])
            .unwrap();
        drop(core);
        core = Core::new(&data).unwrap();
        let original = core.inspect_recovery(&first.operation_id).unwrap();
        assert!(
            original.allowed_actions.is_empty(),
            "{}",
            original.coherence
        );
        for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
            assert!(
                core.resolve_recovery(&first.operation_id, action).is_err(),
                "{action}"
            );
        }
    } else {
        let result = core.execute_operation(&first.operation_id, &first.plan_id);
        assert_eq!(result.unwrap_err().code, "recoveryPending");
        assert!(!core.journal_path(&first.operation_id, "install").exists());
    }
    assert!(!root.join("AbsentParent").exists());
    assert!(!root.join(".weftext-schema.json").exists());
    assert_eq!(fs::read(root.join(".weftext-rules")).unwrap(), before);
    assert!(!core.committed(&first.operation_id).unwrap());
    assert!(!core.committed(&other.operation_id).unwrap());
    assert!(core.operation_details(&first.operation_id).is_ok());
    assert!(core.operation_details(&other.operation_id).is_ok());
}

#[test]
fn pending_update_create_only_refuses_before_original_install() {
    pending_create_only_refuses(false, false);
}
#[test]
fn pending_create_tree_new_parent_refuses_before_original_install() {
    pending_create_only_refuses(true, false);
}
#[test]
fn pending_update_create_only_retains_both_original_records_during_recovery() {
    pending_create_only_refuses(false, true);
}
#[test]
fn pending_create_tree_new_parent_retains_both_original_records_during_recovery() {
    pending_create_only_refuses(true, true);
}

fn pending_absent_child_refuses(recovery: bool) {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    core.create_note_fixture(&root, &root, "Original").unwrap();
    let original_source = fs::read(root.join("Original/Original.typ")).unwrap();
    let first = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::RenameNode {
                source: display(&root.join("Original")),
                name: "Renamed".into(),
            },
        })
        .unwrap();
    let other = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::NewNode {
                parent: display(&root.join("Original")),
                name: "AbsentChild".into(),
                source: None,
            },
        })
        .unwrap();
    core.install_record(&core.operations[&other.operation_id])
        .unwrap();
    if recovery {
        core.install_record(&core.operations[&first.operation_id])
            .unwrap();
        drop(core);
        core = Core::new(&data).unwrap();
        let original = core.inspect_recovery(&first.operation_id).unwrap();
        assert!(
            original.allowed_actions.is_empty(),
            "{}",
            original.coherence
        );
        for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
            assert!(
                core.resolve_recovery(&first.operation_id, action).is_err(),
                "{action}"
            );
        }
    } else {
        assert_eq!(
            core.execute_operation(&first.operation_id, &first.plan_id)
                .unwrap_err()
                .code,
            "recoveryPending"
        );
        assert!(!core.journal_path(&first.operation_id, "install").exists());
    }
    assert_eq!(
        fs::read(root.join("Original/Original.typ")).unwrap(),
        original_source
    );
    assert!(!root.join("Original/AbsentChild").exists());
    assert!(!root.join("Renamed").exists());
    assert!(!core.committed(&first.operation_id).unwrap());
    assert!(!core.committed(&other.operation_id).unwrap());
}

#[test]
fn pending_single_relocate_absent_child_refuses_before_original_install() {
    pending_absent_child_refuses(false);
}

#[test]
fn pending_single_relocate_absent_child_retains_both_original_records_during_recovery() {
    pending_absent_child_refuses(true);
}

#[test]
fn pending_copy_readers_can_share_original_schema_and_source_without_a_write_overlap() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    core.create_note_fixture(&root, &root, "Original").unwrap();
    let schema = core
        .prepare_config_save(
            &root,
            ConfigRole::Schema,
            None,
            serde_json::to_vec(&weftext_data::Schema::bundled().unwrap()).unwrap(),
            &fresh(),
        )
        .unwrap();
    core.execute_operation(&schema.operation_id, &schema.plan_id)
        .unwrap();
    let original_source = fs::read(root.join("Original/Original.typ")).unwrap();
    let original_schema = fs::read(root.join(".weftext-schema.json")).unwrap();
    let mut prepare = |name: &str| {
        core.prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::CopyNode {
                source: display(&root.join("Original")),
                parent: display(&root),
                name: name.into(),
                allow_losses: false,
            },
        })
        .unwrap()
    };
    let first = prepare("First");
    let second = prepare("Second");
    core.install_record(&core.operations[&first.operation_id])
        .unwrap();
    core.execute_operation(&second.operation_id, &second.plan_id)
        .unwrap();
    drop(core);
    core = Core::new(&data).unwrap();
    let original = core.inspect_recovery(&first.operation_id).unwrap();
    assert_eq!(original.coherence, "knownBefore");
    assert!(
        original
            .allowed_actions
            .iter()
            .any(|action| action == "keepCurrent"),
        "{}",
        original.coherence
    );
    core.resolve_recovery(&first.operation_id, "keepCurrent")
        .unwrap();
    assert_eq!(
        fs::read(root.join("Original/Original.typ")).unwrap(),
        original_source
    );
    assert_eq!(
        fs::read(root.join(".weftext-schema.json")).unwrap(),
        original_schema
    );
    assert!(!root.join("First").exists());
    assert!(root.join("Second/Second.typ").exists());
}
