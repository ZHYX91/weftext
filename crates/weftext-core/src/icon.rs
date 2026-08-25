use serde::{Deserialize, Serialize};

use crate::FrontmatterError;
use crate::frontmatter::{parse_node_icon_value, set_node_icon};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeIconKind {
    Emoji,
    BuiltIn,
}

/// A presentation-safe icon resolved from the canonical `weftext.icon` scalar.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedNodeIcon {
    pub kind: NodeIconKind,
    pub value: String,
    pub glyph: String,
}

/// The semantic fallback category for a workspace item. It is derived UI state
/// and is never persisted or used as identity/index authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceItemIconFallback {
    ManagedNode,
    UnmanagedFolder,
    UnmanagedMarkdown,
    OrdinaryFile,
    WorkspaceRoot,
    Trash,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "explicit", rename_all = "snake_case")]
pub enum WorkspaceItemIcon {
    ExplicitNode(ResolvedNodeIcon),
    DefaultNode,
    Folder,
    MarkdownFile,
    File,
    WorkspaceRoot,
    Trash,
}

#[must_use]
pub fn derive_workspace_item_icon(
    explicit_node_icon: Option<ResolvedNodeIcon>,
    fallback: WorkspaceItemIconFallback,
) -> WorkspaceItemIcon {
    if let Some(icon) = explicit_node_icon {
        return WorkspaceItemIcon::ExplicitNode(icon);
    }
    match fallback {
        WorkspaceItemIconFallback::ManagedNode => WorkspaceItemIcon::DefaultNode,
        WorkspaceItemIconFallback::UnmanagedFolder => WorkspaceItemIcon::Folder,
        WorkspaceItemIconFallback::UnmanagedMarkdown => WorkspaceItemIcon::MarkdownFile,
        WorkspaceItemIconFallback::OrdinaryFile => WorkspaceItemIcon::File,
        WorkspaceItemIconFallback::WorkspaceRoot => WorkspaceItemIcon::WorkspaceRoot,
        WorkspaceItemIconFallback::Trash => WorkspaceItemIcon::Trash,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltInNodeIcon {
    pub id: &'static str,
    pub label: &'static str,
    pub glyph: &'static str,
}

const BUILT_INS: [BuiltInNodeIcon; 8] = [
    BuiltInNodeIcon {
        id: "weftext:book",
        label: "书籍",
        glyph: "书",
    },
    BuiltInNodeIcon {
        id: "weftext:star",
        label: "星标",
        glyph: "星",
    },
    BuiltInNodeIcon {
        id: "weftext:idea",
        label: "灵感",
        glyph: "光",
    },
    BuiltInNodeIcon {
        id: "weftext:calendar",
        label: "日历",
        glyph: "日",
    },
    BuiltInNodeIcon {
        id: "weftext:archive",
        label: "归档",
        glyph: "藏",
    },
    BuiltInNodeIcon {
        id: "weftext:pin",
        label: "固定",
        glyph: "钉",
    },
    BuiltInNodeIcon {
        id: "weftext:project",
        label: "项目",
        glyph: "项",
    },
    BuiltInNodeIcon {
        id: "weftext:reference",
        label: "参考",
        glyph: "引",
    },
];

#[must_use]
pub const fn built_in_node_icons() -> &'static [BuiltInNodeIcon] {
    &BUILT_INS
}

#[must_use]
pub fn resolve_node_icon(value: &str) -> Option<ResolvedNodeIcon> {
    if let Some(icon) = BUILT_INS.iter().find(|icon| icon.id == value) {
        return Some(ResolvedNodeIcon {
            kind: NodeIconKind::BuiltIn,
            value: value.to_owned(),
            glyph: icon.glyph.to_owned(),
        });
    }
    (!value.starts_with("weftext:") && weftext_asciidoc::is_canonical_envelope_icon_scalar(value))
        .then(|| ResolvedNodeIcon {
            kind: NodeIconKind::Emoji,
            value: value.to_owned(),
            glyph: value.to_owned(),
        })
}

/// Reads the optional canonical `weftext.icon` scalar.
///
/// # Errors
///
/// The complete envelope is validated and fails closed on legacy top-level
/// icons, lists, duplicates, foreign tokens, or ambiguous YAML. A syntactically
/// valid future `weftext:*` token is preserved even when this client cannot render it.
pub fn read_node_icon_from_source(source: &str) -> Result<Option<String>, FrontmatterError> {
    parse_node_icon_value(source)
}

/// Narrowly sets or removes the canonical `weftext.icon` scalar.
///
/// # Errors
///
/// Foreign values and invalid or ambiguous envelopes are rejected without
/// changing source.
pub fn patch_node_icon_property(
    source: &str,
    value: Option<&str>,
) -> Result<String, FrontmatterError> {
    set_node_icon(source, value)
}

#[must_use]
pub fn resolve_node_icon_from_source(source: &str) -> Option<ResolvedNodeIcon> {
    read_node_icon_from_source(source)
        .ok()?
        .and_then(|value| resolve_node_icon(&value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    #[test]
    fn resolves_only_supported_portable_values() {
        assert_eq!(
            resolve_node_icon("😀").map(|icon| icon.kind),
            Some(NodeIconKind::Emoji)
        );
        assert_eq!(
            resolve_node_icon("weftext:book").map(|icon| icon.glyph),
            Some("书".to_owned())
        );
        assert!(resolve_node_icon("😀😺").is_none());
        assert!(resolve_node_icon("weftext:unknown").is_none());
        assert!(resolve_node_icon("image.png").is_none());
        assert!(resolve_node_icon("文").is_none());
    }

    #[test]
    fn workspace_item_icons_use_explicit_then_shared_fallback_semantics() {
        let explicit = resolve_node_icon("weftext:book").expect("supported icon");
        assert!(matches!(
            derive_workspace_item_icon(Some(explicit), WorkspaceItemIconFallback::WorkspaceRoot),
            WorkspaceItemIcon::ExplicitNode(_)
        ));
        assert_eq!(
            derive_workspace_item_icon(None, WorkspaceItemIconFallback::ManagedNode),
            WorkspaceItemIcon::DefaultNode
        );
        assert_eq!(
            derive_workspace_item_icon(None, WorkspaceItemIconFallback::UnmanagedFolder),
            WorkspaceItemIcon::Folder
        );
        assert_eq!(
            derive_workspace_item_icon(None, WorkspaceItemIconFallback::UnmanagedMarkdown),
            WorkspaceItemIcon::MarkdownFile
        );
        assert_eq!(
            derive_workspace_item_icon(None, WorkspaceItemIconFallback::OrdinaryFile),
            WorkspaceItemIcon::File
        );
    }

    #[test]
    fn reads_and_narrowly_patches_one_canonical_scalar() {
        let source = format!(
            "---\r\nweftext:\r\n  id: \"{ID}\"\r\n  aliases:\r\n    - 文缕\r\n  icon: '😀'\r\n---\r\n= Title\r\n"
        );
        assert_eq!(
            read_node_icon_from_source(&source),
            Ok(Some("😀".to_owned()))
        );
        let selected =
            patch_node_icon_property(&source, Some("weftext:book")).expect("select icon");
        assert_eq!(selected, source.replacen("'😀'", "\"weftext:book\"", 1));
        let cleared = patch_node_icon_property(&selected, None).expect("clear icon");
        assert!(!cleared.contains("  icon:"));
        assert!(cleared.contains("  aliases:\r\n    - 文缕\r\n"));
    }

    #[test]
    fn legacy_list_duplicate_and_foreign_icons_fail_closed() {
        let list = format!("---\nweftext:\n  id: \"{ID}\"\n  icon: [weftext:book, 😀]\n---\n");
        assert!(read_node_icon_from_source(&list).is_err());
        let duplicate = format!("---\nweftext:\n  id: \"{ID}\"\n  icon: 😀\n  icon: 😺\n---\n");
        assert!(patch_node_icon_property(&duplicate, Some("weftext:book")).is_err());
        let unknown = format!("---\nweftext:\n  id: \"{ID}\"\n  icon: vendor:custom\n---\n");
        assert!(resolve_node_icon_from_source(&unknown).is_none());
        assert_eq!(
            patch_node_icon_property(&unknown, Some("weftext:book")),
            Err(FrontmatterError::InvalidIcon)
        );
        let future = format!("---\nweftext:\n  id: \"{ID}\"\n  icon: weftext:future-icon\n---\n");
        assert_eq!(
            read_node_icon_from_source(&future),
            Ok(Some("weftext:future-icon".to_owned()))
        );
        assert!(resolve_node_icon_from_source(&future).is_none());
    }
}
