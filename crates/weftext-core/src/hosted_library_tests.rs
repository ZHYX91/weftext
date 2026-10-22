use super::*;

const ACTOR: &str = "deployment:alice";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct MetadataOnlyReader {
    bytes: std::io::Cursor<Vec<u8>>,
    body: u64,
}
impl Read for MetadataOnlyReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        assert!(
            self.bytes.position() < self.body,
            "private body was read before qualification"
        );
        let count = output
            .len()
            .min((self.body - self.bytes.position()) as usize);
        self.bytes.read(&mut output[..count])
    }
}
impl std::io::Seek for MetadataOnlyReader {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(position)
    }
}

#[test]
fn qualified_private_reader_never_touches_foreign_body_and_rewinds_strict_metadata() {
    let (_temp, mut core, root) = scope_fixture();
    let operation = fresh();
    core.with_actor_for_root(ACTOR, &root, |core| core.actor_settle("Note", &operation))
        .unwrap();
    let original = fs::read(operation_file(&core, &operation)).unwrap();
    let marker = b",\"cancelled\":";
    let body = original
        .windows(marker.len())
        .position(|part| part == marker)
        .unwrap() as u64;
    let mut reader = MetadataOnlyReader {
        bytes: std::io::Cursor::new(original),
        body,
    };
    reader.bytes.set_position(1);
    let result = read_qualified_private_record(&mut reader, |reader| {
        let metadata = read_actor_operation_metadata(reader)?;
        core.validate_actor_operation_metadata(&metadata)?;
        assert_eq!(metadata.resource, "Note");
        Ok(None::<(ActorOperationMetadata, usize)>)
    })
    .unwrap();
    assert!(result.is_none());
    assert_eq!(reader.bytes.position(), 1);
    let rejected =
        read_qualified_private_record(&mut reader, |reader| -> Result<Option<((), usize)>> {
            let metadata = read_actor_operation_metadata(reader)?;
            core.validate_actor_operation_metadata(&metadata)?;
            Err(CoreError::new(
                "operationConflict",
                "Exact original resource differs",
            ))
        });
    assert_eq!(rejected.err().unwrap().code, "operationConflict");
    assert_eq!(reader.bytes.position(), 1);
}

fn scoped_status(
    core: &mut Core,
    root: &Path,
    category: &str,
    resource: &str,
    operation: &str,
) -> Result<String> {
    core.with_actor_for_root(ACTOR, root, |core| {
        Ok(match category {
            "save" => core.actor_save_status(resource, operation)?.unwrap().status,
            "field" => {
                core.actor_field_status(resource, "field", operation)?
                    .unwrap()
                    .status
            }
            "feature" => {
                core.actor_feature_status(resource, operation)?
                    .unwrap()
                    .status
            }
            "definition" => {
                core.actor_definition_status(resource, operation)?
                    .unwrap()
                    .status
            }
            "history" => {
                core.actor_history_restore_status(resource, operation)?
                    .unwrap()
                    .status
            }
            _ => {
                core.actor_owner_backup_status(resource, operation)?
                    .unwrap()
                    .status
            }
        })
    })
}

#[test]
fn every_actor_reader_qualifies_scope_resource_and_original_tuple_before_private_body() {
    let (temp, mut core, root) = scope_fixture();
    let foreign = temp.path().join("Foreign");
    core.create_library(&foreign).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    for category in [
        "save",
        "field",
        "feature",
        "definition",
        "history",
        "backup",
    ] {
        let operation = fresh();
        core.with_actor_for_root(ACTOR, &root, |core| {
            match category {
                "save" | "field" => {
                    let doc = core.open(&note.path, Some(root.clone()))?;
                    let request = HostedSaveRequest {
                        session_id: doc.session_id,
                        base_id: doc.base_id,
                        source: doc.source + "\nActual private author input",
                        operation_id: operation.clone(),
                    };
                    if category == "save" {
                        core.actor_save("Note", request)?;
                    } else {
                        core.actor_field_save(
                            "Note",
                            "field",
                            FP,
                            serde_json::json!("private field value"),
                            request,
                        )?;
                    }
                }
                "feature" => {
                    core.actor_feature_settle("Note", &operation)?;
                }
                "definition" => {
                    core.actor_definition_settle("Note", &operation)?;
                }
                "history" => {
                    core.actor_history_restore_settle("Note", &operation)?;
                }
                _ => {
                    core.actor_owner_backup_settle("Note", &operation)?;
                }
            }
            Ok(())
        })
        .unwrap();
        let path = operation_file(&core, &operation);
        let original = fs::read(&path).unwrap();
        let before = scoped_status(&mut core, &root, category, "Note", &operation).unwrap();
        let source_bytes = fs::read(&note.path).unwrap();
        let count = core.operations.len();
        let metadata: ActorOperation = serde_json::from_slice(&original).unwrap();
        let journal = core.operations.get(&metadata.core_operation_id).map(|_| {
            (
                fs::read(core.journal_path(&metadata.core_operation_id, "plan")).unwrap(),
                fs::read(core.journal_path(&metadata.core_operation_id, "commit")).unwrap(),
            )
        });
        let marker = b",\"cancelled\":";
        let body = original
            .windows(marker.len())
            .position(|part| part == marker)
            .unwrap();
        let mut damaged = original[..body].to_vec();
        damaged.extend_from_slice(b",\"cancelled\":BROKEN_PRIVATE_BODY");
        fs::write(&path, &damaged).unwrap();
        assert_eq!(
            scoped_status(&mut core, &foreign, category, "Note", &operation)
                .unwrap_err()
                .code,
            "operationConflict"
        );
        assert_eq!(
            scoped_status(&mut core, &root, category, "Other", &operation)
                .unwrap_err()
                .code,
            "operationConflict"
        );
        assert_eq!(
            scoped_status(&mut core, &root, category, "Note", &operation)
                .unwrap_err()
                .code,
            "invalidState"
        );
        assert_eq!(fs::read(&path).unwrap(), damaged);
        let mut wrong_tuple = metadata;
        let ActorInputScope::Qualified {
            scope: QualifiedInputScope::Library { binding },
        } = &mut wrong_tuple.scope
        else {
            panic!("original library scope");
        };
        binding.root = display(&temp.path().join("contradictory-original-locator"));
        let wrong = serde_json::to_vec(&wrong_tuple).unwrap();
        let body = wrong
            .windows(marker.len())
            .position(|part| part == marker)
            .unwrap();
        let mut wrong = wrong[..body].to_vec();
        wrong.extend_from_slice(b",\"cancelled\":BROKEN_PRIVATE_BODY");
        fs::write(&path, &wrong).unwrap();
        assert_eq!(
            scoped_status(&mut core, &foreign, category, "Other", &operation)
                .unwrap_err()
                .code,
            "privateEvidence"
        );
        assert_eq!(fs::read(&path).unwrap(), wrong);
        fs::write(&path, &original).unwrap();
        assert_eq!(
            scoped_status(&mut core, &root, category, "Note", &operation).unwrap(),
            before
        );
        assert_eq!(core.operations.len(), count);
        if let Some((plan, commit)) = journal {
            assert_eq!(
                fs::read(core.journal_path(&wrong_tuple.core_operation_id, "plan")).unwrap(),
                plan
            );
            assert_eq!(
                fs::read(core.journal_path(&wrong_tuple.core_operation_id, "commit")).unwrap(),
                commit
            );
        }
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read(&note.path).unwrap(), source_bytes);
    }
}

fn scope_fixture() -> (tempfile::TempDir, Core, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    (temp, core, root)
}
fn operation_file(core: &Core, operation: &str) -> PathBuf {
    core.data_dir
        .join("actors")
        .join(hash(ACTOR.as_bytes()))
        .join("operations")
        .join(format!("{operation}.json"))
}

#[test]
fn unbound_original_actor_input_is_not_read_in_another_bound_library() {
    let (temp, mut core, root) = scope_fixture();
    let foreign = temp.path().join("Foreign");
    core.create_library(&foreign).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    for field in [false, true] {
        let operation = fresh();
        core.with_actor(ACTOR, |core| {
            let doc = core.open(&note.path, Some(root.clone()))?;
            let request = HostedSaveRequest {
                session_id: doc.session_id,
                base_id: doc.base_id,
                source: doc.source + "\nactual unbound private input",
                operation_id: operation.clone(),
            };
            if field {
                core.actor_field_save(
                    "Note",
                    "field",
                    FP,
                    serde_json::json!("private cell"),
                    request,
                )?;
            } else {
                core.actor_save("Note", request)?;
            }
            Ok(())
        })
        .unwrap();
        let category = if field { "field" } else { "save" };
        assert_eq!(
            scoped_status(&mut core, &root, category, "Note", &operation).unwrap(),
            "committed"
        );
        let path = operation_file(&core, &operation);
        let original = fs::read(&path).unwrap();
        let record: ActorOperation = serde_json::from_slice(&original).unwrap();
        assert!(record.library.is_none());
        let plan = fs::read(core.journal_path(&record.core_operation_id, "plan")).unwrap();
        let commit = fs::read(core.journal_path(&record.core_operation_id, "commit")).unwrap();
        let source = fs::read(&note.path).unwrap();
        let count = core.operations.len();
        let marker = b",\"cancelled\":";
        let body = original
            .windows(marker.len())
            .position(|part| part == marker)
            .unwrap();
        let mut damaged = original[..body].to_vec();
        damaged.extend_from_slice(b",\"cancelled\":BROKEN_PRIVATE_BODY");
        fs::write(&path, &damaged).unwrap();
        assert_eq!(
            scoped_status(&mut core, &foreign, category, "Note", &operation)
                .unwrap_err()
                .code,
            "operationConflict"
        );
        assert_eq!(
            scoped_status(&mut core, &root, category, "Note", &operation)
                .unwrap_err()
                .code,
            "invalidState"
        );
        assert_eq!(fs::read(&path).unwrap(), damaged);
        fs::write(&path, &original).unwrap();
        assert_eq!(
            scoped_status(&mut core, &root, category, "Note", &operation).unwrap(),
            "committed"
        );
        assert_eq!(core.operations.len(), count);
        assert_eq!(
            fs::read(core.journal_path(&record.core_operation_id, "plan")).unwrap(),
            plan
        );
        assert_eq!(
            fs::read(core.journal_path(&record.core_operation_id, "commit")).unwrap(),
            commit
        );
        assert_eq!(fs::read(&note.path).unwrap(), source);
        assert_eq!(fs::read(path).unwrap(), original);
    }
}

#[test]
fn actor_library_scoped_save_fences_output_and_draft_have_first_original_header() {
    let (_temp, mut core, root) = scope_fixture();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let operation = fresh();
    let output = fresh();
    let fences = [fresh(), fresh(), fresh(), fresh(), fresh()];
    core.with_actor_for_root(ACTOR, &root, |core| {
        let opened = core.open(&note.path, Some(root.clone()))?;
        core.persist_draft(
            &opened.session_id,
            &(opened.source.clone() + "\nprivate draft"),
        )?;
        let draft: Draft =
            serde_json::from_slice(&fs::read(core.draft_path(Path::new(&note.path)))?)?;
        assert!(matches!(draft.scope, QualifiedInputScope::Library { .. }));
        core.actor_save(
            "Note",
            HostedSaveRequest {
                session_id: opened.session_id,
                base_id: opened.base_id,
                source: opened.source + "\nSaved.",
                operation_id: operation.clone(),
            },
        )?;
        core.actor_settle("Note", &fences[0])?;
        core.actor_field_settle("Note", "field", &fences[1])?;
        core.actor_feature_settle("Note", &fences[2])?;
        core.actor_definition_settle("Note", &fences[3])?;
        core.actor_history_restore_settle("Note", &fences[4])?;
        assert!(core.actor_output_claim(&output, FP)?);
        Ok(())
    })
    .unwrap();
    for id in std::iter::once(&operation).chain(fences.iter()) {
        let path = operation_file(&core, id);
        let bytes = fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"{\"scope\":"));
        let mut file = File::open(path).unwrap();
        let ActorInputScope::Qualified {
            scope: QualifiedInputScope::Library { binding },
        } = read_actor_input_header(&mut file).unwrap()
        else {
            panic!("missing original input scope");
        };
        assert_eq!(
            binding.workspace_id,
            core.require_workspace(&root).unwrap().workspace_id
        );
    }
    let bytes = fs::read(
        core.data_dir
            .join("actors")
            .join(hash(ACTOR.as_bytes()))
            .join("operations/outputs")
            .join(format!("{output}.json")),
    )
    .unwrap();
    assert!(bytes.starts_with(b"{\"library\":"));
    assert!(bytes.len() <= 4096);
}

#[test]
fn actor_library_scope_rejects_cross_root_session_records_and_restores_after_error_and_panic() {
    let (temp, mut core, root) = scope_fixture();
    let other = temp.path().join("Other");
    core.create_library(&other).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let id = fresh();
    let opened = core
        .with_actor_for_root(ACTOR, &root, |core| {
            core.actor_settle("Note", &id)?;
            core.open(&note.path, Some(root.clone()))
        })
        .unwrap();
    assert_eq!(
        core.with_actor_for_root(ACTOR, &other, |core| core
            .persist_draft(&opened.session_id, "foreign"))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        core.with_actor_for_root(ACTOR, &other, |core| core.actor_save_status("Note", &id))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert!(
        core.actor.is_none() && core.actor_library.is_none() && core.actor_library_path.is_none()
    );
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<()> =
            core.with_actor_for_root(ACTOR, &root, |_| panic!("controlled fixture"));
    }));
    assert!(panic.is_err());
    assert!(
        core.actor.is_none() && core.actor_library.is_none() && core.actor_library_path.is_none()
    );
    assert!(core.with_actor_for_root(ACTOR, &root, |_| Ok(())).is_ok());
}

#[test]
fn actor_library_ordinary_root_is_bound_but_damaged_library_controls_do_not_become_ordinary() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ordinary");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("note.typ"), "ordinary").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let id = fresh();
    core.with_actor_for_root(ACTOR, &root, |core| {
        let doc = core.open(root.join("note.typ"), Some(root.clone()))?;
        core.persist_draft(&doc.session_id, "ordinary draft")?;
        core.actor_settle("ordinary", &id)?;
        Ok(())
    })
    .unwrap();
    let record: ActorOperation =
        serde_json::from_slice(&fs::read(operation_file(&core, &id)).unwrap()).unwrap();
    assert!(record.library.is_some());
    assert!(record.library.unwrap().workspace_id.is_none());
    fs::write(root.join(".weftext-workspace.json"), b"broken").unwrap();
    assert_eq!(
        core.with_actor_for_root(ACTOR, &root, |_| Ok(()))
            .unwrap_err()
            .code,
        "unadmittedLibrary"
    );
}

#[test]
fn actor_library_legacy_records_and_recovered_draft_are_not_backfilled() {
    let (_temp, mut core, root) = scope_fixture();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let operation = fresh();
    let output = fresh();
    core.with_actor(ACTOR, |core| {
        let doc = core.open(&note.path, Some(root.clone()))?;
        core.persist_draft(&doc.session_id, "legacy draft")?;
        core.actor_settle("Note", &operation)?;
        core.actor_output_claim(&output, FP)?;
        Ok(())
    })
    .unwrap();
    let original = fs::read(operation_file(&core, &operation)).unwrap();
    core.with_actor_for_root(ACTOR, &root, |core| {
        assert_eq!(
            core.actor_save_status("Note", &operation)?.unwrap().status,
            "cancelled"
        );
        assert!(!core.actor_output_claim(&output, FP)?);
        let doc = core.open(&note.path, Some(root.clone()))?;
        assert_eq!(doc.recovered_draft.as_deref(), Some("legacy draft"));
        core.persist_draft(&doc.session_id, "another draft")?;
        let draft: Draft =
            serde_json::from_slice(&fs::read(core.draft_path(Path::new(&note.path)))?)?;
        assert!(matches!(draft.scope, QualifiedInputScope::Library { .. }));
        Ok(())
    })
    .unwrap();
    assert_eq!(
        fs::read(operation_file(&core, &operation)).unwrap(),
        original
    );
}

#[test]
fn actor_library_first_header_does_not_parse_payload_and_is_bounded_and_rewinds() {
    let (temp, mut core, root) = scope_fixture();
    let binding = core
        .with_actor_for_root(ACTOR, &root, |core| Ok(core.actor_library.clone().unwrap()))
        .unwrap();
    let path = temp.path().join("record.json");
    let mut bytes = b"{\"library\":".to_vec();
    bytes.extend(serde_json::to_vec(&binding).unwrap());
    bytes.extend_from_slice(b",\"feature\":THIS_IS_NOT_JSON");
    fs::write(&path, bytes).unwrap();
    let mut file = File::open(&path).unwrap();
    file.seek(SeekFrom::Start(7)).unwrap();
    assert!(
        matches!(read_actor_library_header(&mut file).unwrap(), ActorLibraryHeader::Bound(b) if b == binding)
    );
    assert_eq!(file.stream_position().unwrap(), 7);
    let mut oversized = binding;
    oversized.root = format!("D:/{}", "x".repeat(5000));
    fs::write(
        &path,
        format!(
            "{{\"library\":{}}}",
            serde_json::to_string(&oversized).unwrap()
        ),
    )
    .unwrap();
    assert_eq!(
        read_actor_library_header(&mut File::open(&path).unwrap())
            .err()
            .unwrap()
            .code,
        "budget"
    );
    fs::write(&path, b"{\"actor\":\"old\",\"feature\":BROKEN").unwrap();
    assert!(matches!(
        read_actor_library_header(&mut File::open(path).unwrap()).unwrap(),
        ActorLibraryHeader::LegacyUnbound
    ));
}

#[test]
fn actor_library_legacy_backup_validator_requires_original_plan_and_exact_namespace() {
    let (temp, mut core, root) = scope_fixture();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let operation = fresh();
    let unknown = fresh();
    core.with_actor(ACTOR, |core| {
        let doc = core.open(&note.path, Some(root.clone()))?;
        core.actor_save(
            "Note",
            HostedSaveRequest {
                session_id: doc.session_id,
                base_id: doc.base_id,
                source: doc.source + "\nlegacy saved",
                operation_id: operation.clone(),
            },
        )?;
        core.actor_settle("Note", &unknown)?;
        Ok(())
    })
    .unwrap();
    let binding = core
        .with_actor_for_root(ACTOR, &root, |core| Ok(core.actor_library.clone().unwrap()))
        .unwrap();
    let path = operation_file(&core, &operation);
    let bytes = fs::read(&path).unwrap();
    assert!(
        core.validate_unbound_actor_backup_record(&path, &bytes, &binding)
            .is_ok()
    );
    assert!(
        core.validate_scoped_actor_record(&path, &bytes, &binding)
            .is_err()
    );
    let unknown_path = operation_file(&core, &unknown);
    assert!(
        core.validate_unbound_actor_backup_record(
            &unknown_path,
            &fs::read(&unknown_path).unwrap(),
            &binding
        )
        .is_err()
    );
    let wrong = core
        .data_dir
        .join("actors")
        .join(hash(b"different actor"))
        .join("operations")
        .join(format!("{operation}.json"));
    assert!(
        core.validate_unbound_actor_backup_record(&wrong, &bytes, &binding)
            .is_err()
    );
    let other = temp.path().join("Other");
    core.create_library(&other).unwrap();
    let other_binding = core
        .with_actor_for_root(ACTOR, &other, |core| {
            Ok(core.actor_library.clone().unwrap())
        })
        .unwrap();
    assert!(
        core.validate_unbound_actor_backup_record(&path, &bytes, &other_binding)
            .is_err()
    );
}

#[test]
fn actor_authority_context_does_not_fork_original_input_owner_or_its_current_draft() {
    let (_temp, mut core, root) = scope_fixture();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let first = core
        .with_actor_for_root(ACTOR, &root, |core| {
            let opened = core.open(&note.path, Some(root.clone()))?;
            core.persist_draft(&opened.session_id, "original input")?;
            Ok(core.draft_path(Path::new(&note.path)))
        })
        .unwrap();
    core.with_actor(ACTOR, |core| {
        let opened = core.open(&note.path, Some(root.clone()))?;
        assert_eq!(opened.recovered_draft.as_deref(), Some("original input"));
        assert_eq!(core.draft_path(Path::new(&note.path)), first);
        core.persist_draft(&opened.session_id, "next input")
    })
    .unwrap();
    core.with_actor_for_root(ACTOR, &root, |core| {
        let opened = core.open(&note.path, Some(root.clone()))?;
        assert_eq!(opened.recovered_draft.as_deref(), Some("next input"));
        assert_eq!(core.draft_path(Path::new(&note.path)), first);
        Ok(())
    })
    .unwrap();
}

#[test]
fn no_input_actor_readers_reject_author_payload_and_never_hide_an_existing_commit() {
    let (_temp, mut core, root) = scope_fixture();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let saved = fresh();
    core.with_actor(ACTOR, |core| {
        let doc = core.open(&note.path, Some(root.clone()))?;
        core.actor_save(
            "Note",
            HostedSaveRequest {
                session_id: doc.session_id,
                base_id: doc.base_id,
                source: doc.source + "\nS",
                operation_id: saved.clone(),
            },
        )
    })
    .unwrap();
    let saved_path = operation_file(&core, &saved);
    let saved_original = fs::read(&saved_path).unwrap();
    let mut damaged: ActorOperation = serde_json::from_slice(&saved_original).unwrap();
    let core_id = damaged.core_operation_id.clone();
    let plan = fs::read(core.journal_path(&core_id, "plan")).unwrap();
    let commit = fs::read(core.journal_path(&core_id, "commit")).unwrap();
    damaged.scope = ActorInputScope::NoInput;
    damaged.cancelled = true;
    damaged.fingerprint.clear();
    fs::write(&saved_path, serde_json::to_vec(&damaged).unwrap()).unwrap();
    assert!(
        core.with_actor(ACTOR, |core| core.actor_save_status("Note", &saved))
            .is_err()
    );
    assert!(
        core.with_actor(ACTOR, |core| core.actor_settle("Note", &saved))
            .is_err()
    );
    assert!(core.committed(&core_id).unwrap());
    assert_eq!(fs::read(core.journal_path(&core_id, "plan")).unwrap(), plan);
    assert_eq!(
        fs::read(core.journal_path(&core_id, "commit")).unwrap(),
        commit
    );
    fs::write(&saved_path, &saved_original).unwrap();
    assert_eq!(
        core.with_actor(ACTOR, |core| core.actor_save_status("Note", &saved))
            .unwrap()
            .unwrap()
            .status,
        "committed"
    );
    for category in ["field", "feature", "definition", "history"] {
        let operation = fresh();
        core.with_actor(ACTOR, |core| {
            match category {
                "field" => {
                    core.actor_field_settle("Note", "field", &operation)?;
                }
                "feature" => {
                    core.actor_feature_settle("Note", &operation)?;
                }
                "definition" => {
                    core.actor_definition_settle("Note", &operation)?;
                }
                _ => {
                    core.actor_history_restore_settle("Note", &operation)?;
                }
            }
            Ok(())
        })
        .unwrap();
        let path = operation_file(&core, &operation);
        let original = fs::read(&path).unwrap();
        let mut damaged: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match category {
            "field" => damaged["field"]["cell"] = serde_json::json!("actual retained author input"),
            "feature" => {
                damaged["feature"]["preview"] = serde_json::json!({"source":"author input"})
            }
            _ => damaged["fingerprint"] = serde_json::json!(FP),
        }
        let bytes = serde_json::to_vec(&damaged).unwrap();
        fs::write(&path, &bytes).unwrap();
        let rejected = core
            .with_actor(ACTOR, |core| {
                Ok(match category {
                    "field" => core
                        .actor_field_status("Note", "field", &operation)
                        .is_err(),
                    "feature" => core.actor_feature_status("Note", &operation).is_err(),
                    "definition" => core.actor_definition_status("Note", &operation).is_err(),
                    _ => core
                        .actor_history_restore_status("Note", &operation)
                        .is_err(),
                })
            })
            .unwrap();
        assert!(
            rejected,
            "{category} reader accepted a contradictory no-input record"
        );
        assert!(core.validate_no_input_actor_fence(&path, &bytes).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::write(&path, &original).unwrap();
        core.validate_no_input_actor_fence(&path, &original)
            .unwrap();
    }
}

#[test]
fn original_scope_headers_reject_duplicate_members_without_reading_payload_and_rewind() {
    let (_temp, core, root) = scope_fixture();
    let scope = core
        .capture_file_save_scope(&root.join("prospective.typ"))
        .unwrap();
    let QualifiedInputScope::Library { binding } = &scope else {
        panic!("library scope")
    };
    let values = [
        ("input", serde_json::to_string(&scope).unwrap()),
        (
            "actor",
            serde_json::to_string(&ActorInputScope::Qualified {
                scope: scope.clone(),
            })
            .unwrap(),
        ),
        (
            "authority",
            serde_json::to_string(&binding.actor_binding()).unwrap(),
        ),
    ];
    for (kind, value) in values {
        let duplicate = value.replace(
            "\"workspaceId\":",
            &format!("\"workspaceId\":\"{}\",\"workspaceId\":", fresh()),
        );
        let key = if kind == "authority" {
            "library"
        } else {
            "scope"
        };
        let bytes = format!("{{\"{key}\":{duplicate},\"source\":BROKEN_PRIVATE_BODY").into_bytes();
        let mut reader = std::io::Cursor::new(bytes);
        reader.set_position(1);
        let rejected = match kind {
            "input" => file_save_scope::read_input_scope(&mut reader).is_err(),
            "actor" => read_actor_input_header(&mut reader).is_err(),
            _ => read_actor_library_header(&mut reader).is_err(),
        };
        assert!(rejected, "{kind} accepted duplicate ownership members");
        assert_eq!(reader.position(), 1);
    }
}

#[test]
fn library_author_producers_reject_device_scope_before_durable_and_can_settle_original() {
    for navigation in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("ordinary");
        fs::create_dir(&root).unwrap();
        let path = root.join("input.typ");
        fs::write(&path, "B").unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let operation = fresh();
        let initial_plans = core.operations.len();
        core.with_actor_for_root(ACTOR, &root, |core| {
            // A missing library cannot issue an admitted snapshot. The
            // bounded author proposal must be rejected before persistence,
            // independently of the later planner's basis qualification.
            let request = OperationRequest {
                operation_id: fresh(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&path),
                        base_id: Some(fresh()),
                        before: Some(b"B".to_vec()),
                        after: b"author proposal".to_vec(),
                    }],
                },
            };
            let attempt = |core: &mut Core| {
                if navigation {
                    core.actor_feature_prepare_navigation(
                        "ordinary",
                        &operation,
                        FP,
                        &root,
                        HostedNavigationIntent::AdmitDirectory {
                            path: display(&root),
                        },
                        serde_json::json!({"reviewed":true}),
                    )
                } else {
                    core.actor_feature_prepare(
                        "ordinary",
                        &operation,
                        FP,
                        request.clone(),
                        serde_json::json!({"reviewed":true}),
                        &[],
                    )
                }
            };
            assert_eq!(attempt(core).unwrap_err().code, "privateEvidence");
            assert!(!operation_file(core, &operation).exists());
            assert_eq!(core.operations.len(), initial_plans);
            assert!(core.actor_feature_status("ordinary", &operation)?.is_none());
            assert_eq!(
                core.actor_feature_settle("ordinary", &operation)?.status,
                "cancelled"
            );
            assert_eq!(attempt(core)?.status, "cancelled");
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"B");
        assert_eq!(core.operations.len(), initial_plans);
    }
}

#[test]
fn no_input_fence_backup_validation_requires_exact_original_actor_namespace_and_uuid_leaf() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let operation = fresh();
    core.with_actor(ACTOR, |core| core.actor_settle("ordinary", &operation))
        .unwrap();
    let path = operation_file(&core, &operation);
    let bytes = fs::read(&path).unwrap();
    core.validate_no_input_actor_fence(&path, &bytes).unwrap();
    for invalid in [
        path.with_file_name("not-a-uuid.json"),
        path.with_extension("json.partial"),
        path.parent()
            .unwrap()
            .join("nested")
            .join(path.file_name().unwrap()),
    ] {
        assert!(
            core.validate_no_input_actor_fence(&invalid, &bytes)
                .is_err()
        );
    }
    assert_eq!(fs::read(path).unwrap(), bytes);
}
