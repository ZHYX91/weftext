use super::*;
use crate::sync::{ObjectKey, SyncIdentity};

struct Fixture {
    temp: tempfile::TempDir,
    core: Core,
    root: PathBuf,
    data: PathBuf,
    reference: String,
    child: String,
    identity: SyncIdentity,
    objects: BTreeSet<ObjectKey>,
}
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    fs::create_dir(root.join("Folder")).unwrap();
    let directory = core
        .prepare_directory_admission(&root, root.join("Folder"), &fresh())
        .unwrap();
    core.execute_operation(&directory.preview.operation_id, &directory.preview.plan_id)
        .unwrap();
    let child = core
        .create_note_fixture(&root, root.join("Folder"), "Child")
        .unwrap()
        .node_id
        .unwrap();
    core.create_note_fixture(&root, &root, "Parent").unwrap();
    let info = core.enable_sync(&root, &fresh()).unwrap();
    Fixture {
        temp,
        core,
        root,
        data,
        reference: directory.target_ref,
        child,
        identity: info.identity,
        objects: info.installed_objects.into_iter().collect(),
    }
}
fn request(f: &Fixture) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::RelocateDirectory {
            source: display(&f.root.join("Folder")),
            parent: display(&f.root.join("Parent")),
            name: "Moved #[dir]".into(),
        },
    }
}
fn assert_directory(f: &Fixture) {
    let target = f
        .core
        .resolve_navigation_target(&f.root, &f.reference)
        .unwrap();
    assert_eq!(target.status, "managed");
    assert_eq!(
        target.path.as_deref().map(Path::new),
        Some(f.root.join("Parent/Moved #[dir]").as_path())
    );
    assert!(!f.root.join("Parent/Moved #[dir]/Moved #[dir].typ").exists());
    assert!(!f.root.join("Folder").exists());
}

#[test]
fn directory_relocation_producer_preserves_marker_revision_child_history_draft_and_late_raw() {
    let mut f = fixture();
    let doc = f
        .core
        .open(f.root.join("Folder/Child/Child.typ"), Some(f.root.clone()))
        .unwrap();
    let draft = format!("{}\nretained child draft", doc.source);
    f.core.persist_draft(&doc.session_id, &draft).unwrap();
    let history = f.core.list_history(&doc.session_id).unwrap().versions;
    let graph = f
        .core
        .read_sync_history(&f.identity)
        .unwrap()
        .objects()
        .unwrap();
    let directory_key = f
        .objects
        .iter()
        .find(|key| matches!(key, ObjectKey::Directory(_)))
        .unwrap()
        .clone();
    let before = graph[&directory_key].versions[0].state.clone();
    let req = request(&f);
    let preview = f.core.prepare_operation(req.clone()).unwrap();
    assert!(preview.node_map.is_empty());
    assert!(preview.new_node_id.is_none());
    assert_eq!(
        f.core.prepare_operation(req.clone()).unwrap().plan_id,
        preview.plan_id
    );
    fs::write(f.root.join("Folder/late.raw"), b"late original tree member").unwrap();
    let receipt = f
        .core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert!(receipt.node_ids.is_empty());
    assert_directory(&f);
    assert_eq!(
        fs::read(f.root.join("Parent/Moved #[dir]/late.raw")).unwrap(),
        b"late original tree member"
    );
    let opened = f
        .core
        .open(
            f.root.join("Parent/Moved #[dir]/Child/Child.typ"),
            Some(f.root.clone()),
        )
        .unwrap();
    assert_eq!(opened.node_id.as_deref(), Some(f.child.as_str()));
    assert_eq!(opened.recovered_draft.as_deref(), Some(draft.as_str()));
    assert_eq!(
        f.core
            .list_history(&opened.session_id)
            .unwrap()
            .versions
            .iter()
            .map(|v| &v.version_id)
            .collect::<Vec<_>>(),
        history.iter().map(|v| &v.version_id).collect::<Vec<_>>()
    );
    f.core.publish_sync(&f.root).unwrap();
    let graph = f
        .core
        .read_sync_history(&f.identity)
        .unwrap()
        .objects()
        .unwrap();
    let after = &graph[&directory_key].versions[0].state;
    assert_eq!(after.files, before.files);
    assert_eq!(after.file_revisions, before.file_revisions);
    assert_eq!(after.name, "Moved #[dir]");
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_directory(&f);
    assert_eq!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .plan_id,
        preview.plan_id
    );
}

#[test]
fn directory_relocation_producer_refuses_unadmitted_node_root_recursive_and_late_occupied_targets()
{
    let mut f = fixture();
    fs::create_dir(f.root.join("Copied")).unwrap();
    fs::copy(
        f.root.join("Folder/weftext.directory.json"),
        f.root.join("Copied/weftext.directory.json"),
    )
    .unwrap();
    for source in [f.root.clone(), f.root.join("Parent"), f.root.join("Copied")] {
        let mut req = request(&f);
        let OperationIntent::RelocateDirectory { source: input, .. } = &mut req.intent else {
            unreachable!()
        };
        *input = display(&source);
        assert!(f.core.prepare_operation(req).is_err());
    }
    // Keep the copied marker outside this workspace before checking the
    // independent recursive-target refusal on the valid original directory.
    fs::rename(
        f.root.join("Copied"),
        f.temp.path().join("retained-copied-marker"),
    )
    .unwrap();
    for slashes in [false, true] {
        for path in [f.root.join("Folder"), f.root.join("Folder").join("Child")] {
            let mut recursive = request(&f);
            let OperationIntent::RelocateDirectory { source, parent, .. } = &mut recursive.intent
            else {
                unreachable!()
            };
            *source = display(&f.root.join("Folder"));
            *parent = display(&path);
            if slashes {
                *source = source.replace('\\', "/");
                *parent = parent.replace('\\', "/");
            }
            assert_eq!(
                f.core
                    .prepare_operation(recursive.clone())
                    .unwrap_err()
                    .code,
                "recursiveMove"
            );
            assert!(
                !f.core
                    .journal_path(&recursive.operation_id, "install")
                    .exists()
            );
        }
    }
    let preview = f.core.prepare_operation(request(&f)).unwrap();
    fs::create_dir(f.root.join("Parent/Moved #[dir]")).unwrap();
    fs::write(
        f.root.join("Parent/Moved #[dir]/foreign.raw"),
        b"retain target occupant",
    )
    .unwrap();
    assert!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(
        !f.core
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(f.root.join("Folder/Child/Child.typ").exists());
    assert_eq!(
        fs::read(f.root.join("Parent/Moved #[dir]/foreign.raw")).unwrap(),
        b"retain target occupant"
    );
}

#[test]
fn directory_relocation_original_after_placement_can_be_adopted_without_new_ids() {
    let mut f = fixture();
    let preview = f.core.prepare_operation(request(&f)).unwrap();
    let plan = f.core.operations[&preview.operation_id].clone();
    f.core.install_record(&plan).unwrap();
    let namespace =
        NamespaceGuard::acquire(&f.root.join("Folder"), &f.root.join("Parent")).unwrap();
    namespace.move_source("Moved #[dir]").unwrap();
    drop(namespace);
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    let recovery = f.core.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(
        recovery.coherence,
        "knownAfter",
        "{}",
        serde_json::to_string(&recovery).unwrap()
    );
    assert!(
        recovery
            .allowed_actions
            .iter()
            .any(|action| action == "adoptAfterImages")
    );
    f.core
        .resolve_recovery(&preview.operation_id, "adoptAfterImages")
        .unwrap();
    assert_directory(&f);
}

#[test]
fn directory_relocation_sync_uses_original_directory_role_and_receiver_child_identity() {
    let mut f = fixture();
    let root = f.temp.path().join("receiver");
    let data = f.temp.path().join("receiver-private");
    let mut receiver = Core::new(&data).unwrap();
    receiver
        .receive_sync(
            &f.identity,
            f.core.export_sync_objects(&f.identity, &f.objects).unwrap(),
        )
        .unwrap();
    let clone = receiver
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::CloneSync {
                identity: f.identity.clone(),
                objects: f.objects.clone(),
            },
        })
        .unwrap();
    receiver
        .execute_operation(&clone.operation_id, &clone.plan_id)
        .unwrap();
    fs::write(
        root.join("Folder/personal.raw"),
        b"receiver original companion",
    )
    .unwrap();
    let original = capture_optional(&root.join("Folder/Child/Child.typ"))
        .unwrap()
        .unwrap()
        .stamp
        .identity;
    let preview = f.core.prepare_operation(request(&f)).unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    f.core.publish_sync(&f.root).unwrap();
    receiver
        .receive_sync(
            &f.identity,
            f.core.export_sync_objects(&f.identity, &f.objects).unwrap(),
        )
        .unwrap();
    let prepared = receiver
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::ApplySync {
                identity: f.identity.clone(),
                objects: f.objects.clone(),
            },
        })
        .unwrap();
    receiver
        .execute_operation(&prepared.operation_id, &prepared.plan_id)
        .unwrap();
    let target = root.join("Parent/Moved #[dir]");
    assert_eq!(
        capture_optional(&target.join("Child/Child.typ"))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        original
    );
    assert_eq!(
        fs::read(target.join("personal.raw")).unwrap(),
        b"receiver original companion"
    );
    assert_eq!(
        receiver
            .resolve_navigation_target(&root, &f.reference)
            .unwrap()
            .status,
        "managed"
    );
    let opened = receiver
        .open(target.join("Child/Child.typ"), Some(root.clone()))
        .unwrap();
    assert_eq!(opened.node_id.as_deref(), Some(f.child.as_str()));
    let owner = weftext_knowledge::NodeRef {
        workspace_id: f.identity.workspace_id.clone(),
        node_id: f.child,
    };
    let qualified = receiver
        .qualified_query_node_at(&root, &target.join("Child/Child.typ"), &owner, "live")
        .unwrap();
    receiver
        .validate_query_node_witness(&root, &qualified.witness)
        .unwrap();
}

fn prepared_child_operation(f: &mut Fixture, kind: &str) -> OperationPreview {
    let intent = match kind {
        "update" => {
            let path = f.root.join("Folder/Child/Child.typ");
            let snapshot = f
                .core
                .read_snapshot(&f.root, &[display(&path)])
                .unwrap()
                .files
                .remove(0);
            let before = snapshot.bytes.unwrap();
            let mut after = before.clone();
            after.extend_from_slice(b"\npending original child update");
            OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: display(&path),
                    base_id: Some(snapshot.base_id),
                    before: Some(before),
                    after,
                }],
            }
        }
        "copyRead" => {
            // Use the actual Core-resolved Node locator so this checkpoint
            // reaches the valid original Copy reader, independent of string
            // spelling compatibility in the Copy transfer planner.
            let reference = format!("weftext-node://{}/{}", f.identity.workspace_id, f.child);
            let resolved = f
                .core
                .resolve_navigation_target(&f.root, &reference)
                .unwrap();
            assert_eq!(resolved.status, "managed");
            OperationIntent::CopyNode {
                source: resolved.path.unwrap(),
                parent: display(&f.root),
                name: "IndependentCopy".into(),
                allow_losses: false,
            }
        }
        "absentChild" => OperationIntent::NewNode {
            parent: display(&f.root.join("Folder")),
            name: "LateChild".into(),
            source: None,
        },
        _ => unreachable!(),
    };
    f.core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent,
        })
        .unwrap()
}

fn pending_child_refuses_directory_relocation(recovery: bool) {
    for kind in ["update", "copyRead", "absentChild"] {
        let mut f = fixture();
        let before = fs::read(f.root.join("Folder/Child/Child.typ")).unwrap();
        let original = f.core.prepare_operation(request(&f)).unwrap();
        let other = prepared_child_operation(&mut f, kind);
        f.core
            .install_record(&f.core.operations[&other.operation_id])
            .unwrap();
        let mut retained = [&original.operation_id, &other.operation_id]
            .into_iter()
            .map(|operation| {
                let path = f.core.journal_path(operation, "plan");
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect::<Vec<_>>();
        let other_install = f.core.journal_path(&other.operation_id, "install");
        retained.push((other_install.clone(), fs::read(other_install).unwrap()));
        if recovery {
            // Retain both original install-only checkpoints. Recovery must
            // refuse this coexistence instead of inventing either writer's
            // absent effects or requalifying its source at another address.
            f.core
                .install_record(&f.core.operations[&original.operation_id])
                .unwrap();
            let original_install = f.core.journal_path(&original.operation_id, "install");
            retained.push((
                original_install.clone(),
                fs::read(original_install).unwrap(),
            ));
            drop(f.core);
            f.core = Core::new(&f.data).unwrap();
            let state = f.core.inspect_recovery(&original.operation_id).unwrap();
            assert!(state.allowed_actions.is_empty(), "{kind}");
            for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
                assert!(
                    f.core
                        .resolve_recovery(&original.operation_id, action)
                        .is_err(),
                    "{kind}: {action}"
                );
            }
        } else {
            assert_eq!(
                f.core
                    .execute_operation(&original.operation_id, &original.plan_id)
                    .unwrap_err()
                    .code,
                "recoveryPending",
                "{kind}"
            );
            assert!(
                !f.core
                    .journal_path(&original.operation_id, "install")
                    .exists(),
                "{kind}"
            );
        }
        assert_eq!(
            fs::read(f.root.join("Folder/Child/Child.typ")).unwrap(),
            before,
            "{kind}"
        );
        assert!(!f.root.join("Parent/Moved #[dir]").exists(), "{kind}");
        assert!(!f.root.join("Folder/LateChild").exists(), "{kind}");
        assert!(!f.root.join("IndependentCopy").exists(), "{kind}");
        assert!(!f.core.committed(&original.operation_id));
        assert!(!f.core.committed(&other.operation_id));
        for (path, bytes) in retained {
            assert_eq!(
                fs::read(path).unwrap(),
                bytes,
                "{kind}: original journal changed"
            );
        }
    }
}

#[test]
fn directory_relocation_late_pending_child_read_write_or_absent_birth_prevents_first_effect() {
    pending_child_refuses_directory_relocation(false);
}

#[test]
fn directory_relocation_recovery_retains_original_pending_child_read_write_or_absent_birth() {
    pending_child_refuses_directory_relocation(true);
}

#[test]
fn directory_relocation_same_bytes_marker_replacement_after_original_move_refuses_recovery() {
    let mut f = fixture();
    let preview = f.core.prepare_operation(request(&f)).unwrap();
    f.core
        .install_record(&f.core.operations[&preview.operation_id])
        .unwrap();
    let namespace =
        NamespaceGuard::acquire(&f.root.join("Folder"), &f.root.join("Parent")).unwrap();
    namespace.move_source("Moved #[dir]").unwrap();
    drop(namespace);
    let marker = f.root.join("Parent/Moved #[dir]/weftext.directory.json");
    let before = fs::read(&marker).unwrap();
    fs::rename(&marker, f.temp.path().join("retained-original-marker.json")).unwrap();
    fs::write(&marker, &before).unwrap();
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    let state = f.core.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(state.coherence, "thirdState");
    assert!(state.allowed_actions.is_empty());
    for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
        assert!(
            f.core
                .resolve_recovery(&preview.operation_id, action)
                .is_err()
        );
    }
    assert_eq!(fs::read(marker).unwrap(), before);
    assert!(!f.root.join("Folder").exists());
    assert!(f.root.join("Parent/Moved #[dir]/Child/Child.typ").exists());
    assert!(!f.core.committed(&preview.operation_id));
}

#[test]
fn directory_relocation_node_guard_rejects_both_separator_recursions_and_allows_adjacent_prefix() {
    let mut f = fixture();
    let source = f.root.join("Folder").join("Child");
    f.core
        .create_note_fixture(&f.root, &source, "Grand")
        .unwrap();
    f.core
        .create_note_fixture(&f.root, f.root.join("Folder"), "ChildNext")
        .unwrap();
    let canonical = source.join("Child.typ");
    let before = capture_optional(&canonical).unwrap().unwrap();
    for slashes in [false, true] {
        for parent in [&source, &source.join("Grand")] {
            let mut input = display(&source);
            let mut target_parent = display(parent);
            if slashes {
                input = input.replace('\\', "/");
                target_parent = target_parent.replace('\\', "/");
            }
            let operation = fresh();
            let error = f
                .core
                .prepare_operation(OperationRequest {
                    operation_id: operation.clone(),
                    root: display(&f.root),
                    intent: OperationIntent::MoveNode {
                        source: input,
                        parent: target_parent,
                    },
                })
                .unwrap_err();
            assert_eq!(error.code, "recursiveMove");
            assert!(!f.core.journal_path(&operation, "install").exists());
            assert_eq!(fs::read(&canonical).unwrap(), before.bytes);
        }
    }
    let preview = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::MoveNode {
                source: display(&source),
                parent: display(&f.root.join("Folder").join("ChildNext")),
            },
        })
        .unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let moved = f.root.join("Folder/ChildNext/Child/Child.typ");
    assert_eq!(
        capture_optional(&moved).unwrap().unwrap().stamp.identity,
        before.stamp.identity
    );
    assert_eq!(fs::read(&moved).unwrap(), before.bytes);
    assert!(
        f.root
            .join("Folder/ChildNext/Child/Grand/Grand.typ")
            .exists()
    );
    assert!(!source.exists());
}

#[test]
fn directory_relocation_ordinal_unicode_descendants_refuse_and_adjacent_prefix_is_valid() {
    let mut f = fixture();
    let source = f.root.join("Ä");
    fs::create_dir(&source).unwrap();
    let admitted = f
        .core
        .prepare_directory_admission(&f.root, &source, &fresh())
        .unwrap();
    f.core
        .execute_operation(&admitted.preview.operation_id, &admitted.preview.plan_id)
        .unwrap();
    f.core
        .create_note_fixture(&f.root, &source, "Inside")
        .unwrap();
    f.core
        .create_note_fixture(&f.root, &f.root, "äNeighbor")
        .unwrap();
    let alias = f.root.join("ä");
    let identity = ParentGuard::acquire(&source.join("weftext.directory.json"))
        .unwrap()
        .directory_identity()
        .unwrap();
    assert_eq!(
        ParentGuard::acquire(&alias.join("weftext.directory.json"))
            .unwrap()
            .directory_identity()
            .unwrap(),
        identity,
        "The actual NTFS namespace must prove this ordinal alias"
    );
    let marker = fs::read(source.join("weftext.directory.json")).unwrap();
    for slashes in [false, true] {
        for parent in [alias.clone(), alias.join("Inside")] {
            let mut input = display(&source);
            let mut target_parent = display(&parent);
            if slashes {
                input = input.replace('\\', "/");
                target_parent = target_parent.replace('\\', "/");
            }
            let operation = fresh();
            let error = f
                .core
                .prepare_operation(OperationRequest {
                    operation_id: operation.clone(),
                    root: display(&f.root),
                    intent: OperationIntent::RelocateDirectory {
                        source: input,
                        parent: target_parent,
                        name: "UnicodeMoved".into(),
                    },
                })
                .unwrap_err();
            assert_eq!(error.code, "recursiveMove");
            assert!(!f.core.journal_path(&operation, "install").exists());
            assert_eq!(
                fs::read(source.join("weftext.directory.json")).unwrap(),
                marker
            );
        }
    }
    let preview = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::RelocateDirectory {
                source: display(&source),
                parent: display(&f.root.join("äNeighbor")),
                name: "UnicodeMoved".into(),
            },
        })
        .unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let moved = f.root.join("äNeighbor/UnicodeMoved");
    assert_eq!(
        ParentGuard::acquire(&moved.join("weftext.directory.json"))
            .unwrap()
            .directory_identity()
            .unwrap(),
        identity
    );
    assert_eq!(
        fs::read(moved.join("weftext.directory.json")).unwrap(),
        marker
    );
    assert_eq!(
        f.core
            .resolve_navigation_target(&f.root, &admitted.target_ref)
            .unwrap()
            .path
            .as_deref()
            .map(Path::new),
        Some(moved.as_path())
    );
    assert!(!source.exists());
}
