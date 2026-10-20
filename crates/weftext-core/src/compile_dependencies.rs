//! Read-only qualification of the files actually consulted by a frozen native
//! compilation. Original cut evidence is consumed, never replaced by a fresh
//! snapshot. These witnesses cannot authorize a save or be deserialized.
use super::*;
use crate::backend::{namespace_path_compare, namespace_relative_path};
use crate::operations::{CapturedDirectory, SnapshotBinding, operation_read_control, read_blob};
use std::cmp::Ordering;

const MAX_DEPENDENCIES: usize = 4096;
const MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileDependencyKind {
    Source,
    File,
    DerivedSource,
    DerivedFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileDependencyStatus {
    Loaded,
    Unavailable,
}

/// Construct only from the trusted compiler's actual dependency report, never
/// from an HTTP path/exclusion list. Derived kinds are emitted only for the
/// exact FileId in that same world's producer-owned overrides.
#[derive(Debug, Clone)]
pub struct CompileDependency {
    path: PathBuf,
    kind: CompileDependencyKind,
    status: CompileDependencyStatus,
}
impl CompileDependency {
    pub fn new(path: &str, kind: &str, status: &str) -> Result<Self> {
        let path = relative_path(path)?;
        let kind = match kind {
            "source" => CompileDependencyKind::Source,
            "file" => CompileDependencyKind::File,
            "derived-source" => CompileDependencyKind::DerivedSource,
            "derived-file" => CompileDependencyKind::DerivedFile,
            _ => {
                return Err(CoreError::new(
                    "invalidDependency",
                    "Unknown native dependency kind",
                ));
            }
        };
        let status = match status {
            "loaded" => CompileDependencyStatus::Loaded,
            "unavailable" => CompileDependencyStatus::Unavailable,
            _ => {
                return Err(CoreError::new(
                    "invalidDependency",
                    "Unknown native dependency status",
                ));
            }
        };
        Ok(Self { path, kind, status })
    }
    fn derived(&self) -> bool {
        matches!(
            self.kind,
            CompileDependencyKind::DerivedSource | CompileDependencyKind::DerivedFile
        )
    }
}

pub(crate) struct CompileDependencyCut {
    root: PathBuf,
    directories: Vec<CapturedDirectory>,
}

#[derive(Clone)]
struct AbsentSlot {
    path: PathBuf,
    parent: FileIdentity,
}
#[derive(Clone)]
struct DirectoryInput {
    path: PathBuf,
    identity: FileIdentity,
    parent: FileIdentity,
}

/// Only selected original bindings survive in this owned read witness. Missing
/// slots are observations rechecked at the final decision, not sustained locks
/// against creation. The main Draft/serial and style selection remain their
/// independent consumers' responsibility.
#[derive(Clone)]
pub struct CompileDependencyWitness {
    root: PathBuf,
    root_identity: FileIdentity,
    files: Vec<SnapshotBinding>,
    absent: Vec<AbsentSlot>,
    directories: Vec<DirectoryInput>,
}
impl std::fmt::Debug for CompileDependencyWitness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompileDependencyWitness")
            .field("root", &self.root)
            .field("files", &self.files.len())
            .field("absent", &self.absent.len())
            .field("directories", &self.directories.len())
            .finish()
    }
}
impl CompileDependencyWitness {
    pub fn dependency_count(&self) -> usize {
        self.files.len() + self.absent.len() + self.directories.len()
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.root.as_os_str().len() * 2
            + self
                .files
                .iter()
                .map(|file| {
                    file.path.capacity()
                        + file
                            .captured
                            .as_ref()
                            .map_or(0, |f| f.bytes.capacity() + f.path.capacity())
                })
                .sum::<usize>()
            + self
                .absent
                .iter()
                .map(|slot| slot.path.as_os_str().len() * 2)
                .sum::<usize>()
            + self
                .directories
                .iter()
                .map(|dir| dir.path.as_os_str().len() * 2)
                .sum::<usize>()
    }
}

fn stale(message: &str) -> CoreError {
    CoreError::new("staleBase", message)
}

fn relative_path(text: &str) -> Result<PathBuf> {
    if text.is_empty() || text.len() > 4096 || text.chars().any(|c| c.is_control() || c == ':') {
        return Err(CoreError::new(
            "invalidPath",
            "Native dependency must have one bounded project-relative path",
        ));
    }
    let text = text.replace('\\', "/");
    if text
        .split('/')
        .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(CoreError::new(
            "invalidPath",
            "Native dependency cannot contain empty or dot path components",
        ));
    }
    let path = PathBuf::from(text);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(CoreError::new(
            "invalidPath",
            "Native dependency must stay in its frozen project",
        ));
    }
    Ok(path)
}

fn equal_path(left: &Path, right: &Path) -> Result<bool> {
    Ok(namespace_path_compare(left, right)? == Ordering::Equal)
}

fn sort_paths<T>(values: &mut [T], path: impl Fn(&T) -> &Path) -> Result<()> {
    let mut error = None;
    values.sort_by(
        |left, right| match namespace_path_compare(path(left), path(right)) {
            Ok(order) => order,
            Err(problem) => {
                error = Some(problem);
                Ordering::Equal
            }
        },
    );
    error.map_or(Ok(()), Err)
}

fn find_path<'a, T>(
    values: &'a [T],
    target: &Path,
    path: impl Fn(&T) -> &Path,
) -> Result<Option<&'a T>> {
    let (mut low, mut high) = (0, values.len());
    while low < high {
        let middle = low + (high - low) / 2;
        match namespace_path_compare(path(&values[middle]), target)? {
            Ordering::Less => low = middle + 1,
            Ordering::Greater => high = middle,
            Ordering::Equal => return Ok(Some(&values[middle])),
        }
    }
    Ok(None)
}

fn same_binding(left: &SnapshotBinding, right: &SnapshotBinding) -> bool {
    left.parent == right.parent
        && left.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
            == right.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
}

impl Core {
    /// Called only at the end of admitted_compile_snapshot while its original
    /// file/directory guards still coexist. Ordinary admitted cuts lack this
    /// marker and cannot manufacture negative filesystem evidence.
    pub(crate) fn retain_compile_dependency_cut(
        &mut self,
        root: &Path,
        cut_id: &str,
        directories: &[CapturedDirectory],
    ) -> Result<()> {
        let root = absolute_path(root)?;
        if !self.admitted_cuts.contains_key(cut_id) || directories.len() > 5000 {
            return Err(CoreError::new(
                "cutExpired",
                "The original compile cut is unavailable",
            ));
        }
        self.compile_dependency_cuts
            .retain(|id, _| self.admitted_cuts.contains_key(id));
        let mut directories = directories.to_vec();
        sort_paths(&mut directories, |directory| Path::new(&directory.path))?;
        if find_path(&directories, &root, |directory| Path::new(&directory.path))?.is_none() {
            return Err(CoreError::new(
                "cutExpired",
                "Compile cut lacks its original directory root",
            ));
        }
        self.compile_dependency_cuts
            .insert(cut_id.into(), CompileDependencyCut { root, directories });
        Ok(())
    }

    /// Release private read evidence only. This is idempotent and never removes
    /// a file, directory, Plan, author Draft, operation or admission fact.
    pub fn release_compile_dependency_cut(&mut self, cut_id: &str) {
        if self.compile_dependency_cuts.remove(cut_id).is_some()
            && let Some(cut) = self.admitted_cuts.remove(cut_id)
        {
            for base in cut.bases {
                self.snapshots.remove(&base);
            }
        }
    }

    /// Consume exactly this original compile cut, including on failure. A caller
    /// can retain the resulting small witness across awaits and validate it at
    /// the final locked decision. No newly captured snapshot can repair it.
    pub fn take_compile_dependency_witness(
        &mut self,
        root: &Path,
        cut_id: &str,
        dependencies: &[CompileDependency],
    ) -> Result<CompileDependencyWitness> {
        let result = self.compile_dependency_witness(root, cut_id, dependencies);
        self.release_compile_dependency_cut(cut_id);
        result
    }

    fn compile_dependency_witness(
        &self,
        root: &Path,
        cut_id: &str,
        dependencies: &[CompileDependency],
    ) -> Result<CompileDependencyWitness> {
        let root = absolute_path(root)?;
        self.require_workspace(&root)?;
        let raw = self.compile_dependency_cuts.get(cut_id).ok_or_else(|| {
            CoreError::new("cutExpired", "Use the original complete native compile cut")
        })?;
        if !equal_path(&raw.root, &root)? {
            return Err(CoreError::new(
                "outsideRoot",
                "The original compile cut belongs to another library",
            ));
        }
        let cut = self
            .admitted_cuts
            .get(cut_id)
            .ok_or_else(|| CoreError::new("cutExpired", "The original admitted cut expired"))?;
        if dependencies.len() > MAX_DEPENDENCIES {
            return Err(CoreError::new(
                "budget",
                "Too many actual native dependency observations",
            ));
        }
        let mut bindings = cut
            .bases
            .iter()
            .map(|base| {
                self.snapshots.get(base).ok_or_else(|| {
                    CoreError::new("cutExpired", "An original compile input basis expired")
                })
            })
            .collect::<Result<Vec<_>>>()?;
        for binding in &bindings {
            if namespace_relative_path(Path::new(&binding.path), &root)?.is_none() {
                return Err(CoreError::new(
                    "outsideRoot",
                    "An original compile input escaped its root",
                ));
            }
        }
        sort_paths(&mut bindings, |binding| Path::new(&binding.path))?;
        let mut unique = Vec::<&SnapshotBinding>::new();
        for binding in bindings {
            if let Some(last) = unique.last()
                && equal_path(Path::new(&last.path), Path::new(&binding.path))?
            {
                if !same_binding(last, binding) {
                    return Err(stale(
                        "Competing original bases describe the same compile input",
                    ));
                }
                continue;
            }
            unique.push(binding);
        }
        let root_identity = find_path(&raw.directories, &root, |directory| {
            Path::new(&directory.path)
        })?
        .unwrap()
        .identity
        .clone();
        let mut witness = CompileDependencyWitness {
            root: root.clone(),
            root_identity,
            files: vec![],
            absent: vec![],
            directories: vec![],
        };
        let mut dependencies = dependencies.to_vec();
        sort_paths(&mut dependencies, |dependency| &dependency.path)?;
        let mut previous: Option<&CompileDependency> = None;
        for dependency in &dependencies {
            let path = absolute_path(&root.join(&dependency.path))?;
            if let Some(previous) = previous
                && equal_path(&previous.path, &dependency.path)?
                && previous.derived() != dependency.derived()
            {
                return Err(stale(
                    "An actual input cannot also be a producer-owned virtual file",
                ));
            }
            previous = Some(dependency);
            let binding = find_path(&unique, &path, |binding| Path::new(&binding.path))?;
            let directory = find_path(&raw.directories, &path, |directory| {
                Path::new(&directory.path)
            })?;
            if dependency.derived() {
                if dependency.status != CompileDependencyStatus::Loaded
                    || binding.is_some_and(|b| b.captured.is_some())
                    || directory.is_some()
                {
                    return Err(stale(
                        "A producer-owned virtual input collides with an original author object",
                    ));
                }
                witness.add_negative(&root, &path, &unique, &raw.directories)?;
            } else if let Some(binding) = binding {
                // unavailable source may still have successfully read file bytes
                // (for example invalid UTF-8); retain that positive original.
                if binding.captured.is_some() {
                    witness.add_file(binding)?;
                } else if dependency.status == CompileDependencyStatus::Loaded {
                    return Err(stale(
                        "Loaded native input was absent from its original cut",
                    ));
                } else {
                    witness.add_negative(&root, &path, &unique, &raw.directories)?;
                }
            } else if dependency.status == CompileDependencyStatus::Loaded {
                return Err(stale("Loaded native input has no original captured bytes"));
            } else if let Some(directory) = directory {
                let parent = path
                    .parent()
                    .ok_or_else(|| stale("A native directory dependency lacks its parent"))?;
                let original_parent = find_path(&raw.directories, parent, |directory| {
                    Path::new(&directory.path)
                })?
                .ok_or_else(|| stale("The original directory parent is unavailable"))?;
                if !witness
                    .directories
                    .iter()
                    .any(|entry| equal_path(&entry.path, &path).unwrap_or(false))
                {
                    witness.directories.push(DirectoryInput {
                        path,
                        identity: directory.identity.clone(),
                        parent: original_parent.identity.clone(),
                    });
                }
            } else {
                witness.add_negative(&root, &path, &unique, &raw.directories)?;
            }
        }
        self.validate_compile_dependency_witness(&witness)?;
        Ok(witness)
    }

    pub fn validate_compile_dependency_witness(
        &self,
        witness: &CompileDependencyWitness,
    ) -> Result<()> {
        self.with_validated_compile_dependencies(witness, |_| Ok(()))
    }

    /// Co-hold all positive original files and directory closures through one
    /// read-only final decision. Validate missing slots before and after it.
    /// Run this after the last await under the caller's Core/session lock;
    /// independent Draft/serial and style checks must remain in that decision.
    pub fn with_validated_compile_dependencies<T>(
        &self,
        witness: &CompileDependencyWitness,
        action: impl FnOnce(&Core) -> Result<T>,
    ) -> Result<T> {
        self.require_workspace(&witness.root)?;
        let root_guard = ParentGuard::acquire(&witness.root.join(".compile-dependency-root"))?;
        if root_guard.directory_identity()? != witness.root_identity {
            return Err(stale("The original compile root was replaced"));
        }
        let mut guards = vec![root_guard];
        let mut handles = vec![];
        for basis in &witness.files {
            let path = Path::new(&basis.path);
            self.check_pending(path)?;
            let guard = ParentGuard::acquire(path)?;
            let mut handle = if operation_read_control(&witness.root, path) {
                guard.open_read_guard_file()?
            } else {
                guard.open_file(false, false)?
            };
            let actual = stamp(&handle)?;
            self.check_pending_identity(&actual.identity)?;
            let expected = basis.captured.as_ref().unwrap();
            if guard.directory_identity()? != basis.parent
                || actual != expected.stamp
                || read_blob(&mut handle)? != expected.bytes
            {
                return Err(stale(
                    "An actual native dependency changed its original object, bytes or parent",
                ));
            }
            guards.push(guard);
            handles.push((handle, expected));
        }
        for directory in &witness.directories {
            let parent = ParentGuard::acquire(&directory.path)?;
            let child = ParentGuard::acquire(&directory.path.join(".compile-directory-input"))?;
            if parent.directory_identity()? != directory.parent
                || child.directory_identity()? != directory.identity
            {
                return Err(stale(
                    "An original directory input was replaced or reparented",
                ));
            }
            guards.push(parent);
            guards.push(child);
        }
        for slot in &witness.absent {
            self.check_pending(&slot.path)?;
            let guard = ParentGuard::acquire(&slot.path)?;
            if guard.directory_identity()? != slot.parent {
                return Err(stale("An unavailable input's original parent was replaced"));
            }
            absent(&slot.path)?;
            guards.push(guard);
        }
        let result = action(self);
        for (handle, expected) in &mut handles {
            if stamp(handle)? != expected.stamp || read_blob(handle)? != expected.bytes {
                return Err(stale(
                    "An actual native dependency changed during the final read decision",
                ));
            }
        }
        for slot in &witness.absent {
            absent(&slot.path)?;
        }
        drop(handles);
        drop(guards);
        result
    }
}

impl CompileDependencyWitness {
    fn add_file(&mut self, basis: &SnapshotBinding) -> Result<()> {
        for original in &self.files {
            if equal_path(Path::new(&original.path), Path::new(&basis.path))? {
                if !same_binding(original, basis) {
                    return Err(stale("Competing native dependency versions"));
                }
                return Ok(());
            }
        }
        if self
            .files
            .iter()
            .filter_map(|file| file.captured.as_ref())
            .map(|file| file.bytes.len())
            .sum::<usize>()
            + basis.captured.as_ref().unwrap().bytes.len()
            > MAX_BYTES
        {
            return Err(CoreError::new(
                "budget",
                "Actual native dependencies exceed 64 MiB",
            ));
        }
        self.files.push(basis.clone());
        Ok(())
    }
    fn add_negative(
        &mut self,
        root: &Path,
        path: &Path,
        bindings: &[&SnapshotBinding],
        directories: &[CapturedDirectory],
    ) -> Result<()> {
        let relative = namespace_relative_path(path, root)?.ok_or_else(|| {
            CoreError::new("outsideRoot", "Native dependency escaped its project")
        })?;
        let mut parent = root.to_path_buf();
        for component in relative.components() {
            let child = parent.join(component);
            if find_path(directories, &child, |directory| Path::new(&directory.path))?.is_some() {
                parent = child;
                continue;
            }
            if let Some(file) = find_path(bindings, &child, |binding| Path::new(&binding.path))?
                .filter(|binding| binding.captured.is_some())
            {
                // An existing ordinary file blocks traversal of this input's
                // missing parent. Preserve that original positive blocker.
                return self.add_file(file);
            }
            let original_parent =
                find_path(directories, &parent, |directory| Path::new(&directory.path))?
                    .ok_or_else(|| stale("A negative input has no original directory anchor"))?;
            if original_parent.entries.iter().any(|entry| {
                entry.rsplit_once(':').is_some_and(|(name, _)| {
                    equal_path(Path::new(name), Path::new(component.as_os_str())).unwrap_or(false)
                })
            }) {
                return Err(stale(
                    "An original directory entry has no complete captured object",
                ));
            }
            for slot in &self.absent {
                if equal_path(&slot.path, &child)? {
                    return Ok(());
                }
            }
            self.absent.push(AbsentSlot {
                path: child,
                parent: original_parent.identity.clone(),
            });
            return Ok(());
        }
        Err(stale(
            "A producer-owned virtual input collides with an original directory",
        ))
    }
}

fn absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(stale(
            "An originally unavailable native input slot is now occupied",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(all(test, windows))]
#[path = "compile_dependencies_tests.rs"]
mod tests;
