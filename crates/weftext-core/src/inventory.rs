use super::*;
use std::collections::HashSet;

const MAX_ENTRIES: usize = 5000;
const MAX_INVENTORY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub root: String,
    pub workspace_id: Option<String>,
    pub status: String,
    pub entries: Vec<InventoryEntry>,
    pub warnings: Vec<String>,
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
#[derive(Default)]
struct Rules {
    ignored: HashSet<PathBuf>,
    unmanaged: HashSet<PathBuf>,
    warnings: Vec<String>,
}
impl Core {
    pub fn inventory(&self, root: impl AsRef<Path>) -> Result<Inventory> {
        let root = absolute_path(root.as_ref())?;
        // Qualify and retain the root ancestor closure while browsing; descendants
        // are separately opened with relative guarded handles before reading bytes.
        let anchor = root.join(".weftext-inventory-prospective");
        let _root_guard = ParentGuard::acquire(&anchor)?;
        let workspace = self.workspace(&root)?;
        let rules = read_rules(&root)?;
        let mut result = Inventory {
            root: display(&root),
            workspace_id: workspace.as_ref().map(|w| w.workspace_id.clone()),
            status: if workspace.is_some() {
                "localMvp"
            } else if root.join(".weftext-format").exists() {
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
            let directory_guard =
                match ParentGuard::acquire(&directory.join(".weftext-inventory-prospective")) {
                    Ok(g) => g,
                    Err(e) => {
                        result.complete = false;
                        result
                            .warnings
                            .push(format!("{}: {}", directory.display(), e.message));
                        continue;
                    }
                };
            let (mut children, enumeration_complete) =
                directory_guard.directory_entries(MAX_ENTRIES)?;
            if !enumeration_complete {
                result.complete = false;
                result.warnings.push(format!(
                    "Directory enumeration capped at 5000 entries: {}",
                    directory.display()
                ));
            }
            children.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, attrs) in children {
                let path = directory.join(name);
                if result.entries.len() >= MAX_ENTRIES {
                    result.complete = false;
                    result
                        .warnings
                        .push("Inventory stopped at 5000 entries".into());
                    return Ok(result);
                }
                let relative = path.strip_prefix(&root).unwrap().to_path_buf();
                if internal(&relative) || excluded(&relative, &rules.ignored) {
                    continue;
                }
                let unmanaged = inherited_unmanaged || excluded(&relative, &rules.unmanaged);
                let blocked = attrs & (0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000) != 0;
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
                    result.warnings.push(format!(
                        "Reparse/offline object skipped: {}",
                        path.display()
                    ));
                } else if attrs & 0x10 != 0 {
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
                            .push("Inventory source budget reached (16 MiB)".into());
                    } else if self.check_pending(&path).is_err() {
                        result.complete = false;
                        item.identity_status = "recoveryPending".into();
                    } else {
                        let observed = (|| -> Result<(String, FileIdentity)> {
                            let guard = ParentGuard::acquire(&path)?;
                            let mut file = guard.open_file(false, false)?;
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
                                        .push("Inventory source budget reached (16 MiB)".into());
                                    result.entries.push(item);
                                    continue;
                                }
                                total += source.len();
                                let info = weftext_engine::inspect(&source);
                                let known = self.admitted_binding(&identity);
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
                                        Some(id)
                                            if workspace.as_ref().is_some_and(|w| {
                                                self.admitted(id, &w.workspace_id, &identity)
                                            }) =>
                                        {
                                            "admitted".into()
                                        }
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
                                result
                                    .warnings
                                    .push(format!("{}: {}", path.display(), e.message));
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
        let relative = parent
            .strip_prefix(root)
            .map_err(|_| CoreError::new("outsideRoot", "Target is outside the selected library"))?;
        let rules = read_rules(root)?;
        if !rules.warnings.is_empty() {
            return Err(CoreError::new(
                "unsupportedRules",
                "The rules file cannot be completely interpreted; create is unavailable",
            ));
        }
        if excluded(relative, &rules.ignored) || excluded(relative, &rules.unmanaged) {
            return Err(CoreError::new(
                "excludedBoundary",
                "Node creation is not permitted inside ignored or explicitly unmanaged boundaries",
            ));
        }
        Ok(())
    }
    pub(crate) fn check_subtree_boundary(&self, root: &Path, directory: &Path) -> Result<()> {
        self.check_discovery_boundary(root, directory)?;
        let relative = directory
            .strip_prefix(root)
            .map_err(|_| CoreError::new("outsideRoot", "Source is outside the library"))?;
        let rules = read_rules(root)?;
        if rules
            .ignored
            .iter()
            .chain(&rules.unmanaged)
            .any(|p| p.starts_with(relative))
        {
            return Err(CoreError::new(
                "excludedBoundary",
                "This subtree contains an ignored or explicitly unmanaged boundary",
            ));
        }
        Ok(())
    }
}
fn read_rules(root: &Path) -> Result<Rules> {
    let path = root.join(".weftext-rules");
    if !path.exists() {
        return Ok(Rules::default());
    }
    let guard = ParentGuard::acquire(&path)?;
    let mut file = guard.open_read_guard_file()?;
    let bytes = read_source(&mut file)?;
    Ok(parse_rules(&bytes))
}
fn parse_rules(bytes: &[u8]) -> Rules {
    let mut rules = Rules::default();
    // This MVP intentionally defines an exact-path subset, no .gitignore inheritance.
    let value: serde_json::Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(_) => {
            rules.warnings.push(".weftext-rules is unsupported: this MVP accepts JSON {version:1,ignored:[relative paths],unmanaged:[relative paths]}".into());
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
            .push("Unsupported .weftext-rules format/version".into());
        return rules;
    }
    for (key, set) in [
        ("ignored", &mut rules.ignored),
        ("unmanaged", &mut rules.unmanaged),
    ] {
        let Some(paths) = value[key].as_array() else {
            rules.warnings.push(format!("Missing rules array: {key}"));
            continue;
        };
        for path in paths {
            let Some(text) = path.as_str() else {
                rules.warnings.push(format!("Invalid {key} path"));
                continue;
            };
            let path = PathBuf::from(text.replace('/', "\\"));
            if path.is_absolute()
                || path
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                || text.contains('*')
                || text.is_empty()
            {
                rules
                    .warnings
                    .push(format!("Unsupported rules path: {text}"));
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
        return Err(CoreError::new(
            "unsupportedRules",
            rules.warnings.join("; "),
        ));
    }
    Ok(())
}
fn excluded(path: &Path, set: &HashSet<PathBuf>) -> bool {
    set.iter().any(|p| path.starts_with(p))
}
pub(crate) fn internal(path: &Path) -> bool {
    path.components()
        .next()
        .is_some_and(|c| c.as_os_str().to_string_lossy().starts_with(".weftext-"))
}
