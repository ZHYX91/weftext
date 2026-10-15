use super::*;
use crate::operations::{Mutation, RelocationParent};

fn with_new_nodes_in_moving_tree() -> (Pair, String, String) {
    let mut pair = fixture_with_child(false);
    let child = pair
        .a
        .create_note(&pair.ar, pair.ar.join("新名"), "NewChild")
        .unwrap()
        .node_id
        .unwrap();
    let nested = pair
        .a
        .create_note(&pair.ar, pair.ar.join("新名/NewChild"), "Nested")
        .unwrap()
        .node_id
        .unwrap();
    save(
        &mut pair.a,
        &pair.ar,
        "新名/NewChild/NewChild.typ",
        "new child within the original moving tree",
    );
    let resource = pair
        .a
        .prepare_import_resource(
            &pair.ar,
            pair.ar.join("新名/NewChild/Nested/Nested.typ"),
            "nested.bin",
            vec![7, 0, 255],
            &fresh(),
        )
        .unwrap();
    pair.a
        .execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    pair.keys.insert(ObjectKey::Node(child.clone()));
    pair.keys.insert(ObjectKey::Node(nested.clone()));
    pair.a.publish_sync(&pair.ar).unwrap();
    pair.b
        .receive_sync(
            &pair.identity,
            pair.a
                .export_sync_objects(&pair.identity, &pair.keys)
                .unwrap(),
        )
        .unwrap();
    (pair, child, nested)
}

fn assert_new_nodes_in_moving_tree(pair: &mut Pair, child: &str, nested: &str, operation: &str) {
    assert_installed(pair, operation);
    for (id, relative) in [
        (child, "新名/NewChild/NewChild.typ"),
        (nested, "新名/NewChild/Nested/Nested.typ"),
    ] {
        let opened = pair
            .b
            .open(pair.br.join(relative), Some(pair.br.clone()))
            .unwrap();
        assert_eq!(opened.node_id.as_deref(), Some(id));
        assert_eq!(
            opened.source.as_bytes(),
            fs::read(pair.ar.join(relative)).unwrap()
        );
        let history = pair.b.list_history(&opened.session_id).unwrap();
        let born = history
            .versions
            .iter()
            .find(|version| version.operation_id == operation)
            .unwrap();
        assert_eq!(
            pair.b
                .read_history(&opened.session_id, &born.version_id)
                .unwrap()
                .source,
            opened.source
        );
        let owner = weftext_knowledge::NodeRef {
            workspace_id: pair.identity.workspace_id.clone(),
            node_id: id.into(),
        };
        let qualified = pair
            .b
            .qualified_query_node_at(&pair.br, &pair.br.join(relative), &owner, "live")
            .unwrap();
        pair.b
            .validate_query_node_witness(&pair.br, &qualified.witness)
            .unwrap();
    }
    assert_eq!(
        fs::read(pair.br.join("新名/NewChild/Nested/nested.bin")).unwrap(),
        [7, 0, 255]
    );
    let resources = pair
        .b
        .managed_resource_snapshot(&pair.br, pair.br.join("新名/NewChild/Nested/Nested.typ"))
        .unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].identity_status, "admitted");
}

#[test]
fn compound_moving_tree_new_nodes_and_owned_resource_commit_in_original_decision() {
    let (mut pair, child, nested) = with_new_nodes_in_moving_tree();
    let original = crate::operations::capture_optional(&pair.br.join("Left/Left.typ"))
        .unwrap()
        .unwrap()
        .stamp
        .identity;
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    assert!(!pair.br.join("Left/NewChild").exists());
    assert!(!pair.br.join("新名").exists());
    let Mutation::Compound {
        births,
        created_directories,
        ..
    } = &pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap()
        .mutation
    else {
        unreachable!()
    };
    assert_eq!(births.len(), 2);
    for relative in ["新名/NewChild", "新名/NewChild/Nested"] {
        assert!(
            created_directories
                .iter()
                .any(|directory| Path::new(directory) == pair.br.join(relative)),
            "{relative}: {created_directories:?}"
        );
    }
    assert!(
        !created_directories
            .iter()
            .any(|directory| Path::new(directory) == pair.br.join("新名"))
    );
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_new_nodes_in_moving_tree(&mut pair, &child, &nested, &preview.operation_id);
    assert_eq!(
        crate::operations::capture_optional(&pair.br.join("新名/新名.typ"))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        original
    );
}

#[test]
fn compound_moving_tree_new_nodes_recover_only_original_materialized_directories_and_births() {
    for point in [
        "installed",
        "directory:NewChild",
        "directory:Nested",
        "move0",
        "materialize:NewChild.typ",
        "create:nested.bin",
        "facts",
    ] {
        let (mut pair, child, nested) = with_new_nodes_in_moving_tree();
        let preview = pair.b.prepare_operation(request(&pair)).unwrap();
        crate::operations::compound::set_compound_failpoint(point);
        assert!(
            pair.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err(),
            "{point}"
        );
        drop(pair.b);
        pair.b = Core::new(&pair.bd).unwrap();
        let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        let action = if recovery.coherence == "knownAfter" {
            "adoptAfterImages"
        } else {
            "resumeOriginalPlan"
        };
        assert!(
            recovery
                .allowed_actions
                .iter()
                .any(|allowed| allowed == action),
            "{point}: {}",
            serde_json::to_string(&recovery).unwrap()
        );
        pair.b
            .resolve_recovery(&preview.operation_id, action)
            .unwrap();
        assert_new_nodes_in_moving_tree(&mut pair, &child, &nested, &preview.operation_id);
    }
}

#[test]
fn compound_moving_tree_new_node_original_slot_occupancy_prevents_first_effect() {
    let (mut pair, _, _) = with_new_nodes_in_moving_tree();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    fs::create_dir(pair.br.join("Left/NewChild")).unwrap();
    fs::write(
        pair.br.join("Left/NewChild/foreign.raw"),
        b"retain original occupant",
    )
    .unwrap();
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert!(!pair.br.join("新名").exists());
    assert_eq!(
        fs::read(pair.br.join("Left/NewChild/foreign.raw")).unwrap(),
        b"retain original occupant"
    );
}

#[test]
fn compound_moving_tree_new_node_materialized_directory_replacement_refuses_recovery() {
    let (mut pair, _, _) = with_new_nodes_in_moving_tree();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    crate::operations::compound::set_compound_failpoint("directory:NewChild");
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    let path = pair.br.join("Left/NewChild");
    fs::rename(&path, pair.temp.path().join("original-materialized-child")).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("foreign.raw"), b"retain replacement directory").unwrap();
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "thirdState");
    assert!(recovery.allowed_actions.is_empty());
    for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
        assert!(
            pair.b
                .resolve_recovery(&preview.operation_id, action)
                .is_err()
        );
    }
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert_eq!(
        fs::read(path.join("foreign.raw")).unwrap(),
        b"retain replacement directory"
    );
    assert!(!pair.b.committed(&preview.operation_id));
}

fn with_new_directory_parent() -> (Pair, String) {
    let mut pair = fixture_with_child(false);
    fs::create_dir(pair.ar.join("NewContainer")).unwrap();
    let directory = pair
        .a
        .prepare_directory_admission(&pair.ar, pair.ar.join("NewContainer"), &fresh())
        .unwrap();
    pair.a
        .execute_operation(&directory.preview.operation_id, &directory.preview.plan_id)
        .unwrap();
    let (_, _, id) = crate::directory_identity::parse_ref(&directory.target_ref).unwrap();
    pair.keys.insert(ObjectKey::Directory(id.into()));
    run(
        &mut pair.a,
        &pair.ar,
        OperationIntent::MoveNode {
            source: display(&pair.ar.join("Container/Right")),
            parent: display(&pair.ar.join("NewContainer")),
        },
    );
    pair.a.publish_sync(&pair.ar).unwrap();
    pair.b
        .receive_sync(
            &pair.identity,
            pair.a
                .export_sync_objects(&pair.identity, &pair.keys)
                .unwrap(),
        )
        .unwrap();
    (pair, directory.target_ref)
}

fn assert_directory_parent_created(pair: &mut Pair, reference: &str, operation: &str) {
    for path in [
        "新名/新名.typ",
        "新名/weftext.resources.json",
        "新名/weftext.annotations.json",
        "NewContainer/weftext.directory.json",
        "NewContainer/Right/Right.typ",
        "NewContainer/Right/weftext.resources.json",
        "NewContainer/Right/baseline.bin",
        "NewContainer/Right/added.bin",
        "Still/Still.typ",
    ] {
        assert_eq!(
            fs::read(pair.br.join(path)).unwrap(),
            fs::read(pair.ar.join(path)).unwrap(),
            "{path}"
        );
    }
    assert_eq!(
        fs::read(pair.br.join("新名/raw.txt")).unwrap(),
        b"receiver-only left"
    );
    assert_eq!(
        fs::read(pair.br.join("NewContainer/Right/raw.txt")).unwrap(),
        b"receiver-only right"
    );
    assert!(!pair.br.join("Left").exists());
    assert!(!pair.br.join("Right").exists());
    assert!(!pair.br.join("NewContainer/NewContainer.typ").exists());
    let resolved = pair
        .b
        .resolve_navigation_target(&pair.br, reference)
        .unwrap();
    assert_eq!(resolved.status, "managed");
    assert_eq!(
        resolved.path.as_deref(),
        Some(display(&pair.br.join("NewContainer")).as_str())
    );
    let objects = pair
        .b
        .sync_binding_info(&pair.br)
        .unwrap()
        .installed_objects
        .into_iter()
        .collect();
    assert_eq!(
        pair.b
            .read_sync_adoption_cut(&pair.br, &objects)
            .unwrap_err()
            .code,
        "noChange"
    );
    assert!(pair.b.committed(operation));
}

#[test]
fn compound_new_directory_parent_is_one_original_create_and_move_decision() {
    let (mut pair, reference) = with_new_directory_parent();
    let req = request(&pair);
    let preview = pair.b.prepare_operation(req.clone()).unwrap();
    let transaction = pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    let Mutation::Compound {
        births,
        created_directories,
        relocations,
        ..
    } = &transaction.mutation
    else {
        unreachable!()
    };
    assert!(births.is_empty(), "A Directory is never a Node birth");
    assert!(
        created_directories
            .iter()
            .any(|path| Path::new(path) == pair.br.join("NewContainer"))
    );
    assert!(relocations.iter().any(|step| matches!(&step.target_parent_identity,
        RelocationParent::Created { created_directory } if Path::new(created_directory) == pair.br.join("NewContainer"))));
    assert!(!pair.br.join("NewContainer").exists());
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    assert_eq!(
        pair.b.prepare_operation(req).unwrap().plan_id,
        preview.plan_id
    );
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_directory_parent_created(&mut pair, &reference, &preview.operation_id);
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    assert_directory_parent_created(&mut pair, &reference, &preview.operation_id);
}

#[test]
fn compound_new_directory_parent_recovers_original_marker_and_directory_after_each_effect() {
    for point in [
        "installed",
        "directory:NewContainer",
        "move0",
        "materialize:weftext.directory.json",
        "facts",
    ] {
        let (mut pair, reference) = with_new_directory_parent();
        let preview = pair.b.prepare_operation(request(&pair)).unwrap();
        crate::operations::compound::set_compound_failpoint(point);
        assert!(
            pair.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err(),
            "{point}"
        );
        drop(pair.b);
        pair.b = Core::new(&pair.bd).unwrap();
        let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        let action = if recovery.coherence == "knownAfter" {
            "adoptAfterImages"
        } else {
            "resumeOriginalPlan"
        };
        assert!(
            recovery
                .allowed_actions
                .iter()
                .any(|allowed| allowed == action),
            "{point}: {}",
            serde_json::to_string(&recovery).unwrap()
        );
        pair.b
            .resolve_recovery(&preview.operation_id, action)
            .unwrap();
        assert_directory_parent_created(&mut pair, &reference, &preview.operation_id);
    }
}

#[test]
fn compound_new_directory_parent_late_occupancy_preserves_all_original_trees() {
    let (mut pair, _) = with_new_directory_parent();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    fs::create_dir(pair.br.join("NewContainer")).unwrap();
    fs::write(
        pair.br.join("NewContainer/foreign.raw"),
        b"preserve foreign container",
    )
    .unwrap();
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert_eq!(
        fs::read(pair.br.join("NewContainer/foreign.raw")).unwrap(),
        b"preserve foreign container"
    );
}

#[test]
fn compound_new_directory_parent_same_bytes_marker_replacement_does_not_gain_identity() {
    let (mut pair, reference) = with_new_directory_parent();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    crate::operations::compound::set_compound_failpoint("facts");
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    let marker = pair.br.join("NewContainer/weftext.directory.json");
    let bytes = fs::read(&marker).unwrap();
    fs::rename(
        &marker,
        pair.temp.path().join("original-directory-marker.json"),
    )
    .unwrap();
    fs::write(&marker, &bytes).unwrap();
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "thirdState");
    assert!(recovery.allowed_actions.is_empty());
    assert_ne!(
        pair.b
            .resolve_navigation_target(&pair.br, &reference)
            .map(|target| target.status)
            .unwrap_or_default(),
        "managed"
    );
    assert_eq!(fs::read(&marker).unwrap(), bytes);
    assert!(!pair.b.committed(&preview.operation_id));
}

fn with_new_parent() -> (Pair, String) {
    let mut pair = fixture_with_child(false);
    let parent = pair
        .a
        .create_note(&pair.ar, &pair.ar, "NewParent")
        .unwrap()
        .node_id
        .unwrap();
    save(
        &mut pair.a,
        &pair.ar,
        "NewParent/NewParent.typ",
        "new parent exact source",
    );
    run(
        &mut pair.a,
        &pair.ar,
        OperationIntent::MoveNode {
            source: display(&pair.ar.join("Container/Right")),
            parent: display(&pair.ar.join("NewParent")),
        },
    );
    pair.a.publish_sync(&pair.ar).unwrap();
    let all = pair
        .a
        .sync_binding_info(&pair.ar)
        .unwrap()
        .installed_objects
        .into_iter()
        .collect();
    fs::write(pair.ar.join("新名/baseline.bin"), [4, 7, 255]).unwrap();
    let cut = pair.a.read_sync_adoption_cut(&pair.ar, &all).unwrap();
    assert_eq!(cut.changes.len(), 1);
    run(
        &mut pair.a,
        &pair.ar,
        OperationIntent::AdoptSyncExternal {
            identity: cut.identity,
            expected_installed: cut.expected_installed,
            changes: cut.changes,
            removed_members: cut.removed_members,
        },
    );
    pair.a.publish_sync(&pair.ar).unwrap();
    pair.b
        .receive_sync(
            &pair.identity,
            pair.a
                .export_sync_objects(&pair.identity, &pair.keys)
                .unwrap(),
        )
        .unwrap();
    (pair, parent)
}

fn assert_created(pair: &mut Pair, parent: &str, operation: &str) {
    for path in [
        "新名/新名.typ",
        "新名/baseline.bin",
        "新名/added.bin",
        "新名/weftext.annotations.json",
        "NewParent/NewParent.typ",
        "NewParent/Right/Right.typ",
        "NewParent/Right/baseline.bin",
        "NewParent/Right/added.bin",
        "Still/Still.typ",
        ".weftext-rules",
    ] {
        assert_eq!(
            fs::read(pair.br.join(path)).unwrap_or_else(|error| panic!("receiver {path}: {error}")),
            fs::read(pair.ar.join(path)).unwrap_or_else(|error| panic!("author {path}: {error}")),
            "{path}"
        );
    }
    assert_eq!(
        fs::read(pair.br.join("新名/raw.txt")).unwrap(),
        b"receiver-only left"
    );
    assert_eq!(
        fs::read(pair.br.join("NewParent/Right/raw.txt")).unwrap(),
        b"receiver-only right"
    );
    assert!(!pair.br.join("Left").exists());
    assert!(!pair.br.join("Right").exists());
    let opened = pair
        .b
        .open(
            pair.br.join("NewParent/NewParent.typ"),
            Some(pair.br.clone()),
        )
        .unwrap();
    assert_eq!(opened.node_id.as_deref(), Some(parent));
    let history = pair.b.list_history(&opened.session_id).unwrap();
    let born = history
        .versions
        .iter()
        .find(|version| version.operation_id == operation)
        .unwrap();
    assert_eq!(
        pair.b
            .read_history(&opened.session_id, &born.version_id)
            .unwrap()
            .source,
        opened.source
    );
    let node = weftext_knowledge::NodeRef {
        workspace_id: pair.identity.workspace_id.clone(),
        node_id: parent.into(),
    };
    let qualified = pair
        .b
        .qualified_query_node_at(
            &pair.br,
            &pair.br.join("NewParent/NewParent.typ"),
            &node,
            "live",
        )
        .unwrap();
    pair.b
        .validate_query_node_witness(&pair.br, &qualified.witness)
        .unwrap();
    assert!(pair.b.committed(operation));
    assert_eq!(
        pair.b
            .sync_binding_info(&pair.br)
            .unwrap()
            .pending_publications,
        0
    );
}

#[test]
fn compound_created_node_parent_and_two_original_moves_keep_birth_history_and_actor_drafts() {
    let (mut pair, parent) = with_new_parent();
    let before = ["Left/Left.typ", "Right/Right.typ"].map(|path| {
        crate::operations::capture_optional(&pair.br.join(path))
            .unwrap()
            .unwrap()
            .stamp
            .identity
    });
    for actor in ["alice", "bob"] {
        pair.b
            .with_actor_for_root(actor, &pair.br, |core| {
                let opened = core.open(pair.br.join("Right/Right.typ"), Some(pair.br.clone()))?;
                core.persist_draft(
                    &opened.session_id,
                    &format!("{}\n{actor} unsaved", opened.source),
                )?;
                Ok(())
            })
            .unwrap();
    }
    let req = request(&pair);
    let preview = pair.b.prepare_operation(req.clone()).unwrap();
    let transaction = pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    let Mutation::Compound {
        births,
        created_directories,
        relocations,
        ..
    } = &transaction.mutation
    else {
        unreachable!()
    };
    assert_eq!(births.len(), 1);
    assert_eq!(births[0].node_id, parent);
    assert!(created_directories.contains(&display(&pair.br.join("NewParent"))));
    assert!(relocations.iter().any(|step| matches!(&step.target_parent_identity, RelocationParent::Created {created_directory} if Path::new(created_directory) == pair.br.join("NewParent"))));
    assert!(!pair.br.join("NewParent").exists());
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    assert_eq!(
        pair.b.prepare_operation(req).unwrap().plan_id,
        preview.plan_id
    );
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_created(&mut pair, &parent, &preview.operation_id);
    for (index, path) in ["新名/新名.typ", "NewParent/Right/Right.typ"]
        .iter()
        .enumerate()
    {
        assert_eq!(
            crate::operations::capture_optional(&pair.br.join(path))
                .unwrap()
                .unwrap()
                .stamp
                .identity,
            before[index]
        );
    }
    for actor in ["alice", "bob"] {
        pair.b
            .with_actor_for_root(actor, &pair.br, |core| {
                let opened = core.open(
                    pair.br.join("NewParent/Right/Right.typ"),
                    Some(pair.br.clone()),
                )?;
                assert!(opened.recovered_draft.is_some());
                assert!(
                    core.read_draft(&pair.br.join("NewParent/Right/Right.typ"))?
                        .unwrap()
                        .source
                        .contains(&format!("{actor} unsaved"))
                );
                Ok(())
            })
            .unwrap();
    }
    save(
        &mut pair.b,
        &pair.br,
        "NewParent/NewParent.typ",
        "ordinary save after original birth",
    );
}

#[test]
fn compound_created_parent_late_occupancy_stops_before_install_or_any_move() {
    let (mut pair, _) = with_new_parent();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    fs::create_dir(pair.br.join("NewParent")).unwrap();
    fs::write(pair.br.join("NewParent/raw.txt"), b"external occupant").unwrap();
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert!(!pair.br.join(".weftext-rules").exists());
    let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "thirdState");
    assert!(recovery.allowed_actions.is_empty());
}

#[test]
fn compound_created_parent_and_birth_resume_only_original_plan_after_each_effect() {
    for point in [
        "installed",
        "directory:NewParent",
        "move0",
        "materialize:NewParent.typ",
        "facts",
    ] {
        let (mut pair, parent) = with_new_parent();
        let preview = pair.b.prepare_operation(request(&pair)).unwrap();
        crate::operations::compound::set_compound_failpoint(point);
        assert!(
            pair.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err(),
            "{point}"
        );
        drop(pair.b);
        pair.b = Core::new(&pair.bd).unwrap();
        let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        let action = if recovery.coherence == "knownAfter" {
            "adoptAfterImages"
        } else {
            "resumeOriginalPlan"
        };
        assert!(
            recovery
                .allowed_actions
                .iter()
                .any(|allowed| allowed == action),
            "{point}: {}",
            serde_json::to_string(&recovery).unwrap()
        );
        pair.b
            .resolve_recovery(&preview.operation_id, action)
            .unwrap();
        assert_created(&mut pair, &parent, &preview.operation_id);
        assert_eq!(
            pair.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap()
                .plan_id,
            preview.plan_id
        );
    }
}

#[test]
fn compound_materialized_parent_replacement_or_extra_member_refuses_all_recovery_actions() {
    for replacement in [false, true] {
        let (mut pair, _) = with_new_parent();
        let preview = pair.b.prepare_operation(request(&pair)).unwrap();
        crate::operations::compound::set_compound_failpoint("directory:NewParent");
        assert!(
            pair.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err()
        );
        let path = pair.br.join("NewParent");
        if replacement {
            fs::rename(&path, pair.temp.path().join("retained-materialized-parent")).unwrap();
            fs::create_dir(&path).unwrap();
        } else {
            fs::write(path.join("unplanned.txt"), b"keep exact external bytes").unwrap();
        }
        drop(pair.b);
        pair.b = Core::new(&pair.bd).unwrap();
        let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        assert_eq!(recovery.coherence, "thirdState");
        assert!(recovery.allowed_actions.is_empty());
        for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
            assert!(
                pair.b
                    .resolve_recovery(&preview.operation_id, action)
                    .is_err()
            );
        }
        assert!(pair.br.join("Left/Left.typ").exists());
        assert!(pair.br.join("Right/Right.typ").exists());
        assert!(!pair.b.committed(&preview.operation_id));
    }
}

#[test]
fn compound_created_parent_pending_fence_blocks_prepare_and_late_execution() {
    for late in [false, true] {
        let (mut pair, _) = with_new_parent();
        let proposed = late.then(|| pair.b.prepare_operation(request(&pair)).unwrap());
        let other = pair
            .b
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&pair.br),
                intent: OperationIntent::NewNode {
                    parent: display(&pair.br),
                    name: "NewParent".into(),
                    source: None,
                },
            })
            .unwrap();
        let original = pair.b.operations[&other.operation_id].clone();
        pair.b.install_record(&original).unwrap();
        let error = if let Some(proposed) = proposed {
            let error = pair
                .b
                .execute_operation(&proposed.operation_id, &proposed.plan_id)
                .unwrap_err();
            assert!(
                !pair
                    .b
                    .journal_path(&proposed.operation_id, "install")
                    .exists()
            );
            error
        } else {
            pair.b.prepare_operation(request(&pair)).unwrap_err()
        };
        assert_eq!(error.code, "recoveryPending");
        assert!(!pair.br.join("NewParent").exists());
        assert!(pair.br.join("Left/Left.typ").exists());
        assert!(pair.br.join("Right/Right.typ").exists());
    }
}

fn assert_late_pending_moving_member_blocks_first_effect(create_only: bool) {
    let (mut pair, _) = with_new_parent();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let relative = if create_only {
        "Left/weftext.annotations.json"
    } else {
        "Right/Right.typ"
    };
    let path = pair.br.join(relative);
    let basis = pair
        .b
        .read_snapshot(&pair.br, &[display(&path)])
        .unwrap()
        .files
        .remove(0);
    assert_eq!(basis.bytes.is_none(), create_only);
    let after = if create_only {
        br#"{"version":1,"annotations":[],"review":"other original writer"}"#.to_vec()
    } else {
        let mut bytes = basis.bytes.clone().unwrap();
        bytes.extend_from_slice(b"\nother original pending source");
        bytes
    };
    let other = pair
        .b
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&pair.br),
            intent: OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: display(&path),
                    base_id: Some(basis.base_id),
                    before: basis.bytes.clone(),
                    after,
                }],
            },
        })
        .unwrap();
    let original = pair.b.operations[&other.operation_id].clone();
    pair.b.install_record(&original).unwrap();
    assert_eq!(fs::read(&path).ok(), basis.bytes);
    assert_eq!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert!(!pair.br.join("NewParent").exists());
    assert!(!pair.br.join("新名").exists());
    assert_eq!(fs::read(&path).ok(), basis.bytes);
    assert!(!pair.b.committed(&other.operation_id));
}

#[test]
fn compound_late_pending_moving_source_blocks_first_effect() {
    assert_late_pending_moving_member_blocks_first_effect(false);
}

#[test]
fn compound_late_pending_moving_create_slot_blocks_first_effect() {
    assert_late_pending_moving_member_blocks_first_effect(true);
}

#[test]
fn compound_late_pending_unmaterialized_child_blocks_first_effect() {
    let (mut pair, _) = with_new_parent();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let other = pair
        .b
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&pair.br),
            intent: OperationIntent::NewNode {
                parent: display(&pair.br.join("Left")),
                name: "LateChild".into(),
                source: None,
            },
        })
        .unwrap();
    let original = pair.b.operations[&other.operation_id].clone();
    pair.b.install_record(&original).unwrap();
    assert!(!pair.br.join("Left/LateChild").exists());
    assert_eq!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert!(!pair.br.join("NewParent").exists());
    assert!(!pair.br.join("新名").exists());
    assert!(!pair.b.committed(&other.operation_id));
}

fn assert_independent_unicode_case_targets_refuse(create_new: bool) {
    let temp = tempfile::tempdir().unwrap();
    let ar = temp.path().join("author");
    let br = temp.path().join("other-author");
    let cr = temp.path().join("receiver");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    a.create_library(&ar).unwrap();
    let mut keys = ["Left", "Right"]
        .into_iter()
        .map(|name| ObjectKey::Node(a.create_note(&ar, &ar, name).unwrap().node_id.unwrap()))
        .collect::<BTreeSet<_>>();
    let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
    let bundle = a.export_sync_objects(&identity, &keys).unwrap();
    let original_keys = keys.clone();
    let mut b = Core::new(temp.path().join("b-private")).unwrap();
    let mut c = Core::new(temp.path().join("c-private")).unwrap();
    for (core, root) in [(&mut b, &br), (&mut c, &cr)] {
        core.receive_sync(&identity, bundle.clone()).unwrap();
        run(
            core,
            root,
            OperationIntent::CloneSync {
                identity: identity.clone(),
                objects: keys.clone(),
            },
        );
    }
    run(
        &mut a,
        &ar,
        OperationIntent::RenameNode {
            source: display(&ar.join("Left")),
            name: "Ä".into(),
        },
    );
    if create_new {
        keys.insert(ObjectKey::Node(
            b.create_note(&br, &br, "ä").unwrap().node_id.unwrap(),
        ));
    } else {
        run(
            &mut b,
            &br,
            OperationIntent::RenameNode {
                source: display(&br.join("Right")),
                name: "ä".into(),
            },
        );
    }
    // This host's actual protected namespace resolves the two spellings to the
    // same object; no Rust Unicode transform is used to establish that fact.
    assert_eq!(
        ParentGuard::acquire(&ar.join("Ä/.probe"))
            .unwrap()
            .directory_identity()
            .unwrap(),
        ParentGuard::acquire(&ar.join("ä/.probe"))
            .unwrap()
            .directory_identity()
            .unwrap()
    );
    for (core, root, selected) in [(&a, &ar, &original_keys), (&b, &br, &keys)] {
        core.publish_sync(root).unwrap();
        c.receive_sync(
            &identity,
            core.export_sync_objects(&identity, selected).unwrap(),
        )
        .unwrap();
    }
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&cr),
        intent: OperationIntent::ApplySync {
            identity,
            objects: keys,
        },
    };
    let operation = request.operation_id.clone();
    assert!(c.prepare_operation(request).is_err());
    assert!(!c.journal_path(&operation, "install").exists());
    assert!(cr.join("Left/Left.typ").exists());
    assert!(cr.join("Right/Right.typ").exists());
    assert!(!cr.join("Ä").exists());
    assert!(!cr.join("ä").exists());
}

#[test]
fn compound_independent_unicode_case_targets_refuse_before_any_namespace_effect() {
    assert_independent_unicode_case_targets_refuse(false);
}

#[test]
fn compound_new_node_and_moving_unicode_case_target_refuse_before_any_namespace_effect() {
    assert_independent_unicode_case_targets_refuse(true);
}

#[test]
fn compound_final_namespace_rejects_file_directory_prefix_and_created_directory_aliases() {
    let (mut pair, _) = with_new_parent();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let original = pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap()
        .clone();
    for alias in [false, true] {
        let mut transaction = original.clone();
        let Mutation::Compound {
            created_files,
            created_directories,
            ..
        } = &mut transaction.mutation
        else {
            unreachable!()
        };
        if alias {
            created_directories.push(display(&pair.br.join("NEwParent")));
        } else {
            // Both are legal portable members independently, but the proposed
            // file cannot also supply a directory for the second output.
            created_files.push(FileChange {
                path: display(&pair.br.join("NewParent/NewParent.typ/x.bin")),
                base_id: None,
                before: None,
                after: vec![0, 255],
            });
        }
        assert!(crate::operations::compound::validate_compound_namespace(&transaction).is_err());
    }
    assert!(!pair.br.join("NewParent").exists());
}

#[test]
fn compound_recovery_retains_legacy_coexisting_pending_absent_child_without_moving_either_plan() {
    let (mut pair, _) = with_new_parent();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let other = pair
        .b
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&pair.br),
            intent: OperationIntent::NewNode {
                parent: display(&pair.br.join("Left")),
                name: "LateChild".into(),
                source: None,
            },
        })
        .unwrap();
    // These are the same original install checkpoints the previous writer
    // permitted to coexist. Recovery must keep both original records and reject
    // continuing the older bad overlap after the guard is upgraded.
    pair.b
        .install_record(&pair.b.operations[&other.operation_id])
        .unwrap();
    pair.b
        .install_record(&pair.b.operations[&preview.operation_id])
        .unwrap();
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert!(recovery.allowed_actions.is_empty());
    for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
        assert!(
            pair.b
                .resolve_recovery(&preview.operation_id, action)
                .is_err()
        );
    }
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert!(!pair.br.join("Left/LateChild").exists());
    assert!(!pair.br.join("NewParent").exists());
    assert!(!pair.b.committed(&preview.operation_id));
    assert!(!pair.b.committed(&other.operation_id));
}

fn native_resource_parent() -> (Pair, String, String) {
    use weftext_export::exchange::{
        NativeEntry, NativeFileRole, read_native_package, write_native_package,
    };
    let temp = tempfile::tempdir().unwrap();
    let ar = temp.path().join("a");
    let br = temp.path().join("b");
    let bd = temp.path().join("b-private");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    let origin = temp.path().join("native-origin");
    a.create_library(&origin).unwrap();
    a.create_note(&origin, &origin, "NewParent").unwrap();
    let imported = a
        .prepare_import_resource(
            &origin,
            origin.join("NewParent/NewParent.typ"),
            "flat.bin",
            vec![0, 255, 7],
            &fresh(),
        )
        .unwrap();
    a.execute_operation(&imported.operation_id, &imported.plan_id)
        .unwrap();
    // This is a validated original package input, not an invented received
    // manifest or an external change to an already installed resource.
    let mut package = read_native_package(&a.export_native_package(&origin).unwrap()).unwrap();
    let source_resource = "NewParent/flat.bin";
    let target_resource = "NewParent/assets/icons/nested.bin";
    let resource_bytes = package.files.remove(source_resource).unwrap();
    let entry = package.manifest.entries.remove(source_resource).unwrap();
    package.files.insert(target_resource.into(), resource_bytes);
    package
        .manifest
        .entries
        .insert(target_resource.into(), entry);
    let manifest_path = "NewParent/weftext.resources.json";
    let mut document: serde_json::Value =
        serde_json::from_slice(&package.files[manifest_path]).unwrap();
    document["resources"][0]["path"] = "assets/icons/nested.bin".into();
    let bytes = serde_json::to_vec(&document).unwrap();
    let mut manifest_entry = NativeEntry::new(NativeFileRole::ResourceManifest, &bytes);
    manifest_entry.owner_node_id = package.manifest.entries[manifest_path]
        .owner_node_id
        .clone();
    manifest_entry.revision = Some(fresh());
    package
        .manifest
        .entries
        .insert(manifest_path.into(), manifest_entry);
    package.files.insert(manifest_path.into(), bytes);
    a.create_library(&ar).unwrap();
    let native = a
        .prepare_native_import(
            &ar,
            &ar,
            "OriginalBundle",
            write_native_package(&package).unwrap(),
            &fresh(),
            false,
        )
        .unwrap();
    let parent = native.node_map.values().next().unwrap().clone();
    let resource = native.resource_map.values().next().unwrap().clone();
    a.execute_operation(&native.operation_id, &native.plan_id)
        .unwrap();
    run(
        &mut a,
        &ar,
        OperationIntent::MoveNode {
            source: display(&ar.join("OriginalBundle/NewParent")),
            parent: display(&ar),
        },
    );
    let mut keys = BTreeSet::new();
    for name in ["Left", "Right", "Container", "Still"] {
        keys.insert(ObjectKey::Node(
            a.create_note(&ar, &ar, name).unwrap().node_id.unwrap(),
        ));
    }
    let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
    let mut b = Core::new(&bd).unwrap();
    b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
        .unwrap();
    run(
        &mut b,
        &br,
        OperationIntent::CloneSync {
            identity: identity.clone(),
            objects: keys.clone(),
        },
    );
    run(
        &mut a,
        &ar,
        OperationIntent::RenameNode {
            source: display(&ar.join("Left")),
            name: "新名".into(),
        },
    );
    run(
        &mut a,
        &ar,
        OperationIntent::MoveNode {
            source: display(&ar.join("Right")),
            parent: display(&ar.join("NewParent")),
        },
    );
    save(&mut a, &ar, "新名/新名.typ", "incoming left");
    save(&mut a, &ar, "NewParent/Right/Right.typ", "incoming right");
    a.publish_sync(&ar).unwrap();
    b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
        .unwrap();
    (
        Pair {
            temp,
            a,
            b,
            ar,
            br,
            bd,
            identity,
            keys,
        },
        parent,
        resource,
    )
}

#[test]
fn compound_native_new_parent_nested_resource_uses_original_portable_owner_and_new_physical_births()
{
    let (mut pair, parent, resource) = native_resource_parent();
    assert!(!pair.br.join("NewParent").exists());
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let resource_path = pair.br.join("NewParent").join("assets/icons/nested.bin");
    assert_eq!(preview.resource_map[&display(&resource_path)], resource);
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let owner = weftext_knowledge::NodeRef {
        workspace_id: pair.identity.workspace_id.clone(),
        node_id: parent,
    };
    let qualified = pair
        .b
        .qualified_resource_at(
            &pair.br,
            &pair.br.join("NewParent/NewParent.typ"),
            &owner,
            &resource,
            &resource_path,
            "live",
            Some(&preview.revisions[&display(&resource_path)]),
        )
        .unwrap();
    assert_eq!(fs::read(&resource_path).unwrap(), [0, 255, 7]);
    assert_eq!(qualified.content_hash, hash(&[0, 255, 7]));
    pair.b
        .validate_resource_witness(&pair.br, &qualified.witness)
        .unwrap();
    let collection = pair
        .b
        .qualified_resource_collection_at(
            &pair.br,
            &pair.br.join("NewParent/NewParent.typ"),
            &owner,
            "live",
        )
        .unwrap();
    assert_eq!(collection.entries.len(), 1);
    assert_eq!(
        collection.entries[0].relative_path,
        "assets/icons/nested.bin"
    );
    let born = pair
        .b
        .open(
            pair.br.join("NewParent/NewParent.typ"),
            Some(pair.br.clone()),
        )
        .unwrap();
    assert!(
        pair.b
            .list_history(&born.session_id)
            .unwrap()
            .versions
            .iter()
            .any(|version| version.operation_id == preview.operation_id)
    );
    assert_eq!(
        fs::read(pair.br.join("NewParent/Right/Right.typ")).unwrap(),
        fs::read(pair.ar.join("NewParent/Right/Right.typ")).unwrap()
    );
}

#[test]
fn compound_nested_created_parent_recovery_requires_each_original_directory_identity() {
    let (mut pair, _, _) = native_resource_parent();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    crate::operations::compound::set_compound_failpoint("directory:icons");
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    let path = pair.br.join("NewParent/assets/icons");
    let original = pair.temp.path().join("retained-original-icons");
    fs::rename(&path, &original).unwrap();
    fs::create_dir(&path).unwrap();
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "thirdState");
    assert!(recovery.allowed_actions.is_empty());
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert!(!pair.b.committed(&preview.operation_id));
}
