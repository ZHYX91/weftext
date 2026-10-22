//! Explicit configured scopes. Directory access never grants access to rows.
use super::*;
use weftext_core::{QueryNodeWitness, QueryScopeWitness, SnapshotWitness};
use weftext_data::{FrozenAdvancedInput, QueryPlan};

pub(super) fn live() -> String {
    "live".into()
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Directory {
    pub(super) path: String,
    pub(super) title: String,
    pub(super) reference: String,
    #[serde(default = "live")]
    pub(super) lifecycle: String,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    pub(super) directories: BTreeSet<String>,
    #[serde(default)]
    pub(super) trash: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Authority {
    pub(super) directories: BTreeMap<String, Directory>,
    pub(super) trash: bool,
    nodes: BTreeMap<String, weftext_knowledge::NodeRef>,
}
pub(super) struct Capture {
    pub(super) authority: Authority,
    lifecycles: BTreeSet<String>,
    scopes: Vec<QueryScopeWitness>,
    nodes: Vec<QueryNodeWitness>,
    coordinates: BTreeMap<String, PathBuf>,
    retained_bytes: usize,
}
fn valid_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .is_ok_and(|u| u.to_string() == value && u.get_version() == Some(uuid::Version::Random))
}
pub(super) fn valid_node(node: &weftext_knowledge::NodeRef) -> bool {
    valid_uuid(&node.workspace_id) && valid_uuid(&node.node_id)
}
pub(super) fn valid_file(file: &FileEntry) -> bool {
    if !matches!(file.lifecycle.as_str(), "live" | "trash") {
        return false;
    }
    let trash_path = std::path::Path::new(&file.path).components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(".weftext-trash")
    });
    if file.lifecycle == "trash" {
        !file.editable
            && !file.importable
            && trash_path
            && std::path::Path::new(&file.path)
                .extension()
                .is_some_and(|e| e == "typ")
            && file.expected_node.as_ref().is_some_and(valid_node)
    } else {
        !trash_path && file.expected_node.as_ref().is_none_or(valid_node)
    }
}
fn reference_kind(reference: &str) -> Option<&'static str> {
    for (prefix, kind) in [
        ("weftext-node://", "node"),
        ("weftext-directory://", "directory"),
    ] {
        if let Some((workspace, object)) = reference
            .strip_prefix(prefix)
            .and_then(|s| s.split_once('/'))
            && valid_uuid(workspace)
            && valid_uuid(object)
        {
            return Some(kind);
        }
    }
    None
}
pub(super) fn valid_directory(directory: &Directory) -> bool {
    directory.title.len() <= 500
        && reference_kind(&directory.reference).is_some()
        && matches!(directory.lifecycle.as_str(), "live" | "trash")
        && !directory.path.contains(':')
        && std::path::Path::new(&directory.path)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}
pub(super) fn validate_rights(config: &Config, rights: &Rights) -> bool {
    rights.directories.len() <= 64
        && rights
            .directories
            .iter()
            .all(|id| config.directories.contains_key(id))
}
pub(super) fn authorize(
    inner: &HostedInner,
    actor: &str,
    authority: &Authority,
) -> Result<(), ApiError> {
    let rights = &inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?
        .query_scopes;
    if authority.trash && !rights.trash {
        return Err(unavailable());
    }
    for (id, original) in &authority.directories {
        if !rights.directories.contains(id) {
            return Err(unavailable());
        }
        if inner.config.directories.get(id) != Some(original) {
            return Err(query::changed());
        }
        inner
            .core
            .qualified_query_scope_at(
                &inner.config.root,
                &inner.config.root.join(&original.path),
                &original.reference,
                &original.lifecycle,
            )
            .map_err(|_| query::changed())?;
    }
    for (resource, owner) in &authority.nodes {
        let file = inner.file(resource).ok().ok_or_else(unavailable)?;
        inner
            .core
            .qualified_query_node_at(
                &inner.config.root,
                &inner.config.root.join(&file.path),
                owner,
                &file.lifecycle,
            )
            .map_err(|_| query::changed())?;
    }
    Ok(())
}
fn select<'a>(
    inner: &'a HostedInner,
    actor: &str,
    selector: &str,
) -> Result<(&'a String, &'a Directory), ApiError> {
    let rights = &inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?
        .query_scopes;
    inner
        .config
        .directories
        .iter()
        .find(|(id, d)| {
            rights.directories.contains(*id) && (*id == selector || d.reference == selector)
        })
        .ok_or_else(unavailable)
}
pub(super) fn canonicalize(
    inner: &HostedInner,
    actor: &str,
    plan: &mut QueryPlan,
) -> Result<(), ApiError> {
    if let Some(directory) = &mut plan.scope.directory {
        *directory = select(inner, actor, directory)?.1.reference.clone();
    }
    if let Some(a) = &mut plan.advanced {
        for child in &mut a.union_all {
            canonicalize(inner, actor, child)?;
        }
        if let Some(r) = &mut a.related {
            canonicalize(inner, actor, &mut r.from)?;
        }
    }
    Ok(())
}
pub(super) fn preauthorize(
    inner: &HostedInner,
    actor: &str,
    plan: &QueryPlan,
) -> Result<(), ApiError> {
    if let Some(a) = &plan.advanced {
        if a.call.is_some() {
            return Ok(());
        }
        if !a.union_all.is_empty() {
            for p in &a.union_all {
                preauthorize(inner, actor, p)?;
            }
            return Ok(());
        }
        if let Some(r) = &a.related {
            preauthorize(inner, actor, &r.from)?;
        }
    }
    if plan.scope.lifecycle == "trash" && !inner.policy.users[actor].query_scopes.trash {
        return Err(unavailable());
    }
    if let Some(selector) = &plan.scope.directory {
        select(inner, actor, selector)?;
    }
    Ok(())
}
/// Read actual producer scopes, including selected current/pinned callees.
fn leaf_scopes(
    plan: &QueryPlan,
    context: &FrozenAdvancedInput,
) -> Result<BTreeSet<(Option<String>, String)>, ApiError> {
    fn visit(
        plan: &QueryPlan,
        context: &FrozenAdvancedInput,
        out: &mut BTreeSet<(Option<String>, String)>,
        nodes: &mut usize,
        depth: usize,
    ) -> Result<(), ApiError> {
        *nodes += 1;
        if *nodes > 1024 || depth > 64 {
            return Err(query::invalid());
        }
        if let Some(a) = &plan.advanced {
            if let Some(call) = &a.call {
                let owner = context
                    .definitions
                    .iter()
                    .find(|o| o.owner == call.owner)
                    .ok_or_else(query::changed)?;
                let d = weftext_data::select_definition(&call.selection, &owner.history)
                    .map_err(|_| query::changed())?;
                return visit(&d.plan, context, out, nodes, depth + 1);
            }
            if !a.union_all.is_empty() {
                for child in &a.union_all {
                    visit(child, context, out, nodes, depth + 1)?;
                }
                return Ok(());
            }
            if let Some(r) = &a.related {
                visit(&r.from, context, out, nodes, depth + 1)?;
            }
        }
        let lifecycle = if plan.scope.lifecycle.is_empty() {
            "live"
        } else {
            &plan.scope.lifecycle
        };
        if !matches!(lifecycle, "live" | "trash") {
            return Err(query::invalid());
        }
        out.insert((plan.scope.directory.clone(), lifecycle.into()));
        Ok(())
    }
    // Preserve the evaluator's graph validation, including cycle bounds.
    weftext_data::required_query_scopes(plan, &context.definitions)
        .map_err(|_| query::invalid())?;
    let mut out = BTreeSet::new();
    visit(plan, context, &mut out, &mut 0, 0)?;
    Ok(out)
}
pub(super) fn capture(
    inner: &HostedInner,
    actor: &str,
    plan: &QueryPlan,
    context: &mut FrozenAdvancedInput,
    initial_bytes: usize,
) -> Result<Capture, ApiError> {
    let mut captured = Capture {
        authority: Authority::default(),
        lifecycles: BTreeSet::new(),
        scopes: vec![],
        nodes: vec![],
        coordinates: BTreeMap::new(),
        retained_bytes: initial_bytes,
    };
    for (selector, lifecycle) in leaf_scopes(plan, context)? {
        captured.lifecycles.insert(lifecycle.clone());
        if lifecycle == "trash" {
            captured.authority.trash = true;
            if !inner.policy.users[actor].query_scopes.trash {
                return Err(unavailable());
            }
        }
        if let Some(selector) = selector {
            let (id, directory) = select(inner, actor, &selector)?;
            if directory.lifecycle != lifecycle {
                return Err(query::changed());
            }
            if captured.authority.directories.contains_key(id) {
                let bound = context
                    .scope_bindings
                    .get(&directory.reference)
                    .ok_or_else(query::changed)?
                    .clone();
                context.scope_bindings.insert(selector, bound);
                continue;
            }
            let qualified = inner
                .core
                .qualified_query_scope_at(
                    &inner.config.root,
                    &inner.config.root.join(&directory.path),
                    &directory.reference,
                    &lifecycle,
                )
                .map_err(|_| query::changed())?;
            query::charge_bytes(
                &mut captured.retained_bytes,
                qualified.witness.retained_bytes(),
            )?;
            if qualified.target_ref != directory.reference {
                return Err(query::changed());
            }
            let path = qualified.directory.to_string_lossy().into_owned();
            for key in [selector, directory.reference.clone()] {
                if context
                    .scope_bindings
                    .insert(key, path.clone())
                    .is_some_and(|old| old != path)
                {
                    return Err(query::changed());
                }
            }
            captured
                .authority
                .directories
                .insert(id.clone(), directory.clone());
            captured.scopes.push(qualified.witness);
        }
    }
    Ok(captured)
}
impl Capture {
    pub(super) fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    pub(super) fn includes(&self, lifecycle: &str) -> bool {
        self.lifecycles.contains(lifecycle)
    }
    pub(super) fn qualify_node(
        &mut self,
        inner: &HostedInner,
        resource: &str,
        file: &weftext_core::SnapshotFile,
        workspace: &str,
    ) -> Result<PathBuf, ApiError> {
        structure::guard_slot(inner, resource)?;
        if let Some(path) = self.coordinates.get(resource) {
            return Ok(path.clone());
        }
        let entry = &inner.file(resource)?;
        self.authority.trash |= entry.lifecycle == "trash";
        let inferred = weftext_knowledge::NodeRef {
            workspace_id: workspace.into(),
            node_id: file.node_id.clone().ok_or_else(query::changed)?,
        };
        let expected = entry.expected_node.as_ref().unwrap_or(&inferred);
        let node = inner
            .core
            .qualified_query_node_at(
                &inner.config.root,
                &inner.config.root.join(&entry.path),
                expected,
                &entry.lifecycle,
            )
            .map_err(|_| query::changed())?;
        query::charge_bytes(&mut self.retained_bytes, node.witness.retained_bytes())?;
        if node.owner != *expected
            || file.node_id.as_ref() != Some(&expected.node_id)
            || node.revision != file.revision
            || file
                .bytes
                .as_ref()
                .is_none_or(|bytes| digest(bytes) != node.source_hash)
        {
            return Err(query::changed());
        }
        self.nodes.push(node.witness);
        self.authority
            .nodes
            .insert(resource.into(), expected.clone());
        self.coordinates
            .insert(resource.into(), node.coordinate.clone());
        Ok(node.coordinate)
    }
    pub(super) fn recheck(&self, inner: &HostedInner, actor: &str) -> Result<(), ApiError> {
        authorize(inner, actor, &self.authority)?;
        for witness in &self.scopes {
            inner
                .core
                .validate_query_scope_witness(&inner.config.root, witness)
                .map_err(|_| query::changed())?;
        }
        for witness in &self.nodes {
            inner
                .core
                .validate_query_node_witness(&inner.config.root, witness)
                .map_err(|_| query::changed())?;
        }
        Ok(())
    }
    pub(super) fn guard_witnesses(&self) -> Result<Vec<SnapshotWitness>, ApiError> {
        let mut pool = vec![];
        let check = |pool: &[SnapshotWitness]| {
            let mut bytes = self.retained_bytes;
            for witness in pool {
                query::charge_bytes(&mut bytes, witness.retained_bytes())?;
            }
            Ok::<(), ApiError>(())
        };
        for witness in &self.scopes {
            Core::merge_query_scope_witness(&mut pool, witness).map_err(guard_error)?;
            check(&pool)?;
        }
        for witness in &self.nodes {
            Core::merge_query_node_witness(&mut pool, witness).map_err(guard_error)?;
            check(&pool)?;
        }
        Ok(pool)
    }
    pub(super) fn needs_rules_guard(&self) -> bool {
        self.authority.trash || !self.scopes.is_empty()
    }
}
fn guard_error(error: CoreError) -> ApiError {
    if error.code == "basisRequired" {
        ApiError::new(
            StatusCode::CONFLICT,
            "basisRequired",
            UiMessage::error_api_hosted_query_scopes_existing_rules_configuration_required(),
        )
    } else {
        query::changed()
    }
}
pub(super) fn catalog(inner: &HostedInner, actor: &str) -> Value {
    let rights = &inner.policy.users[actor].query_scopes;
    let mut partial = false;
    let mut directories = vec![];
    for id in &rights.directories {
        let Some(d) = inner.config.directories.get(id) else {
            partial = true;
            continue;
        };
        let available = (d.lifecycle != "trash" || rights.trash)
            && inner
                .core
                .qualified_query_scope_at(
                    &inner.config.root,
                    &inner.config.root.join(&d.path),
                    &d.reference,
                    &d.lifecycle,
                )
                .and_then(|q| {
                    inner
                        .core
                        .validate_query_scope_witness(&inner.config.root, &q.witness)
                })
                .is_ok();
        partial |= !available;
        directories.push(json!({"id":id,"title":d.title,"reference":d.reference,"kind":reference_kind(&d.reference),"lifecycle":d.lifecycle,"status":if available{"available"}else{"unavailable"}}));
    }
    json!({"directories":directories,"trash":rights.trash,"scopeCoverage":if partial{"partial"}else{"complete"}})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_scope_paths_and_references_are_exact_and_bounded() {
        let mut d = Directory {
            path: "".into(),
            title: "Root".into(),
            reference: format!(
                "weftext-directory://{}/{}",
                uuid::Uuid::new_v4(),
                uuid::Uuid::new_v4()
            ),
            lifecycle: "live".into(),
        };
        assert!(valid_directory(&d));
        for path in ["../Other", "C:/Other", "/Other", "Folder/../../Other"] {
            d.path = path.into();
            assert!(!valid_directory(&d));
        }
        d.path = "Folder".into();
        d.reference.push_str("/extra");
        assert!(!valid_directory(&d));
    }
    #[test]
    fn nested_producer_lifecycles_are_read_without_terminal_scope() {
        let plan: QueryPlan = serde_json::from_value(json!({"scope":{"directory":"ignored","lifecycle":"live"},"advanced":{"unionAll":[{"scope":{"directory":"a","lifecycle":"trash"}},{"scope":{"directory":"b","lifecycle":"live"},"advanced":{"related":{"relation":"parents","from":{"scope":{"directory":"c","lifecycle":"trash"}}}}}]}})).unwrap();
        let scopes = leaf_scopes(&plan, &FrozenAdvancedInput::default()).unwrap();
        assert_eq!(
            scopes,
            BTreeSet::from([
                (Some("a".into()), "trash".into()),
                (Some("b".into()), "live".into()),
                (Some("c".into()), "trash".into())
            ])
        );
    }
    #[cfg(windows)]
    fn configure_node_scope(inner: &mut HostedInner) -> weftext_knowledge::NodeRef {
        let node = inner
            .core
            .qualified_node_at(
                &inner.config.root,
                &inner.config.root.join("Shared/Shared.typ"),
            )
            .unwrap()
            .owner;
        inner.config.directories.insert(
            "shared".into(),
            Directory {
                path: "Shared".into(),
                title: "Shared".into(),
                reference: format!("weftext-node://{}/{}", node.workspace_id, node.node_id),
                lifecycle: "live".into(),
            },
        );
        let user = inner.policy.users.get_mut("alice").unwrap();
        let rights = user.grants.get_mut("Shared").unwrap();
        rights.read = false;
        rights.write = false;
        rights.history = false;
        rights.query.products.insert("self".into(), true);
        node
    }
    #[cfg(windows)]
    fn request(plan: Value, resources: Value) -> query::QueryRequest {
        serde_json::from_value(json!({"resources":resources,"plan":plan})).unwrap()
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn empty_scope_hidden_and_missing_have_same_refusal_and_no_result() {
        let (_temp, state, auth, _, _, _) = history_api::test_fixture();
        configure_node_scope(&mut state.inner.lock().unwrap());
        let mut refusals = vec![];
        for directory in ["shared", "unknown"] {
            let error = query::execute(
                &state,
                &auth,
                request(
                    json!({"scope":{"directory":directory},"select":[{"name":"one","expr":"1"}]}),
                    json!([]),
                ),
            )
            .await
            .unwrap_err();
            refusals.push((
                error.status,
                error.code,
                error.explanation,
                error.diagnostics,
                error.causes,
            ));
        }
        assert_eq!(refusals[0], refusals[1]);
        assert_eq!(refusals[0].0, StatusCode::NOT_FOUND);
        assert!(fields::query_catalog(&state.inner.lock().unwrap(), "alice").is_empty());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn directory_capability_filters_real_rows_without_source_read_and_preserves_case() {
        let (_temp, state, auth, _, _, _) = history_api::test_fixture();
        {
            let mut inner = state.inner.lock().unwrap();
            configure_node_scope(&mut inner);
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .query_scopes
                .directories
                .insert("shared".into());
            inner.config.directories.get_mut("shared").unwrap().path = "sHaReD".into();
            assert!(inner.rights("alice", "Shared").is_err());
            assert_eq!(catalog(&inner, "alice")["scopeCoverage"], "complete");
        }
        let output=query::execute(&state,&auth,request(json!({"scope":{"directory":"shared","descendants":false},"select":[{"name":"one","expr":"1"}]}),json!(["Shared"]))).await.unwrap();
        assert_eq!(output["result"]["rows"].as_array().unwrap().len(), 1);
        let row = &output["result"]["rows"][0];
        assert_eq!(row["entityKey"], "Shared");
        assert_eq!(row["path"], "resource:Shared");
        assert_eq!(row["canOpen"], false);
        let mut inner = state.inner.lock().unwrap();
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .query_scopes
            .directories
            .clear();
        assert!(
            fields::frozen_query(&inner, "alice", output["queryId"].as_str().unwrap()).is_err()
        );
    }
    #[cfg(windows)]
    #[test]
    fn opaque_scope_bytes_share_the_input_cap_and_repeated_aliases_do_not_expand_it() {
        let (_temp, state, _, _, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        configure_node_scope(&mut inner);
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .query_scopes
            .directories
            .insert("shared".into());
        let d = &inner.config.directories["shared"];
        let bytes = inner
            .core
            .qualified_query_scope_at(
                &inner.config.root,
                &inner.config.root.join(&d.path),
                &d.reference,
                "live",
            )
            .unwrap()
            .witness
            .retained_bytes();
        assert!(bytes > 0);
        let plan:QueryPlan=serde_json::from_value(json!({"advanced":{"unionAll":[{"scope":{"directory":"shared"}},{"scope":{"directory":d.reference}}]}})).unwrap();
        let mut context = FrozenAdvancedInput::default();
        let captured = capture(
            &inner,
            "alice",
            &plan,
            &mut context,
            32 * 1024 * 1024 - bytes,
        )
        .unwrap();
        assert_eq!(captured.retained_bytes(), 32 * 1024 * 1024);
        let error = capture(
            &inner,
            "alice",
            &plan,
            &mut FrozenAdvancedInput::default(),
            32 * 1024 * 1024 - bytes + 1,
        )
        .err()
        .unwrap();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, "queryBudget");
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn trash_rows_require_lifecycle_capability_and_never_open_source_or_write() {
        let (_temp, state, auth, _, _, _) = history_api::test_fixture();
        {
            let mut inner = state.inner.lock().unwrap();
            let node = configure_node_scope(&mut inner);
            let operation = uuid::Uuid::new_v4().to_string();
            let root = inner.config.root.clone();
            let plan = inner
                .core
                .prepare_operation(weftext_core::OperationRequest {
                    operation_id: operation.clone(),
                    root: root.to_string_lossy().into_owned(),
                    intent: weftext_core::OperationIntent::TrashNode {
                        source: root.join("Shared").to_string_lossy().into_owned(),
                    },
                })
                .unwrap();
            let receipt = inner
                .core
                .execute_operation(&operation, &plan.plan_id)
                .unwrap();
            let file = inner.config.files.get_mut("Shared").unwrap();
            file.path = PathBuf::from(receipt.path)
                .join("Shared.typ")
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            file.lifecycle = "trash".into();
            file.expected_node = Some(node);
            file.editable = false;
            assert!(valid_file(file));
        }
        let plan = json!({"scope":{"lifecycle":"trash"},"select":[{"name":"one","expr":"1"}]});
        assert_eq!(
            query::execute(&state, &auth, request(plan.clone(), json!([])))
                .await
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        let empty = query::execute(
            &state,
            &auth,
            request(json!({"select":[{"name":"one","expr":"1"}]}), Value::Null),
        )
        .await
        .unwrap();
        assert!(empty["result"]["rows"].as_array().unwrap().is_empty());
        let explicit = query::execute(
            &state,
            &auth,
            request(
                json!({"select":[{"name":"one","expr":"1"}]}),
                json!(["Shared"]),
            ),
        )
        .await
        .unwrap();
        assert!(explicit["result"]["rows"].as_array().unwrap().is_empty());
        assert_eq!(
            query::execute(
                &state,
                &auth,
                request(
                    json!({"select":[{"name":"one","expr":"1"}]}),
                    json!(["Shared", "hidden-or-missing"])
                )
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
        state
            .inner
            .lock()
            .unwrap()
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .query_scopes
            .trash = true;
        let output = query::execute(&state, &auth, request(plan, json!(["Shared"])))
            .await
            .unwrap();
        assert_eq!(output["result"]["rows"][0]["lifecycle"], "trash");
        assert_eq!(output["result"]["rows"][0]["readOnly"], true);
        assert_eq!(output["result"]["rows"][0]["canOpen"], false);
        let mut inner = state.inner.lock().unwrap();
        assert!(inner.open("alice", "Shared").is_err());
        let mut users = inner.policy.users.clone();
        users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap()
            .annotations
            .proofread = true;
        assert!(validate_users(&inner.config, &users).is_err());
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .query_scopes
            .trash = false;
        assert!(
            fields::frozen_query(&inner, "alice", output["queryId"].as_str().unwrap()).is_err()
        );
    }
}
