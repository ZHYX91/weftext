//! Explicit, admitted query objects. The public coordinates come from guarded
//! OS handles; neither paths nor copied portable markers establish identity.
use super::*;
use operations::{CapturedFile, Mutation, SnapshotBinding};
use std::collections::BTreeSet;
use weftext_messages::UiMessage;

pub struct QualifiedQueryScope {
    pub target_ref: String,
    pub directory: PathBuf,
    pub lifecycle: String,
    pub witness: QueryScopeWitness,
}
pub struct QualifiedQueryNode {
    pub owner: weftext_knowledge::NodeRef,
    pub lifecycle: String,
    pub revision: Option<String>,
    pub source_hash: String,
    pub coordinate: PathBuf,
    pub witness: QueryNodeWitness,
}
pub struct QueryScopeWitness {
    root: PathBuf,
    path: PathBuf,
    expected: String,
    lifecycle: String,
    directory: FileIdentity,
    dependencies: Vec<SnapshotWitness>,
}
pub struct QueryNodeWitness {
    root: PathBuf,
    path: PathBuf,
    expected: weftext_knowledge::NodeRef,
    lifecycle: String,
    identity: FileIdentity,
    dependencies: Vec<SnapshotWitness>,
}
impl QueryScopeWitness {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.path.as_os_str().len() * 2
            + self.root.as_os_str().len() * 2
            + self.expected.len()
            + self.lifecycle.len()
            + self.dependencies.capacity() * std::mem::size_of::<SnapshotWitness>()
            + self
                .dependencies
                .iter()
                .map(SnapshotWitness::retained_bytes)
                .sum::<usize>()
    }
}
impl QueryNodeWitness {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.path.as_os_str().len() * 2
            + self.root.as_os_str().len() * 2
            + self.expected.workspace_id.len()
            + self.expected.node_id.len()
            + self.lifecycle.len()
            + self.dependencies.capacity() * std::mem::size_of::<SnapshotWitness>()
            + self
                .dependencies
                .iter()
                .map(SnapshotWitness::retained_bytes)
                .sum::<usize>()
    }
}

impl Core {
    /// Qualify one operator-selected directory with its expected stable ref.
    /// Only its selected identity file, root controls and journal are read.
    pub fn qualified_query_scope_at(
        &self,
        root: &Path,
        path: &Path,
        expected_ref: &str,
        lifecycle: &str,
    ) -> Result<QualifiedQueryScope> {
        let root = absolute_path(root)?;
        let path = absolute_path(path)?;
        let workspace = self.require_workspace(&root)?;
        let (kind, workspace_id, id) = crate::directory_identity::parse_ref(expected_ref)?;
        if workspace_id != workspace.workspace_id || !matches!(kind, "node" | "directory") {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_query_scope_scope_reference_and_admitted_workspace_differ(),
            ));
        }
        self.check_pending(&path)?;
        let guard = ParentGuard::acquire(&self.backend, &path.join(".query-scope-prospective"))?;
        let directory_identity = guard.directory_identity()?;
        let coordinate = guard.directory_coordinates()?;
        let root_coordinate =
            ParentGuard::acquire(&self.backend, &root.join(".query-root-prospective"))?
                .directory_coordinates()?;
        let relative = coordinate.strip_prefix(&root_coordinate).map_err(|_| {
            CoreError::new(
                "outsideRoot",
                UiMessage::error_core_query_scope_scope_outside_selected_physical_root(),
            )
        })?;
        self.query_lifecycle_boundary(&root, &coordinate, lifecycle)?;
        let mut dependencies = if kind == "node" {
            let mut source = None;
            for p in self.committed_plans()?.into_iter() {
                for birth in self
                    .read_births(&p.operation_id)?
                    .into_iter()
                    .filter(|b| b.workspace_id == workspace_id && b.node_id == id)
                {
                    for location in self.node_placement_paths(&birth)? {
                        let Some(parent) = Path::new(&location).parent() else {
                            continue;
                        };
                        if crate::history::history_relative_path(parent, &root)
                            .is_some_and(|p| query_same_relative(&p, relative))
                        {
                            let leaf = Path::new(&location).file_name().unwrap();
                            if source.replace(path.join(leaf)).is_some() {
                                return Err(CoreError::new(
                                    "identityPending",
                                    UiMessage::error_core_query_scope_ambiguous_admitted_scope_placement(),
                                ));
                            }
                        }
                    }
                }
            }
            let source = source.ok_or_else(|| {
                CoreError::new(
                    "identityPending",
                    UiMessage::error_core_query_scope_no_original_node_placement_proves_selected_scope(),
                )
            })?;
            let owner = weftext_knowledge::NodeRef {
                workspace_id: workspace_id.into(),
                node_id: id.into(),
            };
            self.qualified_query_node_at(&root, &source, &owner, lifecycle)?
                .witness
                .dependencies
        } else {
            let marker = path.join("weftext.directory.json");
            let identity_file = self.query_file_witness(&marker)?;
            let captured = identity_file.binding.captured.as_ref().ok_or_else(|| {
                CoreError::new(
                    "identityPending",
                    UiMessage::error_core_query_scope_admitted_directory_marker_required(),
                )
            })?;
            crate::directory_identity::validate_directory_marker(&captured.bytes)?;
            let value: serde_json::Value = serde_json::from_slice(&captured.bytes)?;
            if value["directoryId"] != id || identity_file.binding.parent != directory_identity {
                return Err(CoreError::new(
                    "identityPending",
                    UiMessage::error_core_query_scope_directory_marker_and_expected_physical_object_differ(),
                ));
            }
            let places = self.query_directory_placements(workspace_id, id, &directory_identity)?;
            if places.len() != 1
                || !places.iter().any(|p| {
                    crate::history::history_relative_path(Path::new(p), &root)
                        .is_some_and(|p| query_same_relative(&p, relative))
                })
            {
                return Err(CoreError::new(
                    "identityPending",
                    UiMessage::error_core_query_scope_no_committed_directory_placement_proves_exact_object(),
                ));
            }
            let mut proofs = vec![identity_file];
            proofs.extend(self.query_control_witnesses(&root)?);
            proofs
        };
        // Keep one owned source per qualifier; the checked merge later dedups
        // overlapping scopes across a complete hosted save decision.
        dependencies.shrink_to_fit();
        Ok(QualifiedQueryScope {
            target_ref: expected_ref.into(),
            directory: coordinate,
            lifecycle: lifecycle.into(),
            witness: QueryScopeWitness {
                root,
                path,
                expected: expected_ref.into(),
                lifecycle: lifecycle.into(),
                directory: directory_identity,
                dependencies,
            },
        })
    }

    /// Exact live or retained trash main source. Trash keeps its original main
    /// leaf; the operation-UUID container name is never treated as that leaf.
    pub fn qualified_query_node_at(
        &self,
        root: &Path,
        path: &Path,
        expected: &weftext_knowledge::NodeRef,
        lifecycle: &str,
    ) -> Result<QualifiedQueryNode> {
        let root = absolute_path(root)?;
        let path = absolute_path(path)?;
        let workspace = self.require_workspace(&root)?;
        if workspace.workspace_id != expected.workspace_id {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_query_scope_node_reference_belongs_to_another_workspace(),
            ));
        }
        self.check_pending(&path)?;
        let guard = ParentGuard::acquire(&self.backend, &path)?;
        let mut handle = guard.open_file(false, false)?;
        let actual = stamp(&handle)?;
        self.check_pending_identity(&actual.identity)?;
        let coordinate = backend::file_coordinates(&handle)?;
        self.query_lifecycle_boundary(&root, coordinate.parent().unwrap(), lifecycle)?;
        let bytes = read_source(&mut handle)?;
        let birth = self.admitted_binding(&actual.identity)?.ok_or_else(|| {
            CoreError::new(
                "identityPending",
                UiMessage::error_core_query_scope_selected_source_has_no_original_physical_admission(),
            )
        })?;
        if birth.workspace_id != expected.workspace_id
            || birth.node_id != expected.node_id
            || weftext_engine::inspect_metadata(&decode(&bytes)?)
                .id
                .as_ref()
                != Some(&birth.node_id)
        {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_query_scope_current_owner_declaration_and_original_admission_differ(),
            ));
        }
        let places = self.node_placement_paths(&birth)?;
        self.check_query_node_placement_paths(&root, &coordinate, &places)?;
        let captured = CapturedFile {
            path: display(&path),
            bytes,
            stamp: actual,
        };
        let identity = captured.stamp.identity.clone();
        let revision = self.accepted_revision(&captured)?;
        let source_hash = hash(&captured.bytes);
        let main = SnapshotWitness {
            binding: SnapshotBinding {
                path: display(&path),
                parent: guard.directory_identity()?,
                captured: Some(captured),
            },
        };
        drop(handle);
        drop(guard);
        let mut dependencies = vec![main];
        dependencies.extend(self.query_control_witnesses(&root)?);
        Ok(QualifiedQueryNode {
            owner: expected.clone(),
            lifecycle: lifecycle.into(),
            revision,
            source_hash,
            coordinate,
            witness: QueryNodeWitness {
                root,
                path,
                expected: expected.clone(),
                lifecycle: lifecycle.into(),
                identity,
                dependencies,
            },
        })
    }
    pub fn validate_query_scope_witness(
        &self,
        root: &Path,
        witness: &QueryScopeWitness,
    ) -> Result<()> {
        if absolute_path(root)? != witness.root {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_query_scope_scope_witness_belongs_to_another_root(),
            ));
        }
        self.query_validate_dependencies(root, &witness.dependencies)?;
        let qualified = self.qualified_query_scope_at(
            root,
            &witness.path,
            &witness.expected,
            &witness.lifecycle,
        )?;
        if qualified.witness.directory != witness.directory {
            return Err(CoreError::new(
                "staleBase",
                UiMessage::error_core_query_scope_selected_scope_object_changed(),
            ));
        }
        Ok(())
    }
    pub(crate) fn check_query_node_placement_paths(
        &self,
        root: &Path,
        coordinate: &Path,
        places: &BTreeSet<String>,
    ) -> Result<()> {
        let root_coordinate =
            ParentGuard::acquire(&self.backend, &root.join(".query-root-prospective"))?
                .directory_coordinates()?;
        let relative = coordinate.strip_prefix(&root_coordinate).map_err(|_| {
            CoreError::new(
                "outsideRoot",
                UiMessage::error_core_query_scope_source_outside_selected_physical_root(),
            )
        })?;
        if places.len() != 1
            || !places.iter().any(|path| {
                crate::history::history_relative_path(Path::new(path), root)
                    .is_some_and(|path| query_same_relative(&path, relative))
            })
        {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_query_scope_no_committed_node_placement_proves_exact_object(),
            ));
        }
        Ok(())
    }
    pub fn validate_query_node_witness(
        &self,
        root: &Path,
        witness: &QueryNodeWitness,
    ) -> Result<()> {
        if absolute_path(root)? != witness.root {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_query_scope_node_witness_belongs_to_another_root(),
            ));
        }
        self.query_validate_dependencies(root, &witness.dependencies)?;
        let qualified = self.qualified_query_node_at(
            root,
            &witness.path,
            &witness.expected,
            &witness.lifecycle,
        )?;
        if qualified.witness.identity != witness.identity {
            return Err(CoreError::new(
                "staleBase",
                UiMessage::error_core_query_scope_selected_source_object_changed(),
            ));
        }
        Ok(())
    }
    pub fn merge_query_scope_witness(
        pool: &mut Vec<SnapshotWitness>,
        witness: &QueryScopeWitness,
    ) -> Result<()> {
        Self::merge_snapshot_witnesses(pool, witness.dependencies.clone())
    }
    pub fn merge_query_node_witness(
        pool: &mut Vec<SnapshotWitness>,
        witness: &QueryNodeWitness,
    ) -> Result<()> {
        Self::merge_snapshot_witnesses(pool, witness.dependencies.clone())
    }
    pub(crate) fn merge_query_scope_feature_witness(
        pool: &mut Vec<SnapshotWitness>,
        witness: &QueryScopeWitness,
    ) -> Result<()> {
        Self::merge_feature_witnesses(pool, witness.dependencies.clone())
    }
    pub(crate) fn merge_query_node_feature_witness(
        pool: &mut Vec<SnapshotWitness>,
        witness: &QueryNodeWitness,
    ) -> Result<()> {
        Self::merge_feature_witnesses(pool, witness.dependencies.clone())
    }
    fn query_validate_dependencies(
        &self,
        root: &Path,
        dependencies: &[SnapshotWitness],
    ) -> Result<()> {
        for witness in dependencies {
            self.validate_snapshot_witness(root, Path::new(&witness.binding.path), witness)
                .inspect_err(|error| {
                    let kind = match Path::new(&witness.binding.path)
                        .file_name()
                        .and_then(|name| name.to_str())
                    {
                        Some(".weftext-rules") => "rules",
                        Some(".weftext-workspace.json") => "workspace",
                        Some(".weftext-format") => "format",
                        Some(".weftext-mvp-commit.json") => "commit",
                        _ => "other",
                    };
                    resource_diagnostics::error(
                        &format!("query.control.{kind}"),
                        &witness.binding.path,
                        error,
                    );
                })?;
        }
        Ok(())
    }
    fn query_file_witness(&self, path: &Path) -> Result<SnapshotWitness> {
        self.check_pending(path)?;
        let guard = ParentGuard::acquire(&self.backend, path)?;
        let captured = match guard.open_file(false, false) {
            Ok(mut handle) => {
                let actual = stamp(&handle)?;
                self.check_pending_identity(&actual.identity)?;
                Some(CapturedFile {
                    path: display(path),
                    bytes: read_source(&mut handle)?,
                    stamp: actual,
                })
            }
            Err(e) if e.code == "missing" => None,
            Err(e) => return Err(e),
        };
        Ok(SnapshotWitness {
            binding: SnapshotBinding {
                path: display(path),
                parent: guard.directory_identity()?,
                captured,
            },
        })
    }
    fn query_control_witnesses(&self, root: &Path) -> Result<Vec<SnapshotWitness>> {
        let rules = self.query_file_witness(&root.join(".weftext-rules"))?;
        if rules.binding.captured.is_none() && self.query_rules_ever_committed(root)? {
            return Err(CoreError::new(
                "rulesUnavailable",
                UiMessage::error_core_query_scope_previously_committed_library_rules_missing_absence_not_legacy_default(),
            ));
        }
        let mut controls = vec![rules];
        for name in [
            ".weftext-workspace.json",
            ".weftext-format",
            ".weftext-mvp-commit.json",
        ] {
            controls.push(self.query_file_witness(&root.join(name))?);
        }
        Ok(controls)
    }
    pub(crate) fn query_rules_ever_committed(&self, root: &Path) -> Result<bool> {
        let workspace = self.require_workspace(root)?.workspace_id;
        for p in self
            .operations
            .values()
            .map(|p| -> Result<_> {
                Ok(
                    if {
                        self.committed(&p.operation_id)?
                            && p.workspace_id.as_deref() == Some(&workspace)
                    } {
                        Some(p)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            let mut parents = Vec::new();
            if p.files.iter().any(|(name, _)| name == ".weftext-rules") {
                parents.push(Path::new(&p.path));
            }
            if let Some(t) = &p.transaction {
                let files = match &t.mutation {
                    Mutation::Update { files } | Mutation::CreateTree { files, .. } => {
                        files.iter().collect::<Vec<_>>()
                    }
                    Mutation::Relocate { updates, .. } => updates.iter().collect(),
                    Mutation::Compound {
                        updates,
                        created_files,
                        ..
                    } => updates.iter().chain(created_files).collect(),
                };
                parents.extend(
                    files
                        .iter()
                        .filter(|f| {
                            Path::new(&f.path).file_name().is_some_and(|n| {
                                n.to_string_lossy().eq_ignore_ascii_case(".weftext-rules")
                            })
                        })
                        .filter_map(|f| Path::new(&f.path).parent()),
                );
            }
            for parent in parents {
                if query_same_relative(parent, root)
                    && backend::same_directory(&self.backend, parent, root)?
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    fn query_lifecycle_boundary(
        &self,
        root: &Path,
        coordinate: &Path,
        lifecycle: &str,
    ) -> Result<()> {
        if !matches!(lifecycle, "live" | "trash") {
            return Err(CoreError::new(
                "invalidLifecycle",
                UiMessage::error_core_query_scope_choose_live_or_trash_explicitly(),
            ));
        }
        let root_guard =
            ParentGuard::acquire(&self.backend, &root.join(".query-root-prospective"))?;
        let root_coordinate = root_guard.directory_coordinates()?;
        let relative = coordinate.strip_prefix(&root_coordinate).map_err(|_| {
            CoreError::new(
                "outsideRoot",
                UiMessage::error_core_query_scope_selected_physical_object_outside_library(),
            )
        })?;
        let trash = relative.components().next().is_some_and(|c| {
            c.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(".weftext-trash")
        });
        if trash != (lifecycle == "trash") {
            return Err(CoreError::new(
                "lifecycleMismatch",
                UiMessage::error_core_query_scope_selected_object_in_another_lifecycle(),
            ));
        }
        self.check_discovery_boundary(root, &root.join(relative))?;
        if trash {
            let parts = relative.components().take(2).collect::<PathBuf>();
            if parts.components().count() != 2 {
                return Err(CoreError::new(
                    "identityPending",
                    UiMessage::error_core_query_scope_select_exact_retained_trash_tree_not_trash_container(),
                ));
            }
            let tree = root.join(&parts);
            let tree_guard =
                ParentGuard::acquire(&self.backend, &tree.join(".query-trash-prospective"))?;
            let tree_identity = tree_guard.directory_identity()?;
            let owned = self.original_trash_roots(root)?.iter().any(|entry| {
                crate::history::history_relative_path(&entry.path, root)
                    .is_some_and(|path| query_same_relative(&path, &parts))
                    && entry.identity == tree_identity
            });
            if !owned {
                return Err(CoreError::new(
                    "identityPending",
                    UiMessage::error_core_query_scope_trash_tree_lacks_original_committed_physical_placement(),
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn query_directory_placements(
        &self,
        workspace: &str,
        id: &str,
        identity: &FileIdentity,
    ) -> Result<BTreeSet<String>> {
        let mut places = BTreeSet::new();
        for (path, operation) in self.directory_admission_paths(workspace, id, identity)? {
            let seq = self
                .operations
                .get(&operation)
                .map(|p| self.history_commit_time(p))
                .transpose()?
                .and_then(|v| v.1);
            places.extend(self.replay_placements(
                BTreeSet::from([path]),
                seq,
                identity,
                None,
                Some(id),
            )?);
        }
        Ok(places)
    }
}
fn query_same_relative(a: &Path, b: &Path) -> bool {
    a.components().count() == b.components().count()
        && crate::history::history_relative_path(a, b).is_some()
}
