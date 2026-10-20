//! Ordinary root siblings only: no evaluation, guessed display text or reformat.
use super::*;

pub(super) fn blocks(source: &str) -> Vec<BodyBlock> {
    let parsed = Source::detached(source);
    let root = LinkedNode::new(parsed.root());
    let mut result = vec![];
    let mut run = vec![];
    fn ordinary(node: &SyntaxNode) -> bool {
        matches!(
            node.kind(),
            SyntaxKind::Text
                | SyntaxKind::Space
                | SyntaxKind::Escape
                | SyntaxKind::Shorthand
                | SyntaxKind::SmartQuote
                | SyntaxKind::Linebreak
        ) || matches!(
            node.kind(),
            SyntaxKind::Strong
                | SyntaxKind::Emph
                | SyntaxKind::Markup
                | SyntaxKind::Star
                | SyntaxKind::Underscore
        ) && node.children().all(ordinary)
    }
    fn flush(run: &mut Vec<LinkedNode<'_>>, source: &str, out: &mut Vec<BodyBlock>) {
        if !run.is_empty() && run.iter().all(|n| ordinary(n.get())) {
            let start = run[0].range().start;
            let end = run.last().unwrap().range().end;
            let raw = &source[start..end];
            let from = start + raw.len() - raw.trim_start().len();
            let to = end - (raw.len() - raw.trim_end().len());
            if from < to {
                let range = source_range(source, from..to);
                out.push(BodyBlock {
                    kind: "paragraph".into(),
                    body: range.clone(),
                    range,
                });
            }
        }
        run.clear();
    }
    let children: Vec<_> = root.children().collect();
    for (index, child) in children.iter().enumerate() {
        let structure = if let Some(h) = child.get().cast::<ast::Heading>() {
            Some(("heading", h.body().to_untyped()))
        } else if let Some(item) = child.get().cast::<ast::ListItem>() {
            Some(("bulletList", item.body().to_untyped()))
        } else {
            child
                .get()
                .cast::<ast::EnumItem>()
                .map(|item| ("numberedList", item.body().to_untyped()))
        };
        if let Some((kind, body)) = structure {
            flush(&mut run, source, &mut result);
            let labelled = children
                .iter()
                .skip(index + 1)
                .find(|n| !n.kind().is_trivia())
                .is_some_and(|n| n.kind() == SyntaxKind::Label);
            if !labelled
                && ordinary(body)
                && let Some(body) = parsed.find(body.span())
            {
                result.push(BodyBlock {
                    kind: kind.into(),
                    range: source_range(source, child.range()),
                    body: source_range(source, body.range()),
                });
            }
        } else if child.kind() == SyntaxKind::Parbreak {
            flush(&mut run, source, &mut result);
        } else {
            run.push(child.clone());
        }
    }
    flush(&mut run, source, &mut result);
    result
}

pub(super) fn convert(
    source: &str,
    map: &EditorMap,
    range: &SourceRange,
    target: &str,
    level: Option<usize>,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    let block = map
        .body_blocks
        .iter()
        .find(|b| same_range(&b.range, range))
        .ok_or_else(|| {
            EngineError("没有当前唯一普通根块；生成、标签和复杂内容请保留源码。".into())
        })?;
    // Reusing a multiline body as a single list/heading would change semantics.
    if target != "paragraph" && block.body.raw.contains(['\r', '\n']) {
        return Err(EngineError("多行块不能无损转换为一个标题或列表项。".into()));
    }
    let prefix = match target {
        "paragraph" => String::new(),
        "heading" => {
            if !standard_binding_available(source, "heading")
                || !heading_levels_unmodified(Source::detached(source).root())
            {
                return Err(EngineError(
                    "标题绑定或实际级别规则不能证明，原稿保留。".into(),
                ));
            }
            let level = level.unwrap_or(1);
            check_level(level)?;
            format!("{} ", "=".repeat(level))
        }
        "bulletList" => {
            if !standard_binding_available(source, "list") {
                return Err(EngineError("列表绑定不能证明。".into()));
            }
            "- ".into()
        }
        "numberedList" => {
            if !standard_binding_available(source, "enum") {
                return Err(EngineError("列表绑定不能证明。".into()));
            }
            "+ ".into()
        }
        _ => return Err(EngineError("不支持的普通块转换。".into())),
    };
    // Removing markup must not turn author text into a different native node.
    let insert = format!("{prefix}{}", block.body.raw);
    let parsed = Source::detached(&insert);
    if !parsed.root().errors_and_warnings().0.is_empty()
        || blocks(&insert).len() != 1
        || blocks(&insert)[0].kind != target
    {
        return Err(EngineError("转换会改变原生结构，原稿保留。".into()));
    }
    push_patch(patches, source, range.from..range.to, insert);
    Ok(())
}

pub(super) fn move_block(
    source: &str,
    map: &EditorMap,
    range: &SourceRange,
    direction: &str,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    let index = map
        .body_blocks
        .iter()
        .position(|b| same_range(&b.range, range))
        .ok_or_else(|| EngineError("这不是当前唯一普通根块。".into()))?;
    let other = match direction {
        "up" => index.checked_sub(1),
        "down" => (index + 1 < map.body_blocks.len()).then_some(index + 1),
        _ => return Err(EngineError("未知移动方向。".into())),
    }
    .ok_or_else(|| EngineError("当前方向没有相邻可移动块。".into()))?;
    let a = &map.body_blocks[index.min(other)].range;
    let b = &map.body_blocks[index.max(other)].range;
    let gap = &source[a.to..b.from];
    if !gap.chars().all(char::is_whitespace) || gap.matches('\n').count() < 2 {
        return Err(EngineError(
            "只能移动以空段分隔的相邻普通根块；规则、注释及嵌套结构保源。".into(),
        ));
    }
    push_patch(
        patches,
        source,
        a.from..b.to,
        format!("{}{gap}{}", b.raw, a.raw),
    );
    Ok(())
}
