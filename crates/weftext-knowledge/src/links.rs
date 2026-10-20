use crate::*;
use serde::{Deserialize, Serialize};
use typst_syntax::{
    LinkedNode, Source,
    ast::{self, AstNode},
};
use url::Url;

pub(crate) fn normalized_uri_scheme(raw: &str) -> std::borrow::Cow<'_, str> {
    let Some((scheme, _)) = raw.split_once(':') else {
        return std::borrow::Cow::Borrowed(raw);
    };
    if scheme
        .as_bytes()
        .first()
        .is_none_or(|b| !b.is_ascii_alphabetic())
        || !scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b))
    {
        return std::borrow::Cow::Borrowed(raw);
    }
    if !scheme.bytes().any(|b| b.is_ascii_uppercase()) {
        return std::borrow::Cow::Borrowed(raw);
    }
    std::borrow::Cow::Owned(format!(
        "{}{}",
        scheme.to_ascii_lowercase(),
        &raw[scheme.len()..]
    ))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkForm {
    Id,
    Relative,
    Workspace,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeLink {
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
    pub target: String,
    pub kind: String,
    pub destination_slot: Option<ExactRange>,
    pub body_explicit: bool,
}

/// Enumerates real native syntax independently of rendered occurrences. Calls
/// with an unproven standard binding are retained elsewhere as source syntax,
/// and deliberately never enter the actual link index.
pub fn native_links(source: &str) -> Vec<NativeLink> {
    native_links_bounded(source, usize::MAX, usize::MAX).links
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeLinkScan {
    pub links: Vec<NativeLink>,
    pub complete: bool,
    pub estimated_bytes: usize,
}

/// Bounds discovery before collecting an unbounded AST occurrence list. A
/// stopped scan reports partial coverage, without inventing the omitted count.
pub fn native_links_bounded(
    source: &str,
    link_budget: usize,
    output_budget: usize,
) -> NativeLinkScan {
    let parsed = Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return NativeLinkScan {
            links: vec![],
            complete: false,
            estimated_bytes: 0,
        };
    }
    let standard = weftext_engine::standard_binding_available(source, "link");
    fn walk(
        node: &LinkedNode<'_>,
        source: &str,
        standard: bool,
        scan: &mut NativeLinkScan,
        link_budget: usize,
        output_budget: usize,
    ) -> bool {
        let range = node.range();
        let target = if let Some(link) = node.get().cast::<ast::Link>() {
            Some((link.get().to_string(), "url", None, false))
        } else if let Some(reference) = node.get().cast::<ast::Ref>() {
            Some((format!("#{}", reference.target()), "nativeRef", None, false))
        } else if standard {
            node.get().cast::<ast::FuncCall>().and_then(|call| {
                if call.callee().to_untyped().cast::<ast::Ident>()?.get() != "link" {
                    return None;
                }
                let args: Vec<_> = call.args().items().collect();
                // Official LinkElem consumes positional destination/body only.
                // Named or spread arguments do not certify a real link call.
                if args.iter().any(|arg| !matches!(arg, ast::Arg::Pos(_))) {
                    return None;
                }
                let positional: Vec<_> = args
                    .iter()
                    .filter_map(|arg| {
                        if let ast::Arg::Pos(expr) = arg {
                            Some(*expr)
                        } else {
                            None
                        }
                    })
                    .collect();
                if positional.len() > 2 {
                    return None;
                }
                let expr = *positional.first()?;
                let (text, kind) = if let Some(value) = expr.to_untyped().cast::<ast::Str>() {
                    (value.get().to_string(), "native")
                } else {
                    let label = expr.to_untyped().cast::<ast::Label>()?;
                    (format!("#{}", label.get()), "nativeLabel")
                };
                let child = node.find(expr.to_untyped().span())?;
                let slot = ExactRange::new(source, child.range().start, child.range().end).ok()?;
                if kind == "nativeLabel" && positional.len() < 2 {
                    return None;
                }
                Some((text, kind, Some(slot), positional.len() == 2))
            })
        } else {
            None
        };
        if let Some((target, kind, destination_slot, body_explicit)) = target {
            let cost = target
                .len()
                .saturating_add(destination_slot.as_ref().map_or(0, |slot| slot.raw.len()))
                .saturating_add(512);
            if scan.links.len() >= link_budget
                || scan.estimated_bytes.saturating_add(cost) > output_budget
            {
                return false;
            }
            scan.estimated_bytes = scan.estimated_bytes.saturating_add(cost);
            scan.links.push(NativeLink {
                from: range.start,
                to: range.end,
                utf16_from: source[..range.start].encode_utf16().count(),
                utf16_to: source[..range.end].encode_utf16().count(),
                target,
                kind: kind.into(),
                destination_slot,
                body_explicit,
            });
        }
        for child in node.children() {
            if !walk(&child, source, standard, scan, link_budget, output_budget) {
                return false;
            }
        }
        true
    }
    let mut scan = NativeLinkScan {
        links: vec![],
        complete: true,
        estimated_bytes: 0,
    };
    scan.complete = walk(
        &LinkedNode::new(parsed.root()),
        source,
        standard,
        &mut scan,
        link_budget,
        output_budget,
    );
    scan
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkResolution {
    pub status: String,
    pub target: Option<NodeRef>,
    pub resource: Option<String>,
    pub fragment: Option<String>,
    pub form: Option<LinkForm>,
}
impl LinkResolution {
    fn status(status: &str) -> Self {
        Self {
            status: status.into(),
            target: None,
            resource: None,
            fragment: None,
            form: None,
        }
    }
}

fn decode_component(value: &str) -> Result<String> {
    let mut bytes = vec![];
    let input = value.as_bytes();
    let mut pos = 0;
    while pos < input.len() {
        if input[pos] == b'%' {
            let hex = input
                .get(pos + 1..pos + 3)
                .ok_or_else(|| KnowledgeError::new("invalidUri", "Malformed percent escape"))?;
            let pair = std::str::from_utf8(hex)
                .map_err(|_| KnowledgeError::new("invalidUri", "Malformed escape"))?;
            let b = u8::from_str_radix(pair, 16)
                .map_err(|_| KnowledgeError::new("invalidUri", "Malformed percent escape"))?;
            if b == b'/' || b == b'\\' {
                return Err(KnowledgeError::new(
                    "invalidUri",
                    "Encoded separators are forbidden",
                ));
            }
            bytes.push(b);
            pos += 3;
        } else {
            bytes.push(input[pos]);
            pos += 1;
        }
    }
    let result = String::from_utf8(bytes)
        .map_err(|_| KnowledgeError::new("invalidUri", "URI component is not UTF-8"))?;
    if result.chars().any(char::is_control) || result.contains('\\') {
        return Err(KnowledgeError::new(
            "invalidUri",
            "Controls and backslashes are forbidden",
        ));
    }
    Ok(result)
}
fn encode_component(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
fn path_components(value: &str) -> Result<Vec<String>> {
    value.split('/').map(decode_component).collect()
}
fn canonical_library_path(value: &str) -> Result<Vec<String>> {
    let parts = path_components(value)?;
    if parts.iter().any(|p| p.is_empty() || p == "." || p == "..") {
        return Err(KnowledgeError::new(
            "invalidUri",
            "Inventory paths must be canonical",
        ));
    }
    Ok(parts)
}
fn fragment_split(raw: &str) -> Result<(&str, Option<String>)> {
    if let Some((address, fragment)) = raw.split_once('#') {
        if fragment.contains('#') {
            return Err(KnowledgeError::new(
                "invalidUri",
                "Fragment contains an unencoded separator",
            ));
        }
        Ok((address, Some(decode_component(fragment)?)))
    } else {
        Ok((raw, None))
    }
}
fn node_resolution(
    nodes: Vec<&NodeSnapshot>,
    fragment: Option<String>,
    form: LinkForm,
    coverage: Coverage,
) -> LinkResolution {
    let mut result = LinkResolution::status("resolved");
    result.form = Some(form);
    result.fragment = fragment;
    let [node] = nodes.as_slice() else {
        result.status = if nodes.is_empty() {
            if coverage == Coverage::Complete {
                "missing"
            } else {
                "unavailable"
            }
        } else {
            "ambiguous"
        }
        .into();
        return result;
    };
    result.target = Some(node.owner.clone());
    if let Some(fragment) = &result.fragment {
        let count = node
            .labels
            .iter()
            .filter(|label| *label == fragment)
            .count();
        result.status = if !node.labels_complete {
            "unavailable"
        } else if count == 0 {
            "fragmentMissing"
        } else if count > 1 {
            "ambiguous"
        } else {
            "resolved"
        }
        .into();
    }
    result
}
fn unresolved_case_alias(fragment: Option<String>, form: LinkForm) -> LinkResolution {
    let mut result = LinkResolution::status("unavailable");
    result.fragment = fragment;
    result.form = Some(form);
    result
}

fn resource_resolution(
    resources: Vec<&ResourceSnapshot>,
    fragment: Option<String>,
    form: LinkForm,
    coverage: Coverage,
) -> LinkResolution {
    let mut result = LinkResolution::status(if resources.len() > 1 {
        "ambiguous"
    } else if resources.len() == 1 && fragment.is_none() {
        "resolved"
    } else if resources.len() == 1 || coverage == Coverage::Partial {
        "unavailable"
    } else {
        "missing"
    });
    result.form = Some(form);
    result.fragment = fragment;
    result.resource = resources
        .first()
        .filter(|_| resources.len() == 1)
        .map(|r| r.key.clone());
    result
}

fn located_resolution(
    nodes: Vec<&NodeSnapshot>,
    resources: Vec<&ResourceSnapshot>,
    fragment: Option<String>,
    form: LinkForm,
    coverage: Coverage,
) -> LinkResolution {
    if !nodes.is_empty() && !resources.is_empty() {
        let mut result = LinkResolution::status("ambiguous");
        result.form = Some(form);
        result.fragment = fragment;
        result
    } else if !nodes.is_empty() {
        node_resolution(nodes, fragment, form, coverage)
    } else {
        resource_resolution(resources, fragment, form, coverage)
    }
}

fn resource_workspace(resource: &ResourceSnapshot) -> Option<&str> {
    let rest = resource.key.strip_prefix("weftext-resource://")?;
    let (workspace, id) = rest.split_once('/')?;
    (uuid::Uuid::parse_str(workspace).is_ok() && uuid::Uuid::parse_str(id).is_ok())
        .then_some(workspace)
}

/// The admitted node's canonical file path and library-relative path jointly
/// establish a real root. Resource paths never acquire authority from a title
/// or a guessed filename. Inventory library paths are raw filesystem names.
fn resource_library_path_from(
    source: &NodeSnapshot,
    resource: &ResourceSnapshot,
) -> Option<String> {
    if resource_workspace(resource)? != source.owner.workspace_id {
        return None;
    }
    let source_file = Url::parse(&source.path).ok()?.to_file_path().ok()?;
    let target_file = Url::parse(&resource.path).ok()?.to_file_path().ok()?;
    let parts: Vec<_> = source.library_path.split('/').collect();
    if parts.iter().any(|p| {
        p.is_empty()
            || matches!(*p, "." | "..")
            || p.contains('\\')
            || p.chars().any(char::is_control)
    }) {
        return None;
    }
    let mut root = source_file;
    for part in parts.iter().rev() {
        if root.file_name()?.to_str()? != *part || !root.pop() {
            return None;
        }
    }
    let relative = target_file.strip_prefix(root).ok()?;
    let mut target_parts = vec![];
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return None;
        };
        target_parts.push(part.to_str()?.to_owned());
    }
    (!target_parts.is_empty()).then(|| target_parts.join("/"))
}

fn resource_library_path(
    resource: &ResourceSnapshot,
    inventory: &InventorySnapshot,
) -> Option<String> {
    let paths: std::collections::BTreeSet<_> = inventory
        .nodes
        .iter()
        .filter_map(|source| resource_library_path_from(source, resource))
        .collect();
    (paths.len() == 1)
        .then(|| paths.into_iter().next())
        .flatten()
}

/// Fragment qualification is supplied by the actual resource reader, against
/// the frozen resource revision. A string resembling a PDF page is no proof.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResourceFragmentStatus {
    Found,
    Missing,
    Ambiguous,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFragmentQualification {
    pub resource: String,
    pub revision_id: String,
    pub fragment: String,
    pub status: ResourceFragmentStatus,
}

pub fn resolve_link_with_resource_fragments(
    raw: &str,
    source: &NodeSnapshot,
    inventory: &InventorySnapshot,
    fragments: &[ResourceFragmentQualification],
) -> LinkResolution {
    let mut resolution = resolve_link(raw, source, inventory);
    if resolution.status != "unavailable" {
        return resolution;
    }
    let (Some(resource), Some(fragment)) = (&resolution.resource, &resolution.fragment) else {
        return resolution;
    };
    let Some(snapshot) = inventory.resources.iter().find(|r| &r.key == resource) else {
        return resolution;
    };
    let matches: Vec<_> = fragments
        .iter()
        .filter(|q| {
            &q.resource == resource
                && &q.fragment == fragment
                && q.revision_id == snapshot.revision_id
        })
        .collect();
    resolution.status = match matches.as_slice() {
        [qualification] => match qualification.status {
            ResourceFragmentStatus::Found => "resolved",
            ResourceFragmentStatus::Missing => "fragmentMissing",
            ResourceFragmentStatus::Ambiguous => "ambiguous",
            ResourceFragmentStatus::Unavailable => "unavailable",
        },
        [] => "unavailable",
        _ => "ambiguous",
    }
    .into();
    resolution
}

/// Resolves only the frozen, already authorized inventory. A missing object in a
/// partial inventory is unavailable, without implying workspace-wide absence.
pub fn resolve_link(
    raw: &str,
    source: &NodeSnapshot,
    inventory: &InventorySnapshot,
) -> LinkResolution {
    let normalized = normalized_uri_scheme(raw);
    let raw = normalized.as_ref();
    let run = || -> Result<LinkResolution> {
        let (address, fragment) = fragment_split(raw)?;
        if address.is_empty() && fragment.is_some() {
            let mut result = LinkResolution::status("inDocument");
            result.fragment = fragment;
            return Ok(result);
        }
        if let Some(rest) = address.strip_prefix("weftext-node://") {
            let parts = path_components(rest)?;
            let [workspace, node] = parts.as_slice() else {
                return Err(KnowledgeError::new(
                    "invalidUri",
                    "ID links require full workspace and node IDs",
                ));
            };
            if uuid::Uuid::parse_str(workspace).is_err() || uuid::Uuid::parse_str(node).is_err() {
                return Err(KnowledgeError::new("invalidUri", "IDs must be UUIDs"));
            }
            return Ok(node_resolution(
                inventory
                    .nodes
                    .iter()
                    .filter(|n| n.owner.workspace_id == *workspace && n.owner.node_id == *node)
                    .collect(),
                fragment,
                LinkForm::Id,
                inventory.coverage,
            ));
        }
        if let Some(rest) = address.strip_prefix("weftext-resource://") {
            let parts = path_components(rest)?;
            let [workspace, resource] = parts.as_slice() else {
                return Err(KnowledgeError::new(
                    "invalidUri",
                    "Resource addresses require a full workspace and resource identity",
                ));
            };
            if uuid::Uuid::parse_str(workspace).is_err() || uuid::Uuid::parse_str(resource).is_err()
            {
                return Err(KnowledgeError::new(
                    "invalidUri",
                    "Resource identity is invalid",
                ));
            }
            let key = format!("weftext-resource://{workspace}/{resource}");
            return Ok(resource_resolution(
                inventory
                    .resources
                    .iter()
                    .filter(|r| r.key == key)
                    .collect(),
                fragment,
                LinkForm::Id,
                inventory.coverage,
            ));
        }
        if let Some(rest) = address.strip_prefix("weftext-workspace://") {
            let (workspace, path) = rest.split_once('/').ok_or_else(|| {
                KnowledgeError::new("invalidUri", "Workspace links require a path")
            })?;
            let workspace = decode_component(workspace)?;
            if uuid::Uuid::parse_str(&workspace).is_err() {
                return Err(KnowledgeError::new(
                    "invalidUri",
                    "Unknown workspace authority",
                ));
            }
            let path = canonical_library_path(path)?.join("/");
            let nodes: Vec<_> = inventory
                .nodes
                .iter()
                .filter(|n| n.owner.workspace_id == workspace && n.library_path == path)
                .collect();
            let resource_paths: Vec<_> = inventory
                .resources
                .iter()
                .filter(|r| resource_workspace(r) == Some(workspace.as_str()))
                .filter_map(|r| resource_library_path(r, inventory).map(|p| (r, p)))
                .collect();
            let resources: Vec<_> = resource_paths
                .iter()
                .filter(|(_, p)| p == &path)
                .map(|(r, _)| *r)
                .collect();
            if nodes.is_empty()
                && resources.is_empty()
                && (inventory.nodes.iter().any(|n| {
                    n.owner.workspace_id == workspace && n.library_path.eq_ignore_ascii_case(&path)
                }) || resource_paths
                    .iter()
                    .any(|(_, p)| p.eq_ignore_ascii_case(&path)))
            {
                return Ok(unresolved_case_alias(fragment, LinkForm::Workspace));
            }
            return Ok(located_resolution(
                nodes,
                resources,
                fragment,
                LinkForm::Workspace,
                inventory.coverage,
            ));
        }
        if address.starts_with("file:") {
            let url = Url::parse(address)
                .map_err(|_| KnowledgeError::new("invalidUri", "Invalid platform file URL"))?;
            if url.scheme() != "file"
                || url.host_str().is_some_and(|host| !host.is_empty())
                || url.query().is_some()
            {
                return Err(KnowledgeError::new(
                    "invalidUri",
                    "Only local platform file URLs are supported",
                ));
            }
            for component in address.split('/') {
                if matches!(decode_component(component)?.as_str(), "." | "..") {
                    return Err(KnowledgeError::new(
                        "invalidUri",
                        "File URL aliases require Core canonicalization",
                    ));
                }
            }
            if url
                .path_segments()
                .is_some_and(|parts| parts.clone().any(|p| p == "." || p == ".."))
                || address.contains("/../")
                || address.contains("/./")
            {
                return Err(KnowledgeError::new(
                    "invalidUri",
                    "File URL aliases require Core canonicalization",
                ));
            }
            let requested_path = url.to_file_path().map_err(|_| {
                KnowledgeError::new("invalidUri", "File URL does not form a local platform path")
            })?;
            let same_path = |candidate: &str| {
                Url::parse(candidate)
                    .ok()
                    .and_then(|url| url.to_file_path().ok())
                    .is_some_and(|path| path == requested_path)
            };
            let nodes: Vec<_> = inventory
                .nodes
                .iter()
                .filter(|n| same_path(&n.path))
                .collect();
            let resources: Vec<_> = inventory
                .resources
                .iter()
                .filter(|r| same_path(&r.path))
                .collect();
            if nodes.is_empty()
                && resources.is_empty()
                && inventory
                    .nodes
                    .iter()
                    .map(|n| n.path.as_str())
                    .chain(inventory.resources.iter().map(|r| r.path.as_str()))
                    .any(|candidate| {
                        Url::parse(candidate)
                            .ok()
                            .and_then(|u| u.to_file_path().ok())
                            .is_some_and(|path| {
                                path.to_string_lossy()
                                    .eq_ignore_ascii_case(&requested_path.to_string_lossy())
                            })
                    })
            {
                return Ok(unresolved_case_alias(fragment, LinkForm::File));
            }
            return Ok(located_resolution(
                nodes,
                resources,
                fragment,
                LinkForm::File,
                inventory.coverage,
            ));
        }
        if address.contains(':') {
            return Ok(LinkResolution::status(
                if address.starts_with("http://")
                    || address.starts_with("https://")
                    || address.starts_with("mailto:")
                {
                    "external"
                } else {
                    "invalid"
                },
            ));
        }
        if address.starts_with('/') || address.contains('?') || address.is_empty() {
            return Err(KnowledgeError::new(
                "invalidUri",
                "Relative links require a nonempty local source-relative path",
            ));
        }
        let mut source_parts: Vec<_> = source.library_path.split('/').map(str::to_owned).collect();
        source_parts.pop();
        for component in path_components(address)? {
            match component.as_str() {
                "." => {}
                ".." => {
                    if source_parts.pop().is_none() {
                        return Err(KnowledgeError::new(
                            "outsideRoot",
                            "Relative link escapes the library",
                        ));
                    }
                }
                "" => return Err(KnowledgeError::new("invalidUri", "Empty path component")),
                _ => source_parts.push(component),
            }
        }
        let path = source_parts.join("/");
        let nodes: Vec<_> = inventory
            .nodes
            .iter()
            .filter(|n| n.owner.workspace_id == source.owner.workspace_id && n.library_path == path)
            .collect();
        let resource_paths: Vec<_> = inventory
            .resources
            .iter()
            .filter_map(|r| resource_library_path_from(source, r).map(|p| (r, p)))
            .collect();
        let resources: Vec<_> = resource_paths
            .iter()
            .filter(|(_, p)| p == &path)
            .map(|(r, _)| *r)
            .collect();
        if nodes.is_empty()
            && resources.is_empty()
            && (inventory.nodes.iter().any(|n| {
                n.owner.workspace_id == source.owner.workspace_id
                    && n.library_path.eq_ignore_ascii_case(&path)
            }) || resource_paths
                .iter()
                .any(|(_, p)| p.eq_ignore_ascii_case(&path)))
        {
            return Ok(unresolved_case_alias(fragment, LinkForm::Relative));
        }
        Ok(located_resolution(
            nodes,
            resources,
            fragment,
            LinkForm::Relative,
            inventory.coverage,
        ))
    };
    run().unwrap_or_else(|_| LinkResolution::status("invalid"))
}

pub fn target_address(
    source: &NodeSnapshot,
    target: &NodeSnapshot,
    fragment: Option<&str>,
    form: LinkForm,
) -> Result<String> {
    let mut address = match form {
        LinkForm::Id => format!(
            "weftext-node://{}/{}",
            target.owner.workspace_id, target.owner.node_id
        ),
        LinkForm::Workspace => format!(
            "weftext-workspace://{}/{}",
            target.owner.workspace_id,
            target
                .library_path
                .split('/')
                .map(encode_component)
                .collect::<Vec<_>>()
                .join("/")
        ),
        LinkForm::File => {
            let parsed = Url::parse(&target.path).map_err(|_| {
                KnowledgeError::new(
                    "invalidPath",
                    "Core must provide a canonical platform file URL",
                )
            })?;
            if parsed.scheme() != "file"
                || parsed.host_str().is_some()
                || parsed.fragment().is_some()
                || parsed.query().is_some()
            {
                return Err(KnowledgeError::new(
                    "invalidPath",
                    "Not a canonical local file URL",
                ));
            }
            target.path.clone()
        }
        LinkForm::Relative => {
            if source.owner.workspace_id != target.owner.workspace_id {
                return Err(KnowledgeError::new(
                    "unconnectedLibrary",
                    "A source-relative path cannot cross libraries",
                ));
            }
            let mut from: Vec<_> = source.library_path.split('/').collect();
            from.pop();
            let to: Vec<_> = target.library_path.split('/').collect();
            let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
            let mut result: Vec<String> = (common..from.len()).map(|_| "..".into()).collect();
            result.extend(to[common..].iter().map(|p| encode_component(p)));
            result.join("/")
        }
    };
    if let Some(fragment) = fragment {
        address.push('#');
        address.push_str(&encode_component(fragment));
    }
    Ok(address)
}

pub fn resource_address(
    source: &NodeSnapshot,
    resource: &ResourceSnapshot,
    fragment: Option<&str>,
    form: LinkForm,
    inventory: &InventorySnapshot,
) -> Result<String> {
    let mut address = match form {
        LinkForm::Id => {
            resource_workspace(resource).ok_or_else(|| {
                KnowledgeError::new(
                    "resourceIdentity",
                    "The resource has no admitted full identity",
                )
            })?;
            resource.key.clone()
        }
        LinkForm::File => {
            let parsed = Url::parse(&resource.path).map_err(|_| {
                KnowledgeError::new(
                    "invalidPath",
                    "Core must provide a canonical resource file URL",
                )
            })?;
            if parsed.scheme() != "file"
                || parsed.host_str().is_some()
                || parsed.fragment().is_some()
                || parsed.query().is_some()
                || parsed.to_file_path().is_err()
            {
                return Err(KnowledgeError::new(
                    "invalidPath",
                    "The resource has no canonical local file URL",
                ));
            }
            resource.path.clone()
        }
        LinkForm::Workspace => {
            let workspace = resource_workspace(resource).ok_or_else(|| {
                KnowledgeError::new(
                    "resourceIdentity",
                    "The resource has no full workspace identity",
                )
            })?;
            let path = resource_library_path(resource, inventory).ok_or_else(|| {
                KnowledgeError::new(
                    "unavailable",
                    "A canonical admitted library root does not qualify the resource path",
                )
            })?;
            format!(
                "weftext-workspace://{workspace}/{}",
                path.split('/')
                    .map(encode_component)
                    .collect::<Vec<_>>()
                    .join("/")
            )
        }
        LinkForm::Relative => {
            let path=resource_library_path_from(source,resource).ok_or_else(||KnowledgeError::new("unavailable","The source and resource have no qualified path within the same admitted library"))?;
            let mut from: Vec<_> = source.library_path.split('/').collect();
            from.pop();
            let to: Vec<_> = path.split('/').collect();
            let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
            let mut parts: Vec<String> = (common..from.len()).map(|_| "..".into()).collect();
            parts.extend(to[common..].iter().map(|p| encode_component(p)));
            parts.join("/")
        }
    };
    if let Some(fragment) = fragment {
        address.push('#');
        address.push_str(&encode_component(fragment));
    }
    Ok(address)
}

/// One exact native argument/call patch; body AST and outside bytes
/// are untouched. Computed/shared destinations have no shortcut writer.
pub fn convert_link(
    source: &NodeSnapshot,
    inventory: &InventorySnapshot,
    occurrence_from: usize,
    form: LinkForm,
) -> Result<SourcePatch> {
    convert_link_with_resource_fragments(source, inventory, occurrence_from, form, &[])
}

pub fn convert_link_with_resource_fragments(
    source: &NodeSnapshot,
    inventory: &InventorySnapshot,
    occurrence_from: usize,
    form: LinkForm,
    fragments: &[ResourceFragmentQualification],
) -> Result<SourcePatch> {
    let links: Vec<_> = native_links(&source.source)
        .into_iter()
        .filter(|link| link.from == occurrence_from)
        .collect();
    let [link] = links.as_slice() else {
        return Err(KnowledgeError::new(
            "unmapped",
            "Select one exact authored native link",
        ));
    };
    let resolution =
        resolve_link_with_resource_fragments(&link.target, source, inventory, fragments);
    if resolution.status != "resolved" {
        return Err(KnowledgeError::new(
            &resolution.status,
            "The current link target or fragment is not uniquely resolved",
        ));
    }
    let mut address = if let Some(target) = &resolution.target {
        let target = inventory
            .nodes
            .iter()
            .find(|n| &n.owner == target)
            .ok_or_else(|| {
                KnowledgeError::new("unavailable", "The resolved node left the frozen inventory")
            })?;
        target_address(source, target, None, form)?
    } else if let Some(resource) = &resolution.resource {
        let resource = inventory
            .resources
            .iter()
            .find(|r| &r.key == resource)
            .ok_or_else(|| {
                KnowledgeError::new(
                    "unavailable",
                    "The resolved resource left the frozen inventory",
                )
            })?;
        resource_address(source, resource, None, form, inventory)?
    } else {
        return Err(KnowledgeError::new(
            "unavailable",
            "The target has no admitted node or resource identity",
        ));
    };
    // Address form changes do not canonicalize or alter the authored fragment
    // spelling. Its decoded meaning was already qualified above.
    if let Some((_, fragment)) = link.target.split_once('#') {
        address.push('#');
        address.push_str(fragment);
    }
    let verified = resolve_link_with_resource_fragments(&address, source, inventory, fragments);
    if verified
        != (LinkResolution {
            form: Some(form),
            ..resolution.clone()
        })
    {
        return Err(KnowledgeError::new(
            "roundtrip",
            "Converted address did not resolve to the identical target and fragment",
        ));
    }
    let (slot, encoded) = destination_patch(&source.source, link, &address)?;
    let patch = SourcePatch::new(&source.source, slot, encoded)?;
    let after = native_links(&patch.after);
    let rewritten = after.iter().find(|l| l.from == link.from).ok_or_else(|| {
        KnowledgeError::new("roundtrip", "Native link disappeared after parser readback")
    })?;
    if rewritten.target != address {
        return Err(KnowledgeError::new(
            "roundtrip",
            "Native parser readback changed the destination",
        ));
    }
    Ok(patch)
}

pub(crate) fn destination_patch(
    source: &str,
    link: &NativeLink,
    address: &str,
) -> Result<(ExactRange, String)> {
    let slot = link.destination_slot.clone().ok_or_else(|| {
        KnowledgeError::new(
            "sourceEntry",
            "This native URL needs an explicit source edit",
        )
    })?;
    let encoded = weftext_engine::encode_string(address)
        .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?;
    if link.body_explicit {
        return Ok((slot, encoded));
    }
    let range = ExactRange::new(source, link.from, link.to)?;
    let display = weftext_engine::encode_literal(&link.target)
        .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?;
    let replacement = format!(
        "{}{}{}[{display}]",
        &source[range.from..slot.from],
        encoded,
        &source[slot.to..range.to]
    );
    Ok((range, replacement))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkEdge {
    pub source: NodeRef,
    pub occurrence: NativeLink,
    pub resolution: LinkResolution,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkIndex {
    pub generation: String,
    pub coverage: Coverage,
    pub outgoing: Vec<LinkEdge>,
    pub incoming_id: Vec<LinkEdge>,
    pub incoming_path: Vec<LinkEdge>,
}

/// Completeness of an authored-source-only link projection. The exact native
/// scan must also be complete; callers retain separate compiled occurrences.
pub fn source_link_projection_complete(source: &str, links: &[NativeLink]) -> bool {
    let parsed = Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return false;
    }
    let editor = weftext_engine::inspect_editor(source);
    let literal_title = editor.unavailable_reason.is_none()
        && editor.title.status == "writable"
        && editor.title.encoding.as_deref() == Some("markup")
        && editor.title.range.as_ref().is_some_and(|range| {
            range.writable
                && ExactRange::new(source, range.from, range.to).is_ok_and(|exact| {
                    exact.raw == range.raw
                        && exact.utf16_from == range.utf16_from
                        && exact.utf16_to == range.utf16_to
                })
                && {
                    let literal = Source::detached(&range.raw);
                    literal.root().errors_and_warnings().0.is_empty()
                        && literal.root().children().all(|node| {
                            matches!(
                                node.kind(),
                                typst_syntax::SyntaxKind::Text
                                    | typst_syntax::SyntaxKind::Escape
                                    | typst_syntax::SyntaxKind::Shorthand
                                    | typst_syntax::SyntaxKind::Space
                            )
                        })
                }
        });
    fn safe(node: &LinkedNode<'_>, links: &[NativeLink], literal_title: bool) -> bool {
        if matches!(
            node.kind(),
            typst_syntax::SyntaxKind::ModuleInclude
                | typst_syntax::SyntaxKind::ModuleImport
                | typst_syntax::SyntaxKind::ShowRule
        ) {
            return false;
        }
        if let Some(call) = node.get().cast::<ast::FuncCall>() {
            let Some(name) = call.callee().to_untyped().cast::<ast::Ident>() else {
                return false;
            };
            if name.get() == "link" {
                if !links.iter().any(|link| link.from == node.range().start) {
                    return false;
                }
            }
            // The default native document title is certified by the editor's
            // exact role and consists only of ordinary literal syntax. Only a
            // parameterless standard title call can use that proof.
            else if name.get() == "title" {
                if !literal_title || call.args().items().next().is_some() {
                    return false;
                }
            }
            // Arbitrary native/user function output can contain compiled links.
            // Metadata itself has no displayed output. Its descendants still
            // undergo this check. A source-only index cannot certify absence of
            // an unobserved compiled product from any other call.
            else if name.get() != "metadata" {
                return false;
            }
        }
        // A title-specific set rule can replace its body/defaults independently
        // of document metadata; it is outside this narrow default-title proof.
        if node.get().cast::<ast::SetRule>().is_some_and(|set| {
            set.target()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|name| name.get() == "title")
        }) {
            return false;
        }
        node.children()
            .all(|child| safe(&child, links, literal_title))
    }
    safe(&LinkedNode::new(parsed.root()), links, literal_title)
}

pub fn link_index(inventory: &InventorySnapshot, selected: &NodeRef) -> LinkIndex {
    link_index_bounded(inventory, selected, 20_000, 8 * 1024 * 1024)
}

/// Budgets count scanned occurrences and their potential output, including
/// occurrences that later do not point at the selected note. The selected
/// source is visited first; stopped authorized work is explicitly partial.
pub fn link_index_bounded(
    inventory: &InventorySnapshot,
    selected: &NodeRef,
    edge_budget: usize,
    output_budget: usize,
) -> LinkIndex {
    let mut result = LinkIndex {
        generation: inventory.generation.clone(),
        coverage: inventory.coverage,
        outgoing: vec![],
        incoming_id: vec![],
        incoming_path: vec![],
    };
    let mut scanned = 0usize;
    let mut estimated_bytes = 0usize;
    let sources = inventory
        .nodes
        .iter()
        .filter(|n| &n.owner == selected)
        .chain(inventory.nodes.iter().filter(|n| &n.owner != selected));
    for source in sources {
        // An edge can appear in both outgoing and incoming projections. Reserve
        // for those two copies and the resolution/owner DTO before collecting.
        let scan = native_links_bounded(
            &source.source,
            edge_budget.saturating_sub(scanned),
            output_budget.saturating_sub(estimated_bytes) / 2,
        );
        scanned = scanned.saturating_add(scan.links.len());
        estimated_bytes = estimated_bytes.saturating_add(scan.estimated_bytes.saturating_mul(2));
        if !scan.complete || !source_link_projection_complete(&source.source, &scan.links) {
            result.coverage = Coverage::Partial;
        }
        for occurrence in scan.links {
            let resolution = resolve_link(&occurrence.target, source, inventory);
            if matches!(resolution.status.as_str(), "unavailable" | "ambiguous") {
                result.coverage = Coverage::Partial;
            }
            let edge = LinkEdge {
                source: source.owner.clone(),
                occurrence,
                resolution,
            };
            if &source.owner == selected {
                result.outgoing.push(edge.clone());
            }
            if edge.resolution.target.as_ref() == Some(selected) && edge.source != *selected {
                if edge.resolution.form == Some(LinkForm::Id) {
                    result.incoming_id.push(edge);
                } else {
                    result.incoming_path.push(edge);
                }
            }
        }
        if !scan.complete {
            break;
        }
    }
    result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedLink {
    pub compile_identity: String,
    pub target: String,
    pub projection_generation: String,
    pub occurrence_ordinal: usize,
    pub fragments: Vec<String>,
    pub destination: String,
    /// The final tree proves navigation, but an independent origin producer is
    /// required before any generated occurrence can acquire a source writer.
    pub origin: String,
}
pub fn rendered_links(
    tree: &weftext_engine::RenderNode,
    compile_identity: &str,
    target: &str,
    generation: &str,
) -> Vec<RenderedLink> {
    fn walk(
        node: &weftext_engine::RenderNode,
        identity: &str,
        target: &str,
        generation: &str,
        output: &mut Vec<RenderedLink>,
    ) {
        if node.tag.as_deref() == Some("a")
            && let Some(destination) = node.attrs.get("href")
        {
            output.push(RenderedLink {
                compile_identity: identity.into(),
                target: target.into(),
                projection_generation: generation.into(),
                occurrence_ordinal: output.len(),
                fragments: vec![node.key.clone()],
                destination: destination.clone(),
                origin: "unavailable".into(),
            });
        }
        for child in &node.children {
            walk(child, identity, target, generation, output);
        }
    }
    let mut output = vec![];
    walk(tree, compile_identity, target, generation, &mut output);
    output
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyReferenceInput {
    pub source: NodeRef,
    pub field_id: String,
    pub item_path: Vec<String>,
    pub target: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyReference {
    pub source: NodeRef,
    pub field_id: String,
    pub item_path: Vec<String>,
    pub resolution: LinkResolution,
}
/// Schema-designated references have their own projection. They never acquire
/// fake body-link occurrences, and literal target_or_text branches stay text.
pub fn property_references(
    inventory: &InventorySnapshot,
    inputs: &[PropertyReferenceInput],
) -> Vec<PropertyReference> {
    inputs
        .iter()
        .filter_map(|input| {
            let source = inventory
                .nodes
                .iter()
                .find(|node| node.owner == input.source)?;
            Some(PropertyReference {
                source: input.source.clone(),
                field_id: input.field_id.clone(),
                item_path: input.item_path.clone(),
                resolution: resolve_link(&input.target, source, inventory),
            })
        })
        .collect()
}
