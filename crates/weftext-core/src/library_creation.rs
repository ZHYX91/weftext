//! Library initialization is one frozen create-tree decision. Language remains
//! solely in the native style resource; options are immutable operation input.
use super::*;
use operations::{FileChange, FrozenOperation, Mutation, NewBirth, read_blob, write_blob};
use std::collections::BTreeMap;
use weftext_messages::UiMessage;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryCreateOptions {
    #[serde(default)]
    pub initial_style: Option<InitialLibraryStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_root: Option<InitialRootNote>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitialRootNote {
    pub title: String,
    pub queries: Vec<String>,
}

pub const ROOT_PRESET_SCHEMA: &str =
    include_str!("../../../docs/design/examples/schema.authoring.json");

/// The preview and creation consume the same native Query plans. No sample records.
pub fn root_query_presets() -> serde_json::Value {
    serde_json::json!([
        {"id":"notes","title":"全部笔记","plan":{"scope":{"descendants":true,"lifecycle":"live"},"types":[],"select":[{"name":"title","expr":"title"}],"order":[{"expr":"title","descending":false,"nulls":"last"}],"take":200}},
        {"id":"tasks","title":"任务","plan":{"scope":{"descendants":true,"lifecycle":"live"},"types":["task"],"select":[{"name":"title","expr":"title"},{"name":"status","expr":"status"},{"name":"due","expr":"due"}],"order":[],"take":200}},
        {"id":"events","title":"事件","plan":{"scope":{"descendants":true,"lifecycle":"live"},"types":["event"],"select":[{"name":"title","expr":"title"}],"order":[],"take":200}}
    ])
}

fn validate_root(root: &InitialRootNote) -> Result<()> {
    if root.title.len() > 4096 || root.queries.len() > 3 {
        return Err(CoreError::new(
            "invalidPreset",
            UiMessage::error_core_library_creation_root_preset_exceeds_input_budget(),
        ));
    }
    let catalog = root_query_presets();
    let mut seen = std::collections::BTreeSet::new();
    for query in &root.queries {
        if !seen.insert(query)
            || !catalog
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["id"] == *query)
        {
            return Err(CoreError::new(
                "invalidPreset",
                UiMessage::error_core_library_creation_unknown_or_repeated_root_query_selection(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitialLibraryStyle {
    pub source: String,
    pub enabled: bool,
}

fn fingerprint(root: &Path, options: &LibraryCreateOptions) -> Result<String> {
    // Preserve retries of the existing no-options API and its retained plans.
    let mut input = serde_json::json!({"kind":"createLibrary","path":display(root)});
    if options.initial_style.is_some() || options.initial_root.is_some() {
        input["options"] = serde_json::to_value(options)?;
    }
    Ok(hash(&serde_json::to_vec(&input)?))
}

fn request_fingerprint(path: &Path, options: &LibraryCreateOptions) -> Result<String> {
    match absolute_path(path) {
        Ok(root) => fingerprint(&root, options),
        Err(_) => Ok(hash(&serde_json::to_vec(&serde_json::json!({
            "kind":"cancelInvalidLibraryCreation", "rawPath":display(path), "options":options
        }))?)),
    }
}

/// A complete immutable cancellation has no author participants. Its one
/// flushed Plan is the terminal fact, so a crash cannot leave a raw path as a
/// pending filesystem scope between separate plan and resolution records.
pub(crate) fn is_cancellation(plan: &journal::Plan) -> bool {
    if plan.kind != "cancelLibraryCreation"
        || plan.before.is_some()
        || plan.identity.is_some()
        || plan.receipt.is_some()
        || !plan.files.is_empty()
        || plan.node_id.is_some()
        || plan.workspace_id.is_some()
        || plan.transaction.is_some()
        || plan.history_restore.is_some()
        || plan.sync_setup.is_some()
        || plan.sync_publication.is_some()
        || plan.sync_application.is_some()
        || plan.content_revision.is_some()
        || plan.path.len() > 32_768
    {
        return false;
    }
    let Ok(options) = serde_json::from_slice::<LibraryCreateOptions>(&plan.after) else {
        return false;
    };
    if options
        .initial_style
        .as_ref()
        .is_some_and(|s| s.source.len() > MAX_SOURCE_BYTES)
    {
        return false;
    }
    if options
        .initial_root
        .as_ref()
        .is_some_and(|r| validate_root(r).is_err())
    {
        return false;
    }
    request_fingerprint(Path::new(&plan.path), &options).is_ok_and(|f| f == plan.fingerprint)
}

impl Core {
    pub fn create_library_with_options_and_id(
        &mut self,
        path: impl AsRef<Path>,
        options: LibraryCreateOptions,
        operation_id: &str,
    ) -> Result<LibraryInfo> {
        valid_operation_id(operation_id)?;
        let path = path.as_ref();
        let fingerprint = request_fingerprint(path, &options)?;
        if !self.operations.contains_key(operation_id)
            && let Some(record) = self.resolution_record(operation_id)?
        {
            if record.attempt.as_ref().is_some_and(|attempt| {
                attempt.request_kind != "createLibrary" || attempt.wire_digest != fingerprint
            }) {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_library_creation_cancelled_attempt_belongs_to_different_input(),
                ));
            }
            return Err(CoreError::new(
                "operationCanceled",
                UiMessage::error_core_library_creation_original_creation_attempt_durably_cancelled(
                ),
            ));
        }
        if let Some(plan) = self.operations.get(operation_id) {
            if plan.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_journal_operation_id_already_belongs_to_different_input(),
                ));
            }
            if !self.committed(operation_id)? && self.resolved(operation_id)? {
                return Err(CoreError::new(
                    "operationCanceled",
                    UiMessage::error_core_library_creation_original_creation_was_durably_canceled_or_retained_without_activation(),
                ));
            }
            if !self.committed(operation_id)? {
                return Err(CoreError::new(
                    "recoveryPending",
                    UiMessage::error_core_library_creation_inspect_and_resolve_original_creation_plan_no_second_plan_was_created(),
                ));
            }
            return Ok(library_info(plan));
        }
        let root = absolute_path(path)?;
        let guard = ParentGuard::acquire(&self.backend, &root)?;
        if root.try_exists()? {
            return Err(CoreError::new(
                "exists",
                UiMessage::error_core_library_creation_library_creation_requires_absent_directory(),
            ));
        }
        let plan = self.plan_library_creation(
            &root,
            options,
            operation_id,
            fingerprint,
            guard.directory_identity()?,
        )?;
        self.stage(plan.clone())?;
        self.install_record(&plan)?;
        // Initial install and explicit continuation use identical guarded proof.
        self.resolve_library_creation_recovery(&plan, "resumeOriginalPlan")?;
        Ok(library_info(&plan))
    }

    /// The original library receipt survives independent delivery failure.
    /// Both its root and optional root note must still be their original
    /// physical objects; names and copied workspace declarations are not proof.
    pub fn reopen_local_library_creation(
        &mut self,
        operation_id: &str,
    ) -> Result<(Inventory, Option<OpenedDocument>)> {
        self.sync_local_owner()?;
        valid_operation_id(operation_id)?;
        let invalid = || {
            CoreError::new(
                "committedResultUnavailable",
                UiMessage::error_core_original_scope_creation_physical_result_unavailable(),
            )
        };
        let plan = self
            .operations
            .get(operation_id)
            .cloned()
            .ok_or_else(invalid)?;
        if plan.kind != "createLibrary" || !self.committed(operation_id)? {
            return Err(invalid());
        }
        let root = PathBuf::from(&plan.path);
        if self
            .workspace(&root)?
            .as_ref()
            .map(|workspace| &workspace.workspace_id)
            != plan.workspace_id.as_ref()
            || !self.library_creation_root_matches(&plan, &root)?
        {
            return Err(invalid());
        }
        let inventory = self.inventory(&root)?;
        if inventory.workspace_id != plan.workspace_id {
            return Err(invalid());
        }
        let root_note = library_info(&plan)
            .root_note_path
            .map(|path| self.reopen_local_operation_node(operation_id, &path))
            .transpose()?;
        Ok((inventory, root_note))
    }

    pub fn local_library_creation_info(&self, operation_id: &str) -> Result<LibraryInfo> {
        self.sync_local_owner()?;
        valid_operation_id(operation_id)?;
        let plan = self.operations.get(operation_id).ok_or_else(|| {
            CoreError::new(
                "operationMissing",
                UiMessage::error_core_original_scope_creation_original_not_retained(),
            )
        })?;
        if plan.kind != "createLibrary"
            || plan.transaction.is_none()
            || !self.committed(operation_id)?
        {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_original_scope_creation_original_not_committed_creation(),
            ));
        }
        Ok(library_info(plan))
    }

    /// Fence an unknown original request without allocating another identity.
    /// No author path is created, changed or removed by cancellation.
    pub fn cancel_library_creation_with_options_and_id(
        &mut self,
        path: impl AsRef<Path>,
        options: LibraryCreateOptions,
        operation_id: &str,
    ) -> Result<RecoveryInfo> {
        valid_operation_id(operation_id)?;
        // A raw invalid path is only frozen request evidence here. It is never
        // read, qualified, published or written as an author path.
        let root = path.as_ref();
        if display(root).len() > 32_768 {
            return Err(CoreError::new(
                "tooLarge",
                UiMessage::error_core_library_creation_creation_request_path_exceeds_retained_input_budget(),
            ));
        }
        if let Some(style) = &options.initial_style {
            check_size(style.source.as_bytes())?;
        }
        if let Some(root) = &options.initial_root {
            validate_root(root)?;
        }
        let fingerprint = request_fingerprint(root, &options)?;
        if let Some(plan) = self.operations.get(operation_id).cloned() {
            if plan.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_journal_operation_id_already_belongs_to_different_input(),
                ));
            }
            let current = self.inspect_recovery(operation_id)?;
            if matches!(current.status.as_str(), "committed" | "resolved") {
                return Ok(current);
            }
            return self.resolve_recovery(operation_id, "keepCurrent");
        }
        if let Some(record) = self.resolution_record(operation_id)?
            && record.attempt.as_ref().is_some_and(|attempt| {
                attempt.request_kind != "createLibrary" || attempt.wire_digest != fingerprint
            })
        {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_library_creation_cancelled_attempt_belongs_to_different_input(
                ),
            ));
        }
        self.settle_local_attempt(operation_id, "createLibrary", &fingerprint)?;
        self.inspect_recovery(operation_id)
    }

    fn plan_library_creation(
        &self,
        root: &Path,
        options: LibraryCreateOptions,
        operation_id: &str,
        fingerprint: String,
        parent_identity: FileIdentity,
    ) -> Result<journal::Plan> {
        let workspace_id = fresh();
        let config = WorkspaceConfig {
            version: 1,
            workspace_id: workspace_id.clone(),
            storage_profile: STORAGE_PROFILE.into(),
            schema_support: "notImplemented".into(),
            sync_support: "notImplemented".into(),
        };
        let mut files = vec![
            (
                ".weftext-workspace.json".into(),
                serde_json::to_vec_pretty(&config)?,
            ),
            (".weftext-format".into(), b"weftext.typst.v1\n".to_vec()),
            (
                ".weftext-rules".into(),
                br#"{"version":1,"ignored":[],"unmanaged":[]}"#.to_vec(),
            ),
            (
                ".weftext-mvp-commit.json".into(),
                serde_json::to_vec(
                    &serde_json::json!({"workspaceId":workspace_id,"operationId":operation_id,"storageProfile":STORAGE_PROFILE}),
                )?,
            ),
        ];
        let mut directories = Vec::new();
        let mut births = Vec::new();
        let mut resource_map = BTreeMap::new();
        let mut node_map = BTreeMap::new();
        let mut fixed_revisions = BTreeMap::new();
        if let Some(initial) = &options.initial_root {
            validate_root(initial)?;
            let name = root.file_name().and_then(|s| s.to_str()).ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_library_creation_root_needs_canonical_note_filename(),
                )
            })?;
            valid_name(name)?;
            let relative = format!("{name}.typ");
            let path = display(&root.join(&relative));
            let node_id = fresh();
            let title = weftext_data::NativeValue::Text(initial.title.clone())
                .to_typst()
                .map_err(|e| CoreError::from_data("invalidPreset", e))?;
            let mut source = format!(
                "#metadata((id: \"{node_id}\", types: ())) <weftext>\n\n#set document(title: {title})\n#title()\n\n"
            );
            if !initial.queries.is_empty() {
                let schema_revision = fresh();
                fixed_revisions.insert(
                    display(&root.join(".weftext-schema.json")),
                    schema_revision.clone(),
                );
                files.push((
                    ".weftext-schema.json".into(),
                    ROOT_PRESET_SCHEMA.as_bytes().to_vec(),
                ));
                let schema = weftext_data::Schema::from_author_json(ROOT_PRESET_SCHEMA)
                    .map_err(|e| CoreError::from_data("invalidPreset", e))?;
                let binding = weftext_data::SavedQueryBinding {
                    schema: format!("weftext-config://{workspace_id}/schema"),
                    revision: schema_revision,
                    language: "weftext.query.cel.v1".into(),
                };
                source.push_str(
                    "此处由普通正文和查询组成。可随时改写、移除或重排；刷新结果不会保存笔记。\n\n",
                );
                let catalog = root_query_presets();
                for selection in &initial.queries {
                    let item = catalog
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|item| item["id"] == *selection)
                        .unwrap();
                    let plan = serde_json::from_value(item["plan"].clone())?;
                    let label = format!("weftext-query-{}", item["title"].as_str().unwrap());
                    source = weftext_data::upsert_saved_query(
                        &source,
                        None,
                        &fresh(),
                        &label,
                        &binding,
                        &plan,
                        &schema,
                    )
                    .map_err(|e| CoreError::from_data("invalidPreset", e))?
                    .source;
                }
            }
            births.push(NewBirth {
                path: path.clone(),
                node_id: node_id.clone(),
            });
            node_map.insert(path, node_id);
            files.push((relative, source.into_bytes()));
        }
        if let Some(style) = &options.initial_style {
            check_size(style.source.as_bytes())?;
            let inspection = weftext_engine::inspect_library_style(&style.source);
            if !inspection.syntax_valid
                || inspection
                    .controls
                    .iter()
                    .any(|c| c.reason.as_deref() == Some("missing-or-ambiguous-entry"))
            {
                return Err(CoreError::new(
                    "invalidStyle",
                    UiMessage::error_core_library_creation_initial_native_style_needs_valid_syntax_and_one_unambiguous_library_style_entry(),
                ));
            }
            let node_id = fresh();
            let resource_id = fresh();
            let resource_ref = format!("weftext-resource://{workspace_id}/{resource_id}");
            directories.push("Styles".into());
            let node_path = display(&root.join("Styles/Styles.typ"));
            births.push(NewBirth {
                path: node_path.clone(),
                node_id: node_id.clone(),
            });
            node_map.insert(node_path, node_id.clone());
            resource_map.insert(
                display(&root.join("Styles/library-style.typ")),
                resource_ref.clone(),
            );
            files.extend([
                ("Styles/Styles.typ".into(), format!("#metadata((id: \"{node_id}\", types: ())) <weftext>\n= 库默认样式\n").into_bytes()),
                ("Styles/library-style.typ".into(), style.source.as_bytes().to_vec()),
                ("Styles/weftext.resources.json".into(), serde_json::to_vec_pretty(&serde_json::json!({"version":1,"resources":[{"resourceId":resource_id,"path":"library-style.typ"}]}))?),
                (".weftext-settings.json".into(), serde_json::to_vec_pretty(&serde_json::json!({"version":1,"defaultStyle":{"enabled":style.enabled,"resource":resource_ref,"entryFunction":"library-style"}}))?),
            ]);
        }
        let output = files
            .iter()
            .map(|(relative, bytes): &(String, Vec<u8>)| FileChange {
                path: display(&root.join(relative)),
                base_id: None,
                before: None,
                after: bytes.clone(),
            })
            .collect::<Vec<_>>();
        let plan_id = fresh();
        let preview = OperationPreview {
            operation_id: operation_id.into(),
            plan_id: plan_id.clone(),
            kind: "createLibrary".into(),
            status: "prepared".into(),
            changes: output
                .iter()
                .map(|f| OperationChange {
                    path: f.path.clone(),
                    action: "create".into(),
                    bytes: f.after.len(),
                })
                .collect(),
            new_node_id: births.first().map(|b| b.node_id.clone()),
            node_map,
            revisions: output
                .iter()
                .map(|f| {
                    (
                        f.path.clone(),
                        fixed_revisions.get(&f.path).cloned().unwrap_or_else(fresh),
                    )
                })
                .collect(),
            resource_map,
            losses: Vec::new(),
        };
        let mut plan = journal::Plan::file(
            operation_id.into(),
            fingerprint,
            root,
            None,
            Vec::new(),
            None,
            "createLibrary",
        );
        plan.files = files;
        plan.workspace_id = Some(workspace_id);
        plan.transaction = Some(FrozenOperation {
            request: OperationRequest {
                operation_id: operation_id.into(),
                root: display(root),
                intent: OperationIntent::CreateLibrary { options },
            },
            preview,
            files: Vec::new(),
            directories: Vec::new(),
            receipt: OperationReceipt {
                operation_id: operation_id.into(),
                plan_id,
                status: "committed".into(),
                path: display(root),
                node_ids: births.iter().map(|b| b.node_id.clone()).collect(),
            },
            mutation: Mutation::CreateTree {
                target: display(root),
                existing_target: false,
                directories,
                files: output,
                births,
                updates: Vec::new(),
            },
            hosted_destination: None,
            expected_membership: None,
            expected_trash_membership: None,
            backup_private_digest: None,
            absent_dependencies: Vec::new(),
            write_parents: vec![(display(root), parent_identity)],
        });
        Ok(plan)
    }

    pub(crate) fn library_creation_root_matches(
        &self,
        plan: &journal::Plan,
        root: &Path,
    ) -> Result<bool> {
        if plan.kind != "createLibrary" || plan.transaction.is_none() {
            return Ok(true);
        }
        let expected = self.materialized_identity(&plan.operation_id, Path::new(&plan.path))?;
        let guard = ParentGuard::acquire(&self.backend, &root.join(".library-identity"))?;
        Ok(expected.as_ref() == Some(&guard.directory_identity()?))
    }

    pub(crate) fn inspect_library_creation_recovery(
        &self,
        plan: &journal::Plan,
    ) -> Result<RecoveryInfo> {
        let mut result = self.inspect_transaction_recovery(plan)?;
        if !self.committed(&plan.operation_id)?
            && !self.resolved(&plan.operation_id)?
            && self
                .backend
                .entry_exists(self.journal_path(&plan.operation_id, "install"))?
            && result.members.iter().all(|m| {
                if m.role == "write" {
                    m.matches_before || m.matches_after
                } else {
                    m.matches_after
                }
            })
        {
            result.allowed_actions.push("resumeOriginalPlan".into());
        }
        Ok(result)
    }

    pub(crate) fn resolve_library_creation_recovery(
        &mut self,
        plan: &journal::Plan,
        action: &str,
    ) -> Result<RecoveryInfo> {
        if self.committed(&plan.operation_id)? || self.resolved(&plan.operation_id)? {
            return self.inspect_library_creation_recovery(plan);
        }
        let t = plan.transaction.as_ref().ok_or_else(|| {
            CoreError::new(
                "invalidPlan",
                UiMessage::error_core_library_creation_creation_transaction_missing(),
            )
        })?;
        let Mutation::CreateTree {
            target,
            existing_target: false,
            directories,
            files,
            updates,
            ..
        } = &t.mutation
        else {
            return Err(CoreError::new(
                "invalidPlan",
                UiMessage::error_core_library_creation_creation_requires_original_create_only_tree(
                ),
            ));
        };
        if !matches!(t.request.intent, OperationIntent::CreateLibrary { .. })
            || target != &plan.path
            || t.request.root != plan.path
            || !updates.is_empty()
        {
            return Err(CoreError::new(
                "invalidPlan",
                UiMessage::error_core_library_creation_creation_plan_binding_differs(),
            ));
        }
        let mut capture =
            self.capture_transaction_recovery(plan, action == "resumeOriginalPlan")?;
        if action == "keepCurrent" && capture.coherence == "knownBefore" {
            self.resolve_record(&plan.operation_id, "keepCurrent")?;
        } else {
            let after = capture.members.iter().all(|m| m.matches_after);
            let resumable = capture.members.iter().all(|m| {
                if m.role == "write" {
                    m.matches_before || m.matches_after
                } else {
                    m.matches_after
                }
            });
            if !self
                .backend
                .entry_exists(self.journal_path(&plan.operation_id, "install"))?
                || !match action {
                    "adoptAfterImages" => after,
                    "resumeOriginalPlan" => resumable,
                    _ => false,
                }
            {
                return Err(CoreError::new(
                    "recoveryConflict",
                    UiMessage::error_core_library_creation_original_creation_members_unavailable_or_changed_no_participant_was_written(),
                ));
            }
            if action == "resumeOriginalPlan" {
                let mut planned_directories = vec![PathBuf::from(target)];
                planned_directories.extend(
                    directories
                        .iter()
                        .map(|relative| Path::new(target).join(relative)),
                );
                planned_directories.sort_by_key(|p| p.components().count());
                let mut directory_handles = Vec::new();
                for path in planned_directories {
                    let member = capture
                        .members
                        .iter()
                        .find(|m| m.kind == "directory" && m.path == display(&path))
                        .ok_or_else(|| {
                            CoreError::new(
                                "invalidPlan",
                                UiMessage::error_core_library_creation_created_directory_lacks_original_member(),
                            )
                        })?;
                    if member.matches_after {
                        continue;
                    }
                    let guard = ParentGuard::acquire(&self.backend, &path)?;
                    let directory = guard.create_directory()?;
                    self.record_materialized(&plan.operation_id, &path, &directory)?;
                    capture.guards.push(guard);
                    directory_handles.push(directory);
                }
                for change in files {
                    let index = capture
                        .members
                        .iter()
                        .position(|m| m.kind == "file" && m.path == change.path)
                        .ok_or_else(|| {
                            CoreError::new(
                                "invalidPlan",
                                UiMessage::error_core_library_creation_created_file_lacks_original_member(),
                            )
                        })?;
                    if capture.members[index].matches_after {
                        continue;
                    }
                    if let Some((_, file)) = capture.files.iter_mut().find(|(i, _)| *i == index) {
                        write_blob(file, &change.after)?;
                    } else {
                        let guard = ParentGuard::acquire(&self.backend, Path::new(&change.path))?;
                        let mut file = guard.open_file(true, true)?;
                        self.record_materialized(
                            &plan.operation_id,
                            Path::new(&change.path),
                            &file,
                        )?;
                        write_blob(&mut file, &change.after)?;
                        capture.guards.push(guard);
                        capture.files.push((index, file));
                    }
                }
                // The namespace guards retain every newly allocated directory
                // through the single durable decision below.
                capture.placement_files.extend(directory_handles);
            }
            let mut materialized = Vec::new();
            for (index, file) in &mut capture.files {
                let path = &capture.members[*index].path;
                let change = files.iter().find(|f| &f.path == path).ok_or_else(|| {
                    CoreError::new(
                        "invalidPlan",
                        UiMessage::error_core_library_creation_unexpected_creation_output(),
                    )
                })?;
                if read_blob(file)? != change.after {
                    return Err(CoreError::new(
                        "recoveryConflict",
                        UiMessage::error_core_library_creation_creation_afterimage_changed_before_decision(),
                    ));
                }
                materialized.push((path.clone(), stamp(file)?));
            }
            self.publish_recovery_facts(plan, &materialized)?;
            self.commit(plan)?;
        }
        drop(capture);
        self.inspect_library_creation_recovery(plan)
    }
}

fn library_info(plan: &journal::Plan) -> LibraryInfo {
    LibraryInfo {
        operation_id: plan.operation_id.clone(),
        root: plan.path.clone(),
        workspace_id: plan.workspace_id.clone().unwrap(),
        status: "committed".into(),
        root_note_path: plan.transaction.as_ref().and_then(|t| {
            t.births()
                .iter()
                .find(|birth| Path::new(&birth.path).parent() == Some(Path::new(&plan.path)))
                .map(|birth| birth.path.clone())
        }),
    }
}

#[cfg(all(test, windows))]
#[path = "library_creation_tests.rs"]
mod tests;
