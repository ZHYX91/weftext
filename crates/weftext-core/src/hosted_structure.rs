//! Explicit hosted tree capabilities and original structural receipts. These
//! reuse the original frozen operation, physical admissions and placement log.
use super::*;
use crate::operations::{
    CapturedDirectory, CapturedFile, Mutation, SnapshotBinding, capture_optional, snapshot_tree,
    sorted_entries,
};
use std::collections::BTreeSet;

/// Captured only after the host authorizes complete subtree read/export AND
/// schema use. No portable path, source-read grant or copy action is that grant.
pub struct FeatureTreeWitness {
    pub(crate) root: PathBuf,
    pub(crate) source: PathBuf,
    pub(crate) files: Vec<CapturedFile>,
    pub(crate) directories: Vec<CapturedDirectory>,
    pub(crate) dependencies: Vec<SnapshotWitness>,
    pub(crate) schema: SnapshotWitness,
    owner: QualifiedQueryNode,
}
impl FeatureTreeWitness {
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
    pub fn source_hash(&self) -> &str {
        &self.owner.source_hash
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.root.as_os_str().len() * 2
            + self.source.as_os_str().len() * 2
            + self.owner.witness.retained_bytes()
            + self.owner.owner.workspace_id.capacity()
            + self.owner.owner.node_id.capacity()
            + self.owner.lifecycle.capacity()
            + self.owner.revision.as_ref().map_or(0, String::capacity)
            + self.owner.source_hash.capacity()
            + self.owner.coordinate.as_os_str().len() * 2
            + self.schema.retained_bytes()
            + self.files.capacity() * std::mem::size_of::<CapturedFile>()
            + self
                .files
                .iter()
                .map(|f| f.path.capacity() + f.bytes.capacity())
                .sum::<usize>()
            + self.directories.capacity() * std::mem::size_of::<CapturedDirectory>()
            + self
                .directories
                .iter()
                .map(|d| {
                    d.path.capacity()
                        + d.entries.capacity() * std::mem::size_of::<String>()
                        + d.entries.iter().map(String::capacity).sum::<usize>()
                })
                .sum::<usize>()
            + self.dependencies.capacity() * std::mem::size_of::<SnapshotWitness>()
            + self
                .dependencies
                .iter()
                .map(SnapshotWitness::retained_bytes)
                .sum::<usize>()
    }
}

/// Trusted host-only locator facts. Never serialize server paths or the internal
/// trash operation ID directly into a public DTO. Current delivery also needs
/// actor_feature_structure_result_at and current host grants/configuration.
pub struct ActorStructureReceipt {
    pub root: PathBuf,
    pub target: PathBuf,
    pub nodes: Vec<ActorStructureNode>,
    pub trash_operation_id: Option<String>,
}
pub struct ActorStructureNode {
    pub node_ref: weftext_knowledge::NodeRef,
    pub source: PathBuf,
    pub lifecycle: String,
}

fn same_path(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b).is_some_and(|p| p.as_os_str().is_empty())
}

impl Core {
    /// Resolve one preconfigured future structural source slot from its real
    /// original producer/relocation fact. Ordinary admitted names, raw copies,
    /// replacement files and unrelated producers do not acquire slot authority.
    pub fn qualified_structure_slot_at(
        &self,
        root: &Path,
        path: &Path,
        resource: &str,
    ) -> Result<QualifiedQueryNode> {
        self.backend.check_path(&self.data_dir)?;
        valid_resource(resource)?;
        let root = absolute_path(root)?;
        let path = absolute_path(path)?;
        let workspace = self.require_workspace(&root)?;
        self.check_pending(&path)?;
        let identity = {
            let file = ParentGuard::acquire(&self.backend, &path)?.open_placement_file()?;
            stamp(&file)?.identity
        };
        self.check_pending_identity(&identity)?;
        let mut expected = None;
        for plan in self
            .operations
            .values()
            .map(|plan| -> Result<_> {
                Ok(
                    if {
                        self.committed(&plan.operation_id)?
                            && plan.workspace_id.as_deref() == Some(&workspace.workspace_id)
                    } {
                        Some(plan)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            let Some(transaction) = &plan.transaction else {
                continue;
            };
            if transaction.hosted_destination.as_deref() != Some(resource) {
                continue;
            }
            let node = match (&transaction.request.intent, &transaction.mutation) {
                (
                    OperationIntent::NewNode { .. }
                    | OperationIntent::CopyNode { .. }
                    | OperationIntent::TemplateNode { .. },
                    Mutation::CreateTree { births, .. },
                ) => {
                    let actual = self.read_births(&plan.operation_id)?;
                    births
                        .iter()
                        .filter(|b| {
                            same_path(Path::new(&b.path), &path) && is_canonical(Path::new(&b.path))
                        })
                        .find(|created| {
                            actual.iter().any(|b| {
                                b.node_id == created.node_id
                                    && b.workspace_id == workspace.workspace_id
                                    && b.identity == identity
                            })
                        })
                        .map(|b| b.node_id.clone())
                }
                (
                    OperationIntent::RenameNode { .. }
                    | OperationIntent::MoveNode { .. }
                    | OperationIntent::RestoreNode { .. },
                    Mutation::Relocate {
                        target,
                        new_leaf,
                        leaf_identity,
                        ..
                    },
                ) if *leaf_identity == identity
                    && same_path(&Path::new(target).join(new_leaf), &path) =>
                {
                    transaction
                        .preview
                        .node_map
                        .values()
                        .next()
                        .cloned()
                        .filter(|_| transaction.preview.node_map.len() == 1)
                }
                _ => None,
            };
            if let Some(node_id) = node {
                let found = weftext_knowledge::NodeRef {
                    workspace_id: workspace.workspace_id.clone(),
                    node_id,
                };
                if expected.as_ref().is_some_and(|prior| prior != &found) {
                    return Err(CoreError::new(
                        "identityPending",
                        "Structural slot producer is ambiguous",
                    ));
                }
                expected = Some(found);
            }
        }
        self.qualified_query_node_at(
            &root,
            &path,
            &expected.ok_or_else(|| {
                CoreError::new(
                    "identityPending",
                    "This slot has no exact committed structural producer",
                )
            })?,
            "live",
        )
    }
    /// Read no ambient inventory: only the pre-authorized selected source tree,
    /// its root controls and exact transfer schema. The host must charge the
    /// opaque retained bytes to its shared request budget before keeping it.
    pub fn capture_feature_tree(
        &self,
        root: &Path,
        source_directory: &Path,
        expected: &weftext_knowledge::NodeRef,
    ) -> Result<FeatureTreeWitness> {
        let root = absolute_path(root)?;
        let root_coordinate =
            ParentGuard::acquire(&self.backend, &root.join(".feature-root-prospective"))?
                .directory_coordinates()?;
        let source_coordinate = ParentGuard::acquire(
            &self.backend,
            &source_directory.join(".feature-tree-prospective"),
        )?
        .directory_coordinates()?;
        let source = root.join(
            source_coordinate
                .strip_prefix(&root_coordinate)
                .map_err(|_| {
                    CoreError::new("outsideRoot", "Selected tree is outside the physical root")
                })?,
        );
        let leaf = source.join(format!(
            "{}.typ",
            source.file_name().unwrap().to_string_lossy()
        ));
        let owner = self.qualified_query_node_at(&root, &leaf, expected, "live")?;
        self.check_subtree_boundary(&root, &source)?;
        let mut files = vec![];
        let mut directories = vec![];
        snapshot_tree(&self.backend, &source, &mut files, &mut directories, false)?;
        if files.len() + directories.len() > 5000 {
            return Err(CoreError::new(
                "budget",
                "Tree capability exceeds 5000 members",
            ));
        }
        if !files
            .iter()
            .any(|f| same_path(Path::new(&f.path), &leaf) && hash(&f.bytes) == owner.source_hash)
        {
            return Err(CoreError::new(
                "scopeChanged",
                "Selected owner changed during tree capture",
            ));
        }
        let mut dependencies = vec![];
        for name in [
            ".weftext-workspace.json",
            ".weftext-format",
            ".weftext-mvp-commit.json",
            ".weftext-rules",
        ] {
            dependencies.push(self.feature_file_witness(&root.join(name))?);
        }
        let schema = self.feature_file_witness(&root.join(".weftext-schema.json"))?;
        let witness = FeatureTreeWitness {
            root,
            source,
            files,
            directories,
            dependencies,
            schema,
            owner,
        };
        self.validate_feature_tree_witness(&witness.root, &witness)?;
        Ok(witness)
    }
    fn feature_file_witness(&self, path: &Path) -> Result<SnapshotWitness> {
        self.check_pending(path)?;
        let guard = ParentGuard::acquire(&self.backend, path)?;
        let captured = capture_optional(&self.backend, path)?;
        if let Some(f) = &captured {
            check_size(&f.bytes)?;
            self.check_pending_identity(&f.stamp.identity)?;
        }
        Ok(SnapshotWitness {
            binding: SnapshotBinding {
                path: display(path),
                parent: guard.directory_identity()?,
                captured,
            },
        })
    }
    pub fn validate_feature_tree_witness(
        &self,
        root: &Path,
        witness: &FeatureTreeWitness,
    ) -> Result<()> {
        if !same_path(&absolute_path(root)?, &witness.root) {
            return Err(CoreError::new(
                "outsideRoot",
                "Tree capability belongs to another root",
            ));
        }
        self.validate_query_node_witness(root, &witness.owner.witness)?;
        for dependency in witness
            .dependencies
            .iter()
            .chain(std::iter::once(&witness.schema))
        {
            self.validate_snapshot_witness(root, Path::new(&dependency.binding.path), dependency)?;
        }
        for f in &witness.files {
            self.check_pending(Path::new(&f.path))?;
            let actual = capture_optional(&self.backend, Path::new(&f.path))?.ok_or_else(|| {
                CoreError::new("scopeChanged", "Authorized tree member disappeared")
            })?;
            if actual.stamp != f.stamp || actual.bytes != f.bytes {
                return Err(CoreError::new(
                    "staleBase",
                    "Authorized tree member changed",
                ));
            }
        }
        for d in &witness.directories {
            let guard = ParentGuard::acquire(
                &self.backend,
                &Path::new(&d.path).join(".feature-tree-prospective"),
            )?;
            let (entries, complete) = guard.directory_entries(5000)?;
            let entries = sorted_entries(entries, Path::new(&d.path));
            if !complete || guard.directory_identity()? != d.identity || entries != d.entries {
                return Err(CoreError::new(
                    "scopeChanged",
                    "Authorized tree membership changed",
                ));
            }
        }
        Ok(())
    }
    /// Build the original transfer inventory from already-authorized captured
    /// sources only. Missing external targets stay unavailable; never scan the
    /// rest of the library or infer a current object from its UUID/path.
    pub(crate) fn feature_copy_inventory(
        &self,
        root: &Path,
        files: &[CapturedFile],
    ) -> Result<Inventory> {
        let workspace = self.require_workspace(root)?;
        let mut entries = vec![];
        let mut ids = BTreeSet::new();
        let mut seen_files: Vec<&CapturedFile> = vec![];
        for f in files.iter().filter(|f| {
            let path = Path::new(&f.path);
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("typ"))
                && is_canonical(path)
        }) {
            if let Some(original) = seen_files
                .iter()
                .find(|original| same_path(Path::new(&original.path), Path::new(&f.path)))
            {
                if original.stamp != f.stamp || original.bytes != f.bytes {
                    return Err(CoreError::new(
                        "staleBase",
                        "Competing captured canonical versions",
                    ));
                }
                continue;
            }
            seen_files.push(f);
            let birth = self.admitted_binding(&f.stamp.identity)?.ok_or_else(|| {
                CoreError::new(
                    "identityConflict",
                    "Captured canonical source is not physically admitted",
                )
            })?;
            if birth.workspace_id != workspace.workspace_id
                || weftext_engine::inspect_metadata(&decode(&f.bytes)?)
                    .id
                    .as_deref()
                    != Some(&birth.node_id)
                || !ids.insert(birth.node_id.clone())
            {
                return Err(CoreError::new(
                    "identityConflict",
                    "Captured source declaration or identity is ambiguous",
                ));
            }
            let places = self.node_placement_paths(&birth)?;
            if places.len() != 1
                || !places
                    .iter()
                    .any(|p| same_path(Path::new(p), Path::new(&f.path)))
            {
                return Err(CoreError::new(
                    "identityConflict",
                    "Captured source lacks exact committed placement",
                ));
            }
            let path = Path::new(&f.path);
            entries.push(InventoryEntry {
                path: f.path.clone(),
                relative_path: path
                    .strip_prefix(root)
                    .map_err(|_| {
                        CoreError::new("outsideRoot", "Captured source belongs outside root")
                    })?
                    .to_string_lossy()
                    .into_owned(),
                name: path.file_name().unwrap().to_string_lossy().into_owned(),
                parent_path: display(path.parent().unwrap()),
                kind: "node".into(),
                node_id: Some(birth.node_id),
                identity_status: "admitted".into(),
                title: None,
                excerpt: None,
                source: None,
            });
        }
        Ok(Inventory {
            root: display(root),
            workspace_id: Some(workspace.workspace_id),
            status: "authorizedTree".into(),
            entries,
            warnings: vec![],
            complete: true,
        })
    }
    pub fn actor_feature_structure_receipt(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorStructureReceipt>> {
        self.backend.check_path(&self.data_dir)?;
        let Some(record) = self.actor_feature_record(resource, operation)? else {
            return Ok(None);
        };
        let Some(status) = self.actor_feature_status(resource, operation)? else {
            return Ok(None);
        };
        if status.status != "committed" {
            return Ok(None);
        }
        let plan = &self.operations[&record.core_operation_id];
        let transaction = plan.transaction.as_ref().unwrap();
        let workspace = plan.workspace_id.as_ref().ok_or_else(conflict)?;
        let mut nodes = vec![];
        let (target, trash) = match &transaction.mutation {
            Mutation::CreateTree { target, births, .. } => {
                let actual = self.read_births(&record.core_operation_id)?;
                for birth in births.iter().filter(|b| is_canonical(Path::new(&b.path))) {
                    if actual
                        .iter()
                        .filter(|b| &b.workspace_id == workspace && b.node_id == birth.node_id)
                        .count()
                        != 1
                    {
                        return Err(CoreError::new(
                            "identityPending",
                            "Original producer birth evidence is unavailable",
                        ));
                    }
                    nodes.push(ActorStructureNode {
                        node_ref: weftext_knowledge::NodeRef {
                            workspace_id: workspace.clone(),
                            node_id: birth.node_id.clone(),
                        },
                        source: birth.path.clone().into(),
                        lifecycle: "live".into(),
                    });
                }
                (target, false)
            }
            Mutation::Relocate {
                target, new_leaf, ..
            } => {
                let trash = transaction.is_trash();
                for node_ref in status.node_refs {
                    nodes.push(ActorStructureNode {
                        node_ref,
                        source: Path::new(target).join(new_leaf),
                        lifecycle: if trash { "trash" } else { "live" }.into(),
                    });
                }
                (target, trash)
            }
            _ => {
                return Err(CoreError::new(
                    "operationConflict",
                    "Original feature is not a structural producer",
                ));
            }
        };
        Ok(Some(ActorStructureReceipt {
            root: transaction.request.root.clone().into(),
            target: target.into(),
            nodes,
            trash_operation_id: trash.then_some(record.core_operation_id),
        }))
    }
    pub fn actor_feature_structure_result_at(
        &self,
        resource: &str,
        root: &Path,
        path: &Path,
        operation: &str,
        expected: &weftext_knowledge::NodeRef,
    ) -> Result<QualifiedQueryNode> {
        self.backend.check_path(&self.data_dir)?;
        let receipt = self
            .actor_feature_structure_receipt(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    "Original structural operation is not committed",
                )
            })?;
        if !same_path(root, &receipt.root) {
            return Err(conflict());
        }
        let node = receipt
            .nodes
            .iter()
            .find(|n| &n.node_ref == expected && same_path(path, &n.source))
            .ok_or_else(conflict)?;
        let qualified = self.qualified_query_node_at(root, path, expected, &node.lifecycle)?;
        let record = self.actor_feature_record(resource, operation)?.unwrap();
        let plan = &self.operations[&record.core_operation_id];
        let guard = ParentGuard::acquire(&self.backend, path)?;
        let identity = {
            let file = guard.open_placement_file()?;
            stamp(&file)?.identity
        };
        let original = match &plan.transaction.as_ref().unwrap().mutation {
            Mutation::CreateTree { .. } => {
                self.read_births(&record.core_operation_id)?
                    .iter()
                    .any(|b| {
                        b.node_id == expected.node_id
                            && b.workspace_id == expected.workspace_id
                            && b.identity == identity
                    })
            }
            Mutation::Relocate { leaf_identity, .. } => *leaf_identity == identity,
            _ => false,
        };
        if !original {
            return Err(CoreError::new(
                "identityPending",
                "Current owner is not the original physical receipt object",
            ));
        }
        Ok(qualified)
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_structure_tests.rs"]
mod tests;
