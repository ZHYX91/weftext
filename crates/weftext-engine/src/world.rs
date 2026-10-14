use crate::{Dependency, EngineError};
use std::{
    collections::HashMap,
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
}

#[derive(Default)]
struct Cache {
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
            return Err(EngineError("源文件超过 32 MiB 编译上限".into()));
        }
        if is_reparse(root).unwrap_or(false) {
            return Err(EngineError("依赖根不可是符号链接或 reparse point".into()));
        }
        let root =
            fs::canonicalize(root).map_err(|e| EngineError(format!("无法打开依赖根：{e}")))?;
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
        reject_reparse_chain(&input_path, &lexical_root)
            .map_err(|_| EngineError("主文件必须位于依赖根内且不能经过符号链接".into()))?;
        let parent = input_path
            .parent()
            .ok_or_else(|| EngineError("主文件没有父目录".into()))?;
        let parent =
            fs::canonicalize(parent).map_err(|e| EngineError(format!("主文件目录不可用：{e}")))?;
        if !parent.starts_with(&root) {
            return Err(EngineError("主文件必须在已选择的依赖根内".into()));
        }
        let main_path = parent.join(
            input_path
                .file_name()
                .ok_or_else(|| EngineError("主文件路径无效".into()))?,
        );
        reject_reparse_chain(&main_path, &root)
            .map_err(|_| EngineError("主文件路径含符号链接或 reparse point".into()))?;
        let vpath = VirtualPath::virtualize(&root, &main_path)
            .map_err(|e| EngineError(format!("主文件路径无效：{e}")))?;
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
            return Err(EngineError("样式模块超过 32 MiB".into()));
        }
        if style.entry_function.is_empty()
            || !style.entry_function.chars().enumerate().all(|(i, c)| {
                c.is_ascii_alphabetic() || c == '_' || (i > 0 && (c.is_ascii_digit() || c == '-'))
            })
        {
            return Err(EngineError("样式入口必须是原生函数标识符".into()));
        }
        let style_path = Path::new(&style.path);
        if style_path.is_absolute()
            || style_path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(EngineError("样式模块须使用库内相对路径".into()));
        }
        let style_path = world.root.join(style_path);
        if world.frozen_files.is_none() {
            reject_reparse_chain(&style_path, &world.root)
                .map_err(|_| EngineError("样式模块路径越界或经过链接".into()))?;
        }
        let style_vpath = VirtualPath::virtualize(&world.root, &style_path)
            .map_err(|e| EngineError(e.to_string()))?;
        let style_id = FileId::new(RootedPath::new(VirtualRoot::Project, style_vpath));
        if style_id == world.main_id {
            return Err(EngineError("笔记与样式模块必须是不同对象".into()));
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
            .map_err(|e| EngineError(e.to_string()))?,
        ));
        if entry_id == world.main_id || entry_id == style_id {
            return Err(EngineError("作者源占用了派生入口路径".into()));
        }
        if let Some(files) = &world.frozen_files {
            if files.contains_key(&entry_id) {
                return Err(EngineError("冻结文件占用了派生入口路径".into()));
            }
            if files
                .get(&style_id)
                .is_some_and(|bytes| bytes.as_slice() != style.source.as_bytes())
            {
                return Err(EngineError("冻结样式与显式样式源不一致".into()));
            }
        } else if entry_id
            .vpath()
            .realize(&world.root)
            .is_ok_and(|path| path.exists())
        {
            return Err(EngineError("作者文件占用了派生入口路径".into()));
        }
        let entry = format!(
            "#import {}: {}\n#show: {}\n#include {}\n",
            crate::encode_string(&format!("/{}", style_id.vpath().get_without_slash()))?,
            style.entry_function,
            style.entry_function,
            crate::encode_string(&format!("/{}", world.main_id.vpath().get_without_slash()))?
        );
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
            return Err(EngineError("冻结根须为准确绝对路径".into()));
        }
        let relative = if input.path.is_absolute() {
            input
                .path
                .strip_prefix(&input.root)
                .map_err(|_| EngineError("冻结主源在根外".into()))?
        } else {
            input.path.as_path()
        };
        fn admitted_id(path: &Path) -> Result<FileId, EngineError> {
            if path.as_os_str().is_empty()
                || path
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(EngineError("冻结文件路径必须是根内准确相对路径".into()));
            }
            let logical = path
                .to_str()
                .ok_or_else(|| EngineError("冻结路径须为 UTF-8".into()))?
                .replace('\\', "/");
            if logical.chars().any(char::is_control) {
                return Err(EngineError("冻结路径含控制字符".into()));
            }
            let virtual_path =
                VirtualPath::new(format!("/{logical}")).map_err(|e| EngineError(e.to_string()))?;
            Ok(FileId::new(RootedPath::new(
                VirtualRoot::Project,
                virtual_path,
            )))
        }
        let main_id = admitted_id(relative)?;
        if input.source.len() as u64 > FILE_LIMIT {
            return Err(EngineError("冻结主源超过 32 MiB".into()));
        }
        let mut files = HashMap::new();
        let mut total = input.source.len();
        for (path, data) in &input.files {
            let id = admitted_id(Path::new(path))?;
            if data.len() as u64 > FILE_LIMIT {
                return Err(EngineError("冻结文件超过 32 MiB".into()));
            }
            total = total
                .checked_add(data.len())
                .ok_or_else(|| EngineError("冻结输入大小溢出".into()))?;
            if total > TOTAL_LIMIT {
                return Err(EngineError("冻结输入超过 128 MiB".into()));
            }
            if id == main_id && data.as_slice() != input.source.as_bytes() {
                return Err(EngineError("冻结主文件与显式主源不一致".into()));
            }
            if files.insert(id, Bytes::new(data.clone())).is_some() {
                return Err(EngineError("冻结文件路径不唯一".into()));
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
        };
        match &input.style {
            Some(style) => Self::apply_style(world, style),
            None => Ok(world),
        }
    }

    fn resolve(&self, id: FileId) -> FileResult<PathBuf> {
        if !matches!(id.root(), VirtualRoot::Project) {
            return Err(FileError::Other(Some(
                "packages are unavailable; install explicit project dependencies locally".into(),
            )));
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

    pub fn dependencies(&self) -> Vec<Dependency> {
        let mut deps: Vec<_> = self
            .cache
            .lock()
            .unwrap()
            .dependencies
            .values()
            .cloned()
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
                return Err(FileError::Other(Some(
                    "dependency count exceeds limit".into(),
                )));
            }
            if let Some(files) = &self.frozen_files {
                if !matches!(id.root(), VirtualRoot::Project) {
                    return Err(FileError::Other(Some(
                        "packages are unavailable in the frozen input".into(),
                    )));
                }
                return files.get(&id).cloned().ok_or(FileError::AccessDenied);
            }
            let path = self.resolve(id)?;
            let metadata = fs::metadata(&path).map_err(|e| FileError::from_io(e, &path))?;
            if !metadata.is_file() {
                return Err(FileError::IsDirectory);
            }
            if metadata.len() > FILE_LIMIT {
                return Err(FileError::Other(Some(
                    "dependency exceeds 32 MiB limit".into(),
                )));
            }
            let data = fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
            // Recheck the path after reading, so a straightforward reparse
            // replacement cannot silently become a granted outside dependency.
            if self.resolve(id)? != path {
                return Err(FileError::AccessDenied);
            }
            let mut cache = self.cache.lock().unwrap();
            if cache.bytes.saturating_add(data.len()) > TOTAL_LIMIT {
                return Err(FileError::Other(Some(
                    "total dependencies exceed 128 MiB limit".into(),
                )));
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
