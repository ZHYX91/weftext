use crate::{Dependency, EngineError, ErrorDescription, UiMessage};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use typst::{
    Library, World,
    diag::{FileError, FileResult},
    foundations::{Bytes, Datetime, Duration},
    text::{Font, FontBook},
    utils::LazyHash,
};
use typst_kit::fonts::FontStore;
use typst_syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};

const FILE_LIMIT: u64 = 32 * 1024 * 1024;
const TOTAL_LIMIT: usize = 128 * 1024 * 1024;
const DEPENDENCY_LIMIT: usize = 512;

fn admitted_file_id(path: &Path) -> Result<FileId, EngineError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_world_frozen_path_relative_required(),
        ));
    }
    let logical = path
        .to_str()
        .ok_or_else(|| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_world_frozen_path_utf8_required(),
            )
        })?
        .replace('\\', "/");
    if logical.chars().any(char::is_control) {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_world_frozen_path_controls_unsupported(),
        ));
    }
    let virtual_path = VirtualPath::new(format!("/{logical}")).map_err(|e| {
        EngineError::new(crate::UiMessage::error_engine_world_frozen_virtual_path_invalid())
            .diagnostic(crate::DiagnosticOrigin::Dependency, e.to_string())
    })?;
    Ok(FileId::new(RootedPath::new(
        VirtualRoot::Project,
        virtual_path,
    )))
}

/// An immutable per-compilation source/file cache. No package downloader or
/// ambient project directory is exposed to Typst.
pub(crate) struct LocalWorld<'a> {
    pub library: &'a LazyHash<Library>,
    pub fonts: &'a FontStore,
    pub root: PathBuf,
    pub main_id: FileId,
    pub main_source: Source,
    compile_id: FileId,
    overrides: HashMap<FileId, Source>,
    frozen_files: Option<HashMap<FileId, Bytes>>,
    cache: Mutex<Cache>,
    pub(crate) trusted_style_sources: HashSet<FileId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ProductFailure {
    PackagesUnavailable,
    FrozenPackagesUnavailable,
    DependencyCountBudget,
    DependencyByteBudget,
    TotalDependencyByteBudget,
}

#[derive(Default)]
struct Cache {
    product_failures: BTreeMap<ProductFailure, ErrorDescription>,
    files: HashMap<FileId, FileResult<Bytes>>,
    sources: HashMap<FileId, FileResult<Source>>,
    dependencies: HashMap<(FileId, &'static str), Dependency>,
    bytes: usize,
}

impl<'a> LocalWorld<'a> {
    pub fn new(
        library: &'a LazyHash<Library>,
        fonts: &'a FontStore,
        source: &str,
        path: &Path,
        root: &Path,
    ) -> Result<Self, EngineError> {
        if source.len() as u64 > FILE_LIMIT {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_source_byte_budget(),
            ));
        }
        if is_reparse(root).unwrap_or(false) {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_dependency_root_link_unsupported(),
            ));
        }
        let root = fs::canonicalize(root).map_err(|e| {
            EngineError::new(crate::UiMessage::error_engine_world_dependency_root_unavailable())
                .diagnostic(crate::DiagnosticOrigin::Os, e.to_string())
        })?;
        let input_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        // Check the lexical path before canonicalizing its parent; otherwise a
        // symlink alias could disappear during canonicalization.
        let lexical_root = if path.is_absolute() && !input_path.starts_with(&root) {
            input_path
                .ancestors()
                .find(|ancestor| fs::canonicalize(ancestor).ok().as_ref() == Some(&root))
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.clone())
        } else {
            root.clone()
        };
        reject_reparse_chain(&input_path, &lexical_root).map_err(|_| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_world_main_root_confinement_required(),
            )
        })?;
        let parent = input_path.parent().ok_or_else(|| {
            EngineError::new(weftext_messages::UiMessage::error_engine_world_main_parent_missing())
        })?;
        let parent = fs::canonicalize(parent).map_err(|e| {
            EngineError::new(crate::UiMessage::error_engine_world_main_directory_unavailable())
                .diagnostic(crate::DiagnosticOrigin::Os, e.to_string())
        })?;
        if !parent.starts_with(&root) {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_main_outside_root(),
            ));
        }
        let main_path = parent.join(input_path.file_name().ok_or_else(|| {
            EngineError::new(weftext_messages::UiMessage::error_engine_world_main_path_invalid())
        })?);
        reject_reparse_chain(&main_path, &root).map_err(|_| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_world_main_path_link_unsupported(),
            )
        })?;
        let vpath = VirtualPath::virtualize(&root, &main_path).map_err(|e| {
            EngineError::new(crate::UiMessage::error_engine_world_main_path_invalid())
                .diagnostic(crate::DiagnosticOrigin::Dependency, e.to_string())
        })?;
        let main_id = FileId::new(RootedPath::new(VirtualRoot::Project, vpath));
        let main_source = Source::new(main_id, source.to_owned());
        Ok(Self {
            library,
            fonts,
            root,
            main_id,
            main_source,
            compile_id: main_id,
            overrides: HashMap::new(),
            frozen_files: None,
            cache: Mutex::new(Cache::default()),
            trusted_style_sources: HashSet::new(),
        })
    }

    pub fn with_style(
        library: &'a LazyHash<Library>,
        fonts: &'a FontStore,
        source: &str,
        path: &Path,
        root: &Path,
        style: &crate::StyleModule,
    ) -> Result<Self, EngineError> {
        let world = Self::new(library, fonts, source, path, root)?;
        Self::apply_style(world, style)
    }

    fn apply_style(mut world: Self, style: &crate::StyleModule) -> Result<Self, EngineError> {
        if !style.enabled {
            return Ok(world);
        }
        if style.source.len() as u64 > FILE_LIMIT {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_style_byte_budget(),
            ));
        }
        if style.entry_function.is_empty()
            || !style.entry_function.chars().enumerate().all(|(i, c)| {
                c.is_ascii_alphabetic() || c == '_' || (i > 0 && (c.is_ascii_digit() || c == '-'))
            })
        {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_style_entry_identifier_required(),
            ));
        }
        let style_path = Path::new(&style.path);
        if style_path.is_absolute()
            || style_path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_style_relative_path_required(),
            ));
        }
        let style_path = world.root.join(style_path);
        if world.frozen_files.is_none() {
            reject_reparse_chain(&style_path, &world.root).map_err(|_| {
                EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_style_path_invalid(),
                )
            })?;
        }
        let style_vpath = VirtualPath::virtualize(&world.root, &style_path).map_err(|e| {
            EngineError::new(crate::UiMessage::error_engine_world_style_virtual_path_invalid())
                .diagnostic(crate::DiagnosticOrigin::Dependency, e.to_string())
        })?;
        let style_id = FileId::new(RootedPath::new(VirtualRoot::Project, style_vpath));
        if style_id == world.main_id {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_style_aliases_main(),
            ));
        }
        let entry_id = FileId::new(RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new(format!(
                "/.weftext-runtime-entry-{}.typ",
                crate::source_hash(&format!(
                    "{}\n{}\n{}",
                    world.main_source.text(),
                    style.source,
                    style.path
                ))
            ))
            .map_err(|e| {
                EngineError::new(crate::UiMessage::error_engine_world_runtime_entry_path_invalid())
                    .diagnostic(crate::DiagnosticOrigin::Dependency, e.to_string())
            })?,
        ));
        if entry_id == world.main_id || entry_id == style_id {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_author_source_reserved_path(),
            ));
        }
        if let Some(files) = &world.frozen_files {
            if files.contains_key(&entry_id) {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_frozen_file_reserved_path(),
                ));
            }
            if files
                .get(&style_id)
                .is_some_and(|bytes| bytes.as_slice() != style.source.as_bytes())
            {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_frozen_style_mismatch(),
                ));
            }
        } else if entry_id
            .vpath()
            .realize(&world.root)
            .is_ok_and(|path| path.exists())
        {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_author_file_reserved_path(),
            ));
        }
        let entry = format!(
            "#import {}: {}\n#show: {}\n#include {}\n",
            crate::encode_string(&format!("/{}", style_id.vpath().get_without_slash()))?,
            style.entry_function,
            style.entry_function,
            crate::encode_string(&format!("/{}", world.main_id.vpath().get_without_slash()))?
        );
        if crate::style_allows_rich_editing(style) {
            world.trusted_style_sources.extend([style_id, entry_id]);
        }
        world
            .overrides
            .insert(style_id, Source::new(style_id, style.source.clone()));
        world
            .overrides
            .insert(entry_id, Source::new(entry_id, entry));
        world.compile_id = entry_id;
        Ok(world)
    }

    pub fn frozen(
        library: &'a LazyHash<Library>,
        fonts: &'a FontStore,
        input: &crate::FrozenRenderRequest,
    ) -> Result<Self, EngineError> {
        if !input.root.is_absolute()
            || input.root.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
        {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_frozen_root_absolute_required(),
            ));
        }
        let relative = if input.path.is_absolute() {
            input.path.strip_prefix(&input.root).map_err(|_| {
                EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_frozen_main_outside_root(),
                )
            })?
        } else {
            input.path.as_path()
        };
        let main_id = admitted_file_id(relative)?;
        if input.source.len() as u64 > FILE_LIMIT {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_world_frozen_main_byte_budget(),
            ));
        }
        let mut files = HashMap::new();
        let mut total = input.source.len();
        for (path, data) in &input.files {
            let id = admitted_file_id(Path::new(path))?;
            if data.len() as u64 > FILE_LIMIT {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_frozen_file_byte_budget(),
                ));
            }
            total = total.checked_add(data.len()).ok_or_else(|| {
                EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_frozen_input_size_overflow(),
                )
            })?;
            if total > TOTAL_LIMIT {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_frozen_input_byte_budget(),
                ));
            }
            if id == main_id && data.as_slice() != input.source.as_bytes() {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_frozen_main_mismatch(),
                ));
            }
            if files.insert(id, Bytes::new(data.clone())).is_some() {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_world_frozen_path_duplicate(),
                ));
            }
        }
        let world = Self {
            library,
            fonts,
            root: input.root.clone(),
            main_id,
            main_source: Source::new(main_id, input.source.clone()),
            compile_id: main_id,
            overrides: HashMap::new(),
            frozen_files: Some(files),
            cache: Mutex::new(Cache::default()),
            trusted_style_sources: HashSet::new(),
        };
        match &input.style {
            Some(style) => Self::apply_style(world, style),
            None => Ok(world),
        }
    }

    /// Read only the already validated immutable admission map. No filesystem
    /// lookup, override, or fallback can certify a bundled module's bytes.
    pub(crate) fn admitted_bytes(&self, path: &Path) -> Result<Option<&[u8]>, EngineError> {
        let id = admitted_file_id(path)?;
        Ok(self
            .frozen_files
            .as_ref()
            .and_then(|files| files.get(&id))
            .map(Bytes::as_slice))
    }

    pub(crate) fn is_main_admitted_path(&self, path: &Path) -> Result<bool, EngineError> {
        Ok(admitted_file_id(path)? == self.main_id)
    }

    // One explanation per actual admission fact per compilation. Compiler
    // diagnostics retain every source occurrence independently.
    pub(crate) fn product_diagnostics(&self) -> Vec<ErrorDescription> {
        self.cache
            .lock()
            .unwrap()
            .product_failures
            .values()
            .cloned()
            .collect()
    }
    fn product_failure(&self, kind: ProductFailure, explanation: UiMessage) -> FileError {
        self.cache
            .lock()
            .unwrap()
            .product_failures
            .entry(kind)
            .or_insert_with(|| explanation.into());
        FileError::Other(None)
    }

    fn resolve(&self, id: FileId) -> FileResult<PathBuf> {
        if !matches!(id.root(), VirtualRoot::Project) {
            return Err(self.product_failure(
                ProductFailure::PackagesUnavailable,
                UiMessage::error_engine_world_packages_unavailable(),
            ));
        }
        let path = id.vpath().realize(&self.root)?;
        reject_reparse_chain(&path, &self.root)?;
        let canonical = fs::canonicalize(&path).map_err(|e| FileError::from_io(e, &path))?;
        if !canonical.starts_with(&self.root) {
            return Err(FileError::AccessDenied);
        }
        Ok(canonical)
    }

    pub fn display_path(&self, id: FileId) -> String {
        if matches!(id.root(), VirtualRoot::Project) {
            id.vpath().get_without_slash().to_owned()
        } else {
            format!("{:?}", id.get())
        }
    }

    /// Exact source snapshots already admitted/read by this compilation only.
    /// Does not discover dependencies or read the filesystem.
    pub(crate) fn loaded_sources(&self) -> Vec<Source> {
        let mut sources = vec![self.main_source.clone()];
        sources.extend(self.overrides.values().cloned());
        sources.extend(
            self.cache
                .lock()
                .unwrap()
                .sources
                .values()
                .filter_map(|source| source.as_ref().ok().cloned()),
        );
        sources
    }

    /// Synthetic compiler entry points are identified by their actual FileId,
    /// never by a user-controllable filename prefix.
    pub(crate) fn derived_dependency_paths(&self) -> HashSet<String> {
        if self.compile_id != self.main_id && self.overrides.contains_key(&self.compile_id) {
            HashSet::from([self.display_path(self.compile_id)])
        } else {
            HashSet::new()
        }
    }

    pub fn dependencies(&self) -> Vec<Dependency> {
        let mut deps: Vec<_> = self
            .cache
            .lock()
            .unwrap()
            .dependencies
            .iter()
            .map(|((id, _), dependency)| {
                let mut dependency = dependency.clone();
                if *id == self.compile_id
                    && self.compile_id != self.main_id
                    && self.overrides.contains_key(id)
                {
                    dependency.kind = match dependency.kind.as_str() {
                        "source" => "derived-source".into(),
                        "file" => "derived-file".into(),
                        _ => dependency.kind,
                    };
                }
                dependency
            })
            .collect();
        deps.sort_by(|a, b| (&a.path, &a.kind).cmp(&(&b.path, &b.kind)));
        deps
    }

    fn record(&self, id: FileId, kind: &'static str, good: bool) {
        self.cache.lock().unwrap().dependencies.insert(
            (id, kind),
            Dependency {
                path: self.display_path(id),
                kind: kind.into(),
                status: if good { "loaded" } else { "unavailable" }.into(),
            },
        );
    }
}

impl World for LocalWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }
    fn main(&self) -> FileId {
        self.compile_id
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            return Ok(self.main_source.clone());
        }
        if let Some(source) = self.overrides.get(&id) {
            self.record(id, "source", true);
            return Ok(source.clone());
        }
        if let Some(cached) = self.cache.lock().unwrap().sources.get(&id) {
            return cached.clone();
        }
        let result = self.file(id).and_then(|bytes| {
            let text = std::str::from_utf8(bytes.as_slice()).map_err(|_| FileError::InvalidUtf8)?;
            Ok(Source::new(id, text.to_owned()))
        });
        self.record(id, "source", result.is_ok());
        self.cache
            .lock()
            .unwrap()
            .sources
            .insert(id, result.clone());
        result
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main_id {
            return Ok(Bytes::from_string(self.main_source.text().to_owned()));
        }
        if let Some(source) = self.overrides.get(&id) {
            self.record(id, "file", true);
            return Ok(Bytes::from_string(source.text().to_owned()));
        }
        if let Some(cached) = self.cache.lock().unwrap().files.get(&id) {
            return cached.clone();
        }
        let result = (|| {
            if self.cache.lock().unwrap().files.len() >= DEPENDENCY_LIMIT {
                return Err(self.product_failure(
                    ProductFailure::DependencyCountBudget,
                    UiMessage::error_engine_world_dependency_count_budget(),
                ));
            }
            if let Some(files) = &self.frozen_files {
                if !matches!(id.root(), VirtualRoot::Project) {
                    return Err(self.product_failure(
                        ProductFailure::FrozenPackagesUnavailable,
                        UiMessage::error_engine_world_frozen_packages_unavailable(),
                    ));
                }
                return files.get(&id).cloned().ok_or(FileError::AccessDenied);
            }
            let path = self.resolve(id)?;
            let metadata = fs::metadata(&path).map_err(|e| FileError::from_io(e, &path))?;
            if !metadata.is_file() {
                return Err(FileError::IsDirectory);
            }
            if metadata.len() > FILE_LIMIT {
                return Err(self.product_failure(
                    ProductFailure::DependencyByteBudget,
                    UiMessage::error_engine_world_dependency_byte_budget(),
                ));
            }
            let data = fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
            // Recheck the path after reading, so a straightforward reparse
            // replacement cannot silently become a granted outside dependency.
            if self.resolve(id)? != path {
                return Err(FileError::AccessDenied);
            }
            let mut cache = self.cache.lock().unwrap();
            if cache.bytes.saturating_add(data.len()) > TOTAL_LIMIT {
                cache
                    .product_failures
                    .entry(ProductFailure::TotalDependencyByteBudget)
                    .or_insert_with(|| {
                        UiMessage::error_engine_world_total_dependency_byte_budget().into()
                    });
                return Err(FileError::Other(None));
            }
            cache.bytes += data.len();
            Ok(Bytes::new(data))
        })();
        self.record(id, "file", result.is_ok());
        self.cache.lock().unwrap().files.insert(id, result.clone());
        result
    }
}

fn reject_reparse_chain(path: &Path, root: &Path) -> FileResult<()> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| FileError::AccessDenied)?;
    let mut cursor = root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(FileError::AccessDenied);
        }
        cursor.push(component);
        match is_reparse(&cursor) {
            Ok(true) => return Err(FileError::AccessDenied),
            Ok(false) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(FileError::from_io(e, &cursor)),
        }
    }
    Ok(())
}

fn is_reparse(path: &Path) -> std::io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Ok(metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0)
    }
    #[cfg(not(windows))]
    {
        Ok(metadata.file_type().is_symlink())
    }
}

#[cfg(test)]
mod product_failure_tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn frozen_file_admission_budget_is_typed_and_isolated_per_compilation() {
        let engine = crate::Engine::new_with_system_fonts(false).unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let input = crate::FrozenRenderRequest {
            source: "作者原文😀".into(),
            path: root.path().join("main.typ"),
            root: root.path().into(),
            files: (0..=DEPENDENCY_LIMIT)
                .map(|index| (format!("part{index}.typ"), b"content".to_vec()))
                .collect::<BTreeMap<_, _>>(),
            style: None,
        };
        let world = LocalWorld::frozen(&engine.library, &engine.fonts, &input).unwrap();
        for index in 0..DEPENDENCY_LIMIT {
            let id = admitted_file_id(Path::new(&format!("part{index}.typ"))).unwrap();
            assert!(world.file(id).is_ok());
        }
        let next = admitted_file_id(Path::new(&format!("part{DEPENDENCY_LIMIT}.typ"))).unwrap();
        assert!(matches!(world.file(next), Err(FileError::Other(None))));
        assert!(world.file(next).is_err());
        let diagnostics = world.product_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].explanation,
            UiMessage::error_engine_world_dependency_count_budget()
        );
        assert!(diagnostics[0].diagnostics.is_empty());
        assert!(diagnostics[0].causes.as_slice().is_empty());
        assert_eq!(world.main_source.text(), input.source);
        let separate = LocalWorld::frozen(&engine.library, &engine.fonts, &input).unwrap();
        assert!(separate.product_diagnostics().is_empty());
        assert!(separate.file(next).is_ok());
    }
}
