use super::*;
use std::collections::HashSet;
use weftext_messages::UiMessage;

const MAX_ENTRIES: usize = 5000;
const MAX_INVENTORY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub root: String,
    pub workspace_id: Option<String>,
    pub status: String,
    pub entries: Vec<InventoryEntry>,
    pub warnings: Vec<CoreDescription>,
    pub complete: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryEntry {
    pub path: String,
    pub relative_path: String,
    pub name: String,
    pub parent_path: String,
    pub kind: String,
    pub node_id: Option<String>,
    pub identity_status: String,
    pub title: Option<String>,
    pub excerpt: Option<String>,
    pub source: Option<String>,
}
/// An explicitly selected ordinary inventory file. This read-only observation
/// is not an admitted resource identity or an author/write basis.
#[derive(Debug)]
pub struct InventoryResource {
    pub path: String,
    pub bytes: Vec<u8>,
    pub revision: String,
}

#[derive(Default)]
struct Rules {
    ignored: HashSet<PathBuf>,
    unmanaged: HashSet<PathBuf>,
    warnings: Vec<CoreDescription>,
}
impl Core {
    /// Read only an immediate ordinary file that current discovery rules expose
    /// in the selected folder. Hosted callers must use their independently
    /// authorized resource APIs; directory discovery never grants actor access.
    pub fn read_inventory_resource(
        &self,
        root: impl AsRef<Path>,
        directory: impl AsRef<Path>,
        path: impl AsRef<Path>,
        expected_revision: Option<&str>,
    ) -> Result<InventoryResource> {
        let unavailable = || {
            CoreError::new(
                "resourceUnavailable",
                UiMessage::inventory_resource_not_listed(),
            )
        };
        if self.actor.is_some() {
            return Err(unavailable());
        }
        let root = absolute_path(root.as_ref())?;
        let directory = absolute_path(directory.as_ref())?;
        let path = absolute_path(path.as_ref())?;
        if !directory.starts_with(&root)
            || path.parent() != Some(directory.as_path())
            || path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("typ"))
        {
            return Err(unavailable());
        }
        let root_guard = ParentGuard::acquire(
            &self.backend,
            &root.join(".weftext-inventory-resource-root"),
        )?;
        let root_identity = root_guard.directory_identity()?;
        // Qualify the selected root as discovery does, without rescanning every
        // author note and journal admission for each visible thumbnail.
        let _ = self.workspace(&root)?;
        let rules = read_rules(&self.backend, &root)?;
        let relative = path.strip_prefix(&root).map_err(|_| unavailable())?;
        if !rules.warnings.is_empty() || internal(relative) || excluded(relative, &rules.ignored) {
            return Err(unavailable());
        }
        self.check_pending(&path)?;
        let guard = ParentGuard::acquire(&self.backend, &path)?;
        let parent_identity = guard.directory_identity()?;
        // Discovery's same bounded entry metadata predicate applies to this
        // direct folder only. A partial enumeration cannot admit an unseen leaf.
        let (children, _) = guard.directory_entries(MAX_ENTRIES)?;
        if !children.iter().any(|(name, metadata)| {
            path.file_name() == Some(name.as_os_str()) && metadata.is_file()
        }) {
            return Err(unavailable());
        }
        let mut file = guard.open_read_guard_file()?;
        let before = stamp(&file)?;
        self.check_pending_identity(&before.identity)?;
        let bytes = operations::read_blob(&mut file)?;
        if stamp(&file)? != before
            || guard.directory_identity()? != parent_identity
            || root_guard.directory_identity()? != root_identity
        {
            return Err(CoreError::new(
                "resourceChanged",
                UiMessage::inventory_resource_read_changed(),
            ));
        }
        // A discovery-rule edit can revoke this browsing membership while the
        // byte read runs. Recheck it before publishing even though no writer
        // capability or hosted permission is involved.
        let final_rules = read_rules(&self.backend, &root)?;
        if !final_rules.warnings.is_empty() || excluded(relative, &final_rules.ignored) {
            return Err(unavailable());
        }
        let basis = serde_json::to_vec(&(
            display(&root),
            display(&directory),
            display(&path),
            root_identity,
            parent_identity,
            before,
            hash(&bytes),
        ))?;
        let revision = format!("observed-inventory-resource:v1:{}", hash(&basis));
        if expected_revision.is_some_and(|expected| expected != revision) {
            return Err(CoreError::new(
                "resourceVersion",
                UiMessage::inventory_resource_version_changed(),
            ));
        }
        Ok(InventoryResource {
            path: display(&path),
            bytes,
            revision,
        })
    }
    pub fn inventory(&self, root: impl AsRef<Path>) -> Result<Inventory> {
        let root = absolute_path(root.as_ref())?;
        // Qualify and retain the root ancestor closure while browsing; descendants
        // are separately opened with relative guarded handles before reading bytes.
        let anchor = root.join(".weftext-inventory-prospective");
        let _root_guard = ParentGuard::acquire(&self.backend, &anchor)?;
        let workspace = self.workspace(&root)?;
        let rules = read_rules(&self.backend, &root)?;
        // One call-local cut of original committed admissions. Re-reading the
        // entire journal for each source makes one complete inventory quadratic
        // in journal I/O. Every later inventory builds this evidence afresh.
        let mut births = Vec::new();
        for plan in self.committed_plans()? {
            births.extend(self.read_births(&plan.operation_id)?);
        }
        let mut result = Inventory {
            root: display(&root),
            workspace_id: workspace.as_ref().map(|w| w.workspace_id.clone()),
            status: if workspace.is_some() {
                "localMvp"
            } else if self.backend.entry_exists(root.join(".weftext-format"))? {
                "incomplete"
            } else {
                "ordinary"
            }
            .into(),
            entries: Vec::new(),
            warnings: rules.warnings.clone(),
            complete: rules.warnings.is_empty(),
        };
        let mut queue = vec![(root.clone(), false)];
        let mut total = 0;
        while let Some((directory, inherited_unmanaged)) = queue.pop() {
            let directory_guard = match ParentGuard::acquire(
                &self.backend,
                &directory.join(".weftext-inventory-prospective"),
            ) {
                Ok(g) => g,
                Err(e) => {
                    result.complete = false;
                    result.warnings.push(inventory_issue(&directory, e));
                    continue;
                }
            };
            let (mut children, enumeration_complete) =
                directory_guard.directory_entries(MAX_ENTRIES)?;
            if !enumeration_complete {
                result.complete = false;
                result.warnings.push(
                    UiMessage::inventory_directory_enumeration_capped(
                        weftext_messages::params::InventoryDirectoryEnumerationCapped {
                            path: display(&directory),
                        },
                    )
                    .into(),
                );
            }
            children.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, attrs) in children {
                let path = directory.join(name);
                if result.entries.len() >= MAX_ENTRIES {
                    result.complete = false;
                    result
                        .warnings
                        .push(UiMessage::inventory_entries_capped().into());
                    return Ok(result);
                }
                let relative = path.strip_prefix(&root).unwrap().to_path_buf();
                if internal(&relative) || excluded(&relative, &rules.ignored) {
                    continue;
                }
                let unmanaged = inherited_unmanaged || excluded(&relative, &rules.unmanaged);
                let blocked = attrs.is_unsupported();
                let mut item = InventoryEntry {
                    path: display(&path),
                    relative_path: relative.to_string_lossy().replace('\\', "/"),
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    parent_path: display(&directory),
                    kind: "file".into(),
                    node_id: None,
                    identity_status: if unmanaged { "unmanaged" } else { "ordinary" }.into(),
                    title: None,
                    excerpt: None,
                    source: None,
                };
                if blocked {
                    item.kind = "blocked".into();
                    item.identity_status = "unsupported".into();
                    result.complete = false;
                    result.warnings.push(
                        UiMessage::inventory_blocked_object_skipped(
                            weftext_messages::params::InventoryBlockedObjectSkipped {
                                path: display(&path),
                            },
                        )
                        .into(),
                    );
                } else if attrs.is_directory() {
                    item.kind = "directory".into();
                    item.identity_status = "unmanaged".into();
                    queue.push((path.clone(), unmanaged));
                } else if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("typ"))
                {
                    if total >= MAX_INVENTORY_BYTES {
                        result.complete = false;
                        result
                            .warnings
                            .push(UiMessage::inventory_source_budget_reached().into());
                    } else if self.check_pending(&path).is_err() {
                        result.complete = false;
                        item.identity_status = "recoveryPending".into();
                    } else {
                        let observed = (|| -> Result<(String, FileIdentity)> {
                            let guard = ParentGuard::acquire(&self.backend, &path)?;
                            let mut file = guard.open_read_guard_file()?;
                            let bytes = read_source(&mut file)?;
                            let id = stamp(&file)?.identity;
                            self.check_pending_identity(&id)?;
                            Ok((decode(&bytes)?, id))
                        })();
                        match observed {
                            Ok((source, identity)) => {
                                if total + source.len() > MAX_INVENTORY_BYTES {
                                    result.complete = false;
                                    result
                                        .warnings
                                        .push(UiMessage::inventory_source_budget_reached().into());
                                    result.entries.push(item);
                                    continue;
                                }
                                total += source.len();
                                let info = weftext_engine::inspect(&source);
                                let known = births.iter().find(|birth| birth.identity == identity);
                                if let Some(birth) = known {
                                    item.kind = "node".into();
                                    item.node_id = Some(birth.node_id.clone());
                                    item.identity_status = if !is_canonical(&path)
                                        || info.id.as_ref() != Some(&birth.node_id)
                                    {
                                        "bindingConflict"
                                    } else if unmanaged {
                                        "boundaryConflict"
                                    } else if !rules.warnings.is_empty() {
                                        "unsupportedRules"
                                    } else {
                                        "admitted"
                                    }
                                    .into();
                                } else if is_canonical(&path)
                                    && !unmanaged
                                    && rules.warnings.is_empty()
                                {
                                    item.kind = "node".into();
                                    item.node_id = info.id.clone();
                                    item.identity_status = match info.id.as_ref() {
                                        // An ID in source cannot replace an
                                        // absent admitted physical binding.
                                        Some(_) => "unadmitted".into(),
                                        None if info.identity_status == "absent" => {
                                            "missing".into()
                                        }
                                        None => info.identity_status,
                                    };
                                }
                                // Display fallback only, never an authored-title claim.
                                item.excerpt = Some(source.chars().take(180).collect());
                                item.source = Some(source);
                            }
                            Err(e) => {
                                result.complete = false;
                                result.warnings.push(inventory_issue(&path, e));
                            }
                        }
                    }
                }
                result.entries.push(item);
            }
        }
        let mut counts = HashMap::<String, usize>::new();
        for item in &result.entries {
            if let Some(id) = &item.node_id {
                *counts.entry(id.clone()).or_default() += 1;
            }
        }
        for item in &mut result.entries {
            if item.node_id.as_ref().is_some_and(|id| counts[id] > 1) {
                item.identity_status = "duplicate".into();
            }
        }
        result
            .entries
            .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(result)
    }
    pub(crate) fn check_discovery_boundary(&self, root: &Path, parent: &Path) -> Result<()> {
        let relative = parent.strip_prefix(root).map_err(|_| {
            CoreError::new(
                "outsideRoot",
                UiMessage::error_core_inventory_target_outside_selected_library(),
            )
        })?;
        let rules = read_rules(&self.backend, root)?;
        if !rules.warnings.is_empty() {
            return Err(CoreError::new(
                "unsupportedRules",
                UiMessage::error_core_inventory_rules_file_cannot_completely_interpreted_create_unavailable(),
            ));
        }
        if excluded(relative, &rules.ignored) || excluded(relative, &rules.unmanaged) {
            return Err(CoreError::new(
                "excludedBoundary",
                UiMessage::error_core_inventory_node_creation_not_permitted_inside_ignored_or_explicitly_unmanaged_boundaries(),
            ));
        }
        Ok(())
    }
    pub(crate) fn check_subtree_boundary(&self, root: &Path, directory: &Path) -> Result<()> {
        self.check_discovery_boundary(root, directory)?;
        let relative = directory.strip_prefix(root).map_err(|_| {
            CoreError::new(
                "outsideRoot",
                UiMessage::error_core_inventory_source_outside_library(),
            )
        })?;
        let rules = read_rules(&self.backend, root)?;
        if rules
            .ignored
            .iter()
            .chain(&rules.unmanaged)
            .any(|p| p.starts_with(relative))
        {
            return Err(CoreError::new(
                "excludedBoundary",
                UiMessage::error_core_inventory_subtree_contains_ignored_or_explicitly_unmanaged_boundary(),
            ));
        }
        Ok(())
    }
}
fn read_rules(backend: &crate::BackendContext, root: &Path) -> Result<Rules> {
    let path = root.join(".weftext-rules");
    if !backend.entry_exists(&path)? {
        return Ok(Rules::default());
    }
    let guard = ParentGuard::acquire(backend, &path)?;
    let mut file = guard.open_read_guard_file()?;
    let bytes = read_source(&mut file)?;
    Ok(parse_rules(&bytes))
}
fn parse_rules(bytes: &[u8]) -> Rules {
    let mut rules = Rules::default();
    // This MVP intentionally defines an exact-path subset, no .gitignore inheritance.
    let value: serde_json::Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(error) => {
            rules.warnings.push(
                CoreError::new(
                    "unsupportedRules",
                    UiMessage::inventory_rules_json_required(),
                )
                .diagnostic(DiagnosticOrigin::Json, error.to_string())
                .into(),
            );
            return rules;
        }
    };
    if value["version"] != 1
        || value.as_object().is_none_or(|o| {
            o.keys()
                .any(|k| !matches!(k.as_str(), "version" | "ignored" | "unmanaged"))
        })
    {
        rules
            .warnings
            .push(UiMessage::inventory_rules_format_unsupported().into());
        return rules;
    }
    for (key, set) in [
        ("ignored", &mut rules.ignored),
        ("unmanaged", &mut rules.unmanaged),
    ] {
        let Some(paths) = value[key].as_array() else {
            rules.warnings.push(
                UiMessage::inventory_rules_array_missing(
                    weftext_messages::params::InventoryRulesArrayMissing { array: key.into() },
                )
                .into(),
            );
            continue;
        };
        for path in paths {
            let Some(text) = path.as_str() else {
                rules.warnings.push(
                    UiMessage::inventory_rules_path_invalid(
                        weftext_messages::params::InventoryRulesPathInvalid { array: key.into() },
                    )
                    .into(),
                );
                continue;
            };
            #[cfg(windows)]
            let path = PathBuf::from(text.replace('/', "\\"));
            #[cfg(not(windows))]
            let path = PathBuf::from(text);
            if path.is_absolute()
                || path
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                || text.contains('*')
                || text.is_empty()
            {
                rules.warnings.push(
                    UiMessage::inventory_rules_path_unsupported(
                        weftext_messages::params::InventoryRulesPathUnsupported {
                            path: text.into(),
                        },
                    )
                    .into(),
                );
                continue;
            }
            set.insert(path);
        }
    }
    rules
}
/// Validate the same closed exact-path rules format used by discovery.
pub fn validate_library_rules(bytes: &[u8]) -> Result<()> {
    check_size(bytes)?;
    let rules = parse_rules(bytes);
    if !rules.warnings.is_empty() {
        return Err(CoreError::from_descriptions(
            "unsupportedRules",
            UiMessage::inventory_rules_invalid(),
            rules.warnings,
        ));
    }
    Ok(())
}
fn excluded(path: &Path, set: &HashSet<PathBuf>) -> bool {
    set.iter().any(|p| path.starts_with(p))
}
fn inventory_issue(path: &Path, error: CoreError) -> CoreDescription {
    CoreError::new(
        "inventoryUnavailable",
        UiMessage::inventory_path_unavailable(weftext_messages::params::InventoryPathUnavailable {
            path: display(path),
        }),
    )
    .caused_by(error)
    .into()
}
pub(crate) fn internal(path: &Path) -> bool {
    path.components()
        .next()
        .is_some_and(|c| c.as_os_str().to_string_lossy().starts_with(".weftext-"))
}

#[cfg(test)]
mod description_tests {
    use super::*;

    #[test]
    fn rule_failures_keep_independent_typed_descriptions() {
        let bytes = br#"{"version":1,"ignored":[17,"","*.tmp"],"unmanaged":[true]}"#;
        let rules = parse_rules(bytes);
        assert_eq!(rules.warnings.len(), 4);
        assert!(
            rules
                .warnings
                .iter()
                .all(|warning| warning.explanation.key().starts_with("inventory."))
        );
        let error = validate_library_rules(bytes).unwrap_err();
        assert_eq!(error.explanation.key(), "inventory.rulesInvalid");
        let Some(CoreErrorDetails::Aggregate { groups }) = error.details else {
            panic!("Expected independent rule diagnostics");
        };
        let diagnostics = groups
            .iter()
            .filter_map(|group| match group {
                ErrorDetailGroup::Product { diagnostics, .. } => Some(diagnostics),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 4);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|description| description.explanation.key() == "inventory.rulesPathInvalid")
                .count(),
            2
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(
                    |description| description.explanation.key() == "inventory.rulesPathUnsupported"
                )
                .count(),
            2
        );
    }

    #[test]
    fn malformed_rules_keep_original_json_diagnostic_separate() {
        let error = validate_library_rules(b"{ malformed JSON").unwrap_err();
        assert_eq!(error.explanation.key(), "inventory.rulesInvalid");
        let Some(CoreErrorDetails::Aggregate { groups }) = error.details else {
            panic!("Expected retained JSON diagnostic");
        };
        let descriptions = groups
            .iter()
            .filter_map(|group| match group {
                ErrorDetailGroup::Product { diagnostics, .. } => Some(diagnostics),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        let malformed = descriptions
            .iter()
            .find(|description| description.explanation.key() == "inventory.rulesJsonRequired")
            .unwrap();
        assert!(
            malformed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.origin == DiagnosticOrigin::Json
                    && !diagnostic.text.is_empty())
        );
    }
}

#[cfg(test)]
mod rules_namespace_tests {
    use super::*;
    #[test]
    fn exact_rules_use_native_separators_and_reject_parent_traversal() {
        assert!(
            validate_library_rules(br#"{"version":1,"ignored":["../hidden"],"unmanaged":[]}"#)
                .is_err()
        );
        let rules = parse_rules(br#"{"version":1,"ignored":["A/child.typ"],"unmanaged":[]}"#);
        assert!(rules.warnings.is_empty());
        assert!(rules.ignored.contains(&Path::new("A").join("child.typ")));
        #[cfg(target_os = "linux")]
        {
            let distinct = parse_rules(
                br#"{"version":1,"ignored":["A.typ","a.typ","A\\child.typ"],"unmanaged":[]}"#,
            );
            assert!(distinct.warnings.is_empty());
            assert_eq!(distinct.ignored.len(), 3);
            assert!(distinct.ignored.contains(Path::new("A\\child.typ")));
        }
    }
}

#[cfg(all(test, windows))]
mod resource_tests {
    use super::*;

    #[test]
    fn inventory_resource_reads_ordinary_files_without_sessions_or_admission() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("ordinary");
        fs::create_dir(&root).unwrap();
        let path = root.join("original.txt");
        fs::write(&path, b"untouched ordinary file").unwrap();
        let core = Core::new(temp.path().join("private")).unwrap();
        let first = core
            .read_inventory_resource(&root, &root, &path, None)
            .unwrap();
        let second = core
            .read_inventory_resource(&root, &root, &path, Some(&first.revision))
            .unwrap();
        assert_eq!(first.bytes, b"untouched ordinary file");
        assert_eq!(first.revision, second.revision);
        assert!(
            first
                .revision
                .starts_with("observed-inventory-resource:v1:")
        );
        assert!(core.sessions.is_empty());
        assert!(
            core.snapshots.is_empty(),
            "browsing must not accumulate writer bases"
        );
        assert!(!root.join(".weftext-format").exists());
        assert_eq!(fs::read(path).unwrap(), first.bytes);
    }

    #[test]
    fn inventory_resource_requires_visible_immediate_non_author_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("ordinary");
        fs::create_dir(&root).unwrap();
        for name in ["Hidden", "Visible", ".weftext-private"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::write(root.join(name).join("asset.txt"), b"original").unwrap();
        }
        fs::write(root.join("Note.typ"), b"= Author content").unwrap();
        fs::write(temp.path().join("outside.txt"), b"outside").unwrap();
        fs::write(
            root.join(".weftext-rules"),
            br#"{"version":1,"ignored":["Hidden"],"unmanaged":["Visible"]}"#,
        )
        .unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let visible = root.join("Visible");
        assert_eq!(
            core.read_inventory_resource(&root, &visible, visible.join("asset.txt"), None)
                .unwrap()
                .bytes,
            b"original"
        );
        for (directory, path) in [
            (root.clone(), visible.join("asset.txt")),
            (root.join("Hidden"), root.join("Hidden/asset.txt")),
            (
                root.join(".weftext-private"),
                root.join(".weftext-private/asset.txt"),
            ),
            (root.clone(), root.join("Note.typ")),
            (root.clone(), root.join("Visible")),
            (root.clone(), root.join("missing.txt")),
            (temp.path().to_path_buf(), temp.path().join("outside.txt")),
        ] {
            assert!(
                core.read_inventory_resource(&root, directory, path, None)
                    .is_err()
            );
        }
        let actor_read = core.with_actor("authenticated-test-actor", |core| {
            core.read_inventory_resource(&root, &visible, visible.join("asset.txt"), None)
        });
        assert_eq!(actor_read.unwrap_err().code, "resourceUnavailable");
        fs::write(root.join(".weftext-rules"), br#"{"version":99}"#).unwrap();
        assert!(
            core.read_inventory_resource(&root, &visible, visible.join("asset.txt"), None)
                .is_err()
        );
    }

    #[test]
    fn inventory_resource_rejects_changed_bytes_replacements_and_hard_links() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("ordinary");
        let folder = root.join("Folder");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("asset.txt");
        fs::write(&path, b"original").unwrap();
        let core = Core::new(temp.path().join("private")).unwrap();
        let first = core
            .read_inventory_resource(&root, &folder, &path, None)
            .unwrap();
        fs::write(&path, b"changed").unwrap();
        assert_eq!(
            core.read_inventory_resource(&root, &folder, &path, Some(&first.revision))
                .unwrap_err()
                .code,
            "resourceVersion"
        );
        let changed = core
            .read_inventory_resource(&root, &folder, &path, None)
            .unwrap();
        fs::rename(&path, folder.join("retained-original.txt")).unwrap();
        fs::write(&path, &changed.bytes).unwrap();
        assert_eq!(
            core.read_inventory_resource(&root, &folder, &path, Some(&changed.revision))
                .unwrap_err()
                .code,
            "resourceVersion"
        );
        let replaced = core
            .read_inventory_resource(&root, &folder, &path, None)
            .unwrap();
        fs::rename(&folder, root.join("RetainedFolder")).unwrap();
        fs::create_dir(&folder).unwrap();
        fs::write(&path, &replaced.bytes).unwrap();
        assert_eq!(
            core.read_inventory_resource(&root, &folder, &path, Some(&replaced.revision))
                .unwrap_err()
                .code,
            "resourceVersion"
        );
        fs::hard_link(&path, folder.join("alias.txt")).unwrap();
        assert_eq!(
            core.read_inventory_resource(&root, &folder, &path, None)
                .unwrap_err()
                .code,
            "hardLink"
        );
    }
}
