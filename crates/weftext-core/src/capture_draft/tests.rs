use super::*;

struct Fixture {
    temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    foreign: PathBuf,
    core: Core,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("L1");
        let foreign = temp.path().join("L2");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        core.create_library(&foreign).unwrap();
        Self {
            temp,
            data,
            root,
            foreign,
            core,
        }
    }
    fn create(&mut self) -> CaptureDraft {
        self.core
            .create_targeted_capture_draft(
                &fresh(),
                &fresh(),
                input("before"),
                &self.root,
                &self.root,
                None,
            )
            .unwrap()
    }
    fn path(&self, draft: &CaptureDraft) -> PathBuf {
        capture_record_path(&self.data, &draft.scope.owner, &draft.draft_id).unwrap()
    }
    fn restart(self) -> Self {
        let Self {
            temp,
            data,
            root,
            foreign,
            core,
        } = self;
        drop(core);
        let core = Core::new(&data).unwrap();
        Self {
            temp,
            data,
            root,
            foreign,
            core,
        }
    }
    fn backup(&mut self, name: &str) -> Result<OperationPreview> {
        self.core.prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&self.root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&self.temp.path().join(name)),
            },
        })
    }
    fn operation(&mut self, intent: OperationIntent) -> OperationReceipt {
        let preview = self
            .core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&self.root),
                intent,
            })
            .unwrap();
        self.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
    }
}
fn input(body: &str) -> CaptureInput {
    CaptureInput {
        body: CaptureText::from_text(body),
        ..Default::default()
    }
}
fn frozen(draft: &CaptureDraft) -> FrozenCaptureRequest {
    let operation_id = fresh();
    let root = &draft.target.as_ref().unwrap().root_hint;
    let parent = match &draft.target.as_ref().unwrap().parent {
        CaptureParent::LibraryRoot => root,
        CaptureParent::Managed { path_hint, .. } => path_hint,
    };
    let mut fields = serde_json::json!({"operationId":operation_id,"root":root,"parent":parent,"body":"PLACEHOLDER","fields":{},"types":[]});
    fields["body"] = serde_json::json!("original");
    let wire = serde_json::to_string(&fields)
        .unwrap()
        .replace("original", "original\\ud800");
    FrozenCaptureRequest {
        operation_id,
        request_kind: "capture".into(),
        wire_digest: hash(wire.as_bytes()),
        wire,
        input_revision: draft.input_revision,
    }
}

#[test]
fn exact_utf16_unfinished_text_survives_restart_without_lossy_conversion() {
    let mut f = Fixture::new();
    let units = vec![0xd800, 0, 0xdc00, 0xd83d, 0xde00, 0x4e2d, 0x000d, 0x000a];
    let raw = CaptureText {
        utf16: units.clone(),
    };
    assert_eq!(raw.utf8().unwrap_err().code, "captureTextIncomplete");
    let mut value = input("plain");
    value.name = raw.clone();
    value.body = raw.clone();
    value.parent_input = raw.clone();
    value.parameters_text = raw.clone();
    value.raw_inputs.push(CaptureRawInput {
        field_id: "task/count".into(),
        value_path: vec![weftext_data::ValuePath::Index { index: 1 }],
        control_role: "scalar".into(),
        text: raw,
    });
    let draft = f
        .core
        .create_capture_draft(&fresh(), &fresh(), value)
        .unwrap();
    let path = f.path(&draft);
    let bytes = fs::read(&path).unwrap();
    let f = f.restart();
    let actual = f.core.read_capture_draft(&draft.draft_id).unwrap();
    assert_eq!(actual.current_input.body.utf16, units);
    assert_eq!(actual.current_input.raw_inputs[0].text.utf16, units);
    assert_eq!(actual.current_input.parent_input.utf16, units);
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert!(actual.target.is_none());
    assert!(actual.pending.is_none());
}

#[test]
fn mutation_lost_reply_is_idempotent_and_stale_or_reused_commands_preserve_bytes() {
    let mut f = Fixture::new();
    let initial = f.create();
    let mutation = fresh();
    let first = f
        .core
        .update_capture_input(
            &initial.draft_id,
            initial.revision,
            &mutation,
            input("next"),
        )
        .unwrap();
    // The writer's response may be lost together with the process, not just a socket.
    let mut f = f.restart();
    let replay = f
        .core
        .update_capture_input(
            &initial.draft_id,
            initial.revision,
            &mutation,
            input("next"),
        )
        .unwrap();
    assert_eq!(replay.revision, first.revision);
    assert_eq!(
        f.core
            .capture_mutation_status(
                &initial.draft_id,
                &mutation,
                &CaptureMutationCommand::Input {
                    expected_revision: initial.revision,
                    input: input("next")
                }
            )
            .unwrap()
            .status,
        "applied"
    );
    let path = f.path(&first);
    let bytes = fs::read(&path).unwrap();
    for command in [
        CaptureMutationCommand::Input {
            expected_revision: initial.revision,
            input: input("different"),
        },
        CaptureMutationCommand::Input {
            expected_revision: initial.revision + 1,
            input: input("next"),
        },
    ] {
        assert_eq!(
            f.core
                .capture_mutation_status(&initial.draft_id, &mutation, &command)
                .unwrap()
                .status,
            "differentCommand"
        );
    }
    assert!(
        f.core
            .update_capture_input(
                &initial.draft_id,
                initial.revision,
                &mutation,
                input("changed same id")
            )
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        f.core
            .update_capture_input(
                &initial.draft_id,
                initial.revision,
                &fresh(),
                input("stale window")
            )
            .unwrap_err()
            .code,
        "captureConflict"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let aba = f
        .core
        .update_capture_input(&first.draft_id, first.revision, &fresh(), input("before"))
        .unwrap();
    assert!(aba.input_revision > initial.input_revision);
    assert_eq!(
        f.core
            .capture_mutation_status(
                &initial.draft_id,
                &mutation,
                &CaptureMutationCommand::Input {
                    expected_revision: initial.revision,
                    input: input("next")
                }
            )
            .unwrap()
            .status,
        "supersededOrUnknown"
    );
    let missing = fresh();
    assert!(
        f.core
            .update_capture_input(&missing, 0, &fresh(), input("not implicitly created"))
            .is_err()
    );
    assert!(
        !capture_record_path(&f.data, &CaptureOwner::Local, &missing)
            .unwrap()
            .exists()
    );
}

#[test]
fn frozen_wire_remains_exact_while_new_input_and_close_revision_advance() {
    let mut f = Fixture::new();
    let initial = f.create();
    let request = frozen(&initial);
    let pending = f
        .core
        .freeze_capture_submission(
            &initial.draft_id,
            initial.revision,
            &fresh(),
            request.clone(),
        )
        .unwrap();
    let permit = f
        .core
        .capture_flush_receipt(
            &pending.draft_id,
            pending.revision,
            pending.input_revision,
            &pending.last_mutation.mutation_id,
        )
        .unwrap();
    assert_eq!(
        permit.pending_operation_id.as_deref(),
        Some(request.operation_id.as_str())
    );
    assert_eq!(
        permit.record_digest,
        hash(&fs::read(f.path(&pending)).unwrap())
    );
    let next = f
        .core
        .update_capture_input(
            &pending.draft_id,
            pending.revision,
            &fresh(),
            input("typed during submit"),
        )
        .unwrap();
    assert_eq!(next.pending.as_ref().unwrap().wire, request.wire);
    assert!(
        f.core
            .capture_flush_receipt(
                &permit.draft_id,
                permit.record_revision,
                permit.input_revision,
                &permit.last_mutation_id
            )
            .is_err()
    );
    assert!(
        f.core
            .select_capture_target(
                &next.draft_id,
                next.revision,
                &fresh(),
                &f.foreign,
                &f.foreign,
                None
            )
            .is_err()
    );
    let f = f.restart();
    let restored = f.core.read_capture_draft(&next.draft_id).unwrap();
    assert_eq!(
        restored.pending.as_ref().unwrap().wire_digest,
        request.wire_digest
    );
    assert_eq!(
        restored.current_input.body.utf8().unwrap(),
        "typed during submit"
    );
    assert!(
        f.core
            .capture_flush_receipt(
                &restored.draft_id,
                restored.revision,
                restored.input_revision,
                &restored.last_mutation.mutation_id
            )
            .is_ok()
    );
}

#[test]
fn malformed_or_wrong_operation_wire_cannot_replace_the_previous_draft() {
    let mut f = Fixture::new();
    let initial = f.create();
    let path = f.path(&initial);
    let bytes = fs::read(&path).unwrap();
    for wire in [
        "{}".to_owned(),
        "{\"body\":01}".to_owned(),
        format!(r#"{{"operationId":"{}"}}"#, fresh()),
        "{\"body\":\"\\uZZZZ\"}".to_owned(),
    ] {
        let mut pending = frozen(&initial);
        pending.wire = wire;
        pending.wire_digest = hash(pending.wire.as_bytes());
        assert!(
            f.core
                .freeze_capture_submission(&initial.draft_id, initial.revision, &fresh(), pending)
                .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    let mut pending = frozen(&initial);
    pending.wire_digest = hash(b"another wire");
    assert!(
        f.core
            .freeze_capture_submission(&initial.draft_id, initial.revision, &fresh(), pending)
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn actor_and_library_bindings_are_not_current_document_or_client_aliases() {
    let mut f = Fixture::new();
    let id = fresh();
    let root = f.root.clone();
    let foreign = f.foreign.clone();
    let alice = f
        .core
        .with_actor_for_root("deployment:alice", &root, |core| {
            core.create_targeted_capture_draft(
                &id,
                &fresh(),
                input("alice"),
                &root,
                &root,
                Some("approved-slot".into()),
            )
        })
        .unwrap();
    let bob = f
        .core
        .with_actor_for_root("deployment:bob", &root, |core| {
            core.create_targeted_capture_draft(
                &id,
                &fresh(),
                input("bob"),
                &root,
                &root,
                Some("approved-slot".into()),
            )
        })
        .unwrap();
    assert_ne!(f.path(&alice), f.path(&bob));
    assert!(
        f.core
            .with_actor_for_root("deployment:alice", &foreign, |core| core
                .read_capture_draft(&id))
            .is_err()
    );
    assert_eq!(
        f.core
            .with_actor_for_root("deployment:bob", &root, |core| core.read_capture_draft(&id))
            .unwrap()
            .current_input
            .body
            .utf8()
            .unwrap(),
        "bob"
    );
    fs::write(f.path(&bob), fs::read(f.path(&alice)).unwrap()).unwrap();
    assert!(
        f.core
            .with_actor_for_root("deployment:bob", &root, |core| core.read_capture_draft(&id))
            .is_err()
    );
    let device = f
        .core
        .create_capture_draft(&fresh(), &fresh(), input("unassigned"))
        .unwrap();
    let assigned = f
        .core
        .select_capture_target(
            &device.draft_id,
            device.revision,
            &fresh(),
            &root,
            &root,
            None,
        )
        .unwrap();
    assert!(
        f.core
            .select_capture_target(
                &assigned.draft_id,
                assigned.revision,
                &fresh(),
                &foreign,
                &foreign,
                None
            )
            .is_err()
    );
}

#[test]
fn unknown_version_or_noncanonical_private_payload_never_overwrites_original() {
    let mut f = Fixture::new();
    let mut initial = f.create();
    let path = f.path(&initial);
    initial.version = 2;
    let bytes = serde_json::to_vec(&initial).unwrap();
    fs::write(&path, &bytes).unwrap();
    assert!(f.core.read_capture_draft(&initial.draft_id).is_err());
    assert!(
        f.core
            .update_capture_input(
                &initial.draft_id,
                initial.revision,
                &fresh(),
                input("replace")
            )
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    initial.version = 1;
    let mut bytes = serde_json::to_vec(&initial).unwrap();
    bytes.push(b' ');
    fs::write(&path, &bytes).unwrap();
    assert!(f.core.read_capture_draft(&initial.draft_id).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn byte_and_owner_domain_budgets_fail_before_replacing_readable_input() {
    let mut f = Fixture::new();
    let initial = f.create();
    let path = f.path(&initial);
    let bytes = fs::read(&path).unwrap();
    let mut large = input("small");
    large.body.utf16 = vec![65535; MAX_CAPTURE_TEXT_UNITS];
    large.title = large.body.clone();
    assert!(
        f.core
            .update_capture_input(&initial.draft_id, initial.revision, &fresh(), large)
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let mut oversized_native = input("raw input remains separate");
    oversized_native.fields.push(CaptureFieldInput {
        field_id: "stable-field".into(),
        value: Some(weftext_data::NativeValue::Text(
            "x".repeat(MAX_CAPTURE_RECORD_BYTES + 1),
        )),
    });
    assert_eq!(
        f.core
            .update_capture_input(
                &initial.draft_id,
                initial.revision,
                &fresh(),
                oversized_native
            )
            .unwrap_err()
            .code,
        "budget"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let other = f
        .core
        .create_capture_draft(&fresh(), &fresh(), input("other"))
        .unwrap();
    let other_path = f.path(&other);
    let mut file = OpenOptions::new().write(true).open(&other_path).unwrap();
    file.set_len(MAX_CAPTURE_DOMAIN_BYTES).unwrap();
    file.flush().unwrap();
    drop(file);
    assert!(
        f.core
            .update_capture_input(
                &initial.draft_id,
                initial.revision,
                &fresh(),
                input("budget refused")
            )
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn backup_routes_capture_by_header_without_foreign_or_unassigned_payload_reads() {
    let mut f = Fixture::new();
    let selected = f.create();
    let foreign = f
        .core
        .create_targeted_capture_draft(
            &fresh(),
            &fresh(),
            input("FOREIGN-SECRET"),
            &f.foreign,
            &f.foreign,
            None,
        )
        .unwrap();
    let device = f
        .core
        .create_capture_draft(&fresh(), &fresh(), input("DEVICE-SECRET"))
        .unwrap();
    for draft in [&foreign, &device] {
        let bytes = serde_json::to_vec(draft).unwrap();
        let scope_end = bytes
            .windows(11)
            .position(|part| part == b",\"version\":")
            .unwrap();
        let path = f.path(draft);
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .unwrap();
        file.write_all(&bytes[..scope_end]).unwrap();
        file.set_len(MAX_CAPTURE_DOMAIN_BYTES + 1).unwrap();
        file.sync_all().unwrap();
    }
    let plan = f.backup("backup").unwrap();
    let files = f.core.prepared_operation_files(&plan.operation_id).unwrap();
    assert!(
        files
            .iter()
            .any(|file| file.path.ends_with(&format!("{}.json", selected.draft_id)))
    );
    assert!(!files.iter().any(
        |file| file.path.ends_with(&format!("{}.json", foreign.draft_id))
            || file.path.ends_with(&format!("{}.json", device.draft_id))
    ));
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
}

#[test]
fn backup_selected_corruption_and_same_bytes_record_replacement_are_not_accepted() {
    let mut f = Fixture::new();
    let selected = f.create();
    let path = f.path(&selected);
    let plan = f.backup("stale-backup").unwrap();
    let bytes = fs::read(&path).unwrap();
    let original = f.temp.path().join("preserved-original.json");
    fs::rename(&path, &original).unwrap();
    fs::write(&path, bytes).unwrap();
    assert!(
        f.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .is_err()
    );
    // A selected unknown record version is a failure, never an omitted draft.
    let mut invalid = selected;
    invalid.version = 99;
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(f.backup("invalid-backup").is_err());
}

#[test]
fn interrupted_stage_never_poisoned_formal_list_or_previous_durable_record() {
    let mut f = Fixture::new();
    let initial = f.create();
    let path = f.path(&initial);
    let bytes = fs::read(&path).unwrap();
    for point in ["captureStageCreated", "captureStageFlushed"] {
        hosted::set_test_failpoint(point);
        assert_eq!(
            f.core
                .update_capture_input(&initial.draft_id, initial.revision, &fresh(), input(point))
                .unwrap_err()
                .code,
            "injectedFailure"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    assert_eq!(f.core.capture_storage_status().unwrap().retained_stages, 2);
    let mut f = f.restart();
    assert_eq!(f.core.list_capture_drafts().unwrap().len(), 1);
    assert_eq!(
        f.core
            .read_capture_draft(&initial.draft_id)
            .unwrap()
            .current_input
            .body
            .utf8()
            .unwrap(),
        "before"
    );
    let next = f
        .core
        .update_capture_input(
            &initial.draft_id,
            initial.revision,
            &fresh(),
            input("next reliable input"),
        )
        .unwrap();
    assert_eq!(next.revision, initial.revision + 1);
    assert_eq!(f.core.capture_storage_status().unwrap().retained_stages, 2);
    let plan = f.backup("backup-with-unpublished-stage").unwrap();
    assert!(
        f.core
            .prepared_operation_files(&plan.operation_id)
            .unwrap()
            .iter()
            .all(|file| !file.path.contains(CAPTURE_STAGING_DIRECTORY))
    );
}

#[test]
fn managed_node_parent_stays_in_its_container_and_rejects_container_replacement() {
    let mut f = Fixture::new();
    let preview = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::NewNode {
                parent: display(&f.root),
                name: "P".into(),
                source: None,
            },
        })
        .unwrap();
    let receipt = f
        .core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let parent = PathBuf::from(receipt.path);
    let initial = f
        .core
        .create_targeted_capture_draft(&fresh(), &fresh(), input("nested"), &f.root, &parent, None)
        .unwrap();
    assert_eq!(
        f.core
            .resolve_capture_target(&initial.draft_id)
            .unwrap()
            .parent,
        display(&parent)
    );
    let pending = frozen(&initial);
    assert!(
        f.core
            .freeze_capture_submission(&initial.draft_id, initial.revision, &fresh(), pending)
            .is_ok()
    );
    let moved = f.root.join("OldContainer");
    fs::rename(&parent, &moved).unwrap();
    fs::create_dir(&parent).unwrap();
    fs::rename(moved.join("P.typ"), parent.join("P.typ")).unwrap();
    // The source has its original FI, but this is a different destination container.
    assert!(f.core.resolve_capture_target(&initial.draft_id).is_err());
}

#[test]
fn lawful_parent_relocation_resolves_before_freeze_but_never_rewrites_frozen_wire() {
    let mut f = Fixture::new();
    let parent = PathBuf::from(
        f.operation(OperationIntent::NewNode {
            parent: display(&f.root),
            name: "Original".into(),
            source: None,
        })
        .path,
    );
    let draft = f
        .core
        .create_targeted_capture_draft(&fresh(), &fresh(), input("nested"), &f.root, &parent, None)
        .unwrap();
    let renamed = f.operation(OperationIntent::RenameNode {
        source: display(&parent),
        name: "Renamed".into(),
    });
    let resolved = f.core.resolve_capture_target(&draft.draft_id).unwrap();
    assert_eq!(resolved.parent, renamed.path);
    let mut request = frozen(&draft);
    request.wire = serde_json::to_string(&serde_json::json!({"operationId":request.operation_id,"root":resolved.root,"parent":resolved.parent,"body":"original"})).unwrap();
    request.wire_digest = hash(request.wire.as_bytes());
    let pending = f
        .core
        .freeze_capture_submission(&draft.draft_id, draft.revision, &fresh(), request.clone())
        .unwrap();
    let container = PathBuf::from(
        f.operation(OperationIntent::NewNode {
            parent: display(&f.root),
            name: "Container".into(),
            source: None,
        })
        .path,
    );
    let moved = f.operation(OperationIntent::MoveNode {
        source: renamed.path,
        parent: display(&container),
    });
    let f = f.restart();
    assert_eq!(
        f.core
            .resolve_capture_target(&draft.draft_id)
            .unwrap()
            .parent,
        moved.path
    );
    let restored = f.core.read_capture_draft(&pending.draft_id).unwrap();
    assert_eq!(restored.pending.unwrap().wire, request.wire);
}

#[test]
fn canonical_but_cross_library_pending_wire_is_rejected_by_reads_and_owner_backup() {
    let mut f = Fixture::new();
    let draft = f.create();
    let request = frozen(&draft);
    let mut pending = f
        .core
        .freeze_capture_submission(&draft.draft_id, draft.revision, &fresh(), request)
        .unwrap();
    let path = f.path(&pending);
    let request = pending.pending.as_mut().unwrap();
    request.wire = serde_json::to_string(&serde_json::json!({"operationId":request.operation_id,"root":display(&f.foreign),"parent":display(&f.foreign),"body":"corrupted"})).unwrap();
    request.wire_digest = hash(request.wire.as_bytes());
    let corrupt = serde_json::to_vec(&pending).unwrap();
    fs::write(&path, &corrupt).unwrap();
    assert!(f.core.read_capture_draft(&pending.draft_id).is_err());
    assert!(f.backup("invalid-selected-capture").is_err());
    assert_eq!(fs::read(&path).unwrap(), corrupt);
}

#[test]
fn scope_reader_never_normalizes_spaces_inside_key_and_always_restores_cursor() {
    let f = Fixture::new();
    let header = CaptureScopeHeader {
        owner: CaptureOwner::Local,
        library: CaptureLibrary::DeviceUnassigned,
    };
    let header_bytes = serde_json::to_vec(&header).unwrap();
    let mut allowed = b"{\"scope\":".to_vec();
    allowed.extend_from_slice(&header_bytes);
    // No closing record and no author payload is required to route a header.
    let mut cursor = std::io::Cursor::new(allowed);
    cursor.set_position(3);
    assert_eq!(read_capture_header(&mut cursor).unwrap(), header);
    assert_eq!(cursor.position(), 3);
    let mut wrong = b"{\"s cope\":".to_vec();
    wrong.extend_from_slice(&header_bytes);
    let mut cursor = std::io::Cursor::new(wrong);
    cursor.set_position(2);
    assert!(read_capture_header(&mut cursor).is_err());
    assert_eq!(cursor.position(), 2);
    let path = f
        .data
        .join(CAPTURE_DIRECTORY)
        .join(format!("{}.json", fresh().to_uppercase()));
    assert!(validate_capture_header_path(&path, &f.data, &header).is_err());
}

#[test]
fn list_checks_total_bytes_before_loading_multiple_individually_valid_records() {
    let mut f = Fixture::new();
    let mut draft = f.create();
    draft.current_input.body.utf16 = vec![65535; MAX_CAPTURE_TEXT_UNITS];
    for _ in 0..6 {
        draft.draft_id = fresh();
        let bytes = serde_json::to_vec(&draft).unwrap();
        let path = f.path(&draft);
        validate_capture_record(&path, &f.data, &bytes).unwrap();
        fs::write(path, bytes).unwrap();
    }
    assert_eq!(f.core.list_capture_drafts().unwrap_err().code, "budget");
}
