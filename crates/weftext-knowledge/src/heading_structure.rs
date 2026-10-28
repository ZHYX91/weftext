//! Real compiled outline products; text resemblance never certifies a heading
//! or caption. Source ranges are read-navigation addresses, never edit grants.
use crate::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureHeading {
    pub key: String,
    pub text: String,
    pub number: Option<String>,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_kind: Option<String>,
    /// Positive only for headings. Captions do not establish outline ancestry.
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
            && node.attrs.get("data-weftext-role").is_some_and(|r| {
                matches!(
                    r.as_str(),
                    "heading-prefix" | "caption-prefix" | "equation-prefix"
                )
            })
        {
            return String::new();
        }
        if node.kind == "text" {
            return node.text.clone().unwrap_or_default();
        }
        node.children.iter().map(|n| text(n, prefix)).collect()
    }
    fn role(node: &weftext_engine::RenderNode) -> Option<&str> {
        node.attrs.get("data-weftext-role").map(String::as_str)
    }
    fn child_role<'a>(
        node: &'a weftext_engine::RenderNode,
        wanted: &str,
    ) -> Option<&'a weftext_engine::RenderNode> {
        if role(node) == Some(wanted) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| child_role(child, wanted))
    }
    fn exact_navigation_range(
        node: &weftext_engine::RenderNode,
        source: &str,
    ) -> Option<ExactRange> {
        let origin = node.origin.as_ref().filter(|origin| {
            origin.is_main
                && matches!(
                    origin.syntax_kind.as_str(),
                    "FuncCall" | "ContentBlock" | "Math" | "Equation" | "Markup"
                )
                && source.get(origin.from..origin.to) == Some(origin.raw.as_str())
        })?;
        ExactRange::new(source, origin.from, origin.to).ok()
    }
    fn visit<'a>(
        node: &'a weftext_engine::RenderNode,
        source: &str,
        budget: usize,
        result: &mut HeadingStructure,
        owning_figure: Option<&'a weftext_engine::RenderNode>,
    ) {
        let native_role = (node.kind != "tag").then(|| role(node)).flatten();
        let numbered_equation = native_role == Some("equation")
            && node
                .attrs
                .get("data-weftext-numbered")
                .is_some_and(|value| value == "true");
        if native_role == Some("heading") || native_role == Some("caption") || numbered_equation {
            if result.items.len() == budget {
                result.truncated = true;
                return;
            }
            if native_role == Some("heading")
                && let Some(level) = node
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
                    kind: "heading",
                    native_kind: None,
                    level,
                    generated: range.is_none(),
                    range,
                });
            } else if native_role == Some("caption") || numbered_equation {
                let native_kind = if numbered_equation {
                    Some("equation".to_string())
                } else {
                    node.attrs.get("data-weftext-caption-kind").cloned()
                };
                let kind = match native_kind.as_deref() {
                    Some("figure") => "figure",
                    Some("table") => "table",
                    Some("equation") => "equation",
                    Some("native:raw") => "code",
                    _ => "caption",
                };
                let body = if numbered_equation {
                    node
                } else {
                    child_role(node, "caption-body").unwrap_or(node)
                };
                let number = child_role(
                    node,
                    if numbered_equation {
                        "equation-number"
                    } else {
                        "caption-number"
                    },
                )
                .map(|number| text(number, true).trim().to_string())
                .filter(|number| !number.is_empty());
                let range = exact_navigation_range(body, source)
                    .or_else(|| exact_navigation_range(node, source))
                    .or_else(|| {
                        owning_figure.and_then(|figure| exact_navigation_range(figure, source))
                    });
                result.items.push(StructureHeading {
                    key: node.key.clone(),
                    text: text(body, false).trim().into(),
                    number,
                    kind,
                    native_kind,
                    level: 0,
                    generated: range.is_none(),
                    range,
                });
            }
        }
        for child in &node.children {
            visit(
                child,
                source,
                budget,
                result,
                if native_role == Some("figure") {
                    Some(node)
                } else {
                    owning_figure
                },
            );
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
        visit(tree, source, budget, &mut result, None);
        let mut occurrences = std::collections::BTreeMap::new();
        for item in &result.items {
            if let Some(range) = &item.range {
                *occurrences.entry((range.from, range.to)).or_insert(0usize) += 1;
            }
        }
        // A repeated producer may point to a macro call rather than one unique
        // displayed source occurrence. Keep display navigation, without claiming
        // an exact source address for that occurrence.
        for item in &mut result.items {
            if item.range.as_ref().is_some_and(|range| {
                occurrences
                    .get(&(range.from, range.to))
                    .is_some_and(|count| *count > 1)
            }) {
                item.range = None;
                item.generated = true;
            }
        }
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
        assert_eq!(structure.items.len(), 4, "{:?}", structure.items);
        assert_eq!(
            structure.items.iter().map(|h| h.level).collect::<Vec<_>>(),
            vec![1, 4, 7, 0]
        );
        assert_eq!(structure.items[3].kind, "figure");
        assert_eq!(
            structure
                .items
                .iter()
                .filter(|item| item.kind == "heading")
                .count(),
            3,
            "native figure captions must not be promoted to headings"
        );
        assert_eq!(structure.items[3].level, 0);
        assert_eq!(structure.items[3].text, "题注");
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
            4
        );
        let limited = heading_structure(output.tree.as_ref(), source, 1);
        assert!(limited.truncated);
        assert_eq!(limited.items.len(), 1);
        assert_eq!(
            heading_structure(None, source, 100).coverage,
            Coverage::Partial
        );
    }
    #[test]
    fn native_captions_and_numbered_equations_are_distinct_outline_leaves() {
        let source = "= Root\n#figure(rect(width: 10pt, height: 10pt), kind: image, caption: [Picture])\n#figure(table(columns: 1, [Cell]), caption: [Data table])\n#figure(raw(\"print(1)\", block: true), caption: [Code listing])\n#set math.equation(numbering: \"(1)\")\n$ x = 1 $\n$x$";
        let root = std::env::current_dir().unwrap();
        let output = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &root.join("captions.typ"), &root)
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        let structure = heading_structure(output.tree.as_ref(), source, 100);
        assert_eq!(
            structure
                .items
                .iter()
                .map(|item| item.kind)
                .collect::<Vec<_>>(),
            vec!["heading", "figure", "table", "code", "equation"]
        );
        assert!(structure.items.iter().skip(1).all(|item| item.level == 0));
        assert_eq!(structure.items[1].text, "Picture");
        assert_eq!(structure.items[2].text, "Data table");
        assert_eq!(structure.items[3].text, "Code listing");
        assert!(structure.items.iter().skip(1).all(|item| {
            item.number
                .as_ref()
                .is_some_and(|number| !number.is_empty())
        }));
        assert!(
            structure
                .items
                .iter()
                .all(|item| item.range.as_ref().is_none_or(|range| range.verify(source)))
        );
    }
    #[test]
    fn generated_repeated_and_stale_caption_origins_do_not_claim_exact_addresses() {
        use std::collections::BTreeMap;
        let source = "figure()[Caption]";
        let caption = |key: &str, raw: &str| weftext_engine::RenderNode {
            key: key.into(),
            kind: "element".into(),
            tag: Some("figcaption".into()),
            text: None,
            attrs: BTreeMap::from([
                ("data-weftext-role".into(), "caption".into()),
                ("data-weftext-caption-kind".into(), "custom:Listing".into()),
            ]),
            children: vec![weftext_engine::RenderNode {
                key: format!("{key}-text"),
                kind: "text".into(),
                tag: None,
                text: Some("Caption".into()),
                attrs: BTreeMap::new(),
                children: vec![],
                origin: None,
                writable: false,
            }],
            origin: Some(weftext_engine::Origin {
                path: "captions.typ".into(),
                from: 0,
                to: source.len(),
                utf16_from: 0,
                utf16_to: source.len(),
                raw: raw.into(),
                syntax_kind: "FuncCall".into(),
                is_main: true,
            }),
            writable: false,
        };
        let mut tree = caption("one", source);
        tree.children = vec![caption("two", source), caption("stale", "other")];
        let structure = heading_structure(Some(&tree), source, 100);
        assert_eq!(structure.items.len(), 3);
        assert!(
            structure
                .items
                .iter()
                .all(|item| item.range.is_none() && item.generated)
        );
        assert!(
            structure.items.iter().all(|item| item.kind == "caption"
                && item.native_kind.as_deref() == Some("custom:Listing"))
        );
        tree.kind = "tag".into();
        tree.children.clear();
        assert!(heading_structure(Some(&tree), source, 100).items.is_empty());
    }
}
