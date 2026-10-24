use super::*;
use crate::test_support::{FixtureRoots, open_core};
use std::fs;

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires three explicit enrolled service roots and retained fixture directory"]
fn distinct_linux_leaf_pending_and_completion_do_not_touch_the_other_session() {
    let roots = FixtureRoots::new();
    let root = roots.a_path().join("library");
    let mut core = open_core(roots.a_path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let upper = core.create_note_fixture(&root, &root, "A").unwrap();
    let lower_path = Path::new(&upper.path).with_file_name("a.typ");
    fs::write(&lower_path, "lower baseline").unwrap();
    let lower = core.open(&lower_path, Some(root.clone())).unwrap();
    let snapshot = core
        .read_snapshot(&root, std::slice::from_ref(&upper.path))
        .unwrap()
        .files
        .remove(0);
    let before = snapshot.bytes.unwrap();
    let mut after = before.clone();
    after.extend_from_slice(b"\nupper update");
    let pending = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: upper.path.clone(),
                    base_id: Some(snapshot.base_id),
                    before: Some(before),
                    after,
                }],
            },
        })
        .unwrap();
    core.install_record(&core.operations[&pending.operation_id])
        .unwrap();
    assert_eq!(
        core.open(Path::new(&upper.path), Some(root.clone()))
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert!(core.open(&lower_path, Some(root.clone())).is_ok());
    let saved = core
        .save(SaveRequest {
            operation_id: fresh(),
            session_id: lower.session_id.clone(),
            base_id: lower.base_id,
            source: "lower saved".into(),
        })
        .unwrap();
    let lower_base = saved.receipt.base_id;
    let mut next_request = core.operations[&pending.operation_id]
        .transaction
        .as_ref()
        .unwrap()
        .request
        .clone();
    assert_eq!(
        core.resolve_recovery(&pending.operation_id, "keepCurrent")
            .unwrap()
            .status,
        "resolved"
    );
    next_request.operation_id = fresh();
    let next = core.prepare_operation(next_request).unwrap();
    core.execute_operation(&next.operation_id, &next.plan_id)
        .unwrap();
    assert_eq!(core.sessions[&lower.session_id].base_id, lower_base);
    assert_eq!(fs::read_to_string(lower_path).unwrap(), "lower saved");
    assert!(
        fs::read_to_string(&upper.path)
            .unwrap()
            .ends_with("upper update")
    );
}

#[test]
#[cfg_attr(
    target_os = "linux",
    ignore = "requires three explicit enrolled service roots and retained fixture directory"
)]
fn pending_subtree_uses_components_and_completion_keeps_the_sibling_base() {
    let roots = FixtureRoots::new();
    let root = roots.a_path().join("library");
    let mut core = open_core(roots.a_path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let child = core.create_note_fixture(&root, &root, "A").unwrap();
    let sibling = core.create_note_fixture(&root, &root, "AB").unwrap();
    let pending = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::RenameNode {
                source: display(Path::new(&child.path).parent().unwrap()),
                name: "Moved".into(),
            },
        })
        .unwrap();
    core.install_record(&core.operations[&pending.operation_id])
        .unwrap();
    assert_eq!(
        core.open(Path::new(&child.path), Some(root.clone()))
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    #[cfg(windows)]
    assert_eq!(
        core.open(
            Path::new(&child.path).with_file_name("a.TYP"),
            Some(root.clone())
        )
        .unwrap_err()
        .code,
        "recoveryPending"
    );
    assert!(
        core.open(Path::new(&sibling.path), Some(root.clone()))
            .is_ok()
    );
    let mut next_request = core.operations[&pending.operation_id]
        .transaction
        .as_ref()
        .unwrap()
        .request
        .clone();
    assert_eq!(
        core.resolve_recovery(&pending.operation_id, "keepCurrent")
            .unwrap()
            .status,
        "resolved"
    );
    next_request.operation_id = fresh();
    let next = core.prepare_operation(next_request).unwrap();
    core.execute_operation(&next.operation_id, &next.plan_id)
        .unwrap();
    assert_eq!(core.sessions[&sibling.session_id].base_id, sibling.base_id);
    assert_eq!(fs::read_to_string(&sibling.path).unwrap(), sibling.source);
}
