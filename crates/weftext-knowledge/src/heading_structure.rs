//! Real compiled heading products; text resemblance never certifies a heading.
use crate::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureHeading {
    pub key: String,
    pub text: String,
    pub number: Option<String>,
    pub level: usize,
    pub range: Option<ExactRange>,
    pub generated: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadingStructure {
    pub coverage: Coverage,
    pub truncated: bool,
    pub items: Vec<StructureHeading>,
}
pub fn heading_structure(
    tree: Option<&weftext_engine::RenderNode>,
    source: &str,
    budget: usize,
) -> HeadingStructure {
    fn text(node: &weftext_engine::RenderNode, prefix: bool) -> String {
        if !prefix
            && node
                .attrs
                .get("data-weftext-role")
                .is_some_and(|r| r == "heading-prefix")
        {
            return String::new();
        }
        if node.kind == "text" {
            return node.text.clone().unwrap_or_default();
        }
        node.children.iter().map(|n| text(n, prefix)).collect()
    }
    fn visit(
        node: &weftext_engine::RenderNode,
        source: &str,
        budget: usize,
        result: &mut HeadingStructure,
    ) {
        if node.kind != "tag"
            && node
                .attrs
                .get("data-weftext-role")
                .is_some_and(|r| r == "heading")
        {
            if result.items.len() == budget {
                result.truncated = true;
                return;
            }
            if let Some(level) = node
                .attrs
                .get("data-weftext-level")
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|n| *n > 0)
            {
                let range = node
                    .origin
                    .as_ref()
                    .filter(|o| {
                        o.is_main
                            && o.syntax_kind == "Heading"
                            && source.get(o.from..o.to) == Some(o.raw.as_str())
                    })
                    .and_then(|o| ExactRange::new(source, o.from, o.to).ok());
                fn number(node: &weftext_engine::RenderNode) -> Option<String> {
                    if node
                        .attrs
                        .get("data-weftext-role")
                        .is_some_and(|r| r == "heading-prefix")
                    {
                        return Some(text(node, true).trim().to_owned()).filter(|s| !s.is_empty());
                    }
                    node.children.iter().find_map(number)
                }
                let number = number(node);
                result.items.push(StructureHeading {
                    key: node.key.clone(),
                    text: text(node, false).trim().into(),
                    number,
                    level,
                    generated: range.is_none(),
                    range,
                });
            }
        }
        for child in &node.children {
            visit(child, source, budget, result);
        }
    }
    let mut result = HeadingStructure {
        coverage: if tree.is_some() {
            Coverage::Complete
        } else {
            Coverage::Partial
        },
        truncated: false,
        items: vec![],
    };
    if let Some(tree) = tree {
        visit(tree, source, budget, &mut result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_compilation_preserves_deep_heading_identity_and_does_not_promote_caption() {
        let source = "#set heading(numbering: \"1.\")\n= 文档根\n==== 深层\n#heading(level: 7)[生成标题]\n普通段落\n#figure(rect(width: 10pt, height: 10pt), caption: [题注])";
        let root = std::env::current_dir().unwrap();
        let output = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &root.join("structure.typ"), &root)
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        let structure = heading_structure(output.tree.as_ref(), source, 100);
        assert_eq!(structure.coverage, Coverage::Complete);
        assert_eq!(structure.items.len(), 3, "{:?}", structure.items);
        assert_eq!(
            structure.items.iter().map(|h| h.level).collect::<Vec<_>>(),
            vec![1, 4, 7]
        );
        assert!(
            structure.items[0]
                .range
                .as_ref()
                .is_some_and(|r| r.verify(source))
        );
        assert!(structure.items[2].generated);
        assert!(structure.items[2].range.is_none());
        assert_eq!(
            structure
                .items
                .iter()
                .map(|h| &h.key)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3
        );
        let limited = heading_structure(output.tree.as_ref(), source, 1);
        assert!(limited.truncated);
        assert_eq!(limited.items.len(), 1);
        assert_eq!(
            heading_structure(None, source, 100).coverage,
            Coverage::Partial
        );
    }
}
