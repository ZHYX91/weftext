//! Closed hosted destination capabilities in the original frozen Plan. Member
//! discovery reads committed Plan/Birth symbols, never directory occupants.
use super::*;
use crate::operations::Mutation;
use std::collections::BTreeSet;
use weftext_knowledge::NodeRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum HostedDestination {
    Fixed { slot: String },
    UniqueChild(HostedUniqueChild),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostedUniqueChild {
    slot: String,
    parent: String,
    parent_identity: FileIdentity,
    workspace_id: String,
    prefix: String,
    max_members: u32,
    external_operation_id: String,
}

struct UniqueMemberScope<'a> {
    slot: &'a str,
    parent: &'a str,
    parent_identity: &'a FileIdentity,
    workspace_id: &'a str,
    prefix: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UniqueBirth {
    node_id: String,
    workspace_id: String,
    identity: FileIdentity,
}
impl<'a> From<&'a HostedUniqueChild> for UniqueMemberScope<'a> {
    fn from(binding: &'a HostedUniqueChild) -> Self {
        Self {
            slot: &binding.slot,
            parent: &binding.parent,
            parent_identity: &binding.parent_identity,
            workspace_id: &binding.workspace_id,
            prefix: &binding.prefix,
        }
    }
}

impl HostedDestination {
    pub fn fixed(slot: &str) -> Self {
        Self::Fixed { slot: slot.into() }
    }
    pub fn slot(&self) -> &str {
        match self {
            Self::Fixed { slot } => slot,
            Self::UniqueChild(binding) => &binding.slot,
        }
    }
    pub(crate) fn is_fixed_slot(&self, expected: &str) -> bool {
        matches!(self, Self::Fixed { slot } if slot == expected)
    }
}
impl HostedUniqueChild {
    pub(crate) fn external_operation_id(&self) -> &str {
        &self.external_operation_id
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedUniqueMember {
    pub owner: NodeRef,
    pub relative_path: String,
}

fn unavailable() -> CoreError {
    CoreError::new(
        "slotUnavailable",
        weftext_messages::UiMessage::error_core_hosted_destination_original_unique_child_capability_is_unavailable(),
    )
}
fn same_path(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b).is_some_and(|r| r.as_os_str().is_empty())
}
fn prefix_valid(prefix: &str) -> bool {
    !prefix.is_empty()
        && prefix.len() <= 180
        && !prefix.contains(['/', '\\', ':'])
        && !prefix.ends_with(['.', ' '])
        && !prefix.chars().any(char::is_control)
        && !matches!(prefix, "." | "..")
}
fn parent_identity(backend: &BackendContext, parent: &Path) -> Result<FileIdentity> {
    // The leaf is not opened or created. Its placement guard binds the actual
    // parent directory and does not adopt a path-equivalent replacement.
    ParentGuard::acquire(backend, &parent.join(".weftext-unique-child-placement"))?
        .directory_identity()
}

impl Core {
    fn unique_evidence(
        &self,
        operation: &str,
        suffix: &str,
        max_bytes: usize,
    ) -> Result<Option<Vec<u8>>> {
        let path = self.journal_path(operation, suffix);
        let guard = ParentGuard::acquire(&self.backend, &path)?;
        let mut file = match guard.open_read_guard_file() {
            Ok(file) => file,
            Err(error) if error.code == "missing" => return Ok(None),
            Err(_) => return Err(unavailable()),
        };
        if file.metadata()?.len() > max_bytes as u64 {
            return Err(CoreError::new(
                "budget",
                weftext_messages::UiMessage::error_core_hosted_destination_original_unique_child_evidence_exceeds_its_byte_budget(),
            ));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(max_bytes as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > max_bytes {
            return Err(unavailable());
        }
        Ok(Some(bytes))
    }
    fn unique_committed(&self, plan: &journal::Plan) -> Result<bool> {
        let Some(bytes) = self.unique_evidence(&plan.operation_id, "commit", 64 * 1024)? else {
            if self
                .unique_evidence(&plan.operation_id, "install", MAX_SOURCE_BYTES)?
                .is_some()
            {
                return Err(unavailable());
            }
            return Ok(false);
        };
        journal::validate_original_commit_bytes(plan, &bytes)
            .map_err(|cause| unavailable().caused_by(cause))?;
        Ok(true)
    }
    fn unique_births(&self, operation: &str) -> Result<Vec<UniqueBirth>> {
        // Unique creation always uses the unified CreateTree producer. A
        // second single-birth marker is ambiguous evidence, not a fallback.
        if self
            .unique_evidence(operation, "birth", 16 * 1024)?
            .is_some()
        {
            return Err(unavailable());
        }
        let bytes = self
            .unique_evidence(operation, "births", MAX_SOURCE_BYTES)?
            .ok_or_else(unavailable)?;
        let births: Vec<UniqueBirth> = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
        if births.is_empty() || births.len() > 5000 {
            return Err(unavailable());
        }
        Ok(births)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn hosted_unique_child_destination(
        &self,
        root: &Path,
        parent: &Path,
        slot: &str,
        prefix: &str,
        max_members: u32,
        external_operation_id: &str,
    ) -> Result<HostedDestination> {
        hosted::valid_resource(slot)?;
        valid_operation_id(external_operation_id)?;
        if !prefix_valid(prefix) || !(1..=5000).contains(&max_members) {
            return Err(unavailable());
        }
        let root = absolute_path(root)?;
        let parent = absolute_path(parent)?;
        if crate::history::history_relative_path(&parent, &root).is_none() {
            return Err(unavailable());
        }
        let workspace = self.require_workspace(&root)?;
        Ok(HostedDestination::UniqueChild(HostedUniqueChild {
            slot: slot.into(),
            parent: display(&parent),
            parent_identity: parent_identity(&self.backend, &parent)?,
            workspace_id: workspace.workspace_id,
            prefix: prefix.into(),
            max_members,
            external_operation_id: external_operation_id.into(),
        }))
    }

    fn unique_members(
        &self,
        binding: &UniqueMemberScope<'_>,
        selected_prefix: bool,
    ) -> Result<Vec<HostedUniqueMember>> {
        let mut members = Vec::new();
        let mut ids = BTreeSet::new();
        let mut metadata = 0usize;
        for plan in self
            .operations
            .values()
            .filter(|p| p.workspace_id.as_deref() == Some(binding.workspace_id))
        {
            let Some(transaction) = &plan.transaction else {
                continue;
            };
            let Some(HostedDestination::UniqueChild(original)) = &transaction.hosted_destination
            else {
                continue;
            };
            if original.slot != binding.slot
                || original.parent_identity != *binding.parent_identity
                || original.workspace_id != binding.workspace_id
                || !same_path(Path::new(&original.parent), Path::new(binding.parent))
                || selected_prefix && original.prefix != binding.prefix
            {
                continue;
            }
            if !self.unique_committed(plan)? {
                continue;
            }
            self.validate_unique_intent(original, &transaction.request.intent)?;
            let Mutation::CreateTree {
                target,
                births,
                existing_target: false,
                files,
                ..
            } = &transaction.mutation
            else {
                return Err(unavailable());
            };
            let name = format!("{}{}", original.prefix, original.external_operation_id);
            if !same_path(Path::new(target), &Path::new(&original.parent).join(name)) {
                return Err(unavailable());
            }
            let actual = self.unique_births(&plan.operation_id)?;
            if actual.len() != births.len() {
                return Err(unavailable());
            }
            for birth in births {
                let path = Path::new(&birth.path);
                if !is_canonical(path)
                    || !files
                        .iter()
                        .any(|f| f.path == birth.path && f.before.is_none())
                    || actual
                        .iter()
                        .filter(|b| {
                            b.node_id == birth.node_id && b.workspace_id == binding.workspace_id
                        })
                        .count()
                        != 1
                    || !ids.insert(birth.node_id.clone())
                {
                    return Err(unavailable());
                }
                let relative =
                    crate::history::history_relative_path(path, Path::new(binding.parent))
                        .ok_or_else(unavailable)?;
                if relative.as_os_str().is_empty()
                    || relative
                        .components()
                        .any(|c| !matches!(c, Component::Normal(_)))
                {
                    return Err(unavailable());
                }
                let relative_path = display(&relative).replace('\\', "/");
                metadata = metadata
                    .checked_add(
                        relative_path.len() + birth.node_id.len() + binding.workspace_id.len(),
                    )
                    .ok_or_else(unavailable)?;
                if members.len() == 5000 || metadata > MAX_SOURCE_BYTES {
                    return Err(CoreError::new(
                        "budget",
                        weftext_messages::UiMessage::error_core_hosted_destination_unique_child_member_metadata_exceeds_its_budget(),
                    ));
                }
                members.push(HostedUniqueMember {
                    owner: NodeRef {
                        workspace_id: binding.workspace_id.to_owned(),
                        node_id: birth.node_id.clone(),
                    },
                    relative_path,
                });
            }
        }
        members.sort_by(|a, b| {
            a.relative_path
                .cmp(&b.relative_path)
                .then(a.owner.node_id.cmp(&b.owner.node_id))
        });
        Ok(members)
    }

    fn validate_unique_intent(
        &self,
        binding: &HostedUniqueChild,
        intent: &OperationIntent,
    ) -> Result<()> {
        hosted::valid_resource(&binding.slot)?;
        valid_operation_id(&binding.external_operation_id)?;
        if !prefix_valid(&binding.prefix) || !(1..=5000).contains(&binding.max_members) {
            return Err(unavailable());
        }
        let (parent, name) = match intent {
            OperationIntent::NewNode { parent, name, .. }
            | OperationIntent::TemplateNode { parent, name, .. } => (parent, name),
            _ => return Err(unavailable()),
        };
        if !same_path(Path::new(parent), Path::new(&binding.parent))
            || name != &format!("{}{}", binding.prefix, binding.external_operation_id)
        {
            return Err(unavailable());
        }
        Ok(())
    }

    pub(crate) fn validate_hosted_destination(
        &self,
        root: &Path,
        intent: &OperationIntent,
        destination: &HostedDestination,
        mutation: Option<&Mutation>,
    ) -> Result<()> {
        hosted::valid_resource(destination.slot())?;
        let HostedDestination::UniqueChild(binding) = destination else {
            return Ok(());
        };
        self.validate_unique_intent(binding, intent)?;
        let parent = Path::new(&binding.parent);
        if self.require_workspace(root)?.workspace_id != binding.workspace_id
            || crate::history::history_relative_path(parent, root).is_none()
            || parent_identity(&self.backend, parent)? != binding.parent_identity
        {
            return Err(unavailable());
        }
        let new_members = match mutation {
            None => 1,
            Some(Mutation::CreateTree {
                births,
                existing_target: false,
                ..
            }) if !births.is_empty() => births.len(),
            _ => return Err(unavailable()),
        };
        // Count every retained original member in this slot/physical parent,
        // including unavailable members and earlier prefix configurations.
        if self
            .unique_members(&binding.into(), false)?
            .len()
            .saturating_add(new_members)
            > binding.max_members as usize
        {
            return Err(CoreError::new(
                "budget",
                weftext_messages::UiMessage::error_core_hosted_destination_unique_child_member_budget_is_exhausted(),
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn selected_unique_members_at(
        &self,
        root: &Path,
        parent: &Path,
        slot: &str,
        prefix: &str,
        max_members: u32,
        selected_prefix: bool,
    ) -> Result<Vec<HostedUniqueMember>> {
        hosted::valid_resource(slot)?;
        if !prefix_valid(prefix) || !(1..=5000).contains(&max_members) {
            return Err(unavailable());
        }
        let root = absolute_path(root)?;
        let parent = absolute_path(parent)?;
        if crate::history::history_relative_path(&parent, &root).is_none() {
            return Err(unavailable());
        }
        let workspace = self.require_workspace(&root)?;
        let identity = parent_identity(&self.backend, &parent)?;
        let parent = display(&parent);
        let selector = UniqueMemberScope {
            slot,
            parent: &parent,
            parent_identity: &identity,
            workspace_id: &workspace.workspace_id,
            prefix,
        };
        let all = self.unique_members(&selector, false)?;
        if all.len() > max_members as usize {
            return Err(CoreError::new(
                "budget",
                weftext_messages::UiMessage::error_core_hosted_destination_retained_unique_child_members_exceed_the_configured_budget(),
            ));
        }
        if selected_prefix {
            self.unique_members(&selector, true)
        } else {
            Ok(all)
        }
    }
    pub fn hosted_unique_child_members_at(
        &self,
        root: &Path,
        parent: &Path,
        slot: &str,
        prefix: &str,
        max_members: u32,
    ) -> Result<Vec<HostedUniqueMember>> {
        self.selected_unique_members_at(root, parent, slot, prefix, max_members, true)
    }
    pub fn hosted_unique_child_available_at(
        &self,
        root: &Path,
        parent: &Path,
        slot: &str,
        prefix: &str,
        max_members: u32,
    ) -> Result<bool> {
        Ok(self
            .selected_unique_members_at(root, parent, slot, prefix, max_members, false)?
            .len()
            < max_members as usize)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn qualified_hosted_unique_child_at(
        &self,
        root: &Path,
        parent: &Path,
        slot: &str,
        prefix: &str,
        max_members: u32,
        node_id: &str,
    ) -> Result<QualifiedQueryNode> {
        valid_operation_id(node_id)?;
        let member = self
            .hosted_unique_child_members_at(root, parent, slot, prefix, max_members)?
            .into_iter()
            .find(|m| m.owner.node_id == node_id)
            .ok_or_else(unavailable)?;
        let path = parent.join(&member.relative_path);
        let guard = ParentGuard::acquire(&self.backend, &path)?;
        let file = guard.open_placement_file()?;
        let identity = stamp(&file)?.identity;
        let mut matches = 0usize;
        for plan in self.operations.values() {
            let Some(transaction) = &plan.transaction else {
                continue;
            };
            let Some(HostedDestination::UniqueChild(binding)) = &transaction.hosted_destination
            else {
                continue;
            };
            if binding.slot != slot
                || binding.prefix != prefix
                || binding.workspace_id != member.owner.workspace_id
            {
                continue;
            }
            if !self.unique_committed(plan)? {
                continue;
            }
            if transaction
                .births()
                .iter()
                .any(|b| b.node_id == node_id && same_path(Path::new(&b.path), &path))
                && self.unique_births(&plan.operation_id)?.iter().any(|b| {
                    b.node_id == node_id
                        && b.workspace_id == member.owner.workspace_id
                        && b.identity == identity
                })
            {
                matches += 1;
            }
        }
        if matches != 1 {
            return Err(unavailable());
        }
        self.qualified_query_node_at(root, &path, &member.owner, "live")
    }
}
