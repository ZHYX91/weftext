use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const CONTENT_RULES_FILE_NAME: &str = ".weftext-rules";
const RULES_HEADER: &str = "weftext-content-rules-v1";
const MAX_RULES_BYTES: u64 = 1024 * 1024;
const MAX_RULE_LINE_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoundaryAction {
    Unmanaged,
    Ignore,
}

#[derive(Clone, Debug)]
struct ContentRule {
    action: BoundaryAction,
    pattern: GlobPattern,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ContentRules {
    rules: Vec<ContentRule>,
    total_pattern_segments: usize,
    total_segment_tokens: usize,
}

impl ContentRules {
    pub(crate) fn load(root: &Path) -> Result<Self, ContentRulesError> {
        let path = root.join(CONTENT_RULES_FILE_NAME);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(ContentRulesError::Io(error)),
        };
        if linked_or_reparse(&metadata) {
            return Err(ContentRulesError::LinkedRulesFile);
        }
        if !metadata.is_file() {
            return Err(ContentRulesError::RulesPathNotFile);
        }
        if metadata.len() > MAX_RULES_BYTES {
            return Err(ContentRulesError::RulesFileTooLarge);
        }
        let bytes = fs::read(&path).map_err(ContentRulesError::Io)?;
        if bytes.len() as u64 > MAX_RULES_BYTES {
            return Err(ContentRulesError::RulesFileTooLarge);
        }
        let source = String::from_utf8(bytes).map_err(|_| ContentRulesError::InvalidUtf8)?;
        parse_rules(&source)
    }

    pub(crate) fn classify(&self, relative: &str, is_directory: bool) -> Option<BoundaryAction> {
        self.rules
            .iter()
            .filter(|rule| rule.pattern.matches(relative, is_directory))
            .map(|rule| rule.action)
            .next_back()
    }

    pub(crate) fn managed_node_classification_work_upper_bound(
        &self,
        relative_directory: &str,
        name: &str,
    ) -> Option<usize> {
        let document = crate::canonical_document_locator(relative_directory, name);
        let directory_segments = portable_segment_count(relative_directory);
        self.classification_work_upper_bound(&document)?
            .checked_mul(directory_segments.checked_add(1)?)
    }

    fn classification_work_upper_bound(&self, relative: &str) -> Option<usize> {
        if self.rules.is_empty() {
            return Some(0);
        }
        let path_segments = portable_segment_count(relative);
        let maximum_segment_characters = relative
            .split('/')
            .map(str::chars)
            .map(Iterator::count)
            .max()
            .unwrap_or(0);
        self.rules
            .len()
            .checked_add(
                self.total_pattern_segments
                    .checked_mul(path_segments.checked_add(1)?)?,
            )?
            .checked_add(
                self.total_segment_tokens
                    .checked_mul(path_segments)?
                    .checked_mul(maximum_segment_characters.checked_add(1)?)?,
            )
    }
}

fn portable_segment_count(relative: &str) -> usize {
    usize::from(!relative.is_empty()) + relative.bytes().filter(|byte| *byte == b'/').count()
}

fn parse_rules(source: &str) -> Result<ContentRules, ContentRulesError> {
    if source.contains('\0') {
        return Err(ContentRulesError::NulByte);
    }
    for (index, raw) in source.lines().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.len() > MAX_RULE_LINE_BYTES {
            return Err(ContentRulesError::LineTooLong { line: index + 1 });
        }
    }
    let mut meaningful = source.lines().enumerate().filter_map(|(index, raw)| {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim();
        (!trimmed.is_empty() && !trimmed.starts_with('#')).then_some((index + 1, line))
    });
    let Some((header_line, header)) = meaningful.next() else {
        return Err(ContentRulesError::MissingHeader);
    };
    if header != RULES_HEADER {
        return Err(ContentRulesError::InvalidHeader { line: header_line });
    }
    let mut rules = Vec::new();
    let mut total_pattern_segments = 0_usize;
    let mut total_segment_tokens = 0_usize;
    for (line_number, line) in meaningful {
        if line.starts_with(char::is_whitespace) {
            return Err(ContentRulesError::InvalidSyntax { line: line_number });
        }
        let Some((verb, encoded_pattern)) = line.split_once(' ') else {
            return Err(ContentRulesError::InvalidSyntax { line: line_number });
        };
        if encoded_pattern.is_empty() || encoded_pattern.starts_with(' ') {
            return Err(ContentRulesError::InvalidSyntax { line: line_number });
        }
        let action = match verb {
            "unmanaged" => BoundaryAction::Unmanaged,
            "ignore" => BoundaryAction::Ignore,
            _ => return Err(ContentRulesError::UnknownAction { line: line_number }),
        };
        let pattern = GlobPattern::parse(encoded_pattern, line_number)?;
        total_pattern_segments = total_pattern_segments.saturating_add(pattern.segments.len());
        total_segment_tokens = total_segment_tokens.saturating_add(
            pattern
                .segments
                .iter()
                .map(|segment| match segment {
                    PatternSegment::Recursive => 0,
                    PatternSegment::Segment(tokens) => tokens.len(),
                })
                .sum::<usize>(),
        );
        rules.push(ContentRule { action, pattern });
    }
    Ok(ContentRules {
        rules,
        total_pattern_segments,
        total_segment_tokens,
    })
}

#[derive(Clone, Debug)]
struct GlobPattern {
    segments: Vec<PatternSegment>,
    directory_only: bool,
}

impl GlobPattern {
    fn parse(encoded: &str, line: usize) -> Result<Self, ContentRulesError> {
        if encoded.starts_with('/')
            || encoded.starts_with('\\')
            || (encoded.as_bytes().get(1) == Some(&b':')
                && encoded.as_bytes()[0].is_ascii_alphabetic())
        {
            return Err(ContentRulesError::AbsolutePattern { line });
        }
        let directory_only = encoded.ends_with('/');
        let encoded = encoded.strip_suffix('/').unwrap_or(encoded);
        if encoded.is_empty() {
            return Err(ContentRulesError::RootPattern { line });
        }
        let raw_segments = encoded.split('/').collect::<Vec<_>>();
        if raw_segments.iter().any(|segment| segment.is_empty()) {
            return Err(ContentRulesError::EmptySegment { line });
        }
        let mut segments = Vec::with_capacity(raw_segments.len());
        for raw in raw_segments {
            if raw == "." || raw == ".." {
                return Err(ContentRulesError::TraversalPattern { line });
            }
            if raw == "**" {
                segments.push(PatternSegment::Recursive);
                continue;
            }
            if raw.contains("**") {
                return Err(ContentRulesError::InvalidRecursiveWildcard { line });
            }
            segments.push(PatternSegment::Segment(parse_segment(raw, line)?));
        }
        Ok(Self {
            segments,
            directory_only,
        })
    }

    fn matches(&self, relative: &str, is_directory: bool) -> bool {
        if self.directory_only && !is_directory {
            return false;
        }
        let path_segments = relative.split('/').collect::<Vec<_>>();
        match_segments(&self.segments, &path_segments)
    }
}

#[derive(Clone, Debug)]
enum PatternSegment {
    Recursive,
    Segment(Vec<SegmentToken>),
}

#[derive(Clone, Debug)]
enum SegmentToken {
    Literal(char),
    AnyOne,
    AnyMany,
}

fn parse_segment(raw: &str, line: usize) -> Result<Vec<SegmentToken>, ContentRulesError> {
    let mut tokens = Vec::new();
    let mut characters = raw.chars();
    while let Some(character) = characters.next() {
        match character {
            '*' => tokens.push(SegmentToken::AnyMany),
            '?' => tokens.push(SegmentToken::AnyOne),
            '\\' => {
                let escaped = characters
                    .next()
                    .ok_or(ContentRulesError::DanglingEscape { line })?;
                if !matches!(escaped, ' ' | '#' | '\\' | '*' | '?') {
                    return Err(ContentRulesError::InvalidEscape { line });
                }
                tokens.push(SegmentToken::Literal(escaped));
            }
            ' ' | '#' => return Err(ContentRulesError::InvalidSyntax { line }),
            character => tokens.push(SegmentToken::Literal(character)),
        }
    }
    if tokens.is_empty() {
        return Err(ContentRulesError::EmptySegment { line });
    }
    Ok(tokens)
}

fn match_segments(pattern: &[PatternSegment], path: &[&str]) -> bool {
    let mut matches = vec![vec![false; path.len() + 1]; pattern.len() + 1];
    matches[pattern.len()][path.len()] = true;
    for pattern_index in (0..pattern.len()).rev() {
        for path_index in (0..=path.len()).rev() {
            matches[pattern_index][path_index] = match &pattern[pattern_index] {
                PatternSegment::Recursive => {
                    matches[pattern_index + 1][path_index]
                        || (path_index < path.len() && matches[pattern_index][path_index + 1])
                }
                PatternSegment::Segment(tokens) => {
                    path_index < path.len()
                        && match_segment(tokens, path[path_index])
                        && matches[pattern_index + 1][path_index + 1]
                }
            };
        }
    }
    matches[0][0]
}

fn match_segment(pattern: &[SegmentToken], value: &str) -> bool {
    let characters = value.chars().collect::<Vec<_>>();
    let mut matches = vec![vec![false; characters.len() + 1]; pattern.len() + 1];
    matches[pattern.len()][characters.len()] = true;
    for pattern_index in (0..pattern.len()).rev() {
        for character_index in (0..=characters.len()).rev() {
            matches[pattern_index][character_index] = match pattern[pattern_index] {
                SegmentToken::Literal(expected) => {
                    character_index < characters.len()
                        && characters[character_index] == expected
                        && matches[pattern_index + 1][character_index + 1]
                }
                SegmentToken::AnyOne => {
                    character_index < characters.len()
                        && matches[pattern_index + 1][character_index + 1]
                }
                SegmentToken::AnyMany => {
                    matches[pattern_index + 1][character_index]
                        || (character_index < characters.len()
                            && matches[pattern_index][character_index + 1])
                }
            };
        }
    }
    matches[0][0]
}

pub(crate) fn portable_path(root: &Path, path: &Path) -> Result<String, ContentRulesError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ContentRulesError::PathEscape(path.to_path_buf()))?;
    let mut pieces = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => pieces.push(
                value
                    .to_str()
                    .ok_or_else(|| ContentRulesError::NonUtf8Path(path.to_path_buf()))?,
            ),
            _ => return Err(ContentRulesError::PathEscape(path.to_path_buf())),
        }
    }
    Ok(pieces.join("/"))
}

pub(crate) fn linked_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub(crate) fn validate_managed_node_path(node_directory: &Path) -> Result<(), ContentRulesError> {
    reject_linked_existing_ancestors(node_directory)?;
    let mut workspace_roots = Vec::new();
    for ancestor in node_directory.ancestors() {
        let candidate = ancestor.join(CONTENT_RULES_FILE_NAME);
        match fs::symlink_metadata(&candidate) {
            Ok(_) => {
                workspace_roots.push(ancestor.to_path_buf());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(ContentRulesError::Io(error)),
        }
    }
    if workspace_roots.len() > 1 {
        return Err(ContentRulesError::MultipleRulesFiles);
    }
    let Some(root) = workspace_roots.pop() else {
        return Ok(());
    };
    let rules = ContentRules::load(&root)?;
    if crate::workspace_document_format(&root).generation
        != crate::WorkspaceDocumentGeneration::AsciiDocV1
    {
        return Err(ContentRulesError::CanonicalDocumentRule {
            path: portable_path(&root, node_directory)?,
        });
    }
    validate_managed_node_path_with_loaded_rules(&root, node_directory, &rules)
}

/// Validates one prospective canonical `AsciiDoc` node with already-loaded root rules.
///
/// The caller must have established that `workspace_root` is the one workspace authority and that
/// the selected generation is `AsciiDoc` v1. Existing ancestors are still checked for link/reparse
/// substitution on every call.
pub(crate) fn validate_managed_node_path_with_rules(
    workspace_root: &Path,
    node_directory: &Path,
    rules: &ContentRules,
) -> Result<(), ContentRulesError> {
    reject_linked_existing_ancestors(node_directory)?;
    validate_managed_node_path_with_loaded_rules(workspace_root, node_directory, rules)
}

fn validate_managed_node_path_with_loaded_rules(
    root: &Path,
    node_directory: &Path,
    rules: &ContentRules,
) -> Result<(), ContentRulesError> {
    let relative = portable_path(root, node_directory)?;
    if !relative.is_empty() {
        let pieces = relative.split('/').collect::<Vec<_>>();
        for end in 1..=pieces.len() {
            let candidate = pieces[..end].join("/");
            if let Some(action) = rules.classify(&candidate, true) {
                return Err(ContentRulesError::NodeOutsideManagedBoundary {
                    path: relative,
                    action,
                });
            }
        }
    }
    let name = node_directory
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ContentRulesError::NonUtf8Path(node_directory.to_path_buf()))?;
    let document = crate::canonical_document_locator(&relative, name);
    if rules.classify(&document, false).is_some() {
        return Err(ContentRulesError::CanonicalDocumentRule { path: document });
    }
    Ok(())
}

pub(crate) fn validate_managed_file_path(
    workspace_root: &Path,
    file_path: &Path,
) -> Result<(), ContentRulesError> {
    reject_linked_existing_ancestors(file_path)?;
    let rules = ContentRules::load(workspace_root)?;
    let relative = portable_path(workspace_root, file_path)?;
    let pieces = relative.split('/').collect::<Vec<_>>();
    if pieces.is_empty() || pieces[0].is_empty() {
        return Err(ContentRulesError::PathEscape(file_path.to_path_buf()));
    }
    for end in 1..pieces.len() {
        let candidate = pieces[..end].join("/");
        if let Some(action) = rules.classify(&candidate, true) {
            return Err(ContentRulesError::NodeOutsideManagedBoundary {
                path: relative,
                action,
            });
        }
    }
    if let Some(action) = rules.classify(&relative, false) {
        return Err(ContentRulesError::ManagedFileBoundary {
            path: relative,
            action,
        });
    }
    Ok(())
}

pub(crate) fn reject_linked_existing_ancestors(path: &Path) -> Result<(), ContentRulesError> {
    let absolute;
    let path = if path.is_absolute() {
        path
    } else {
        absolute = std::env::current_dir()
            .map_err(ContentRulesError::Io)?
            .join(path);
        &absolute
    };
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if linked_or_reparse(&metadata) => {
                return Err(ContentRulesError::LinkedPath(ancestor.to_path_buf()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(ContentRulesError::Io(error)),
        }
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) enum ContentRulesError {
    Io(std::io::Error),
    LinkedRulesFile,
    RulesPathNotFile,
    RulesFileTooLarge,
    MultipleRulesFiles,
    InvalidUtf8,
    NulByte,
    MissingHeader,
    InvalidHeader {
        line: usize,
    },
    LineTooLong {
        line: usize,
    },
    InvalidSyntax {
        line: usize,
    },
    UnknownAction {
        line: usize,
    },
    AbsolutePattern {
        line: usize,
    },
    RootPattern {
        line: usize,
    },
    EmptySegment {
        line: usize,
    },
    TraversalPattern {
        line: usize,
    },
    InvalidRecursiveWildcard {
        line: usize,
    },
    DanglingEscape {
        line: usize,
    },
    InvalidEscape {
        line: usize,
    },
    NonUtf8Path(PathBuf),
    PathEscape(PathBuf),
    LinkedPath(PathBuf),
    NodeOutsideManagedBoundary {
        path: String,
        action: BoundaryAction,
    },
    CanonicalDocumentRule {
        path: String,
    },
    ManagedFileBoundary {
        path: String,
        action: BoundaryAction,
    },
}

impl fmt::Display for ContentRulesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "cannot read content rules: {error}"),
            Self::LinkedRulesFile => {
                formatter.write_str("content rules file cannot be a link or reparse point")
            }
            Self::RulesPathNotFile => {
                formatter.write_str("content rules path is not a regular file")
            }
            Self::RulesFileTooLarge => formatter.write_str("content rules file exceeds 1 MiB"),
            Self::MultipleRulesFiles => {
                formatter.write_str("workspace path crosses multiple content rules authorities")
            }
            Self::InvalidUtf8 => formatter.write_str("content rules file is not UTF-8"),
            Self::NulByte => formatter.write_str("content rules contain a NUL byte"),
            Self::MissingHeader => formatter.write_str("content rules header is missing"),
            Self::InvalidHeader { line } => {
                write!(formatter, "invalid content rules header on line {line}")
            }
            Self::LineTooLong { line } => {
                write!(formatter, "content rule line {line} exceeds 4096 bytes")
            }
            Self::InvalidSyntax { line } => {
                write!(formatter, "invalid content rule syntax on line {line}")
            }
            Self::UnknownAction { line } => {
                write!(formatter, "unknown content rule action on line {line}")
            }
            Self::AbsolutePattern { line } => {
                write!(formatter, "absolute content rule pattern on line {line}")
            }
            Self::RootPattern { line } => write!(
                formatter,
                "content rule cannot target the workspace root on line {line}"
            ),
            Self::EmptySegment { line } => {
                write!(formatter, "empty content rule path segment on line {line}")
            }
            Self::TraversalPattern { line } => {
                write!(formatter, "content rule traversal segment on line {line}")
            }
            Self::InvalidRecursiveWildcard { line } => write!(
                formatter,
                "** must occupy a complete segment on line {line}"
            ),
            Self::DanglingEscape { line } => {
                write!(formatter, "dangling content rule escape on line {line}")
            }
            Self::InvalidEscape { line } => {
                write!(formatter, "invalid content rule escape on line {line}")
            }
            Self::NonUtf8Path(path) => {
                write!(formatter, "workspace path is not UTF-8: {}", path.display())
            }
            Self::PathEscape(path) => write!(
                formatter,
                "workspace path escapes the root: {}",
                path.display()
            ),
            Self::LinkedPath(path) => write!(
                formatter,
                "workspace path crosses a link or reparse point: {}",
                path.display()
            ),
            Self::NodeOutsideManagedBoundary { path, action } => {
                write!(formatter, "node path {path} is classified as {action:?}")
            }
            Self::CanonicalDocumentRule { path } => {
                write!(
                    formatter,
                    "canonical node document {path} is classified separately"
                )
            }
            Self::ManagedFileBoundary { path, action } => {
                write!(
                    formatter,
                    "managed file path {path} is classified as {action:?}"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_rules_and_globs_are_deterministic() {
        let rules = parse_rules(
            "weftext-content-rules-v1\nunmanaged Notes/**/*.md\nignore Notes/private/**\nunmanaged Notes/private/public.md\n",
        )
        .expect("rules");
        assert_eq!(
            rules.classify("Notes/a.md", false),
            Some(BoundaryAction::Unmanaged)
        );
        assert_eq!(rules.classify("notes/a.md", false), None);
        assert_eq!(
            rules.classify("Notes/a/b.md", false),
            Some(BoundaryAction::Unmanaged)
        );
        assert_eq!(
            rules.classify("Notes/private/secret.md", false),
            Some(BoundaryAction::Ignore)
        );
        assert_eq!(
            rules.classify("Notes/private/public.md", false),
            Some(BoundaryAction::Unmanaged)
        );
    }

    #[test]
    fn escaping_is_literal_and_backslash_is_not_a_separator() {
        let rules = parse_rules("weftext-content-rules-v1\nunmanaged docs/a\\ b\\#c\\*.md\n")
            .expect("rules");
        assert_eq!(
            rules.classify("docs/a b#c*.md", false),
            Some(BoundaryAction::Unmanaged)
        );
        assert!(parse_rules("weftext-content-rules-v1\nunmanaged docs\\child.md\n").is_err());
    }

    #[test]
    fn traversal_absolute_and_ambiguous_recursion_fail_closed() {
        for source in [
            "weftext-content-rules-v1\nignore ../outside\n",
            "weftext-content-rules-v1\nignore /outside\n",
            "weftext-content-rules-v1\nignore ab**cd\n",
        ] {
            assert!(parse_rules(source).is_err(), "accepted {source:?}");
        }
    }
}
