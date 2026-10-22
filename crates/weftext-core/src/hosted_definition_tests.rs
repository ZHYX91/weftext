use super::*;
use weftext_data::{QueryPlan, Schema};

const ACTOR: &str = "hosted:alice:definitions";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_FP: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn definition_enumeration_and_admission_status_skip_other_resources_before_private_body() {
    let mut f = Fixture::new();
    let doc = f.open();
    let own = fresh();
    f.reserve(&doc, &doc.source, "weftext-query-ownprivate", &own)
        .unwrap();
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
    let other = f
        .core
        .with_actor(ACTOR, |core| core.open(&other.path, Some(f.root.clone())))
        .unwrap();
    let other_id = fresh();
    let request = f.request(&other, &other_id);
    let proposal = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_definition_reserve("Other", &f.root, request, |key, binding| {
                weftext_data::upsert_saved_query(
                    &other.source,
                    None,
                    key,
                    "weftext-query-otherprivate",
                    binding,
                    &QueryPlan::default(),
                    &f.schema,
                )
                .map_err(|error| CoreError::new("invalidQueryDefinition", error.to_string()))
            })
        })
        .unwrap();
    let path = f
        .core
        .with_actor(ACTOR, |core| core.actor_record(&other_id))
        .unwrap();
    let original = fs::read(&path).unwrap();
    let marker = b",\"cancelled\":";
    let at = original
        .windows(marker.len())
        .position(|part| part == marker)
        .unwrap();
    let mut damaged = original[..at].to_vec();
    damaged.extend_from_slice(b",\"cancelled\":BROKEN_OTHER_PRIVATE_BODY");
    fs::write(&path, &damaged).unwrap();
    let count = f.core.operations.len();
    let listed = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_definition_list("Note", &f.root, &doc.session_id)
        })
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].operation_id, own);
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len((MAX_LIST_BYTES + 1) as u64).unwrap();
    drop(file);
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_definition_list(
                "Note",
                &f.root,
                &doc.session_id
            ))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        (MAX_LIST_BYTES + 1) as u64
    );
    fs::write(&path, &original).unwrap();
    let source = proposal.patch.unwrap().source;
    let saved_id = fresh();
    f.core
        .with_actor(ACTOR, |core| {
            core.actor_save_with_definition_reservations(
                "Other",
                &f.root,
                HostedSaveRequest {
                    session_id: other.session_id.clone(),
                    base_id: other.base_id.clone(),
                    source: source.clone(),
                    operation_id: saved_id.clone(),
                },
                std::slice::from_ref(&other_id),
            )
        })
        .unwrap();
    let saved = f
        .core
        .with_actor(ACTOR, |core| {
            core.read_actor_operation(&saved_id, "Other", MAX_RECORD_BYTES)
        })
        .unwrap()
        .unwrap();
    let sidecar = f
        .core
        .journal_path(&saved.core_operation_id, "definition-reservations");
    let original_sidecar = fs::read(&sidecar).unwrap();
    let marker = b",\"reservations\":";
    let at = original_sidecar
        .windows(marker.len())
        .position(|part| part == marker)
        .unwrap();
    let mut damaged_sidecar = original_sidecar[..at].to_vec();
    damaged_sidecar.extend_from_slice(b",\"reservations\":BROKEN_OTHER_RESERVATIONS");
    fs::write(&sidecar, &damaged_sidecar).unwrap();
    assert_eq!(f.status(&own).status, "reserved");
    assert_eq!(fs::read(&sidecar).unwrap(), damaged_sidecar);
    assert!(f.core.operations.len() > count);
    fs::write(&sidecar, &original_sidecar).unwrap();
    assert_eq!(f.status(&own).status, "reserved");
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core
                .actor_definition_status("Other", &other_id))
            .unwrap()
            .unwrap()
            .status,
        "admitted"
    );
    assert_eq!(fs::read(sidecar).unwrap(), original_sidecar);
    assert_eq!(fs::read(path).unwrap(), original);
}

struct Fixture {
    core: Core,
    temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    path: PathBuf,
    binding: SavedQueryBinding,
    schema: Schema,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let bytes = include_bytes!("../../../docs/design/examples/schema.authoring.json").to_vec();
        let schema = Schema::from_author_json(std::str::from_utf8(&bytes).unwrap()).unwrap();
        let operation = fresh();
        let plan = core
            .prepare_config_save(&root, ConfigRole::Schema, None, bytes, &operation)
            .unwrap();
        core.execute_operation(&operation, &plan.plan_id).unwrap();
        let config = core.read_config(&root, ConfigRole::Schema).unwrap();
        let binding = SavedQueryBinding {
            schema: config.role_ref,
            revision: config.revision.unwrap(),
            language: "weftext.query.cel.v1".into(),
        };
        Self {
            core,
            temp,
            data,
            root,
            path: note.path.into(),
            binding,
            schema,
        }
    }
    fn open(&mut self) -> OpenedDocument {
        self.core
            .with_actor(ACTOR, |c| c.open(&self.path, Some(self.root.clone())))
            .unwrap()
    }
    fn restart(&mut self) {
        let placeholder = Core::new(self.temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, placeholder));
        self.core = Core::new(&self.data).unwrap();
    }
    fn request(&self, doc: &OpenedDocument, operation: &str) -> HostedDefinitionReservationRequest {
        HostedDefinitionReservationRequest {
            session_id: doc.session_id.clone(),
            base_id: doc.base_id.clone(),
            operation_id: operation.into(),
            intent_fingerprint: FP.into(),
            binding: self.binding.clone(),
        }
    }
    fn reserve(
        &mut self,
        doc: &OpenedDocument,
        original: &str,
        label: &str,
        operation: &str,
    ) -> Result<ActorDefinitionProposal> {
        let request = self.request(doc, operation);
        self.core.with_actor(ACTOR, |core| {
            core.actor_definition_reserve("Note", &self.root, request, |key, binding| {
                weftext_data::upsert_saved_query(
                    original,
                    None,
                    key,
                    label,
                    binding,
                    &QueryPlan::default(),
                    &self.schema,
                )
                .map_err(|e| CoreError::from_data("invalidQueryDefinition", e))
            })
        })
    }
    fn save(
        &mut self,
        doc: &OpenedDocument,
        source: &str,
        ids: &[String],
        operation: &str,
    ) -> Result<ActorSaveStatus> {
        self.core.with_actor(ACTOR, |c| {
            c.actor_save_with_definition_reservations(
                "Note",
                &self.root,
                HostedSaveRequest {
                    session_id: doc.session_id.clone(),
                    base_id: doc.base_id.clone(),
                    source: source.into(),
                    operation_id: operation.into(),
                },
                ids,
            )
        })
    }
    fn status(&mut self, operation: &str) -> ActorDefinitionStatus {
        self.core
            .with_actor(ACTOR, |c| c.actor_definition_status("Note", operation))
            .unwrap()
            .unwrap()
    }
    fn admitted(&self, doc: &OpenedDocument, key: &str) -> bool {
        self.core
            .is_definition_key_admitted(&self.root, doc.node_id.as_ref().unwrap(), key)
            .unwrap()
    }
}

#[test]
fn hosted_definitions_exact_proposal_retry_restart_and_multiple_source_admission() {
    let mut f = Fixture::new();
    let doc = f.open();
    let first_id = fresh();
    let first = f
        .reserve(&doc, &doc.source, "weftext-query-first", &first_id)
        .unwrap();
    let first_key = first.status.key.clone().unwrap();
    let first_source = first.patch.as_ref().unwrap().source.clone();
    assert_eq!(first.status.status, "reserved");
    assert_eq!(fs::read_to_string(&f.path).unwrap(), doc.source);
    assert!(!f.admitted(&doc, &first_key));
    let second_id = fresh();
    let second = f
        .reserve(&doc, &first_source, "weftext-query-second", &second_id)
        .unwrap();
    let second_key = second.status.key.clone().unwrap();
    let source = second.patch.unwrap().source + "\n作者的其他未保存正文 😀\n";
    f.core
        .with_actor(ACTOR, |c| c.persist_draft(&doc.session_id, &source))
        .unwrap();
    f.restart();
    let doc = f.open();
    assert_eq!(doc.recovered_draft.as_deref(), Some(source.as_str()));
    let retried = f
        .core
        .with_actor(ACTOR, |c| c.actor_definition_retry("Note", &first_id, FP))
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(retried.patch).unwrap(),
        serde_json::to_value(first.patch).unwrap()
    );
    assert_eq!(retried.status.key.as_deref(), Some(first_key.as_str()));
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_definition_list(
                "Note",
                &f.root,
                &doc.session_id
            ))
            .unwrap()
            .len(),
        2
    );
    let save_id = fresh();
    let status = f
        .save(
            &doc,
            &source,
            &[second_id.clone(), first_id.clone(), first_id.clone()],
            &save_id,
        )
        .unwrap();
    assert_eq!(status.status, "committed");
    let raw: ActorOperation = f
        .core
        .with_actor(ACTOR, |c| {
            Ok(serde_json::from_slice(&fs::read(c.actor_record(&save_id)?).unwrap()).unwrap())
        })
        .unwrap();
    assert_ne!(raw.core_operation_id, save_id);
    assert_eq!(raw.save_definition_reservations.len(), 2);
    assert!(f.admitted(&doc, &first_key));
    assert!(f.admitted(&doc, &second_key));
    assert_eq!(
        f.status(&first_id).save_operation_id.as_deref(),
        Some(save_id.as_str())
    );
    f.restart();
    assert_eq!(f.status(&first_id).status, "admitted");
    assert_eq!(
        f.save(&doc, &source, &[first_id, second_id], &save_id)
            .unwrap()
            .status,
        "committed"
    );
    assert!(
        fs::read_to_string(&f.path)
            .unwrap()
            .contains("作者的其他未保存正文")
    );
}

#[test]
fn hosted_definition_undo_save_redo_uses_original_key_and_new_current_basis() {
    let mut f = Fixture::new();
    let mut doc = f.open();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-original", &id)
        .unwrap();
    let key = proposal.status.key.unwrap();
    let undone = doc.source.clone() + "\n正文在Undo定义后仍继续编辑\n";
    let saved = f
        .save(&doc, &undone, std::slice::from_ref(&id), &fresh())
        .unwrap();
    assert_eq!(f.status(&id).status, "reserved");
    assert!(!f.admitted(&doc, &key));
    doc.base_id = saved.base_id.unwrap();
    let redone = proposal.patch.unwrap().source + "\n正文在Undo定义后仍继续编辑\n";
    let edited = weftext_data::upsert_saved_query(
        &redone,
        Some(&key),
        &key,
        "weftext-query-renamed",
        &f.binding,
        &QueryPlan {
            take: Some(2),
            ..QueryPlan::default()
        },
        &f.schema,
    )
    .unwrap();
    assert_eq!(
        f.save(&doc, &edited.source, std::slice::from_ref(&id), &fresh())
            .unwrap()
            .status,
        "committed"
    );
    assert!(f.admitted(&doc, &key));
    let actual = fs::read_to_string(&f.path).unwrap();
    assert!(actual.contains("weftext-query-renamed"));
    assert!(actual.contains("正文在Undo定义后仍继续编辑"));
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_definition_settle("Note", &id))
            .unwrap()
            .status,
        "admitted"
    );
}

#[test]
fn hosted_definition_request_fingerprint_owner_actor_category_and_base_are_fenced() {
    let mut f = Fixture::new();
    let doc = f.open();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-owned", &id)
        .unwrap();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_definition_retry("Note", &id, OTHER_FP))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert!(
        f.core
            .with_actor("hosted:bob", |c| c.actor_definition_retry("Note", &id, FP))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_definition_status("Other", &id))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_save_status("Note", &id))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        f.save(&doc, &proposal.patch.unwrap().source, &[], &id)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
    let other = f
        .core
        .with_actor(ACTOR, |c| c.open(&other.path, Some(f.root.clone())))
        .unwrap();
    assert_eq!(
        f.save(&other, &other.source, std::slice::from_ref(&id), &fresh())
            .unwrap_err()
            .code,
        "operationConflict"
    );
    let mut stale = f.request(&doc, &fresh());
    stale.base_id = fresh();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_definition_reserve(
                "Note",
                &f.root,
                stale,
                |_, _| panic!("stale callback")
            ))
            .unwrap_err()
            .code,
        "staleBase"
    );
    fs::write(&f.path, doc.source.clone() + "\nexternal").unwrap();
    let request = f.request(&doc, &fresh());
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_definition_reserve(
                "Note",
                &f.root,
                request,
                |_, _| panic!("changed callback")
            ))
            .unwrap_err()
            .code,
        "staleBase"
    );
}

#[test]
fn hosted_definition_false_key_and_raw_occurrences_never_admit() {
    let mut f = Fixture::new();
    let mut doc = f.open();
    let false_key = fresh();
    let fabricated = weftext_data::upsert_saved_query(
        &doc.source,
        None,
        &false_key,
        "weftext-query-false",
        &f.binding,
        &QueryPlan::default(),
        &f.schema,
    )
    .unwrap();
    let saved = f.save(&doc, &fabricated.source, &[], &fresh()).unwrap();
    assert!(!f.admitted(&doc, &false_key));
    doc.base_id = saved.base_id.unwrap();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &fabricated.source, "weftext-query-real", &id)
        .unwrap();
    let key = proposal.status.key.unwrap();
    let raw = fabricated.source.clone() + &format!("\n只是普通文字 {key}\n");
    let saved = f
        .save(&doc, &raw, std::slice::from_ref(&id), &fresh())
        .unwrap();
    assert!(!f.admitted(&doc, &key));
    doc.base_id = saved.base_id.unwrap();
    let malformed = proposal
        .patch
        .unwrap()
        .source
        .replace("weftext.query", "weftext.query.unknown");
    f.save(&doc, &malformed, std::slice::from_ref(&id), &fresh())
        .unwrap();
    assert!(!f.admitted(&doc, &key));
    assert_eq!(f.status(&id).status, "reserved");
}

#[test]
fn hosted_definition_cancel_unknown_and_known_cannot_revive_but_undo_still_saves() {
    let mut f = Fixture::new();
    let doc = f.open();
    let unknown = fresh();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_definition_settle("Note", &unknown))
            .unwrap()
            .status,
        "cancelled"
    );
    let request = f.request(&doc, &unknown);
    let result = f
        .core
        .with_actor(ACTOR, |c| {
            c.actor_definition_reserve("Note", &f.root, request, |_, _| {
                panic!("cancelled callback")
            })
        })
        .unwrap();
    assert_eq!(result.status.status, "cancelled");
    assert!(result.patch.is_none());
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-cancelled", &id)
        .unwrap();
    f.core
        .with_actor(ACTOR, |c| c.actor_definition_settle("Note", &id))
        .unwrap();
    f.restart();
    assert_eq!(f.status(&id).status, "cancelled");
    let doc = f.open();
    assert_eq!(
        f.save(
            &doc,
            &proposal.patch.unwrap().source,
            std::slice::from_ref(&id),
            &fresh()
        )
        .unwrap_err()
        .code,
        "operationCancelled"
    );
    assert_eq!(
        f.save(
            &doc,
            &(doc.source.clone() + "\nUndo保留正文"),
            &[id, unknown],
            &fresh()
        )
        .unwrap()
        .status,
        "committed"
    );
}

#[test]
fn hosted_definition_lost_reservation_response_keeps_exact_original_patch() {
    let mut f = Fixture::new();
    let doc = f.open();
    let id = fresh();
    FAILPOINT.with(|point| point.set(Some("reservationDurable")));
    assert_eq!(
        f.reserve(&doc, &doc.source, "weftext-query-lost", &id)
            .unwrap_err()
            .code,
        "injectedFailure"
    );
    let original = f
        .core
        .with_actor(ACTOR, |c| c.actor_definition_details("Note", &id))
        .unwrap()
        .unwrap();
    f.restart();
    let retry = f
        .core
        .with_actor(ACTOR, |c| c.actor_definition_retry("Note", &id, FP))
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(original).unwrap(),
        serde_json::to_value(retry).unwrap()
    );
}

#[test]
fn hosted_definition_preplan_failures_settle_original_and_explicitly_save_same_key_after_restart() {
    for point in ["actorSaveDurable", "admissionsDurable"] {
        let mut f = Fixture::new();
        let doc = f.open();
        let id = fresh();
        let proposal = f
            .reserve(&doc, &doc.source, "weftext-query-gap", &id)
            .unwrap();
        let key = proposal.status.key.unwrap();
        let source = proposal.patch.unwrap().source;
        let save_id = fresh();
        FAILPOINT.with(|active| active.set(Some(point)));
        assert_eq!(
            f.save(&doc, &source, std::slice::from_ref(&id), &save_id)
                .unwrap_err()
                .code,
            "injectedFailure"
        );
        f.restart();
        assert_eq!(f.status(&id).status, "reserved");
        assert_eq!(
            f.core
                .with_actor(ACTOR, |c| c.actor_save_status("Note", &save_id))
                .unwrap()
                .unwrap()
                .status,
            "notCommitted"
        );
        assert_eq!(
            f.core
                .with_actor(ACTOR, |c| c.actor_settle("Note", &save_id))
                .unwrap()
                .status,
            "cancelled"
        );
        let current = f.open();
        assert_eq!(
            f.save(&current, &source, std::slice::from_ref(&id), &save_id)
                .unwrap_err()
                .code,
            "operationCancelled"
        );
        f.save(&current, &source, std::slice::from_ref(&id), &fresh())
            .unwrap();
        assert!(f.admitted(&current, &key));
    }
}

#[test]
fn hosted_definition_installed_pending_is_not_cancelled_and_committed_lost_response_is_idempotent()
{
    for point in ["saveStaged", "sourceWritten", "saveCommitted"] {
        let mut f = Fixture::new();
        let doc = f.open();
        let id = fresh();
        let proposal = f
            .reserve(&doc, &doc.source, "weftext-query-installed", &id)
            .unwrap();
        let key = proposal.status.key.unwrap();
        let source = proposal.patch.unwrap().source;
        let save_id = fresh();
        FAILPOINT.with(|active| active.set(Some(point)));
        assert_eq!(
            f.save(&doc, &source, std::slice::from_ref(&id), &save_id)
                .unwrap_err()
                .code,
            "injectedFailure"
        );
        f.restart();
        let expected = if point == "saveCommitted" {
            "admitted"
        } else {
            "recoveryPending"
        };
        assert_eq!(f.status(&id).status, expected);
        assert_eq!(
            f.core
                .with_actor(ACTOR, |c| c.actor_definition_settle("Note", &id))
                .unwrap()
                .status,
            expected
        );
        assert_eq!(f.admitted(&doc, &key), point == "saveCommitted");
        let retry = f
            .save(&doc, &source, std::slice::from_ref(&id), &save_id)
            .unwrap();
        assert_eq!(
            retry.status,
            if point == "saveCommitted" {
                "committed"
            } else {
                "recoveryPending"
            }
        );
    }
}

#[test]
fn hosted_definition_save_fingerprint_freezes_the_explicit_id_set() {
    let mut f = Fixture::new();
    let doc = f.open();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-set", &id)
        .unwrap();
    let source = proposal.patch.unwrap().source;
    let save_id = fresh();
    f.save(&doc, &source, std::slice::from_ref(&id), &save_id)
        .unwrap();
    assert_eq!(
        f.save(&doc, &source, &[], &save_id).unwrap_err().code,
        "operationConflict"
    );
    assert_eq!(
        f.save(&doc, &source, &vec![id; 33], &fresh())
            .unwrap_err()
            .code,
        "budget"
    );
}

#[test]
fn hosted_definition_guard_holds_schema_and_callee_but_excludes_owner_and_deduplicates() {
    let mut f = Fixture::new();
    let doc = f.open();
    let callee = f
        .core
        .create_note_fixture(&f.root, &f.root, "Callee")
        .unwrap();
    let schema_path = f.root.join(".weftext-schema.json");
    let paths = vec![display(&f.path), display(&schema_path), callee.path.clone()];
    let snapshots = f.core.read_snapshot(&f.root, &paths).unwrap().files;
    let witnesses = snapshots
        .iter()
        .map(|s| f.core.take_snapshot_witness(&s.base_id).unwrap())
        .collect::<Vec<_>>();
    let refs = vec![&witnesses[0], &witnesses[1], &witnesses[2], &witnesses[1]];
    let source = doc.source.clone() + "\nprotected owner save";
    let mixed_owner_path = f.root.join("Note/Note.typ");
    f.core
        .with_actor(ACTOR, |c| {
            c.with_guarded_snapshot_witnesses(&f.root, &refs, &mixed_owner_path, |c| {
                assert!(fs::write(&schema_path, b"changed").is_err());
                assert!(fs::write(&callee.path, b"changed").is_err());
                c.actor_save(
                    "Note",
                    HostedSaveRequest {
                        session_id: doc.session_id.clone(),
                        base_id: doc.base_id.clone(),
                        source,
                        operation_id: fresh(),
                    },
                )
            })
        })
        .unwrap();
    fs::write(&callee.path, b"changed after protection").unwrap();
    assert_eq!(
        f.core
            .with_guarded_snapshot_witnesses(
                &f.root,
                &[&witnesses[2]],
                &f.path,
                |_| -> Result<()> { panic!("stale callback") }
            )
            .unwrap_err()
            .code,
        "staleBase"
    );
}

#[test]
fn hosted_definition_replaced_owner_and_mismatched_admission_evidence_fail_closed() {
    let mut f = Fixture::new();
    let doc = f.open();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-object", &id)
        .unwrap();
    let key = proposal.status.key.unwrap();
    let source = proposal.patch.unwrap().source;
    let save_id = fresh();
    f.save(&doc, &source, std::slice::from_ref(&id), &save_id)
        .unwrap();
    let record = f
        .core
        .with_actor(ACTOR, |c| {
            Ok(serde_json::from_slice::<ActorOperation>(
                &fs::read(c.actor_record(&save_id)?).unwrap(),
            )
            .unwrap())
        })
        .unwrap();
    let path = f
        .core
        .journal_path(&record.core_operation_id, "definition-reservations");
    let mut evidence: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    evidence["sourceHash"] = serde_json::json!("0".repeat(64));
    private_replace(&path, &serde_json::to_vec(&evidence).unwrap()).unwrap();
    assert_eq!(
        f.core
            .is_definition_key_admitted(&f.root, doc.node_id.as_ref().unwrap(), &key)
            .unwrap_err()
            .code,
        "privateEvidence"
    );
    // Same declaration on a replacement object cannot acquire the old writer.
    fs::rename(&f.path, f.temp.path().join("owned-original.typ")).unwrap();
    fs::write(&f.path, &source).unwrap();
    assert!(
        f.core
            .with_actor(ACTOR, |c| c
                .check_session_base(&doc.session_id, &doc.base_id))
            .is_err()
    );
}

#[test]
fn hosted_definition_legacy_single_reservation_still_admits_only_its_original_save() {
    let mut f = Fixture::new();
    let doc = f.core.open(&f.path, Some(f.root.clone())).unwrap();
    let snapshot = f
        .core
        .read_snapshot(&f.root, &[display(&f.path)])
        .unwrap()
        .files
        .remove(0);
    let operation = fresh();
    let key = f
        .core
        .reserve_definition_key(&f.root, &f.path, &snapshot.base_id, &operation)
        .unwrap();
    let patch = weftext_data::upsert_saved_query(
        &doc.source,
        None,
        &key,
        "weftext-query-legacy",
        &f.binding,
        &QueryPlan::default(),
        &f.schema,
    )
    .unwrap();
    assert!(!f.admitted(&doc, &key));
    f.core
        .save(SaveRequest {
            session_id: doc.session_id,
            base_id: doc.base_id,
            source: patch.source,
            operation_id: operation.clone(),
        })
        .unwrap();
    let owner_id = doc.node_id.unwrap();
    assert!(
        f.core
            .is_definition_key_admitted(&f.root, &owner_id, &key)
            .unwrap()
    );
    assert_eq!(
        f.core
            .saved_operation_source(&operation)
            .unwrap()
            .unwrap()
            .definition_key
            .as_deref(),
        Some(key.as_str())
    );
    f.restart();
    assert!(
        f.core
            .is_definition_key_admitted(&f.root, &owner_id, &key)
            .unwrap()
    );
}

#[test]
fn hosted_definition_other_actor_cannot_claim_proposal_but_can_preserve_raw_native_source() {
    let mut f = Fixture::new();
    let doc = f.open();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-private", &id)
        .unwrap();
    let key = proposal.status.key.unwrap();
    let source = proposal.patch.unwrap().source;
    let bob = f
        .core
        .with_actor("hosted:bob", |c| c.open(&f.path, Some(f.root.clone())))
        .unwrap();
    assert_eq!(
        f.core
            .with_actor("hosted:bob", |c| c.actor_save_with_definition_reservations(
                "Note",
                &f.root,
                HostedSaveRequest {
                    session_id: bob.session_id.clone(),
                    base_id: bob.base_id.clone(),
                    source: source.clone(),
                    operation_id: fresh(),
                },
                std::slice::from_ref(&id)
            ))
            .unwrap_err()
            .code,
        "operationMissing"
    );
    f.core
        .with_actor("hosted:bob", |c| {
            c.actor_save(
                "Note",
                HostedSaveRequest {
                    session_id: bob.session_id,
                    base_id: bob.base_id,
                    source,
                    operation_id: fresh(),
                },
            )
        })
        .unwrap();
    assert!(!f.admitted(&doc, &key));
    assert_eq!(f.status(&id).status, "reserved");
}

#[test]
fn hosted_definition_cancel_in_preplan_gap_prevents_delayed_original_save() {
    let mut f = Fixture::new();
    let doc = f.open();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-gapcancel", &id)
        .unwrap();
    let key = proposal.status.key.unwrap();
    let source = proposal.patch.unwrap().source;
    let operation = fresh();
    FAILPOINT.with(|active| active.set(Some("admissionsDurable")));
    assert!(
        f.save(&doc, &source, std::slice::from_ref(&id), &operation)
            .is_err()
    );
    f.core
        .with_actor(ACTOR, |c| c.actor_definition_settle("Note", &id))
        .unwrap();
    assert_eq!(
        f.save(&doc, &source, std::slice::from_ref(&id), &operation)
            .unwrap_err()
            .code,
        "operationCancelled"
    );
    assert!(!f.admitted(&doc, &key));
    assert_eq!(fs::read_to_string(&f.path).unwrap(), doc.source);
}

#[test]
fn hosted_definition_guard_rejects_competing_witnesses_absence_budget_and_other_library() {
    let mut f = Fixture::new();
    let schema_path = f.root.join(".weftext-schema.json");
    let original = f
        .core
        .read_snapshot(&f.root, &[display(&schema_path)])
        .unwrap()
        .files
        .remove(0);
    let original = f.core.take_snapshot_witness(&original.base_id).unwrap();
    fs::write(&schema_path, b"changed schema").unwrap();
    let changed = f
        .core
        .read_snapshot(&f.root, &[display(&schema_path)])
        .unwrap()
        .files
        .remove(0);
    let changed = f.core.take_snapshot_witness(&changed.base_id).unwrap();
    assert_eq!(
        f.core
            .with_guarded_snapshot_witnesses(
                &f.root,
                &[&original, &changed],
                &f.path,
                |_| -> Result<()> { panic!("competing witness") }
            )
            .unwrap_err()
            .code,
        "staleBase"
    );
    assert_eq!(
        f.core
            .with_guarded_snapshot_witnesses(
                &f.root,
                &vec![&changed; 2049],
                &f.path,
                |_| -> Result<()> { panic!("budget") }
            )
            .unwrap_err()
            .code,
        "budget"
    );
    f.core
        .with_guarded_snapshot_witnesses(&f.root, &vec![&changed; 2048], &f.path, |_| {
            assert!(fs::write(&schema_path, b"changed while guarded").is_err());
            Ok(())
        })
        .unwrap();
    let mut competing = vec![&changed; 65];
    competing.push(&original);
    assert_eq!(
        f.core
            .with_guarded_snapshot_witnesses(&f.root, &competing, &f.path, |_| -> Result<()> {
                panic!("late competing duplicate")
            })
            .unwrap_err()
            .code,
        "staleBase"
    );
    let absent_path = f.root.join("absent.json");
    let absent = f
        .core
        .read_snapshot(&f.root, &[display(&absent_path)])
        .unwrap()
        .files
        .remove(0);
    let absent = f.core.take_snapshot_witness(&absent.base_id).unwrap();
    assert_eq!(
        f.core
            .with_guarded_snapshot_witnesses(&f.root, &[&absent], &f.path, |_| -> Result<()> {
                panic!("absence")
            })
            .unwrap_err()
            .code,
        "basisRequired"
    );
    let foreign_root = f.temp.path().join("foreign");
    f.core.create_library(&foreign_root).unwrap();
    let foreign = f
        .core
        .create_note_fixture(&foreign_root, &foreign_root, "Note")
        .unwrap();
    let foreign = f
        .core
        .read_snapshot(&foreign_root, &[foreign.path])
        .unwrap()
        .files
        .remove(0);
    let foreign = f.core.take_snapshot_witness(&foreign.base_id).unwrap();
    assert_eq!(
        f.core
            .with_guarded_snapshot_witnesses(&f.root, &[&foreign], &f.path, |_| -> Result<()> {
                panic!("foreign")
            })
            .unwrap_err()
            .code,
        "outsideRoot"
    );
}

#[test]
fn hosted_definition_guard_rejects_more_than_64_unique_dependencies_after_deduplication() {
    let mut f = Fixture::new();
    let mut witnesses = vec![];
    let paths = (0..65)
        .map(|n| {
            let path = f.root.join(format!("guarded-{n}.json"));
            fs::write(&path, b"dependency").unwrap();
            display(&path)
        })
        .collect::<Vec<_>>();
    for batch in paths.chunks(64) {
        let snapshots = f.core.read_snapshot(&f.root, batch).unwrap().files;
        for snapshot in snapshots {
            witnesses.push(f.core.take_snapshot_witness(&snapshot.base_id).unwrap());
        }
    }
    let mut refs = witnesses[..64].iter().collect::<Vec<_>>();
    refs.extend(std::iter::repeat_n(&witnesses[0], 65));
    f.core
        .with_guarded_snapshot_witnesses(&f.root, &refs, &f.path, |_| Ok(()))
        .unwrap();
    refs.push(&witnesses[64]);
    assert_eq!(
        f.core
            .with_guarded_snapshot_witnesses(&f.root, &refs, &f.path, |_| -> Result<()> {
                panic!("65 unique dependencies")
            })
            .unwrap_err()
            .code,
        "budget"
    );
    drop(refs);
    let mut pool = vec![];
    assert_eq!(
        Core::merge_snapshot_witnesses(&mut pool, witnesses)
            .unwrap_err()
            .code,
        "budget"
    );
    assert!(pool.is_empty());
}

#[test]
fn hosted_definition_checked_witness_merge_releases_duplicates_and_clears_competing_pool() {
    let mut f = Fixture::new();
    let schema_path = f.root.join(".weftext-schema.json");
    let mut pool = vec![];
    let snapshot_baseline = f.core.snapshots.len();
    for count in [1, 65] {
        let mut incoming = vec![];
        for _ in 0..count {
            let file = f
                .core
                .read_snapshot(&f.root, &[display(&schema_path)])
                .unwrap()
                .files
                .remove(0);
            incoming.push(f.core.take_snapshot_witness(&file.base_id).unwrap());
        }
        Core::merge_snapshot_witnesses(&mut pool, incoming).unwrap();
        assert_eq!(pool.len(), 1);
        assert_eq!(f.core.snapshots.len(), snapshot_baseline);
    }
    f.core
        .with_guarded_snapshot_witnesses(&f.root, &[&pool[0]], &f.path, |_| {
            assert!(fs::write(&schema_path, b"write while protected").is_err());
            Ok(())
        })
        .unwrap();
    fs::write(&schema_path, b"changed version").unwrap();
    let changed = f
        .core
        .read_snapshot(&f.root, &[display(&schema_path)])
        .unwrap()
        .files
        .remove(0);
    let changed = f.core.take_snapshot_witness(&changed.base_id).unwrap();
    assert_eq!(
        Core::merge_snapshot_witnesses(&mut pool, vec![changed])
            .unwrap_err()
            .code,
        "staleBase"
    );
    assert!(pool.is_empty());
    assert_eq!(f.core.snapshots.len(), snapshot_baseline);
}

#[test]
fn hosted_definition_pending_identity_budget_is_bounded_without_truncating_list() {
    let mut f = Fixture::new();
    let doc = f.open();
    for n in 0..MAX_PENDING_RESERVATIONS {
        f.reserve(
            &doc,
            &doc.source,
            &format!("weftext-query-pending-{n}"),
            &fresh(),
        )
        .unwrap();
    }
    assert_eq!(
        f.core
            .with_actor(ACTOR, |c| c.actor_definition_list(
                "Note",
                &f.root,
                &doc.session_id
            ))
            .unwrap()
            .len(),
        MAX_PENDING_RESERVATIONS
    );
    assert_eq!(
        f.reserve(&doc, &doc.source, "weftext-query-overbudget", &fresh())
            .unwrap_err()
            .code,
        "budget"
    );
}

#[test]
fn hosted_definition_first_admission_rejects_changed_owner_and_binding_before_source_commit() {
    let mut f = Fixture::new();
    let doc = f.open();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-firstbinding", &id)
        .unwrap();
    let key = proposal.status.key.unwrap();
    let source = proposal.patch.unwrap().source;
    let wrong_owner = source.replace(doc.node_id.as_ref().unwrap(), &fresh());
    assert_eq!(
        f.save(&doc, &wrong_owner, std::slice::from_ref(&id), &fresh())
            .unwrap_err()
            .code,
        "identityConflict"
    );
    let wrong_binding = source.replace(&f.binding.revision, &fresh());
    let save_id = fresh();
    assert_eq!(
        f.save(&doc, &wrong_binding, std::slice::from_ref(&id), &save_id)
            .unwrap_err()
            .code,
        "schemaBasisChanged"
    );
    assert!(
        f.core
            .with_actor(ACTOR, |c| c.actor_save_status("Note", &save_id))
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read_to_string(&f.path).unwrap(), doc.source);
    assert!(!f.admitted(&doc, &key));
    assert_eq!(f.status(&id).status, "reserved");
}

#[test]
fn hosted_definition_creation_patch_is_exact_utf16_insertion_and_can_recover_original_basis() {
    let mut f = Fixture::new();
    let doc = f.open();
    let before = doc.source.clone() + "\n😀 中文正文\n";
    let id = fresh();
    let patch = f
        .reserve(&doc, &before, "weftext-query-unicode", &id)
        .unwrap()
        .patch
        .unwrap();
    let from = utf16_byte_offset(&patch.source, patch.from).unwrap();
    let mut recovered = patch.source.clone();
    recovered.replace_range(from..from + patch.text.len(), "");
    assert_eq!(recovered, before);
    for malformed in ["replace", "text", "surrogate"] {
        let operation = fresh();
        let request = f.request(&doc, &operation);
        let result = f.core.with_actor(ACTOR, |c| {
            c.actor_definition_reserve("Note", &f.root, request, |key, binding| {
                let mut patch = weftext_data::upsert_saved_query(
                    &before,
                    None,
                    key,
                    "weftext-query-malformedpatch",
                    binding,
                    &QueryPlan::default(),
                    &f.schema,
                )
                .map_err(|e| CoreError::from_data("invalidQueryDefinition", e))?;
                match malformed {
                    "replace" => patch.to += 1,
                    "text" => patch.text.push_str("unmatched"),
                    _ => {
                        patch.from =
                            before[..before.find('😀').unwrap()].encode_utf16().count() + 1;
                        patch.to = patch.from;
                    }
                }
                Ok(patch)
            })
        });
        assert_eq!(result.unwrap_err().code, "definitionQualification");
        assert!(
            f.core
                .with_actor(ACTOR, |c| c.actor_definition_status("Note", &operation))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn hosted_definition_recovery_keep_current_never_admits_until_explicit_new_save() {
    let mut f = Fixture::new();
    let doc = f.open();
    let id = fresh();
    let proposal = f
        .reserve(&doc, &doc.source, "weftext-query-recovered", &id)
        .unwrap();
    let key = proposal.status.key.unwrap();
    let source = proposal.patch.unwrap().source;
    let save_id = fresh();
    FAILPOINT.with(|active| active.set(Some("sourceWritten")));
    assert!(
        f.save(&doc, &source, std::slice::from_ref(&id), &save_id)
            .is_err()
    );
    f.restart();
    let internal = f
        .core
        .with_actor(ACTOR, |c| {
            Ok(serde_json::from_slice::<ActorOperation>(
                &fs::read(c.actor_record(&save_id)?).unwrap(),
            )
            .unwrap()
            .core_operation_id)
        })
        .unwrap();
    f.core
        .with_actor(ACTOR, |c| c.resolve_recovery(&internal, "keepCurrent"))
        .unwrap();
    assert_eq!(f.status(&id).status, "reserved");
    assert!(!f.admitted(&doc, &key));
    assert_eq!(
        f.save(&doc, &source, std::slice::from_ref(&id), &save_id)
            .unwrap()
            .status,
        "resolved"
    );
    let current = f.open();
    f.save(&current, &source, std::slice::from_ref(&id), &fresh())
        .unwrap();
    assert!(f.admitted(&current, &key));
}
