//! Trash locations are projections of original committed Plans. This module
//! stores no state and never grants an identity from the current pathname.
use super::*;
use crate::sync::ObjectKey;
use weftext_messages::UiMessage;

pub(crate) struct OriginalTrashRoot {
    pub(crate) operation_id: String,
    pub(crate) path: PathBuf,
    pub(crate) identity: FileIdentity,
    pub(crate) node_id: Option<String>,
    pub(crate) leaf: String,
    pub(crate) leaf_identity: FileIdentity,
}

impl Core {
    pub(crate) fn original_trash_roots(&self, root: &Path) -> Result<Vec<OriginalTrashRoot>> {
        let workspace = self.require_workspace(root)?.workspace_id;
        let trash = root.join(".weftext-trash");
        let mut roots = Vec::<OriginalTrashRoot>::new();
        for plan in self.committed_plans()?.into_iter() {
            let Some(transaction) = &plan.transaction else {
                continue;
            };
            if !locator_equal(&transaction.request.root, &display(root))
                || plan.workspace_id.as_deref() != Some(&workspace)
            {
                continue;
            }
            for step in transaction.relocations() {
                if locator_equal(step.source, step.target)
                    || !direct_child_of(Path::new(step.target), &trash)?
                {
                    continue;
                }
                let node_id = self
                    .admitted_binding(step.leaf_identity)?
                    .filter(|birth| birth.workspace_id == workspace)
                    .map(|birth| birth.node_id);
                // The existing Node relocation freezes its original admitted
                // canonical file. A Directory requires its exact typed role.
                if node_id.is_none()
                    && !plan.sync_application.as_ref().is_some_and(|application| {
                        application.after.iter().any(|object| {
                            matches!(object.object, ObjectKey::Directory(_))
                                && object.state.retired
                                && locator_equal(&object.directory, step.target)
                                && step.new_leaf == "weftext.directory.json"
                        })
                    })
                {
                    continue;
                }
                add_root(
                    &mut roots,
                    OriginalTrashRoot {
                        operation_id: plan.operation_id.clone(),
                        path: PathBuf::from(step.target),
                        identity: step.source_identity.clone(),
                        node_id,
                        leaf: step.new_leaf.to_owned(),
                        leaf_identity: step.leaf_identity.clone(),
                    },
                )?;
            }
            if plan.kind != "cloneSync" {
                continue;
            }
            let Some(setup) = &plan.sync_setup else {
                continue;
            };
            if setup.identity.workspace_id != workspace
                || !locator_equal(&setup.root, &display(root))
            {
                return Err(CoreError::new(
                    "identityConflict",
                    UiMessage::error_core_operations_clone_trash_has_different_original_workspace(),
                ));
            }
            let Mutation::CreateTree {
                target,
                files,
                births,
                ..
            } = &transaction.mutation
            else {
                return Err(CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_operations_clone_trash_requires_original_create_tree(),
                ));
            };
            if !locator_equal(target, &display(root)) {
                return Err(CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_operations_clone_trash_escaped_original_receiver(),
                ));
            }
            for object in &setup.initial {
                if !object.state.retired || !direct_child_of(Path::new(&object.directory), &trash)?
                {
                    continue;
                }
                let (node_id, leaf) = match &object.object {
                    ObjectKey::Node(id) => (Some(id.clone()), format!("{}.typ", object.state.name)),
                    ObjectKey::Directory(_) => (None, "weftext.directory.json".into()),
                    _ => {
                        return Err(CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_clone_trash_lacks_placed_content_role(
                            ),
                        ));
                    }
                };
                let path = Path::new(&object.directory).join(&leaf);
                let output = files
                    .iter()
                    .find(|file| locator_equal(&file.path, &display(&path)))
                    .ok_or_else(|| {
                        CoreError::new(
                            "identityPending",
                            UiMessage::error_core_operations_clone_trash_lacks_exact_original_output(),
                        )
                    })?;
                if object.state.files.get(&leaf) != Some(&hash(&output.after))
                    || transaction.preview.revisions.get(&output.path)
                        != object.state.file_revisions.get(&leaf)
                {
                    return Err(CoreError::new(
                        "identityConflict",
                        UiMessage::error_core_operations_clone_trash_differs_from_original_role_and_file_revision(),
                    ));
                }
                let identity = self
                    .materialized_identity(&plan.operation_id, Path::new(&object.directory))?
                    .ok_or_else(|| {
                        CoreError::new(
                            "identityPending",
                            UiMessage::error_core_operations_clone_trash_lacks_original_materialized_directory(),
                        )
                    })?;
                let leaf_identity = self
                    .materialized_identity(&plan.operation_id, &path)?
                    .ok_or_else(|| {
                        CoreError::new(
                            "identityPending",
                            UiMessage::error_core_operations_clone_trash_lacks_original_materialized_file(),
                        )
                    })?;
                if let Some(id) = &node_id {
                    if !births.iter().any(|birth| {
                        birth.node_id == *id && locator_equal(&birth.path, &display(&path))
                    }) || weftext_engine::inspect_metadata(&decode(&output.after)?)
                        .id
                        .as_ref()
                        != Some(id)
                        || self.admitted_binding(&leaf_identity)?.is_none_or(|birth| {
                            birth.workspace_id != workspace || birth.node_id != *id
                        })
                    {
                        return Err(CoreError::new(
                            "identityConflict",
                            UiMessage::error_core_operations_clone_trash_node_lacks_exact_original_birth(),
                        ));
                    }
                } else if let ObjectKey::Directory(id) = &object.object {
                    crate::directory_identity::validate_directory_marker(&output.after)?;
                    let marker: serde_json::Value = serde_json::from_slice(&output.after)?;
                    if marker["directoryId"] != *id {
                        return Err(CoreError::new(
                            "identityConflict",
                            UiMessage::error_core_operations_clone_trash_directory_marker_differs_from_exact_role(),
                        ));
                    }
                }
                add_root(
                    &mut roots,
                    OriginalTrashRoot {
                        operation_id: plan.operation_id.clone(),
                        path: PathBuf::from(&object.directory),
                        identity,
                        node_id,
                        leaf,
                        leaf_identity,
                    },
                )?;
            }
        }
        Ok(roots)
    }

    pub(crate) fn select_original_trash_node(
        &self,
        root: &Path,
        operation_id: &str,
        node_id: Option<&str>,
    ) -> Result<OriginalTrashRoot> {
        if let Some(id) = node_id {
            valid_operation_id(id)?;
        }
        let mut selected = self
            .original_trash_roots(root)?
            .into_iter()
            .filter(|entry| {
                entry.operation_id == operation_id
                    && entry
                        .node_id
                        .as_deref()
                        .is_some_and(|id| node_id.is_none_or(|wanted| id == wanted))
            });
        let first = selected.next().ok_or_else(|| {
            CoreError::new(
                if node_id.is_some() {
                    "trashSelectionMismatch"
                } else {
                    "trashMissing"
                },
                UiMessage::error_core_operations_original_committed_operation_has_no_matching_retained_node_root(),
            )
        })?;
        if selected.next().is_some() {
            return Err(CoreError::new(
                "trashSelectionRequired",
                UiMessage::error_core_operations_select_exact_node_identity_from_operation_s_retained_trash_roots(),
            ));
        }
        Ok(first)
    }

    pub(crate) fn capture_original_trash_node(
        &self,
        entry: &OriginalTrashRoot,
    ) -> Result<CapturedFile> {
        let guard = ParentGuard::acquire(&self.backend, &entry.path.join(".trash-root-proof"))?;
        if guard.directory_identity()? != entry.identity {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_operations_original_committed_trash_directory_was_replaced(),
            ));
        }
        let source = entry.path.join(&entry.leaf);
        self.check_pending(&source)?;
        let file = capture_optional(&self.backend, &source)?.ok_or_else(|| {
            CoreError::new("missingNode", "The retained canonical source is missing")
        })?;
        if file.stamp.identity != entry.leaf_identity
            || self
                .admitted_binding(&file.stamp.identity)?
                .is_none_or(|birth| entry.node_id.as_ref() != Some(&birth.node_id))
        {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_operations_retained_canonical_source_not_original_admitted_object(),
            ));
        }
        Ok(file)
    }
}

fn add_root(roots: &mut Vec<OriginalTrashRoot>, root: OriginalTrashRoot) -> Result<()> {
    if let Some(previous) = roots
        .iter()
        .find(|entry| locator_equal(&display(&entry.path), &display(&root.path)))
    {
        if previous.identity != root.identity
            || previous.node_id != root.node_id
            || previous.leaf_identity != root.leaf_identity
        {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_operations_original_trash_plans_disagree_on_retained_physical_tree(),
            ));
        }
        return Ok(());
    }
    if roots.len() == MAX_OPERATION_FILES {
        return Err(CoreError::new(
            "budget",
            UiMessage::error_core_operations_original_trash_roots_exceed_bounded_operation_scope(),
        ));
    }
    roots.push(root);
    Ok(())
}

fn direct_child_of(path: &Path, parent: &Path) -> Result<bool> {
    match path.parent() {
        Some(actual) => Ok(
            crate::backend::namespace_path_compare(actual, parent)? == std::cmp::Ordering::Equal
        ),
        None => Ok(false),
    }
}
