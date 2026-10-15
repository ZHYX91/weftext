use super::*;
use crate::ConfigRole;

fn missing_configuration(extra: bool) -> Pair {
    let mut pair = fixture_with_child(false);
    assert!(!pair.br.join(".weftext-rules").exists());
    if extra {
        for (role, bytes) in [
            (
                ConfigRole::Schema,
                include_bytes!("../../../../docs/design/examples/schema.authoring.json").to_vec(),
            ),
            (
                ConfigRole::WorkspaceSettings,
                br#"{"version":1,"libraryStyle":{"enabled":false}}"#.to_vec(),
            ),
        ] {
            let before = pair.a.read_config(&pair.ar, role).unwrap();
            let preview = pair
                .a
                .prepare_config_save(&pair.ar, role, before.bytes, bytes, &fresh())
                .unwrap();
            pair.a
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
        }
    }
    // Match the real HTTP producer: preserve the old resource's FI, explicitly
    // adopt its observed external bytes with the complete installed object cut.
    pair.a.publish_sync(&pair.ar).unwrap();
    let all = pair
        .a
        .sync_binding_info(&pair.ar)
        .unwrap()
        .installed_objects
        .into_iter()
        .collect();
    let resource = pair.ar.join("新名/baseline.bin");
    fs::write(&resource, [4, 7, 255]).unwrap();
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
    pair
}

#[test]
fn compound_new_configuration_and_two_moves_share_original_plan_and_portable_versions() {
    let mut pair = missing_configuration(true);
    let req = request(&pair);
    let preview = pair.b.prepare_operation(req.clone()).unwrap();
    let transaction = pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    let crate::operations::Mutation::Compound { created_files, .. } = &transaction.mutation else {
        unreachable!()
    };
    assert_eq!(created_files.len(), 3);
    assert!(created_files.iter().all(|change| change.before.is_none()));
    for role in [
        ConfigRole::Rules,
        ConfigRole::Schema,
        ConfigRole::WorkspaceSettings,
    ] {
        assert!(pair.b.read_config(&pair.br, role).unwrap().bytes.is_none());
    }
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    assert_eq!(
        pair.b.prepare_operation(req).unwrap().plan_id,
        preview.plan_id
    );
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_installed(&mut pair, &preview.operation_id);
    let graph = pair
        .a
        .read_sync_history(&pair.identity)
        .unwrap()
        .objects()
        .unwrap();
    for (role, object, leaf) in [
        (ConfigRole::Rules, ObjectKey::Rules, ".weftext-rules"),
        (
            ConfigRole::Schema,
            ObjectKey::Schema,
            ".weftext-schema.json",
        ),
        (
            ConfigRole::WorkspaceSettings,
            ObjectKey::Settings,
            ".weftext-settings.json",
        ),
    ] {
        let author = pair.a.read_config(&pair.ar, role).unwrap();
        let received = pair.b.read_config(&pair.br, role).unwrap();
        assert_eq!(received.bytes, author.bytes);
        assert_eq!(graph[&object].versions.len(), 1);
        let expected = &graph[&object].versions[0].state.file_revisions[leaf];
        assert_eq!(received.revision.as_ref(), Some(expected));
        assert_eq!(preview.revisions[&display(&pair.br.join(leaf))], *expected);
        if let Some(revision) = author.revision {
            assert_eq!(revision, *expected);
        }
    }
    assert!(pair.b.query_rules_ever_committed(&pair.br).unwrap());
    assert_eq!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .plan_id,
        preview.plan_id
    );
}

#[test]
fn compound_new_rules_slot_late_occupancy_keeps_all_original_sources_and_no_first_effect() {
    let mut pair = missing_configuration(false);
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let before_left = fs::read(pair.br.join("Left/Left.typ")).unwrap();
    let before_right = fs::read(pair.br.join("Right/Right.typ")).unwrap();
    let bytes = fs::read(pair.ar.join(".weftext-rules")).unwrap();
    fs::write(pair.br.join(".weftext-rules"), &bytes).unwrap();
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
    assert!(pair.br.join("Left").exists());
    assert!(pair.br.join("Right").exists());
    assert_eq!(
        fs::read(pair.br.join("Left/Left.typ")).unwrap(),
        before_left
    );
    assert_eq!(
        fs::read(pair.br.join("Right/Right.typ")).unwrap(),
        before_right
    );
    assert_eq!(fs::read(pair.br.join(".weftext-rules")).unwrap(), bytes);
    let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "thirdState");
    assert!(recovery.allowed_actions.is_empty());
}

#[test]
fn compound_missing_rules_and_original_materialized_rules_resume_after_restart() {
    for point in ["move0", "create:.weftext-rules"] {
        let mut pair = missing_configuration(false);
        let preview = pair.b.prepare_operation(request(&pair)).unwrap();
        let revision = preview.revisions[&display(&pair.br.join(".weftext-rules"))].clone();
        crate::operations::compound::set_compound_failpoint(point);
        assert!(
            pair.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err()
        );
        drop(pair.b);
        pair.b = Core::new(&pair.bd).unwrap();
        let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        let action = if info.coherence == "knownAfter" {
            "adoptAfterImages"
        } else {
            "resumeOriginalPlan"
        };
        assert!(
            info.allowed_actions.iter().any(|allowed| allowed == action),
            "{}",
            serde_json::to_string(&info).unwrap()
        );
        pair.b
            .resolve_recovery(&preview.operation_id, action)
            .unwrap();
        assert_installed(&mut pair, &preview.operation_id);
        assert_eq!(
            pair.b
                .read_config(&pair.br, ConfigRole::Rules)
                .unwrap()
                .revision
                .as_ref(),
            Some(&revision)
        );
        assert!(pair.b.query_rules_ever_committed(&pair.br).unwrap());
    }
}

#[test]
fn compound_materialized_rules_same_bytes_replacement_does_not_inherit_admission() {
    let mut pair = missing_configuration(false);
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    crate::operations::compound::set_compound_failpoint("create:.weftext-rules");
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    let path = pair.br.join(".weftext-rules");
    let original = pair.temp.path().join("retained-original-rules");
    fs::rename(&path, &original).unwrap();
    fs::copy(&original, &path).unwrap();
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    let recovery = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "thirdState");
    assert!(recovery.allowed_actions.is_empty());
    for action in ["keepCurrent", "adoptAfterImages", "resumeOriginalPlan"] {
        assert!(
            pair.b
                .resolve_recovery(&preview.operation_id, action)
                .is_err()
        );
    }
    assert!(!pair.b.committed(&preview.operation_id));
    assert_eq!(fs::read(&path).unwrap(), fs::read(&original).unwrap());
}

#[test]
fn compound_installed_new_rules_cannot_be_deleted_and_reclassified_as_legacy_absence() {
    let mut pair = missing_configuration(false);
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    fs::rename(
        pair.br.join(".weftext-rules"),
        pair.temp.path().join("retained-rules-after-commit"),
    )
    .unwrap();
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    assert_eq!(
        pair.b
            .read_config(&pair.br, ConfigRole::Rules)
            .unwrap_err()
            .code,
        "rulesUnavailable"
    );
    assert!(!pair.br.join(".weftext-rules").exists());
}

fn installing_rules_save(pair: &mut Pair) {
    let config = pair.b.read_config(&pair.br, ConfigRole::Rules).unwrap();
    assert!(config.bytes.is_none());
    let preview = pair
        .b
        .prepare_config_save(
            &pair.br,
            ConfigRole::Rules,
            None,
            fs::read(pair.ar.join(".weftext-rules")).unwrap(),
            &fresh(),
        )
        .unwrap();
    // The original Core install checkpoint owns its still-absent destination;
    // no materialized file or before/after bytes can cancel that fence.
    let plan = pair.b.operations[&preview.operation_id].clone();
    pair.b.install_record(&plan).unwrap();
    assert!(!pair.br.join(".weftext-rules").exists());
}

#[test]
fn compound_configuration_pending_absent_slot_blocks_prepare_without_any_effect() {
    let mut pair = missing_configuration(false);
    installing_rules_save(&mut pair);
    let before = fs::read(pair.br.join("Left/Left.typ")).unwrap();
    assert_eq!(
        pair.b.prepare_operation(request(&pair)).unwrap_err().code,
        "recoveryPending"
    );
    assert!(pair.br.join("Left").exists());
    assert!(pair.br.join("Right").exists());
    assert!(!pair.br.join(".weftext-rules").exists());
    assert_eq!(fs::read(pair.br.join("Left/Left.typ")).unwrap(), before);
}

#[test]
fn compound_configuration_pending_arriving_after_prepare_blocks_first_effect() {
    let mut pair = missing_configuration(false);
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    installing_rules_save(&mut pair);
    let before = fs::read(pair.br.join("Right/Right.typ")).unwrap();
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
    assert!(pair.br.join("Left").exists());
    assert!(pair.br.join("Right").exists());
    assert!(!pair.br.join(".weftext-rules").exists());
    assert_eq!(fs::read(pair.br.join("Right/Right.typ")).unwrap(), before);
}
