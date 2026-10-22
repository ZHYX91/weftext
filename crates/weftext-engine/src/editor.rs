//! Exact-snapshot native authoring. No display text or generated span is a write address.
use crate::{editor_types::*, encode_literal, inspect::Utf16Offsets, types::*};
use sha2::{Digest, Sha256};
use std::ops::Range;
use typst_syntax::{
    LinkedNode, Source, SyntaxKind, SyntaxNode,
    ast::{self, AstNode},
};
#[path = "everyday.rs"]
mod everyday;
#[path = "table_clipboard.rs"]
mod table_clipboard;
#[path = "table_editor.rs"]
mod table_editor;

pub fn source_hash(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}

pub(crate) fn source_range(source: &str, range: Range<usize>) -> SourceRange {
    SourceRange {
        from: range.start,
        to: range.end,
        utf16_from: utf16(source, range.start),
        utf16_to: utf16(source, range.end),
        raw: source[range].into(),
        writable: true,
    }
}

fn indexed_source_range(
    source: &str,
    offsets: &Utf16Offsets<'_>,
    range: Range<usize>,
) -> SourceRange {
    SourceRange {
        from: range.start,
        to: range.end,
        utf16_from: offsets.at(range.start),
        utf16_to: offsets.at(range.end),
        raw: source[range].into(),
        writable: true,
    }
}

fn absent() -> DocumentRole {
    DocumentRole {
        status: "absent".into(),
        text: None,
        range: None,
        construct: None,
        encoding: None,
    }
}

/// A source-only qualified map. Compiled output must additionally match this map
/// before a displayed region is attached to an input control.
pub fn inspect_editor(source: &str) -> EditorMap {
    let parsed = Source::detached(source);
    let valid = parsed.root().errors_and_warnings().0.is_empty();
    let root = LinkedNode::new(parsed.root());
    let mut map = EditorMap {
        source_hash: source_hash(source),
        syntax_valid: valid,
        literal_regions: vec![],
        structures: vec![],
        title: absent(),
        subtitle: absent(),
        body_insertion: None,
        unavailable_reason: None,
        notes: crate::inspect_notes_source(source),
        heading_numbering: crate::inspect_heading_numbering(source),
        body_blocks: vec![],
    };
    if !valid {
        map.unavailable_reason =
            Some(weftext_messages::UiMessage::error_engine_reason_syntax_invalid().into());
        return map;
    }
    // Arbitrary show transformations make source-to-display inversion unknown.
    // Recognize only the exact native per-label recipe owned by this adapter.
    if has_unknown_show(&root, source) {
        map.unavailable_reason =
            Some(weftext_messages::UiMessage::error_engine_reason_show_rule_unverified().into());
        return map;
    }
    let offsets = Utf16Offsets::new(source);
    collect_markup(&root, source, &parsed, &offsets, "body", &mut map);
    map.body_blocks = everyday::blocks(source);
    let heading_levels = heading_levels_unmodified(parsed.root());
    for structure in &mut map.structures {
        if structure.kind == "heading" {
            structure.level_writable = heading_levels;
            structure.level_reason = (!heading_levels).then(|| {
                weftext_messages::UiMessage::error_engine_reason_heading_level_rule().into()
            });
        }
    }
    roles(&root, source, &parsed, &mut map);
    let children: Vec<_> = root.children().collect();
    if children
        .last()
        .is_none_or(|n| n.kind() != SyntaxKind::LineComment)
    {
        map.body_insertion = Some(source_range(source, source.len()..source.len()));
    }
    if let Some(at) = crate::notes::trailing_endnotes_insertion(source) {
        map.body_insertion = Some(source_range(source, at..at));
    }
    map
}

fn heading_levels_unmodified(node: &SyntaxNode) -> bool {
    if let Some(set) = node.cast::<ast::SetRule>()
        && set.target().to_untyped().cast::<ast::Ident>().is_some_and(|i| i.get() == "heading")
        && set.args().items().any(|a| matches!(a, ast::Arg::Spread(_)) || matches!(a, ast::Arg::Named(n) if matches!(n.name().get().as_str(), "level" | "depth" | "offset")))
    {
        return false;
    }
    node.children().all(heading_levels_unmodified)
}

pub fn inspect_editor_with_style(source: &str, style: &crate::StyleModule) -> EditorMap {
    if crate::style_allows_rich_editing(style) {
        return inspect_editor(source);
    }
    let mut map = inspect_editor(source);
    map.literal_regions.clear();
    map.structures.clear();
    map.body_blocks.clear();
    map.body_insertion = None;
    map.title = absent();
    map.title.status = "unverified-library-style".into();
    map.subtitle = absent();
    map.subtitle.status = "unverified-library-style".into();
    map.unavailable_reason =
        Some(weftext_messages::UiMessage::error_engine_reason_library_style_unverified().into());
    map
}

pub fn edit_with_style(
    source: &str,
    request: &EditRequest,
    style: &crate::StyleModule,
) -> Result<EditOutput, EngineError> {
    if !crate::style_allows_rich_editing(style) {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_rich_style_unverified(),
        ));
    }
    edit(source, request)
}

fn has_unknown_show(node: &LinkedNode<'_>, source: &str) -> bool {
    if node.kind() == SyntaxKind::ShowRule {
        if crate::numbering::verified_numbering_header_range(source)
            .is_some_and(|range| node.range().start >= range.start && node.range().end <= range.end)
        {
            return false;
        }
        return !run_in_label(&source[node.range()])
            .is_some_and(|label| source[node.range()] == run_in_recipe(&label));
    }
    node.children().any(|c| has_unknown_show(&c, source))
}

fn run_in_recipe(label: &str) -> String {
    format!(
        "show <{label}>: it => context {{\n  strong[\n    #if it.numbering != none {{\n      counter(heading).display(it.numbering)\n      text(\" \")\n    }}\n    #it.body\n  ]\n  text(\" \")\n}}"
    )
}
fn run_in_label(raw: &str) -> Option<String> {
    let rest = raw.strip_prefix("show <")?;
    let label = rest.split_once('>')?.0;
    (!label.is_empty()
        && label
            .chars()
            .all(|c| c.is_alphanumeric() || "-_:".contains(c) || c == '.'))
    .then(|| label.into())
}

fn literal_piece(node: &LinkedNode<'_>, _source: &str) -> Option<String> {
    match node.kind() {
        SyntaxKind::Text => Some(node.get().cast::<ast::Text>()?.get().to_string()),
        SyntaxKind::Escape => Some(node.get().cast::<ast::Escape>()?.get().to_string()),
        SyntaxKind::Shorthand => Some(node.get().cast::<ast::Shorthand>()?.get().to_string()),
        SyntaxKind::Space => Some(" ".into()),
        _ => None,
    }
}

fn cell_literal_piece(node: &LinkedNode<'_>, source: &str) -> Option<String> {
    match node.kind() {
        SyntaxKind::Linebreak => Some("\n".into()),
        SyntaxKind::Space
            if node
                .prev_sibling_with_trivia()
                .is_some_and(|n| n.kind() == SyntaxKind::Linebreak) =>
        {
            Some(String::new())
        }
        _ => literal_piece(node, source),
    }
}

/// Cell text has real line breaks; an ordinary native soft source wrap remains
/// distinct. CRLF clipboard line endings normalize to one logical break.
fn encode_cell_literal(text: &str) -> Result<String, EngineError> {
    if text.len() > 1_048_576 || text.contains('\0') {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_cell_text_invalid(),
        ));
    }
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(encode_literal)
        .collect::<Result<Vec<_>, _>>()
        .map(|lines| lines.join("\\\n"))
}

fn collect_markup(
    node: &LinkedNode<'_>,
    source: &str,
    parsed: &Source,
    offsets: &Utf16Offsets<'_>,
    role: &str,
    map: &mut EditorMap,
) {
    let children: Vec<_> = node.children().collect();
    if node.kind() == SyntaxKind::Markup {
        let mut run = vec![];
        fn flush(
            run: &mut Vec<LinkedNode<'_>>,
            source: &str,
            offsets: &Utf16Offsets<'_>,
            role: &str,
            map: &mut EditorMap,
        ) {
            let owns = |n: &LinkedNode<'_>| {
                !matches!(n.kind(), SyntaxKind::Space | SyntaxKind::Parbreak)
                    || role == "tableCell"
                        && n.kind() == SyntaxKind::Space
                        && n.prev_sibling_with_trivia()
                            .is_some_and(|previous| previous.kind() == SyntaxKind::Linebreak)
            };
            let start = run.iter().position(owns);
            let end = run.iter().rposition(owns);
            if let (Some(start), Some(end)) = (start, end) {
                let run = &run[start..=end];
                let text = run
                    .iter()
                    .map(|n| {
                        if role == "tableCell" {
                            cell_literal_piece(n, source)
                        } else {
                            literal_piece(n, source)
                        }
                        .unwrap_or_else(|| "\n\n".into())
                    })
                    .collect();
                let paragraph_breaks = run
                    .iter()
                    .filter(|n| n.kind() == SyntaxKind::Parbreak)
                    .map(|n| indexed_source_range(source, offsets, n.range()))
                    .collect();
                map.literal_regions.push(LiteralRegion {
                    range: indexed_source_range(
                        source,
                        offsets,
                        run[0].range().start..run.last().unwrap().range().end,
                    ),
                    text,
                    role: role.into(),
                    paragraph_breaks,
                });
            }
            run.clear();
        }
        for child in &children {
            let piece = (if role == "tableCell" {
                cell_literal_piece(child, source)
            } else {
                literal_piece(child, source)
            })
            .or_else(|| {
                (matches!(role, "body" | "tableCell") && child.kind() == SyntaxKind::Parbreak)
                    .then(|| "\n\n".into())
            });
            if piece.is_some() {
                run.push(child.clone());
            } else {
                flush(&mut run, source, offsets, role, map);
            }
        }
        flush(&mut run, source, offsets, role, map);
        if children.is_empty() {
            map.literal_regions.push(LiteralRegion {
                range: indexed_source_range(source, offsets, node.range()),
                text: String::new(),
                role: role.into(),
                paragraph_breaks: vec![],
            });
        }
    }
    for (child_index, child) in children.iter().enumerate() {
        if child.kind() == SyntaxKind::Markup {
            collect_markup(child, source, parsed, offsets, role, map);
            continue;
        }
        let (kind, body, level) = if let Some(h) = child.get().cast::<ast::Heading>() {
            (
                "heading",
                Some(h.body().to_untyped()),
                Some(h.depth().get()),
            )
        } else if let Some(s) = child.get().cast::<ast::Strong>() {
            ("strong", Some(s.body().to_untyped()), None)
        } else if let Some(s) = child.get().cast::<ast::Emph>() {
            ("emph", Some(s.body().to_untyped()), None)
        } else if let Some(s) = child.get().cast::<ast::ListItem>() {
            ("bulletListItem", Some(s.body().to_untyped()), None)
        } else if let Some(s) = child.get().cast::<ast::EnumItem>() {
            ("numberedListItem", Some(s.body().to_untyped()), None)
        } else if let Some(s) = child.get().cast::<ast::TermItem>() {
            if let Some(term) = parsed.find(s.term().to_untyped().span()) {
                collect_markup(&term, source, parsed, offsets, "term", map);
            }
            ("termListItem", Some(s.description().to_untyped()), None)
        } else if let Some(eq) = child.get().cast::<ast::Equation>() {
            map.structures.push(StructureInfo {
                kind: if eq.block() {
                    "displayMath"
                } else {
                    "inlineMath"
                }
                .into(),
                range: indexed_source_range(source, offsets, child.range()),
                body: parsed
                    .find(eq.body().to_untyped().span())
                    .map(|n| indexed_source_range(source, offsets, n.range())),
                level: None,
                level_writable: false,
                level_reason: None,
                label: None,
                run_in: false,
                table: None,
                language: None,
            });
            continue;
        } else if let Some(raw) = child.get().cast::<ast::Raw>() {
            let lines: Vec<_> = raw
                .lines()
                .filter_map(|line| parsed.find(line.to_untyped().span()))
                .collect();
            let body = lines.first().zip(lines.last()).map(|(first, last)| {
                indexed_source_range(source, offsets, first.range().start..last.range().end)
            });
            map.structures.push(StructureInfo {
                kind: if raw.block() { "blockRaw" } else { "inlineRaw" }.into(),
                range: indexed_source_range(source, offsets, child.range()),
                body,
                level: None,
                level_writable: false,
                level_reason: None,
                label: None,
                language: raw.lang().map(|lang| lang.get().to_string()),
                run_in: false,
                table: None,
            });
            continue;
        } else {
            ("", None, None)
        };
        if !kind.is_empty() {
            let body_node = body.and_then(|b| parsed.find(b.span()));
            let label = children
                .iter()
                .skip(child_index + 1)
                .find(|n| !n.kind().is_trivia())
                .and_then(|n| n.get().cast::<ast::Label>())
                .map(|l| l.get().to_string());
            let run_in = label
                .as_ref()
                .is_some_and(|l| source.contains(&format!("#{}", run_in_recipe(l))));
            map.structures.push(StructureInfo {
                kind: kind.into(),
                range: indexed_source_range(source, offsets, child.range()),
                body: body_node
                    .as_ref()
                    .map(|n| indexed_source_range(source, offsets, n.range())),
                level,
                level_writable: false,
                level_reason: None,
                label,
                run_in,
                table: None,
                language: None,
            });
            if let Some(body) = body_node {
                collect_markup(&body, source, parsed, offsets, kind, map);
            }
            continue;
        }
        // Bare native content expressions preserve their own lexical scope.
        // The table merger uses these wrappers to keep one cell's set/let from
        // affecting the next; their explicit inner literals remain editable.
        if let Some(block) = child.get().cast::<ast::ContentBlock>()
            && let Some(body) = parsed.find(block.body().to_untyped().span())
        {
            collect_markup(&body, source, parsed, offsets, role, map);
            continue;
        }
        if let Some(call) = child.get().cast::<ast::FuncCall>() {
            let Some(id) = call.callee().to_untyped().cast::<ast::Ident>() else {
                continue;
            };
            let name = id.get().as_str();
            if !standard_binding_available(source, name) {
                continue;
            }
            if name == "table" {
                collect_table(child, call, source, parsed, offsets, map);
                continue;
            }
            if matches!(
                name,
                "title" | "block" | "strike" | "link" | "box" | "strong" | "emph" | "text"
            ) {
                for arg in call.args().items() {
                    if let ast::Arg::Pos(expr) = arg
                        && let Some(b) = expr.to_untyped().cast::<ast::ContentBlock>()
                        && let Some(body) = parsed.find(b.body().to_untyped().span())
                    {
                        if name == "strike" {
                            map.structures.push(StructureInfo {
                                kind: name.into(),
                                range: indexed_source_range(source, offsets, child.range()),
                                body: Some(indexed_source_range(source, offsets, body.range())),
                                level: None,
                                level_writable: false,
                                level_reason: None,
                                label: None,
                                run_in: false,
                                table: None,
                                language: None,
                            });
                        }
                        collect_markup(&body, source, parsed, offsets, name, map);
                    }
                }
            }
        }
    }
}

fn collect_table(
    node: &LinkedNode<'_>,
    call: ast::FuncCall<'_>,
    source: &str,
    parsed: &Source,
    offsets: &Utf16Offsets<'_>,
    map: &mut EditorMap,
) {
    let mut columns = None;
    let mut cells = vec![];
    let mut simple = true;
    let mut columns_seen = false;
    for arg in call.args().items() {
        match arg {
            ast::Arg::Named(n) if n.name().get() == "columns" => {
                if columns_seen {
                    simple = false;
                }
                columns_seen = true;
                columns = n
                    .expr()
                    .to_untyped()
                    .cast::<ast::Int>()
                    .and_then(|i| usize::try_from(i.get()).ok())
                    .filter(|n| *n > 0);
                if columns.is_none() {
                    simple = false;
                }
            }
            ast::Arg::Named(_) => simple = false,
            ast::Arg::Pos(e) => {
                if let Some(block) = e.to_untyped().cast::<ast::ContentBlock>()
                    && let Some(body) = parsed.find(block.body().to_untyped().span())
                {
                    cells.push(indexed_source_range(source, offsets, body.range()));
                    collect_markup(&body, source, parsed, offsets, "tableCell", map);
                } else {
                    simple = false;
                    collect_complex_cells(e.to_untyped(), source, parsed, offsets, map, &mut cells);
                }
            }
            _ => simple = false,
        }
    }
    let columns = columns.unwrap_or(1);
    let grid_writable = simple
        && !cells.is_empty()
        && cells.len().is_multiple_of(columns)
        && !uncertain_table_trivia(node.get());
    map.structures.push(StructureInfo {
        kind: "table".into(),
        range: indexed_source_range(source, offsets, node.range()),
        body: None,
        level: None,
        level_writable: false,
        level_reason: None,
        label: None,
        run_in: false,
        table: Some(table_editor::inspect(
            node,
            call,
            source,
            parsed,
            columns,
            cells,
            grid_writable,
        )),
        language: None,
    });
}

fn uncertain_table_trivia(node: &SyntaxNode) -> bool {
    if node.kind() == SyntaxKind::ContentBlock {
        return false;
    }
    matches!(
        node.kind(),
        SyntaxKind::LineComment | SyntaxKind::BlockComment
    ) || node.children().any(uncertain_table_trivia)
}

fn collect_complex_cells(
    node: &SyntaxNode,
    source: &str,
    parsed: &Source,
    offsets: &Utf16Offsets<'_>,
    map: &mut EditorMap,
    cells: &mut Vec<SourceRange>,
) {
    let Some(call) = node.cast::<ast::FuncCall>() else {
        return;
    };
    let Some(field) = call.callee().to_untyped().cast::<ast::FieldAccess>() else {
        return;
    };
    if !field
        .target()
        .to_untyped()
        .cast::<ast::Ident>()
        .is_some_and(|i| i.get() == "table")
        || !matches!(field.field().get().as_str(), "cell" | "header" | "footer")
    {
        return;
    }
    for arg in call.args().items() {
        match arg {
            ast::Arg::Pos(_) | ast::Arg::Named(_) => {
                let expr = match arg {
                    ast::Arg::Pos(e) => e,
                    ast::Arg::Named(n) if n.name().get() == "body" => n.expr(),
                    _ => continue,
                };
                if let Some(block) = expr.to_untyped().cast::<ast::ContentBlock>()
                    && let Some(body) = parsed.find(block.body().to_untyped().span())
                {
                    cells.push(indexed_source_range(source, offsets, body.range()));
                    collect_markup(&body, source, parsed, offsets, "tableCell", map);
                } else {
                    collect_complex_cells(expr.to_untyped(), source, parsed, offsets, map, cells);
                }
            }
            _ => {}
        }
    }
}

fn roles(root: &LinkedNode<'_>, source: &str, parsed: &Source, map: &mut EditorMap) {
    if !standard_binding_available(source, "title")
        || !standard_binding_available(source, "document")
    {
        map.title.status = "unverified-binding".into();
        return;
    }
    let mut metadata = vec![];
    let mut direct = vec![];
    let mut render_calls = vec![];
    let mut subtitles = vec![];
    let children: Vec<_> = root.children().collect();
    for (index, node) in children.iter().enumerate() {
        if let Some(set) = node.get().cast::<ast::SetRule>()
            && set
                .target()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "document")
        {
            for arg in set.args().items() {
                if let ast::Arg::Named(n) = arg
                    && n.name().get() == "title"
                {
                    metadata.push((node, n.expr().to_untyped()));
                }
            }
        }
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && let Some(id) = call.callee().to_untyped().cast::<ast::Ident>()
        {
            if id.get() == "title" {
                render_calls.push(node);
                for arg in call.args().items() {
                    if let ast::Arg::Pos(e) = arg {
                        direct.push((node, e.to_untyped()));
                    }
                }
            }
            if id.get() == "block"
                && let Some(label) = children
                    .iter()
                    .skip(index + 1)
                    .find(|n| !n.kind().is_trivia())
                && label
                    .get()
                    .cast::<ast::Label>()
                    .is_some_and(|l| l.get() == "weftext-subtitle")
            {
                for arg in call.args().items() {
                    if let ast::Arg::Pos(e) = arg {
                        subtitles.push((node, e.to_untyped(), label));
                    }
                }
            }
        }
    }
    let chosen = match (metadata.as_slice(), direct.as_slice()) {
        ([(node, value)], []) if render_calls.len() == 1 => Some((*node, *value)),
        ([], [(node, value)]) if render_calls.len() == 1 => Some((*node, *value)),
        ([], []) if render_calls.len() <= 1 => {
            if let Some(call) = render_calls.first() {
                map.title.construct = Some(source_range(source, including_hash(call, source)));
            }
            None
        }
        _ => {
            map.title.status = "ambiguous".into();
            None
        }
    };
    if let Some((node, value)) = chosen {
        if let Some(range) = value_body(value, parsed, source)
            && let Some(text) = value
                .cast::<ast::Str>()
                .map(|s| s.get().to_string())
                .or_else(|| decode_literal(&range.raw))
        {
            map.title = DocumentRole {
                status: "writable".into(),
                text: Some(text),
                range: Some(range),
                construct: Some(source_range(source, including_hash(node, source))),
                encoding: Some(
                    if value.cast::<ast::Str>().is_some() {
                        "string"
                    } else {
                        "markup"
                    }
                    .into(),
                ),
            };
        } else {
            map.title.status = "computed-or-complex".into();
        }
    }
    match subtitles.as_slice() {
        [(node, value, label)]
            if map.title.status == "writable" && standard_binding_available(source, "block") =>
        {
            if let Some(range) = value_body(value, parsed, source)
                && let Some(text) = decode_literal(&range.raw)
            {
                map.subtitle = DocumentRole {
                    status: "writable".into(),
                    text: Some(text),
                    range: Some(range),
                    construct: Some(source_range(
                        source,
                        including_hash(node, source).start..label.range().end,
                    )),
                    encoding: Some("markup".into()),
                };
            } else {
                map.subtitle.status = "computed-or-complex".into();
            }
        }
        [] => {}
        [_] => map.subtitle.status = "missing-unique-title".into(),
        _ => map.subtitle.status = "ambiguous".into(),
    }
    fn count_label(node: &SyntaxNode) -> usize {
        usize::from(
            node.cast::<ast::Label>()
                .is_some_and(|l| l.get() == "weftext-subtitle"),
        ) + node.children().map(count_label).sum::<usize>()
    }
    let labels = count_label(root.get());
    if labels != subtitles.len() {
        map.subtitle = absent();
        map.subtitle.status = "unsupported-role-label".into();
    }
    if labels > 1 {
        map.subtitle = absent();
        map.subtitle.status = "ambiguous".into();
    }
}

fn value_body(node: &SyntaxNode, parsed: &Source, source: &str) -> Option<SourceRange> {
    if let Some(block) = node.cast::<ast::ContentBlock>() {
        Some(source_range(
            source,
            parsed.find(block.body().to_untyped().span())?.range(),
        ))
    } else if node.cast::<ast::Str>().is_some() {
        Some(source_range(source, parsed.find(node.span())?.range()))
    } else {
        None
    }
}

fn including_hash(node: &LinkedNode<'_>, source: &str) -> Range<usize> {
    let range = node.range();
    if range.start > 0 && source.as_bytes()[range.start - 1] == b'#' {
        range.start - 1..range.end
    } else {
        range
    }
}

/// Conservative binding certificate shared by application feature adapters.
pub fn standard_binding_available(source: &str, name: &str) -> bool {
    let parsed = Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return false;
    }
    fn safe(node: &SyntaxNode, name: &str) -> bool {
        if matches!(
            node.kind(),
            SyntaxKind::ModuleImport | SyntaxKind::ModuleInclude
        ) {
            return false;
        }
        if node
            .cast::<ast::LetBinding>()
            .is_some_and(|b| b.kind().bindings().iter().any(|i| i.get() == name))
        {
            return false;
        }
        if node
            .cast::<ast::DestructAssignment>()
            .is_some_and(|b| b.pattern().bindings().iter().any(|i| i.get() == name))
        {
            return false;
        }
        if let Some(binary) = node.cast::<ast::Binary>()
            && matches!(
                binary.op(),
                ast::BinOp::Assign
                    | ast::BinOp::AddAssign
                    | ast::BinOp::SubAssign
                    | ast::BinOp::MulAssign
                    | ast::BinOp::DivAssign
            )
            && mentions(binary.lhs().to_untyped(), name)
        {
            return false;
        }
        if node.kind() == SyntaxKind::Params && mentions(node, name) {
            return false;
        }
        node.children().all(|n| safe(n, name))
    }
    fn mentions(node: &SyntaxNode, name: &str) -> bool {
        node.cast::<ast::Ident>().is_some_and(|i| i.get() == name)
            || node.children().any(|n| mentions(n, name))
    }
    safe(parsed.root(), name)
}

fn decode_literal(raw: &str) -> Option<String> {
    let parsed = Source::detached(raw);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return None;
    }
    let root = LinkedNode::new(parsed.root());
    root.children().map(|n| literal_piece(&n, raw)).collect()
}

/// Quoted native string, re-read through the official native string AST.
pub fn encode_string(text: &str) -> Result<String, EngineError> {
    if text.contains('\0') || text.len() > 1_048_576 {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_string_invalid(),
        ));
    }
    let mut encoded = String::from("\"");
    for character in text.chars() {
        match character {
            '\\' => encoded.push_str("\\\\"),
            '"' => encoded.push_str("\\\""),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            c if c.is_control() => encoded.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => encoded.push(c),
        }
    }
    encoded.push('"');
    let parsed = Source::detached(format!("#{encoded}"));
    if !parsed.root().errors_and_warnings().0.is_empty()
        || !parsed
            .root()
            .children()
            .find_map(|n| n.cast::<ast::Str>())
            .is_some_and(|s| s.get() == text)
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_native_string_readback_failed(),
        ));
    }
    Ok(encoded)
}

pub fn edit(source: &str, request: &EditRequest) -> Result<EditOutput, EngineError> {
    if request.source_hash != source_hash(source) {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_source_stale(),
        ));
    }
    let map = inspect_editor(source);
    if !map.syntax_valid {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_source_syntax_invalid(),
        ));
    }
    if let Some(reason) = &map.unavailable_reason {
        return Err(EngineError::new(reason.clone()));
    }
    let mut patches = vec![];
    let mut selection_override = None;
    match &request.intent {
        EditIntent::ConvertBodyBlock {
            range,
            target,
            level,
        } => everyday::convert(source, &map, range, target, *level, &mut patches)?,
        EditIntent::MoveBodyBlock { range, direction } => {
            everyday::move_block(source, &map, range, direction, &mut patches)?
        }
        EditIntent::ReplaceLiteral { range, text } => {
            let region = map
                .literal_regions
                .iter()
                .find(|r| same_range(&r.range, range))
                .ok_or_else(|| {
                    EngineError::new(
                        weftext_messages::UiMessage::error_engine_editor_text_slot_unavailable(),
                    )
                })?;
            literal_patch(source, region, text, &mut patches)?;
        }
        EditIntent::InsertBlock { at, block } => {
            if matches!(block, BlockInsert::Image { .. })
                && !standard_binding_available(source, "image")
            {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_image_binding_unverified(),
                ));
            }
            if !map.body_insertion.as_ref().is_some_and(|r| r.from == *at) {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_body_insertion_unavailable(),
                ));
            }
            let body = encode_block(block)?;
            let before = &source[..*at];
            let prefix = if before.is_empty() || before.ends_with("\n\n") {
                String::new()
            } else if before.ends_with('\n') {
                "\n".into()
            } else {
                "\n\n".into()
            };
            let suffix = if *at < source.len() { "\n\n" } else { "" };
            push_patch(
                &mut patches,
                source,
                *at..*at,
                format!("{prefix}{body}{suffix}"),
            );
        }
        EditIntent::SetHeadingLevel { range, level } => {
            check_level(*level)?;
            let structure = find_structure(&map, range, "heading")?;
            if !structure.level_writable {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_heading_level_unverified(),
                ));
            }
            let marker = structure
                .range
                .raw
                .bytes()
                .take_while(|b| *b == b'=')
                .count();
            push_patch(
                &mut patches,
                source,
                range.from..range.from + marker,
                "=".repeat(*level),
            );
        }
        EditIntent::SetTitle { text } => edit_title(source, &map, text.as_deref(), &mut patches)?,
        EditIntent::SetSubtitle { text } => {
            edit_subtitle(source, &map, text.as_deref(), &mut patches)?
        }
        EditIntent::SetRunIn { range, enabled } => {
            edit_run_in(source, &map, range, *enabled, &mut patches)?
        }
        EditIntent::SetInline { range, format } => {
            exact_range(source, range)?;
            selection_override = Some(inline_edit(source, &map, range, *format, &mut patches)?);
        }
        EditIntent::ReplaceMath {
            range,
            text,
            display,
        } => {
            if !map.structures.iter().any(|s| {
                same_range(&s.range, range)
                    && matches!(s.kind.as_str(), "inlineMath" | "displayMath")
            }) {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_math_unavailable(),
                ));
            }
            push_patch(
                &mut patches,
                source,
                range.from..range.to,
                encode_math(text, *display)?,
            );
        }
        EditIntent::ReplaceRaw {
            range,
            text,
            language,
            block,
        } => {
            if !map.structures.iter().any(|s| {
                same_range(&s.range, range) && matches!(s.kind.as_str(), "inlineRaw" | "blockRaw")
            }) {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_raw_unavailable(),
                ));
            }
            push_patch(
                &mut patches,
                source,
                range.from..range.to,
                encode_block(&BlockInsert::Raw {
                    text: text.clone(),
                    language: language.clone(),
                    block: *block,
                })?,
            );
        }
        EditIntent::SetTableCell { range, text } => {
            if !map
                .structures
                .iter()
                .filter_map(|s| s.table.as_ref())
                .flat_map(|t| &t.cells)
                .any(|c| same_range(c, range))
            {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_cell_slot_unavailable(),
                ));
            }
            push_patch(
                &mut patches,
                source,
                range.from..range.to,
                encode_cell_literal(text)?,
            );
        }
        EditIntent::TableEdit { range, action } => {
            edit_table(source, &map, range, action, &mut patches)?
        }
        EditIntent::NativeNote { action } => {
            let updated = crate::edit_notes(source, action)?;
            patch_changed_source(source, &updated, &mut patches);
        }
        EditIntent::HeadingNumbering { action } => {
            let updated = crate::edit_heading_numbering(source, action)?;
            patch_changed_source(source, &updated, &mut patches);
        }
        EditIntent::ListEdit { range, action } => {
            edit_list(source, &map, range, action, &mut patches)?
        }
    }
    let mut out = finish(source, patches)?;
    if let EditIntent::ListEdit {
        range,
        action: ListAction::Split { .. },
    } = &request.intent
        && let Some(body) = out
            .map
            .structures
            .iter()
            .filter(|s| {
                matches!(s.kind.as_str(), "bulletListItem" | "numberedListItem")
                    && s.range.from > range.from
            })
            .min_by_key(|s| s.range.from)
            .and_then(|s| s.body.as_ref())
    {
        selection_override = Some(
            out.map
                .literal_regions
                .iter()
                .find(|r| r.range.from >= body.from && r.range.to <= body.to)
                .map_or(body.from, |r| r.range.from),
        );
    }
    if let Some(at) = selection_override {
        let at = utf16(&out.source, at);
        out.selection = EditSelection { from: at, to: at };
    }
    Ok(out)
}

fn same_range(a: &SourceRange, b: &SourceRange) -> bool {
    a.from == b.from
        && a.to == b.to
        && a.raw == b.raw
        && a.utf16_from == b.utf16_from
        && a.utf16_to == b.utf16_to
        && b.writable
}

/// Keep a helper's unchanged source outside its smallest UTF-8 edit boundary.
fn patch_changed_source(source: &str, updated: &str, patches: &mut Vec<SourcePatch>) {
    if source == updated {
        return;
    }
    let prefix = source
        .chars()
        .zip(updated.chars())
        .take_while(|(a, b)| a == b)
        .map(|(ch, _)| ch.len_utf8())
        .sum::<usize>();
    let suffix = source[prefix..]
        .chars()
        .rev()
        .zip(updated[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(ch, _)| ch.len_utf8())
        .sum::<usize>();
    push_patch(
        patches,
        source,
        prefix..source.len() - suffix,
        updated[prefix..updated.len() - suffix].into(),
    );
}

fn literal_patch(
    source: &str,
    region: &LiteralRegion,
    text: &str,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    let old = &region.text;
    if old == text {
        return Ok(());
    }
    if text.is_empty()
        && matches!(
            region.role.as_str(),
            "heading" | "bulletListItem" | "numberedListItem" | "termListItem"
        )
    {
        push_patch(
            patches,
            source,
            region.range.from..region.range.to,
            "#box[]".into(),
        );
        return Ok(());
    }
    let mut prefix = old
        .chars()
        .zip(text.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let mut suffix = old[prefix..]
        .chars()
        .rev()
        .zip(text[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let raw = &region.range.raw;
    let parsed = Source::detached(raw);
    let root = LinkedNode::new(parsed.root());
    let mut boundaries = std::collections::BTreeMap::from([(0usize, 0usize)]);
    let mut displayed = 0;
    for node in root.children() {
        let piece = (if region.role == "tableCell" {
            cell_literal_piece(&node, raw)
        } else {
            literal_piece(&node, raw)
        })
        .or_else(|| (node.kind() == SyntaxKind::Parbreak).then(|| "\n\n".into()))
        .ok_or_else(|| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_editor_text_readback_nontext(),
            )
        })?;
        if node.kind() == SyntaxKind::Text {
            for (at, c) in piece.char_indices() {
                boundaries.insert(displayed + at, node.range().start + at);
                boundaries.insert(
                    displayed + at + c.len_utf8(),
                    node.range().start + at + c.len_utf8(),
                );
            }
        } else {
            boundaries.insert(displayed, node.range().start);
            boundaries.insert(displayed + piece.len(), node.range().end);
        }
        displayed += piece.len();
    }
    while !boundaries.contains_key(&prefix) {
        prefix -= 1;
    }
    while !boundaries.contains_key(&(old.len() - suffix)) {
        suffix -= 1;
    }
    let from = *boundaries.get(&prefix).ok_or_else(|| {
        EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_text_boundary_unavailable(),
        )
    })?;
    let to = *boundaries.get(&(old.len() - suffix)).ok_or_else(|| {
        EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_text_boundary_unavailable(),
        )
    })?;
    push_patch(
        patches,
        source,
        region.range.from + from..region.range.from + to,
        if region.role == "tableCell" {
            encode_cell_literal(&text[prefix..text.len() - suffix])?
        } else {
            encode_literal(&text[prefix..text.len() - suffix])?
        },
    );
    Ok(())
}
fn exact_range(source: &str, range: &SourceRange) -> Result<(), EngineError> {
    if range.from > range.to
        || !source.is_char_boundary(range.from)
        || !source.is_char_boundary(range.to)
        || source.get(range.from..range.to) != Some(range.raw.as_str())
        || utf16(source, range.from) != range.utf16_from
        || utf16(source, range.to) != range.utf16_to
        || !range.writable
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_range_stale_or_invalid(),
        ));
    }
    Ok(())
}
fn find_structure<'a>(
    map: &'a EditorMap,
    range: &SourceRange,
    kind: &str,
) -> Result<&'a StructureInfo, EngineError> {
    map.structures
        .iter()
        .find(|s| s.kind == kind && same_range(&s.range, range))
        .ok_or_else(|| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_editor_construct_uncertified(),
            )
        })
}
fn push_patch(out: &mut Vec<SourcePatch>, source: &str, range: Range<usize>, insert: String) {
    out.push(SourcePatch {
        from: range.start,
        to: range.end,
        utf16_from: utf16(source, range.start),
        utf16_to: utf16(source, range.end),
        expected: source[range].into(),
        insert,
    });
}
pub(crate) fn finish(
    source: &str,
    mut patches: Vec<SourcePatch>,
) -> Result<EditOutput, EngineError> {
    patches.sort_by_key(|p| (p.from, p.to));
    if patches
        .windows(2)
        .any(|p| p[0].to > p[1].from || p[0].from == p[1].from)
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_patches_overlap(),
        ));
    }
    let mut result = source.to_owned();
    for patch in patches.iter().rev() {
        result.replace_range(patch.from..patch.to, &patch.insert);
    }
    let parsed = Source::detached(&result);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_native_reparse_failed(),
        ));
    }
    let end = patches
        .last()
        .map(|p| {
            let shift: isize = patches
                .iter()
                .take(patches.len() - 1)
                .map(|p| p.insert.len() as isize - (p.to - p.from) as isize)
                .sum();
            (p.from as isize + shift) as usize + p.insert.len()
        })
        .unwrap_or(source.len());
    let map = inspect_editor(&result);
    let selection = EditSelection {
        from: utf16(&result, end),
        to: utf16(&result, end),
    };
    Ok(EditOutput {
        source_hash: map.source_hash.clone(),
        source: result,
        patches,
        selection,
        map,
    })
}

fn check_level(level: usize) -> Result<(), EngineError> {
    if level == 0 || level > 1024 {
        Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_heading_level_bounds(),
        ))
    } else {
        Ok(())
    }
}
fn content(text: &str) -> Result<String, EngineError> {
    let encoded = encode_literal(text)?;
    Ok(if encoded.is_empty() {
        "#box[]".into()
    } else {
        encoded
    })
}
fn encode_math(text: &str, display: bool) -> Result<String, EngineError> {
    if text.len() > 1_048_576 || text.contains(['\0', '$', '#']) {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_math_delimiters_unsupported(),
        ));
    }
    let raw = if display {
        format!("$ {text} $")
    } else {
        format!("${text}$")
    };
    let parsed = Source::detached(&raw);
    let nodes: Vec<_> = parsed
        .root()
        .children()
        .filter(|n| !n.kind().is_trivia())
        .collect();
    if !parsed.root().errors_and_warnings().0.is_empty()
        || nodes.len() != 1
        || nodes[0].cast::<ast::Equation>().is_none()
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_math_invalid(),
        ));
    }
    Ok(raw)
}
fn encode_block(block: &BlockInsert) -> Result<String, EngineError> {
    Ok(match block {
        BlockInsert::Image { path, alt } => {
            if path.is_empty() || path.len() > 4096 || path.contains(['\0', '\r', '\n']) {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_image_path_invalid(),
                ));
            }
            format!(
                "#image({}, alt: {})",
                encode_string(path)?,
                encode_string(alt)?
            )
        }
        BlockInsert::Paragraph { text } => encode_literal(text)?,
        BlockInsert::Heading { text, level } => {
            check_level(*level)?;
            format!("{} {}", "=".repeat(*level), content(text)?)
        }
        BlockInsert::BulletList { items } | BlockInsert::NumberedList { items } => {
            if items.is_empty() || items.len() > 10000 {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_list_count_bounds(),
                ));
            }
            let marker = if matches!(block, BlockInsert::BulletList { .. }) {
                "-"
            } else {
                "+"
            };
            items
                .iter()
                .map(|i| content(i).map(|c| format!("{marker} {c}")))
                .collect::<Result<Vec<_>, _>>()?
                .join("\n")
        }
        BlockInsert::TermList { items } => items
            .iter()
            .map(|i| {
                Ok(format!(
                    "/ {}: {}",
                    content(&i.term)?,
                    content(&i.description)?
                ))
            })
            .collect::<Result<Vec<_>, EngineError>>()?
            .join("\n"),
        BlockInsert::Math { text, display } => encode_math(text, *display)?,
        BlockInsert::Table { columns, cells } => {
            check_grid(*columns, cells.len())?;
            format!(
                "#table(columns: {columns}, {})",
                cells
                    .iter()
                    .map(|c| encode_cell_literal(c).map(|c| format!("[{c}]")))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )
        }
        BlockInsert::ClipboardTable {
            rows,
            columns,
            cells,
        } => {
            format!(
                "#{}",
                table_editor::encode_clipboard(*rows, *columns, cells)?
            )
        }
        BlockInsert::Link { target, text } => {
            if !crate::render::safe_url(target) {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_link_scheme_unsupported(),
                ));
            }
            format!(
                "#link({})[{}]",
                encode_string(target)?,
                encode_literal(text)?
            )
        }
        BlockInsert::Raw {
            text,
            language,
            block,
        } => {
            if text.contains('\0') || text.len() > 1_048_576 {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_raw_budget(),
                ));
            }
            let language = language.as_deref().unwrap_or("");
            if !language
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
            {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_code_language_invalid(),
                ));
            }
            let ticks = "`".repeat(
                text.split(|c| c != '`')
                    .map(str::len)
                    .max()
                    .unwrap_or(0)
                    .max(if *block { 2 } else { 0 })
                    + 1,
            );
            if *block {
                format!("{ticks}{language}\n{text}\n{ticks}")
            } else {
                format!("{ticks}{text}{ticks}")
            }
        }
    })
}

fn edit_title(
    source: &str,
    map: &EditorMap,
    text: Option<&str>,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    match (map.title.status.as_str(), text) {
        ("writable", Some(text)) => {
            let range = map.title.range.as_ref().unwrap();
            push_patch(
                patches,
                source,
                range.from..range.to,
                if map.title.encoding.as_deref() == Some("string") {
                    encode_string(text)?
                } else {
                    encode_literal(text)?
                },
            );
        }
        ("writable", None) => {
            let range = map.title.construct.as_ref().unwrap();
            let deletion = if range.raw.starts_with("#set document") {
                title_set_deletion(source, range)?
            } else {
                range.from..range.to
            };
            push_patch(patches, source, deletion, String::new());
            // Removing document.title also removes its now-invalid title() presentation.
            if range.raw.starts_with("#set document") {
                let parsed = Source::detached(source);
                for n in LinkedNode::new(parsed.root()).children() {
                    if let Some(c) = n.get().cast::<ast::FuncCall>()
                        && c.callee()
                            .to_untyped()
                            .cast::<ast::Ident>()
                            .is_some_and(|i| i.get() == "title")
                    {
                        push_patch(patches, source, including_hash(&n, source), String::new());
                    }
                }
            }
            if let Some(sub) = &map.subtitle.construct {
                push_patch(patches, source, sub.from..sub.to, String::new());
            }
        }
        ("absent", Some(text)) => push_patch(
            patches,
            source,
            0..0,
            format!(
                "#set document(title: [{}])\n{}",
                encode_literal(text)?,
                if map.title.construct.is_some() {
                    ""
                } else {
                    "#title()\n\n"
                }
            ),
        ),
        ("absent", None) => {}
        _ => {
            return Err(EngineError::new(
                UiMessage::error_engine_editor_title_not_lossless(
                    weftext_messages::params::ErrorEngineEditorTitleNotLossless {
                        status: map.title.status.clone(),
                    },
                ),
            ));
        }
    }
    Ok(())
}
fn edit_subtitle(
    source: &str,
    map: &EditorMap,
    text: Option<&str>,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    if map.title.status != "writable" {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_subtitle_title_required(),
        ));
    }
    match (map.subtitle.status.as_str(), text) {
        ("writable", Some(text)) => {
            let r = map.subtitle.range.as_ref().unwrap();
            push_patch(patches, source, r.from..r.to, encode_literal(text)?);
        }
        ("writable", None) => {
            let r = map.subtitle.construct.as_ref().unwrap();
            push_patch(patches, source, r.from..r.to, String::new());
        }
        ("absent", Some(text)) => {
            let parsed = Source::detached(source);
            let at = LinkedNode::new(parsed.root())
                .children()
                .find(|node| {
                    node.get().cast::<ast::FuncCall>().is_some_and(|call| {
                        call.callee()
                            .to_untyped()
                            .cast::<ast::Ident>()
                            .is_some_and(|i| i.get() == "title")
                    })
                })
                .map_or(map.title.construct.as_ref().unwrap().to, |node| {
                    node.range().end
                });
            push_patch(
                patches,
                source,
                at..at,
                format!("\n#block[{}] <weftext-subtitle>\n", encode_literal(text)?),
            );
        }
        ("absent", None) => {}
        _ => {
            return Err(EngineError::new(
                UiMessage::error_engine_editor_subtitle_not_lossless(
                    weftext_messages::params::ErrorEngineEditorSubtitleNotLossless {
                        status: map.subtitle.status.clone(),
                    },
                ),
            ));
        }
    }
    Ok(())
}

fn title_set_deletion(source: &str, construct: &SourceRange) -> Result<Range<usize>, EngineError> {
    let parsed = Source::detached(source);
    let root = LinkedNode::new(parsed.root());
    let node = root
        .children()
        .find(|n| n.range().start == construct.from + 1 && n.get().cast::<ast::SetRule>().is_some())
        .ok_or_else(|| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_editor_title_setter_unavailable(),
            )
        })?;
    let set = node.get().cast::<ast::SetRule>().unwrap();
    if set.args().items().count() == 1 {
        return Ok(construct.from..construct.to);
    }
    let args = parsed.find(set.args().to_untyped().span()).unwrap();
    let children: Vec<_> = args.children().collect();
    if children
        .iter()
        .any(|n| matches!(n.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment))
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_title_comment_ambiguous(),
        ));
    }
    let index = children
        .iter()
        .position(|n| {
            n.get()
                .cast::<ast::Named>()
                .is_some_and(|n| n.name().get() == "title")
        })
        .ok_or_else(|| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_editor_title_argument_unavailable(),
            )
        })?;
    if let Some(comma) = children
        .iter()
        .skip(index + 1)
        .find(|n| n.kind() == SyntaxKind::Comma)
    {
        Ok(children[index].range().start..comma.range().end)
    } else if let Some(comma) = children[..index]
        .iter()
        .rev()
        .find(|n| n.kind() == SyntaxKind::Comma)
    {
        Ok(comma.range().start..children[index].range().end)
    } else {
        Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_title_separator_ambiguous(),
        ))
    }
}

fn edit_run_in(
    source: &str,
    map: &EditorMap,
    range: &SourceRange,
    enabled: bool,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    let heading = find_structure(map, range, "heading")?;
    let parsed = Source::detached(source);
    let root = LinkedNode::new(parsed.root());
    let siblings: Vec<_> = root.children().collect();
    let index = siblings
        .iter()
        .position(|n| n.range() == (range.from..range.to))
        .ok_or_else(|| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_editor_run_in_root_scope_required(),
            )
        })?;
    for builtin in ["heading", "counter", "strong", "text"] {
        if !standard_binding_available(source, builtin) {
            return Err(EngineError::new(
                UiMessage::error_engine_editor_run_in_binding_unverified(
                    weftext_messages::params::ErrorEngineEditorRunInBindingUnverified {
                        binding: builtin.into(),
                    },
                ),
            ));
        }
    }
    fn controlled_context(node: &SyntaxNode) -> bool {
        if let Some(set) = node.cast::<ast::SetRule>()
            && set
                .target()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "heading")
            && set.args().items().any(|a|matches!(a,ast::Arg::Named(n) if n.name().get()=="numbering"&&!matches!(n.expr().to_untyped().kind(),SyntaxKind::Str|SyntaxKind::None)))
        {
            return false;
        }
        if let Some(call) = node.cast::<ast::FuncCall>() {
            if call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "label")
            {
                return false;
            }
            if let Some(field) = call.callee().to_untyped().cast::<ast::FieldAccess>()
                && matches!(field.field().get().as_str(), "update" | "step")
                && field
                    .target()
                    .to_untyped()
                    .cast::<ast::FuncCall>()
                    .is_some_and(|c| {
                        c.callee()
                            .to_untyped()
                            .cast::<ast::Ident>()
                            .is_some_and(|i| i.get() == "counter")
                    })
            {
                return false;
            }
        }
        node.children().all(controlled_context)
    }
    if !controlled_context(root.get()) {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_run_in_scope_uncertified(),
        ));
    }
    let following_at = {
        siblings
            .iter()
            .skip(index + 1)
            .find(|n| !n.kind().is_trivia())
            .filter(|n| n.get().cast::<ast::Label>().is_some())
            .map_or(range.to, |n| n.range().end)
    };
    let following = &source[following_at..];
    let trimmed = following.trim_start_matches(char::is_whitespace);
    let first_body = siblings
        .iter()
        .skip(index + 1)
        .filter(|n| n.range().start >= following_at)
        .find(|n| !n.kind().is_trivia() && n.kind() != SyntaxKind::Hash);
    let body_qualified = first_body.is_some_and(|n| {
        matches!(
            n.kind(),
            SyntaxKind::Text
                | SyntaxKind::Escape
                | SyntaxKind::Shorthand
                | SyntaxKind::Strong
                | SyntaxKind::Emph
                | SyntaxKind::Link
                | SyntaxKind::Ref
        ) || n.get().cast::<ast::Equation>().is_some_and(|e| !e.block())
            || n.get().cast::<ast::FuncCall>().is_some_and(|c| {
                c.callee()
                    .to_untyped()
                    .cast::<ast::Ident>()
                    .is_some_and(|i| {
                        matches!(
                            i.get().as_str(),
                            "link" | "text" | "strong" | "emph" | "strike"
                        )
                    })
            })
    });
    if enabled && !body_qualified {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_run_in_paragraph_required(),
        ));
    }
    let mut used = std::collections::HashMap::<String, usize>::new();
    fn labels(n: &LinkedNode<'_>, set: &mut std::collections::HashMap<String, usize>) {
        if let Some(l) = n.get().cast::<ast::Label>() {
            *set.entry(l.get().into()).or_default() += 1;
        }
        for c in n.children() {
            labels(&c, set)
        }
    }
    labels(&root, &mut used);
    let label = if let Some(l) = &heading.label {
        if used.get(l).copied().unwrap_or(0) > if heading.run_in { 2 } else { 1 } {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_editor_heading_label_ambiguous(),
            ));
        }
        l.clone()
    } else {
        let mut index = 1;
        loop {
            let l = format!("weftext-heading-{index}");
            if !used.contains_key(&l) {
                break l;
            }
            index += 1;
        }
    };
    if enabled {
        if heading.run_in {
            return Ok(());
        }
        push_patch(
            patches,
            source,
            range.from..range.from,
            format!("#{}\n", run_in_recipe(&label)),
        );
        let whitespace = following.len() - trimmed.len();
        let suffix = if heading.label.is_none() {
            format!(" <{label}>\n")
        } else {
            "\n".into()
        };
        push_patch(
            patches,
            source,
            following_at..following_at + whitespace,
            suffix,
        );
    } else if heading.run_in {
        for n in root.children() {
            if n.kind() == SyntaxKind::ShowRule && source[n.range()] == run_in_recipe(&label) {
                let r = including_hash(&n, source);
                push_patch(patches, source, r, String::new());
            }
        }
        let whitespace = following.len() - trimmed.len();
        push_patch(
            patches,
            source,
            following_at..following_at + whitespace,
            "\n\n".into(),
        );
    }
    Ok(())
}

fn check_grid(columns: usize, cells: usize) -> Result<(), EngineError> {
    if columns == 0
        || columns > 1000
        || cells == 0
        || cells > 100000
        || !cells.is_multiple_of(columns)
    {
        Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_table_rectangle_required(),
        ))
    } else {
        Ok(())
    }
}
fn edit_table(
    source: &str,
    map: &EditorMap,
    range: &SourceRange,
    action: &TableAction,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    let structure = find_structure(map, range, "table")?;
    table_editor::edit(source, structure, action, patches)
}
fn edit_list(
    source: &str,
    map: &EditorMap,
    range: &SourceRange,
    action: &ListAction,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    let structure = map
        .structures
        .iter()
        .find(|s| {
            same_range(&s.range, range)
                && matches!(s.kind.as_str(), "bulletListItem" | "numberedListItem")
        })
        .ok_or_else(|| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_editor_list_item_unavailable(),
            )
        })?;
    let line_start = source[..range.from].rfind('\n').map_or(0, |n| n + 1);
    let indentation = &source[line_start..range.from];
    if !indentation.chars().all(|c| c == ' ') {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_list_indent_ambiguous(),
        ));
    }
    match action {
        ListAction::Indent => {
            if range.raw.contains(['\n', '\r']) || !preceding_list_sibling(source, range) {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_list_indent_predecessor_required(),
                ));
            }
            push_patch(patches, source, line_start..line_start, "  ".into());
        }
        ListAction::Outdent => {
            if indentation.len() < 2 || range.raw.contains(['\n', '\r']) {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_list_already_outermost(),
                ));
            }
            push_patch(patches, source, line_start..line_start + 2, String::new());
        }
        ListAction::Split { at } => {
            let body = structure.body.as_ref().unwrap();
            let decoded = decode_list_body(source, &body.raw).ok_or_else(|| {
                EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_formatted_list_split_unsupported(),
                )
            })?;
            let byte = utf16_byte(&decoded, *at).ok_or_else(|| {
                EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_list_split_boundary_invalid(),
                )
            })?;
            let marker = if structure.kind == "bulletListItem" {
                "-"
            } else {
                "+"
            };
            push_patch(
                patches,
                source,
                body.from..body.to,
                format!(
                    "{}\n{indentation}{marker} {}",
                    content(&decoded[..byte])?,
                    content(&decoded[byte..])?
                ),
            );
        }
    }
    Ok(())
}
fn preceding_list_sibling(source: &str, range: &SourceRange) -> bool {
    let parsed = Source::detached(source);
    let root = LinkedNode::new(parsed.root());
    fn find(node: &LinkedNode<'_>, range: &SourceRange) -> bool {
        let children: Vec<_> = node.children().collect();
        if let Some(index) = children
            .iter()
            .position(|n| n.range() == (range.from..range.to))
        {
            return children[..index]
                .iter()
                .rev()
                .find(|n| !n.kind().is_trivia())
                .is_some_and(|n| matches!(n.kind(), SyntaxKind::ListItem | SyntaxKind::EnumItem));
        }
        children.iter().any(|n| find(n, range))
    }
    find(&root, range)
}
// Empty list slots use exactly this ordinary native box construction. Its
// content may remain literal after typing; do not unwrap arbitrary box calls.
fn decode_list_body(source: &str, raw: &str) -> Option<String> {
    if let Some(text) = decode_literal(raw) {
        return Some(text);
    }
    if !standard_binding_available(source, "box") {
        return None;
    }
    let parsed = Source::detached(raw);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return None;
    }
    let children: Vec<_> = parsed
        .root()
        .children()
        .filter(|n| !n.kind().is_trivia())
        .collect();
    if children.len() != 2 || children[0].kind() != SyntaxKind::Hash {
        return None;
    }
    let call = children[1].cast::<ast::FuncCall>()?;
    if !call
        .callee()
        .to_untyped()
        .cast::<ast::Ident>()
        .is_some_and(|n| n.get() == "box")
    {
        return None;
    }
    let args: Vec<_> = call.args().items().collect();
    let [ast::Arg::Pos(ast::Expr::ContentBlock(content))] = args.as_slice() else {
        return None;
    };
    let markup = parsed.find(content.body().to_untyped().span())?;
    decode_literal(&raw[markup.range()])
}
fn utf16_byte(text: &str, at: usize) -> Option<usize> {
    let mut n = 0;
    for (i, c) in text.char_indices() {
        if n == at {
            return Some(i);
        }
        n += c.len_utf16();
    }
    (n == at).then_some(text.len())
}

/// Split the actual inline AST at the selected source endpoints. Wrapper bytes
/// may change inside the certified patch; all unselected leaf/trivia bytes stay.
fn inline_edit(
    source: &str,
    map: &EditorMap,
    range: &SourceRange,
    format: InlineFormat,
    patches: &mut Vec<SourcePatch>,
) -> Result<usize, EngineError> {
    let selected_kind = match format {
        InlineFormat::Strong => "strong",
        InlineFormat::Emph => "emph",
        InlineFormat::Strike => "strike",
    };
    if !standard_binding_available(source, selected_kind) {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_format_binding_unverified(),
        ));
    }
    if range.from == range.to
        || !map
            .literal_regions
            .iter()
            .any(|r| range.from >= r.range.from && range.from < r.range.to)
        || !map
            .literal_regions
            .iter()
            .any(|r| range.to > r.range.from && range.to <= r.range.to)
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_format_endpoints_unavailable(),
        ));
    }
    let parsed = Source::detached(source);
    let root = LinkedNode::new(parsed.root());
    fn containing<'a>(node: &LinkedNode<'a>, range: &SourceRange) -> Option<LinkedNode<'a>> {
        if node.range().start > range.from || node.range().end < range.to {
            return None;
        }
        node.children()
            .find_map(|c| containing(&c, range))
            .or_else(|| (node.kind() == SyntaxKind::Markup).then(|| node.clone()))
    }
    let markup = containing(&root, range).ok_or_else(|| {
        EngineError::new(weftext_messages::UiMessage::error_engine_editor_format_crosses_slots())
    })?;
    let children: Vec<_> = markup.children().collect();
    let first = children
        .iter()
        .position(|n| n.range().end > range.from)
        .ok_or_else(|| {
            EngineError::new(
                weftext_messages::UiMessage::error_engine_editor_format_empty_selection(),
            )
        })?;
    let last = children
        .iter()
        .rposition(|n| n.range().start < range.to)
        .unwrap();
    let first = if first > 0 && children[first - 1].kind() == SyntaxKind::Hash {
        first - 1
    } else {
        first
    };
    let touched = children[first].range().start..children[last].range().end;
    #[derive(Default)]
    struct Parts {
        left: String,
        middle: String,
        right: String,
    }
    fn split(
        node: &LinkedNode<'_>,
        source: &str,
        parsed: &Source,
        range: &SourceRange,
    ) -> Result<Parts, EngineError> {
        let address = node.range();
        let mut parts = Parts::default();
        if node.kind() == SyntaxKind::Hash {
            return Ok(parts);
        }
        if address.end <= range.from {
            parts.left = source[address].into();
            return Ok(parts);
        }
        if address.start >= range.to {
            parts.right = source[address].into();
            return Ok(parts);
        }
        if literal_piece(node, source).is_some() {
            if matches!(node.kind(), SyntaxKind::Escape | SyntaxKind::Shorthand)
                && (range.from > address.start && range.from < address.end
                    || range.to > address.start && range.to < address.end)
            {
                return Err(EngineError::new(
                    weftext_messages::UiMessage::error_engine_editor_format_splits_native_escape(),
                ));
            }
            let start = range.from.clamp(address.start, address.end);
            let end = range.to.clamp(address.start, address.end);
            parts.left = source[address.start..start].into();
            parts.middle = source[start..end].into();
            parts.right = source[end..address.end].into();
            return Ok(parts);
        }
        let (kind, body) = if let Some(s) = node.get().cast::<ast::Strong>() {
            ("strong", Some(s.body().to_untyped()))
        } else if let Some(s) = node.get().cast::<ast::Emph>() {
            ("emph", Some(s.body().to_untyped()))
        } else if let Some(c) = node.get().cast::<ast::FuncCall>() {
            let kind = c
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .map(|i| i.get().as_str())
                .unwrap_or("");
            let args: Vec<_> = c.args().items().collect();
            let body = if matches!(kind, "strong" | "emph" | "strike") {
                match args.as_slice() {
                    [ast::Arg::Pos(e)] => e
                        .to_untyped()
                        .cast::<ast::ContentBlock>()
                        .map(|b| b.body().to_untyped()),
                    _ => None,
                }
            } else {
                None
            };
            (kind, body)
        } else {
            ("", None)
        };
        if let Some(body) = body
            && let Some(body) = parsed.find(body.span())
        {
            for child in body.children() {
                let child = split(&child, source, parsed, range)?;
                parts.left.push_str(&child.left);
                parts.middle.push_str(&child.middle);
                parts.right.push_str(&child.right);
            }
            let wrap = |text: String| {
                if text.is_empty() {
                    text
                } else {
                    format!("#{kind}[{text}]")
                }
            };
            parts.left = wrap(parts.left);
            parts.middle = wrap(parts.middle);
            parts.right = wrap(parts.right);
            return Ok(parts);
        }
        Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_editor_format_crosses_uncertified_content(),
        ))
    }
    let mut parts = Parts::default();
    for child in &children[first..=last] {
        let part = split(child, source, &parsed, range)?;
        parts.left.push_str(&part.left);
        parts.middle.push_str(&part.middle);
        parts.right.push_str(&part.right);
    }
    let kind = match format {
        InlineFormat::Strong => "strong",
        InlineFormat::Emph => "emph",
        InlineFormat::Strike => "strike",
    };
    let cursor = touched.start + parts.left.len() + kind.len() + 2 + parts.middle.len();
    push_patch(
        patches,
        source,
        touched,
        format!("{}#{kind}[{}]{}", parts.left, parts.middle, parts.right),
    );
    Ok(cursor)
}
