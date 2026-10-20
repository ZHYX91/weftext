//! Shared author sibling order is a projection over admitted physical parents.
//! The existing Settings object owns the sole order fact; pathnames never do.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const SETTINGS: &str = ".weftext-settings.json";
const MAX_ORDER_MEMBERS: usize = 5000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationOrderMember {
    pub target_ref: String,
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationOrderSnapshot {
    pub parent_ref: String,
    pub base_id: String,
    pub before_source: Option<String>,
    pub members: Vec<NavigationOrderMember>,
    pub ordered_refs: Vec<String>,
    pub complete: bool,
}

fn invalid(message: &str) -> CoreError {
    CoreError::new("navigationOrder", message)
}

fn order_ref(reference: &str) -> Result<(&str, &str, &str)> {
    let parsed = crate::directory_identity::parse_ref(reference)?;
    if !matches!(parsed.0, "node" | "directory")
        || Uuid::parse_str(parsed.1).unwrap().to_string() != parsed.1
        || Uuid::parse_str(parsed.2).unwrap().to_string() != parsed.2
    {
        return Err(invalid(
            "Sibling order requires canonical NodeRef or DirectoryRef identities",
        ));
    }
    Ok(parsed)
}

/// Settings retain unrelated extension members. Only the navigationOrders
/// member is closed here, so processing/style/template writers share one codec.
pub fn validate_workspace_settings(bytes: &[u8]) -> Result<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    if !value.is_object() || value["version"] != 1 {
        return Err(invalid(
            "Workspace settings require an object with version 1",
        ));
    }
    let Some(orders) = value.get("navigationOrders") else {
        return Ok(value);
    };
    let orders = orders
        .as_object()
        .ok_or_else(|| invalid("navigationOrders must be an object"))?;
    let mut total = orders.len();
    let mut workspace = None;
    for (parent, children) in orders {
        let (_, owner, _) = order_ref(parent)?;
        if workspace.is_some_and(|previous| previous != owner) {
            return Err(invalid(
                "One Settings object cannot order multiple workspaces",
            ));
        }
        workspace = Some(owner);
        let children = children
            .as_array()
            .ok_or_else(|| invalid("Each parent order must be an array of typed references"))?;
        total += children.len();
        if total > MAX_ORDER_MEMBERS {
            return Err(invalid("Navigation order exceeds the 5000-member budget"));
        }
        let mut seen = BTreeSet::new();
        for child in children {
            let child = child
                .as_str()
                .ok_or_else(|| invalid("An ordered child must be a typed reference"))?;
            if order_ref(child)?.1 != owner || child == parent || !seen.insert(child) {
                return Err(invalid(
                    "Ordered children must be unique, same-workspace identities distinct from their parent",
                ));
            }
        }
    }
    Ok(value)
}

pub(crate) fn validate_workspace_settings_scope(bytes: &[u8], workspace: &str) -> Result<()> {
    let value = validate_workspace_settings(bytes)?;
    if value
        .get("navigationOrders")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|orders| {
            orders
                .keys()
                .any(|parent| order_ref(parent).is_ok_and(|(_, owner, _)| owner != workspace))
        })
    {
        return Err(CoreError::new(
            "outsideWorkspace",
            "Navigation order belongs to another workspace",
        ));
    }
    Ok(())
}

fn settings_value(bytes: Option<&[u8]>) -> Result<serde_json::Value> {
    bytes
        .map(validate_workspace_settings)
        .transpose()
        .map(|value| value.unwrap_or_else(|| serde_json::json!({"version":1})))
}

fn orders(value: &serde_json::Value) -> BTreeMap<String, Vec<String>> {
    value
        .get("navigationOrders")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .map(|(parent, children)| {
            (
                parent.clone(),
                children
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|child| child.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect()
}

pub fn navigation_orders_changed(before: Option<&[u8]>, after: &[u8]) -> Result<bool> {
    Ok(orders(&settings_value(before)?) != orders(&validate_workspace_settings(after)?))
}

/// The one config slot may be created by this exact update. Its own file
/// participant proves before/after identity and bytes; it is not a new sibling.
pub(crate) fn navigation_order_directory_entries(
    transaction: &operations::FrozenOperation,
    directory: &Path,
    entries: Vec<String>,
) -> Vec<String> {
    let root = Path::new(&transaction.request.root);
    if directory != root
        || !matches!(&transaction.request.intent, OperationIntent::UpdateFiles { changes }
            if changes.iter().any(|change| Path::new(&change.path) == root.join(SETTINGS)
                && navigation_orders_changed(change.before.as_deref(), &change.after).unwrap_or(false)))
    {
        return entries;
    }
    entries
        .into_iter()
        .filter(|entry| !entry.starts_with(&format!("{SETTINGS}:")))
        .collect()
}

impl Core {
    fn navigation_settings_were_committed(&self, root: &Path) -> Result<bool> {
        let workspace = self.require_workspace(root)?;
        Ok(self.operations.values().any(|plan| {
            plan.workspace_id.as_deref() == Some(&workspace.workspace_id)
                && self.committed(&plan.operation_id)
                && plan.transaction.as_ref().is_some_and(|transaction| {
                    transaction.preview.revisions.contains_key(&display(
                        &Path::new(&transaction.request.root).join(SETTINGS),
                    ))
                })
        }))
    }

    fn navigation_members(
        &self,
        root: &Path,
        parent_ref: &str,
    ) -> Result<(Vec<NavigationOrderMember>, bool)> {
        let (_, owner, _) = order_ref(parent_ref)?;
        let workspace = self.require_workspace(root)?;
        if owner != workspace.workspace_id {
            return Err(CoreError::new(
                "outsideWorkspace",
                "The ordered parent belongs to another workspace",
            ));
        }
        let parent = self.resolve_navigation_target(root, parent_ref)?;
        if parent.status != "managed" {
            return Err(invalid(
                "The parent needs one current admitted navigation identity",
            ));
        }
        let parent = PathBuf::from(
            parent
                .path
                .ok_or_else(|| invalid("The parent has no live placement"))?,
        );
        let inventory = self.inventory(root)?;
        let mut complete = inventory.complete;
        let parent_guard = ParentGuard::acquire(&parent.join(".order-prospective"))?;
        let (physical, enumerated) = parent_guard.directory_entries(MAX_ORDER_MEMBERS)?;
        complete &= enumerated;
        let visible_directories = inventory
            .entries
            .iter()
            .filter(|entry| entry.kind == "directory")
            .map(|entry| Path::new(&entry.path))
            .collect::<BTreeSet<_>>();
        // Valid ignore rules do not make a partial list an entire sibling set.
        // Do not disclose excluded names, but refuse a subset author reorder.
        for (name, attributes) in physical {
            let path = parent.join(name);
            if attributes & 0x10 != 0
                && !crate::inventory::internal(path.strip_prefix(root).unwrap())
                && !visible_directories.contains(path.as_path())
            {
                complete = false;
            }
        }
        let mut nodes = BTreeMap::<PathBuf, Vec<&InventoryEntry>>::new();
        let mut ids = BTreeMap::<&str, usize>::new();
        for entry in inventory
            .entries
            .iter()
            .filter(|entry| entry.kind == "node")
        {
            nodes
                .entry(PathBuf::from(&entry.parent_path))
                .or_default()
                .push(entry);
            if let Some(id) = entry.node_id.as_deref() {
                *ids.entry(id).or_default() += 1;
            }
        }
        // Scan marker claims once, rather than resolving every child through a
        // second whole-library inventory. Duplicate claims never gain identity.
        let mut directories = BTreeMap::new();
        let mut directory_ids = BTreeMap::<String, usize>::new();
        for path in std::iter::once(root.to_path_buf()).chain(
            inventory
                .entries
                .iter()
                .filter(|entry| entry.kind == "directory")
                .map(|entry| PathBuf::from(&entry.path)),
        ) {
            let marker = path.join("weftext.directory.json");
            if let Some(file) = operations::capture_optional(&marker)? {
                self.check_pending(&marker)?;
                if crate::directory_identity::validate_directory_marker(&file.bytes).is_err() {
                    if path.parent() == Some(parent.as_path()) {
                        complete = false;
                    }
                    continue;
                }
                let value: serde_json::Value = serde_json::from_slice(&file.bytes)?;
                let id = value["directoryId"].as_str().unwrap().to_owned();
                *directory_ids.entry(id.clone()).or_default() += 1;
                directories.insert(
                    path,
                    (id, ParentGuard::acquire(&marker)?.directory_identity()?),
                );
            }
        }
        let mut members = Vec::new();
        for entry in inventory
            .entries
            .iter()
            .filter(|entry| entry.kind == "directory" && Path::new(&entry.parent_path) == parent)
        {
            let path = PathBuf::from(&entry.path);
            let reference = if let Some(children) = nodes.get(&path) {
                if children.len() == 1
                    && children[0].identity_status == "admitted"
                    && children[0]
                        .node_id
                        .as_deref()
                        .is_some_and(|id| ids.get(id) == Some(&1))
                {
                    Some(format!(
                        "weftext-node://{}/{}",
                        owner,
                        children[0].node_id.as_deref().unwrap()
                    ))
                } else {
                    None
                }
            } else if let Some((id, identity)) = directories.get(&path) {
                if directory_ids.get(id) == Some(&1)
                    && self.directory_admitted(owner, id, identity)?
                {
                    Some(format!("weftext-directory://{owner}/{id}"))
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(target_ref) = reference {
                members.push(NavigationOrderMember {
                    target_ref,
                    path: entry.path.clone(),
                    name: entry.name.clone(),
                });
            } else {
                complete = false;
            }
        }
        members.sort_by(|a, b| {
            (a.name.to_lowercase(), &a.name, &a.target_ref).cmp(&(
                b.name.to_lowercase(),
                &b.name,
                &b.target_ref,
            ))
        });
        Ok((members, complete))
    }

    pub fn read_navigation_order(
        &mut self,
        root: impl AsRef<Path>,
        parent_ref: &str,
    ) -> Result<NavigationOrderSnapshot> {
        let root = absolute_path(root.as_ref())?;
        let (members, complete) = self.navigation_members(&root, parent_ref)?;
        let config = self.read_config(&root, ConfigRole::WorkspaceSettings)?;
        if config.bytes.is_none() && self.navigation_settings_were_committed(&root)? {
            self.snapshots.remove(&config.base_id);
            return Err(CoreError::new(
                "settingsUnavailable",
                "Previously committed Settings are missing; recover the original object before author reordering",
            ));
        }
        if let Some(bytes) = &config.bytes {
            validate_workspace_settings_scope(bytes, &self.require_workspace(&root)?.workspace_id)?;
        }
        let value = settings_value(config.bytes.as_deref())?;
        let live = members
            .iter()
            .map(|member| member.target_ref.as_str())
            .collect::<BTreeSet<_>>();
        let mut ordered_refs = orders(&value)
            .remove(parent_ref)
            .unwrap_or_default()
            .into_iter()
            .filter(|reference| live.contains(reference.as_str()))
            .collect::<Vec<_>>();
        let retained = ordered_refs.iter().cloned().collect::<BTreeSet<_>>();
        ordered_refs.extend(
            members
                .iter()
                .filter(|member| !retained.contains(&member.target_ref))
                .map(|member| member.target_ref.clone()),
        );
        Ok(NavigationOrderSnapshot {
            parent_ref: parent_ref.into(),
            base_id: config.base_id,
            before_source: config.bytes.as_deref().map(decode).transpose()?,
            members,
            ordered_refs,
            complete,
        })
    }

    pub fn prepare_navigation_reorder(
        &mut self,
        root: impl AsRef<Path>,
        parent_ref: &str,
        children: &[String],
        base_id: &str,
        operation_id: &str,
    ) -> Result<OperationPreview> {
        let root = absolute_path(root.as_ref())?;
        let canonical = serde_json::json!({"kind":"reorderNavigation","root":display(&root),"parentRef":parent_ref,"children":children,"baseId":base_id});
        if let Some(preview) = self.feature_operation_status(operation_id, &canonical)? {
            return Ok(preview);
        }
        let path = root.join(SETTINGS);
        let basis = self.snapshots.get(base_id).ok_or_else(|| {
            CoreError::new(
                "basisRequired",
                "Read the exact navigation Settings snapshot first",
            )
        })?;
        if basis.path != display(&path) {
            return Err(invalid("The order basis belongs to another object"));
        }
        let before = basis.captured.as_ref().map(|file| file.bytes.clone());
        if before.is_none() && self.navigation_settings_were_committed(&root)? {
            return Err(CoreError::new(
                "settingsUnavailable",
                "Previously committed Settings cannot be recreated from an empty order basis",
            ));
        }
        let (members, complete) = self.navigation_members(&root, parent_ref)?;
        let requested = children.iter().collect::<BTreeSet<_>>();
        if !complete
            || requested.len() != children.len()
            || requested != members.iter().map(|member| &member.target_ref).collect()
        {
            return Err(invalid(
                "Reordering requires every current admitted direct child exactly once",
            ));
        }
        let mut value = settings_value(before.as_deref())?;
        if value.get("navigationOrders").is_none() {
            value["navigationOrders"] = serde_json::json!({});
        }
        value["navigationOrders"][parent_ref] = serde_json::json!(children);
        self.prepare_feature_operation(
            OperationRequest {
                operation_id: operation_id.into(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&path),
                        base_id: Some(base_id.into()),
                        before,
                        after: serde_json::to_vec_pretty(&value)?,
                    }],
                },
            },
            &canonical,
        )
    }

    pub fn cancel_navigation_reorder(
        &mut self,
        root: impl AsRef<Path>,
        operation_id: &str,
        plan_id: &str,
    ) -> Result<OperationPreview> {
        let root = absolute_path(root.as_ref())?;
        let preview = self.operation_details(operation_id)?;
        let transaction = self.operations[operation_id].transaction.as_ref().unwrap();
        if preview.plan_id != plan_id
            || transaction.request.root != display(&root)
            || !matches!(&transaction.request.intent, OperationIntent::UpdateFiles { changes } if changes.len() == 1 && Path::new(&changes[0].path) == root.join(SETTINGS) && validate_workspace_settings(&changes[0].after)?.get("navigationOrders").is_some())
        {
            return Err(invalid(
                "Cancel requires the exact original navigation Settings Plan",
            ));
        }
        if matches!(preview.status.as_str(), "committed" | "resolved") {
            return Ok(preview);
        }
        if self.journal_path(operation_id, "install").exists() {
            return Err(CoreError::new(
                "recoveryPending",
                "Original installation may have effects; inspect recovery instead",
            ));
        }
        self.resolve_record(operation_id, "cancelPrepared")?;
        self.operation_details(operation_id)
    }

    pub(crate) fn validate_navigation_order_update(
        &self,
        root: &Path,
        change: &FileChange,
    ) -> Result<()> {
        if Path::new(&change.path) != root.join(SETTINGS) {
            return Ok(());
        }
        validate_workspace_settings_scope(
            &change.after,
            &self.require_workspace(root)?.workspace_id,
        )?;
        if change.before.is_none()
            && validate_workspace_settings(&change.after)?
                .get("navigationOrders")
                .is_some()
            && self.navigation_settings_were_committed(root)?
        {
            return Err(CoreError::new(
                "settingsUnavailable",
                "Recover missing committed Settings before writing a new author order",
            ));
        }
        let before = orders(&settings_value(change.before.as_deref())?);
        for (parent, children) in orders(&validate_workspace_settings(&change.after)?) {
            if before.get(&parent) == Some(&children) {
                continue;
            }
            let (members, complete) = self.navigation_members(root, &parent)?;
            let actual = members
                .into_iter()
                .map(|member| member.target_ref)
                .collect::<BTreeSet<_>>();
            if !complete || actual != children.into_iter().collect() {
                return Err(invalid(
                    "Reordering requires the complete current admitted direct-child set",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn capture_navigation_order_dependencies(
        &self,
        root: &Path,
        change: &FileChange,
        files: &mut Vec<operations::CapturedFile>,
        directories: &mut Vec<operations::CapturedDirectory>,
    ) -> Result<()> {
        if Path::new(&change.path) != root.join(SETTINGS) {
            return Ok(());
        }
        let before = orders(&settings_value(change.before.as_deref())?);
        for (parent, children) in orders(&validate_workspace_settings(&change.after)?) {
            if before.get(&parent) == Some(&children) {
                continue;
            }
            let resolved = self.resolve_navigation_target(root, &parent)?;
            let parent_path = PathBuf::from(
                resolved
                    .path
                    .ok_or_else(|| invalid("The ordered parent lost its placement"))?,
            );
            let (members, complete) = self.navigation_members(root, &parent)?;
            if !complete {
                return Err(invalid("The sibling inventory is incomplete"));
            }
            for path in std::iter::once(parent_path)
                .chain(members.into_iter().map(|member| PathBuf::from(member.path)))
            {
                let anchor = path.join(".order-prospective");
                let guard = ParentGuard::acquire(&anchor)?;
                let (entries, complete) = guard.directory_entries(MAX_ORDER_MEMBERS)?;
                if !complete {
                    return Err(invalid(
                        "The sibling directory observation exceeded its budget",
                    ));
                }
                let captured = operations::CapturedDirectory {
                    path: display(&path),
                    identity: guard.directory_identity()?,
                    entries: operations::sorted_entries(entries, &path),
                };
                if !directories
                    .iter()
                    .any(|directory| directory.path == captured.path)
                {
                    directories.push(captured);
                }
                let canonical = path.join(format!(
                    "{}.typ",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ));
                for dependency in [canonical, path.join("weftext.directory.json")] {
                    if let Some(file) = operations::capture_optional(&dependency)?
                        && !files.iter().any(|old| old.path == file.path)
                    {
                        files.push(file);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests;
