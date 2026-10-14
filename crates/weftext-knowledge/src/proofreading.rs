//! Local, deterministic checks of frozen visible prose. No IO or author writes.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use unicode_segmentation::UnicodeSegmentation;
use weftext_engine::RenderNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProofreadRule {
    pub id: String,
    pub version: String,
    pub category: String,
    pub text: String,
    pub message: String,
    pub replacement: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProofreadOptions {
    pub characters: bool,
    pub typos: bool,
    pub pairs: bool,
    pub sensitive: bool,
    #[serde(default)]
    pub rules: Vec<ProofreadRule>,
    #[serde(default)]
    pub allowlist: Vec<String>,
    /// UTF-8 source coordinates, checked against the actual frozen source.
    pub selection: Option<(usize, usize)>,
}
impl Default for ProofreadOptions {
    fn default() -> Self {
        Self {
            characters: true,
            typos: true,
            pairs: true,
            sensitive: true,
            rules: vec![],
            allowlist: vec![],
            selection: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofreadFinding {
    pub id: String,
    pub category: String,
    pub rule_id: String,
    pub rule_version: String,
    pub excerpt: String,
    pub message: String,
    pub replacement: Option<String>,
    pub source_range: Option<ExactRange>,
    pub executable: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofreadResult {
    pub coverage: Coverage,
    pub findings: Vec<ProofreadFinding>,
    pub warnings: Vec<String>,
    pub checked_characters: usize,
}
#[derive(Default)]
struct Prose {
    text: String,
    // One optional source byte range per Unicode scalar, never UTF-16 offsets.
    positions: Vec<Option<(usize, usize)>>,
}
fn prose(tree: &RenderNode, source: &str) -> Vec<Prose> {
    fn flush(current: &mut Prose, out: &mut Vec<Prose>) {
        if !current.text.is_empty() {
            out.push(std::mem::take(current));
        }
    }
    fn walk(n: &RenderNode, source: &str, current: &mut Prose, out: &mut Vec<Prose>) {
        let tag = n.tag.as_deref().unwrap_or("");
        let excluded = matches!(
            tag,
            "a" | "code" | "pre" | "math" | "script" | "style" | "head"
        ) || n.kind == "frame"
            || n.attrs.get("class").is_some_and(|v| {
                v.split_whitespace()
                    .any(|c| matches!(c, "typst-math" | "typst-raw"))
            });
        if excluded {
            flush(current, out);
            return;
        }
        let block = matches!(
            tag,
            "p" | "div"
                | "li"
                | "td"
                | "th"
                | "tr"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "br"
                | "hr"
                | "section"
                | "blockquote"
        );
        if block {
            flush(current, out);
        }
        if n.kind == "text"
            && let Some(t) = &n.text
        {
            let origin = n
                .origin
                .as_ref()
                .filter(|o| o.is_main && o.raw == *t && source.get(o.from..o.to) == Some(t));
            for (b, c) in t.char_indices() {
                current.text.push(c);
                current
                    .positions
                    .push(origin.map(|o| (o.from + b, o.from + b + c.len_utf8())));
            }
        }
        for child in &n.children {
            walk(child, source, current, out);
        }
        if block {
            flush(current, out);
        }
    }
    let mut out = vec![];
    let mut current = Prose::default();
    walk(tree, source, &mut current, &mut out);
    flush(&mut current, &mut out);
    out
}
fn validate(options: &ProofreadOptions, source: &str) -> Result<()> {
    if options.rules.len() > 2000 || options.allowlist.len() > 2000 {
        return Err(KnowledgeError::new(
            "proofreadBudget",
            "规则或白名单超过 2000 项",
        ));
    }
    let mut ids = BTreeSet::new();
    for r in &options.rules {
        if r.id.is_empty()
            || r.id.len() > 100
            || r.id.starts_with("builtin.")
            || !ids.insert(&r.id)
            || r.version.is_empty()
            || r.version.len() > 100
            || r.text.is_empty()
            || r.text.len() > 512
            || r.message.len() > 2048
            || r.replacement.as_ref().is_some_and(|v| v.len() > 2048)
            || !matches!(r.category.as_str(), "typo" | "sensitive" | "character")
            || r.category == "sensitive" && r.replacement.is_some()
        {
            return Err(KnowledgeError::new(
                "proofreadRule",
                "规则需要唯一标识、版本、非空词条与有效类别；敏感词规则不提供自动替换",
            ));
        }
    }
    if options
        .allowlist
        .iter()
        .any(|s| s.is_empty() || s.len() > 512)
    {
        return Err(KnowledgeError::new(
            "proofreadRule",
            "白名单词条须非空且不超过 512 字节",
        ));
    }
    if let Some((a, b)) = options.selection
        && (a >= b || source.get(a..b).is_none())
    {
        return Err(KnowledgeError::new(
            "proofreadRange",
            "请提供准确非空源选区",
        ));
    }
    Ok(())
}

pub fn proofread(
    tree: &RenderNode,
    source: &str,
    options: &ProofreadOptions,
) -> Result<ProofreadResult> {
    validate(options, source)?;
    let boundaries: BTreeSet<_> = source
        .grapheme_indices(true)
        .map(|(at, _)| at)
        .chain(std::iter::once(source.len()))
        .collect();
    let mut result = ProofreadResult {
        coverage: Coverage::Complete,
        findings: vec![],
        warnings: vec![],
        checked_characters: 0,
    };
    let rules: Vec<_> = options
        .rules
        .iter()
        .filter(|rule| match rule.category.as_str() {
            "typo" => options.typos,
            "sensitive" => options.sensitive,
            "character" => options.characters,
            _ => false,
        })
        .collect();
    let matcher = if rules.is_empty() {
        None
    } else {
        Some(
            aho_corasick::AhoCorasick::new(rules.iter().map(|r| &r.text))
                .map_err(|e| KnowledgeError::new("proofreadRule", e.to_string()))?,
        )
    };
    for (enabled, category, label) in [
        (options.typos, "typo", "错别字"),
        (options.sensitive, "sensitive", "敏感词"),
    ] {
        if enabled && !options.rules.iter().any(|r| r.category == category) {
            result
                .warnings
                .push(format!("{label}词典未配置；本次不包含该类词典检查"));
        }
    }
    for (fragment_no, fragment) in prose(tree, source).into_iter().enumerate() {
        let chars: Vec<_> = fragment.text.char_indices().collect();
        if result.checked_characters + chars.len() > 2_000_000 {
            result.coverage = Coverage::Partial;
            result
                .warnings
                .push("校对达到字符预算；结果不代表全文".into());
            break;
        }
        result.checked_characters += chars.len();
        let mut match_budget_exhausted = false;
        let mut add = |from: usize,
                       to: usize,
                       category: &str,
                       id: &str,
                       version: &str,
                       message: String,
                       replacement: Option<String>| {
            let start = chars.partition_point(|(b, _)| *b < from);
            let end = chars.partition_point(|(b, _)| *b < to);
            if from >= to || start >= end || end > chars.len() || result.findings.len() >= 2000 {
                return;
            }
            let excerpt = fragment.text[from..to].to_owned();
            if options.allowlist.iter().any(|s| s == &excerpt) {
                return;
            }
            let positions = &fragment.positions[start..end];
            let range = positions
                .iter()
                .copied()
                .collect::<Option<Vec<_>>>()
                .filter(|p| p.windows(2).all(|w| w[0].1 <= w[1].0))
                .and_then(|p| ExactRange::new(source, p.first()?.0, p.last()?.1).ok())
                .filter(|r| boundaries.contains(&r.from) && boundaries.contains(&r.to));
            if options
                .selection
                .is_some_and(|(a, b)| range.as_ref().is_none_or(|r| r.from < a || r.to > b))
            {
                return;
            }
            let executable =
                range.as_ref().is_some_and(|r| r.raw == excerpt) && replacement.is_some();
            result.findings.push(ProofreadFinding {
                id: format!("{id}:{version}:{fragment_no}:{from}:{to}"),
                category: category.into(),
                rule_id: id.into(),
                rule_version: version.into(),
                excerpt,
                message,
                replacement,
                source_range: range,
                executable,
            });
        };
        if let Some(matcher) = &matcher {
            for (n, hit) in matcher.find_overlapping_iter(&fragment.text).enumerate() {
                if n >= 20_000 {
                    match_budget_exhausted = true;
                    break;
                }
                let rule = rules[hit.pattern().as_usize()];
                add(
                    hit.start(),
                    hit.end(),
                    &rule.category,
                    &rule.id,
                    &rule.version,
                    rule.message.clone(),
                    rule.replacement.clone(),
                );
            }
        }
        if options.characters {
            for (i, c) in &chars {
                if *c == '\u{fffd}'
                    || *c == '\u{feff}'
                    || c.is_control() && !matches!(c, '\n' | '\r' | '\t')
                {
                    add(
                        *i,
                        *i + c.len_utf8(),
                        "character",
                        "builtin.suspicious-character",
                        "1",
                        format!("可疑字符 U+{:04X}，请核对来源与上下文", *c as u32),
                        None,
                    );
                }
            }
        }
        if options.pairs {
            let pairs = [
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('（', '）'),
                ('【', '】'),
                ('《', '》'),
                ('〈', '〉'),
                ('“', '”'),
                ('‘', '’'),
                ('「', '」'),
                ('『', '』'),
            ];
            let mut stack: Vec<(char, usize)> = vec![];
            for (n, (i, c)) in chars.iter().enumerate() {
                if *c == '’' && n > 0 && chars[n - 1].1.is_ascii_alphanumeric() {
                    continue;
                }
                if pairs.iter().any(|(a, _)| a == c) {
                    stack.push((*c, *i));
                } else if let Some((open, _)) = pairs.iter().find(|(_, b)| b == c) {
                    if let Some(index) = stack.iter().rposition(|(a, _)| a == open) {
                        if index + 1 != stack.len() {
                            add(
                                *i,
                                *i + c.len_utf8(),
                                "pair",
                                "builtin.crossed-pair",
                                "1",
                                "成对符号交叉嵌套，请核对上下文".into(),
                                None,
                            );
                        }
                        stack.remove(index);
                    } else {
                        add(
                            *i,
                            *i + c.len_utf8(),
                            "pair",
                            "builtin.unmatched-close",
                            "1",
                            "未找到对应的起始符号，请核对上下文".into(),
                            None,
                        );
                    }
                }
            }
            for (c, i) in stack {
                add(
                    i,
                    i + c.len_utf8(),
                    "pair",
                    "builtin.unmatched-open",
                    "1",
                    "未找到对应的结束符号，请核对上下文".into(),
                    None,
                );
            }
        }
        if result.findings.len() >= 2000 {
            result.coverage = Coverage::Partial;
            result
                .warnings
                .push("校对达到 2000 条结果预算；结果不代表全文".into());
            break;
        }
        if match_budget_exhausted {
            result.coverage = Coverage::Partial;
            result
                .warnings
                .push("词典匹配达到单段预算（含白名单与选区外命中）；结果不代表全文".into());
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tree(text: &str) -> RenderNode {
        serde_json::from_value(serde_json::json!({"kind":"element","key":"p","tag":"p","children":[{"kind":"text","key":"t","text":text,"children":[],"attrs":{},"writable":true,"origin":{"path":"main.typ","syntaxKind":"Text","from":0,"to":text.len(),"utf16From":0,"utf16To":text.encode_utf16().count(),"raw":text,"isMain":true}}],"attrs":{},"writable":false})).unwrap()
    }
    #[test]
    fn checks_unicode_overlap_pair_nesting_and_empty_dictionaries() {
        let source = "😀甲甲甲 ([)] ([]) don't ’";
        let mut options = ProofreadOptions::default();
        options.rules.push(ProofreadRule {
            id: "test.typo".into(),
            version: "1".into(),
            category: "typo".into(),
            text: "甲甲".into(),
            message: "疑似重复".into(),
            replacement: Some("甲".into()),
        });
        let result = proofread(&tree(source), source, &options).unwrap();
        let hits: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.category == "typo")
            .collect();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].source_range.as_ref().unwrap().utf16_from, 2);
        assert_eq!(
            result
                .findings
                .iter()
                .filter(|f| f.rule_id == "builtin.crossed-pair")
                .count(),
            1
        );
        assert!(result.warnings.iter().any(|w| w.contains("敏感词")));
        options.allowlist.push("甲甲".into());
        assert!(
            !proofread(&tree(source), source, &options)
                .unwrap()
                .findings
                .iter()
                .any(|f| f.category == "typo")
        );
    }
    #[test]
    fn excludes_code_and_does_not_grant_writes_for_generated_text() {
        let mut t = tree("([)]");
        t.tag = Some("code".into());
        assert!(
            proofread(&t, "([)]", &ProofreadOptions::default())
                .unwrap()
                .findings
                .is_empty()
        );
        t.tag = Some("p".into());
        t.children[0].origin = None;
        assert!(
            proofread(&t, "([)]", &ProofreadOptions::default())
                .unwrap()
                .findings
                .iter()
                .all(|f| f.source_range.is_none())
        );
    }
    #[test]
    fn filtered_matches_cannot_hide_partial_coverage_or_split_graphemes() {
        let mut options = ProofreadOptions::default();
        options.rules.push(ProofreadRule {
            id: "repeated".into(),
            version: "1".into(),
            category: "typo".into(),
            text: "a".into(),
            message: "check".into(),
            replacement: Some("b".into()),
        });
        options.allowlist.push("a".into());
        let source = "a".repeat(20_001);
        let result = proofread(&tree(&source), &source, &options).unwrap();
        assert_eq!(result.coverage, Coverage::Partial);
        assert!(result.findings.is_empty());
        options.allowlist.clear();
        let source = "a\u{301}";
        let result = proofread(&tree(source), source, &options).unwrap();
        assert_eq!(result.findings.len(), 1);
        assert!(result.findings[0].source_range.is_none());
        assert!(!result.findings[0].executable);
    }
}
