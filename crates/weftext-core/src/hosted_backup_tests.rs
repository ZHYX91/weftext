use super::*;

const ACTOR: &str = "deployment:alice:owner-backup";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct Fixture {
    temp: tempfile::TempDir,
    core: Core,
    data: PathBuf,
    root: PathBuf,
    target: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Library");
        let data = temp.path().join("private");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let target = temp.path().join("backup");
        Self {
            temp,
            core,
            data,
            root,
            target,
        }
    }
    fn prepare(&mut self, operation: &str) -> Result<ActorOwnerBackupStatus> {
        self.core.with_actor_for_root(ACTOR, &self.root, |core| {
            core.actor_owner_backup_prepare(
                "library",
                operation,
                FP,
                &self.target,
                serde_json::json!({"kind":"ownerBackup"}),
            )
        })
    }
    fn restart(&mut self) {
        let replacement = Core::new(self.temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, replacement));
        self.core = Core::new(&self.data).unwrap();
    }
    fn internal(&self, operation: &str) -> String {
        let path = self
            .data
            .join("actors")
            .join(hash(ACTOR.as_bytes()))
            .join("operations")
            .join(format!("{operation}.json"));
        serde_json::from_slice::<ActorOperation>(&fs::read(path).unwrap())
            .unwrap()
            .core_operation_id
    }
}

#[test]
fn owner_backup_original_fence_preflight_restart_commit_and_fixed_artifact_preserve_actors_and_empty_dirs()
 {
    let mut f = Fixture::new();
    fs::create_dir(f.root.join("empty-目录")).unwrap();
    let note = f
        .core
        .create_note_fixture(&f.root, &f.root, "Note")
        .unwrap();
    for (actor, text) in [
        ("deployment:alice", "ALICE_PRIVATE"),
        ("deployment:bob", "BOB_PRIVATE"),
    ] {
        f.core
            .with_actor_for_root(actor, &f.root, |core| {
                let doc = core.open(&note.path, Some(f.root.clone()))?;
                core.persist_draft(&doc.session_id, text)?;
                core.actor_output_claim(&fresh(), FP)?;
                core.actor_settle("Note", &fresh())?;
                Ok(())
            })
            .unwrap();
    }
    let operation = fresh();
    let prepared = f.prepare(&operation).unwrap();
    let plan_id = prepared.plan_id.unwrap();
    let internal = f.internal(&operation);
    assert_ne!(internal, operation);
    let original_record = fs::read(
        f.data
            .join("actors")
            .join(hash(ACTOR.as_bytes()))
            .join("operations")
            .join(format!("{operation}.json")),
    )
    .unwrap();
    f.core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            let artifact =
                core.actor_owner_backup_preflight_artifact("library", &operation, &plan_id)?;
            assert!(
                artifact
                    .directories
                    .iter()
                    .any(|name| name == "library/empty-目录")
            );
            for text in [b"ALICE_PRIVATE".as_slice(), b"BOB_PRIVATE".as_slice()] {
                assert!(
                    artifact
                        .files
                        .iter()
                        .any(|(_, bytes)| bytes.windows(text.len()).any(|part| part == text))
                );
            }
            assert_eq!(
                artifact
                    .files
                    .iter()
                    .filter(|(name, _)| name == "backup.json")
                    .count(),
                1
            );
            assert!(
                core.actor_owner_backup_artifact("library", &operation)
                    .is_err()
            );
            Ok(())
        })
        .unwrap();
    f.restart();
    let committed = f
        .core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            core.actor_owner_backup_commit("library", &operation, &plan_id)
        })
        .unwrap();
    assert_eq!(committed.status, "committed");
    assert_eq!(
        fs::read(
            f.data
                .join("actors")
                .join(hash(ACTOR.as_bytes()))
                .join("operations")
                .join(format!("{operation}.json"))
        )
        .unwrap(),
        original_record
    );
    fs::write(f.target.join("backup.json"), b"EXTERNAL_SPOOL_REPLACEMENT").unwrap();
    f.core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            let artifact = core.actor_owner_backup_artifact("library", &operation)?;
            assert!(
                artifact
                    .files
                    .iter()
                    .find(|(name, _)| name == "backup.json")
                    .unwrap()
                    .1
                    .starts_with(b"{")
            );
            assert_eq!(
                core.actor_owner_backup_settle("library", &operation)?
                    .status,
                "committed"
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn owner_backup_preplan_loss_restart_settle_and_category_fence_do_not_regenerate() {
    let mut f = Fixture::new();
    let operation = fresh();
    set_test_failpoint("ownerBackupActorDurable");
    assert!(f.prepare(&operation).is_err());
    assert!(!f.target.exists());
    f.restart();
    f.core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            assert_eq!(
                core.actor_owner_backup_retry("library", &operation, FP)?
                    .unwrap()
                    .status,
                "notCommitted"
            );
            assert_eq!(
                core.actor_owner_backup_settle("library", &operation)?
                    .status,
                "cancelled"
            );
            assert_eq!(
                core.actor_owner_backup_prepare(
                    "library",
                    &operation,
                    FP,
                    &f.target,
                    serde_json::json!({"different":true})
                )?
                .status,
                "cancelled"
            );
            assert!(core.actor_feature_status("library", &operation).is_err());
            assert!(core.actor_save_status("library", &operation).is_err());
            Ok(())
        })
        .unwrap();
    assert!(!f.target.exists());
    assert!(
        f.core
            .with_actor_for_root("deployment:bob:owner-backup", &f.root, |core| core
                .actor_owner_backup_status("library", &operation))
            .unwrap()
            .is_none()
    );
}

#[test]
fn owner_backup_installing_status_survives_restart_and_settle_cannot_cancel() {
    let mut f = Fixture::new();
    let operation = fresh();
    let prepared = f.prepare(&operation).unwrap();
    let internal = f.internal(&operation);
    let plan = f.core.operations.get(&internal).unwrap().clone();
    // Persist the original decision's real installing marker without claiming
    // that any afterimage was installed; recovery must remain authoritative.
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &f.core.journal_path(&internal, "install"),
        &serde_json::to_vec(
            &serde_json::json!({"operationId":plan.operation_id,"fingerprint":plan.fingerprint}),
        )
        .unwrap(),
    )
    .unwrap();
    f.restart();
    f.core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            assert_eq!(
                core.actor_owner_backup_settle("library", &operation)?
                    .status,
                "recoveryPending"
            );
            assert!(
                core.actor_owner_backup_preflight_artifact(
                    "library",
                    &operation,
                    prepared.plan_id.as_ref().unwrap()
                )
                .is_err()
            );
            assert!(
                core.actor_owner_backup_artifact("library", &operation)
                    .is_err()
            );
            Ok(())
        })
        .unwrap();
    assert!(!f.target.exists());
}

#[test]
fn owner_backup_artifact_counts_unique_actual_directories_in_deep_plan() {
    let mut f = Fixture::new();
    let deep = f.root.join("a/b/c/d/e/f/g/h");
    fs::create_dir_all(&deep).unwrap();
    for index in 0..12 {
        fs::write(deep.join(format!("{index}.txt")), b"x").unwrap();
    }
    let operation = fresh();
    let prepared = f.prepare(&operation).unwrap();
    let internal = f.internal(&operation);
    let transaction = f
        .core
        .operations
        .get_mut(&internal)
        .unwrap()
        .transaction
        .as_mut()
        .unwrap();
    if let Mutation::CreateTree { directories, .. } = &mut transaction.mutation {
        // Pure accessor compatibility with an older frozen directory list:
        // equivalent repeated ancestors never add new filesystem members.
        let originals = directories.clone();
        while directories.len() <= 5000 {
            directories.extend(originals.clone());
        }
    }
    let Mutation::CreateTree { directories, .. } = &f
        .core
        .operations
        .get(&internal)
        .unwrap()
        .transaction
        .as_ref()
        .unwrap()
        .mutation
    else {
        panic!("not backup");
    };
    assert!(directories.len() > 5000);
    f.core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            let artifact = core.actor_owner_backup_preflight_artifact(
                "library",
                &operation,
                prepared.plan_id.as_ref().unwrap(),
            )?;
            assert!(artifact.files.len() + artifact.directories.len() < 1000);
            Ok(())
        })
        .unwrap();
}
