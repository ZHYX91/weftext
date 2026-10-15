//! Pure graph fixtures: these tests intentionally allocate no physical author
//! facts. A successful semantic check must still return writer obligations.
use super::*;
use crate::fresh;

struct Fixture {
    workspace: String,
    replica: String,
    graph: ReplicaGraph,
    head: String,
}
impl Fixture {
    fn new() -> Self {
        let workspace = fresh();
        let replica = fresh();
        let genesis = Manifest {
            version: 1,
            workspace_id: workspace.clone(),
            replica_id: replica.clone(),
            operation_id: fresh(),
            parents: BTreeSet::new(),
            changes: vec![],
            dependencies: vec![],
        };
        let head = genesis.id().unwrap();
        let mut graph = ReplicaGraph::new(&workspace, &head).unwrap();
        assert_eq!(
            graph.offer_manifest(&head, genesis).unwrap(),
            Admission::Accepted
        );
        Self {
            workspace,
            replica,
            graph,
            head,
        }
    }
    fn state(&mut self, name: &str, path: &str, bytes: &[u8]) -> ObjectState {
        let digest = hash(bytes);
        self.graph.offer_blob(&digest, bytes.to_vec()).unwrap();
        ObjectState {
            parent: None,
            name: name.into(),
            retired: false,
            files: BTreeMap::from([(path.into(), digest)]),
            file_revisions: BTreeMap::from([(path.into(), fresh())]),
        }
    }
    fn node(&mut self, id: &str, name: &str, tail: &str) -> ObjectState {
        let source = format!(
            "#metadata((id: \"{id}\", types: (\"task\",), status: \"open\")) <weftext>\n{tail}"
        );
        self.state(name, &format!("{name}.typ"), source.as_bytes())
    }
    fn schema(&mut self, schema: &Schema) -> ObjectState {
        self.state(
            "Schema",
            ".weftext-schema.json",
            &serde_json::to_vec(schema).unwrap(),
        )
    }
    fn change(object: ObjectKey, state: ObjectState, before: &[String]) -> Change {
        Change {
            object,
            before: before.iter().cloned().collect(),
            revision: fresh(),
            after: state,
        }
    }
    fn offer(
        &mut self,
        parents: &[String],
        changes: Vec<Change>,
        dependencies: Vec<Dependency>,
    ) -> String {
        let manifest = Manifest {
            version: 1,
            workspace_id: self.workspace.clone(),
            replica_id: self.replica.clone(),
            operation_id: fresh(),
            parents: parents.iter().cloned().collect(),
            changes,
            dependencies,
        };
        let id = manifest.id().unwrap();
        assert_eq!(
            self.graph.offer_manifest(&id, manifest).unwrap(),
            Admission::Accepted
        );
        self.head = id.clone();
        id
    }
    fn chosen(&self) -> BTreeMap<ObjectKey, Version> {
        self.graph
            .objects()
            .unwrap()
            .into_iter()
            .filter_map(|(k, v)| (v.versions.len() == 1).then(|| (k, v.versions[0].clone())))
            .collect()
    }
    fn reviewed(
        &self,
        object: &ObjectKey,
        state: ObjectState,
        dependencies: &[ObjectKey],
    ) -> SyncRevalidation {
        let current = self.graph.objects().unwrap();
        SyncRevalidation {
            object: object.clone(),
            revisions: revisions(&current[object]),
            decision: current[object]
                .versions
                .iter()
                .find(|v| v.state == state)
                .map(|v| SyncRevalidationDecision::Preserve {
                    revision: v.revision.clone(),
                })
                .unwrap_or_else(|| SyncRevalidationDecision::Repair {
                    base_revision: current[object].versions[0].revision.clone(),
                }),
            state,
            reviewed_dependencies: dependencies
                .iter()
                .map(|k| Dependency {
                    object: k.clone(),
                    revisions: revisions(&current[k]),
                })
                .collect(),
        }
    }
    fn definition(&self, key: &str, schema_revision: &str, select: &str, advanced: &str) -> String {
        format!(
            "#metadata((kind: \"weftext.query\", key: \"{key}\", binding: (schema: \"weftext-config://{}/schema\", revision: \"{schema_revision}\", language: \"weftext.query.cel.v1\"), {select}{advanced})) <weftext-query-test>",
            self.workspace
        )
    }
}

#[test]
fn single_head_changed_schema_dependency_is_reviewed_without_bool_approval() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let schema = f.schema(&Schema::bundled().unwrap());
    let source = f.node(id, "Note", "original");
    let born_schema = Fixture::change(ObjectKey::Schema, schema, &[]);
    let born_source = Fixture::change(node.clone(), source, &[]);
    let schema_revision = born_schema.revision.clone();
    let source_revision = born_source.revision.clone();
    let initial = f.offer(&[f.head.clone()], vec![born_schema, born_source], vec![]);
    let newer = f.node(id, "Note", "head with old dependency");
    let incoming = Fixture::change(node.clone(), newer, &[source_revision]);
    let incoming_id = f.offer(
        &[initial],
        vec![incoming],
        vec![Dependency {
            object: ObjectKey::Schema,
            revisions: BTreeSet::from([schema_revision.clone()]),
        }],
    );
    let mut altered = Schema::bundled().unwrap();
    altered
        .aliases
        .insert("extra-task-alias".into(), "tasks/task".into());
    let state = f.schema(&altered);
    f.offer(
        std::slice::from_ref(&incoming_id),
        vec![Fixture::change(
            ObjectKey::Schema,
            state,
            &[schema_revision],
        )],
        vec![],
    );
    assert_eq!(
        f.graph.unsettled_dependencies(&incoming_id).unwrap(),
        vec![ObjectKey::Schema]
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]);
    assert_eq!(review.revisions.len(), 1);
    let checked = validate_revalidation(
        &f.graph,
        &chosen,
        &BTreeMap::new(),
        std::slice::from_ref(&review),
    )
    .unwrap();
    assert_eq!(checked.states[&node].state, chosen[&node].state);
    assert!(!checked.required_evidence.is_empty()); // Still no author qualification.
    let mut omitted = review;
    omitted.reviewed_dependencies.clear();
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[omitted])
            .err()
            .unwrap()
            .code,
        "syncDependencyRequired"
    );
}

#[test]
fn concurrent_complete_set_is_required_and_new_branch_invalidates_original_choice() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let first = f.node(id, "Note", "common");
    let birth = Fixture::change(node.clone(), first, &[]);
    let prior = birth.revision.clone();
    let initial = f.offer(&[f.head.clone()], vec![birth], vec![]);
    let a = f.node(id, "Note", "left original");
    let ca = Fixture::change(node.clone(), a, std::slice::from_ref(&prior));
    let ar = ca.revision.clone();
    f.offer(std::slice::from_ref(&initial), vec![ca], vec![]);
    let b = f.node(id, "Note", "right original");
    let cb = Fixture::change(node.clone(), b, std::slice::from_ref(&prior));
    f.offer(std::slice::from_ref(&initial), vec![cb], vec![]);
    let current = f.graph.objects().unwrap();
    let selected = current[&node]
        .versions
        .iter()
        .find(|v| v.revision == ar)
        .unwrap()
        .clone();
    let chosen = BTreeMap::from([(node.clone(), selected.clone())]);
    let review = f.reviewed(&node, selected.state.clone(), &[]);
    assert_eq!(review.revisions.len(), 2);
    let checked = validate_revalidation(
        &f.graph,
        &chosen,
        &BTreeMap::new(),
        std::slice::from_ref(&review),
    )
    .unwrap();
    assert_eq!(checked.states[&node].state, selected.state);
    let mut incomplete = review.clone();
    incomplete.revisions = BTreeSet::from([ar]);
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[incomplete])
            .err()
            .unwrap()
            .code,
        "syncConflictChanged"
    );
    let third = f.node(id, "Note", "late third");
    f.offer(
        &[initial],
        vec![Fixture::change(node.clone(), third, &[prior])],
        vec![],
    );
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "syncConflictChanged"
    );
}

#[test]
fn opaque_authored_values_and_unresolved_ordinary_links_are_kept_exactly() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let source = format!(
        "#metadata((id: \"{id}\", types: (\"task\",), status: \"open\", opaque: (possible_ref: \"weftext-node://{}/{}\"), opaque_code: preserved_expression())) <weftext>\n#link(\"missing.typ\")[not an asserted typed target]",
        f.workspace,
        fresh()
    );
    let state = f.state("Note", "Note.typ", source.as_bytes());
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(node.clone(), state, &[])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[]);
    let checked = validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review]).unwrap();
    assert_eq!(checked.states[&node].state.files, chosen[&node].state.files);
    assert!(checked.dependency_reads[&node].is_empty());
    assert!(checked.required_evidence.iter().any(|e| matches!(
        e,
        RequiredSemanticEvidence::AbsentConfiguration {
            object: ObjectKey::Schema
        }
    )));
    let unknown = fresh();
    let broken = format!(
        "#metadata((id: \"{id}\", types: (\"task\",), status: \"open\", project: \"weftext-node://{}/{unknown}\")) <weftext>",
        f.workspace
    );
    let state = f.state("Note", "Note.typ", broken.as_bytes());
    let prior = chosen[&node].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(node.clone(), state, &[prior])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[]);
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "syncDependencyRequired"
    );
}

#[test]
fn work_citation_checks_real_type_and_publication_not_uri_looking_body() {
    let mut f = Fixture::new();
    let doc = ObjectKey::Node(fresh());
    let work = ObjectKey::Node(fresh());
    let ObjectKey::Node(doc_id) = &doc else {
        unreachable!()
    };
    let ObjectKey::Node(work_id) = &work else {
        unreachable!()
    };
    let citation = format!(
        "#metadata((kind: \"weftext.citations\", entries: ((key: \"actual\", work: \"weftext-node://{}/{work_id}\"),))) <weftext-citations>",
        f.workspace
    );
    let source = f.node(doc_id, "Doc", &citation);
    let publication = format!(
        "#metadata((id: \"{work_id}\", types: (\"paper\",), publication: (kind: \"article\", title: \"Actual Publication\", contributors: ((agent: (text: \"Author\"), roles: (\"author\",)),)))) <weftext>\n#text(\"PRIVATE BODY\")"
    );
    let target = f.state("Work", "Work.typ", publication.as_bytes());
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(doc.clone(), source, &[]),
            Fixture::change(work.clone(), target, &[]),
        ],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(
        &doc,
        chosen[&doc].state.clone(),
        std::slice::from_ref(&work),
    );
    let checked = validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review]).unwrap();
    assert!(checked.dependencies.iter().any(|d| d.object == work));
    let wrong = f.node(work_id, "Work", "not a Work");
    let prior = chosen[&work].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(work.clone(), wrong, &[prior])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&doc, chosen[&doc].state.clone(), &[work]);
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "citationWorkType"
    );
}

#[test]
fn saved_required_parameters_are_typed_but_not_fabricated_as_runtime_values() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let schema = f.schema(&Schema::bundled().unwrap());
    let baseline = schema.file_revisions[".weftext-schema.json"].clone();
    let key = fresh();
    let definition=f.definition(&key,&baseline,"select: ((name: \"n\", expr: \"threshold\"),), ","advanced: (parameters: ((name: \"threshold\", shape: (kind: \"scalar\", semantic: \"int\"), nullable: false),),)");
    let source = f.node(id, "Note", &definition);
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(ObjectKey::Schema, schema, &[]),
            Fixture::change(node.clone(), source, &[]),
        ],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]);
    let checked = validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review]).unwrap();
    assert_eq!(checked.deferred_queries.len(), 1);
    assert_eq!(
        checked.deferred_queries[0].required_parameters,
        vec!["threshold"]
    );
    assert!(
        checked
            .required_evidence
            .iter()
            .any(|e| matches!(e,RequiredSemanticEvidence::Definition{key:k,..} if k==&key))
    );
    let mut changed = Schema::bundled().unwrap();
    changed
        .fields
        .iter_mut()
        .find(|f| f.id == "tasks/priority")
        .unwrap()
        .nullable = true;
    let state = f.schema(&changed);
    let prior = chosen[&ObjectKey::Schema].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(ObjectKey::Schema, state, &[prior])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]);
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "schemaBasisChanged"
    );
}

#[test]
fn repaired_schema_cannot_rebind_an_accepted_baseline_file_revision() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let mut original = Schema::bundled().unwrap();
    original
        .fields
        .iter_mut()
        .find(|field| field.id == "tasks/status")
        .unwrap()
        .nullable = false;
    let schema = f.schema(&original);
    let baseline = schema.file_revisions[".weftext-schema.json"].clone();
    let definition = f.definition(
        &fresh(),
        &baseline,
        "select: ((name: \"status\", expr: \"status\"),), ",
        "",
    );
    let source = f.node(id, "Note", &definition);
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(ObjectKey::Schema, schema, &[]),
            Fixture::change(node.clone(), source, &[]),
        ],
        vec![],
    );

    let mut proposed = original.clone();
    proposed
        .fields
        .iter_mut()
        .find(|field| field.id == "tasks/status")
        .unwrap()
        .nullable = true;
    let mut state = f.schema(&proposed);
    let fresh_revision = state.file_revisions[".weftext-schema.json"].clone();
    state
        .file_revisions
        .insert(".weftext-schema.json".into(), baseline.clone());
    let mut chosen = f.chosen();
    chosen.get_mut(&ObjectKey::Schema).unwrap().state = state.clone();
    let review = |f: &Fixture, chosen: &BTreeMap<ObjectKey, Version>| {
        vec![
            f.reviewed(
                &ObjectKey::Schema,
                chosen[&ObjectKey::Schema].state.clone(),
                &[],
            ),
            f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]),
        ]
    };
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &review(&f, &chosen))
            .err()
            .unwrap()
            .code,
        "invalidSync"
    );

    // A genuine new file revision does not become the old binding's Schema.
    // The retained original baseline still rejects its nullable change.
    state
        .file_revisions
        .insert(".weftext-schema.json".into(), fresh_revision);
    chosen.get_mut(&ObjectKey::Schema).unwrap().state = state;
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &review(&f, &chosen))
            .err()
            .unwrap()
            .code,
        "schemaBasisChanged"
    );

    // An independently revised compatible Schema is a legitimate proposal.
    let mut compatible = original;
    compatible
        .aliases
        .insert("reviewed-task-alias".into(), "tasks/task".into());
    chosen.get_mut(&ObjectKey::Schema).unwrap().state = f.schema(&compatible);
    let checked =
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &review(&f, &chosen)).unwrap();
    assert!(
        checked
            .historical_inputs
            .iter()
            .any(|input| { input.object == ObjectKey::Schema && input.file_revision == baseline })
    );
}

#[test]
fn proposed_file_revision_controls_retain_original_distinct_owner_binding() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let state = f.node(id, "Note", "original");
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(node.clone(), state, &[])],
        vec![],
    );
    let chosen = f.chosen();
    let accepted_revision = chosen[&node].state.file_revisions["Note.typ"].clone();
    let digest = chosen[&node].state.files["Note.typ"].clone();
    let other = ObjectKey::Node(fresh());
    let mut moved_owner = chosen.clone();
    moved_owner.insert(other, chosen[&node].clone());
    assert_eq!(
        validate_proposed_file_revisions(&f.graph, &moved_owner)
            .unwrap_err()
            .code,
        "invalidSync"
    );

    let mut malformed = chosen.clone();
    malformed
        .get_mut(&node)
        .unwrap()
        .state
        .file_revisions
        .clear();
    assert_eq!(
        validate_proposed_file_revisions(&f.graph, &malformed)
            .unwrap_err()
            .code,
        "invalidSync"
    );
    malformed = chosen.clone();
    malformed
        .get_mut(&node)
        .unwrap()
        .state
        .file_revisions
        .insert("Note.typ".into(), "not-a-revision".into());
    assert_eq!(
        validate_proposed_file_revisions(&f.graph, &malformed)
            .unwrap_err()
            .code,
        "invalidSync"
    );
    malformed = chosen.clone();
    let state = &mut malformed.get_mut(&node).unwrap().state;
    state.files.insert("duplicate.bin".into(), digest);
    state
        .file_revisions
        .insert("duplicate.bin".into(), accepted_revision);
    assert_eq!(
        validate_proposed_file_revisions(&f.graph, &malformed)
            .unwrap_err()
            .code,
        "invalidSync"
    );

    // Moving the same owner's immutable bytes retains their file revision.
    let mut renamed = chosen;
    let state = &mut renamed.get_mut(&node).unwrap().state;
    state.name = "Renamed".into();
    let digest = state.files.remove("Note.typ").unwrap();
    let revision = state.file_revisions.remove("Note.typ").unwrap();
    state.files.insert("Renamed.typ".into(), digest);
    state.file_revisions.insert("Renamed.typ".into(), revision);
    validate_proposed_file_revisions(&f.graph, &renamed).unwrap();
}

#[test]
fn pinned_callee_uses_exact_file_revision_and_never_falls_back_to_current() {
    let mut f = Fixture::new();
    let caller = ObjectKey::Node(fresh());
    let callee = ObjectKey::Node(fresh());
    let ObjectKey::Node(caller_id) = &caller else {
        unreachable!()
    };
    let ObjectKey::Node(callee_id) = &callee else {
        unreachable!()
    };
    let schema = f.schema(&Schema::bundled().unwrap());
    let baseline = schema.file_revisions[".weftext-schema.json"].clone();
    let key = fresh();
    let definition = f.definition(
        &key,
        &baseline,
        "select: ((name: \"value\", expr: \"status\"),), ",
        "",
    );
    let state = f.node(callee_id, "Callee", &definition);
    let pinned = state.file_revisions["Callee.typ"].clone();
    let original = Fixture::change(callee.clone(), state, &[]);
    let original_revision = original.revision.clone();
    let initial = f.offer(
        &[f.head.clone()],
        vec![Fixture::change(ObjectKey::Schema, schema, &[]), original],
        vec![],
    );
    let replacement = f.definition(
        &key,
        &baseline,
        "select: ((name: \"value\", expr: \"2\"),), ",
        "",
    );
    let state = f.node(callee_id, "Callee", &replacement);
    f.offer(
        &[initial],
        vec![Fixture::change(callee.clone(), state, &[original_revision])],
        vec![],
    );
    // Equal projections remain different exact portable Schema baselines.
    let schema = f.schema(&Schema::bundled().unwrap());
    let root_baseline = schema.file_revisions[".weftext-schema.json"].clone();
    let schema_before = f.chosen()[&ObjectKey::Schema].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(ObjectKey::Schema, schema, &[schema_before])],
        vec![],
    );
    let root_key = fresh();
    let advanced = format!(
        "advanced: (parameters: ((name: \"spare\", shape: (kind: \"scalar\", semantic: \"int\"), nullable: false),), call: (owner: \"weftext-node://{}/{callee_id}\", selection: (mode: \"pinned\", key: \"{key}\", revision: \"{pinned}\"), arguments: (:)))",
        f.workspace
    );
    let root = f.definition(&root_key, &root_baseline, "", &advanced);
    let state = f.node(caller_id, "Caller", &root);
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(caller.clone(), state, &[])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(
        &caller,
        chosen[&caller].state.clone(),
        &[ObjectKey::Schema, callee.clone()],
    );
    let checked = validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review]).unwrap();
    let callees = checked
        .invocations
        .iter()
        .filter(|invocation| invocation.definition.is_some())
        .collect::<Vec<_>>();
    let [callee_invocation] = callees.as_slice() else {
        panic!("The exact pinned callee must have one independent descriptor");
    };
    assert_eq!(callee_invocation.schema_baseline.revision, baseline);
    assert!(callee_invocation.context.parameters.is_empty());
    assert_eq!(callee_invocation.plan.select.len(), 1);
    assert_eq!(callee_invocation.plan.select[0].name, "value");
    assert_eq!(callee_invocation.plan.select[0].expr, "status");
    let exact_callee = callee_invocation.definition.as_ref().unwrap();
    assert_eq!(
        exact_callee.owner,
        format!("weftext-node://{}/{callee_id}", f.workspace)
    );
    assert_eq!(exact_callee.source_revision, pinned);
    assert_eq!(exact_callee.key, key);
    assert!(matches!(&exact_callee.selection,
        weftext_data::DefinitionSelection::Pinned { key: selected_key, revision }
        if selected_key == &key && revision == &pinned));

    let outers = checked
        .invocations
        .iter()
        .filter(|invocation| invocation.definition.is_none())
        .collect::<Vec<_>>();
    let [outer] = outers.as_slice() else {
        panic!("The closed outer terminal must retain one exact producer projection");
    };
    assert_eq!(outer.owner.node_id, *caller_id);
    assert_eq!(outer.key, root_key);
    assert_eq!(
        outer.source_revision,
        chosen[&caller].state.file_revisions["Caller.typ"]
    );
    assert_eq!(outer.schema_baseline.revision, root_baseline);
    assert_ne!(
        outer.schema_baseline.revision,
        callee_invocation.schema_baseline.revision
    );
    assert!(outer.context.parameters.is_empty());
    assert_eq!(outer.plan.select.len(), 1);
    assert_eq!(outer.plan.select[0].name, "value");
    assert_eq!(outer.plan.select[0].expr, "value");
    let outer_advanced = outer.plan.advanced.as_ref().unwrap();
    assert!(outer_advanced.parameters.is_empty());
    let outer_call = outer_advanced.call.as_ref().unwrap();
    assert_eq!(outer_call.owner, exact_callee.owner);
    assert!(outer_call.arguments.is_empty());
    assert!(matches!(&outer_call.selection,
        weftext_data::DefinitionSelection::Pinned { key: selected_key, revision }
        if selected_key == &key && revision == &pinned));
    // This is an evidence projection only: the authored required parameter
    // and its original source/file revision remain in the chosen version.
    assert_eq!(checked.invocations.len(), callees.len() + outers.len());
    assert_eq!(
        checked.deferred_queries[0].required_parameters,
        vec!["spare"]
    );
    assert_eq!(
        checked.provider_dependencies[0].root_baseline.revision,
        root_baseline
    );
    assert!(
        checked
            .historical_inputs
            .iter()
            .any(|i| i.object == callee && i.file_revision == pinned)
    );
    let invalid = root.replace(&pinned, &fresh());
    let state = f.node(caller_id, "Caller", &invalid);
    let prior = chosen[&caller].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(caller.clone(), state, &[prior])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(
        &caller,
        chosen[&caller].state.clone(),
        &[ObjectKey::Schema, callee.clone()],
    );
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "syncSemanticUnavailable"
    );
}

#[test]
fn schema_binding_cannot_substitute_object_revision_for_file_revision() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let schema = f.schema(&Schema::bundled().unwrap());
    let schema_change = Fixture::change(ObjectKey::Schema, schema, &[]);
    let definition = f.definition(
        &fresh(),
        &schema_change.revision,
        "select: ((name: \"n\", expr: \"1\"),), ",
        "",
    );
    let source = f.node(id, "Note", &definition);
    f.offer(
        &[f.head.clone()],
        vec![schema_change, Fixture::change(node.clone(), source, &[])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]);
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "syncSemanticUnavailable"
    );
}

#[test]
fn cut_reader_returns_exact_symbols_without_current_author_or_blob_claims() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let state = f.node(id, "Note", "rawbody");
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(node.clone(), state, &[])],
        vec![],
    );
    let cut = revalidation_cut(&f.graph, &BTreeMap::new(), &BTreeSet::from([node])).unwrap();
    assert_eq!(cut.coverage, "complete");
    assert!(cut.expected_installed.is_empty());
    assert_eq!(cut.objects.len(), 1);
    assert_eq!(cut.files.len(), 1);
    assert_ne!(cut.files[0].file_revision, cut.files[0].object_revision);
    let serialized = serde_json::to_string(&cut).unwrap();
    assert!(!serialized.contains("rawbody"));
    assert!(!serialized.contains("canOpen"));
}

#[test]
fn closed_actual_call_cannot_defer_an_invalid_literal_parameter() {
    let mut f = Fixture::new();
    let caller = ObjectKey::Node(fresh());
    let callee = ObjectKey::Node(fresh());
    let ObjectKey::Node(caller_id) = &caller else {
        unreachable!()
    };
    let ObjectKey::Node(callee_id) = &callee else {
        unreachable!()
    };
    let schema = f.schema(&Schema::bundled().unwrap());
    let baseline = schema.file_revisions[".weftext-schema.json"].clone();
    let key = fresh();
    let definition = f.definition(&key, &baseline, "select: ((name: \"wanted\", expr: \"wanted\"),), ",
        "advanced: (parameters: ((name: \"wanted\", shape: (kind: \"enum\", values: (\"open\",), open: false), nullable: false),),)");
    let source = f.node(callee_id, "Callee", &definition);
    let root_key = fresh();
    let expression = weftext_data::typst_string("\"open\"");
    let advanced = format!(
        "advanced: (call: (owner: \"weftext-node://{}/{callee_id}\", selection: (mode: \"current\", key: \"{key}\"), arguments: (wanted: {expression})))",
        f.workspace
    );
    let root = f.definition(&root_key, &baseline, "", &advanced);
    let current = f.node(caller_id, "Caller", &root);
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(ObjectKey::Schema, schema, &[]),
            Fixture::change(callee.clone(), source, &[]),
            Fixture::change(caller.clone(), current, &[]),
        ],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(
        &caller,
        chosen[&caller].state.clone(),
        &[ObjectKey::Schema, callee.clone()],
    );
    let checked = validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review]).unwrap();
    assert!(checked.deferred_queries.is_empty());
    assert_eq!(checked.provider_dependencies.len(), 1);
    let wrong = root.replace(&expression, &weftext_data::typst_string("\"closed\""));
    let state = f.node(caller_id, "Caller", &wrong);
    let prior = chosen[&caller].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(caller.clone(), state, &[prior])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(
        &caller,
        chosen[&caller].state.clone(),
        &[ObjectKey::Schema, callee.clone()],
    );
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "queryInvocationUnavailable"
    );

    // An unrelated required root parameter cannot turn this fixed bad call
    // into a proved invocation. Preserve its parameterized declaration, but
    // force the original writer to discharge the call's actual evidence.
    let with_spare = wrong.replace(
        "advanced: (call:",
        "advanced: (parameters: ((name: \"spare\", shape: (kind: \"scalar\", semantic: \"int\"), nullable: false),), call:",
    );
    let state = f.node(caller_id, "Caller", &with_spare);
    let prior = chosen[&caller].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(caller.clone(), state, &[prior])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(
        &caller,
        chosen[&caller].state.clone(),
        &[ObjectKey::Schema, callee.clone()],
    );
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "queryInvocationUnavailable"
    );

    // A valid fixed call remains valid with the unrelated required parameter;
    // the parameter declaration survives, and the original evaluator still
    // executes the fixed call using its exact literal argument.
    let legal = with_spare.replace(&weftext_data::typst_string("\"closed\""), &expression);
    let state = f.node(caller_id, "Caller", &legal);
    let prior = chosen[&caller].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(caller.clone(), state, &[prior])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(
        &caller,
        chosen[&caller].state.clone(),
        &[ObjectKey::Schema, callee],
    );
    let checked = validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review]).unwrap();
    assert_eq!(
        checked.deferred_queries[0].required_parameters,
        vec!["spare"]
    );
    assert_eq!(checked.provider_dependencies.len(), 1);
    assert!(
        !checked
            .required_evidence
            .iter()
            .any(|evidence| matches!(evidence, RequiredSemanticEvidence::QueryInvocation { .. }))
    );
}

#[test]
fn review_derives_actual_dependencies_without_preparing_or_allocating() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let schema = f.schema(&Schema::bundled().unwrap());
    let state = f.node(
        id,
        "Note",
        "native unresolved weftext-node://unresolved is ordinary body",
    );
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(ObjectKey::Schema, schema, &[]),
            Fixture::change(node.clone(), state, &[]),
        ],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[]);
    let selected = chosen.keys().cloned().collect();
    let original_revisions = revisions(&f.graph.objects().unwrap()[&node]);
    let output = review_revalidation(
        f.graph,
        &BTreeMap::new(),
        &selected,
        &[review],
        &BTreeMap::new(),
        &BTreeSet::new(),
    )
    .unwrap();
    assert!(output.expected_installed.is_empty());
    assert_eq!(output.resolutions[0].revisions, original_revisions);
    assert_eq!(
        output.resolutions[0]
            .reviewed_dependencies
            .iter()
            .map(|d| &d.object)
            .collect::<Vec<_>>(),
        vec![&ObjectKey::Schema]
    );
    assert!(serde_json::to_value(&output).is_ok());
}

#[test]
fn one_head_revalidation_prepares_one_new_causal_change_and_retains_file_revisions() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let state = f.node(id, "Note", "original");
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(node.clone(), state, &[])],
        vec![],
    );
    let chosen = f.chosen();
    let original = chosen[&node].clone();
    let review = f.reviewed(&node, original.state.clone(), &[]);
    let checked = validate_revalidation(
        &f.graph,
        &chosen,
        &BTreeMap::new(),
        std::slice::from_ref(&review),
    )
    .unwrap();
    assert_eq!(checked.states[&node], original);
    assert!(!checked.required_evidence.is_empty());
    let reviewed_versions = received_cut(&f.graph, &BTreeSet::from([node.clone()])).unwrap();
    let prepared = prepare_revalidation(
        f.graph,
        &BTreeMap::new(),
        &BTreeSet::from([node.clone()]),
        &[review],
        &BTreeMap::new(),
        &f.replica,
        &fresh(),
        &BTreeSet::new(),
        &[],
        &reviewed_versions,
    )
    .unwrap();
    assert_eq!(prepared.manifest.changes.len(), 1);
    assert_eq!(
        prepared.manifest.changes[0].before,
        BTreeSet::from([original.revision.clone()])
    );
    assert_ne!(prepared.states[&node].revision, original.revision);
    assert_eq!(prepared.states[&node].state, original.state);
    assert_eq!(
        prepared
            .graph
            .status(&prepared.manifest.id().unwrap())
            .unwrap(),
        Admission::Accepted
    );
    assert!(prepared.publication_blobs.is_empty());
}

#[test]
fn preserve_cannot_edit_bytes_and_repair_is_an_explicit_new_file_revision() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let state = f.node(id, "Note", "original");
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(node.clone(), state, &[])],
        vec![],
    );
    let mut chosen = f.chosen();
    let mut review = f.reviewed(&node, chosen[&node].state.clone(), &[]);
    let repaired = f.node(id, "Note", "explicit repair");
    review.state = repaired.clone();
    chosen.get_mut(&node).unwrap().state = repaired;
    assert_eq!(
        validate_revalidation(
            &f.graph,
            &chosen,
            &BTreeMap::new(),
            std::slice::from_ref(&review)
        )
        .err()
        .unwrap()
        .code,
        "syncConflictChanged"
    );
    review.decision = SyncRevalidationDecision::Repair {
        base_revision: chosen[&node].revision.clone(),
    };
    let reviewed_versions = received_cut(&f.graph, &BTreeSet::from([node.clone()])).unwrap();
    let prepared = prepare_revalidation(
        f.graph,
        &BTreeMap::new(),
        &BTreeSet::from([node.clone()]),
        &[review],
        &BTreeMap::new(),
        &f.replica,
        &fresh(),
        &BTreeSet::new(),
        &[],
        &reviewed_versions,
    )
    .unwrap();
    assert_eq!(prepared.manifest.changes[0].after, chosen[&node].state);
}

#[test]
fn review_installed_cut_cannot_be_rebased_at_prepare() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let state = f.node(id, "Note", "original");
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(node.clone(), state, &[])],
        vec![],
    );
    let chosen = f.chosen();
    let original = &chosen[&node];
    let local = BTreeMap::from([(
        node.clone(),
        LocalObject {
            object: node.clone(),
            directory: "original".into(),
            commit: original.commit.clone(),
            revision: original.revision.clone(),
            state: original.state.clone(),
        },
    )]);
    let review = f.reviewed(&node, original.state.clone(), &[]);
    assert_eq!(
        prepare_revalidation(
            f.graph,
            &local,
            &BTreeSet::from([node]),
            &[review],
            &BTreeMap::new(),
            &f.replica,
            &fresh(),
            &BTreeSet::new(),
            &[],
            &[]
        )
        .err()
        .unwrap()
        .code,
        "syncBasisChanged"
    );
}

#[test]
fn newly_received_transaction_mate_cannot_upgrade_an_already_reviewed_cut() {
    let mut f = Fixture::new();
    let target = ObjectKey::Node(fresh());
    let mate = ObjectKey::Node(fresh());
    let ObjectKey::Node(target_id) = &target else {
        unreachable!()
    };
    let ObjectKey::Node(mate_id) = &mate else {
        unreachable!()
    };
    let target_state = f.node(target_id, "Target", "unchanged chosen target");
    let mate_state = f.node(mate_id, "Mate", "original transaction mate");
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(target.clone(), target_state, &[]),
            Fixture::change(mate.clone(), mate_state, &[]),
        ],
        vec![],
    );
    let chosen = f.chosen();
    let selected = BTreeSet::from([target.clone(), mate.clone()]);
    let reviewed_versions = received_cut(&f.graph, &selected).unwrap();
    let review = f.reviewed(&target, chosen[&target].state.clone(), &[]);
    let newer = f.node(mate_id, "Mate", "late mate update");
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(
            mate.clone(),
            newer,
            &[chosen[&mate].revision.clone()],
        )],
        vec![],
    );
    assert_eq!(
        prepare_revalidation(
            f.graph,
            &BTreeMap::new(),
            &selected,
            &[review],
            &BTreeMap::new(),
            &f.replica,
            &fresh(),
            &BTreeSet::new(),
            &[],
            &reviewed_versions
        )
        .err()
        .unwrap()
        .code,
        "syncConflictChanged"
    );
}

#[test]
fn review_discovers_an_independent_directory_scope_from_its_exact_received_role() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let directory_id = fresh();
    let directory = ObjectKey::Directory(directory_id.clone());
    let marker = serde_json::to_vec(&serde_json::json!({"directoryId":directory_id})).unwrap();
    let directory_state = f.state("Folder", "weftext.directory.json", &marker);
    let schema = f.schema(&Schema::bundled().unwrap());
    let baseline = schema.file_revisions[".weftext-schema.json"].clone();
    let definition = f.definition(&fresh(), &baseline,
        &format!("scope: (directory: \"weftext-directory://{}/{directory_id}\", descendants: true, lifecycle: \"live\"), select: ((name: \"n\", expr: \"1\"),), ", f.workspace), "");
    let state = f.node(id, "Note", &definition);
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(ObjectKey::Schema, schema, &[]),
            Fixture::change(directory.clone(), directory_state, &[]),
            Fixture::change(node.clone(), state, &[]),
        ],
        vec![],
    );
    let received = f.chosen();
    let review = f.reviewed(&node, received[&node].state.clone(), &[]);
    let chosen = BTreeMap::from([(node.clone(), received[&node].clone())]);
    let checked = inspect_revalidation(
        &f.graph,
        &chosen,
        &BTreeMap::new(),
        &[review],
        &BTreeSet::new(),
    )
    .unwrap();
    assert!(
        checked.dependency_reads[&node]
            .iter()
            .any(|dependency| dependency.object == directory)
    );
    assert!(
        !checked
            .states
            .contains_key(&ObjectKey::RootDirectory(directory_id))
    );
}

#[test]
fn required_used_parameter_does_not_hide_closed_provider_failure_or_block_legal_definition() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let schema = f.schema(&Schema::bundled().unwrap());
    let baseline = schema.file_revisions[".weftext-schema.json"].clone();
    let select = format!(
        "select: ((name: \"closed\", expr: {}), (name: \"unknown\", expr: \"spare\")), ",
        weftext_data::typst_string("nextCivilDay(date('9999-12-31'))")
    );
    let source = f.definition(&fresh(), &baseline, &select, "advanced: (parameters: ((name: \"spare\", shape: (kind: \"scalar\", semantic: \"int\"), nullable: false),),)");
    let state = f.node(id, "Note", &source);
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(ObjectKey::Schema, schema, &[]),
            Fixture::change(node.clone(), state, &[]),
        ],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]);
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "queryInvocationUnavailable"
    );
    let legal = source.replace("9999-12-31", "2026-01-01");
    let state = f.node(id, "Note", &legal);
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(
            node.clone(),
            state,
            &[chosen[&node].revision.clone()],
        )],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]);
    let checked = validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review]).unwrap();
    assert_eq!(
        checked.deferred_queries[0].required_parameters,
        vec!["spare"]
    );
    assert!(
        !checked
            .required_evidence
            .iter()
            .any(|e| matches!(e, RequiredSemanticEvidence::QueryInvocation { .. }))
    );
    assert!(
        checked
            .provider_dependencies
            .iter()
            .flat_map(|trace| &trace.baselines)
            .any(|baseline| !baseline.providers.is_empty())
    );
}

#[test]
fn exact_registered_provider_literal_preflight_runs_without_latest_fallback() {
    let mut f = Fixture::new();
    let node = ObjectKey::Node(fresh());
    let ObjectKey::Node(id) = &node else {
        unreachable!()
    };
    let schema = f.schema(&Schema::bundled().unwrap());
    let baseline = schema.file_revisions[".weftext-schema.json"].clone();
    let expression = weftext_data::typst_string("nextCivilDay(date('2026-01-01'))");
    let definition = f.definition(
        &fresh(),
        &baseline,
        &format!("select: ((name: \"value\", expr: {expression}),), "),
        "",
    );
    let source = f.node(id, "Note", &definition);
    f.offer(
        &[f.head.clone()],
        vec![
            Fixture::change(ObjectKey::Schema, schema, &[]),
            Fixture::change(node.clone(), source, &[]),
        ],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]);
    let checked = validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review]).unwrap();
    assert!(
        checked
            .provider_requirements
            .iter()
            .any(|r| r.provider_calls.contains("nextCivilDay"))
    );
    assert!(
        checked
            .provider_dependencies
            .iter()
            .flat_map(|d| &d.baselines)
            .any(|b| !b.providers.is_empty())
    );
    let missing = definition.replace(&baseline, &fresh());
    let state = f.node(id, "Note", &missing);
    let prior = chosen[&node].revision.clone();
    f.offer(
        &[f.head.clone()],
        vec![Fixture::change(node.clone(), state, &[prior])],
        vec![],
    );
    let chosen = f.chosen();
    let review = f.reviewed(&node, chosen[&node].state.clone(), &[ObjectKey::Schema]);
    assert_eq!(
        validate_revalidation(&f.graph, &chosen, &BTreeMap::new(), &[review])
            .err()
            .unwrap()
            .code,
        "syncSemanticUnavailable"
    );
}
