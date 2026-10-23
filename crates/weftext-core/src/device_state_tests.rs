use super::*;
use crate::device_state::{DEVICE_STATE_ROOT, DeviceRecordKind};

#[test]
fn device_identity_and_three_independent_preferences_survive_core_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("state");
    let mut core = Core::new(&data).unwrap();
    let initial = core.desktop_preferences().unwrap();
    assert!(initial.device.is_current);
    assert_eq!(initial.revision, 0);
    assert_eq!(initial.preferences, DesktopPreferences::default());
    let identity = initial.device.id;
    let first = core
        .update_desktop_preferences(
            0,
            DesktopPreferences {
                resident: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(first.revision, 1);
    assert!(!first.preferences.login_startup_requested);
    assert!(!first.preferences.open_library_on_launch);
    let second = core
        .update_desktop_preferences(
            1,
            DesktopPreferences {
                resident: false,
                open_library_on_launch: true,
                login_startup_requested: true,
            },
        )
        .unwrap();
    drop(core);
    let mut reopened = Core::new(&data).unwrap();
    assert_eq!(
        reopened.device_identity().unwrap(),
        DeviceIdentity {
            id: identity,
            is_current: true
        }
    );
    assert_eq!(reopened.desktop_preferences().unwrap(), second);
    // An app intent is never returned as an OS StartupTask state.
    assert!(!second.preferences.resident);
}

#[test]
fn device_preference_stale_revision_and_digest_never_replace_original_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("state")).unwrap();
    core.update_desktop_preferences(
        0,
        DesktopPreferences {
            resident: true,
            ..Default::default()
        },
    )
    .unwrap();
    let (original, digest) = core
        .read_device_record(DeviceRecordKind::DesktopPreferences, 16384)
        .unwrap()
        .unwrap();
    assert_eq!(
        core.update_desktop_preferences(0, DesktopPreferences::default())
            .unwrap_err()
            .code,
        "deviceStateConflict"
    );
    assert_eq!(
        core.compare_and_write_device_record(DeviceRecordKind::DesktopPreferences, None, &original)
            .unwrap_err()
            .code,
        "deviceStateConflict"
    );
    assert_eq!(
        core.compare_and_write_device_record(
            DeviceRecordKind::DesktopPreferences,
            Some(&"0".repeat(64)),
            &original
        )
        .unwrap_err()
        .code,
        "deviceStateConflict"
    );
    assert!(
        core.compare_and_write_device_record(
            DeviceRecordKind::DesktopPreferences,
            Some(&digest.to_uppercase()),
            &original
        )
        .is_err()
    );
    assert_eq!(
        core.read_device_record(DeviceRecordKind::DesktopPreferences, 16384)
            .unwrap()
            .unwrap(),
        (original, digest)
    );
}

#[test]
fn device_physical_profile_copy_is_retained_foreign_and_never_auto_adopted() {
    let temp = tempfile::tempdir().unwrap();
    let mut source = Core::new(temp.path().join("source")).unwrap();
    let source_snapshot = source
        .update_desktop_preferences(
            0,
            DesktopPreferences {
                resident: true,
                login_startup_requested: true,
                open_library_on_launch: true,
            },
        )
        .unwrap();
    let mut copy = Core::new(temp.path().join("copy")).unwrap();
    for kind in [
        DeviceRecordKind::Identity,
        DeviceRecordKind::DesktopPreferences,
    ] {
        fs::copy(
            source.device_record_path(kind),
            copy.device_record_path(kind),
        )
        .unwrap();
    }
    let original_identity = fs::read(copy.device_record_path(DeviceRecordKind::Identity)).unwrap();
    let original_preferences =
        fs::read(copy.device_record_path(DeviceRecordKind::DesktopPreferences)).unwrap();
    let snapshot = copy.desktop_preferences().unwrap();
    assert_eq!(snapshot.device.id, source_snapshot.device.id);
    assert!(!snapshot.device.is_current);
    assert_eq!(snapshot.preferences, source_snapshot.preferences);
    assert_eq!(
        snapshot.effective_preferences(),
        DesktopPreferences::default()
    );
    assert_eq!(
        copy.update_desktop_preferences(snapshot.revision, DesktopPreferences::default())
            .unwrap_err()
            .code,
        "deviceNotCurrent"
    );
    assert_eq!(
        fs::read(copy.device_record_path(DeviceRecordKind::Identity)).unwrap(),
        original_identity
    );
    assert_eq!(
        fs::read(copy.device_record_path(DeviceRecordKind::DesktopPreferences)).unwrap(),
        original_preferences
    );
}

#[test]
fn device_missing_identity_with_retained_originals_never_allocates_another_id() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("state")).unwrap();
    let orphan = core.device_record_path(DeviceRecordKind::Reminders);
    fs::write(&orphan, b"retained-original-not-adopted").unwrap();
    assert_eq!(
        core.device_identity().unwrap_err().code,
        "deviceStateEvidence"
    );
    assert!(!core.device_record_path(DeviceRecordKind::Identity).exists());
    assert_eq!(fs::read(&orphan).unwrap(), b"retained-original-not-adopted");
}

#[test]
fn device_unknown_namespace_member_and_replaced_directory_are_not_ignored() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("state");
    let mut core = Core::new(&data).unwrap();
    let foreign = data.join(DEVICE_STATE_ROOT).join("unowned.json");
    fs::write(&foreign, b"original").unwrap();
    assert_eq!(
        core.device_identity().unwrap_err().code,
        "deviceStateEvidence"
    );
    assert!(fs::rename(data.join(DEVICE_STATE_ROOT), data.join("replaced")).is_err());
    assert_eq!(fs::read(foreign).unwrap(), b"original");
}

#[test]
fn device_strict_schema_and_byte_bounds_preserve_invalid_originals() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("state")).unwrap();
    let identity = core.device_identity().unwrap();
    let path = core.device_record_path(DeviceRecordKind::DesktopPreferences);
    let invalid = serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"deviceId":identity.id,"revision":1,"preferences":{"resident":false,"openLibraryOnLaunch":false,"loginStartupRequested":false,"uiLocale":"en-US"}})).unwrap();
    fs::write(&path, &invalid).unwrap();
    assert!(core.desktop_preferences().is_err());
    assert_eq!(fs::read(&path).unwrap(), invalid);
    let oversized = vec![b' '; 16385];
    fs::write(&path, &oversized).unwrap();
    assert!(core.desktop_preferences().is_err());
    assert_eq!(fs::read(&path).unwrap(), oversized);
}

#[test]
fn device_malformed_reminder_is_not_a_blanket_backup_exclusion() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("state");
    let mut core = Core::new(&data).unwrap();
    let root = temp.path().join("library");
    core.create_library(&root).unwrap();
    core.device_identity().unwrap();
    fs::write(core.device_record_path(DeviceRecordKind::Reminders), b"{}").unwrap();
    let result = core.prepare_operation(OperationRequest {
        operation_id: fresh(),
        root: display(&root),
        intent: OperationIntent::BackupLibrary {
            destination: display(&temp.path().join("backup")),
        },
    });
    assert_eq!(result.unwrap_err().code, "deviceStateEvidence");
    assert_eq!(
        fs::read(core.device_record_path(DeviceRecordKind::Reminders)).unwrap(),
        b"{}"
    );
}

#[test]
fn device_validated_global_records_are_qualified_but_not_copied_to_library_backup() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("state");
    let mut core = Core::new(&data).unwrap();
    let root = temp.path().join("library");
    core.create_library(&root).unwrap();
    let state = core
        .update_desktop_preferences(
            0,
            DesktopPreferences {
                resident: true,
                ..Default::default()
            },
        )
        .unwrap();
    core.set_reminder_library(&root, false, 0, &fresh())
        .unwrap();
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&temp.path().join("backup")),
            },
        })
        .unwrap();
    let transaction = core.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    assert!(transaction.backup_private_digest.is_some());
    assert!(
        transaction
            .files
            .iter()
            .all(|file| !Path::new(&file.path).starts_with(data.join(DEVICE_STATE_ROOT)))
    );
    let frozen = serde_json::to_string(transaction).unwrap();
    assert!(!frozen.contains(&state.device.id));
}

#[test]
fn device_profile_qualification_does_not_acquire_core_and_rejects_path_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("new/parent/profile");
    let qualified = qualify_profile(&path).unwrap();
    let identity = qualified.physical_identity().to_owned();
    assert_eq!(
        qualify_profile(&path).unwrap().physical_identity(),
        identity
    );
    assert!(fs::rename(&path, path.with_file_name("moved")).is_err());
    let core = Core::new(&path).unwrap();
    assert!(fs::rename(&path, path.with_file_name("moved")).is_err());
    drop(core);
    assert!(fs::rename(&path, path.with_file_name("moved")).is_err());
    drop(qualified);
    let moved = path.with_file_name("moved");
    fs::rename(&path, &moved).unwrap();
    assert_eq!(
        qualify_profile(&moved).unwrap().physical_identity(),
        identity
    );
    let replacement = qualify_profile(&path).unwrap();
    assert_ne!(replacement.physical_identity(), identity);
}

#[test]
fn device_excluded_preference_update_does_not_invalidate_library_backup_but_unknown_member_does() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("state");
    let mut core = Core::new(&data).unwrap();
    let root = temp.path().join("library");
    core.create_library(&root).unwrap();
    core.update_desktop_preferences(
        0,
        DesktopPreferences {
            resident: true,
            ..Default::default()
        },
    )
    .unwrap();
    let prepare = |core: &mut Core, name: &str| {
        core.prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&temp.path().join(name)),
            },
        })
        .unwrap()
    };
    let first = prepare(&mut core, "backup1");
    core.update_desktop_preferences(1, DesktopPreferences::default())
        .unwrap();
    core.execute_operation(&first.operation_id, &first.plan_id)
        .unwrap();
    let second = prepare(&mut core, "backup2");
    let unknown = data.join(DEVICE_STATE_ROOT).join("unclassified.raw");
    fs::write(&unknown, b"retain original").unwrap();
    assert_eq!(
        core.execute_operation(&second.operation_id, &second.plan_id)
            .unwrap_err()
            .code,
        "deviceStateEvidence"
    );
    assert_eq!(fs::read(&unknown).unwrap(), b"retain original");
}
