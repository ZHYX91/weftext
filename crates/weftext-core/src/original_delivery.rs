//! Read-only delivery from the original physical effect, never its former name.
use super::*;
use operations::Mutation;

pub(crate) fn save_fingerprint(
    request: &SaveRequest,
    path: &Path,
    restore: Option<&history::RestoreBinding>,
    scope: QualifiedInputScope,
    producer: &str,
    input_revision: u64,
    author_digest: Option<&str>,
) -> Result<String> {
    file_save_scope::fingerprint(
        "save",
        path,
        &request.source,
        &FileSaveData::new(
            request.session_id.clone(),
            Some(request.base_id.clone()),
            scope,
            producer.to_owned(),
        )
        .with_input_revision(input_revision)
        .with_author_digest(author_digest.map(str::to_owned)),
        restore,
    )
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn same_writer_save_delivery_keeps_later_input_and_does_not_create_an_alternative() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let path = temp.path().join("ordinary.typ");
        fs::write(&path, "baseline").unwrap();
        let mut core = Core::new(&data).unwrap();
        let opened = core.open(&path, None).unwrap();
        let producer = core.sessions[&opened.session_id].draft_producer_id.clone();
        let id = fresh();
        let saved = core
            .save(SaveRequest {
                session_id: opened.session_id.clone(),
                base_id: opened.base_id,
                source: "committed X".into(),
                operation_id: id.clone(),
            })
            .unwrap();
        core.persist_draft(&opened.session_id, "later Y").unwrap();
        let (_, refreshed) = core
            .refresh_local_saved_session(&id, &opened.session_id)
            .unwrap();
        assert_eq!(refreshed.session_id, opened.session_id);
        assert_eq!(refreshed.base_id, saved.receipt.base_id);
        assert_eq!(refreshed.recovered_draft.as_deref(), Some("later Y"));
        assert!(!refreshed.recovered_draft_conflict);
        assert_eq!(
            core.sessions[&opened.session_id].draft_producer_id,
            producer
        );
        core.persist_draft(&opened.session_id, "latest Z 😀")
            .unwrap();
        let (_, replay) = core
            .refresh_local_saved_session(&id, &opened.session_id)
            .unwrap();
        assert_eq!(replay.base_id, refreshed.base_id);
        assert!(replay.recovered_draft_variants.is_empty());
        drop(core);
        let mut core = Core::new(data).unwrap();
        let recovered = core.open(path, None).unwrap();
        assert_eq!(recovered.recovered_draft.as_deref(), Some("latest Z 😀"));
        assert!(!recovered.recovered_draft_conflict);
        assert!(recovered.recovered_draft_variants.is_empty());
    }

    #[test]
    fn save_refresh_external_edit_is_comparison_without_rebasing_the_original_writer() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("ordinary.typ");
        fs::write(&path, "baseline").unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let original = core.open(&path, None).unwrap();
        let id = fresh();
        let saved = core
            .save(SaveRequest {
                session_id: original.session_id.clone(),
                base_id: original.base_id,
                source: "committed X".into(),
                operation_id: id.clone(),
            })
            .unwrap();
        core.persist_draft(&original.session_id, "pending Y")
            .unwrap();
        let before = core
            .current_session_draft_record(&original.session_id)
            .unwrap()
            .unwrap()
            .1;
        fs::write(&path, "external T").unwrap();
        let (committed, comparison) = core
            .refresh_local_saved_session(&id, &original.session_id)
            .unwrap();
        assert_eq!(committed, "committed X");
        assert_eq!(comparison.source, "external T");
        assert_ne!(comparison.session_id, original.session_id);
        assert_eq!(
            core.sessions[&original.session_id].base_id,
            saved.receipt.base_id
        );
        assert_eq!(core.sessions[&original.session_id].bytes, b"committed X");
        assert_eq!(
            core.current_session_draft_record(&original.session_id)
                .unwrap()
                .unwrap()
                .1,
            before
        );
        let error = core
            .save(SaveRequest {
                session_id: original.session_id,
                base_id: saved.receipt.base_id,
                source: "pending Y".into(),
                operation_id: fresh(),
            })
            .unwrap_err();
        assert_eq!(error.code, "conflict");
        assert_eq!(error.current_source.as_deref(), Some("external T"));
        assert_eq!(fs::read(path).unwrap(), b"external T");
    }
    #[test]
    fn save_refresh_never_adopts_a_competing_writer_or_grants_replay_a_producer() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("ordinary.typ");
        fs::write(&path, "baseline").unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let a = core.open(&path, None).unwrap();
        let id = fresh();
        core.save(SaveRequest {
            session_id: a.session_id.clone(),
            base_id: a.base_id,
            source: "committed".into(),
            operation_id: id.clone(),
        })
        .unwrap();
        let b = core.open(&path, None).unwrap();
        core.persist_draft(&b.session_id, "independent B").unwrap();
        let before = core
            .current_session_draft_record(&a.session_id)
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(
            core.refresh_local_saved_session(&id, &b.session_id)
                .unwrap_err()
                .code,
            "operationConflict"
        );
        assert_eq!(
            core.current_session_draft_record(&a.session_id)
                .unwrap()
                .unwrap()
                .1,
            before
        );
        let (_, refreshed) = core
            .refresh_local_saved_session(&id, &a.session_id)
            .unwrap();
        assert!(refreshed.recovered_draft_conflict);
        core.persist_draft(&a.session_id, "late A").unwrap();
        let retained = core.current_session_draft(&a.session_id).unwrap().unwrap();
        assert_eq!(retained.source, "late A");
        assert!(
            retained
                .alternatives
                .iter()
                .any(|input| input.source == "independent B")
        );
        let (_, independent) = core.reopen_local_save(&id).unwrap();
        assert_ne!(independent.session_id, a.session_id);
        assert_ne!(
            core.sessions[&independent.session_id].draft_producer_id,
            core.sessions[&a.session_id].draft_producer_id
        );
        core.close_session(&a.session_id).unwrap();
        assert_eq!(
            core.refresh_local_saved_session(&id, &a.session_id)
                .unwrap_err()
                .code,
            "sessionMissing"
        );
    }

    #[test]
    fn committed_move_refresh_keeps_the_original_writer_and_rejects_a_substitute() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let original = core.create_note_fixture(&root, &root, "Old").unwrap();
        let producer = core.sessions[&original.session_id]
            .draft_producer_id
            .clone();
        core.persist_draft(&original.session_id, "pending before move")
            .unwrap();
        let receipt = operate(
            &mut core,
            &root,
            OperationIntent::RenameNode {
                source: display(&root.join("Old")),
                name: "New".into(),
            },
        );
        let moved = core
            .refresh_local_operation_session(&receipt.operation_id, &original.session_id)
            .unwrap()
            .unwrap();
        assert_eq!(moved.session_id, original.session_id);
        assert_eq!(core.sessions[&moved.session_id].draft_producer_id, producer);
        core.persist_draft(&original.session_id, "late input after move")
            .unwrap();
        let replay = core
            .refresh_local_operation_session(&receipt.operation_id, &original.session_id)
            .unwrap()
            .unwrap();
        assert_eq!(replay.base_id, moved.base_id);
        assert_eq!(
            replay.recovered_draft.as_deref(),
            Some("late input after move")
        );
        assert!(replay.recovered_draft_variants.is_empty());
        let original_bytes = fs::read(&moved.path).unwrap();
        let retained = temp.path().join("retained.typ");
        fs::rename(&moved.path, &retained).unwrap();
        fs::write(&moved.path, &original_bytes).unwrap();
        let draft = core
            .current_session_draft_record(&original.session_id)
            .unwrap()
            .unwrap()
            .1;
        assert!(
            core.refresh_local_operation_session(&receipt.operation_id, &original.session_id)
                .is_err()
        );
        assert_eq!(
            core.current_session_draft_record(&original.session_id)
                .unwrap()
                .unwrap()
                .1,
            draft
        );
        assert_eq!(fs::read(retained).unwrap(), original_bytes);
    }
    #[test]
    fn old_local_writer_reopen_rejects_same_bytes_substitute_and_missing_or_hosted_session() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("ordinary.typ");
        fs::write(&path, "= Original S\n").unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let opened = core.open(&path, None).unwrap();
        let original_identity = core.sessions[&opened.session_id].stamp.identity.clone();
        let refreshed = core.reopen_local_session(&opened.session_id).unwrap();
        assert_eq!(
            core.sessions[&refreshed.session_id].stamp.identity,
            original_identity
        );
        let retained = temp.path().join("retained-original.typ");
        fs::rename(&path, &retained).unwrap();
        fs::write(&path, &opened.source).unwrap();
        let session_count = core.sessions.len();
        let error = core.reopen_local_session(&opened.session_id).unwrap_err();
        assert_eq!(error.code, "sourceIdentityChanged");
        assert_eq!(
            error.explanation,
            UiMessage::error_core_original_delivery_original_open_file_unavailable()
        );
        assert!(error.diagnostics.is_empty());
        assert_eq!(core.sessions.len(), session_count);
        assert_eq!(fs::read(&retained).unwrap(), fs::read(&path).unwrap());
        // A separately requested path open remains a new intent, not an old tab refresh.
        let replacement = core.open(&path, None).unwrap();
        assert_ne!(
            core.sessions[&replacement.session_id].stamp.identity,
            original_identity
        );
        fs::rename(&path, temp.path().join("retained-substitute.typ")).unwrap();
        fs::rename(&retained, &path).unwrap();
        fs::write(&path, "= Later T on the original object\n").unwrap();
        let latest = core.reopen_local_session(&opened.session_id).unwrap();
        assert_eq!(latest.source, "= Later T on the original object\n");
        assert_eq!(
            core.sessions[&latest.session_id].stamp.identity,
            original_identity
        );
        core.actor = Some(fresh());
        assert!(core.reopen_local_session(&opened.session_id).is_err());
        core.actor = None;
        core.close_session(&opened.session_id).unwrap();
        assert_eq!(
            core.reopen_local_session(&opened.session_id)
                .unwrap_err()
                .code,
            "sessionMissing"
        );
    }
    fn operate(core: &mut Core, root: &Path, intent: OperationIntent) -> OperationReceipt {
        let preview = core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(root),
                intent,
            })
            .unwrap();
        core.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
    }
    #[test]
    fn birth_and_restore_delivery_follow_original_physical_placement_and_never_a_reused_name() {
        for kind in ["new", "copy", "template", "restore"] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("library");
            let mut core = Core::new(temp.path().join("private")).unwrap();
            core.create_library(&root).unwrap();
            let seed = core.create_note_fixture(&root, &root, "Seed").unwrap();
            let parent = core.create_note_fixture(&root, &root, "Parent").unwrap();
            let intent = match kind {
                "new" => OperationIntent::NewNode {
                    parent: display(&root),
                    name: "X".into(),
                    source: None,
                },
                "copy" => OperationIntent::CopyNode {
                    source: display(&root.join("Seed")),
                    parent: display(&root),
                    name: "X".into(),
                    allow_losses: false,
                },
                "template" => {
                    let source = core
                        .read_snapshot(&root, std::slice::from_ref(&seed.path))
                        .unwrap()
                        .files
                        .remove(0);
                    OperationIntent::TemplateNode {
                        source: seed.path.clone(),
                        parent: display(&root),
                        name: "X".into(),
                        instantiated_source: seed.source.clone(),
                        source_base_id: source.base_id,
                    }
                }
                _ => {
                    let trash = operate(
                        &mut core,
                        &root,
                        OperationIntent::TrashNode {
                            source: display(&root.join("Seed")),
                        },
                    );
                    OperationIntent::RestoreNode {
                        trash_operation_id: trash.operation_id,
                        node_id: seed.node_id.clone(),
                        parent: display(&root),
                        name: "X".into(),
                    }
                }
            };
            let receipt = operate(&mut core, &root, intent);
            let original_path = display(&root.join("X/X.typ"));
            let x = core
                .reopen_local_operation_node(&receipt.operation_id, &original_path)
                .unwrap();
            let original_identity = core.sessions[&x.session_id].stamp.identity.clone();
            let original_bytes = fs::read(&x.path).unwrap();
            operate(
                &mut core,
                &root,
                OperationIntent::MoveNode {
                    source: display(&root.join("X")),
                    parent: display(Path::new(&parent.path).parent().unwrap()),
                },
            );
            let y = core.create_note_fixture(&root, &root, "X").unwrap();
            let moved = core
                .reopen_local_operation_node(&receipt.operation_id, &original_path)
                .unwrap();
            assert_eq!(moved.node_id, x.node_id, "{kind}");
            assert_ne!(moved.node_id, y.node_id, "{kind}");
            assert_eq!(
                core.sessions[&moved.session_id].stamp.identity,
                original_identity
            );
            assert_eq!(fs::read(&moved.path).unwrap(), original_bytes);
            assert_eq!(
                core.committed_operation_receipt(&receipt.operation_id)
                    .unwrap()
                    .unwrap()
                    .path,
                receipt.path
            );
            // A byte-identical portable ID at the exact current location is
            // still another physical file, not the original producer result.
            let retained = temp.path().join("retained-original.typ");
            fs::rename(&moved.path, &retained).unwrap();
            fs::write(&moved.path, &original_bytes).unwrap();
            assert!(
                core.reopen_local_operation_node(&receipt.operation_id, &original_path)
                    .is_err(),
                "{kind}"
            );
            assert_eq!(fs::read(&retained).unwrap(), original_bytes);
            // Preserve the substitute too, then put the original back.
            fs::rename(&moved.path, temp.path().join("retained-substitute.typ")).unwrap();
            fs::rename(&retained, &moved.path).unwrap();
            assert_eq!(
                core.reopen_local_operation_node(&receipt.operation_id, &original_path)
                    .unwrap()
                    .node_id,
                x.node_id
            );
        }
    }
    #[test]
    fn readonly_save_replay_requires_original_session_base_and_bytes_without_a_live_session() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let doc = core.create_note_fixture(&root, &root, "Note").unwrap();
        let request = || SaveRequest {
            session_id: doc.session_id.clone(),
            base_id: doc.base_id.clone(),
            source: format!("{}\nS", doc.source),
            operation_id: "8e4af0cf-96ec-4a15-82a8-41c99c9acc32".into(),
        };
        let outcome = core.save(request()).unwrap();
        core.sessions.clear();
        let count = core.operations.len();
        assert_eq!(
            core.saved_outcome_for_request(&request())
                .unwrap()
                .unwrap()
                .receipt
                .base_id,
            outcome.receipt.base_id
        );
        for change in ["session", "base", "source"] {
            let mut changed = request();
            match change {
                "session" => changed.session_id = fresh(),
                "base" => changed.base_id = fresh(),
                _ => changed.source.push('Y'),
            }
            assert_eq!(
                core.saved_outcome_for_request(&changed).unwrap_err().code,
                "operationConflict"
            );
        }
        assert_eq!(core.operations.len(), count);
        assert_eq!(fs::read_to_string(doc.path).unwrap(), request().source);
    }
    #[test]
    fn save_as_delivery_requires_original_materialized_file_even_for_identical_substitute_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let doc = core.create_note_fixture(&root, &root, "Source").unwrap();
        let path = temp.path().join("ordinary.typ");
        let id = fresh();
        let receipt = core
            .save_as(SaveAsRequest {
                session_id: doc.session_id,
                path: display(&path),
                source: doc.source,
                operation_id: id.clone(),
            })
            .unwrap()
            .receipt;
        let identity = core.materialized_identity(&id, &path).unwrap().unwrap();
        let (_, opened) = core.reopen_local_save(&id).unwrap();
        assert_eq!(core.sessions[&opened.session_id].stamp.identity, identity);
        let original = fs::read(&path).unwrap();
        let plan = fs::read(core.journal_path(&id, "plan")).unwrap();
        let commit = fs::read(core.journal_path(&id, "commit")).unwrap();
        let retained = temp.path().join("retained-original.typ");
        fs::rename(&path, &retained).unwrap();
        fs::write(&path, &original).unwrap();
        assert_eq!(
            core.local_operation_status(&id).unwrap()["baseId"],
            receipt.base_id
        );
        assert_eq!(
            core.reopen_local_save(&id).unwrap_err().code,
            "committedResultUnavailable"
        );
        assert_eq!(fs::read(core.journal_path(&id, "plan")).unwrap(), plan);
        assert_eq!(fs::read(core.journal_path(&id, "commit")).unwrap(), commit);
        assert_eq!(fs::read(&retained).unwrap(), original);
        fs::rename(&path, temp.path().join("retained-substitute.typ")).unwrap();
        fs::rename(&retained, &path).unwrap();
        assert_eq!(
            core.reopen_local_save(&id).unwrap().1.source.as_bytes(),
            original
        );
        let witness =
            core.journal_path(&id, &format!("object-{}", hash(display(&path).as_bytes())));
        let preserved = temp.path().join("retained-original-materialized.json");
        fs::rename(&witness, &preserved).unwrap();
        assert_eq!(
            core.local_operation_status(&id).unwrap()["status"],
            "committed"
        );
        assert_eq!(
            core.reopen_local_save(&id).unwrap_err().code,
            "committedResultUnavailable"
        );
        fs::rename(&preserved, &witness).unwrap();
        assert!(core.reopen_local_save(&id).is_ok());
    }
}

impl Core {
    /// Refresh an existing local writer, never select a replacement by its name
    /// or bytes. A missing Session has no authority to reopen an old tab.
    pub fn reopen_local_session(&mut self, session_id: &str) -> Result<OpenedDocument> {
        self.sync_local_owner()?;
        self.check_actor_session(session_id)?;
        let original = self.sessions.get(session_id).unwrap().clone();
        let unavailable = || {
            CoreError::new(
                "sourceIdentityChanged",
                UiMessage::error_core_original_delivery_original_open_file_unavailable(),
            )
        };
        let scope = original.file_scope.clone()?;
        self.validate_current_file_save_scope(&scope, &original.path)?;
        let guard = ParentGuard::acquire(&self.backend, &original.path)?;
        let file = guard.open_file(false, false)?;
        if stamp(&file)?.identity != original.stamp.identity {
            return Err(unavailable());
        }
        drop(file);
        drop(guard);
        let document = self.open(&original.path, Some(original.root))?;
        if self.sessions[&document.session_id].stamp.identity != original.stamp.identity
            || self.sessions[&document.session_id].node_id != original.node_id
            || self.sessions[&document.session_id].file_scope.as_ref().ok() != Some(&scope)
        {
            self.sessions.remove(&document.session_id);
            return Err(unavailable());
        }
        self.refresh_original_session(session_id, document, false)
    }

    // Keep the existing writer. No producer is transferred to a new Session,
    // and no receipt grants authority to adopt another surface's input.
    fn refresh_original_session(
        &mut self,
        session_id: &str,
        mut document: OpenedDocument,
        committed_basis_change: bool,
    ) -> Result<OpenedDocument> {
        let original = self.sessions[session_id].clone();
        let mut refreshed = self.sessions[&document.session_id].clone();
        if original.stamp.identity != refreshed.stamp.identity
            || original.node_id != refreshed.node_id
            || original.file_scope.as_ref().ok() != refreshed.file_scope.as_ref().ok()
            || original.actor != refreshed.actor
        {
            self.sessions.remove(&document.session_id);
            return Err(CoreError::new(
                "sourceIdentityChanged",
                UiMessage::error_core_original_delivery_original_open_file_unavailable(),
            ));
        }
        let unchanged = original.stamp == refreshed.stamp && original.bytes == refreshed.bytes;
        if document.recovered_draft_conflict || (!unchanged && !committed_basis_change) {
            // An external edit is an independent comparison result, not
            // consent to rebase the original writer or its pending input.
            return Ok(document);
        }
        if unchanged {
            refreshed.base_id = original.base_id.clone();
            document.base_id = original.base_id.clone();
        }
        self.sessions.remove(&document.session_id);
        self.sessions.insert(
            session_id.to_owned(),
            Session {
                draft_producer_id: original.draft_producer_id,
                draft_input: original.draft_input,
                draft_input_revision: original.draft_input_revision,
                draft_cuts: original.draft_cuts,
                draft_witness: original.draft_witness,
                conflicting_draft: original.conflicting_draft,
                ..refreshed
            },
        );
        document.session_id = session_id.to_owned();
        Ok(document)
    }

    /// Refresh the active author of this exact ordinary save. Independent
    /// result lookup and SaveAs continue to deliver independent Sessions.
    pub fn refresh_local_saved_session(
        &mut self,
        id: &str,
        session_id: &str,
    ) -> Result<(String, OpenedDocument)> {
        self.sync_local_owner()?;
        self.check_actor_session(session_id)?;
        valid_operation_id(id)?;
        let plan = self.operations.get(id).ok_or_else(|| {
            CoreError::new("operationMissing", UiMessage::error_reopen_file_session())
        })?;
        self.validate_file_save_plan(plan)?;
        if plan.kind != "save" || plan.file_save.as_ref().unwrap().session_id != session_id {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_original_scope_operation_transaction_identity_or_kind_differs(
                ),
            ));
        }
        let (source, document) = self.reopen_local_save(id)?;
        Ok((
            source,
            self.refresh_original_session(session_id, document, false)?,
        ))
    }

    /// Refresh only a live writer captured by this original transaction. Late
    /// writes still use that same producer; generic replay never acquires it.
    pub fn refresh_local_operation_session(
        &mut self,
        id: &str,
        session_id: &str,
    ) -> Result<Option<OpenedDocument>> {
        self.sync_local_owner()?;
        valid_operation_id(id)?;
        let original = self.sessions.get(session_id).ok_or_else(|| {
            CoreError::new("sessionMissing", UiMessage::error_reopen_file_session())
        })?;
        let original_path = self
            .operations
            .get(id)
            .and_then(|plan| plan.transaction.as_ref())
            .and_then(|transaction| {
                transaction
                    .files
                    .iter()
                    .find(|file| file.stamp.identity == original.stamp.identity)
                    .map(|file| {
                        (
                            transaction.relocated_path(&file.path),
                            file.stamp == original.stamp && file.bytes == original.bytes,
                        )
                    })
                    .or_else(|| {
                        transaction.relocations().into_iter().find_map(|step| {
                            if original.node_id.is_some()
                                && *step.leaf_identity == original.stamp.identity
                            {
                                Some((display(&Path::new(step.target).join(step.new_leaf)), false))
                            } else {
                                step.members
                                    .iter()
                                    .find(|member| {
                                        member.identity == original.stamp.identity
                                            && member.node_id.is_some()
                                            && member.node_id == original.node_id
                                    })
                                    .map(|member| {
                                        (
                                            display(
                                                &Path::new(step.target).join(&member.relative_path),
                                            ),
                                            false,
                                        )
                                    })
                            }
                        })
                    })
            });
        let Some((original_path, captured_basis)) = original_path else {
            return Ok(None);
        };
        self.check_actor_session(session_id)?;
        let scope = original.file_scope.clone()?;
        self.validate_original_input_scope(&scope)?;
        let document = self.reopen_local_operation_node(id, &original_path)?;
        let refreshed = &self.sessions[&document.session_id];
        let changed = refreshed.stamp != self.sessions[session_id].stamp
            || refreshed.bytes != self.sessions[session_id].bytes;
        let committed_basis_change = if changed && captured_basis {
            match self.read_observation_facts(id) {
                Ok(facts) => facts.iter().any(|fact| {
                    fact.stamp == refreshed.stamp && fact.hash == hash(&refreshed.bytes)
                }),
                Err(error) => {
                    self.sessions.remove(&document.session_id);
                    return Err(error);
                }
            }
        } else {
            false
        };
        self.refresh_original_session(session_id, document, committed_basis_change)
            .map(Some)
    }
    /// Verify the complete original source-save intent without a live Session
    /// and without executing save again. The original Plan owns its path.
    pub fn saved_outcome_for_request(&self, request: &SaveRequest) -> Result<Option<SaveOutcome>> {
        self.saved_outcome_for_request_revision(request, None, None)
    }
    /// A status/delivery replay never changes the original frozen author cut.
    /// When supplied, the client cut must match the frozen Plan participant;
    /// a generic status lookup still has no producer or writer transfer rights.
    pub fn saved_outcome_for_request_revision(
        &self,
        request: &SaveRequest,
        expected_revision: Option<u64>,
        expected_author_source: Option<&str>,
    ) -> Result<Option<SaveOutcome>> {
        self.sync_local_owner()?;
        valid_operation_id(&request.operation_id)?;
        let Some(plan) = self.operations.get(&request.operation_id) else {
            return Ok(None);
        };
        self.validate_file_save_plan(plan)?;
        let expected_digest = expected_author_source.map(|source| hash(source.as_bytes()));
        if plan.kind != "save"
            || plan.transaction.is_some()
            || expected_revision.is_some_and(|revision| {
                plan.file_save.as_ref().unwrap().input_revision != revision
                    || expected_digest.as_deref()
                        != plan
                            .file_save
                            .as_ref()
                            .unwrap()
                            .author_input_digest
                            .as_deref()
            })
            || save_fingerprint(
                request,
                Path::new(&plan.path),
                None,
                plan.file_save.as_ref().unwrap().scope.clone(),
                &plan.file_save.as_ref().unwrap().draft_producer_id,
                plan.file_save.as_ref().unwrap().input_revision,
                plan.file_save
                    .as_ref()
                    .unwrap()
                    .author_input_digest
                    .as_deref(),
            )? != plan.fingerprint
        {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_original_delivery_save_retry_differs(),
            ));
        }
        Ok(self
            .file_save_fact(&request.operation_id)?
            .map(SaveOutcome::committed))
    }

    /// Reopen one exact canonical result named by its original transaction.
    /// Current placement is projected from the original physical birth. A
    /// subsequently admitted name or copied NodeID is never a replacement.
    pub fn reopen_local_operation_node(
        &mut self,
        id: &str,
        original_path: &str,
    ) -> Result<OpenedDocument> {
        self.sync_local_owner()?;
        self.reopen_original_operation_node(id, original_path)
            .map_err(|cause| {
                if cause.code == "missing" {
                    CoreError::new(
                        "committedResultUnavailable",
                        UiMessage::error_core_original_delivery_physical_result_unavailable(),
                    )
                    .caused_by(cause)
                } else {
                    cause
                }
            })
    }
    /// Internal actor callers must first qualify their original category,
    /// actor, resource and physical library binding.
    pub(crate) fn reopen_original_operation_node(
        &mut self,
        id: &str,
        original_path: &str,
    ) -> Result<OpenedDocument> {
        valid_operation_id(id)?;
        let unavailable = || {
            CoreError::new(
                "committedResultUnavailable",
                UiMessage::error_core_original_delivery_physical_result_unavailable(),
            )
        };
        if !self.committed(id)? {
            return Err(unavailable());
        }
        let plan = self.operations.get(id).cloned().ok_or_else(unavailable)?;
        let transaction = plan.transaction.as_ref().ok_or_else(unavailable)?;
        let root = PathBuf::from(&transaction.request.root);
        let workspace = plan.workspace_id.as_ref().ok_or_else(unavailable)?;
        if self.require_workspace(&root)?.workspace_id != *workspace {
            return Err(unavailable());
        }
        let same = |left: &str, right: &str| -> Result<bool> {
            Ok(
                backend::namespace_path_compare(Path::new(left), Path::new(right))?
                    == std::cmp::Ordering::Equal,
            )
        };
        let mut born = Vec::new();
        for birth in transaction.births() {
            if same(&birth.path, original_path)? {
                born.push(birth)
            }
        }
        let birth = if born.len() == 1 && is_canonical(Path::new(original_path)) {
            let mut actual = self.read_births(id)?.into_iter().filter(|birth| {
                &birth.workspace_id == workspace && birth.node_id == born[0].node_id
            });
            let birth = actual.next().ok_or_else(unavailable)?;
            if actual.next().is_some() {
                return Err(unavailable());
            }
            if self
                .materialized_identity(id, Path::new(&born[0].path))?
                .as_ref()
                != Some(&birth.identity)
            {
                return Err(unavailable());
            }
            birth
        } else if born.is_empty() {
            let identity = match &transaction.mutation {
                Mutation::Relocate {
                    target,
                    new_leaf,
                    leaf_identity,
                    members,
                    ..
                } => {
                    if same(&display(&Path::new(target).join(new_leaf)), original_path)? {
                        leaf_identity.clone()
                    } else {
                        let mut found = None;
                        for member in members {
                            if member.node_id.is_some()
                                && same(
                                    &display(&Path::new(target).join(&member.relative_path)),
                                    original_path,
                                )?
                                && found.replace(member.identity.clone()).is_some()
                            {
                                return Err(unavailable());
                            }
                        }
                        found.ok_or_else(unavailable)?
                    }
                }
                Mutation::Update { .. } => {
                    let mut found = None;
                    for file in &transaction.files {
                        if same(&file.path, original_path)?
                            && found.replace(file.stamp.identity.clone()).is_some()
                        {
                            return Err(unavailable());
                        }
                    }
                    found.ok_or_else(unavailable)?
                }
                _ => return Err(unavailable()),
            };
            self.admitted_binding(&identity)?
                .filter(|birth| &birth.workspace_id == workspace)
                .ok_or_else(unavailable)?
        } else {
            return Err(unavailable());
        };
        let places = self.node_placement_paths(&birth)?;
        if places.len() != 1 {
            return Err(unavailable());
        }
        let path = PathBuf::from(places.into_iter().next().unwrap());
        let owner = weftext_knowledge::NodeRef {
            workspace_id: birth.workspace_id.clone(),
            node_id: birth.node_id.clone(),
        };
        // Creation and restore deliver live results. A later trash does not
        // silently install a retained read-only object into a writing session.
        self.qualified_query_node_at(&root, &path, &owner, "live")?;
        let opened = self.open(&path, Some(root))?;
        let session = &self.sessions[&opened.session_id];
        if session.stamp.identity != birth.identity
            || opened.node_id.as_ref() != Some(&birth.node_id)
            || opened.workspace_id.as_ref() != Some(&birth.workspace_id)
            || opened.kind != "managed"
        {
            self.sessions.remove(&opened.session_id);
            return Err(unavailable());
        }
        Ok(opened)
    }
    pub(crate) fn reopen_original_operation_member(
        &mut self,
        id: &str,
        owner: &weftext_knowledge::NodeRef,
    ) -> Result<OpenedDocument> {
        valid_operation_id(id)?;
        let invalid = || {
            CoreError::new(
                "committedResultUnavailable",
                UiMessage::error_core_original_delivery_producer_member_mismatch(),
            )
        };
        let plan = self.operations.get(id).ok_or_else(invalid)?;
        if plan.workspace_id.as_ref() != Some(&owner.workspace_id) {
            return Err(invalid());
        }
        let transaction = plan.transaction.as_ref().ok_or_else(invalid)?;
        let mut births = transaction
            .births()
            .iter()
            .filter(|birth| birth.node_id == owner.node_id);
        let path = births.next().ok_or_else(invalid)?.path.clone();
        if births.next().is_some() {
            return Err(invalid());
        }
        self.reopen_original_operation_node(id, &path)
    }
}
