use super::*;

#[test]
fn owner_backup_reused_root_locator_excludes_original_foreign_workspace_and_all_its_private_families()
 {
    for legacy_actor in [false, true] {
        let mut f = Fixture::new();
        let workspace_a = f.core.require_workspace(&f.root).unwrap().workspace_id;
        let (actor_op, claim) = f.actor_facts("alice", false, "A_PRIVATE_SECRET");
        let actorfile = f
            .actor_root("alice")
            .join("operations")
            .join(format!("{actor_op}.json"));
        let actor_core_id = serde_json::from_slice::<serde_json::Value>(
            &fs::read(&actorfile).unwrap(),
        )
        .unwrap()["coreOperationId"]
            .as_str()
            .unwrap()
            .to_owned();
        if legacy_actor {
            let bytes = fs::read(&actorfile).unwrap();
            let key = bytes
                .windows(b",\"actor\"".len())
                .position(|w| w == b",\"actor\"")
                .unwrap();
            let library = bytes
                .windows(b",\"library\"".len())
                .position(|w| w == b",\"library\"")
                .unwrap();
            let mut without_authority = bytes[..library].to_vec();
            without_authority.extend_from_slice(&bytes[key..]);
            fs::write(&actorfile, without_authority).unwrap();
        }
        let opened_a = f.core.open(&f.note, Some(f.root.clone())).unwrap();
        f.core
            .persist_draft(&opened_a.session_id, "A_PRIVATE_DRAFT")
            .unwrap();
        let draft_path = f.core.draft_path(Path::new(&f.note));
        let capture_id = fresh();
        let capture = f
            .core
            .create_targeted_capture_draft(
                &capture_id,
                &fresh(),
                capture_draft::CaptureInput {
                    body: capture_draft::CaptureText::from_text("A_PRIVATE_CAPTURE"),
                    ..Default::default()
                },
                &f.root,
                &f.root,
                None,
            )
            .unwrap();
        let capture_path =
            capture_draft::capture_record_path(&f.data, &capture.scope.owner, &capture_id).unwrap();
        let reservation = fresh();
        let snapshot = f
            .core
            .read_snapshot(&f.root, std::slice::from_ref(&f.note))
            .unwrap()
            .files
            .remove(0);
        f.core
            .reserve_definition_key(&f.root, &f.note, &snapshot.base_id, &reservation)
            .unwrap();
        f.core
            .settle_local_attempt(&reservation, "save", &hash(b"A rejected original proposal"))
            .unwrap();
        let mut a_plans = f
            .core
            .operations
            .values()
            .filter(|plan| plan.workspace_id.as_deref() == Some(&workspace_a))
            .map(|plan| plan.operation_id.clone())
            .collect::<Vec<_>>();
        a_plans.push(actor_core_id);
        let originals = [
            &actorfile,
            &draft_path,
            &capture_path,
            &f.core.journal_path(&reservation, "definition-reservation"),
            &f.core.journal_path(&reservation, "resolution"),
        ]
        .into_iter()
        .map(|path| (path.clone(), fs::read(path).unwrap()))
        .collect::<Vec<_>>();
        let retained_root = f.temp.path().join("retained-original-A");
        fs::rename(&f.root, &retained_root).unwrap();
        f.core.create_library(&f.root).unwrap();
        let workspace_b = f.core.require_workspace(&f.root).unwrap().workspace_id;
        assert_ne!(workspace_a, workspace_b);
        let b = f.core.create_note_fixture(&f.root, &f.root, "B").unwrap();
        f.core
            .persist_draft(&b.session_id, "B selected draft")
            .unwrap();
        let preview = f.prepare("only-B");
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        let private = f.temp.path().join("only-B/private");
        let journal = private.join("journal");
        for name in fs::read_dir(&journal).unwrap().flat_map(|bucket| {
            fs::read_dir(bucket.unwrap().path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        }) {
            assert!(
                !a_plans
                    .iter()
                    .chain(std::iter::once(&reservation))
                    .any(|id| name.starts_with(&format!("{id}."))),
                "foreign original journal selected: {name}"
            );
        }
        assert!(
            !private
                .join("actors")
                .join(hash(b"alice"))
                .join("operations")
                .join(format!("{actor_op}.json"))
                .exists()
        );
        assert!(
            !private
                .join("actors")
                .join(hash(b"alice"))
                .join("operations/outputs")
                .join(format!("{claim}.json"))
                .exists()
        );
        assert!(
            !private
                .join("capture-drafts")
                .join(format!("{capture_id}.json"))
                .exists()
        );
        assert!(
            !private
                .join("drafts")
                .join(draft_path.file_name().unwrap())
                .exists()
        );
        for (path, original) in &originals {
            assert_eq!(fs::read(path).unwrap(), *original);
        }
        assert!(retained_root.join("A/A.typ").is_file());
        assert!(
            private
                .join("drafts")
                .join(f.core.draft_path(Path::new(&b.path)).file_name().unwrap())
                .is_file()
        );
    }
}

#[test]
fn owner_backup_file_save_scope_requires_original_root_witness_not_current_path_or_birth_index() {
    let mut f = Fixture::new();
    let ordinary = f.root.join("nested").join("ordinary.typ");
    fs::create_dir(ordinary.parent().unwrap()).unwrap();
    fs::write(&ordinary, "ordinary original").unwrap();
    let doc = f.core.open(&ordinary, None).unwrap();
    let id = fresh();
    f.core
        .save(SaveRequest {
            session_id: doc.session_id,
            base_id: doc.base_id,
            source: "selected ordinary private S".into(),
            operation_id: id.clone(),
        })
        .unwrap();
    let library = f
        .core
        .operations
        .values()
        .find(|plan| plan.kind == "createLibrary" && Path::new(&plan.path) == f.root)
        .unwrap();
    let witness = f.core.journal_path(
        &library.operation_id,
        &format!("object-{}", hash(library.path.as_bytes())),
    );
    let original = fs::read(&witness).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&original).unwrap();
    changed["identity"]["index"] = serde_json::json!(0);
    let bad = serde_json::to_vec(&changed).unwrap();
    fs::write(&witness, &bad).unwrap();
    assert!(
        f.core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&f.root),
                intent: OperationIntent::BackupLibrary {
                    destination: display(&f.temp.path().join("blocked"))
                }
            })
            .is_err()
    );
    assert_eq!(fs::read(&witness).unwrap(), bad);
    assert!(!f.temp.path().join("blocked").exists());
    fs::write(&witness, original).unwrap();
    let p = f.prepare("original-scope-verified");
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    assert!(
        f.temp
            .path()
            .join("original-scope-verified/private/journal")
            .join(&id[..2])
            .join(format!("{id}.plan.json"))
            .is_file()
    );
}

#[test]
fn owner_backup_preserves_current_ordinary_family_and_excludes_original_a_after_root_reuse_and_restart()
 {
    for family in ["saveAs", "save", "staged", "actor"] {
        let mut f = Fixture::new();
        let device = f.temp.path().join("device-input.typ");
        fs::write(&device, "device input").unwrap();
        let input = f.core.open(&device, None).unwrap();
        let target = f.root.join("nested/ordinary.typ");
        fs::create_dir(target.parent().unwrap()).unwrap();
        let save_as = fresh();
        f.core
            .save_as(SaveAsRequest {
                session_id: input.session_id,
                path: display(&target),
                source: "A PRIVATE ordinary SaveAs".into(),
                operation_id: save_as.clone(),
            })
            .unwrap();
        let mut ids = vec![save_as];
        let doc = f.core.open(&target, None).unwrap();
        assert_eq!(doc.kind, "ordinary");
        assert!(
            matches!(&f.core.sessions[&doc.session_id].file_scope, Ok(QualifiedInputScope::Library { binding }) if binding.workspace_id == f.core.require_workspace(&f.root).unwrap().workspace_id)
        );
        let mut actor_record = None;
        if family != "saveAs" {
            let id = fresh();
            if family == "staged" {
                crate::hosted::set_test_failpoint("saveStaged");
            }
            if family == "actor" {
                let actor_id = fresh();
                f.core
                    .with_actor_for_root("nested-author", target.parent().unwrap(), |core| {
                        let opened = core.open(&target, None)?;
                        core.actor_save(
                            "ordinary",
                            HostedSaveRequest {
                                input_revision: None,
                                session_id: opened.session_id,
                                base_id: opened.base_id,
                                source: "A PRIVATE hosted ordinary".into(),
                                operation_id: actor_id.clone(),
                            },
                        )
                    })
                    .unwrap();
                let record_path = f
                    .actor_root("nested-author")
                    .join("operations")
                    .join(format!("{actor_id}.json"));
                let record: serde_json::Value =
                    serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
                ids.push(record["coreOperationId"].as_str().unwrap().to_owned());
                actor_record = Some(record_path);
            } else {
                let result = f.core.save(SaveRequest {
                    session_id: doc.session_id.clone(),
                    base_id: doc.base_id,
                    source: "A PRIVATE ordinary proposal".into(),
                    operation_id: id.clone(),
                });
                assert_eq!(result.is_err(), family == "staged");
                ids.push(id);
            }
        }
        f.core
            .persist_draft(&doc.session_id, "A PRIVATE unsaved Y")
            .unwrap();
        let draft = f.core.draft_path(&target);
        let before = ids
            .iter()
            .map(|id| {
                (
                    f.core.journal_path(id, "plan"),
                    fs::read(f.core.journal_path(id, "plan")).unwrap(),
                )
            })
            .chain(std::iter::once((draft.clone(), fs::read(&draft).unwrap())))
            .chain(
                actor_record
                    .iter()
                    .map(|p| (p.clone(), fs::read(p).unwrap())),
            )
            .collect::<Vec<_>>();
        let selected = f.prepare("A-complete");
        f.core
            .execute_operation(&selected.operation_id, &selected.plan_id)
            .unwrap();
        for id in &ids {
            assert!(
                f.temp
                    .path()
                    .join("A-complete/private/journal")
                    .join(&id[..2])
                    .join(format!("{id}.plan.json"))
                    .is_file()
            );
        }
        if let Some(record) = &actor_record {
            assert!(
                f.temp
                    .path()
                    .join("A-complete/private")
                    .join(record.strip_prefix(&f.data).unwrap())
                    .is_file()
            );
        }
        if family == "staged" {
            // Backup above preserves the uncommitted proposal. Creating a new
            // library at its old root remains fenced until explicit recovery.
            let pending = ids.last().unwrap();
            assert!(!f.core.committed(pending).unwrap());
            assert_eq!(
                f.core
                    .resolve_recovery(pending, "keepCurrent")
                    .unwrap()
                    .status,
                "resolved"
            );
        }
        fs::rename(&f.root, f.temp.path().join("retained-A")).unwrap();
        f.core.create_library(&f.root).unwrap();
        // Autosave from an old live A Session retains A ownership; it cannot
        // discover B through its now reused old locator.
        f.core
            .persist_draft(&doc.session_id, "A PRIVATE later Y")
            .unwrap();
        let retained_draft = fs::read(&draft).unwrap();
        drop(f.core);
        f.core = Core::new(&f.data).unwrap();
        let b_ordinary = f.root.join("B-ordinary.typ");
        fs::write(&b_ordinary, "B source").unwrap();
        let b = f.core.open(&b_ordinary, None).unwrap();
        let b_id = fresh();
        f.core
            .save(SaveRequest {
                session_id: b.session_id,
                base_id: b.base_id,
                source: "B selected private S".into(),
                operation_id: b_id.clone(),
            })
            .unwrap();
        let selected = f.prepare("B-only");
        f.core
            .execute_operation(&selected.operation_id, &selected.plan_id)
            .unwrap();
        let b_private = f.temp.path().join("B-only/private");
        for id in &ids {
            assert!(
                !b_private
                    .join("journal")
                    .join(&id[..2])
                    .join(format!("{id}.plan.json"))
                    .exists()
            );
        }
        assert!(
            b_private
                .join("journal")
                .join(&b_id[..2])
                .join(format!("{b_id}.plan.json"))
                .is_file()
        );
        assert!(
            !b_private
                .join("drafts")
                .join(draft.file_name().unwrap())
                .exists()
        );
        assert_eq!(fs::read(&draft).unwrap(), retained_draft);
        for (path, bytes) in before {
            if path != draft {
                assert_eq!(fs::read(path).unwrap(), bytes);
            }
        }
    }
}

#[test]
fn owner_backup_same_logical_or_physical_root_with_contradictory_binding_remains_an_error() {
    let f = Fixture::new();
    let guard = ParentGuard::acquire(&f.core.backend, &f.root.join("prospective")).unwrap();
    let expected = ActorLibraryBinding {
        root: display(&f.root),
        root_identity: guard.directory_identity().unwrap(),
        workspace_id: Some(f.core.require_workspace(&f.root).unwrap().workspace_id),
    };
    let foreign_guard =
        ParentGuard::acquire(&f.core.backend, &f.foreign.join("prospective")).unwrap();
    let foreign = ActorLibraryBinding {
        root: expected.root.clone(),
        root_identity: foreign_guard.directory_identity().unwrap(),
        workspace_id: Some(f.core.require_workspace(&f.foreign).unwrap().workspace_id),
    };
    assert!(!selected_library_scope(Some(&foreign), &expected).unwrap());
    assert_eq!(
        selected_library_scope(
            Some(&ActorLibraryBinding {
                workspace_id: expected.workspace_id.clone(),
                ..foreign.clone()
            }),
            &expected
        )
        .unwrap_err()
        .code,
        "scopeChanged"
    );
    assert_eq!(
        selected_library_scope(
            Some(&ActorLibraryBinding {
                root_identity: expected.root_identity.clone(),
                ..foreign.clone()
            }),
            &expected
        )
        .unwrap_err()
        .code,
        "scopeChanged"
    );
    assert_eq!(
        selected_library_scope(
            Some(&ActorLibraryBinding {
                workspace_id: None,
                ..foreign
            }),
            &expected
        )
        .unwrap_err()
        .code,
        "scopeChanged"
    );
}

struct Fixture {
    temp: tempfile::TempDir,
    core: Core,
    root: PathBuf,
    foreign: PathBuf,
    note: String,
    foreign_note: String,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        Self::with_private_path("private")
    }
    fn with_private_path(relative: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join(relative);
        let mut core = Core::new(&data).unwrap();
        let root = temp.path().join("L1");
        let foreign = temp.path().join("L2");
        core.create_library(&root).unwrap();
        core.create_library(&foreign).unwrap();
        let note = core.create_note_fixture(&root, &root, "A").unwrap().path;
        let foreign_note = core
            .create_note_fixture(&foreign, &foreign, "B")
            .unwrap()
            .path;
        Self {
            temp,
            core,
            root,
            foreign,
            note,
            foreign_note,
            data,
        }
    }
    fn actor_facts(&mut self, actor: &str, foreign: bool, marker: &str) -> (String, String) {
        let (root, note) = if foreign {
            (&self.foreign, &self.foreign_note)
        } else {
            (&self.root, &self.note)
        };
        let operation = fresh();
        let claim = fresh();
        self.core
            .with_actor_for_root(actor, root, |c| {
                let doc = c.open(note, Some(root.clone()))?;
                let original = fs::read_to_string(note)?;
                c.actor_save(
                    "note",
                    HostedSaveRequest {
                        input_revision: None,
                        session_id: doc.session_id.clone(),
                        base_id: doc.base_id,
                        source: format!("{original}\n{marker}"),
                        operation_id: operation.clone(),
                    },
                )?;
                let reopened = c.open(note, Some(root.clone()))?;
                c.persist_draft(&reopened.session_id, &format!("draft {marker}"))?;
                c.actor_output_claim(&claim, &hash(marker.as_bytes()))?;
                Ok(())
            })
            .unwrap();
        (operation, claim)
    }
    fn prepare(&mut self, name: &str) -> OperationPreview {
        self.core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&self.root),
                intent: OperationIntent::BackupLibrary {
                    destination: display(&self.temp.path().join(name)),
                },
            })
            .unwrap()
    }
    fn actor_root(&self, actor: &str) -> PathBuf {
        self.data.join("actors").join(hash(actor.as_bytes()))
    }
}

#[test]
fn owner_backup_keeps_scoped_actor_drafts_history_unknown_fences_and_pending_source() {
    let mut f = Fixture::new();
    let (operation, claim) = f.actor_facts("alice", false, "SELECTED");
    f.actor_facts("bob", true, "FOREIGN_SECRET");
    // A pre-Plan fence is still an original scoped fact and must be retained.
    let unknown = fresh();
    let fingerprint = hash(b"unknown");
    let outside = f.temp.path().join("unused-spool");
    crate::hosted::set_test_failpoint("ownerBackupActorDurable");
    assert!(
        f.core
            .with_actor_for_root("alice", &f.root, |c| c.actor_owner_backup_prepare(
                "library",
                &unknown,
                &fingerprint,
                &outside,
                serde_json::json!({"intent":"ownerBackup"})
            ))
            .is_err()
    );
    // Keep a recovered proposal's original history in its original actor tree.
    let history = f.actor_root("alice").join("draft-history");
    let draft_entry = fs::read_dir(f.actor_root("alice").join("drafts"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::copy(&draft_entry, history.join("retained-original.json")).unwrap();
    let history_original = fs::read(&draft_entry).unwrap();
    // SourceWritten is a real unresolved Core operation, not a fabricated
    // status file. Backup preserves its Plan and current source without repair.
    let pending = fresh();
    crate::hosted::set_test_failpoint("sourceWritten");
    let note = f.note.clone();
    let source = format!("{}\nPENDING_SELECTED", fs::read_to_string(&note).unwrap());
    assert!(
        f.core
            .with_actor_for_root("alice", &f.root, |c| {
                let doc = c.open(&note, Some(f.root.clone()))?;
                // Explicitly review the pending actor proposal before this
                // separate source-write interruption; retain its exact record.
                c.resolve_draft(&doc.session_id, "mergeRecovered", Some(&source))?;
                c.save(SaveRequest {
                    session_id: doc.session_id,
                    base_id: doc.base_id,
                    source,
                    operation_id: pending.clone(),
                })
            })
            .is_err()
    );
    let pending_original = fs::read(f.core.journal_path(&pending, "plan")).unwrap();
    fs::write(f.data.join("credentials.json"), b"CREDENTIAL_SECRET").unwrap();
    fs::create_dir_all(f.root.join("empty")).unwrap();
    let p = f.prepare("backup");
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let target = f.temp.path().join("backup");
    let alice = target.join("private/actors").join(hash(b"alice"));
    assert_eq!(
        fs::read(alice.join("operations").join(format!("{operation}.json"))).unwrap(),
        fs::read(
            f.actor_root("alice")
                .join("operations")
                .join(format!("{operation}.json"))
        )
        .unwrap()
    );
    assert_eq!(
        fs::read(
            alice
                .join("operations/outputs")
                .join(format!("{claim}.json"))
        )
        .unwrap(),
        fs::read(
            f.actor_root("alice")
                .join("operations/outputs")
                .join(format!("{claim}.json"))
        )
        .unwrap()
    );
    assert!(
        alice
            .join("operations")
            .join(format!("{unknown}.json"))
            .exists()
    );
    assert_eq!(
        fs::read(alice.join("draft-history/retained-original.json")).unwrap(),
        history_original
    );
    assert!(!target.join("private/actors").join(hash(b"bob")).exists());
    assert!(!target.join("private/credentials.json").exists());
    assert!(target.join("library/empty").is_dir());
    assert_eq!(
        fs::read(
            target
                .join("private/journal")
                .join(&pending[..2])
                .join(format!("{pending}.plan.json"))
        )
        .unwrap(),
        pending_original
    );
    assert!(f.core.inspect_recovery(&pending).unwrap().status != "committed");
    let backed = fs::read_to_string(target.join("library/A/A.typ")).unwrap();
    assert!(backed.contains("PENDING_SELECTED"));
    assert!(!f.core.committed(&pending).unwrap());
}

#[test]
fn owner_backup_foreign_bound_payload_is_never_parsed_or_budgeted_as_selected() {
    let mut f = Fixture::new();
    f.actor_facts("bob", true, "foreign");
    let actor = f.actor_root("bob");
    let path = actor.join("operations").join(format!("{}.json", fresh()));
    let binding = f
        .core
        .with_actor_for_root("bob", &f.foreign, |c| Ok(c.actor_library.clone().unwrap()))
        .unwrap();
    let input = crate::hosted::ActorInputScope::Qualified {
        scope: f
            .core
            .capture_file_save_scope(Path::new(&f.foreign_note))
            .unwrap(),
    };
    let prefix = format!(
        "{{\"scope\":{},\"payload\":",
        serde_json::to_string(&input).unwrap()
    );
    let _ = binding;
    let mut file = File::create(&path).unwrap();
    file.write_all(prefix.as_bytes()).unwrap();
    // An invalid, over-budget foreign body proves there is no full JSON read.
    file.set_len((MAX_OPERATION_BYTES + 1) as u64).unwrap();
    drop(file);
    let p = f.prepare("backup");
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    assert!(
        !f.temp
            .path()
            .join("backup/private/actors")
            .join(hash(b"bob"))
            .exists()
    );
}

#[test]
fn owner_backup_late_foreign_fact_and_same_bytes_parent_replacement_are_stale() {
    for replace_parent in [false, true] {
        let mut f = Fixture::new();
        f.actor_facts("bob", true, "foreign");
        let p = f.prepare("backup");
        if replace_parent {
            let parent = f.actor_root("bob").join("operations/outputs");
            let retained = f.temp.path().join("retained-outputs");
            fs::rename(&parent, &retained).unwrap();
            fs::create_dir(&parent).unwrap();
            for entry in fs::read_dir(&retained).unwrap() {
                let entry = entry.unwrap();
                fs::copy(entry.path(), parent.join(entry.file_name())).unwrap();
            }
        } else {
            f.core
                .with_actor_for_root("bob", &f.foreign, |c| {
                    c.actor_output_claim(&fresh(), &hash(b"late"))
                })
                .unwrap();
        }
        assert!(
            f.core
                .execute_operation(&p.operation_id, &p.plan_id)
                .is_err()
        );
        let recovery = f.core.inspect_recovery(&p.operation_id).unwrap();
        assert_eq!(recovery.coherence, "thirdState");
        assert!(recovery.allowed_actions.is_empty());
        assert!(!f.temp.path().join("backup").exists());
    }
}

#[test]
fn owner_backup_same_bytes_selected_file_replacement_and_own_unknown_suffix_are_preserved() {
    for unknown_suffix in [false, true] {
        let mut f = Fixture::new();
        let (_, claim) = f.actor_facts("alice", false, "own");
        let p = f.prepare("backup");
        if unknown_suffix {
            fs::write(f.core.journal_path(&p.operation_id, "unexpected"), b"{}").unwrap();
        } else {
            let path = f
                .actor_root("alice")
                .join("operations/outputs")
                .join(format!("{claim}.json"));
            let retained = f.temp.path().join("original-claim.json");
            fs::rename(&path, &retained).unwrap();
            fs::copy(&retained, &path).unwrap();
        }
        assert!(
            f.core
                .execute_operation(&p.operation_id, &p.plan_id)
                .is_err()
        );
        let recovery = f.core.inspect_recovery(&p.operation_id).unwrap();
        assert_eq!(recovery.coherence, "thirdState");
        assert!(recovery.allowed_actions.is_empty());
        assert!(
            f.core
                .resolve_recovery(&p.operation_id, "adoptAfterImages")
                .is_err()
        );
        assert!(!f.temp.path().join("backup").exists());
        assert!(f.core.journal_path(&p.operation_id, "plan").exists());
    }
}

#[test]
fn owner_backup_missing_current_input_scope_is_unsupported_and_originals_remain_untouched() {
    for actor_record in [false, true] {
        let mut f = Fixture::new();
        let target = if actor_record {
            let (op, _) = f.actor_facts("alice", false, "retained author proposal");
            f.actor_root("alice")
                .join("operations")
                .join(format!("{op}.json"))
        } else {
            let doc = f.core.open(&f.note, Some(f.root.clone())).unwrap();
            f.core
                .persist_draft(&doc.session_id, "retained proposal")
                .unwrap();
            f.core.draft_path(Path::new(&f.note))
        };
        let mut old: serde_json::Value =
            serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
        old.as_object_mut().unwrap().remove("scope");
        let original = serde_json::to_vec(&old).unwrap();
        fs::write(&target, &original).unwrap();
        let error = f
            .core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&f.root),
                intent: OperationIntent::BackupLibrary {
                    destination: display(&f.temp.path().join("unsupported")),
                },
            })
            .unwrap_err();
        assert_eq!(error.code, "unsupportedInputScope");
        assert_eq!(fs::read(&target).unwrap(), original);
        assert!(!f.temp.path().join("unsupported").exists());
    }
}

#[test]
fn owner_backup_pending_actor_source_and_field_freeze_input_owner_before_core_plan_exists() {
    for field in [false, true] {
        let mut f = Fixture::new();
        let nested = f.root.join("nested");
        fs::create_dir(&nested).unwrap();
        let path = nested.join("ordinary.typ");
        fs::write(&path, "original ordinary B").unwrap();
        let external_id = fresh();
        f.core
            .with_actor_for_root("nested", &nested, |core| {
                let doc = core.open(&path, None)?;
                let request = HostedSaveRequest {
                    input_revision: None,
                    session_id: doc.session_id,
                    base_id: fresh(),
                    source: "A PRIVATE rejected but retained proposal".into(),
                    operation_id: external_id.clone(),
                };
                let error = if field {
                    core.actor_field_save(
                        "ordinary",
                        "field",
                        &hash(b"field intent"),
                        serde_json::json!("private field value"),
                        request,
                    )
                    .unwrap_err()
                } else {
                    core.actor_save("ordinary", request).unwrap_err()
                };
                assert_eq!(error.code, "staleBase");
                Ok(())
            })
            .unwrap();
        let actor = f
            .actor_root("nested")
            .join("operations")
            .join(format!("{external_id}.json"));
        let original = fs::read(&actor).unwrap();
        let record: serde_json::Value = serde_json::from_slice(&original).unwrap();
        assert!(record["library"]["workspaceId"].is_null());
        assert_eq!(
            record["scope"]["scope"]["binding"]["workspaceId"],
            f.core.require_workspace(&f.root).unwrap().workspace_id
        );
        assert!(
            !f.core
                .operations
                .contains_key(record["coreOperationId"].as_str().unwrap())
        );
        let selected = f.prepare("A-pending");
        f.core
            .execute_operation(&selected.operation_id, &selected.plan_id)
            .unwrap();
        assert_eq!(
            fs::read(
                f.temp
                    .path()
                    .join("A-pending/private")
                    .join(actor.strip_prefix(&f.data).unwrap())
            )
            .unwrap(),
            original
        );
        fs::rename(&f.root, f.temp.path().join("retained-A")).unwrap();
        f.core.create_library(&f.root).unwrap();
        drop(f.core);
        f.core = Core::new(&f.data).unwrap();
        let selected = f.prepare("B-without-A");
        f.core
            .execute_operation(&selected.operation_id, &selected.plan_id)
            .unwrap();
        assert!(
            !f.temp
                .path()
                .join("B-without-A/private")
                .join(actor.strip_prefix(&f.data).unwrap())
                .exists()
        );
        assert_eq!(fs::read(actor).unwrap(), original);
    }
}

#[test]
fn owner_backup_qualified_input_header_stops_before_foreign_author_body_and_rewinds() {
    let f = Fixture::new();
    let scope = f
        .core
        .capture_file_save_scope(Path::new(&f.foreign_note))
        .unwrap();
    let mut bytes = b"{\"scope\":".to_vec();
    bytes.extend(serde_json::to_vec(&scope).unwrap());
    bytes.extend_from_slice(b",\"source\":BROKEN_FOREIGN_BODY");
    let mut reader = std::io::Cursor::new(bytes);
    let header = file_save_scope::read_input_scope(&mut reader).unwrap();
    assert_eq!(reader.position(), 0);
    let guard = ParentGuard::acquire(&f.core.backend, &f.root.join("prospective")).unwrap();
    let expected = ActorLibraryBinding {
        root: display(&f.root),
        root_identity: guard.directory_identity().unwrap(),
        workspace_id: Some(f.core.require_workspace(&f.root).unwrap().workspace_id),
    };
    assert!(!selected_input_scope(&f.core, &header, &expected).unwrap());
    let mut missing = std::io::Cursor::new(b"{\"path\":\"D:/old/file.typ\",\"source\":BROKEN");
    assert_eq!(
        file_save_scope::read_input_scope(&mut missing)
            .unwrap_err()
            .code,
        "unsupportedInputScope"
    );
    assert_eq!(missing.position(), 0);
}

#[test]
fn original_qualified_scope_rejects_damaged_stable_locator_before_foreign_routing_or_actor_delivery()
 {
    let mut f = Fixture::new();
    let scope = f
        .core
        .capture_file_save_scope(Path::new(&f.foreign_note))
        .unwrap();
    let QualifiedInputScope::Library { mut binding } = scope else {
        panic!("library scope")
    };
    binding.root = display(&f.temp.path().join("contradictory-locator"));
    let bad = QualifiedInputScope::Library { binding };
    let guard = ParentGuard::acquire(&f.core.backend, &f.root.join("prospective")).unwrap();
    let expected = ActorLibraryBinding {
        root: display(&f.root),
        root_identity: guard.directory_identity().unwrap(),
        workspace_id: Some(f.core.require_workspace(&f.root).unwrap().workspace_id),
    };
    assert_eq!(
        selected_input_scope(&f.core, &bad, &expected)
            .unwrap_err()
            .code,
        "privateEvidence"
    );

    let external = fresh();
    let source = f.note.clone();
    let root = f.root.clone();
    f.core
        .with_actor_for_root("tuple-owner", &root, |core| {
            let doc = core.open(&source, Some(root.clone()))?;
            assert_eq!(
                core.actor_save(
                    "Note",
                    HostedSaveRequest {
                        input_revision: None,
                        session_id: doc.session_id,
                        base_id: fresh(),
                        source: "retained original author proposal".into(),
                        operation_id: external.clone()
                    }
                )
                .unwrap_err()
                .code,
                "staleBase"
            );
            Ok(())
        })
        .unwrap();
    let path = f
        .actor_root("tuple-owner")
        .join("operations")
        .join(format!("{external}.json"));
    let original = fs::read(&path).unwrap();
    let mut changed =
        hosted::read_actor_input_header(&mut std::io::Cursor::new(&original)).unwrap();
    let original_scope = serde_json::to_vec(&changed).unwrap();
    let hosted::ActorInputScope::Qualified {
        scope: QualifiedInputScope::Library { binding },
    } = &mut changed
    else {
        panic!("library input scope")
    };
    binding.root = display(&f.temp.path().join("wrong-original-root"));
    let prefix = b"{\"scope\":";
    assert!(original.starts_with(prefix));
    assert_eq!(
        &original[prefix.len()..prefix.len() + original_scope.len()],
        original_scope
    );
    let mut bytes = prefix.to_vec();
    bytes.extend(serde_json::to_vec(&changed).unwrap());
    bytes.extend_from_slice(&original[prefix.len() + original_scope.len()..]);
    fs::write(&path, &bytes).unwrap();
    f.core
        .with_actor_for_root("tuple-owner", &root, |core| {
            assert_eq!(
                core.actor_save_status("Note", &external).unwrap_err().code,
                "privateEvidence"
            );
            assert_eq!(
                core.actor_settle("Note", &external).unwrap_err().code,
                "privateEvidence"
            );
            Ok(())
        })
        .unwrap();
    assert!(
        f.core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&root),
                intent: OperationIntent::BackupLibrary {
                    destination: display(&f.temp.path().join("must-not-capture"))
                }
            })
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(!f.temp.path().join("must-not-capture").exists());
    fs::write(&path, &original).unwrap();
    f.core
        .with_actor_for_root("tuple-owner", &root, |core| {
            assert!(core.actor_save_status("Note", &external)?.is_some());
            Ok(())
        })
        .unwrap();
}

#[test]
fn owner_backup_manifest_and_directories_are_inside_pre_effect_member_budget() {
    let mut f = Fixture::new();
    // snapshot_tree's independent directory limit would permit this, but the
    // final delivery counts original files, manifest and all output parents.
    for index in 0..4990 {
        fs::create_dir(f.root.join(format!("empty-{index}"))).unwrap();
    }
    let operation = fresh();
    let result = f.core.prepare_operation(OperationRequest {
        operation_id: operation.clone(),
        root: display(&f.root),
        intent: OperationIntent::BackupLibrary {
            destination: display(&f.temp.path().join("over-budget")),
        },
    });
    assert_eq!(result.unwrap_err().code, "budget");
    assert!(!f.temp.path().join("over-budget").exists());
    assert!(!f.core.journal_path(&operation, "plan").exists());
}

#[test]
fn owner_backup_data_directory_ancestor_names_do_not_change_record_roles() {
    let mut f = Fixture::with_private_path("journal/drafts/outputs/private");
    let (op, claim) = f.actor_facts("alice", false, "selected original");
    let p = f.prepare("backup");
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let actors = f
        .temp
        .path()
        .join("backup/private/actors")
        .join(hash(b"alice"));
    assert!(
        actors
            .join("operations")
            .join(format!("{op}.json"))
            .exists()
    );
    assert!(
        actors
            .join("operations/outputs")
            .join(format!("{claim}.json"))
            .exists()
    );
}

#[test]
fn owner_backup_case_alias_private_root_is_never_silently_omitted() {
    let mut f = Fixture::new();
    f.actor_facts("alice", false, "preserve");
    fs::rename(f.data.join("actors"), f.data.join("Actors")).unwrap();
    let error = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&f.temp.path().join("backup")),
            },
        })
        .unwrap_err();
    assert_eq!(error.code, "privateEvidence");
    assert!(f.data.join("Actors").is_dir());
    assert!(!f.temp.path().join("backup").exists());
}

#[test]
fn owner_backup_excludes_device_fence_but_retains_its_exact_routing_dependency() {
    let mut f = Fixture::new();
    let id = fresh();
    f.core
        .cancel_navigation_attempt(&f.root, &id, None)
        .unwrap();
    let p = f.prepare("with-fence");
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    assert!(
        !f.temp
            .path()
            .join("with-fence/private/journal")
            .join(&id[..2])
            .join(format!("{id}.resolution.json"))
            .exists()
    );
    assert!(f.core.journal_path(&id, "resolution").exists());
    let p = f.prepare("fence-changed");
    let mut decision: serde_json::Value =
        serde_json::from_slice(&fs::read(f.core.journal_path(&id, "resolution")).unwrap()).unwrap();
    decision["message"] = serde_json::json!("changed original device fact");
    fs::write(
        f.core.journal_path(&id, "resolution"),
        serde_json::to_vec(&decision).unwrap(),
    )
    .unwrap();
    assert_eq!(
        f.core
            .execute_operation(&p.operation_id, &p.plan_id)
            .unwrap_err()
            .code,
        "scopeChanged"
    );
    assert!(!f.temp.path().join("fence-changed").exists());
}
#[test]
fn owner_backup_never_ignores_orphan_or_malformed_selected_resolution() {
    let mut f = Fixture::new();
    let id = fresh();
    fs::create_dir_all(f.core.journal_path(&id, "commit").parent().unwrap()).unwrap();
    fs::write(
        f.core.journal_path(&id, "commit"),
        b"unknown retained commit",
    )
    .unwrap();
    let error = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&f.temp.path().join("blocked")),
            },
        })
        .unwrap_err();
    assert_eq!(error.code, "resolutionUnavailable");
    assert!(!f.temp.path().join("blocked").exists());
    let mut f = Fixture::new();
    let id = f
        .core
        .operations
        .values()
        .find(|plan| plan.kind == "createLibrary" && Path::new(&plan.path) == f.root)
        .unwrap()
        .operation_id
        .clone();
    fs::write(f.core.journal_path(&id, "resolution"), b"not a decision").unwrap();
    let error = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&f.temp.path().join("blocked")),
            },
        })
        .unwrap_err();
    assert_eq!(error.code, "resolutionUnavailable");
}

#[test]
fn owner_backup_registered_plan_unknown_leaf_is_never_a_formal_backup_member() {
    let mut f = Fixture::new();
    let id = f
        .core
        .operations
        .values()
        .find(|p| p.kind == "createLibrary" && Path::new(&p.path) == f.root)
        .unwrap()
        .operation_id
        .clone();
    fs::write(
        f.core.journal_path(&id, "unknown"),
        b"retained unknown evidence",
    )
    .unwrap();
    let error = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&f.temp.path().join("blocked")),
            },
        })
        .unwrap_err();
    assert_eq!(error.code, "privateEvidence");
    assert!(!f.temp.path().join("blocked").exists());
}
#[test]
fn owner_backup_routes_typed_preplan_reservation_and_its_decision_to_only_its_original_library() {
    let mut f = Fixture::new();
    let doc = f.core.open(&f.note, Some(f.root.clone())).unwrap();
    let id = fresh();
    let snapshot = f
        .core
        .read_snapshot(&f.root, std::slice::from_ref(&doc.path))
        .unwrap()
        .files
        .remove(0);
    f.core
        .reserve_definition_key(&f.root, &doc.path, &snapshot.base_id, &id)
        .unwrap();
    f.core
        .settle_local_attempt(&id, "save", &hash(b"rejected definition"))
        .unwrap();
    let p = f.prepare("scoped-cancelled-reservation");
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let journal = f
        .temp
        .path()
        .join("scoped-cancelled-reservation/private/journal");
    assert!(
        journal
            .join(&id[..2])
            .join(format!("{id}.definition-reservation.json"))
            .exists()
    );
    assert!(
        journal
            .join(&id[..2])
            .join(format!("{id}.resolution.json"))
            .exists()
    );
    let p = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.foreign),
            intent: OperationIntent::BackupLibrary {
                destination: display(&f.temp.path().join("other-library")),
            },
        })
        .unwrap();
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    assert!(
        !f.temp
            .path()
            .join("other-library/private/journal")
            .join(&id[..2])
            .join(format!("{id}.definition-reservation.json"))
            .exists()
    );
}

#[test]
fn own_new_journal_bucket_does_not_change_the_frozen_backup_input() {
    let mut f = Fixture::new();
    let journal = f.data.join("journal");
    let bucket = (0..256)
        .map(|n| format!("{n:02x}"))
        .find(|b| !journal.join(b).exists())
        .unwrap();
    let id = format!("{bucket}{}", &fresh()[2..]);
    let destination = f.temp.path().join("new-bucket-backup");
    let prepared = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: id.clone(),
            root: display(&f.root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&destination),
            },
        })
        .unwrap();
    assert!(journal.join(bucket).is_dir());
    f.core.execute_operation(&id, &prepared.plan_id).unwrap();
    assert!(destination.join("library/A/A.typ").is_file());
}

#[test]
fn complete_backup_reserves_materializations_without_charging_existing_inputs_twice() {
    let mut f = Fixture::new();
    for n in 0..1300 {
        fs::write(f.root.join(format!("owned-raw-{n}.bin")), b"retained raw").unwrap();
    }
    let prepared = f.prepare("many-existing-inputs");
    f.core
        .execute_operation(&prepared.operation_id, &prepared.plan_id)
        .unwrap();
    let target = f.temp.path().join("many-existing-inputs/library");
    for n in [0, 1299] {
        assert_eq!(
            fs::read(target.join(format!("owned-raw-{n}.bin"))).unwrap(),
            b"retained raw"
        );
    }
}
