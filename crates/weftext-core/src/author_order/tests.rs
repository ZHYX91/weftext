use super::*;

fn fixture() -> (tempfile::TempDir, Core, PathBuf, PathBuf, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let prepared = core
        .prepare_directory_admission(&root, &root, &fresh())
        .unwrap();
    core.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
        .unwrap();
    (temp, core, root, data, prepared.target_ref)
}

fn commit(core: &mut Core, preview: &OperationPreview) {
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}

#[test]
fn author_order_preserves_settings_and_replays_exact_original_after_restart() {
    let (_temp, mut core, root, data, parent) = fixture();
    core.create_note_fixture(&root, &root, "Alpha").unwrap();
    core.create_note_fixture(&root, &root, "Beta").unwrap();
    let before = core
        .read_config(&root, ConfigRole::WorkspaceSettings)
        .unwrap();
    let initial = core
        .prepare_config_save(
            &root,
            ConfigRole::WorkspaceSettings,
            before.bytes,
            br#"{"version":1,"processing":{"fixture":true},"custom":"preserved"}"#.to_vec(),
            &fresh(),
        )
        .unwrap();
    commit(&mut core, &initial);
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    assert!(snapshot.complete);
    assert_eq!(
        snapshot
            .members
            .iter()
            .map(|member| member.name.as_str())
            .collect::<Vec<_>>(),
        ["Alpha", "Beta"]
    );
    let children = snapshot.ordered_refs.into_iter().rev().collect::<Vec<_>>();
    let id = fresh();
    let preview = core
        .prepare_navigation_reorder(&root, &parent, &children, &snapshot.base_id, &id)
        .unwrap();
    drop(core);
    core = Core::new(&data).unwrap();
    assert_eq!(
        core.prepare_navigation_reorder(&root, &parent, &children, &snapshot.base_id, &id)
            .unwrap()
            .plan_id,
        preview.plan_id
    );
    assert_eq!(
        core.prepare_navigation_reorder(&root, &parent, &children[..1], &snapshot.base_id, &id)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    commit(&mut core, &preview);
    let saved = core.read_navigation_order(&root, &parent).unwrap();
    assert_eq!(saved.ordered_refs, children);
    let value: serde_json::Value =
        serde_json::from_str(saved.before_source.as_deref().unwrap()).unwrap();
    assert_eq!(value["custom"], "preserved");
    assert_eq!(value["processing"]["fixture"], true);
}

#[test]
fn exact_membership_and_generic_updates_cannot_omit_or_forge_siblings() {
    let (_temp, mut core, root, _data, parent) = fixture();
    core.create_note_fixture(&root, &root, "Alpha").unwrap();
    core.create_note_fixture(&root, &root, "Beta").unwrap();
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    assert!(
        core.prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs[..1],
            &snapshot.base_id,
            &fresh()
        )
        .is_err()
    );
    let forged = format!(
        "weftext-node://{}/{}",
        core.require_workspace(&root).unwrap().workspace_id,
        fresh()
    );
    let after = serde_json::to_vec(
        &serde_json::json!({"version":1,"navigationOrders":{parent.clone():[forged]}}),
    )
    .unwrap();
    let before = core
        .read_config(&root, ConfigRole::WorkspaceSettings)
        .unwrap();
    assert!(
        core.prepare_config_save(
            &root,
            ConfigRole::WorkspaceSettings,
            before.bytes,
            after,
            &fresh()
        )
        .is_err()
    );
    let duplicate = vec![
        snapshot.ordered_refs[0].clone(),
        snapshot.ordered_refs[0].clone(),
    ];
    assert!(
        core.prepare_navigation_reorder(&root, &parent, &duplicate, &snapshot.base_id, &fresh())
            .is_err()
    );
}

#[test]
fn late_sibling_or_replaced_physical_source_prevents_commit() {
    let (temp, mut core, root, _data, parent) = fixture();
    core.create_note_fixture(&root, &root, "Alpha").unwrap();
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    let preview = core
        .prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs,
            &snapshot.base_id,
            &fresh(),
        )
        .unwrap();
    core.create_note_fixture(&root, &root, "Late").unwrap();
    assert!(
        core.execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(!root.join(SETTINGS).exists());
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    let preview = core
        .prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs,
            &snapshot.base_id,
            &fresh(),
        )
        .unwrap();
    let source = root.join("Alpha/Alpha.typ");
    let bytes = fs::read(&source).unwrap();
    fs::rename(&source, temp.path().join("original.typ")).unwrap();
    fs::write(&source, bytes).unwrap();
    assert!(
        core.execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(!root.join(SETTINGS).exists());
}

#[test]
fn moved_children_drop_from_old_order_and_new_children_append_without_path_substitution() {
    let (_temp, mut core, root, _data, parent) = fixture();
    core.create_note_fixture(&root, &root, "Alpha").unwrap();
    core.create_note_fixture(&root, &root, "Beta").unwrap();
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    let children = snapshot
        .ordered_refs
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<_>>();
    let preview = core
        .prepare_navigation_reorder(&root, &parent, &children, &snapshot.base_id, &fresh())
        .unwrap();
    commit(&mut core, &preview);
    let container = root.join("Container");
    fs::create_dir(&container).unwrap();
    let prepared = core
        .prepare_directory_admission(&root, &container, &fresh())
        .unwrap();
    commit(&mut core, &prepared.preview);
    let moved = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::MoveNode {
                source: display(&root.join("Beta")),
                parent: display(&container),
            },
        })
        .unwrap();
    commit(&mut core, &moved);
    core.create_note_fixture(&root, &root, "Beta").unwrap();
    let after = core.read_navigation_order(&root, &parent).unwrap();
    assert!(!after.ordered_refs.contains(&children[0]));
    assert_eq!(after.ordered_refs[0], children[1]);
    let moved = core
        .read_navigation_order(&root, &prepared.target_ref)
        .unwrap();
    assert_eq!(moved.ordered_refs, [children[0].clone()]);
}

#[test]
fn unadmitted_containers_are_incomplete_and_cancel_is_durable() {
    let (_temp, mut core, root, data, parent) = fixture();
    fs::create_dir(root.join("Unknown")).unwrap();
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    assert!(!snapshot.complete);
    assert!(
        core.prepare_navigation_reorder(&root, &parent, &[], &snapshot.base_id, &fresh())
            .is_err()
    );
    let directory = core
        .prepare_directory_admission(&root, root.join("Unknown"), &fresh())
        .unwrap();
    commit(&mut core, &directory.preview);
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    let preview = core
        .prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs,
            &snapshot.base_id,
            &fresh(),
        )
        .unwrap();
    assert_eq!(
        core.cancel_navigation_reorder(&root, &preview.operation_id, &preview.plan_id)
            .unwrap()
            .status,
        "resolved"
    );
    drop(core);
    let mut core = Core::new(data).unwrap();
    assert_eq!(
        core.prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs,
            &snapshot.base_id,
            &preview.operation_id
        )
        .unwrap()
        .status,
        "resolved"
    );
    assert!(
        core.execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(!root.join(SETTINGS).exists());
}

#[test]
fn original_created_settings_after_image_recovers_without_treating_it_as_a_new_sibling() {
    let (_temp, mut core, root, data, parent) = fixture();
    core.create_note_fixture(&root, &root, "Alpha").unwrap();
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    let preview = core
        .prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs,
            &snapshot.base_id,
            &fresh(),
        )
        .unwrap();
    let transaction = core.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    let OperationIntent::UpdateFiles { changes } = &transaction.request.intent else {
        unreachable!()
    };
    let after = changes[0].after.clone();
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &core.journal_path(&preview.operation_id, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    let guard = ParentGuard::acquire(
        &crate::BackendContext::local().unwrap(),
        &root.join(SETTINGS),
    )
    .unwrap();
    let mut file = guard.open_file(true, true).unwrap();
    core.record_materialized(&preview.operation_id, &root.join(SETTINGS), &file)
        .unwrap();
    operations::write_blob(&mut file, &after).unwrap();
    drop(file);
    drop(guard);
    drop(core);
    let mut core = Core::new(data).unwrap();
    let recovery = core.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "knownAfter", "{recovery:?}");
    assert!(
        recovery
            .allowed_actions
            .iter()
            .any(|action| action == "adoptAfterImages")
    );
    core.resolve_recovery(&preview.operation_id, "adoptAfterImages")
        .unwrap();
    assert_eq!(
        core.read_navigation_order(&root, &parent)
            .unwrap()
            .ordered_refs,
        snapshot.ordered_refs
    );
    assert_eq!(fs::read(root.join(SETTINGS)).unwrap(), after);
}

#[test]
fn closed_order_codec_rejects_foreign_duplicate_and_non_identity_members() {
    let workspace = fresh();
    let parent = format!("weftext-directory://{workspace}/{}", fresh());
    let child = format!("weftext-node://{workspace}/{}", fresh());
    for children in [
        serde_json::json!([child, child]),
        serde_json::json!(["same-name/path"]),
        serde_json::json!([parent]),
        serde_json::json!([format!("weftext-node://{}/{}", fresh(), fresh())]),
    ] {
        assert!(
            validate_workspace_settings(
                &serde_json::to_vec(
                    &serde_json::json!({"version":1,"navigationOrders":{parent.clone():children}})
                )
                .unwrap()
            )
            .is_err()
        );
    }
    let value = serde_json::json!({"version":1,"extension":{"unknown":"retained"},"navigationOrders":{parent:[child]}});
    assert_eq!(
        validate_workspace_settings(&serde_json::to_vec(&value).unwrap()).unwrap(),
        value
    );
}

#[test]
fn config_read_preserves_invalid_settings_without_granting_navigation_or_write_validity() {
    let (_temp, mut core, root, _data, parent) = fixture();
    let path = root.join(SETTINGS);
    let foreign_parent = format!("weftext-directory://{}/{}", fresh(), fresh());
    let foreign_orders = serde_json::to_vec(
        &serde_json::json!({"version":1,"navigationOrders":{foreign_parent:[]}}),
    )
    .unwrap();
    for source in [
        b"not json".as_slice(),
        b"[]",
        br#"{"version":2}"#,
        br#"{"version":1,"navigationOrders":[]}"#,
        &foreign_orders,
    ] {
        fs::write(&path, source).unwrap();
        let config = core
            .read_config(&root, ConfigRole::WorkspaceSettings)
            .unwrap();
        assert_eq!(config.bytes.as_deref(), Some(source));
        assert_eq!(Path::new(&config.path), path);
        core.validate_snapshot(&root, &path, &config.base_id)
            .unwrap();
        let before_navigation = core.snapshots.len();
        assert!(core.read_navigation_order(&root, &parent).is_err());
        assert_eq!(core.snapshots.len(), before_navigation);
        core.validate_snapshot(&root, &path, &config.base_id)
            .unwrap();
        assert!(
            core.prepare_config_save(
                &root,
                ConfigRole::WorkspaceSettings,
                config.bytes,
                source.to_vec(),
                &fresh(),
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), source);
        core.validate_snapshot(&root, &path, &config.base_id)
            .unwrap();
    }
    let valid = br#"{"version":1,"navigationOrders":{}}"#;
    fs::write(&path, valid).unwrap();
    let config = core
        .read_config(&root, ConfigRole::WorkspaceSettings)
        .unwrap();
    let before_navigation = core.snapshots.len();
    let navigation = core.read_navigation_order(&root, &parent).unwrap();
    assert_eq!(core.snapshots.len(), before_navigation + 1);
    assert_ne!(navigation.base_id, config.base_id);
    assert_eq!(
        navigation.before_source.as_deref().unwrap().as_bytes(),
        valid
    );
    core.validate_snapshot(&root, &path, &config.base_id)
        .unwrap();
    core.validate_snapshot(&root, &path, &navigation.base_id)
        .unwrap();
}

#[test]
fn excluded_physical_child_does_not_become_a_complete_subset_reorder() {
    let (_temp, mut core, root, _data, parent) = fixture();
    core.create_note_fixture(&root, &root, "Visible").unwrap();
    core.create_note_fixture(&root, &root, "Excluded").unwrap();
    let rules = core.read_config(&root, ConfigRole::Rules).unwrap();
    let preview = core
        .prepare_config_save(
            &root,
            ConfigRole::Rules,
            rules.bytes,
            br#"{"version":1,"ignored":["Excluded"],"unmanaged":[]}"#.to_vec(),
            &fresh(),
        )
        .unwrap();
    commit(&mut core, &preview);
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    assert!(!snapshot.complete);
    assert_eq!(snapshot.members.len(), 1);
    assert_eq!(snapshot.members[0].name, "Visible");
    assert!(
        core.prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs,
            &snapshot.base_id,
            &fresh()
        )
        .is_err()
    );
}

#[test]
fn author_order_uses_whole_settings_cas_and_preserves_a_concurrent_extension_change() {
    let (_temp, mut core, root, _data, parent) = fixture();
    core.create_note_fixture(&root, &root, "Alpha").unwrap();
    let first = core
        .prepare_config_save(
            &root,
            ConfigRole::WorkspaceSettings,
            None,
            br#"{"version":1,"extension":"original"}"#.to_vec(),
            &fresh(),
        )
        .unwrap();
    commit(&mut core, &first);
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    let order = core
        .prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs,
            &snapshot.base_id,
            &fresh(),
        )
        .unwrap();
    let current = core
        .read_config(&root, ConfigRole::WorkspaceSettings)
        .unwrap();
    let concurrent = core
        .prepare_config_save(
            &root,
            ConfigRole::WorkspaceSettings,
            current.bytes,
            br#"{"version":1,"extension":"concurrent"}"#.to_vec(),
            &fresh(),
        )
        .unwrap();
    commit(&mut core, &concurrent);
    assert!(
        core.execute_operation(&order.operation_id, &order.plan_id)
            .is_err()
    );
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(SETTINGS)).unwrap()).unwrap();
    assert_eq!(saved["extension"], "concurrent");
    assert!(saved.get("navigationOrders").is_none());
}

#[test]
fn missing_previously_committed_settings_are_not_recreated_from_defaults() {
    let (temp, mut core, root, _data, parent) = fixture();
    core.create_note_fixture(&root, &root, "Alpha").unwrap();
    let snapshot = core.read_navigation_order(&root, &parent).unwrap();
    let order = core
        .prepare_navigation_reorder(
            &root,
            &parent,
            &snapshot.ordered_refs,
            &snapshot.base_id,
            &fresh(),
        )
        .unwrap();
    commit(&mut core, &order);
    fs::rename(
        root.join(SETTINGS),
        temp.path().join("retained-settings.json"),
    )
    .unwrap();
    assert_eq!(
        core.read_navigation_order(&root, &parent).unwrap_err().code,
        "settingsUnavailable"
    );
    let current = core
        .read_config(&root, ConfigRole::WorkspaceSettings)
        .unwrap();
    let replacement = serde_json::to_vec(
        &serde_json::json!({"version":1,"navigationOrders":{parent:snapshot.ordered_refs}}),
    )
    .unwrap();
    assert_eq!(
        core.prepare_config_save(
            &root,
            ConfigRole::WorkspaceSettings,
            current.bytes,
            replacement,
            &fresh()
        )
        .unwrap_err()
        .code,
        "settingsUnavailable"
    );
    assert!(!root.join(SETTINGS).exists());
}
