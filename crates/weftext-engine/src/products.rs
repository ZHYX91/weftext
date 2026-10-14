//! Query products consume the successful final native HTML product only.
use crate::{HeadingInfo, RenderNode};
use serde::{Deserialize, Serialize};
use typst_html::HtmlNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderProducts {
    pub target: String,
    pub body_status: String,
    pub body: Option<String>,
    pub title_status: String,
    pub title: Option<String>,
    pub metadata_title: Option<String>,
    pub native_title_bodies: Vec<String>,
    pub headings: Vec<HeadingInfo>,
}

pub(crate) fn native_products(
    native: &HtmlNode,
    tree: &RenderNode,
    metadata_title: Option<String>,
    headings: Vec<HeadingInfo>,
) -> RenderProducts {
    fn producer_bodies(node: &HtmlNode, out: &mut Vec<String>) {
        if let HtmlNode::Element(element) = node {
            if element.weftext.role == Some("title")
                && let Some(body) = &element.weftext.body
            {
                out.push(body.to_string());
            }
            for child in &element.children {
                producer_bodies(child, out);
            }
        }
    }
    fn text(node: &RenderNode, out: &mut String, payload_end: &mut usize) -> bool {
        if node.kind == "tag" {
            return true;
        }
        if node.kind == "frame"
            || node
                .attrs
                .get("class")
                .is_some_and(|v| v == "weftext-unsupported")
        {
            return false;
        }
        let tag = node.tag.as_deref().unwrap_or("");
        if matches!(tag, "annotation" | "annotation-xml") {
            return true;
        }
        if node.kind == "text" {
            out.push_str(node.text.as_deref().unwrap_or(""));
            *payload_end = out.len();
            return true;
        }
        let mut complete = true;
        for child in &node.children {
            complete &= text(child, out, payload_end);
        }
        if tag == "br" {
            out.push('\n');
            *payload_end = out.len();
        } else if matches!(tag, "td" | "th") {
            out.push('\t');
        } else if matches!(
            tag,
            "p" | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "li"
                | "dt"
                | "dd"
                | "tr"
                | "table"
                | "pre"
                | "blockquote"
                | "figcaption"
        ) || node.attrs.get("role").is_some_and(|r| r == "heading")
        {
            out.push('\n');
        }
        complete
    }
    let mut native_title_bodies = vec![];
    producer_bodies(native, &mut native_title_bodies);
    let (title_status, title) = match (&metadata_title, native_title_bodies.as_slice()) {
        (Some(metadata), []) => ("value", Some(metadata.clone())),
        (Some(metadata), [native]) if native == metadata => ("value", Some(metadata.clone())),
        (None, [native]) => ("value", Some(native.clone())),
        (None, []) => ("absent", None),
        _ => ("ambiguous", None),
    };
    let mut body = String::new();
    let mut payload_end = 0;
    let complete = text(tree, &mut body, &mut payload_end);
    body.truncate(payload_end);
    RenderProducts {
        target: "html".into(),
        body_status: if complete { "complete" } else { "unavailable" }.into(),
        body: complete.then_some(body),
        title_status: title_status.into(),
        title,
        metadata_title,
        native_title_bodies,
        headings,
    }
}
