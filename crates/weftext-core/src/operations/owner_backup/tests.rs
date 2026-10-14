use super::*;

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
        let note = core.create_note(&root, &root, "A").unwrap().path;
        let foreign_note = core.create_note(&foreign, &foreign, "B").unwrap().path;
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
                .join(format!("{pending}.plan.json"))
        )
        .unwrap(),
        pending_original
    );
    assert!(f.core.inspect_recovery(&pending).unwrap().status != "committed");
    let backed = fs::read_to_string(target.join("library/A/A.typ")).unwrap();
    assert!(backed.contains("PENDING_SELECTED"));
    assert!(!f.core.committed(&pending));
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
    let prefix = format!(
        "{{\"library\":{},\"payload\":",
        serde_json::to_string(&binding).unwrap()
    );
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
fn owner_backup_selected_legacy_plan_join_and_draft_are_allowed_but_unscoped_fence_is_not() {
    let mut f = Fixture::new();
    // The ordinary local API has an original path-bound legacy Draft.
    let doc = f.core.open(&f.note, Some(f.root.clone())).unwrap();
    f.core
        .persist_draft(&doc.session_id, "legacy selected proposal")
        .unwrap();
    let draftpath = f.core.draft_path(Path::new(&f.note));
    let draftbytes = fs::read(&draftpath).unwrap();
    let firstpath = draftbytes
        .windows(b",\"path\"".len())
        .position(|w| w == b",\"path\"")
        .unwrap();
    let mut olddraft = vec![b'{'];
    olddraft.extend_from_slice(&draftbytes[firstpath + 1..]);
    fs::write(&draftpath, olddraft).unwrap();
    let (op, _) = f.actor_facts("alice", false, "joined");
    let actorfile = f
        .actor_root("alice")
        .join("operations")
        .join(format!("{op}.json"));
    let bytes = fs::read(&actorfile).unwrap();
    let key = bytes
        .windows(b",\"actor\"".len())
        .position(|w| w == b",\"actor\"")
        .unwrap();
    let mut legacy = vec![b'{'];
    legacy.extend_from_slice(&bytes[key + 1..]);
    fs::write(&actorfile, legacy).unwrap();
    let p = f.prepare("backup");
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    assert!(
        f.temp
            .path()
            .join("backup/private/actors")
            .join(hash(b"alice"))
            .join("operations")
            .join(format!("{op}.json"))
            .exists()
    );
    let unknown = f
        .actor_root("alice")
        .join("operations")
        .join(format!("{}.json", fresh()));
    fs::write(&unknown,format!("{{\"actor\":\"alice\",\"resource\":\"note\",\"fingerprint\":\"{}\",\"coreOperationId\":\"{}\",\"cancelled\":false,\"payload\":\"private\"}}",hash(b"unknown"),fresh())).unwrap();
    let error = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&f.temp.path().join("incomplete")),
            },
        })
        .unwrap_err();
    assert_eq!(error.code, "privateEvidence");
    assert!(unknown.exists());
    assert!(!f.temp.path().join("incomplete").exists());
}

#[test]
fn owner_backup_legacy_scope_checks_stop_before_foreign_source_bytes() {
    let root = Path::new(r"D:\selected");
    let foreign_prefix = br#"{"path":"D:\\foreign\\N.typ""#;
    let mut reader = std::io::Cursor::new(foreign_prefix);
    assert_eq!(
        legacy_draft_route(&mut reader, root).unwrap_err().code,
        "privateEvidence"
    );
    assert_eq!(reader.position() as usize, foreign_prefix.len());
    let alternative_prefix=br#"{"path":"D:\\selected\\N.typ","baseSource":"owned","source":"owned","alternatives":[{"path":"D:\\foreign\\N.typ""#;
    let mut reader = std::io::Cursor::new(alternative_prefix);
    assert_eq!(
        legacy_draft_route(&mut reader, root).unwrap_err().code,
        "privateEvidence"
    );
    assert_eq!(reader.position() as usize, alternative_prefix.len());
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
