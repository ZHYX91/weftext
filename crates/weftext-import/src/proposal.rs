use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use uuid::Uuid;
use weftext_asciidoc::DiagnosticCode;

use crate::{
    IMPORT_PROPOSAL_CONTRACT_VERSION, ImportDocument, ImportError, ImportErrorCode, ImportLimits,
    ImportNode, ImportNodeKind, ImportPlan, ImportProposal, ImportResource, PortablePath,
    ProposedNode, ProposedResource, ResourcePolicy, SourceArtifact, SplitPolicy, ValidatedProposal,
    sha256_bytes,
};

pub trait CanonicalProposalValidator: Send + Sync {
    /// Deterministically renders IR and validates the exact proposal bytes.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid IR, unsupported split policy, unsafe output,
    /// or a canonical Profile validation failure.
    fn render_and_validate(
        &self,
        source: &SourceArtifact,
        source_bytes: &[u8],
        plan: &ImportPlan,
        document: &ImportDocument,
    ) -> Result<ValidatedProposal, ImportError>;

    /// Validates an already rendered proposal against its source, plan, and IR.
    ///
    /// # Errors
    ///
    /// Returns an error for stale authority, unsafe paths/resources, invalid
    /// canonical source, or any configured limit violation.
    fn validate(
        &self,
        source: &SourceArtifact,
        source_bytes: &[u8],
        plan: &ImportPlan,
        document: &ImportDocument,
        proposal: ImportProposal,
    ) -> Result<ValidatedProposal, ImportError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AsciiDocV1ProposalValidator;

impl CanonicalProposalValidator for AsciiDocV1ProposalValidator {
    // Keep the complete deterministic source/resource serialization sequence
    // together so reviewers can audit that no adapter-specific branch writes
    // canonical bytes.
    #[allow(clippy::too_many_lines)]
    fn render_and_validate(
        &self,
        source: &SourceArtifact,
        source_bytes: &[u8],
        plan: &ImportPlan,
        document: &ImportDocument,
    ) -> Result<ValidatedProposal, ImportError> {
        document.validate(source, plan)?;
        if !matches!(plan.split_policy, SplitPolicy::SingleNode) {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "the provisional foundation renders only the reviewed single-node split policy",
            ));
        }

        let node_id = Uuid::parse_str(&plan.proposed_root_id).map_err(|error| {
            ImportError::new(
                ImportErrorCode::InvalidContract,
                format!("planned root node id is invalid: {error}"),
            )
        })?;
        let document_file = format!("{}.adoc", plan.destination.file_name());
        let proposed_locators = proposed_resource_locators(&document.resources, &document_file)?;
        let resources_by_id: BTreeMap<&str, (&ImportResource, &PortablePath)> = document
            .resources
            .iter()
            .zip(&proposed_locators)
            .map(|(resource, locator)| (resource.id.as_str(), (resource, locator)))
            .collect();
        let mut resource_references = Vec::new();
        let mut exact_asciidoc = String::new();
        writeln!(exact_asciidoc, "---").expect("writing to a String cannot fail");
        writeln!(exact_asciidoc, "weftext:").expect("writing to a String cannot fail");
        writeln!(exact_asciidoc, "  id: \"{node_id}\"").expect("writing to a String cannot fail");
        writeln!(exact_asciidoc, "---").expect("writing to a String cannot fail");
        writeln!(exact_asciidoc, "= {}", inline_text(&document.title))
            .expect("writing to a String cannot fail");
        exact_asciidoc.push('\n');
        render_nodes(
            &document.nodes,
            &resources_by_id,
            plan.resource_policy,
            &mut resource_references,
            &mut exact_asciidoc,
        )?;

        let mut resources = document
            .resources
            .iter()
            .zip(proposed_locators)
            .filter(|_| !matches!(plan.resource_policy, ResourcePolicy::SkipAll))
            .map(|(resource, locator)| {
                let embedded = resource_references.contains(&locator);
                ProposedResource {
                    locator,
                    source_locator: Some(resource.locator.clone()),
                    media_type: resource.media_type.clone(),
                    byte_length: resource.byte_length,
                    sha256: resource.sha256.clone(),
                    bytes: resource.bytes.clone(),
                    embedded,
                }
            })
            .collect::<Vec<_>>();

        if matches!(
            plan.resource_policy,
            ResourcePolicy::ExtractAndRetainOriginal
        ) {
            let locator = original_resource_locator(&source.display_name, &resources)?;
            resources.push(ProposedResource {
                locator,
                source_locator: None,
                media_type: source_media_type(source),
                byte_length: u64::try_from(source_bytes.len()).unwrap_or(u64::MAX),
                sha256: source.sha256.clone(),
                bytes: source_bytes.to_vec(),
                embedded: false,
            });
        }

        let profile_analysis = weftext_asciidoc::analyze(&exact_asciidoc);
        let profile_warnings = profile_analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                !matches!(
                    diagnostic.code,
                    DiagnosticCode::UnclosedFrontmatter
                        | DiagnosticCode::ParserError
                        | DiagnosticCode::UnsupportedProfileSyntax
                )
            })
            .map(|diagnostic| format!("canonical profile: {}", diagnostic.message));

        let node = ProposedNode {
            locator: plan.destination.clone(),
            node_id: node_id.to_string(),
            document_file,
            document_sha256: sha256_bytes(exact_asciidoc.as_bytes()),
            exact_asciidoc,
            resource_references,
            resources,
        };
        let conflicts = document
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == crate::DiagnosticSeverity::Blocking)
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect();
        let warnings = document
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == crate::DiagnosticSeverity::Warning)
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .chain(profile_warnings)
            .collect();
        let omissions = if matches!(plan.resource_policy, ResourcePolicy::SkipAll)
            && !document.resources.is_empty()
        {
            vec!["resource extraction was disabled by the reviewed import plan".to_owned()]
        } else {
            Vec::new()
        };
        let proposal = ImportProposal::create(
            source.sha256.clone(),
            document.revision.clone(),
            plan.destination.clone(),
            vec![node],
            conflicts,
            warnings,
            omissions,
        )?;
        validate_proposal(source, plan, document, proposal)
    }

    fn validate(
        &self,
        source: &SourceArtifact,
        source_bytes: &[u8],
        plan: &ImportPlan,
        document: &ImportDocument,
        proposal: ImportProposal,
    ) -> Result<ValidatedProposal, ImportError> {
        let expected = self.render_and_validate(source, source_bytes, plan, document)?;
        if expected.proposal() != &proposal {
            return invalid_proposal(
                "proposal differs from deterministic IR-to-AsciiDoc rendering",
            );
        }
        Ok(expected)
    }
}

fn validate_proposal(
    source: &SourceArtifact,
    plan: &ImportPlan,
    document: &ImportDocument,
    proposal: ImportProposal,
) -> Result<ValidatedProposal, ImportError> {
    if proposal.contract_version != IMPORT_PROPOSAL_CONTRACT_VERSION
        || proposal.source_digest != source.sha256
        || proposal.base_ir_revision != document.revision
        || proposal.destination != plan.destination
    {
        return invalid_proposal("proposal authority does not match its source, plan, and IR");
    }
    if proposal.nodes.is_empty() {
        return invalid_proposal("a proposal must contain at least one proposed node");
    }
    if proposal.conflicts.len() > 1_000
        || proposal.warnings.len() > 10_000
        || proposal.omissions.len() > 10_000
    {
        return invalid_proposal("proposal diagnostics exceed their bounded contract");
    }

    let expected = ImportProposal::create(
        proposal.source_digest.clone(),
        proposal.base_ir_revision.clone(),
        proposal.destination.clone(),
        proposal.nodes.clone(),
        proposal.conflicts.clone(),
        proposal.warnings.clone(),
        proposal.omissions.clone(),
    )?;
    if expected.proposal_id != proposal.proposal_id {
        return invalid_proposal("proposal id does not match its exact reviewed bytes");
    }

    let mut locators = BTreeSet::new();
    let mut node_ids = BTreeSet::new();
    let mut total_output_bytes = 0_u64;
    for node in &proposal.nodes {
        if !locators.insert(node.locator.as_str()) || !node_ids.insert(node.node_id.as_str()) {
            return invalid_proposal("proposed node locators and identities must be unique");
        }
        validate_node(node, &plan.limits, source, plan, document)?;
        total_output_bytes = checked_add(
            total_output_bytes,
            u64::try_from(node.exact_asciidoc.len()).unwrap_or(u64::MAX),
        )?;
        for resource in &node.resources {
            total_output_bytes = checked_add(total_output_bytes, resource.byte_length)?;
        }
    }
    plan.limits.check(
        "proposal total output bytes",
        total_output_bytes,
        plan.limits.max_total_output_bytes,
    )?;
    let bytes =
        serde_json::to_vec(&proposal).map_err(|error| ImportError::serialization(&error))?;
    Ok(ValidatedProposal::new(proposal, sha256_bytes(&bytes)))
}

fn validate_node(
    node: &ProposedNode,
    limits: &ImportLimits,
    source: &SourceArtifact,
    plan: &ImportPlan,
    document: &ImportDocument,
) -> Result<(), ImportError> {
    let expected_document = format!("{}.adoc", node.locator.file_name());
    if node.document_file != expected_document
        || node.document_file.contains(['/', '\\'])
        || node.document_file == "weftext.annotations.json"
    {
        return invalid_proposal("proposed documents must use the exact X/X.adoc node shape");
    }
    if node.document_sha256 != sha256_bytes(node.exact_asciidoc.as_bytes()) {
        return invalid_proposal("proposed document digest does not match its exact source");
    }
    limits.check(
        "proposed document bytes",
        u64::try_from(node.exact_asciidoc.len()).unwrap_or(u64::MAX),
        limits.max_text_bytes,
    )?;
    let expected_id = Uuid::parse_str(&node.node_id).map_err(|error| {
        ImportError::new(
            ImportErrorCode::InvalidProposal,
            format!("proposed node id is invalid: {error}"),
        )
    })?;
    if expected_id.get_version_num() != 4
        || expected_id.get_variant() != uuid::Variant::RFC4122
        || expected_id.to_string() != node.node_id
    {
        return invalid_proposal("proposed node id must be one lowercase RFC 4122 UUIDv4");
    }
    let exact_envelope = format!("---\nweftext:\n  id: \"{}\"\n---\n", node.node_id);
    if !node.exact_asciidoc.starts_with(&exact_envelope) {
        return invalid_proposal(
            "proposed source must start with the deterministic sole-top-level weftext envelope",
        );
    }
    let analysis = weftext_asciidoc::analyze(&node.exact_asciidoc);
    if analysis.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.code,
            DiagnosticCode::UnclosedFrontmatter
                | DiagnosticCode::ParserError
                | DiagnosticCode::UnsupportedProfileSyntax
        )
    }) {
        return invalid_proposal(
            "proposed source does not reparse as canonical AsciiDoc Profile v1",
        );
    }

    limits.check(
        "proposed resource count",
        u64::try_from(node.resources.len()).unwrap_or(u64::MAX),
        u64::from(limits.max_resource_count),
    )?;
    let mut resources = BTreeMap::new();
    for resource in &node.resources {
        validate_resource(resource, &node.document_file, limits)?;
        if resources
            .insert(resource.locator.as_str(), resource)
            .is_some()
        {
            return invalid_proposal("proposed resource locators must be unique within a node");
        }
    }
    let references = scan_image_targets(&node.exact_asciidoc)?;
    let declared: BTreeSet<&str> = node
        .resource_references
        .iter()
        .map(PortablePath::as_str)
        .collect();
    if references != declared {
        return invalid_proposal(
            "exact AsciiDoc image targets do not match declared proposal resource references",
        );
    }
    for reference in declared {
        let Some(resource) = resources.get(reference) else {
            return invalid_proposal("proposed source refers to a missing resource");
        };
        if !resource.embedded {
            return invalid_proposal("a referenced proposed resource must be marked embedded");
        }
    }
    for (locator, resource) in resources {
        if resource.embedded
            && !node
                .resource_references
                .iter()
                .any(|item| item.as_str() == locator)
        {
            return invalid_proposal("an embedded resource must have one exact source reference");
        }
    }
    validate_resource_binding(node, source, plan, document)?;
    Ok(())
}

fn validate_resource_binding(
    node: &ProposedNode,
    source: &SourceArtifact,
    plan: &ImportPlan,
    document: &ImportDocument,
) -> Result<(), ImportError> {
    if matches!(plan.resource_policy, ResourcePolicy::SkipAll) {
        if !node.resources.is_empty() || !node.resource_references.is_empty() {
            return invalid_proposal("resource-skipping plans cannot contain proposed resources");
        }
        return Ok(());
    }
    let expected = document
        .resources
        .iter()
        .map(|resource| (resource.locator.as_str(), resource))
        .collect::<BTreeMap<_, _>>();
    let mut matched = BTreeSet::new();
    let mut originals = 0_usize;
    for resource in &node.resources {
        if let Some(source_locator) = resource.source_locator.as_ref() {
            let Some(ir_resource) = expected.get(source_locator.as_str()) else {
                return invalid_proposal(
                    "proposal resource source locator is unavailable in validated IR",
                );
            };
            if resource.media_type != ir_resource.media_type
                || resource.byte_length != ir_resource.byte_length
                || resource.sha256 != ir_resource.sha256
                || resource.bytes != ir_resource.bytes
                || !matched.insert(source_locator.as_str())
            {
                return invalid_proposal("proposed extracted resource differs from validated IR");
            }
        } else if matches!(
            plan.resource_policy,
            ResourcePolicy::ExtractAndRetainOriginal
        ) && resource.source_locator.is_none()
            && resource.locator.as_str().starts_with("original-")
            && !resource.embedded
            && resource.byte_length == source.byte_length
            && resource.sha256 == source.sha256
        {
            originals += 1;
        } else {
            return invalid_proposal("proposal contains a resource not authorized by IR or policy");
        }
    }
    if matched.len() != expected.len() {
        return invalid_proposal("proposal omitted one or more validated IR resources");
    }
    let expected_originals = usize::from(matches!(
        plan.resource_policy,
        ResourcePolicy::ExtractAndRetainOriginal
    ));
    if originals != expected_originals {
        return invalid_proposal(
            "original-retention proposal is missing or duplicates source bytes",
        );
    }
    Ok(())
}

fn validate_resource(
    resource: &ProposedResource,
    document_file: &str,
    limits: &ImportLimits,
) -> Result<(), ImportError> {
    if resource.byte_length != u64::try_from(resource.bytes.len()).unwrap_or(u64::MAX)
        || resource.sha256 != sha256_bytes(&resource.bytes)
    {
        return invalid_proposal("proposed resource length or digest does not match its bytes");
    }
    limits.check(
        "proposed resource bytes",
        resource.byte_length,
        limits.max_resource_bytes,
    )?;
    validate_proposed_resource_file_name(resource.locator.as_str(), document_file)?;
    if resource.media_type.is_empty() || !resource.media_type.contains('/') {
        return invalid_proposal("proposed resource media type is invalid");
    }
    Ok(())
}

fn render_nodes(
    nodes: &[ImportNode],
    resources: &BTreeMap<&str, (&ImportResource, &PortablePath)>,
    policy: ResourcePolicy,
    references: &mut Vec<PortablePath>,
    output: &mut String,
) -> Result<(), ImportError> {
    for node in nodes {
        match &node.kind {
            ImportNodeKind::Section {
                level,
                title,
                children,
            } => {
                writeln!(
                    output,
                    "{} {}",
                    "=".repeat(usize::from(*level) + 1),
                    inline_text(title)
                )
                .expect("writing to a String cannot fail");
                output.push('\n');
                render_nodes(children, resources, policy, references, output)?;
            }
            ImportNodeKind::Paragraph { text } => {
                output.push_str(&paragraph_text(text));
                output.push_str("\n\n");
            }
            ImportNodeKind::Quote { depth, text } => render_quote(*depth, text, output),
            ImportNodeKind::Listing { language, source } => {
                render_listing(language.as_deref(), source, output);
            }
            ImportNodeKind::ThematicBreak => output.push_str("'''\n\n"),
            ImportNodeKind::List { ordered, items } => {
                let marker = if *ordered { "." } else { "*" };
                for item in items {
                    writeln!(output, "{marker} {}", inline_text(item))
                        .expect("writing to a String cannot fail");
                }
                output.push('\n');
            }
            ImportNodeKind::Table { header_rows, rows } => {
                if *header_rows > 0 {
                    writeln!(output, "[cols=\"{}*\",options=\"header\"]", rows[0].len())
                        .expect("writing to a String cannot fail");
                }
                output.push_str("|===\n");
                for row in rows {
                    for cell in row {
                        writeln!(output, "|{}", table_cell(cell))
                            .expect("writing to a String cannot fail");
                    }
                    output.push('\n');
                }
                output.push_str("|===\n\n");
            }
            ImportNodeKind::Figure {
                resource_id,
                alt,
                caption,
            } => {
                if matches!(policy, ResourcePolicy::SkipAll) {
                    writeln!(output, "[Figure omitted: {}]\n", inline_text(alt))
                        .expect("writing to a String cannot fail");
                    continue;
                }
                let (_resource, locator) =
                    resources.get(resource_id.as_str()).ok_or_else(|| {
                        ImportError::new(
                            ImportErrorCode::InvalidIr,
                            "figure refers to an unavailable IR resource",
                        )
                    })?;
                if let Some(caption) = caption {
                    writeln!(output, ".{}", inline_text(caption))
                        .expect("writing to a String cannot fail");
                }
                writeln!(output, "image::{}[{}]\n", locator, attribute_text(alt))
                    .expect("writing to a String cannot fail");
                references.push((*locator).clone());
            }
            ImportNodeKind::Formula { notation, source } => {
                writeln!(
                    output,
                    "stem:[{}] // imported notation: {}\n",
                    attribute_text(source),
                    inline_text(notation)
                )
                .expect("writing to a String cannot fail");
            }
            ImportNodeKind::Link { target, label } => {
                writeln!(
                    output,
                    "link:{}[{}]\n",
                    link_target(target),
                    attribute_text(label)
                )
                .expect("writing to a String cannot fail");
            }
        }
    }
    Ok(())
}

fn render_quote(depth: u8, text: &str, output: &mut String) {
    let marker = std::iter::repeat_n(">", usize::from(depth))
        .collect::<Vec<_>>()
        .join(" ");
    for line in text.lines() {
        writeln!(output, "{marker} {}", inline_text(line))
            .expect("writing to a String cannot fail");
    }
    output.push('\n');
}

fn render_listing(language: Option<&str>, source: &str, output: &mut String) {
    if let Some(language) = language {
        writeln!(output, "[source,{language}]").expect("writing to a String cannot fail");
    } else {
        output.push_str("[source]\n");
    }
    let delimiter = listing_delimiter(source);
    writeln!(output, "{delimiter}").expect("writing to a String cannot fail");
    output.push_str(source);
    if !source.ends_with('\n') {
        output.push('\n');
    }
    writeln!(output, "{delimiter}\n").expect("writing to a String cannot fail");
}

fn listing_delimiter(source: &str) -> String {
    let longest = source
        .lines()
        .filter(|line| line.bytes().all(|byte| byte == b'-'))
        .map(str::len)
        .max()
        .unwrap_or(0);
    "-".repeat(longest.saturating_add(1).max(4))
}

fn proposed_resource_locators(
    resources: &[ImportResource],
    document_file: &str,
) -> Result<Vec<PortablePath>, ImportError> {
    let mut used = BTreeSet::new();
    let mut result = Vec::with_capacity(resources.len());
    for resource in resources {
        let leaf = resource.locator.file_name();
        validate_proposed_resource_file_name(leaf, document_file)?;
        let mut candidate = leaf.to_owned();
        if !used.insert(candidate.to_lowercase()) {
            let material = format!("{}\0{}\0{}", resource.id, resource.locator, resource.sha256);
            let digest = sha256_bytes(material.as_bytes());
            let suffix = &digest.as_str()[..12];
            candidate = disambiguated_resource_file_name(leaf, suffix);
            validate_proposed_resource_file_name(&candidate, document_file)?;
            if !used.insert(candidate.to_lowercase()) {
                return invalid_proposal(
                    "resource leaf-name collision could not be resolved deterministically",
                );
            }
        }
        result.push(PortablePath::parse(candidate)?);
    }
    Ok(result)
}

fn disambiguated_resource_file_name(leaf: &str, suffix: &str) -> String {
    let candidate = if let Some((stem, extension)) = leaf.rsplit_once('.') {
        if stem.is_empty() || extension.is_empty() {
            format!("resource-{suffix}.bin")
        } else {
            format!("{stem}-{suffix}.{extension}")
        }
    } else {
        format!("{leaf}-{suffix}")
    };
    if candidate.len() <= 120 {
        candidate
    } else {
        format!("resource-{suffix}.bin")
    }
}

fn validate_proposed_resource_file_name(
    file_name: &str,
    document_file: &str,
) -> Result<(), ImportError> {
    let lower = file_name.to_lowercase();
    if file_name.is_empty()
        || file_name.contains(['/', '\\'])
        || lower == document_file.to_lowercase()
        || lower == ".weftext-format"
        || lower == ".weftext-rules"
        || lower == "weftext.annotations.json"
        || lower == ".git"
        || lower.starts_with(".__weftext-transaction-")
        || lower.starts_with(".__weftext-resource-")
        || Path::new(&lower)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return invalid_proposal(
            "proposed resource leaf name conflicts with canonical workspace storage",
        );
    }
    Ok(())
}

fn original_resource_locator(
    display_name: &str,
    resources: &[ProposedResource],
) -> Result<PortablePath, ImportError> {
    let mut safe = String::with_capacity(display_name.len());
    for character in display_name.chars() {
        if character.is_control()
            || matches!(
                character,
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
            )
        {
            safe.push('_');
        } else {
            safe.push(character);
        }
    }
    let safe = safe.trim_matches([' ', '.']);
    let safe = if safe.is_empty() { "source.bin" } else { safe };
    // A retained Markdown input is evidence, never another managed document. Preserve the
    // recognizable source name while changing the terminal extension so X.md cannot reappear
    // beside canonical X.adoc.
    let markdown_source = Path::new(safe)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
    let suffix = if markdown_source { ".source" } else { "" };
    let mut safe = safe.to_owned();
    truncate_utf8(&mut safe, 120 - "original-source-".len() - suffix.len());
    safe.push_str(suffix);
    let used = resources
        .iter()
        .map(|resource| resource.locator.as_str().to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    for prefix in ["original-", "original-source-"] {
        let candidate = PortablePath::parse(format!("{prefix}{safe}"))?;
        validate_proposed_resource_file_name(candidate.as_str(), "")?;
        if !used.contains(&candidate.as_str().to_ascii_lowercase()) {
            return Ok(candidate);
        }
    }
    invalid_proposal("retained original resource name collides case-insensitively")
}

fn truncate_utf8(value: &mut String, max_bytes: usize) {
    if value.len() <= max_bytes {
        return;
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    while value.ends_with([' ', '.']) {
        value.pop();
    }
    if value.is_empty() {
        value.push_str("source.bin");
    }
}

fn source_media_type(source: &SourceArtifact) -> String {
    match source.detected_format {
        crate::SourceFormat::Pdf => "application/pdf",
        crate::SourceFormat::Image
        | crate::SourceFormat::Unknown
        | crate::SourceFormat::FakeFixture => "application/octet-stream",
        crate::SourceFormat::Html => "text/html",
        crate::SourceFormat::Docx => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        crate::SourceFormat::Odt => "application/vnd.oasis.opendocument.text",
        crate::SourceFormat::Markdown => "text/markdown",
        crate::SourceFormat::Tex => "application/x-tex",
        crate::SourceFormat::Csv => "text/csv",
        crate::SourceFormat::Xlsx => {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        }
        crate::SourceFormat::Ods => "application/vnd.oasis.opendocument.spreadsheet",
        crate::SourceFormat::Epub => "application/epub+zip",
    }
    .to_owned()
}

fn paragraph_text(value: &str) -> String {
    value
        .lines()
        .map(|line| {
            let escaped = inline_text(line);
            if line_starts_active_construct(&escaped) {
                format!("\\{escaped}")
            } else {
                escaped
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn inline_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(['\r', '\n'], " ")
        .replace("image::", "image\\::")
        .replace("include::", "include\\::")
        .replace("ifdef::", "ifdef\\::")
        .replace("ifndef::", "ifndef\\::")
}

fn attribute_text(value: &str) -> String {
    inline_text(value).replace(']', "\\]").replace(',', "\\,")
}

fn table_cell(value: &str) -> String {
    inline_text(value).replace('|', "\\|")
}

fn link_target(value: &str) -> String {
    inline_text(value)
        .replace('[', "%5B")
        .replace(']', "%5D")
        .replace(' ', "%20")
}

fn line_starts_active_construct(value: &str) -> bool {
    let trimmed = value.trim_start();
    trimmed.starts_with('=')
        || trimmed.starts_with("----")
        || trimmed.starts_with("....")
        || trimmed.starts_with("++++")
        || trimmed.starts_with("|===")
}

fn scan_image_targets(source: &str) -> Result<BTreeSet<&str>, ImportError> {
    let mut targets = BTreeSet::new();
    for line in source.lines() {
        let Some(rest) = line.strip_prefix("image::") else {
            continue;
        };
        let Some((target, _attributes)) = rest.split_once('[') else {
            return invalid_proposal("proposed image macro is malformed");
        };
        PortablePath::parse(target.to_owned())?;
        if !targets.insert(target) {
            return invalid_proposal("duplicate exact image references are not supported");
        }
    }
    Ok(targets)
}

fn checked_add(left: u64, right: u64) -> Result<u64, ImportError> {
    left.checked_add(right).ok_or_else(|| {
        ImportError::new(
            ImportErrorCode::LimitExceeded,
            "proposal output byte total overflowed",
        )
    })
}

fn invalid_proposal<T>(message: &str) -> Result<T, ImportError> {
    Err(ImportError::new(ImportErrorCode::InvalidProposal, message))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::{AsciiDocV1ProposalValidator, CanonicalProposalValidator};
    use crate::{
        Confidence, FakeAdapter, ImportAdapter, ImportDocument, ImportErrorCode, ImportLimits,
        ImportNode, ImportNodeKind, ImportProposal, ImportResource, OriginClass, PlanRequest,
        PortablePath, ProposedResource, SourceArtifact, sha256_bytes,
    };

    #[test]
    fn renderer_tracks_exact_cjk_resource_references() {
        let (source, bytes, plan) = source_and_plan();
        let resource_bytes = b"not-a-real-png-but-digest-exact".to_vec();
        let resource = ImportResource {
            id: "figure-1".to_owned(),
            locator: PortablePath::parse("figures/示意图.png").expect("resource path"),
            media_type: "image/png".to_owned(),
            byte_length: u64::try_from(resource_bytes.len()).expect("length"),
            sha256: sha256_bytes(&resource_bytes),
            bytes: resource_bytes,
            source_locations: Vec::new(),
            provenance: Vec::new(),
        };
        let node = ImportNode {
            id: "figure-node-1".to_owned(),
            kind: ImportNodeKind::Figure {
                resource_id: "figure-1".to_owned(),
                alt: "架构示意图".to_owned(),
                caption: Some("导入证据".to_owned()),
            },
            confidence: Confidence::from_basis_points(9_000).expect("confidence"),
            source_locations: Vec::new(),
            provenance: Vec::new(),
        };
        let document = ImportDocument::create(
            "document-resource-test",
            source.sha256.clone(),
            "含资源的文档",
            vec![node],
            vec![resource],
            Vec::new(),
            Vec::new(),
        )
        .expect("IR");

        let proposal = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &bytes, &plan, &document)
            .expect("validated proposal");

        let node = &proposal.proposal().nodes[0];
        assert!(
            node.exact_asciidoc
                .contains("image::示意图.png[架构示意图]")
        );
        assert_eq!(node.resource_references.len(), 1);
        assert!(node.resources[0].embedded);
        assert_eq!(
            node.resources[0]
                .source_locator
                .as_ref()
                .map(PortablePath::as_str),
            Some("figures/示意图.png")
        );
    }

    #[test]
    fn nested_ir_resource_paths_flatten_without_losing_source_provenance() {
        let (source, bytes, plan) = source_and_plan();
        let resources = [
            ("figure-a", "chapter-a/figure.png", b"first".to_vec()),
            ("figure-b", "chapter-b/figure.png", b"second".to_vec()),
        ]
        .into_iter()
        .map(|(id, locator, bytes)| ImportResource {
            id: id.to_owned(),
            locator: PortablePath::parse(locator).expect("IR resource path"),
            media_type: "image/png".to_owned(),
            byte_length: u64::try_from(bytes.len()).expect("length"),
            sha256: sha256_bytes(&bytes),
            bytes,
            source_locations: Vec::new(),
            provenance: Vec::new(),
        })
        .collect::<Vec<_>>();
        let nodes = ["figure-a", "figure-b"]
            .into_iter()
            .map(|resource_id| ImportNode {
                id: format!("node-{resource_id}"),
                kind: ImportNodeKind::Figure {
                    resource_id: resource_id.to_owned(),
                    alt: resource_id.to_owned(),
                    caption: None,
                },
                confidence: Confidence::from_basis_points(9_000).expect("confidence"),
                source_locations: Vec::new(),
                provenance: Vec::new(),
            })
            .collect();
        let document = ImportDocument::create(
            "document-flat-resources",
            source.sha256.clone(),
            "Figures",
            nodes,
            resources,
            Vec::new(),
            Vec::new(),
        )
        .expect("IR");

        let proposal = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &bytes, &plan, &document)
            .expect("proposal");
        let resources = &proposal.proposal().nodes[0].resources;
        assert_eq!(resources.len(), 2);
        assert!(
            resources
                .iter()
                .all(|resource| !resource.locator.as_str().contains('/'))
        );
        assert_ne!(resources[0].locator, resources[1].locator);
        assert_eq!(
            resources
                .iter()
                .filter_map(|resource| resource.source_locator.as_ref())
                .map(PortablePath::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["chapter-a/figure.png", "chapter-b/figure.png"])
        );
    }

    #[test]
    fn reserved_resource_collision_is_rejected() {
        let (source, bytes, plan) = source_and_plan();
        let document = ImportDocument::create(
            "document-reserved-test",
            source.sha256.clone(),
            "Title",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("IR");
        let validated = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &bytes, &plan, &document)
            .expect("initial proposal");
        let old = validated.proposal().clone();
        let mut nodes = old.nodes;
        nodes[0].resources.push(ProposedResource {
            locator: PortablePath::parse("weftext.annotations.json")
                .expect("portable but reserved"),
            source_locator: None,
            media_type: "application/json".to_owned(),
            byte_length: 2,
            sha256: sha256_bytes(b"{}"),
            bytes: b"{}".to_vec(),
            embedded: false,
        });
        let forged = ImportProposal::create(
            old.source_digest,
            old.base_ir_revision,
            old.destination,
            nodes,
            old.conflicts,
            old.warnings,
            old.omissions,
        )
        .expect("internally consistent forged proposal");

        let error = AsciiDocV1ProposalValidator
            .validate(&source, &bytes, &plan, &document, forged)
            .expect_err("reserved collision");
        assert_eq!(error.code(), ImportErrorCode::InvalidProposal);
    }

    #[test]
    fn retained_markdown_source_cannot_recreate_a_managed_markdown_peer() {
        let bytes = b"WEFTEXT-FAKE/1\nTitle\n".to_vec();
        let limits = ImportLimits::default();
        let source =
            SourceArtifact::from_bytes("Imported.MD", OriginClass::TestFixture, &bytes, &limits)
                .expect("source");
        let adapter = FakeAdapter;
        let probe = crate::probe_source_bytes(&adapter, &source, &bytes, &limits).expect("probe");
        let mut request = PlanRequest::single_node(PortablePath::parse("Target").unwrap());
        request.resource_policy = crate::ResourcePolicy::ExtractAndRetainOriginal;
        let plan = adapter
            .plan(&source, &probe, request, limits)
            .expect("plan");
        let document = ImportDocument::create(
            "document-retained-markdown",
            source.sha256.clone(),
            "Title",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("IR");

        let proposal = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &bytes, &plan, &document)
            .expect("retained source proposal");
        let retained = proposal.proposal().nodes[0]
            .resources
            .iter()
            .find(|resource| resource.source_locator.is_none())
            .expect("retained original");
        assert_eq!(retained.locator.as_str(), "original-Imported.MD.source");
        assert_ne!(
            Path::new(retained.locator.as_str())
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("md")
        );
    }

    #[test]
    fn canonical_validation_rejects_recomputed_but_non_deterministic_source() {
        let (source, bytes, plan) = source_and_plan();
        let document = ImportDocument::create(
            "document-exact-render-test",
            source.sha256.clone(),
            "Title",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("IR");
        let validated = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &bytes, &plan, &document)
            .expect("proposal");
        let old = validated.proposal().clone();
        let mut nodes = old.nodes;
        nodes[0].exact_asciidoc.push_str("forged paragraph\n");
        nodes[0].document_sha256 = sha256_bytes(nodes[0].exact_asciidoc.as_bytes());
        let forged = ImportProposal::create(
            old.source_digest,
            old.base_ir_revision,
            old.destination,
            nodes,
            old.conflicts,
            old.warnings,
            old.omissions,
        )
        .expect("self-consistent forged proposal");

        let error = AsciiDocV1ProposalValidator
            .validate(&source, &bytes, &plan, &document, forged)
            .expect_err("non-deterministic source must fail");
        assert_eq!(error.code(), ImportErrorCode::InvalidProposal);
    }

    fn source_and_plan() -> (SourceArtifact, Vec<u8>, crate::ImportPlan) {
        let bytes = b"WEFTEXT-FAKE/1\nTitle\n".to_vec();
        let limits = ImportLimits::default();
        let source =
            SourceArtifact::from_bytes("resource.fake", OriginClass::TestFixture, &bytes, &limits)
                .expect("source");
        let adapter = FakeAdapter;
        let probe = crate::probe_source_bytes(&adapter, &source, &bytes, &limits).expect("probe");
        let plan = adapter
            .plan(
                &source,
                &probe,
                PlanRequest::single_node(PortablePath::parse("Target").expect("path")),
                limits,
            )
            .expect("plan");
        (source, bytes, plan)
    }
}
