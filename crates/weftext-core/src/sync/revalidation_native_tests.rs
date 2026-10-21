use super::*;
use crate::{ConfigRole, SaveRequest};
use std::{fs, sync::OnceLock};

fn engine() -> &'static weftext_engine::Engine {
    static ENGINE: OnceLock<weftext_engine::Engine> = OnceLock::new();
    ENGINE.get_or_init(|| weftext_engine::Engine::new_with_system_fonts(false).unwrap())
}
fn compile(inputs: &SyncNativeRenderInputs) -> Vec<SyncNativeRenderOutput> {
    (0..inputs.len())
        .map(|index| {
            engine()
                .render_frozen(&inputs.render_request(index).unwrap())
                .unwrap()
                .into()
        })
        .collect()
}
struct Fixture {
    _temp: tempfile::TempDir,
    a: Core,
    b: Core,
    ar: PathBuf,
    br: PathBuf,
    bd: PathBuf,
    identity: SyncIdentity,
    note: ObjectKey,
}
impl Fixture {
    fn new(body: &str) -> Self {
        Self::with_expression(
            body,
            "nextCivilDay(title == '精确标题' && body.contains('循环2') && !body.contains('不存在标题') && headings.exists(h, h.level == 12 && h.generated) ? date('2026-01-01') : date('9999-12-31'))",
        )
    }
    fn with_expression(body: &str, expression: &str) -> Self {
        Self::with_plan(
            body,
            weftext_data::QueryPlan {
                types: vec!["task".into()],
                select: vec![weftext_data::QueryColumn {
                    name: "actual".into(),
                    expr: expression.into(),
                }],
                ..Default::default()
            },
        )
    }
    fn with_plan(body: &str, plan: weftext_data::QueryPlan) -> Self {
        Self::with_plan_before_sync(body, plan, |_, _| {})
    }
    fn with_plan_before_sync(
        body: &str,
        plan: weftext_data::QueryPlan,
        before_sync: impl FnOnce(&mut Core, &Path),
    ) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let ar = temp.path().join("a-library");
        let br = temp.path().join("b-library");
        let bd = temp.path().join("b-private");
        let mut a = Core::new(temp.path().join("a-private")).unwrap();
        a.create_library(&ar).unwrap();
        let mut nodes = BTreeMap::new();
        for name in ["Note", "Task", "Unrelated"] {
            nodes.insert(
                name,
                ObjectKey::Node(
                    a.create_note_fixture(&ar, &ar, name)
                        .unwrap()
                        .node_id
                        .unwrap(),
                ),
            );
        }
        let schema = a.read_config(&ar, ConfigRole::Schema).unwrap();
        let saved = a
            .prepare_config_save(
                &ar,
                ConfigRole::Schema,
                schema.bytes,
                serde_json::to_vec(&weftext_data::Schema::bundled().unwrap()).unwrap(),
                &crate::fresh(),
            )
            .unwrap();
        a.execute_operation(&saved.operation_id, &saved.plan_id)
            .unwrap();
        for (name, source_body, task) in [
            ("Task", body, true),
            ("Unrelated", "#undefined-body-function()", false),
        ] {
            let path = ar.join(name).join(format!("{name}.typ"));
            let doc = a.open(&path, Some(ar.clone())).unwrap();
            let id = weftext_engine::inspect_metadata(&doc.source).id.unwrap();
            let metadata = if task {
                format!(
                    "#metadata((id: \"{id}\", types: (\"task\",), status: \"open\")) <weftext>\n"
                )
            } else {
                format!("#metadata((id: \"{id}\")) <weftext>\n")
            };
            a.save(SaveRequest {
                session_id: doc.session_id,
                base_id: doc.base_id,
                operation_id: crate::fresh(),
                source: metadata + source_body,
            })
            .unwrap();
        }
        before_sync(&mut a, &ar);
        let identity = a.enable_sync(&ar, &crate::fresh()).unwrap().identity;
        let mut keys = nodes.values().cloned().collect::<BTreeSet<_>>();
        keys.insert(ObjectKey::Schema);
        let mut b = Core::new(&bd).unwrap();
        b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
            .unwrap();
        let clone = b
            .prepare_operation(OperationRequest {
                operation_id: crate::fresh(),
                root: display(&br),
                intent: OperationIntent::CloneSync {
                    identity: identity.clone(),
                    objects: keys,
                },
            })
            .unwrap();
        b.execute_operation(&clone.operation_id, &clone.plan_id)
            .unwrap();
        let path = ar.join("Note").join("Note.typ");
        let doc = a.open(&path, Some(ar.clone())).unwrap();
        let canonical = a.managed_node(&ar, path.parent().unwrap(), false).unwrap();
        assert_eq!(
            canonical.path,
            display(&path),
            "requested={path:?}; opened={:?}; canonical={:?}",
            doc.path,
            canonical.path
        );
        let observed = a
            .read_snapshot(&ar, &[display(&path)])
            .unwrap()
            .files
            .remove(0);
        let operation = crate::fresh();
        let key = a
            .reserve_definition_key(&ar, &path, &observed.base_id, &operation)
            .unwrap();
        let schema = a.read_config(&ar, ConfigRole::Schema).unwrap();
        let source = weftext_data::upsert_saved_query_with_context(
            &doc.source,
            None,
            &key,
            "weftext-query-original",
            &weftext_data::SavedQueryBinding {
                schema: schema.role_ref,
                revision: schema.revision.unwrap(),
                language: "weftext.query.cel.v1".into(),
            },
            &plan,
            &weftext_data::Schema::bundled().unwrap(),
            &weftext_data::FrozenAdvancedInput::default(),
        )
        .unwrap()
        .source;
        a.save(SaveRequest {
            session_id: doc.session_id,
            base_id: doc.base_id,
            operation_id: operation,
            source,
        })
        .unwrap();
        let note = nodes.remove("Note").unwrap();
        a.publish_sync(&ar).unwrap();
        b.receive_sync(
            &identity,
            a.export_sync_objects(&identity, &BTreeSet::from([note.clone()]))
                .unwrap(),
        )
        .unwrap();
        Self {
            _temp: temp,
            a,
            b,
            ar,
            br,
            bd,
            identity,
            note,
        }
    }
    fn choice(&self) -> SyncRevalidation {
        let graph = self.b.read_sync_history(&self.identity).unwrap();
        let objects = graph.objects().unwrap();
        let view = &objects[&self.note];
        assert_eq!(view.versions.len(), 1);
        let chosen = &view.versions[0];
        SyncRevalidation {
            object: self.note.clone(),
            revisions: BTreeSet::from([chosen.revision.clone()]),
            decision: super::super::revalidation::SyncRevalidationDecision::Preserve {
                revision: chosen.revision.clone(),
            },
            state: chosen.state.clone(),
            reviewed_dependencies: vec![],
        }
    }
    fn capture_review(&mut self) -> Box<SyncNativeReview> {
        let choice = self.choice();
        match self
            .b
            .capture_sync_revalidation_review(&self.br, &self.identity, &[choice], &BTreeMap::new())
            .unwrap()
        {
            SyncReviewPreparation::Native(captured) => captured,
            SyncReviewPreparation::Reviewed(_) => {
                panic!("Native products must execute the real compiler")
            }
        }
    }
    fn request(&mut self) -> OperationRequest {
        let captured = self.capture_review();
        let mut next = SyncReviewPreparation::Native(captured);
        let review = loop {
            match next {
                SyncReviewPreparation::Reviewed(review) => break review,
                SyncReviewPreparation::Native(captured) => {
                    let outputs = compile(captured.render_inputs());
                    next = self
                        .b
                        .advance_sync_native_review(captured, outputs)
                        .unwrap();
                }
            }
        };
        OperationRequest {
            operation_id: crate::fresh(),
            root: display(&self.br),
            intent: OperationIntent::RevalidateSync {
                identity: self.identity.clone(),
                expected_installed: review.expected_installed,
                reviewed_versions: review.reviewed_versions,
                resolutions: review.resolutions,
                blobs: BTreeMap::new(),
            },
        }
    }
    fn capture(&mut self, request: OperationRequest) -> Box<SyncNativePreparation> {
        match self.b.capture_sync_operation(request).unwrap() {
            SyncOperationPreparation::Native(captured) => captured,
            SyncOperationPreparation::Prepared(_) => {
                panic!("Native qualification cannot be fabricated by metadata checking")
            }
        }
    }
}

#[test]
fn typed_native_same_request_retire_and_new_definition_reviews_the_actual_after_lifecycle() {
    let plan = weftext_data::QueryPlan {
        scope: weftext_data::QueryScope {
            lifecycle: "trash".into(),
            ..Default::default()
        },
        types: vec!["task".into()],
        select: vec![weftext_data::QueryColumn {
            name: "actual".into(),
            expr: "nextCivilDay(title == '精确标题' ? date('2026-01-01') : date('9999-12-31'))"
                .into(),
        }],
        ..Default::default()
    };
    let mut f = Fixture::with_plan(GOOD, plan);
    let task_id = weftext_engine::inspect_metadata(
        &fs::read_to_string(f.ar.join("Task").join("Task.typ")).unwrap(),
    )
    .id
    .unwrap();
    let task = ObjectKey::Node(task_id);
    let retired =
        f.a.prepare_operation(OperationRequest {
            operation_id: crate::fresh(),
            root: display(&f.ar),
            intent: OperationIntent::TrashNode {
                source: display(&f.ar.join("Task")),
            },
        })
        .unwrap();
    f.a.execute_operation(&retired.operation_id, &retired.plan_id)
        .unwrap();
    f.transfer(&BTreeSet::from([task.clone()]));
    assert!(f.br.join("Task").join("Task.typ").exists());
    let graph =
        f.b.read_sync_history(&f.identity)
            .unwrap()
            .objects()
            .unwrap();
    let version = &graph[&task].versions[0];
    assert!(version.state.retired);
    let choices = vec![
        f.choice(),
        SyncRevalidation {
            object: task,
            revisions: BTreeSet::from([version.revision.clone()]),
            decision: SyncRevalidationDecision::Preserve {
                revision: version.revision.clone(),
            },
            state: version.state.clone(),
            reviewed_dependencies: vec![],
        },
    ];
    let operations = f.b.operations.len();
    let snapshots = f.b.snapshots.len();
    let cuts = f.b.admitted_cuts.len();
    let original = capture_optional(&f.br.join("Task").join("Task.typ"))
        .unwrap()
        .unwrap();
    match f
        .b
        .capture_sync_revalidation_review(&f.br, &f.identity, &choices, &BTreeMap::new())
    {
        Err(error) => assert_eq!(error.code, "syncStructureRequired"),
        Ok(_) => panic!("A new Trash location cannot be guessed without the original operationId"),
    }
    let operation_id = crate::fresh();
    let preparation =
        f.b.capture_sync_revalidation_review_for_operation(
            &f.br,
            &f.identity,
            &choices,
            &BTreeMap::new(),
            Some(&operation_id),
        )
        .unwrap();
    let SyncReviewPreparation::Native(captured) = preparation else {
        panic!("The original after-lifecycle provider must consume real native products");
    };
    let expected_path =
        f.br.join(".weftext-trash")
            .join(&operation_id)
            .join("Task.typ");
    assert!(
        captured
            .render_inputs()
            .notes
            .iter()
            .any(|(_, path)| same(path, &expected_path))
    );
    let output = compile(captured.render_inputs());
    let review = f.b.finish_sync_native_review(captured, output).unwrap();
    assert_eq!(review.operation_id.as_deref(), Some(operation_id.as_str()));
    assert_eq!(f.b.operations.len(), operations);
    assert_eq!(f.b.snapshots.len(), snapshots);
    assert_eq!(f.b.admitted_cuts.len(), cuts);
    assert!(!expected_path.exists());
    let unchanged = capture_optional(&f.br.join("Task").join("Task.typ"))
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.bytes, original.bytes);
    assert_eq!(unchanged.stamp, original.stamp);
    let request = OperationRequest {
        operation_id,
        root: display(&f.br),
        intent: OperationIntent::RevalidateSync {
            identity: f.identity.clone(),
            expected_installed: review.expected_installed,
            reviewed_versions: review.reviewed_versions,
            resolutions: review.resolutions,
            blobs: BTreeMap::new(),
        },
    };
    let preview = f.prepare_native(request);
    f.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(
        capture_optional(&expected_path)
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        original.stamp.identity
    );
    assert!(!f.br.join("Task").exists());
    let trash = f.b.trash_admitted_snapshot(&f.br).unwrap();
    assert!(trash.complete);
    assert_eq!(trash.files.len(), 1);
}

impl Fixture {
    fn transfer(&mut self, objects: &BTreeSet<ObjectKey>) {
        self.a.publish_sync(&self.ar).unwrap();
        self.b
            .receive_sync(
                &self.identity,
                self.a.export_sync_objects(&self.identity, objects).unwrap(),
            )
            .unwrap();
    }
    fn apply(&mut self, objects: BTreeSet<ObjectKey>) {
        let preview = self
            .b
            .prepare_operation(OperationRequest {
                operation_id: crate::fresh(),
                root: display(&self.br),
                intent: OperationIntent::ApplySync {
                    identity: self.identity.clone(),
                    objects,
                },
            })
            .unwrap();
        self.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    }
    fn prepare_native(&mut self, request: OperationRequest) -> OperationPreview {
        let captured = self.capture(request);
        let output = compile(captured.render_inputs());
        self.b
            .finish_sync_native_preparation(captured, output)
            .unwrap()
    }
    fn install_definition(&mut self) {
        let request = self.request();
        let preview = self.prepare_native(request);
        self.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    }
    fn adoption_request(&mut self) -> OperationRequest {
        let objects = self
            .b
            .sync_binding_info(&self.br)
            .unwrap()
            .installed_versions
            .into_iter()
            .map(|(object, _)| object)
            .collect();
        let cut = self.b.read_sync_adoption_cut(&self.br, &objects).unwrap();
        OperationRequest {
            operation_id: crate::fresh(),
            root: display(&self.br),
            intent: OperationIntent::AdoptSyncExternal {
                identity: cut.identity,
                expected_installed: cut.expected_installed,
                changes: cut.changes,
                removed_members: cut.removed_members,
            },
        }
    }
    fn install_style(&mut self) {
        let style = self.a.read_library_style(&self.ar).unwrap();
        let preview = self
            .a
            .prepare_library_style(
                &self.ar,
                &style.base_id,
                &style.source,
                true,
                &crate::fresh(),
            )
            .unwrap();
        self.a
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        let objects = BTreeSet::from([ObjectKey::Settings]);
        self.transfer(&objects);
        self.apply(objects);
    }
}

fn trash_native_plan() -> weftext_data::QueryPlan {
    weftext_data::QueryPlan {
        types: vec!["task".into()],
        scope: weftext_data::QueryScope {
            lifecycle: "trash".into(),
            ..Default::default()
        },
        select: vec![weftext_data::QueryColumn {
            name: "actual".into(),
            expr: "nextCivilDay(title == '精确标题' && body.contains('循环2') && headings.exists(h, h.level == 12 && h.generated) ? date('2026-01-01') : date('9999-12-31'))".into(),
        }],
        ..Default::default()
    }
}

#[test]
fn semantic_membership_original_created_node_does_not_mask_a_later_foreign_birth() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    core.create_note_fixture(&root, &root, "Existing").unwrap();
    let cut = core.admitted_snapshot(&root).unwrap();
    let request = OperationRequest {
        operation_id: crate::fresh(),
        root: display(&root),
        intent: OperationIntent::NewNode {
            parent: display(&root),
            name: "OriginalBirth".into(),
            source: None,
        },
    };
    let preview = core
        .prepare_admitted_operation(request.clone(), &cut.cut_id)
        .unwrap();
    let original = core.operations[&preview.operation_id].clone();
    let transaction = original.transaction.as_ref().unwrap();
    assert_eq!(transaction.births().len(), 1);
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    drop(core);
    let mut core = Core::new(&data).unwrap();
    assert_eq!(
        core.prepare_operation(request).unwrap().plan_id,
        preview.plan_id
    );
    core.validate_recovery_scope(transaction).unwrap();
    let own = capture_optional(&root.join("OriginalBirth").join("OriginalBirth.typ"))
        .unwrap()
        .unwrap();
    let foreign = core
        .create_note_fixture(&root, &root, "LaterForeignBirth")
        .unwrap();
    let foreign_file = capture_optional(Path::new(&foreign.path)).unwrap().unwrap();
    assert_eq!(
        core.validate_recovery_scope(transaction).unwrap_err().code,
        "scopeChanged"
    );
    assert_eq!(
        capture_optional(Path::new(&own.path))
            .unwrap()
            .unwrap()
            .stamp,
        own.stamp
    );
    assert_eq!(
        capture_optional(Path::new(&foreign.path))
            .unwrap()
            .unwrap()
            .stamp,
        foreign_file.stamp
    );
}

#[test]
fn typed_native_inherited_trash_child_keeps_original_false_flag_and_actual_compiled_lifecycle() {
    let mut f = Fixture::with_plan_before_sync(GOOD, trash_native_plan(), |core, root| {
        for intent in [
            OperationIntent::MoveNode {
                source: display(&root.join("Task")),
                parent: display(&root.join("Unrelated")),
            },
            OperationIntent::TrashNode {
                source: display(&root.join("Unrelated")),
            },
        ] {
            let preview = core
                .prepare_operation(OperationRequest {
                    operation_id: crate::fresh(),
                    root: display(root),
                    intent,
                })
                .unwrap();
            core.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
        }
    });
    let local =
        f.b.sync_local_state(f.b.sync_setup_for(&f.br).unwrap())
            .unwrap()
            .0;
    let (task, child) = local
        .iter()
        .find(|(_, object)| object.state.name == "Task")
        .unwrap();
    assert!(!child.state.retired);
    let parent = child.state.parent.as_ref().unwrap();
    assert!(local[parent].state.retired);
    let source_path = Path::new(&child.directory).join("Task.typ");
    let original = capture_optional(&source_path).unwrap().unwrap();
    let task = task.clone();
    let request = f.request();
    let preview = f.prepare_native(request);
    f.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let after =
        f.b.sync_local_state(f.b.sync_setup_for(&f.br).unwrap())
            .unwrap()
            .0;
    assert!(!after[&task].state.retired);
    assert_eq!(
        after[&task].directory,
        display(source_path.parent().unwrap())
    );
    let current = capture_optional(&source_path).unwrap().unwrap();
    assert_eq!(current.bytes, original.bytes);
    assert_eq!(current.stamp.identity, original.stamp.identity);
    let trash = f.b.trash_admitted_snapshot(&f.br).unwrap();
    assert!(trash.complete);
    assert!(
        trash
            .files
            .iter()
            .any(|file| file.path == display(&source_path))
    );
}

#[test]
fn typed_native_multiple_retired_roots_share_the_original_compound_layout_and_file_identities() {
    let mut f = Fixture::with_plan(GOOD, trash_native_plan());
    let late =
        f.b.create_note_fixture(&f.br, &f.br, "LateForeign")
            .unwrap();
    let late_directory = Path::new(&late.path).parent().unwrap().to_path_buf();
    let late_original = capture_optional(Path::new(&late.path)).unwrap().unwrap();
    let foreign =
        f.b.create_note_fixture(&f.br, &f.br, "ExistingTrash")
            .unwrap();
    let foreign_retirement =
        f.b.prepare_operation(OperationRequest {
            operation_id: crate::fresh(),
            root: display(&f.br),
            intent: OperationIntent::TrashNode {
                source: display(Path::new(&foreign.path).parent().unwrap()),
            },
        })
        .unwrap();
    f.b.execute_operation(
        &foreign_retirement.operation_id,
        &foreign_retirement.plan_id,
    )
    .unwrap();
    f.b.publish_sync(&f.br).unwrap();
    let foreign_root = PathBuf::from(
        f.b.trash_entries(&f.br)
            .unwrap()
            .into_iter()
            .find(|entry| Some(&entry.node_id) == foreign.node_id.as_ref())
            .unwrap()
            .path,
    );
    let mut objects = BTreeSet::from([f.note.clone()]);
    let mut original_files = BTreeMap::new();
    for name in ["Task", "Unrelated"] {
        let source_path = f.br.join(name).join(format!("{name}.typ"));
        let source = capture_optional(&source_path).unwrap().unwrap();
        let id =
            weftext_engine::inspect_metadata(&String::from_utf8(source.bytes.clone()).unwrap())
                .id
                .unwrap();
        let object = ObjectKey::Node(id);
        objects.insert(object.clone());
        original_files.insert(object, source);
        let retired =
            f.a.prepare_operation(OperationRequest {
                operation_id: crate::fresh(),
                root: display(&f.ar),
                intent: OperationIntent::TrashNode {
                    source: display(&f.ar.join(name)),
                },
            })
            .unwrap();
        f.a.execute_operation(&retired.operation_id, &retired.plan_id)
            .unwrap();
    }
    f.transfer(&objects);
    let graph = f.b.read_sync_history(&f.identity).unwrap();
    let views = graph.objects().unwrap();
    let choices = objects
        .iter()
        .map(|object| {
            let [version] = views[object].versions.as_slice() else {
                panic!("Actual sender roots have exact singleton versions")
            };
            SyncRevalidation {
                object: object.clone(),
                revisions: BTreeSet::from([version.revision.clone()]),
                decision: SyncRevalidationDecision::Preserve {
                    revision: version.revision.clone(),
                },
                state: version.state.clone(),
                reviewed_dependencies: vec![],
            }
        })
        .collect::<Vec<_>>();
    let operation_id = crate::fresh();
    let original_count = f.b.operations.len();
    let mut review =
        f.b.capture_sync_revalidation_review_for_operation(
            &f.br,
            &f.identity,
            &choices,
            &BTreeMap::new(),
            Some(&operation_id),
        )
        .unwrap();
    let review = loop {
        match review {
            SyncReviewPreparation::Reviewed(review) => break review,
            SyncReviewPreparation::Native(captured) => {
                for (object, original) in &original_files {
                    let ObjectKey::Node(id) = object else {
                        unreachable!()
                    };
                    let name = &views[object].versions[0].state.name;
                    let expected =
                        f.br.join(".weftext-trash")
                            .join(format!("{operation_id}-node-{id}"))
                            .join(format!("{name}.typ"));
                    assert!(
                        captured
                            .render_inputs()
                            .notes
                            .iter()
                            .any(|(_, path)| same(path, &expected))
                    );
                    assert_eq!(
                        capture_optional(Path::new(&original.path))
                            .unwrap()
                            .unwrap()
                            .stamp,
                        original.stamp
                    );
                    assert!(!expected.exists());
                }
                let output = compile(captured.render_inputs());
                review = f.b.advance_sync_native_review(captured, output).unwrap();
            }
        }
    };
    assert_eq!(f.b.operations.len(), original_count);
    assert_eq!(review.operation_id.as_deref(), Some(operation_id.as_str()));
    let request = OperationRequest {
        operation_id: operation_id.clone(),
        root: display(&f.br),
        intent: OperationIntent::RevalidateSync {
            identity: f.identity.clone(),
            expected_installed: review.expected_installed,
            reviewed_versions: review.reviewed_versions,
            resolutions: review.resolutions,
            blobs: BTreeMap::new(),
        },
    };
    let preview = f.prepare_native(request.clone());
    assert!(matches!(
        f.b.operations[&operation_id]
            .transaction
            .as_ref()
            .unwrap()
            .mutation,
        crate::operations::Mutation::Compound { .. }
    ));
    let frozen_plan = f.b.operations[&operation_id].clone();
    let transaction = frozen_plan.transaction.as_ref().unwrap();
    assert_eq!(
        transaction.expected_trash_membership.as_ref(),
        Some(&BTreeSet::from([foreign.node_id.clone().unwrap()]))
    );
    let moves = transaction
        .relocations()
        .iter()
        .map(|step| (PathBuf::from(step.source), PathBuf::from(step.target)))
        .collect::<Vec<_>>();
    assert_eq!(moves.len(), 2);
    crate::operations::compound::set_compound_failpoint("move0");
    let interrupted =
        f.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err();
    assert_eq!(interrupted.code, "recoveryPending");
    assert!(
        interrupted.message.contains("underlying testInterrupted:"),
        "{interrupted:?}"
    );
    assert!(moves[0].1.is_dir());
    assert!(moves[1].0.is_dir());
    assert!(!f.b.committed(&operation_id));
    let install_path = f.b.journal_path(&operation_id, "install");
    let install = fs::read(&install_path).unwrap();
    drop(f.b);
    f.b = Core::new(&f.bd).unwrap();
    f.b.validate_recovery_scope(transaction).unwrap();
    assert!(
        f.b.inspect_recovery(&operation_id)
            .unwrap()
            .allowed_actions
            .iter()
            .any(|action| action == "resumeOriginalPlan")
    );
    let late_target = foreign_root.join("LateForeign");
    fs::rename(&late_directory, &late_target).unwrap();
    let late_current = capture_optional(&late_target.join("LateForeign.typ"))
        .unwrap()
        .unwrap();
    assert_eq!(late_current.stamp.identity, late_original.stamp.identity);
    assert_eq!(
        f.b.validate_recovery_scope(transaction).unwrap_err().code,
        "scopeChanged"
    );
    assert!(
        f.b.inspect_recovery(&operation_id)
            .unwrap()
            .allowed_actions
            .is_empty()
    );
    for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
        assert!(f.b.resolve_recovery(&operation_id, action).is_err());
    }
    assert_eq!(fs::read(&install_path).unwrap(), install);
    assert!(!f.b.committed(&operation_id));
    assert_eq!(
        fs::read(late_target.join("LateForeign.typ")).unwrap(),
        late_original.bytes
    );
    fs::rename(&late_target, &late_directory).unwrap();
    f.b.validate_recovery_scope(transaction).unwrap();
    f.b.resolve_recovery(&operation_id, "resumeOriginalPlan")
        .unwrap();
    assert!(f.b.committed(&operation_id));
    assert_eq!(
        f.b.prepare_operation(request).unwrap().plan_id,
        preview.plan_id
    );
    for (object, original) in original_files {
        let ObjectKey::Node(id) = object else {
            unreachable!()
        };
        let name = Path::new(&original.path).file_name().unwrap();
        let target =
            f.br.join(".weftext-trash")
                .join(format!("{operation_id}-node-{id}"))
                .join(name);
        let after = capture_optional(&target).unwrap().unwrap();
        assert_eq!(after.bytes, original.bytes);
        assert_eq!(after.stamp.identity, original.stamp.identity);
        assert!(!Path::new(&original.path).exists());
    }
}
const GOOD: &str = "#let generated = [精确标题]\n#title(generated)\n#if false [= 不存在标题]\n#heading(level:12)[实际深标题]\n#for n in (1,2) [循环#n]\n";

#[test]
fn typed_native_final_products_drive_actual_provider_and_original_plan_across_restart() {
    let mut f = Fixture::new(GOOD);
    let operations = f.b.operations.len();
    let snapshots = f.b.snapshots.len();
    let cuts = f.b.admitted_cuts.len();
    let before = fs::read(f.br.join("Note/Note.typ")).unwrap();
    let captured = f.capture_review();
    let outputs = compile(captured.render_inputs());
    assert!(
        outputs.iter().any(|output| !output.ok),
        "Unrelated invalid bodies are real compiler failures"
    );
    let task = outputs
        .iter()
        .find_map(|output| {
            output
                .products
                .as_ref()
                .filter(|products| products.title.as_deref() == Some("精确标题"))
        })
        .unwrap();
    assert!(task.body.as_ref().unwrap().contains("循环2"));
    assert!(!task.body.as_ref().unwrap().contains("不存在标题"));
    assert_eq!(task.headings.len(), 1);
    assert_eq!(task.headings[0].level, 12);
    assert!(task.headings[0].generated);
    let review = f.b.finish_sync_native_review(captured, outputs).unwrap();
    assert_eq!(f.b.operations.len(), operations);
    assert_eq!(f.b.snapshots.len(), snapshots);
    assert_eq!(f.b.admitted_cuts.len(), cuts);
    assert_eq!(fs::read(f.br.join("Note/Note.typ")).unwrap(), before);
    let request = OperationRequest {
        operation_id: crate::fresh(),
        root: display(&f.br),
        intent: OperationIntent::RevalidateSync {
            identity: f.identity.clone(),
            expected_installed: review.expected_installed,
            reviewed_versions: review.reviewed_versions,
            resolutions: review.resolutions,
            blobs: BTreeMap::new(),
        },
    };
    let captured = f.capture(request.clone());
    assert!(!f.b.operations.contains_key(&request.operation_id));
    let outputs = compile(captured.render_inputs());
    let preview =
        f.b.finish_sync_native_preparation(captured, outputs)
            .unwrap();
    assert_eq!(fs::read(f.br.join("Note/Note.typ")).unwrap(), before);
    let evidence = f.b.operations[&preview.operation_id]
        .sync_application
        .as_ref()
        .unwrap()
        .semantic_evidence
        .as_ref()
        .unwrap();
    assert!(
        evidence
            .native_products
            .iter()
            .any(|proof| proof["role"] == "frozenNativeInput")
    );
    assert!(
        evidence
            .native_products
            .iter()
            .any(|proof| proof["target"] == "html" && proof["ok"] == true)
    );
    assert!(!evidence.provider_dependencies.is_empty());
    drop(f.b);
    f.b = Core::new(&f.bd).unwrap();
    assert_eq!(
        f.b.prepare_operation(request.clone()).unwrap().plan_id,
        preview.plan_id
    );
    f.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(f.b.prepare_operation(request).unwrap().status, "committed");
    assert_ne!(fs::read(f.br.join("Note/Note.typ")).unwrap(), before);
}

#[test]
fn typed_native_bad_final_products_cannot_be_replaced_by_empty_or_static_rows() {
    for body in [
        GOOD.replace("精确标题", "错误标题"),
        "#set document(title:[早期标题])\n#title()\n#set document(title:[后期元标题])\n正文".into(),
        "#html.frame[无法承诺流式文字]".into(),
    ] {
        let mut f = Fixture::new(&body);
        let before = fs::read(f.br.join("Note/Note.typ")).unwrap();
        let operations = f.b.operations.len();
        let captured = f.capture_review();
        let outputs = compile(captured.render_inputs());
        assert_eq!(
            f.b.finish_sync_native_review(captured, outputs)
                .unwrap_err()
                .code,
            "queryInvocationUnavailable"
        );
        assert_eq!(f.b.operations.len(), operations);
        assert_eq!(fs::read(f.br.join("Note/Note.typ")).unwrap(), before);
    }
}

#[test]
fn typed_native_late_finish_preserves_original_same_id_cancel_and_rejects_changed_request() {
    let mut f = Fixture::new(GOOD);
    let request = f.request();
    let first = f.capture(request.clone());
    let late = f.capture(request.clone());
    let outputs = compile(first.render_inputs());
    let preview = f.b.finish_sync_native_preparation(first, outputs).unwrap();
    f.b.cancel_prepared_sync(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let before = fs::read(f.br.join("Note/Note.typ")).unwrap();
    assert_eq!(
        f.b.finish_sync_native_preparation(late, vec![])
            .unwrap()
            .status,
        "resolved"
    );
    let mut changed = request;
    changed.root.push('x');
    match f.b.capture_sync_operation(changed) {
        Err(error) => assert_eq!(error.code, "operationConflict"),
        Ok(_) => panic!("A changed native request cannot reuse the original operation"),
    }
    assert_eq!(fs::read(f.br.join("Note/Note.typ")).unwrap(), before);
    assert!(!f.b.journal_path(&preview.operation_id, "install").exists());
}

#[test]
fn typed_native_settings_missing_to_present_is_an_original_negative_read() {
    let mut f = Fixture::new(GOOD);
    let request = f.request();
    let captured = f.capture(request.clone());
    let outputs = compile(captured.render_inputs());
    let path = f.br.join(".weftext-settings.json");
    // Real raw external observation, without inventing a received object.
    assert!(!path.exists());
    fs::write(&path, b"{\"version\":1}").unwrap();
    assert_eq!(
        f.b.finish_sync_native_preparation(captured, outputs)
            .unwrap_err()
            .code,
        "staleBase"
    );
    assert!(!f.b.operations.contains_key(&request.operation_id));
    assert_eq!(fs::read(path).unwrap(), b"{\"version\":1}");
}

#[test]
fn typed_native_late_received_row_invalidates_the_original_reviewed_cut() {
    let mut f = Fixture::new(GOOD);
    let request = f.request();
    let captured = f.capture(request.clone());
    let outputs = compile(captured.render_inputs());
    let task_path = f.ar.join("Task/Task.typ");
    let document = f.a.open(&task_path, Some(f.ar.clone())).unwrap();
    let id = weftext_engine::inspect_metadata(&document.source)
        .id
        .unwrap();
    f.a.save(SaveRequest {
        session_id: document.session_id,
        base_id: document.base_id,
        operation_id: crate::fresh(),
        source: document.source + "\nlate received source",
    })
    .unwrap();
    f.a.publish_sync(&f.ar).unwrap();
    f.b.receive_sync(
        &f.identity,
        f.a.export_sync_objects(&f.identity, &BTreeSet::from([ObjectKey::Node(id)]))
            .unwrap(),
    )
    .unwrap();
    let before = fs::read(f.br.join("Note/Note.typ")).unwrap();
    assert_eq!(
        f.b.finish_sync_native_preparation(captured, outputs)
            .unwrap_err()
            .code,
        "syncConflictChanged"
    );
    assert!(!f.b.operations.contains_key(&request.operation_id));
    assert_eq!(fs::read(f.br.join("Note/Note.typ")).unwrap(), before);
}

#[test]
fn typed_native_real_include_identity_is_retained_and_rechecked_before_finish_and_after_restart() {
    for before_finish in [true, false] {
        let mut f = Fixture::new(&(GOOD.to_owned() + "\n#include \"../shared.typ\""));
        let include = f.br.join("shared.typ");
        fs::write(f.ar.join("shared.typ"), "原始共享输入").unwrap();
        fs::write(&include, "原始共享输入").unwrap();
        let request = f.request();
        let captured = f.capture(request.clone());
        let outputs = compile(captured.render_inputs());
        assert!(outputs.iter().any(|output| {
            output
                .dependencies
                .iter()
                .any(|dependency| dependency.path == "shared.typ" && dependency.status == "loaded")
        }));
        let before = fs::read(f.br.join("Note/Note.typ")).unwrap();
        if before_finish {
            fs::rename(&include, f.br.join("retained-original-shared.typ")).unwrap();
            fs::write(&include, "原始共享输入").unwrap();
            assert_eq!(
                f.b.finish_sync_native_preparation(captured, outputs)
                    .unwrap_err()
                    .code,
                "staleBase"
            );
            assert!(!f.b.operations.contains_key(&request.operation_id));
        } else {
            let preview =
                f.b.finish_sync_native_preparation(captured, outputs)
                    .unwrap();
            assert!(
                f.b.operations[&preview.operation_id]
                    .transaction
                    .as_ref()
                    .unwrap()
                    .files
                    .iter()
                    .any(|file| same(Path::new(&file.path), &include))
            );
            drop(f.b);
            f.b = Core::new(&f.bd).unwrap();
            fs::write(&include, "稍后共享输入").unwrap();
            assert_eq!(
                f.b.execute_operation(&preview.operation_id, &preview.plan_id)
                    .unwrap_err()
                    .code,
                "conflict"
            );
            assert!(!f.b.journal_path(&preview.operation_id, "install").exists());
        }
        assert_eq!(fs::read(f.br.join("Note/Note.typ")).unwrap(), before);
    }
}

#[test]
fn typed_native_direct_select_rejects_actually_consumed_unavailable_or_ambiguous_cells() {
    for (body, expression) in [
        ("#undefined-body-function()", "title"),
        ("#html.frame[无法承诺流式文字]", "body"),
        (
            "#set document(title:[早期标题])\n#title()\n#set document(title:[后期元标题])",
            "title",
        ),
    ] {
        let mut f = Fixture::with_expression(body, expression);
        let operations = f.b.operations.len();
        let captured = f.capture_review();
        let outputs = compile(captured.render_inputs());
        assert_eq!(
            f.b.finish_sync_native_review(captured, outputs)
                .unwrap_err()
                .code,
            "queryInvocationUnavailable"
        );
        assert_eq!(f.b.operations.len(), operations);
    }
}

#[test]
fn typed_native_review_closes_a_real_independent_directory_read_without_preparing_or_upgrading() {
    let mut f = Fixture::new(GOOD);
    let directory = f.ar.join("Independent");
    fs::create_dir(&directory).unwrap();
    let admitted =
        f.a.prepare_directory_admission(&f.ar, &directory, &crate::fresh())
            .unwrap();
    f.a.execute_operation(&admitted.preview.operation_id, &admitted.preview.plan_id)
        .unwrap();
    let directory_key =
        ObjectKey::Directory(admitted.target_ref.rsplit('/').next().unwrap().into());
    f.transfer(&BTreeSet::from([directory_key.clone()]));
    f.apply(BTreeSet::from([directory_key.clone()]));
    let task_path = f.ar.join("Task").join("Task.typ");
    let task = f.a.open(&task_path, Some(f.ar.clone())).unwrap();
    let task_id = weftext_engine::inspect_metadata(&task.source).id.unwrap();
    f.a.save(SaveRequest {
        session_id: task.session_id,
        base_id: task.base_id,
        operation_id: crate::fresh(),
        source: task.source + "\n#read(\"../Independent/weftext.directory.json\")",
    })
    .unwrap();
    f.transfer(&BTreeSet::from([ObjectKey::Node(task_id.clone())]));
    f.apply(BTreeSet::from([ObjectKey::Node(task_id)]));
    let operations = f.b.operations.len();
    let original = f.capture_review();
    assert!(!original.native.selected.contains(&directory_key));
    let unreviewed = OperationRequest {
        operation_id: crate::fresh(),
        root: display(&f.br),
        intent: OperationIntent::RevalidateSync {
            identity: f.identity.clone(),
            expected_installed: original.review.expected_installed.clone(),
            reviewed_versions: original.review.reviewed_versions.clone(),
            resolutions: original.review.resolutions.clone(),
            blobs: BTreeMap::new(),
        },
    };
    let unreviewed_capture = f.capture(unreviewed.clone());
    let unreviewed_outputs = compile(unreviewed_capture.render_inputs());
    assert_eq!(
        f.b.finish_sync_native_preparation(unreviewed_capture, unreviewed_outputs)
            .unwrap_err()
            .code,
        "syncDependencyRequired"
    );
    assert!(!f.b.operations.contains_key(&unreviewed.operation_id));
    let outputs = compile(original.render_inputs());
    assert!(outputs.iter().any(|output| output.dependencies.iter().any(
        |dependency| dependency.path == "Independent/weftext.directory.json"
            && dependency.status == "loaded"
    )));
    let next = f.b.advance_sync_native_review(original, outputs).unwrap();
    let SyncReviewPreparation::Native(next) = next else {
        panic!("A newly discovered native owner must be recompiled at its explicit exact version");
    };
    assert!(next.native.selected.contains(&directory_key));
    let outputs = compile(next.render_inputs());
    let SyncReviewPreparation::Reviewed(review) =
        f.b.advance_sync_native_review(next, outputs).unwrap()
    else {
        panic!("The finite read closure must settle");
    };
    assert!(
        review.resolutions[0]
            .reviewed_dependencies
            .iter()
            .any(|dependency| dependency.object == directory_key)
    );
    assert_eq!(f.b.operations.len(), operations);
    let request = OperationRequest {
        operation_id: crate::fresh(),
        root: display(&f.br),
        intent: OperationIntent::RevalidateSync {
            identity: f.identity.clone(),
            expected_installed: review.expected_installed,
            reviewed_versions: review.reviewed_versions,
            resolutions: review.resolutions,
            blobs: BTreeMap::new(),
        },
    };
    let preview = f.prepare_native(request);
    f.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}

#[test]
fn typed_native_concurrent_actual_directory_read_requires_an_explicit_exact_decision() {
    let mut f = Fixture::new(GOOD);
    let directory = f.ar.join("Independent");
    fs::create_dir(&directory).unwrap();
    let admitted =
        f.a.prepare_directory_admission(&f.ar, &directory, &crate::fresh())
            .unwrap();
    f.a.execute_operation(&admitted.preview.operation_id, &admitted.preview.plan_id)
        .unwrap();
    let object = ObjectKey::Directory(admitted.target_ref.rsplit('/').next().unwrap().into());
    f.transfer(&BTreeSet::from([object.clone()]));
    f.apply(BTreeSet::from([object.clone()]));
    let task =
        f.a.open(f.ar.join("Task").join("Task.typ"), Some(f.ar.clone()))
            .unwrap();
    let task_id = weftext_engine::inspect_metadata(&task.source).id.unwrap();
    f.a.save(SaveRequest {
        session_id: task.session_id,
        base_id: task.base_id,
        operation_id: crate::fresh(),
        source: task.source + "\n#read(\"../Independent/weftext.directory.json\")",
    })
    .unwrap();
    f.transfer(&BTreeSet::from([ObjectKey::Node(task_id.clone())]));
    f.apply(BTreeSet::from([ObjectKey::Node(task_id)]));
    for (core, root, branch) in [
        (&mut f.a, &f.ar, "AuthorBranch"),
        (&mut f.b, &f.br, "ReceiverBranch"),
    ] {
        for (source, name) in [("Independent", branch), (branch, "Independent")] {
            let prepared = core
                .prepare_operation(OperationRequest {
                    operation_id: crate::fresh(),
                    root: display(root),
                    intent: OperationIntent::RelocateDirectory {
                        source: display(&root.join(source)),
                        parent: display(root),
                        name: name.into(),
                    },
                })
                .unwrap();
            core.execute_operation(&prepared.operation_id, &prepared.plan_id)
                .unwrap();
        }
        core.publish_sync(root).unwrap();
    }
    f.transfer(&BTreeSet::from([object.clone()]));
    let candidate =
        f.b.sync_revalidation_cut(&f.br, &BTreeSet::from([f.note.clone()]))
            .unwrap()
            .objects
            .into_iter()
            .find(|entry| entry.object == object)
            .unwrap();
    assert_eq!(candidate.view.versions.len(), 2);
    assert_eq!(
        candidate.view.versions[0].state,
        candidate.view.versions[1].state
    );
    let revisions = candidate
        .view
        .versions
        .iter()
        .map(|version| version.revision.clone())
        .collect::<BTreeSet<_>>();
    let original = capture_optional(&f.br.join("Independent").join("weftext.directory.json"))
        .unwrap()
        .unwrap();
    let operations = f.b.operations.len();
    let captured = f.capture_review();
    let outputs = compile(captured.render_inputs());
    assert!(outputs.iter().any(|output| output.ok
        && output.dependencies.iter().any(|dependency| dependency.path
            == "Independent/weftext.directory.json"
            && dependency.status == "loaded")));
    match f.b.advance_sync_native_review(captured, outputs) {
        Err(error) => assert_eq!(error.code, "syncConflict"),
        Ok(_) => {
            panic!("An actual read cannot choose one of the received directory heads automatically")
        }
    }
    assert_eq!(f.b.operations.len(), operations);
    let chosen = &candidate.view.versions[0];
    let choices = vec![
        f.choice(),
        SyncRevalidation {
            object: object.clone(),
            revisions: revisions.clone(),
            decision: SyncRevalidationDecision::Preserve {
                revision: chosen.revision.clone(),
            },
            state: chosen.state.clone(),
            reviewed_dependencies: vec![],
        },
    ];
    let operation_id = crate::fresh();
    let preparation =
        f.b.capture_sync_revalidation_review_for_operation(
            &f.br,
            &f.identity,
            &choices,
            &BTreeMap::new(),
            Some(&operation_id),
        )
        .unwrap();
    let SyncReviewPreparation::Native(captured) = preparation else {
        panic!("The explicit directory choice still requires real native invocation");
    };
    let outputs = compile(captured.render_inputs());
    let review = f.b.finish_sync_native_review(captured, outputs).unwrap();
    assert_eq!(review.operation_id.as_deref(), Some(operation_id.as_str()));
    let note = review
        .resolutions
        .iter()
        .find(|item| item.object == f.note)
        .unwrap();
    assert!(
        note.reviewed_dependencies
            .iter()
            .any(|dependency| dependency.object == object && dependency.revisions == revisions)
    );
    assert_eq!(f.b.operations.len(), operations);
    let request = OperationRequest {
        operation_id,
        root: display(&f.br),
        intent: OperationIntent::RevalidateSync {
            identity: f.identity.clone(),
            expected_installed: review.expected_installed,
            reviewed_versions: review.reviewed_versions,
            resolutions: review.resolutions,
            blobs: BTreeMap::new(),
        },
    };
    let preview = f.prepare_native(request);
    f.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let now = capture_optional(&f.br.join("Independent").join("weftext.directory.json"))
        .unwrap()
        .unwrap();
    assert_eq!(now.bytes, original.bytes);
    assert_eq!(now.stamp.identity, original.stamp.identity);
    let publication = f.b.operations[&preview.operation_id]
        .sync_publication
        .as_ref()
        .unwrap();
    let decision = publication
        .manifest
        .changes
        .iter()
        .find(|change| change.object == object)
        .unwrap();
    assert_eq!(decision.before, revisions);
    assert_eq!(decision.after, chosen.state);
    assert_eq!(
        f.b.sync_binding_info(&f.br).unwrap().pending_publications,
        1
    );
    f.b.publish_sync(&f.br).unwrap();
    assert_eq!(
        f.b.sync_binding_info(&f.br).unwrap().pending_publications,
        0
    );
    assert_eq!(
        f.b.read_sync_history(&f.identity)
            .unwrap()
            .objects()
            .unwrap()[&object]
            .versions
            .len(),
        1
    );
}

#[test]
fn typed_native_adoption_uses_original_installed_cut_and_no_write_plan_across_remote_receive_and_restart()
 {
    let mut f = Fixture::new(GOOD);
    f.install_definition();
    f.b.publish_sync(&f.br).unwrap();
    let remote_note =
        f.a.open(f.ar.join("Note").join("Note.typ"), Some(f.ar.clone()))
            .unwrap();
    f.a.save(SaveRequest {
        session_id: remote_note.session_id,
        base_id: remote_note.base_id,
        operation_id: crate::fresh(),
        source: remote_note.source + "\nremote uninstalled branch",
    })
    .unwrap();
    let style = f.a.read_library_style(&f.ar).unwrap();
    let prepared =
        f.a.prepare_library_style(&f.ar, &style.base_id, &style.source, true, &crate::fresh())
            .unwrap();
    f.a.execute_operation(&prepared.operation_id, &prepared.plan_id)
        .unwrap();
    f.transfer(&BTreeSet::from([f.note.clone(), ObjectKey::Settings]));
    let received =
        f.b.read_sync_history(&f.identity)
            .unwrap()
            .objects()
            .unwrap()[&f.note]
            .versions
            .iter()
            .map(|version| version.revision.clone())
            .collect::<BTreeSet<_>>();
    assert_eq!(received.len(), 2);
    assert!(!f.br.join(".weftext-settings.json").exists());
    let path = f.br.join("Note").join("Note.typ");
    let original = capture_optional(&path).unwrap().unwrap();
    let external = String::from_utf8(original.bytes).unwrap()
        + "\n#undefined-display-only()\nexternal exact body";
    fs::write(&path, &external).unwrap();
    let task_path = f.br.join("Task").join("Task.typ");
    let original_task = capture_optional(&task_path).unwrap().unwrap();
    let external_task =
        String::from_utf8(original_task.bytes).unwrap() + "\n#for n in (3,) [外部新增循环#n]";
    fs::write(&task_path, &external_task).unwrap();
    let request = f.adoption_request();
    let captured = f.capture(request.clone());
    let outputs = compile(captured.render_inputs());
    assert!(
        outputs.iter().any(
            |output| output.products.as_ref().is_some_and(|products| products
                .body
                .as_ref()
                .is_some_and(|body| body.contains("外部新增循环3")))
        )
    );
    let preview =
        f.b.finish_sync_native_preparation(captured, outputs)
            .unwrap();
    assert_eq!(preview.kind, "adoptSyncExternal");
    assert_eq!(fs::read(&path).unwrap(), external.as_bytes());
    assert_eq!(
        capture_optional(&path).unwrap().unwrap().stamp.identity,
        original.stamp.identity
    );
    let plan = &f.b.operations[&preview.operation_id];
    assert!(
        plan.sync_application
            .as_ref()
            .unwrap()
            .semantic_evidence
            .as_ref()
            .unwrap()
            .absent_objects
            .contains(&ObjectKey::Settings)
    );
    assert!(
        revalidation_evidence::original_outputs(plan.transaction.as_ref().unwrap())
            .iter()
            .all(|output| output.before.as_ref() == Some(&output.after))
    );
    drop(f.b);
    f.b = Core::new(&f.bd).unwrap();
    assert_eq!(
        f.b.prepare_operation(request.clone()).unwrap().plan_id,
        preview.plan_id
    );
    f.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    f.b.publish_sync(&f.br).unwrap();
    assert_eq!(f.b.prepare_operation(request).unwrap().status, "committed");
    assert!(!f.br.join(".weftext-settings.json").exists());
    let now =
        f.b.read_sync_history(&f.identity)
            .unwrap()
            .objects()
            .unwrap()[&f.note]
            .versions
            .iter()
            .map(|version| version.revision.clone())
            .collect::<BTreeSet<_>>();
    assert_eq!(now.len(), 2);
    assert_eq!(now.intersection(&received).count(), 1);
    assert_eq!(fs::read(path).unwrap(), external.as_bytes());
    assert_eq!(fs::read(&task_path).unwrap(), external_task.as_bytes());
    assert_eq!(
        capture_optional(&task_path)
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        original_task.stamp.identity
    );
}

#[test]
fn typed_native_adoption_rejects_a_real_changed_query_invocation_and_preserves_external_bytes() {
    let mut f = Fixture::new(GOOD);
    f.install_definition();
    let path = f.br.join("Note").join("Note.typ");
    let external = fs::read_to_string(&path)
        .unwrap()
        .replace("date('2026-01-01')", "date('9999-12-31')");
    fs::write(&path, &external).unwrap();
    let request = f.adoption_request();
    let operation = request.operation_id.clone();
    let captured = f.capture(request);
    let outputs = compile(captured.render_inputs());
    assert_eq!(
        f.b.finish_sync_native_preparation(captured, outputs)
            .unwrap_err()
            .code,
        "queryInvocationUnavailable"
    );
    assert!(!f.b.operations.contains_key(&operation));
    assert_eq!(fs::read(path).unwrap(), external.as_bytes());
}

#[test]
fn typed_native_receiver_removed_member_world_requires_the_original_retirement_plan() {
    for consumed in [false, true] {
        let mut f = Fixture::new(GOOD);
        let a_task = f.ar.join("Task").join("Task.typ");
        let imported =
            f.a.prepare_import_resource(
                &f.ar,
                &a_task,
                "removed.typ",
                "original retained Resource body".as_bytes().to_vec(),
                &crate::fresh(),
            )
            .unwrap();
        f.a.execute_operation(&imported.operation_id, &imported.plan_id)
            .unwrap();
        if consumed {
            let task = f.a.open(&a_task, Some(f.ar.clone())).unwrap();
            f.a.save(SaveRequest {
                session_id: task.session_id,
                base_id: task.base_id,
                operation_id: crate::fresh(),
                source: task.source + "\n#include \"removed.typ\"",
            })
            .unwrap();
        }
        let task = ObjectKey::Node(
            weftext_engine::inspect_metadata(&fs::read_to_string(&a_task).unwrap())
                .id
                .unwrap(),
        );
        f.transfer(&BTreeSet::from([task.clone()]));
        f.apply(BTreeSet::from([task.clone()]));
        f.install_definition();
        f.b.publish_sync(&f.br).unwrap();
        let b_asset = f.br.join("Task").join("removed.typ");
        let original = capture_optional(&b_asset).unwrap().unwrap();
        let a_asset = f.ar.join("Task").join("removed.typ");
        let a_retained = f
            ._temp
            .path()
            .join("sender-original-outside-author-root.typ");
        fs::rename(&a_asset, &a_retained).unwrap();
        let registry = f.ar.join("Task").join("weftext.resources.json");
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(&registry).unwrap()).unwrap();
        document["resources"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["path"] != "removed.typ");
        fs::write(&registry, serde_json::to_vec(&document).unwrap()).unwrap();
        let available =
            f.a.read_sync_history(&f.identity)
                .unwrap()
                .objects()
                .unwrap();
        let keys = [task.clone(), ObjectKey::Schema, ObjectKey::Rules]
            .into_iter()
            .filter(|object| available.contains_key(object))
            .collect();
        let cut = f.a.read_sync_adoption_cut(&f.ar, &keys).unwrap();
        assert_eq!(cut.removed_members.len(), 1);
        let adopted =
            f.a.prepare_operation(OperationRequest {
                operation_id: crate::fresh(),
                root: display(&f.ar),
                intent: OperationIntent::AdoptSyncExternal {
                    identity: cut.identity,
                    expected_installed: cut.expected_installed,
                    changes: cut.changes,
                    removed_members: cut.removed_members,
                },
            })
            .unwrap();
        f.a.execute_operation(&adopted.operation_id, &adopted.plan_id)
            .unwrap();
        f.transfer(&BTreeSet::from([task.clone()]));
        let graph = f.b.read_sync_history(&f.identity).unwrap();
        let views = graph.objects().unwrap();
        let [version] = views[&task].versions.as_slice() else {
            panic!("One original sender removal has one successor")
        };
        let choices = vec![
            f.choice(),
            SyncRevalidation {
                object: task.clone(),
                revisions: BTreeSet::from([version.revision.clone()]),
                decision: SyncRevalidationDecision::Preserve {
                    revision: version.revision.clone(),
                },
                state: version.state.clone(),
                reviewed_dependencies: vec![],
            },
        ];
        let operation_id = crate::fresh();
        let reviewed =
            f.b.capture_sync_revalidation_review_for_operation(
                &f.br,
                &f.identity,
                &choices,
                &BTreeMap::new(),
                Some(&operation_id),
            )
            .unwrap();
        let SyncReviewPreparation::Native(captured) = reviewed else {
            panic!("Actual body Provider requires the real compiler")
        };
        assert!(
            !captured
                .render_inputs()
                .files
                .contains_key("Task/removed.typ")
        );
        assert_eq!(
            capture_optional(&b_asset).unwrap().unwrap().stamp,
            original.stamp
        );
        let outputs = compile(captured.render_inputs());
        if consumed {
            assert_eq!(
                f.b.finish_sync_native_review(captured, outputs)
                    .unwrap_err()
                    .code,
                "queryInvocationUnavailable"
            );
            assert!(!f.b.operations.contains_key(&operation_id));
            assert_eq!(
                capture_optional(&b_asset).unwrap().unwrap().stamp,
                original.stamp
            );
            continue;
        }
        let review = f.b.finish_sync_native_review(captured, outputs).unwrap();
        let request = OperationRequest {
            operation_id: operation_id.clone(),
            root: display(&f.br),
            intent: OperationIntent::RevalidateSync {
                identity: f.identity.clone(),
                expected_installed: review.expected_installed,
                reviewed_versions: review.reviewed_versions,
                resolutions: review.resolutions,
                blobs: BTreeMap::new(),
            },
        };
        let captured = f.capture(request.clone());
        assert!(
            !captured
                .render_inputs()
                .files
                .contains_key("Task/removed.typ")
        );
        let transaction = captured.plan.transaction.as_ref().unwrap();
        let [retired] = transaction.retired_members() else {
            panic!("Only the original Compound Plan can retain this resource")
        };
        assert_eq!(retired.object, task);
        assert_eq!(retired.identity, original.stamp.identity);
        assert!(
            transaction
                .files
                .iter()
                .any(|file| file.path == display(&b_asset) && file.stamp == original.stamp)
        );
        let target = PathBuf::from(&retired.target);
        let outputs = compile(captured.render_inputs());
        let preview =
            f.b.finish_sync_native_preparation(captured, outputs)
                .unwrap();
        assert_eq!(
            capture_optional(&b_asset).unwrap().unwrap().stamp,
            original.stamp
        );
        assert!(!target.exists());
        drop(f.b);
        f.b = Core::new(&f.bd).unwrap();
        assert_eq!(
            f.b.prepare_operation(request).unwrap().plan_id,
            preview.plan_id
        );
        f.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert!(!b_asset.exists());
        let retained = capture_optional(&target).unwrap().unwrap();
        assert_eq!(retained.stamp.identity, original.stamp.identity);
        assert_eq!(retained.bytes, original.bytes);
        assert_eq!(fs::read(a_retained).unwrap(), original.bytes);
        let local =
            f.b.sync_local_state(f.b.sync_setup_for(&f.br).unwrap())
                .unwrap()
                .0;
        assert!(!local[&task].state.files.contains_key("removed.typ"));
        assert!(
            !local[&task]
                .state
                .file_revisions
                .contains_key("removed.typ")
        );
        assert_eq!(
            f.b.read_sync_history(&f.identity)
                .unwrap()
                .blob(&hash(&original.bytes))
                .unwrap(),
            original.bytes
        );
    }
}

#[test]
fn typed_native_adoption_removed_member_uses_actual_after_world_and_original_negative_basis() {
    for consumed in [false, true] {
        let mut f = Fixture::new(GOOD);
        let task_path = f.br.join("Task").join("Task.typ");
        let imported =
            f.b.prepare_import_resource(
                &f.br,
                &task_path,
                "removed.typ",
                "真实原资源正文".as_bytes().to_vec(),
                &crate::fresh(),
            )
            .unwrap();
        f.b.execute_operation(&imported.operation_id, &imported.plan_id)
            .unwrap();
        if consumed {
            let task = f.b.open(&task_path, Some(f.br.clone())).unwrap();
            f.b.save(SaveRequest {
                session_id: task.session_id,
                base_id: task.base_id,
                operation_id: crate::fresh(),
                source: task.source + "\n#include \"removed.typ\"",
            })
            .unwrap();
        }
        f.b.publish_sync(&f.br).unwrap();
        f.install_definition();
        let asset = f.br.join("Task").join("removed.typ");
        let original = capture_optional(&asset).unwrap().unwrap();
        let retained = f._temp.path().join("retained-external-resource.typ");
        fs::rename(&asset, &retained).unwrap();
        let manifest_path = f.br.join("Task").join("weftext.resources.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["resources"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["path"] != "removed.typ");
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let note_path = f.br.join("Note").join("Note.typ");
        let external_note = fs::read_to_string(&note_path).unwrap()
            + "\nexternal adoption still validates this exact original definition";
        fs::write(&note_path, &external_note).unwrap();
        let request = f.adoption_request();
        let captured = f.capture(request.clone());
        assert!(
            !captured
                .render_inputs()
                .files
                .contains_key("Task/removed.typ")
        );
        let outputs = compile(captured.render_inputs());
        if consumed {
            assert_eq!(
                f.b.finish_sync_native_preparation(captured, outputs)
                    .unwrap_err()
                    .code,
                "queryInvocationUnavailable"
            );
            assert!(!f.b.operations.contains_key(&request.operation_id));
        } else {
            let preview =
                f.b.finish_sync_native_preparation(captured, outputs)
                    .unwrap();
            assert!(
                f.b.operations[&preview.operation_id]
                    .transaction
                    .as_ref()
                    .unwrap()
                    .absent_dependencies
                    .iter()
                    .any(|negative| same(Path::new(&negative.path), &asset))
            );
            drop(f.b);
            f.b = Core::new(&f.bd).unwrap();
            f.b.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
            f.b.publish_sync(&f.br).unwrap();
            assert_eq!(f.b.prepare_operation(request).unwrap().status, "committed");
        }
        assert!(!asset.exists());
        assert_eq!(fs::read(&retained).unwrap(), original.bytes);
        assert_eq!(
            capture_optional(&retained).unwrap().unwrap().stamp.identity,
            original.stamp.identity
        );
        assert_eq!(fs::read(&note_path).unwrap(), external_note.as_bytes());
    }
}

#[test]
fn typed_native_style_uses_real_admitted_roles_and_rechecks_resource_and_settings() {
    for mutation in ["none", "module", "settings"] {
        let mut f = Fixture::new(GOOD);
        f.install_style();
        let request = f.request();
        let captured = f.capture(request.clone());
        let inputs = captured.render_inputs();
        assert!(inputs.style.is_some());
        let outputs = compile(inputs);
        assert!(outputs.iter().any(|output| {
            output.ok
                && output.dependencies.iter().any(|dependency| {
                    dependency.path == "Styles/library-style.typ" && dependency.status == "loaded"
                })
        }));
        if mutation == "none" {
            let preview =
                f.b.finish_sync_native_preparation(captured, outputs)
                    .unwrap();
            f.b.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
            assert_eq!(
                f.b.operation_details(&preview.operation_id).unwrap().status,
                "committed"
            );
            continue;
        }
        if mutation == "module" {
            let path = f.br.join("Styles").join("library-style.typ");
            let bytes = fs::read(&path).unwrap();
            fs::rename(&path, f._temp.path().join("retained-original-style.typ")).unwrap();
            fs::write(path, bytes).unwrap();
        } else {
            let path = f.br.join(".weftext-settings.json");
            let mut settings: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            settings["defaultStyle"]["enabled"] = false.into();
            fs::write(path, serde_json::to_vec(&settings).unwrap()).unwrap();
        }
        assert_eq!(
            f.b.finish_sync_native_preparation(captured, outputs)
                .unwrap_err()
                .code,
            "staleBase"
        );
        assert!(!f.b.operations.contains_key(&request.operation_id));
    }
    let mut f = Fixture::new(GOOD);
    let directory = f.br.join("Styles");
    fs::create_dir(&directory).unwrap();
    let owner = crate::fresh();
    let resource = crate::fresh();
    fs::write(
        directory.join("Styles.typ"),
        format!("#metadata((id:\"{owner}\")) <weftext>\n"),
    )
    .unwrap();
    fs::write(
        directory.join("library-style.typ"),
        "#let library-style(body) = body",
    )
    .unwrap();
    fs::write(directory.join("weftext.resources.json"), serde_json::to_vec(&serde_json::json!({"version":1,"resources":[{"resourceId":resource,"path":"library-style.typ"}]})).unwrap()).unwrap();
    fs::write(f.br.join(".weftext-settings.json"), serde_json::to_vec(&serde_json::json!({"version":1,"defaultStyle":{"enabled":true,"resource":format!("weftext-resource://{}/{}", f.identity.workspace_id, resource),"entryFunction":"library-style"}})).unwrap()).unwrap();
    let choice = f.choice();
    match f
        .b
        .capture_sync_revalidation_review(&f.br, &f.identity, &[choice], &BTreeMap::new())
    {
        Err(error) => assert_eq!(error.code, "syncDependencyRequired"),
        Ok(_) => panic!("Raw style marker strings cannot grant admitted compiler roles"),
    }
}

#[test]
fn typed_native_moved_include_keeps_original_file_identity_through_real_plan_and_recovery() {
    for replace_after_effect in [false, true] {
        let mut f = Fixture::new(&(GOOD.to_owned() + "\n#include \"part.typ\""));
        let source = f.ar.join("Task").join("Task.typ");
        let task_id = weftext_engine::inspect_metadata(&fs::read_to_string(&source).unwrap())
            .id
            .unwrap();
        let task = ObjectKey::Node(task_id);
        let imported =
            f.a.prepare_import_resource(
                &f.ar,
                &source,
                "part.typ",
                "真实被移动的输入".as_bytes().to_vec(),
                &crate::fresh(),
            )
            .unwrap();
        f.a.execute_operation(&imported.operation_id, &imported.plan_id)
            .unwrap();
        f.transfer(&BTreeSet::from([task.clone()]));
        f.apply(BTreeSet::from([task.clone()]));
        let original_path = f.br.join("Task").join("part.typ");
        let original = capture_optional(&original_path).unwrap().unwrap();
        let renamed =
            f.a.prepare_operation(OperationRequest {
                operation_id: crate::fresh(),
                root: display(&f.ar),
                intent: OperationIntent::RenameNode {
                    source: display(&f.ar.join("Task")),
                    name: "Renamed".into(),
                },
            })
            .unwrap();
        f.a.execute_operation(&renamed.operation_id, &renamed.plan_id)
            .unwrap();
        f.transfer(&BTreeSet::from([task]));
        let request = f.request();
        let captured = f.capture(request);
        let inputs = captured.render_inputs();
        assert!(
            inputs
                .notes
                .iter()
                .any(|(_, path)| same(path, &f.br.join("Renamed").join("Renamed.typ")))
        );
        let outputs = compile(inputs);
        assert!(outputs.iter().any(|output| {
            output.ok
                && output.dependencies.iter().any(|dependency| {
                    dependency.path == "Renamed/part.typ" && dependency.status == "loaded"
                })
        }));
        let preview =
            f.b.finish_sync_native_preparation(captured, outputs)
                .unwrap();
        assert!(
            f.b.operations[&preview.operation_id]
                .transaction
                .as_ref()
                .unwrap()
                .files
                .iter()
                .any(|file| same(Path::new(&file.path), &original_path)
                    && file.stamp.identity == original.stamp.identity)
        );
        f.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        let moved = f.br.join("Renamed").join("part.typ");
        assert_eq!(
            capture_optional(&moved).unwrap().unwrap().stamp.identity,
            original.stamp.identity
        );
        assert!(!f.br.join("Task").exists());
        fs::rename(
            f.b.journal_path(&preview.operation_id, "commit"),
            f._temp.path().join("retained-original-decision.json"),
        )
        .unwrap();
        if replace_after_effect {
            fs::rename(&moved, f._temp.path().join("retained-original-part.typ")).unwrap();
            fs::write(&moved, &original.bytes).unwrap();
        }
        drop(f.b);
        f.b = Core::new(&f.bd).unwrap();
        let recovery = f.b.inspect_recovery(&preview.operation_id).unwrap();
        if replace_after_effect {
            assert!(
                recovery.allowed_actions.is_empty(),
                "{}",
                serde_json::to_string(&recovery).unwrap()
            );
            assert!(
                f.b.resolve_recovery(&preview.operation_id, "adoptAfterImages")
                    .is_err()
            );
            assert_ne!(
                capture_optional(&moved).unwrap().unwrap().stamp.identity,
                original.stamp.identity
            );
        } else {
            assert_eq!(
                recovery.coherence,
                "knownAfter",
                "{}",
                serde_json::to_string(&recovery).unwrap()
            );
            assert_eq!(
                f.b.resolve_recovery(&preview.operation_id, "adoptAfterImages")
                    .unwrap()
                    .status,
                "committed"
            );
            assert_eq!(
                capture_optional(&moved).unwrap().unwrap().stamp.identity,
                original.stamp.identity
            );
        }
        assert_eq!(fs::read(moved).unwrap(), original.bytes);
    }
}
