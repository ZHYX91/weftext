use super::*;
use serde_json::Value;

#[test]
fn hosted_creation_lost_reply_uses_original_location_hints_after_separator_change_and_restart() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let root_slashes = display(&root).replace('\\', "/");
    let draft_id = fresh();
    let mutation_id = fresh();
    let authored = input("Unconfirmed author input");
    f.core
        .with_actor_for_root("deployment:alice:structure", &root, |core| {
            core.create_targeted_capture_draft(
                &draft_id,
                &mutation_id,
                authored.clone(),
                Path::new(&root_slashes),
                &root,
                Some("captures".into()),
            )
        })
        .unwrap();
    let mut f = f.restart();
    let (status, projected) = f
        .core
        .with_actor("deployment:alice:structure", |core| {
            core.capture_hosted_create_status(&draft_id, &mutation_id, "captures", authored.clone())
        })
        .unwrap();
    assert_eq!(status, "applied");
    assert_eq!(projected.draft_id, draft_id);
    assert_eq!(projected.texts.body[0].text, authored.body);
    let (changed, _) = f
        .core
        .with_actor("deployment:alice:structure", |core| {
            core.capture_hosted_create_status(
                &draft_id,
                &mutation_id,
                "captures",
                input("Changed bytes"),
            )
        })
        .unwrap();
    assert_eq!(changed, "differentCommand");
    assert!(
        f.core
            .with_actor("deployment:bob:structure", |core| {
                core.capture_hosted_create_status(&draft_id, &mutation_id, "captures", authored)
            })
            .is_err()
    );
}

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
fn author_texts(text: CaptureText) -> CaptureAuthorTexts {
    CaptureAuthorTexts {
        body: vec![CaptureRawAuthor {
            text,
            provenance: CaptureRawProvenance {
                producer_id: "00000000-0000-4000-8000-000000000001".into(),
                input_id: "b".repeat(32),
                revision: 1,
            },
        }],
        ..Default::default()
    }
}
fn input(body: &str) -> CaptureInput {
    let mut own = author_texts(CaptureText::from_text(body));
    own.body[0].provenance.input_id = "a".repeat(32);
    CaptureInput {
        author_texts: own,
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
        binding: CaptureRawBinding::Exact,
        author: None,
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

fn incomplete_field_input(wire: &str) -> CaptureInput {
    let mut value = input("own body");
    value.fields = vec![CaptureFieldInput {
        field_id: "tasks/reminder".into(),
        value_state: CaptureFieldValueState::Incomplete {
            wire: CaptureText::from_text(wire),
        },
    }];
    value.raw_inputs = vec![CaptureRawInput {
        field_id: "tasks/reminder".into(),
        value_path: vec![
            weftext_data::ValuePath::Member {
                key: "siblings".into(),
            },
            weftext_data::ValuePath::Index { index: 1 },
            weftext_data::ValuePath::Member { key: "raw".into() },
        ],
        control_role: "exact-instant".into(),
        text: CaptureText {
            utf16: vec![0xd800, 0],
        },
        binding: CaptureRawBinding::Exact,
        author: None,
    }];
    value
}
const INCOMPLETE_FIELD: &str = r#"{"kind":"record","value":[{"key":"kind","value":{"kind":"text","value":"absolute"}},{"key":"siblings","value":{"kind":"array","value":[{"kind":"int","value":"9007199254740993"},{"kind":"record","value":[{"key":"raw","value":{"kind":"text","value":"\ud800\u0000"}},{"key":"other","value":{"kind":"text","value":"preserved😀"}}]}]}},{"key":"tail","value":{"kind":"bool","value":true}}]}"#;

#[test]
fn unfinished_private_field_preserves_complete_siblings_union_and_list_after_restart_and_backup() {
    let mut f = Fixture::new();
    let initial = f.create();
    let value = incomplete_field_input(INCOMPLETE_FIELD);
    let saved = f
        .core
        .update_capture_input(&initial.draft_id, initial.revision, &fresh(), value)
        .unwrap();
    let path = f.path(&saved);
    let bytes = fs::read(&path).unwrap();
    let mut f = f.restart();
    let restored = f.core.read_capture_draft(&saved.draft_id).unwrap();
    let CaptureFieldValueState::Incomplete { wire } = &restored.current_input.fields[0].value_state
    else {
        panic!("unfinished field became another candidate")
    };
    assert_eq!(wire.utf8().unwrap(), INCOMPLETE_FIELD);
    assert_eq!(
        restored.current_input.raw_inputs[0].text.utf16,
        vec![0xd800, 0]
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let pure = serde_json::to_string(
        &f.core
            .read_capture_author_recovery(&saved.draft_id)
            .unwrap(),
    )
    .unwrap();
    assert!(
        !pure.contains("siblings") && !pure.contains("preserved") && !pure.contains("valueState")
    );
    let plan = f.backup("unfinished-field-backup").unwrap();
    assert!(
        f.core
            .prepared_operation_files(&plan.operation_id)
            .unwrap()
            .iter()
            .any(|file| file.path.ends_with(&format!("{}.json", saved.draft_id)))
    );
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn malformed_or_retargeted_unfinished_field_never_replaces_previous_private_bytes() {
    let mut f = Fixture::new();
    let initial = f.create();
    let path = f.path(&initial);
    let bytes = fs::read(&path).unwrap();
    let mut cases = Vec::new();
    for wire in [
        r#"{"kind":"text","kind":"text","value":"\ud800"}"#,
        r#"{"kind":"text","value":"\ud800","schema":{}}"#,
        r#"{"kind":"text","value":"complete"}"#,
        r#"{"schema":{},"fields":[],"body":"\ud800"}"#,
        r#"{"kind":"record","value":[{"key":"a","value":{"kind":"text","value":"\ud800"}},{"key":"a","value":{"kind":"none"}}]}"#,
    ] {
        cases.push(incomplete_field_input(wire));
    }
    let mut wrong_address = incomplete_field_input(INCOMPLETE_FIELD);
    wrong_address.raw_inputs[0].value_path[1] = weftext_data::ValuePath::Index { index: 0 };
    cases.push(wrong_address);
    let mut copy_only = incomplete_field_input(INCOMPLETE_FIELD);
    copy_only.raw_inputs[0].binding = CaptureRawBinding::CopyOnly;
    cases.push(copy_only);
    let mut foreign_field = incomplete_field_input(INCOMPLETE_FIELD);
    foreign_field.raw_inputs[0].field_id = "another/field".into();
    cases.push(foreign_field);
    for value in cases {
        assert!(
            f.core
                .update_capture_input(&initial.draft_id, initial.revision, &fresh(), value)
                .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    assert!(
        serde_json::from_value::<CaptureFieldInput>(
            serde_json::json!({"fieldId":"old","value":null})
        )
        .is_err()
    );
}

#[test]
fn capture_valid_native_and_address_decoders_reject_information_that_shared_values_would_drop() {
    for value in [
        r#"{"kind":"text","value":"x","schema":{}}"#,
        r#"{"kind":"record","value":[{"key":"a","value":{"kind":"none"},"extra":true}]}"#,
        r#"{"kind":"text","value":"x","value":"y"}"#,
    ] {
        let wire =
            format!(r#"{{"fieldId":"field","valueState":{{"kind":"valid","value":{value}}}}}"#);
        assert!(serde_json::from_str::<CaptureFieldInput>(&wire).is_err());
    }
    let raw = CaptureRawInput {
        field_id: "field".into(),
        value_path: vec![weftext_data::ValuePath::Index { index: 0 }],
        control_role: "text".into(),
        text: CaptureText::default(),
        binding: CaptureRawBinding::Exact,
        author: None,
    };
    let mut wire = serde_json::to_value(raw).unwrap();
    wire["valuePath"][0]["key"] = serde_json::json!("another");
    assert!(serde_json::from_value::<CaptureRawInput>(wire).is_err());
}

#[test]
fn all_top_level_private_values_and_original_pending_survive_pure_producer_updates_and_restart() {
    let mut f = Fixture::new();
    let private = CaptureText::from_text("PRIVATE DEFAULT");
    let old = CaptureRawAuthor {
        text: CaptureText::from_text("OLD OWN"),
        provenance: CaptureRawProvenance {
            producer_id: fresh(),
            input_id: "a".repeat(32),
            revision: 1,
        },
    };
    let texts = CaptureAuthorTexts {
        name: vec![old.clone()],
        title: vec![old.clone()],
        body: vec![old.clone()],
        root_input: vec![old.clone()],
        parent_input: vec![old.clone()],
        parameters_text: vec![old.clone()],
    };
    let input = CaptureInput {
        name: private.clone(),
        title: private.clone(),
        body: private.clone(),
        root_input: private.clone(),
        parent_input: private.clone(),
        parameters_text: private,
        author_texts: texts,
        ..Default::default()
    };
    let root = f.root.clone();
    let draft = f
        .core
        .create_targeted_capture_draft(&fresh(), &fresh(), input, &root, &root, None)
        .unwrap();
    let pending = frozen(&draft);
    let draft = f
        .core
        .freeze_capture_submission(&draft.draft_id, draft.revision, &fresh(), pending.clone())
        .unwrap();
    let new = CaptureRawAuthor {
        text: CaptureText {
            utf16: vec![78, 0xd800, 0],
        },
        provenance: CaptureRawProvenance {
            producer_id: fresh(),
            input_id: "b".repeat(32),
            revision: 1,
        },
    };
    let new_texts = CaptureAuthorTexts {
        name: vec![new.clone()],
        title: vec![new.clone()],
        body: vec![new.clone()],
        root_input: vec![new.clone()],
        parent_input: vec![new.clone()],
        parameters_text: vec![new.clone()],
    };
    let command = CaptureAuthorInput {
        texts: new_texts,
        body_mode: Some(CaptureBodyMode::Text),
        ..Default::default()
    };
    let mutation = fresh();
    f.core
        .update_capture_author_input(&draft.draft_id, draft.revision, &mutation, command.clone())
        .unwrap();
    let mut f = f.restart();
    let (status, recovery) = f
        .core
        .capture_author_mutation_status(&draft.draft_id, &mutation, draft.revision, &command)
        .unwrap();
    assert_eq!(status, "applied");
    assert_eq!(recovery.texts.body.len(), 2);
    assert_eq!(recovery.texts.body[0].text, old.text);
    assert_eq!(recovery.texts.body[1].text, new.text);
    for slot in [
        &recovery.texts.name,
        &recovery.texts.title,
        &recovery.texts.body,
        &recovery.texts.root_input,
        &recovery.texts.parent_input,
        &recovery.texts.parameters_text,
    ] {
        assert_eq!(slot.len(), 2);
        assert_eq!(slot[0].text, old.text);
        assert_eq!(slot[1].text, new.text);
    }
    let record = f.core.read_capture_draft(&draft.draft_id).unwrap();
    for text in [
        &record.current_input.name,
        &record.current_input.title,
        &record.current_input.body,
        &record.current_input.root_input,
        &record.current_input.parent_input,
        &record.current_input.parameters_text,
    ] {
        assert_eq!(text.utf8().unwrap(), "PRIVATE DEFAULT");
    }
    assert!(matches!(
        record.current_input.body_mode,
        CaptureBodyMode::Markup
    ));
    assert_eq!(record.pending.as_ref().unwrap().wire, pending.wire);
    let pure = serde_json::to_string(&recovery).unwrap();
    assert!(!pure.contains("PRIVATE DEFAULT"));
    let path = f.path(&record);
    let before = fs::read(&path).unwrap();
    let mut changed = command.clone();
    changed.texts.body[0].text = CaptureText::from_text("contradictory same proof");
    assert!(
        f.core
            .update_capture_author_input(&draft.draft_id, record.revision, &fresh(), changed)
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut cleared = command;
    for slot in [
        &mut cleared.texts.name,
        &mut cleared.texts.title,
        &mut cleared.texts.body,
        &mut cleared.texts.root_input,
        &mut cleared.texts.parent_input,
        &mut cleared.texts.parameters_text,
    ] {
        slot[0].text = CaptureText::default();
        slot[0].provenance.revision = 2;
    }
    let saved = f
        .core
        .update_capture_author_input(&draft.draft_id, record.revision, &fresh(), cleared)
        .unwrap();
    assert_eq!(saved.texts.body.len(), 2);
    assert!(saved.texts.body[1].text.utf16.is_empty());
    let plan = f.backup("top-provenance-backup").unwrap();
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
}

fn capture_request(root: &Path, operation_id: &str) -> CaptureCreationRequest {
    CaptureCreationRequest {
        root: display(root),
        parent: display(root),
        name: "CaptureOriginal".into(),
        types: vec![],
        title: None,
        fields: BTreeMap::new(),
        body: "owned original".into(),
        body_mode: CaptureBodyMode::Text,
        form_basis: None,
        operation_id: operation_id.into(),
    }
}
fn freeze_request(
    f: &mut Fixture,
    draft: &CaptureDraft,
    request: &CaptureCreationRequest,
) -> Result<CaptureDraft> {
    let wire = serde_json::to_string(request).unwrap();
    f.core.freeze_capture_submission(
        &draft.draft_id,
        draft.revision,
        &fresh(),
        FrozenCaptureRequest {
            operation_id: request.operation_id.clone(),
            request_kind: "capture".into(),
            wire_digest: hash(wire.as_bytes()),
            wire,
            input_revision: draft.input_revision,
        },
    )
}
#[test]
fn capture_result_reopens_only_its_canonical_original_plan_and_actual_current_birth_member() {
    let mut f = Fixture::new();
    let draft = f.create();
    let operation_id = fresh();
    let request = capture_request(&f.root, &operation_id);
    let plan = f
        .core
        .prepare_feature_operation(
            OperationRequest {
                operation_id: operation_id.clone(),
                root: display(&f.root),
                intent: OperationIntent::NewNode {
                    parent: display(&f.root),
                    name: request.name.clone(),
                    source: None,
                },
            },
            &serde_json::json!({"kind":"capture","request":request}),
        )
        .unwrap();
    let frozen = freeze_request(&mut f, &draft, &request).unwrap();
    assert!(
        f.core
            .open_capture_result(&draft.draft_id, &operation_id)
            .is_err()
    ); // Prepared is not committed.
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
    let mut f = f.restart();
    let opened = f
        .core
        .open_capture_result(&draft.draft_id, &operation_id)
        .unwrap();
    let original_node = opened.node_id.clone();
    f.operation(OperationIntent::RenameNode {
        source: display(Path::new(&opened.path).parent().unwrap()),
        name: "Relocated".into(),
    });
    f.operation(OperationIntent::NewNode {
        parent: display(&f.root),
        name: request.name.clone(),
        source: None,
    });
    let moved = f
        .core
        .open_capture_result(&draft.draft_id, &operation_id)
        .unwrap();
    assert_eq!(moved.node_id, original_node);
    assert_ne!(moved.path, opened.path);
    let original_bytes = fs::read(&moved.path).unwrap();
    let original_plan = fs::read(f.core.journal_path(&operation_id, "plan")).unwrap();
    for variant in ["differentIntent", "oldSchema", "anotherRoot"] {
        let mut changed = request.clone();
        match variant {
            "differentIntent" => changed.body.push_str(" changed"),
            "oldSchema" => {
                changed.form_basis = Some(CaptureFormBasis {
                    schema_ref: format!(
                        "weftext-config://{}/schema",
                        moved.workspace_id.as_ref().unwrap()
                    ),
                    semantic_digest: "a".repeat(64),
                })
            }
            _ => {
                changed.root = display(&f.foreign);
                changed.parent = display(&f.foreign);
            }
        }
        let root = PathBuf::from(&changed.root);
        let other = f
            .core
            .create_targeted_capture_draft(
                &fresh(),
                &fresh(),
                input("different cut"),
                &root,
                &root,
                None,
            )
            .unwrap();
        freeze_request(&mut f, &other, &changed).unwrap();
        let private = fs::read(f.path(&other)).unwrap();
        assert!(
            f.core
                .open_capture_result(&other.draft_id, &operation_id)
                .is_err()
        );
        assert_eq!(fs::read(f.path(&other)).unwrap(), private);
    }
    let foreign_actor = f.core.with_actor("deployment:bob:structure", |core| {
        core.open_capture_result(&draft.draft_id, &operation_id)
    });
    assert!(foreign_actor.is_err());
    let different_id = fresh();
    let different = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: different_id.clone(),
            root: display(&f.root),
            intent: OperationIntent::NewNode {
                parent: display(&f.root),
                name: "OrdinaryNode".into(),
                source: None,
            },
        })
        .unwrap();
    f.core
        .execute_operation(&different.operation_id, &different.plan_id)
        .unwrap();
    let other = f.create();
    let other_request = capture_request(&f.root, &different_id);
    freeze_request(&mut f, &other, &other_request).unwrap();
    assert!(
        f.core
            .open_capture_result(&other.draft_id, &different_id)
            .is_err()
    );
    assert_eq!(fs::read(&moved.path).unwrap(), original_bytes);
    assert_eq!(
        fs::read(f.core.journal_path(&operation_id, "plan")).unwrap(),
        original_plan
    );
    assert_eq!(
        f.core
            .read_capture_draft(&frozen.draft_id)
            .unwrap()
            .pending
            .unwrap()
            .wire,
        frozen.pending.unwrap().wire
    );
}

#[test]
fn retired_proof_revision_cannot_roll_back_and_a_new_live_producer_keeps_old_own_fragments() {
    let mut f = Fixture::new();
    let mut value = input("private body");
    let old = CaptureRawInput {
        field_id: "field".into(),
        value_path: vec![],
        control_role: "text".into(),
        text: CaptureText::from_text("PRIVATE FIELD"),
        binding: CaptureRawBinding::Exact,
        author: Some(CaptureRawAuthor {
            text: CaptureText::from_text("OLD OWN"),
            provenance: CaptureRawProvenance {
                producer_id: fresh(),
                input_id: "a".repeat(32),
                revision: 2,
            },
        }),
    };
    let mut detached = old.clone();
    detached.author.as_mut().unwrap().provenance.input_id = "c".repeat(32);
    let retirement_id = fresh();
    value.raw_inputs.push(old.clone());
    value.detached_raw_inputs.push(CaptureDetachedRawInput {
        retirement_id: retirement_id.clone(),
        reason: CaptureRawRetirement::Structure,
        input: detached.clone(),
    });
    let root = f.root.clone();
    let record = f
        .core
        .create_targeted_capture_draft(&fresh(), &fresh(), value, &root, &root, None)
        .unwrap();
    let path = f.path(&record);
    let bytes = fs::read(&path).unwrap();
    for variant in ["oldRevision", "changedBytes", "differentProducer"] {
        let mut author = detached.clone().author_only().unwrap();
        match variant {
            "oldRevision" => author.provenance.revision = 1,
            "changedBytes" => author.text = CaptureText::from_text("changed"),
            _ => author.provenance.producer_id = fresh(),
        }
        let command = CaptureAuthorInput {
            detached_raw_inputs: vec![CaptureDetachedAuthorInput {
                retirement_id: retirement_id.clone(),
                reason: CaptureRawRetirement::Structure,
                input: author,
            }],
            ..Default::default()
        };
        assert!(
            f.core
                .update_capture_author_input(&record.draft_id, record.revision, &fresh(), command)
                .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    let mut new = old.author_only().unwrap();
    new.provenance.producer_id = fresh();
    new.provenance.revision = 1;
    new.text = CaptureText::from_text("NEW OWN");
    let command = CaptureAuthorInput {
        raw_inputs: vec![new],
        ..Default::default()
    };
    f.core
        .update_capture_author_input(&record.draft_id, record.revision, &fresh(), command)
        .unwrap();
    let f = f.restart();
    let saved = f.core.read_capture_draft(&record.draft_id).unwrap();
    assert_eq!(
        saved.current_input.raw_inputs[0].text.utf8().unwrap(),
        "PRIVATE FIELD"
    );
    assert_eq!(saved.current_input.detached_raw_inputs.len(), 2);
    let pure = f
        .core
        .read_capture_author_recovery(&record.draft_id)
        .unwrap();
    assert_eq!(pure.raw_inputs[0].text.utf8().unwrap(), "NEW OWN");
    assert!(
        pure.detached_raw_inputs
            .iter()
            .all(|entry| entry.input.text.utf8().unwrap() == "OLD OWN")
    );
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
    initial.version = 99;
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
    initial.version = 2;
    let mut bytes = serde_json::to_vec(&initial).unwrap();
    bytes.push(b' ');
    fs::write(&path, &bytes).unwrap();
    assert!(f.core.read_capture_draft(&initial.draft_id).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn retired_raw_survives_restart_and_owner_projection_never_returns_protected_candidates() {
    let mut f = Fixture::new();
    let root = f.root.clone();
    let mut authored = input("own text");
    authored.template_id = Some("protected-template".into());
    authored.type_ids.push("protected-type".into());
    authored.fields.push(CaptureFieldInput {
        field_id: "private/field".into(),
        value_state: CaptureFieldValueState::Valid {
            value: weftext_data::NativeValue::Text("protected-candidate".into()),
        },
    });
    authored.detached_raw_inputs.push(CaptureDetachedRawInput {
        retirement_id: fresh(),
        reason: CaptureRawRetirement::Structure,
        input: CaptureRawInput {
            field_id: "private/field".into(),
            value_path: vec![weftext_data::ValuePath::Index { index: 0 }],
            control_role: "decimal".into(),
            text: CaptureText {
                utf16: "PRIVATE DEFAULT"
                    .encode_utf16()
                    .chain([45, 0xd800, 0])
                    .collect(),
            },
            binding: CaptureRawBinding::Exact,
            author: Some(CaptureRawAuthor {
                text: CaptureText {
                    utf16: vec![45, 0xd800, 0],
                },
                provenance: CaptureRawProvenance {
                    producer_id: fresh(),
                    input_id: "a".repeat(32),
                    revision: 3,
                },
            }),
        },
    });
    let created = f
        .core
        .with_actor_for_root("deployment:alice:structure", &root, |core| {
            core.create_targeted_capture_draft(
                &fresh(),
                &fresh(),
                authored,
                &root,
                &root,
                Some("inbox".into()),
            )
        })
        .unwrap();
    let mut f = f.restart();
    assert!(
        f.core
            .with_actor("deployment:alice:structure", |core| core
                .read_capture_draft(&created.draft_id))
            .is_err()
    );
    let own = f
        .core
        .with_actor("deployment:alice:structure", |core| {
            core.read_capture_author_recovery(&created.draft_id)
        })
        .unwrap();
    assert_eq!(own.detached_raw_inputs[0].input.text.utf16, [45, 0xd800, 0]);
    let projected = serde_json::to_string(&own).unwrap();
    for protected in [
        "protected-template",
        "protected-type",
        "protected-candidate",
        "PRIVATE DEFAULT",
        "rootHint",
        "formBasis",
        "wireDigest",
        "\"wire\"",
    ] {
        assert!(
            !projected.contains(protected),
            "projection leaked {protected}"
        );
    }
    assert!(
        f.core
            .with_actor("deployment:bob:structure", |core| core
                .read_capture_author_recovery(&created.draft_id))
            .is_err()
    );
    assert!(
        f.core
            .read_capture_author_recovery(&created.draft_id)
            .is_err()
    );
}

#[test]
fn author_command_fingerprint_matches_browser_exact_wire_fixture() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../app/src/capture-author-wire.fixture.json"
    ))
    .unwrap();
    let input: CaptureAuthorInput = serde_json::from_value(fixture["input"].clone()).unwrap();
    author_input_valid(&input).unwrap();
    assert_eq!(
        capture_fingerprint(&(
            fixture["expectedRevision"].as_u64().unwrap(),
            "authorInput",
            &input
        ))
        .unwrap(),
        fixture["fingerprint"].as_str().unwrap()
    );
}

#[test]
fn private_checkpoint_first_fences_late_input_across_restart_without_touching_creation_effect() {
    let mut f = Fixture::new();
    let mut current = input("current B");
    current.body.utf16.extend([0xd800, 0]);
    current.fields.push(CaptureFieldInput {
        field_id: "protected/field".into(),
        value_state: CaptureFieldValueState::Valid {
            value: weftext_data::NativeValue::Text("protected candidate".into()),
        },
    });
    let root = f.root.clone();
    let created = f
        .core
        .create_targeted_capture_draft(&fresh(), &fresh(), current, &root, &root, None)
        .unwrap();
    let pending = frozen(&created);
    let saved = f
        .core
        .freeze_capture_submission(
            &created.draft_id,
            created.revision,
            &fresh(),
            pending.clone(),
        )
        .unwrap();
    let before_input = serde_json::to_vec(&saved.current_input).unwrap();
    let before_pending = serde_json::to_vec(&saved.pending).unwrap();
    let checkpoint_id = fresh();
    let proof = f
        .core
        .checkpoint_capture_input(&saved.draft_id, saved.revision, &checkpoint_id)
        .unwrap();
    assert_eq!(proof.revision, saved.revision + 1);
    assert_eq!(proof.input_revision, saved.input_revision);
    let mut f = f.restart();
    let actual = f.core.read_capture_draft(&saved.draft_id).unwrap();
    assert_eq!(
        serde_json::to_vec(&actual.current_input).unwrap(),
        before_input
    );
    assert_eq!(serde_json::to_vec(&actual.pending).unwrap(), before_pending);
    let original = CaptureAuthorInput {
        texts: author_texts(CaptureText::from_text("late A")),
        ..Default::default()
    };
    let original_id = fresh();
    let bytes = fs::read(f.path(&actual)).unwrap();
    assert!(
        f.core
            .update_capture_author_input(
                &saved.draft_id,
                saved.revision,
                &original_id,
                original.clone()
            )
            .is_err()
    );
    assert_eq!(fs::read(f.path(&actual)).unwrap(), bytes);
    let lookup = f
        .core
        .capture_mutation_status(
            &saved.draft_id,
            &checkpoint_id,
            &CaptureMutationCommand::Checkpoint {
                expected_revision: saved.revision,
            },
        )
        .unwrap();
    assert_eq!(lookup.status, "applied");
    // Installation/reply can be lost; original checkpoint retry does not advance again.
    assert_eq!(
        f.core
            .checkpoint_capture_input(&saved.draft_id, saved.revision, &checkpoint_id)
            .unwrap()
            .revision,
        proof.revision
    );
    assert!(
        f.core
            .checkpoint_capture_input(&saved.draft_id, saved.revision + 1, &checkpoint_id)
            .is_err()
    );
    assert_eq!(fs::read(f.path(&actual)).unwrap(), bytes);
    assert_eq!(
        f.core
            .capture_mutation_status(
                &saved.draft_id,
                &checkpoint_id,
                &CaptureMutationCommand::Checkpoint {
                    expected_revision: saved.revision + 1
                }
            )
            .unwrap()
            .status,
        "differentCommand"
    );
}

#[test]
fn original_input_first_conflicts_checkpoint_and_query_keeps_original_evidence() {
    let mut f = Fixture::new();
    let created = f.create();
    let pending = frozen(&created);
    let saved = f
        .core
        .freeze_capture_submission(&created.draft_id, created.revision, &fresh(), pending)
        .unwrap();
    let original_id = fresh();
    let original = CaptureAuthorInput {
        texts: author_texts(CaptureText {
            utf16: vec![65, 0xd800, 0],
        }),
        ..Default::default()
    };
    let applied = f
        .core
        .update_capture_author_input(
            &saved.draft_id,
            saved.revision,
            &original_id,
            original.clone(),
        )
        .unwrap();
    let checkpoint_id = fresh();
    let mut f = f.restart();
    let bytes = fs::read(f.path(&saved)).unwrap();
    assert!(
        f.core
            .checkpoint_capture_input(&saved.draft_id, saved.revision, &checkpoint_id)
            .is_err()
    );
    let (status, actual) = f
        .core
        .capture_author_mutation_status(&saved.draft_id, &original_id, saved.revision, &original)
        .unwrap();
    assert_eq!(status, "applied");
    assert_eq!(actual.revision, applied.revision);
    assert_eq!(
        actual
            .texts
            .body
            .iter()
            .find(|proof| proof.provenance.input_id == "b".repeat(32))
            .unwrap()
            .text
            .utf16,
        [65, 0xd800, 0]
    );
    assert!(actual.pending.is_some());
    let mut changed = original;
    changed.texts.body[0].text.utf16.push(66);
    assert_eq!(
        f.core
            .capture_author_mutation_status(&saved.draft_id, &original_id, saved.revision, &changed)
            .unwrap()
            .0,
        "differentCommand"
    );
    assert!(
        f.core
            .update_capture_author_input(&saved.draft_id, saved.revision, &original_id, changed)
            .is_err()
    );
    assert_eq!(fs::read(f.path(&saved)).unwrap(), bytes);
}

#[test]
fn unknown_capture_cannot_advance_and_exact_original_fence_preserves_new_input() {
    let mut f = Fixture::new();
    let created = f.create();
    let request = frozen(&created);
    let pending = f
        .core
        .freeze_capture_submission(
            &created.draft_id,
            created.revision,
            &fresh(),
            request.clone(),
        )
        .unwrap();
    let bytes = fs::read(f.path(&pending)).unwrap();
    assert!(
        f.core
            .advance_capture_submission(
                &pending.draft_id,
                pending.revision,
                &fresh(),
                &request.operation_id,
                true
            )
            .is_err()
    );
    assert_eq!(fs::read(f.path(&pending)).unwrap(), bytes);
    f.core
        .settle_local_attempt(
            &request.operation_id,
            &request.request_kind,
            &request.wire_digest,
        )
        .unwrap();
    let next = f
        .core
        .update_capture_input(
            &pending.draft_id,
            pending.revision,
            &fresh(),
            input("new current input"),
        )
        .unwrap();
    let mutation = fresh();
    let advanced = f
        .core
        .advance_capture_submission(
            &next.draft_id,
            next.revision,
            &mutation,
            &request.operation_id,
            true,
        )
        .unwrap();
    assert!(advanced.pending.is_none());
    assert_eq!(
        advanced.current_input.body.utf8().unwrap(),
        "new current input"
    );
    assert_eq!(
        f.core
            .capture_mutation_status(
                &next.draft_id,
                &mutation,
                &CaptureMutationCommand::Advance {
                    expected_revision: next.revision,
                    operation_id: request.operation_id.clone(),
                    reset_input: true,
                }
            )
            .unwrap()
            .status,
        "applied"
    );
    let advanced = f
        .restart()
        .core
        .read_capture_draft(&advanced.draft_id)
        .unwrap();
    assert_eq!(
        advanced.current_input.body.utf8().unwrap(),
        "new current input"
    );
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
        value_state: CaptureFieldValueState::Valid {
            value: weftext_data::NativeValue::Text("x".repeat(MAX_CAPTURE_RECORD_BYTES + 1)),
        },
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
        // Unsupported versions still route by scope. The sparse, invalid body
        // must never be decoded or charged to the selected library's budget.
        file.write_all(b",\"version\":1,").unwrap();
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
fn unsupported_selected_capture_versions_precede_new_body_decoding_and_preserve_originals() {
    let mut f = Fixture::new();
    let selected = f.create();
    let path = f.path(&selected);
    let scope = serde_json::to_string(&selected.scope).unwrap();

    // A v1-shaped record has none of the new required proof/cut/kind fields.
    // Keep scope/version first while making an independently encoded fixture;
    // the unsupported-version result must precede any body-shape diagnosis.
    let mut old = serde_json::to_value(&selected).unwrap();
    let object = old.as_object_mut().unwrap();
    object.remove("scope");
    object.remove("version");
    object.remove("inputCut");
    object
        .get_mut("lastMutation")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("kind");
    let input = object
        .get_mut("currentInput")
        .unwrap()
        .as_object_mut()
        .unwrap();
    for field in [
        "rootInput",
        "authorTexts",
        "authorBodyMode",
        "detachedRawInputs",
    ] {
        input.remove(field);
    }
    let body = serde_json::to_string(&old).unwrap();
    let missing_new_fields =
        format!("{{\"scope\":{scope},\"version\":1,{}", &body[1..]).into_bytes();
    let broken_old_body =
        format!("{{\"scope\":{scope},\"version\":1,\"currentInput\":NOT JSON").into_bytes();
    let broken_future_body =
        format!("{{\"scope\":{scope},\"version\":3,\"currentInput\":NOT JSON").into_bytes();
    for (version, bytes) in [
        (1, missing_new_fields),
        (1, broken_old_body),
        (3, broken_future_body),
    ] {
        fs::write(&path, &bytes).unwrap();
        for error in [
            validate_capture_record(&path, &f.data, &bytes).unwrap_err(),
            f.core.read_capture_draft(&selected.draft_id).unwrap_err(),
            f.core
                .read_capture_author_recovery(&selected.draft_id)
                .unwrap_err(),
            f.core.list_capture_drafts().unwrap_err(),
            f.backup(&format!("unsupported-{version}-{}", fresh()))
                .unwrap_err(),
        ] {
            assert_eq!(error.code, "captureRecordUnavailable");
            assert_eq!(
                error.explanation,
                UiMessage::error_capture_version_unsupported(
                    weftext_messages::params::ErrorCaptureVersionUnsupported {
                        actual: version.to_string(),
                        expected: "2".into()
                    }
                )
            );
        }
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn backup_excludes_unsupported_foreign_and_device_records_without_changing_bytes() {
    for version in [1, 3] {
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
        let originals = [&foreign, &device]
            .into_iter()
            .map(|draft| {
                let scope = serde_json::to_string(&draft.scope).unwrap();
                let bytes =
                    format!("{{\"scope\":{scope},\"version\":{version},\"currentInput\":NOT JSON")
                        .into_bytes();
                let path = f.path(draft);
                fs::write(&path, &bytes).unwrap();
                (path, bytes)
            })
            .collect::<Vec<_>>();
        let plan = f.backup(&format!("exclude-unsupported-{version}")).unwrap();
        let files = f.core.prepared_operation_files(&plan.operation_id).unwrap();
        assert!(
            files
                .iter()
                .any(|file| file.path.ends_with(&format!("{}.json", selected.draft_id)))
        );
        for excluded in [&foreign, &device] {
            assert!(
                !files
                    .iter()
                    .any(|file| file.path.ends_with(&format!("{}.json", excluded.draft_id)))
            );
        }
        f.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap();
        for (path, original) in originals {
            assert_eq!(fs::read(path).unwrap(), original);
        }
    }
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
fn prefix_reader_never_normalizes_keys_or_reads_body_and_always_restores_cursor() {
    let f = Fixture::new();
    let header = CaptureScopeHeader {
        owner: CaptureOwner::Local,
        library: CaptureLibrary::DeviceUnassigned,
    };
    let header_bytes = serde_json::to_vec(&header).unwrap();
    let mut allowed = b"{\"scope\":".to_vec();
    allowed.extend_from_slice(&header_bytes);
    allowed.extend_from_slice(b",\"version\":2,");
    // No closing record or byte of author payload is required to route metadata.
    let mut cursor = std::io::Cursor::new(allowed);
    cursor.set_position(3);
    assert_eq!(
        read_capture_prefix(&mut cursor).unwrap(),
        CaptureRecordPrefix {
            scope: header.clone(),
            version: CAPTURE_RECORD_VERSION,
        }
    );
    assert_eq!(cursor.position(), 3);
    let mut wrong = b"{\"s cope\":".to_vec();
    wrong.extend_from_slice(&header_bytes);
    let mut cursor = std::io::Cursor::new(wrong);
    cursor.set_position(2);
    assert!(read_capture_prefix(&mut cursor).is_err());
    assert_eq!(cursor.position(), 2);
    for suffix in [
        ",\"ver sion\":2,",
        ",\"version\":02,",
        ",\"version\":2.0,",
        ",\"version\":-1,",
        ",\"version\":4294967296,",
        ",\"currentInput\":{},\"version\":2,",
    ] {
        let mut bytes = b"{\"scope\":".to_vec();
        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(suffix.as_bytes());
        let mut cursor = std::io::Cursor::new(bytes);
        cursor.set_position(1);
        assert!(read_capture_prefix(&mut cursor).is_err(), "{suffix}");
        assert_eq!(cursor.position(), 1);
    }
    let mut oversized = b"{\"scope\":".to_vec();
    oversized.extend_from_slice(&header_bytes);
    oversized.extend_from_slice(b",\"version\":");
    oversized.extend(std::iter::repeat_n(b'9', MAX_CAPTURE_PREFIX_BYTES));
    let mut cursor = std::io::Cursor::new(oversized);
    cursor.set_position(2);
    assert_eq!(read_capture_prefix(&mut cursor).unwrap_err().code, "budget");
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

#[test]
fn full_input_lost_reply_uses_typed_original_digest_and_author_only_never_promotes_by_target_or_checkpoint()
 {
    let mut f = Fixture::new();
    let created = f.create();
    let original_id = fresh();
    let original = input("full original A");
    let digest = full_input_fingerprint(created.revision, &original).unwrap();
    let applied = f
        .core
        .update_capture_input(
            &created.draft_id,
            created.revision,
            &original_id,
            original.clone(),
        )
        .unwrap();
    let mut f = f.restart();
    let path = f.path(&applied);
    let bytes = fs::read(&path).unwrap();
    let (status, proof) = f
        .core
        .capture_input_mutation_status(&applied.draft_id, &original_id, created.revision, &digest)
        .unwrap();
    assert_eq!(status, "applied");
    assert_eq!(proof.input_cut, CaptureInputCut::Full);
    assert_eq!(proof.last_mutation.kind, CaptureMutationKind::Input);
    assert_eq!(
        f.core
            .capture_input_mutation_status(
                &applied.draft_id,
                &original_id,
                created.revision,
                &"0".repeat(64)
            )
            .unwrap()
            .0,
        "differentCommand"
    );
    assert!(
        f.core
            .with_actor("foreign:actor", |core| core.capture_input_mutation_status(
                &applied.draft_id,
                &original_id,
                created.revision,
                &digest
            ))
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    // Explicit deletion is still a positive own proof, not an absent fragment.
    let mut texts = original.author_texts.clone();
    texts.body[0].text = CaptureText::default();
    texts.body[0].provenance.revision += 1;
    let own = CaptureAuthorInput {
        texts,
        ..Default::default()
    };
    let own_id = fresh();
    let partial = f
        .core
        .update_capture_author_input(&applied.draft_id, applied.revision, &own_id, own.clone())
        .unwrap();
    assert_eq!(partial.input_cut, CaptureInputCut::AuthorOnly);
    assert!(partial.texts.body[0].text.utf16.is_empty());
    let mut f = f.restart();
    let same = f.core.read_capture_draft(&applied.draft_id).unwrap();
    assert_eq!(same.input_cut, CaptureInputCut::AuthorOnly);
    assert_eq!(same.current_input.body, original.body); // private former cut is not promoted
    assert_eq!(
        f.core
            .capture_input_mutation_status(&same.draft_id, &original_id, created.revision, &digest)
            .unwrap()
            .0,
        "supersededOrUnknown"
    );
    let (status, _) = f
        .core
        .capture_author_mutation_status(&same.draft_id, &own_id, applied.revision, &own)
        .unwrap();
    assert_eq!(status, "applied");
    let selected = f
        .core
        .select_capture_target(
            &same.draft_id,
            same.revision,
            &fresh(),
            &f.root,
            &f.root,
            None,
        )
        .unwrap();
    assert_eq!(selected.input_cut, CaptureInputCut::AuthorOnly);
    let checkpoint = f
        .core
        .checkpoint_capture_input(&same.draft_id, selected.revision, &fresh())
        .unwrap();
    assert_eq!(checkpoint.input_cut, CaptureInputCut::AuthorOnly);
    let latest = f.core.read_capture_draft(&same.draft_id).unwrap();
    let unchanged = fs::read(&path).unwrap();
    assert!(
        f.core
            .freeze_capture_submission(&latest.draft_id, latest.revision, &fresh(), frozen(&latest))
            .is_err()
    );
    assert!(
        f.core
            .capture_flush_receipt(
                &latest.draft_id,
                latest.revision,
                latest.input_revision,
                &latest.last_mutation.mutation_id
            )
            .is_err()
    );
    assert_eq!(
        f.core
            .capture_author_flush_receipt(
                &latest.draft_id,
                latest.revision,
                latest.input_revision,
                &latest.last_mutation.mutation_id
            )
            .unwrap()
            .input_cut,
        CaptureInputCut::AuthorOnly
    );
    assert_eq!(fs::read(&path).unwrap(), unchanged);
    let restored = f
        .core
        .update_capture_input(
            &latest.draft_id,
            latest.revision,
            &fresh(),
            input("fresh complete B"),
        )
        .unwrap();
    assert_eq!(restored.input_cut, CaptureInputCut::Full);
}

#[test]
fn full_digest_cannot_adopt_another_command_and_private_checkpoint_permanently_fences_late_full() {
    let mut f = Fixture::new();
    let created = f.create();
    let full = input("late A");
    let full_id = fresh();
    let checkpoint = f
        .core
        .checkpoint_capture_input(&created.draft_id, created.revision, &fresh())
        .unwrap();
    assert_eq!(
        f.core
            .capture_input_mutation_status(
                &created.draft_id,
                &checkpoint.last_mutation.mutation_id,
                created.revision,
                &checkpoint.last_mutation.fingerprint
            )
            .unwrap()
            .0,
        "differentCommand"
    );
    let mut f = f.restart();
    let saved = f.core.read_capture_draft(&created.draft_id).unwrap();
    let path = f.path(&saved);
    let bytes = fs::read(&path).unwrap();
    for candidate in [full.clone(), input("changed late A")] {
        assert!(
            f.core
                .update_capture_input(&created.draft_id, created.revision, &full_id, candidate)
                .is_err()
        );
    }
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let target = f
        .core
        .select_capture_target(
            &saved.draft_id,
            saved.revision,
            &fresh(),
            &f.root,
            &f.root,
            None,
        )
        .unwrap();
    assert_eq!(
        f.core
            .capture_input_mutation_status(
                &target.draft_id,
                &target.last_mutation.mutation_id,
                saved.revision,
                &target.last_mutation.fingerprint
            )
            .unwrap()
            .0,
        "differentCommand"
    );
    let pending = f
        .core
        .freeze_capture_submission(&target.draft_id, target.revision, &fresh(), frozen(&target))
        .unwrap();
    assert_eq!(
        f.core
            .capture_input_mutation_status(
                &pending.draft_id,
                &pending.last_mutation.mutation_id,
                target.revision,
                &pending.last_mutation.fingerprint
            )
            .unwrap()
            .0,
        "differentCommand"
    );
    // Full wins the race in another record: the old checkpoint cannot overwrite it.
    let other = f.create();
    let applied = f
        .core
        .update_capture_input(&other.draft_id, other.revision, &full_id, full.clone())
        .unwrap();
    let other_path = f.path(&applied);
    let before = fs::read(&other_path).unwrap();
    assert!(
        f.core
            .checkpoint_capture_input(&other.draft_id, other.revision, &fresh())
            .is_err()
    );
    assert_eq!(
        f.core
            .capture_input_mutation_status(
                &other.draft_id,
                &full_id,
                other.revision,
                &full_input_fingerprint(other.revision, &full).unwrap()
            )
            .unwrap()
            .0,
        "applied"
    );
    assert_eq!(fs::read(&other_path).unwrap(), before);
}

#[test]
fn full_input_fingerprint_matches_browser_canonical_fixture() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../app/src/capture-author-wire.fixture.json"
    ))
    .unwrap();
    let case = &fixture["fullInput"];
    let input: CaptureInput = serde_json::from_value(case["input"].clone()).unwrap();
    input_valid(&input).unwrap();
    assert_eq!(
        full_input_fingerprint(case["expectedRevision"].as_u64().unwrap(), &input).unwrap(),
        case["fingerprint"].as_str().unwrap()
    );
}
