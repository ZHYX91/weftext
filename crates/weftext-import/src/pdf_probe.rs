use std::collections::{BTreeMap, BTreeSet};

use crate::probe::probe_source_bytes_with;
use crate::{
    AdapterDescriptor, Confidence, DiagnosticSeverity, EncryptionState,
    FORMAT_PROBE_CONTRACT_VERSION, FormatProbe, ImportDiagnostic, ImportError, ImportErrorCode,
    ImportLimits, ProbeReader, SourceArtifact, SourceFormat,
};

const PDF_HEAD_BYTES: u64 = 1_024;
const PDF_TAIL_BYTES: u64 = 16 * 1_024;
const XREF_WINDOW_BYTES: u64 = 32 * 1_024;
const OBJECT_WINDOW_BYTES: u64 = 8 * 1_024;
const MAX_XREF_REVISIONS: usize = 128;

/// Derives the one reviewed Docling PDF probe solely from exact source bytes,
/// bounded random-access evidence, limits, and an explicit adapter capability.
///
/// No worker, model, installation directory, or mutable process state is
/// consulted. Only a complete classic cross-reference chain whose active
/// objects can all be inspected is authorized. Cross-reference streams,
/// hybrid references, and object streams remain explicitly unsupported and
/// therefore fail closed instead of being declared inert.
///
/// # Errors
///
/// Returns an error only when the common evidence/source contract is broken.
/// Malformed, unsupported, encrypted, active, or budget-incomplete PDFs return
/// a blocking probe with unknown safety state.
pub fn derive_docling_pdf_probe(
    source: &SourceArtifact,
    evidence: &mut ProbeReader<'_>,
    limits: &ImportLimits,
    adapter: AdapterDescriptor,
    capability_available: bool,
    capability_message: &str,
) -> Result<FormatProbe, ImportError> {
    limits.validate()?;
    let inspection = inspect_pdf(evidence, limits);
    let extension_matches = source.extension_hint.as_deref() == Some("pdf");
    let mut mismatch_evidence = Vec::new();
    let signature = inspection.state != PdfInspectionState::MissingSignature;
    let complete = inspection.state == PdfInspectionState::Complete;
    if signature != extension_matches {
        mismatch_evidence.push(format!(
            "PDF signature match is {signature} while .pdf extension match is {extension_matches}"
        ));
    }
    let mut diagnostics = inspection.diagnostics;
    if !capability_available {
        diagnostics.push(blocking("docling_lite_unavailable", capability_message));
    }
    let encryption = if inspection.encrypted {
        EncryptionState::PasswordRequired
    } else if complete {
        EncryptionState::NotEncrypted
    } else {
        EncryptionState::Unknown
    };
    let safe_to_plan = signature
        && complete
        && encryption == EncryptionState::NotEncrypted
        && !inspection.active_content
        && capability_available;
    Ok(FormatProbe {
        contract_version: FORMAT_PROBE_CONTRACT_VERSION.to_owned(),
        adapter,
        source_digest: source.sha256.clone(),
        evidence: evidence.evidence(),
        detected_format: if signature {
            SourceFormat::Pdf
        } else {
            SourceFormat::Unknown
        },
        signature_confidence: Confidence::from_basis_points(if signature { 9_900 } else { 0 })?,
        parser_confidence: Confidence::from_basis_points(if complete { 9_800 } else { 0 })?,
        encryption,
        signature_evidence: inspection.signature_evidence,
        mismatch_evidence,
        active_content_detected: inspection.active_content,
        page_count: inspection.page_count,
        container_entry_count: inspection.object_count,
        safe_to_plan,
        diagnostics,
    })
}

/// Replays the pure PDF safety probe from exact source bytes without requiring
/// a Docling installation or worker. A stored preview uses this boundary to
/// prove that its detected format and safety claims still follow from the
/// bundled source bytes.
///
/// # Errors
///
/// Returns an error for inconsistent source bytes or evidence contracts.
pub fn replay_docling_pdf_probe(
    source: &SourceArtifact,
    source_bytes: &[u8],
    limits: &ImportLimits,
    adapter: AdapterDescriptor,
) -> Result<FormatProbe, ImportError> {
    probe_source_bytes_with(source, source_bytes, limits, |evidence| {
        derive_docling_pdf_probe(
            source,
            evidence,
            limits,
            adapter,
            true,
            "reviewed preview capability",
        )
    })
}

#[derive(Default)]
struct PdfInspection {
    state: PdfInspectionState,
    encrypted: bool,
    active_content: bool,
    page_count: Option<u32>,
    object_count: Option<u32>,
    signature_evidence: Vec<String>,
    diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum PdfInspectionState {
    #[default]
    MissingSignature,
    Incomplete,
    Complete,
}

fn inspect_pdf(evidence: &mut ProbeReader<'_>, limits: &ImportLimits) -> PdfInspection {
    let mut result = PdfInspection::default();
    let head = match evidence.read_head(PDF_HEAD_BYTES.min(evidence.source_byte_length())) {
        Ok(bytes) => bytes,
        Err(error) => {
            result
                .diagnostics
                .push(probe_incomplete(&error.to_string()));
            return result;
        }
    };
    if !valid_pdf_header(&head) {
        result.diagnostics.push(blocking(
            "pdf_signature_missing",
            "exact leading source bytes do not contain a supported PDF header",
        ));
        return result;
    }
    result.state = PdfInspectionState::Incomplete;
    result
        .signature_evidence
        .push("exact leading PDF header".to_owned());

    match inspect_pdf_structure(evidence, limits) {
        Ok(structure) => {
            result.state = PdfInspectionState::Complete;
            result.encrypted = structure.encrypted;
            result.active_content = structure.active_content;
            result.page_count = Some(structure.page_count);
            result.object_count = Some(structure.object_count);
            result.signature_evidence.push(format!(
                "complete classic xref/trailer chain with {} active objects and {} revisions",
                structure.object_count, structure.revision_count
            ));
            if structure.encrypted {
                result.diagnostics.push(blocking(
                    "pdf_password_required",
                    "the complete PDF trailer chain declares encryption; password handling is unavailable",
                ));
            }
            if structure.active_content {
                result.diagnostics.push(blocking(
                    "pdf_active_content_detected",
                    "an active PDF object declares an action or executable-content name",
                ));
            }
        }
        Err(failure) => {
            result.encrypted = failure.encrypted;
            result.active_content = failure.active_content;
            if failure.encrypted {
                result.diagnostics.push(blocking(
                    "pdf_password_required",
                    "the inspected PDF trailer evidence declares encryption; password handling is unavailable",
                ));
            }
            if failure.active_content {
                result.diagnostics.push(blocking(
                    "pdf_active_content_detected",
                    "inspected active PDF objects declare an action or executable-content name",
                ));
            }
            result
                .diagnostics
                .push(blocking(&failure.code, &failure.message));
        }
    }
    result
}

fn valid_pdf_header(bytes: &[u8]) -> bool {
    bytes.len() >= 8
        && bytes.starts_with(b"%PDF-")
        && matches!(bytes.get(5), Some(b'1' | b'2'))
        && bytes.get(6) == Some(&b'.')
        && bytes.get(7).is_some_and(u8::is_ascii_digit)
}

struct StructureAudit {
    encrypted: bool,
    active_content: bool,
    page_count: u32,
    object_count: u32,
    revision_count: usize,
}

#[derive(Debug)]
struct ProbeFailure {
    code: String,
    message: String,
    encrypted: bool,
    active_content: bool,
}

impl ProbeFailure {
    fn malformed(message: impl Into<String>) -> Self {
        Self {
            code: "pdf_structure_unproven".to_owned(),
            message: message.into(),
            encrypted: false,
            active_content: false,
        }
    }

    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "pdf_unsupported_structure".to_owned(),
            message: message.into(),
            encrypted: false,
            active_content: false,
        }
    }

    fn with_observations(mut self, encrypted: bool, active_content: bool) -> Self {
        self.encrypted |= encrypted;
        self.active_content |= active_content;
        self
    }
}

#[derive(Clone, Copy, Debug)]
enum XrefState {
    Free,
    InUse { offset: u64, generation: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ObjectRef {
    object: u32,
    generation: u16,
}

struct XrefSection {
    entries: Vec<(u32, XrefState)>,
    previous: Option<u64>,
    root: Option<ObjectRef>,
    encrypted: bool,
    hybrid_xref: bool,
    size: u32,
}

#[allow(clippy::too_many_lines)]
fn inspect_pdf_structure(
    evidence: &mut ProbeReader<'_>,
    limits: &ImportLimits,
) -> Result<StructureAudit, ProbeFailure> {
    let (start_xref, final_eof) = find_final_startxref(evidence)?;
    if start_xref >= final_eof {
        return Err(ProbeFailure::malformed(
            "the final startxref offset does not precede the final EOF marker",
        ));
    }

    let mut seen_xrefs = BTreeSet::new();
    let mut active_entries: BTreeMap<u32, XrefState> = BTreeMap::new();
    let mut root = None;
    let mut current = Some(start_xref);
    let mut encrypted = false;
    let mut revision_count = 0_usize;
    let mut declared_size = 0_u32;
    while let Some(offset) = current {
        if revision_count >= MAX_XREF_REVISIONS || !seen_xrefs.insert(offset) {
            return Err(ProbeFailure::malformed(
                "the incremental PDF xref chain is cyclic or exceeds its revision limit",
            )
            .with_observations(encrypted, false));
        }
        let section = read_classic_xref(evidence, offset, limits)
            .map_err(|failure| failure.with_observations(encrypted, false))?;
        if section.hybrid_xref {
            return Err(ProbeFailure::unsupported(
                "hybrid/xref-stream PDFs are not yet proven by the Lite probe",
            )
            .with_observations(encrypted || section.encrypted, false));
        }
        encrypted |= section.encrypted;
        declared_size = declared_size.max(section.size);
        if root.is_none() {
            root = section.root;
        }
        for (object, state) in section.entries {
            active_entries.entry(object).or_insert(state);
        }
        if let Some(previous) = section.previous
            && previous >= offset
        {
            return Err(ProbeFailure::malformed(
                "an incremental PDF Prev offset does not point backward",
            )
            .with_observations(encrypted, false));
        }
        current = section.previous;
        revision_count += 1;
    }

    let root = root.ok_or_else(|| {
        ProbeFailure::malformed("the complete PDF trailer chain does not define a Root catalog")
            .with_observations(encrypted, false)
    })?;
    if declared_size == 0 || u64::from(declared_size) > u64::from(limits.max_container_entries) {
        return Err(ProbeFailure::malformed(
            "the PDF trailer Size is zero or exceeds the object-count limit",
        )
        .with_observations(encrypted, false));
    }
    if active_entries.keys().any(|object| *object >= declared_size) {
        return Err(ProbeFailure::malformed(
            "an xref entry falls outside the final declared object space",
        )
        .with_observations(encrypted, false));
    }
    if u32::try_from(active_entries.len()) != Ok(declared_size)
        || active_entries.keys().copied().ne(0_u32..declared_size)
    {
        return Err(ProbeFailure::malformed(
            "the complete xref chain does not account for every declared object number",
        )
        .with_observations(encrypted, false));
    }
    if !matches!(active_entries.get(&0), Some(XrefState::Free)) {
        return Err(ProbeFailure::malformed(
            "the complete PDF xref authority does not keep object zero free",
        )
        .with_observations(encrypted, false));
    }

    let mut active_content = false;
    let mut root_is_catalog = false;
    let mut page_count = 0_u32;
    let mut object_count = 0_u32;
    let mut offsets = BTreeSet::new();
    for (object, state) in &active_entries {
        let XrefState::InUse { offset, generation } = *state else {
            continue;
        };
        if *object == 0 || offset >= start_xref || !offsets.insert(offset) {
            return Err(ProbeFailure::malformed(
                "an active PDF object has an impossible or duplicate xref offset",
            )
            .with_observations(encrypted, active_content));
        }
        let audit = inspect_indirect_object(evidence, *object, generation, offset)
            .map_err(|failure| failure.with_observations(encrypted, active_content))?;
        if audit.kind == ObjectKind::ObjectStream {
            return Err(ProbeFailure::unsupported(
                "PDF object streams are not yet proven by the Lite probe",
            )
            .with_observations(encrypted, active_content || audit.active_content));
        }
        active_content |= audit.active_content;
        page_count = page_count.saturating_add(u32::from(audit.kind == ObjectKind::Page));
        if root.object == *object && root.generation == generation {
            root_is_catalog = audit.kind == ObjectKind::Catalog;
        }
        object_count = object_count.saturating_add(1);
    }
    if !root_is_catalog {
        return Err(ProbeFailure::malformed(
            "the active Root reference does not resolve to an inspected Catalog object",
        )
        .with_observations(encrypted, active_content));
    }
    limits
        .check(
            "PDF active object count",
            u64::from(object_count),
            u64::from(limits.max_container_entries),
        )
        .map_err(|error| {
            ProbeFailure::malformed(error.to_string()).with_observations(encrypted, active_content)
        })?;
    limits
        .check(
            "PDF page count",
            u64::from(page_count),
            u64::from(limits.max_pages),
        )
        .map_err(|error| {
            ProbeFailure::malformed(error.to_string()).with_observations(encrypted, active_content)
        })?;
    Ok(StructureAudit {
        encrypted,
        active_content,
        page_count,
        object_count,
        revision_count,
    })
}

fn find_final_startxref(evidence: &mut ProbeReader<'_>) -> Result<(u64, u64), ProbeFailure> {
    let source_length = evidence.source_byte_length();
    let tail_length = source_length.min(PDF_TAIL_BYTES);
    let tail = read_window(
        evidence,
        source_length.saturating_sub(tail_length),
        tail_length,
    )?;
    let mut end = tail.len();
    while end > 0 && is_whitespace(tail[end - 1]) {
        end -= 1;
    }
    let eof_at = rfind_bytes(&tail[..end], b"%%EOF").ok_or_else(|| {
        ProbeFailure::malformed("the bounded PDF tail does not contain a final EOF marker")
    })?;
    if eof_at + 5 != end {
        return Err(ProbeFailure::malformed(
            "non-whitespace bytes follow the final PDF EOF marker",
        ));
    }
    let start_at = rfind_bytes(&tail[..eof_at], b"startxref").ok_or_else(|| {
        ProbeFailure::malformed("the bounded PDF tail does not contain final startxref evidence")
    })?;
    let mut cursor = start_at + b"startxref".len();
    skip_ascii_space(&tail, &mut cursor);
    let xref = parse_decimal(&tail, &mut cursor)
        .ok_or_else(|| ProbeFailure::malformed("the final startxref offset is malformed"))?;
    skip_ascii_space(&tail, &mut cursor);
    if cursor != eof_at {
        return Err(ProbeFailure::malformed(
            "unexpected bytes occur between the final startxref offset and EOF marker",
        ));
    }
    let base = source_length.saturating_sub(tail_length);
    Ok((xref, base + u64::try_from(eof_at).unwrap_or(u64::MAX)))
}

fn read_classic_xref(
    evidence: &mut ProbeReader<'_>,
    offset: u64,
    limits: &ImportLimits,
) -> Result<XrefSection, ProbeFailure> {
    let bytes = read_window(evidence, offset, XREF_WINDOW_BYTES)?;
    let mut lexer = Lexer::new(&bytes);
    expect_keyword(&mut lexer, b"xref", "xref table")?;
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    loop {
        let token = lexer
            .next_token()?
            .ok_or_else(|| ProbeFailure::malformed("xref table ends before its trailer"))?;
        if token.kind.keyword_eq(b"trailer") {
            break;
        }
        let first = token.kind.as_u64().ok_or_else(|| {
            ProbeFailure::malformed("xref subsection does not begin with an object number")
        })?;
        let count = next_u64(&mut lexer, "xref subsection count")?;
        if count == 0
            || first.checked_add(count).is_none()
            || first + count > u64::from(limits.max_container_entries)
        {
            return Err(ProbeFailure::malformed(
                "xref subsection exceeds the configured object-count limit",
            ));
        }
        for index in 0..count {
            let object_offset = next_u64(&mut lexer, "xref object offset")?;
            let generation = next_u64(&mut lexer, "xref generation")?;
            let state = lexer
                .next_token()?
                .ok_or_else(|| ProbeFailure::malformed("xref entry is incomplete"))?;
            let object = u32::try_from(first + index).map_err(|_| {
                ProbeFailure::malformed("xref object number exceeds the supported range")
            })?;
            if !seen.insert(object) || generation > u64::from(u16::MAX) {
                return Err(ProbeFailure::malformed(
                    "xref table contains a duplicate object or invalid generation",
                ));
            }
            let generation = u16::try_from(generation).map_err(|_| {
                ProbeFailure::malformed("xref generation exceeds the supported range")
            })?;
            let state = if state.kind.keyword_eq(b"n") {
                XrefState::InUse {
                    offset: object_offset,
                    generation,
                }
            } else if state.kind.keyword_eq(b"f") {
                XrefState::Free
            } else {
                return Err(ProbeFailure::malformed(
                    "xref entry has an unknown in-use/free marker",
                ));
            };
            entries.push((object, state));
        }
    }
    let trailer = parse_trailer_dictionary(&mut lexer)?;
    let size = trailer.size.ok_or_else(|| {
        ProbeFailure::malformed("PDF trailer does not contain a direct integer Size")
    })?;
    Ok(XrefSection {
        entries,
        previous: trailer.previous,
        root: trailer.root,
        encrypted: trailer.encrypted,
        hybrid_xref: trailer.hybrid_xref,
        size,
    })
}

struct TrailerAudit {
    size: Option<u32>,
    previous: Option<u64>,
    root: Option<ObjectRef>,
    encrypted: bool,
    hybrid_xref: bool,
}

fn parse_trailer_dictionary(lexer: &mut Lexer<'_>) -> Result<TrailerAudit, ProbeFailure> {
    let start = lexer
        .next_token()?
        .ok_or_else(|| ProbeFailure::malformed("PDF trailer dictionary is absent"))?;
    if !matches!(start.kind, TokenKind::DictStart) {
        return Err(ProbeFailure::malformed(
            "PDF trailer does not begin with a dictionary",
        ));
    }
    let tokens = collect_balanced_tokens(start.kind, lexer, "PDF trailer dictionary")?;
    let PdfValue::Dictionary(dictionary) = parse_single_value(&tokens)? else {
        return Err(ProbeFailure::malformed(
            "PDF trailer does not decode as one exact dictionary",
        ));
    };
    let size = dictionary
        .get(b"Size".as_slice())
        .and_then(PdfValue::as_integer)
        .and_then(|value| u32::try_from(value).ok());
    let previous = dictionary
        .get(b"Prev".as_slice())
        .and_then(PdfValue::as_integer);
    let root = match dictionary.get(b"Root".as_slice()) {
        Some(PdfValue::Reference { object, generation }) => Some(ObjectRef {
            object: u32::try_from(*object)
                .map_err(|_| ProbeFailure::malformed("PDF Root object number is out of range"))?,
            generation: u16::try_from(*generation)
                .map_err(|_| ProbeFailure::malformed("PDF Root generation is out of range"))?,
        }),
        Some(_) => {
            return Err(ProbeFailure::malformed(
                "PDF Root must be one indirect object reference",
            ));
        }
        None => None,
    };
    Ok(TrailerAudit {
        size,
        previous,
        root,
        encrypted: dictionary.contains_key(b"Encrypt".as_slice()),
        hybrid_xref: dictionary.contains_key(b"XRefStm".as_slice()),
    })
}

struct ObjectAudit {
    kind: ObjectKind,
    active_content: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectKind {
    Other,
    Catalog,
    Page,
    ObjectStream,
}

fn inspect_indirect_object(
    evidence: &mut ProbeReader<'_>,
    expected_object: u32,
    expected_generation: u16,
    offset: u64,
) -> Result<ObjectAudit, ProbeFailure> {
    let bytes = read_window(evidence, offset, OBJECT_WINDOW_BYTES)?;
    let mut lexer = Lexer::new(&bytes);
    let object = next_u64(&mut lexer, "indirect object number")?;
    let generation = next_u64(&mut lexer, "indirect object generation")?;
    expect_keyword(&mut lexer, b"obj", "indirect object header")?;
    if object != u64::from(expected_object) || generation != u64::from(expected_generation) {
        return Err(ProbeFailure::malformed(
            "xref offset does not resolve to the declared indirect object",
        ));
    }

    let mut body_tokens = Vec::new();
    let mut depth = 0_usize;
    let mut terminated = None;
    while let Some(token) = lexer.next_token()? {
        if depth == 0 && token.kind.keyword_eq(b"endobj") {
            terminated = Some(false);
            break;
        }
        if depth == 0 && token.kind.keyword_eq(b"stream") {
            terminated = Some(true);
            break;
        }
        match token.kind {
            TokenKind::DictStart | TokenKind::ArrayStart => depth = depth.saturating_add(1),
            TokenKind::DictEnd | TokenKind::ArrayEnd => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    ProbeFailure::malformed("an indirect object closes an unopened container")
                })?;
            }
            _ => {}
        }
        body_tokens.push(token.kind);
    }
    let Some(is_stream) = terminated else {
        return Err(ProbeFailure::malformed(
            "an active indirect object exceeds its bounded structural window",
        ));
    };
    if depth != 0 || body_tokens.is_empty() {
        return Err(ProbeFailure::malformed(
            "an active indirect object has incomplete container syntax",
        ));
    }
    let value = parse_single_value(&body_tokens)?;
    if is_stream {
        let PdfValue::Dictionary(dictionary) = &value else {
            return Err(ProbeFailure::malformed(
                "a PDF stream object does not begin with one exact dictionary",
            ));
        };
        validate_stream_termination(evidence, offset, &bytes, lexer.cursor, dictionary)?;
    }
    let type_value = value
        .as_dictionary()
        .and_then(|dictionary| dictionary.get(b"Type".as_slice()))
        .and_then(PdfValue::as_name);
    let kind = match type_value {
        Some(value) if value == b"Catalog" => ObjectKind::Catalog,
        Some(value) if value == b"Page" => ObjectKind::Page,
        Some(value) if value == b"ObjStm" => ObjectKind::ObjectStream,
        _ => ObjectKind::Other,
    };
    Ok(ObjectAudit {
        kind,
        active_content: value.has_active_content(),
    })
}

fn validate_stream_termination(
    evidence: &mut ProbeReader<'_>,
    object_offset: u64,
    object_window: &[u8],
    stream_keyword_end: usize,
    dictionary: &BTreeMap<Vec<u8>, PdfValue>,
) -> Result<(), ProbeFailure> {
    let length = dictionary
        .get(b"Length".as_slice())
        .and_then(PdfValue::as_integer)
        .ok_or_else(|| {
            ProbeFailure::unsupported(
                "a PDF stream Length is absent or indirect and cannot be proven by the Lite probe",
            )
        })?;
    let data_start = match object_window.get(stream_keyword_end..) {
        Some(bytes) if bytes.starts_with(b"\r\n") => stream_keyword_end.saturating_add(2),
        Some(bytes) if bytes.starts_with(b"\r") || bytes.starts_with(b"\n") => {
            stream_keyword_end.saturating_add(1)
        }
        _ => {
            return Err(ProbeFailure::malformed(
                "a PDF stream keyword is not followed by an end-of-line marker",
            ));
        }
    };
    let data_end = object_offset
        .checked_add(u64::try_from(data_start).unwrap_or(u64::MAX))
        .and_then(|start| start.checked_add(length))
        .ok_or_else(|| ProbeFailure::malformed("a PDF stream byte range overflowed"))?;
    let suffix = read_window(evidence, data_end, 256)?;
    let mut lexer = Lexer::new(&suffix);
    expect_keyword(&mut lexer, b"endstream", "stream terminator")?;
    expect_keyword(&mut lexer, b"endobj", "stream object terminator")?;
    Ok(())
}

#[derive(Clone, Debug)]
enum PdfValue {
    Integer(u64),
    Reference { object: u64, generation: u64 },
    Name(Vec<u8>),
    Dictionary(BTreeMap<Vec<u8>, PdfValue>),
    Array(Vec<PdfValue>),
    Scalar,
}

impl PdfValue {
    fn as_integer(&self) -> Option<u64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    fn as_name(&self) -> Option<&[u8]> {
        match self {
            Self::Name(value) => Some(value),
            _ => None,
        }
    }

    fn as_dictionary(&self) -> Option<&BTreeMap<Vec<u8>, Self>> {
        match self {
            Self::Dictionary(value) => Some(value),
            _ => None,
        }
    }

    fn has_active_content(&self) -> bool {
        match self {
            Self::Name(name) => active_name(name),
            Self::Dictionary(entries) => entries
                .iter()
                .any(|(name, value)| active_name(name) || value.has_active_content()),
            Self::Array(values) => values.iter().any(Self::has_active_content),
            Self::Integer(_) | Self::Reference { .. } | Self::Scalar => false,
        }
    }
}

fn collect_balanced_tokens(
    first: TokenKind,
    lexer: &mut Lexer<'_>,
    label: &str,
) -> Result<Vec<TokenKind>, ProbeFailure> {
    let mut tokens = vec![first];
    let mut depth = 1_usize;
    while depth > 0 {
        let token = lexer
            .next_token()?
            .ok_or_else(|| ProbeFailure::malformed(format!("{label} is unterminated")))?;
        match token.kind {
            TokenKind::DictStart | TokenKind::ArrayStart => depth = depth.saturating_add(1),
            TokenKind::DictEnd | TokenKind::ArrayEnd => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    ProbeFailure::malformed(format!("{label} has mismatched delimiters"))
                })?;
            }
            _ => {}
        }
        tokens.push(token.kind);
    }
    Ok(tokens)
}

fn parse_single_value(tokens: &[TokenKind]) -> Result<PdfValue, ProbeFailure> {
    let mut cursor = 0_usize;
    let value = parse_pdf_value(tokens, &mut cursor, 0)?;
    if cursor != tokens.len() {
        return Err(ProbeFailure::malformed(
            "a PDF object contains bytes outside its single direct value",
        ));
    }
    Ok(value)
}

fn parse_pdf_value(
    tokens: &[TokenKind],
    cursor: &mut usize,
    depth: usize,
) -> Result<PdfValue, ProbeFailure> {
    if depth > 64 {
        return Err(ProbeFailure::malformed(
            "PDF direct object nesting exceeds the reviewed depth",
        ));
    }
    let token = tokens
        .get(*cursor)
        .ok_or_else(|| ProbeFailure::malformed("a PDF direct object value is absent"))?;
    *cursor += 1;
    match token {
        TokenKind::Integer(object) => {
            let generation = tokens.get(*cursor).and_then(TokenKind::as_u64);
            let reference = tokens
                .get(cursor.saturating_add(1))
                .is_some_and(|token| token.keyword_eq(b"R"));
            if let (Some(generation), true) = (generation, reference) {
                *cursor += 2;
                Ok(PdfValue::Reference {
                    object: *object,
                    generation,
                })
            } else {
                Ok(PdfValue::Integer(*object))
            }
        }
        TokenKind::Name(name) => Ok(PdfValue::Name(name.clone())),
        TokenKind::DictStart => {
            let mut values = BTreeMap::new();
            loop {
                let key = match tokens.get(*cursor) {
                    Some(TokenKind::DictEnd) => {
                        *cursor += 1;
                        break;
                    }
                    Some(TokenKind::Name(name)) => name.clone(),
                    _ => {
                        return Err(ProbeFailure::malformed(
                            "a PDF dictionary contains a non-name key",
                        ));
                    }
                };
                *cursor += 1;
                let value = parse_pdf_value(tokens, cursor, depth + 1)?;
                if values.insert(key, value).is_some() {
                    return Err(ProbeFailure::malformed(
                        "a PDF dictionary contains a duplicate key",
                    ));
                }
            }
            Ok(PdfValue::Dictionary(values))
        }
        TokenKind::ArrayStart => {
            let mut values = Vec::new();
            loop {
                if matches!(tokens.get(*cursor), Some(TokenKind::ArrayEnd)) {
                    *cursor += 1;
                    break;
                }
                values.push(parse_pdf_value(tokens, cursor, depth + 1)?);
            }
            Ok(PdfValue::Array(values))
        }
        TokenKind::DictEnd | TokenKind::ArrayEnd => Err(ProbeFailure::malformed(
            "a PDF direct object closes an unopened container",
        )),
        TokenKind::Keyword(_) | TokenKind::Literal => Ok(PdfValue::Scalar),
    }
}

fn active_name(name: &[u8]) -> bool {
    matches!(
        name,
        b"OpenAction"
            | b"AA"
            | b"JavaScript"
            | b"JS"
            | b"Launch"
            | b"SubmitForm"
            | b"ImportData"
            | b"RichMedia"
            | b"GoToR"
            | b"Movie"
            | b"Sound"
    )
}

fn read_window(
    evidence: &mut ProbeReader<'_>,
    offset: u64,
    desired: u64,
) -> Result<Vec<u8>, ProbeFailure> {
    match evidence.read_up_to(offset, desired) {
        Ok(bytes) => Ok(bytes),
        Err(error) if error.code() == ImportErrorCode::LimitExceeded => {
            let remaining = evidence.remaining_budget();
            if remaining == 0 {
                Err(ProbeFailure::malformed(
                    "the PDF structure exceeds the bounded random-access probe budget",
                ))
            } else {
                evidence.read_up_to(offset, remaining).map_err(|retry| {
                    ProbeFailure::malformed(format!(
                        "the PDF structure cannot be read within the bounded probe budget: {retry}"
                    ))
                })
            }
        }
        Err(error) => Err(ProbeFailure::malformed(error.to_string())),
    }
}

#[derive(Clone, Debug)]
enum TokenKind {
    Integer(u64),
    Name(Vec<u8>),
    Keyword(Vec<u8>),
    DictStart,
    DictEnd,
    ArrayStart,
    ArrayEnd,
    Literal,
}

impl TokenKind {
    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    fn keyword_eq(&self, expected: &[u8]) -> bool {
        matches!(self, Self::Keyword(value) if value == expected)
    }
}

struct Token {
    kind: TokenKind,
}

struct Lexer<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Lexer<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn next_token(&mut self) -> Result<Option<Token>, ProbeFailure> {
        self.skip_space_and_comments();
        let Some(&byte) = self.bytes.get(self.cursor) else {
            return Ok(None);
        };
        let kind = match byte {
            b'<' if self.bytes.get(self.cursor + 1) == Some(&b'<') => {
                self.cursor += 2;
                TokenKind::DictStart
            }
            b'>' if self.bytes.get(self.cursor + 1) == Some(&b'>') => {
                self.cursor += 2;
                TokenKind::DictEnd
            }
            b'[' => {
                self.cursor += 1;
                TokenKind::ArrayStart
            }
            b']' => {
                self.cursor += 1;
                TokenKind::ArrayEnd
            }
            b'/' => TokenKind::Name(self.read_name()?),
            b'(' => {
                self.skip_literal_string()?;
                TokenKind::Literal
            }
            b'<' => {
                self.skip_hex_string()?;
                TokenKind::Literal
            }
            b'+' | b'-' | b'0'..=b'9' => self.read_number_or_keyword()?,
            _ => TokenKind::Keyword(self.read_keyword()?),
        };
        Ok(Some(Token { kind }))
    }

    fn skip_space_and_comments(&mut self) {
        loop {
            while self
                .bytes
                .get(self.cursor)
                .is_some_and(|byte| is_whitespace(*byte))
            {
                self.cursor += 1;
            }
            if self.bytes.get(self.cursor) != Some(&b'%') {
                break;
            }
            while self
                .bytes
                .get(self.cursor)
                .is_some_and(|byte| !matches!(*byte, b'\r' | b'\n'))
            {
                self.cursor += 1;
            }
        }
    }

    fn read_name(&mut self) -> Result<Vec<u8>, ProbeFailure> {
        self.cursor += 1;
        let mut decoded = Vec::new();
        while let Some(&byte) = self.bytes.get(self.cursor) {
            if is_delimiter_or_space(byte) {
                break;
            }
            if byte == b'#' {
                let high = self
                    .bytes
                    .get(self.cursor + 1)
                    .and_then(|value| hex(*value));
                let low = self
                    .bytes
                    .get(self.cursor + 2)
                    .and_then(|value| hex(*value));
                let (Some(high), Some(low)) = (high, low) else {
                    return Err(ProbeFailure::malformed(
                        "a PDF name contains an invalid hexadecimal escape",
                    ));
                };
                decoded.push(high * 16 + low);
                self.cursor += 3;
            } else {
                decoded.push(byte);
                self.cursor += 1;
            }
        }
        if decoded.is_empty() {
            return Err(ProbeFailure::malformed("a PDF name is empty"));
        }
        Ok(decoded)
    }

    fn skip_literal_string(&mut self) -> Result<(), ProbeFailure> {
        self.cursor += 1;
        let mut depth = 1_usize;
        while let Some(&byte) = self.bytes.get(self.cursor) {
            self.cursor += 1;
            if byte == b'\\' {
                if self.cursor < self.bytes.len() {
                    self.cursor += 1;
                }
            } else if byte == b'(' {
                depth = depth.saturating_add(1);
            } else if byte == b')' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(());
                }
            }
        }
        Err(ProbeFailure::malformed(
            "a PDF literal string is unterminated inside inspected structure",
        ))
    }

    fn skip_hex_string(&mut self) -> Result<(), ProbeFailure> {
        self.cursor += 1;
        while let Some(&byte) = self.bytes.get(self.cursor) {
            self.cursor += 1;
            if byte == b'>' {
                return Ok(());
            }
            if !is_whitespace(byte) && hex(byte).is_none() {
                return Err(ProbeFailure::malformed(
                    "a PDF hexadecimal string contains an invalid byte",
                ));
            }
        }
        Err(ProbeFailure::malformed(
            "a PDF hexadecimal string is unterminated inside inspected structure",
        ))
    }

    fn read_number_or_keyword(&mut self) -> Result<TokenKind, ProbeFailure> {
        let start = self.cursor;
        if matches!(self.bytes.get(self.cursor), Some(b'+' | b'-')) {
            self.cursor += 1;
        }
        while self.bytes.get(self.cursor).is_some_and(u8::is_ascii_digit) {
            self.cursor += 1;
        }
        let bytes = &self.bytes[start..self.cursor];
        if bytes.first() == Some(&b'-') {
            return Ok(TokenKind::Keyword(bytes.to_vec()));
        }
        let digits = bytes.strip_prefix(b"+").unwrap_or(bytes);
        if digits.is_empty() {
            return Ok(TokenKind::Keyword(bytes.to_vec()));
        }
        let text = std::str::from_utf8(digits)
            .map_err(|_| ProbeFailure::malformed("a PDF integer is not ASCII"))?;
        let value = text
            .parse::<u64>()
            .map_err(|_| ProbeFailure::malformed("a PDF integer is out of range"))?;
        Ok(TokenKind::Integer(value))
    }

    fn read_keyword(&mut self) -> Result<Vec<u8>, ProbeFailure> {
        let start = self.cursor;
        while self
            .bytes
            .get(self.cursor)
            .is_some_and(|byte| !is_delimiter_or_space(*byte))
        {
            self.cursor += 1;
        }
        if start == self.cursor {
            return Err(ProbeFailure::malformed(
                "an unexpected PDF delimiter occurs in inspected structure",
            ));
        }
        Ok(self.bytes[start..self.cursor].to_vec())
    }
}

fn next_u64(lexer: &mut Lexer<'_>, label: &str) -> Result<u64, ProbeFailure> {
    lexer
        .next_token()?
        .and_then(|token| token.kind.as_u64())
        .ok_or_else(|| ProbeFailure::malformed(format!("{label} is not a direct integer")))
}

fn expect_keyword(lexer: &mut Lexer<'_>, expected: &[u8], label: &str) -> Result<(), ProbeFailure> {
    let matches = lexer
        .next_token()?
        .is_some_and(|token| token.kind.keyword_eq(expected));
    if matches {
        Ok(())
    } else {
        let kind = if expected == b"xref" {
            "a cross-reference stream, hybrid table, or malformed offset"
        } else {
            "malformed syntax"
        };
        Err(ProbeFailure::unsupported(format!(
            "{label} is not the reviewed classic form ({kind})"
        )))
    }
}

fn parse_decimal(bytes: &[u8], cursor: &mut usize) -> Option<u64> {
    let start = *cursor;
    while bytes.get(*cursor).is_some_and(u8::is_ascii_digit) {
        *cursor += 1;
    }
    if start == *cursor {
        return None;
    }
    std::str::from_utf8(&bytes[start..*cursor])
        .ok()?
        .parse()
        .ok()
}

fn skip_ascii_space(bytes: &[u8], cursor: &mut usize) {
    while bytes.get(*cursor).is_some_and(|byte| is_whitespace(*byte)) {
        *cursor += 1;
    }
}

fn rfind_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

fn is_whitespace(byte: u8) -> bool {
    matches!(byte, 0 | b'\t' | b'\n' | 0x0c | b'\r' | b' ')
}

fn is_delimiter_or_space(byte: u8) -> bool {
    is_whitespace(byte)
        || matches!(
            byte,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn blocking(code: &str, message: &str) -> ImportDiagnostic {
    ImportDiagnostic {
        code: code.to_owned(),
        severity: DiagnosticSeverity::Blocking,
        message: message.to_owned(),
        source_location: None,
        ir_node_id: None,
    }
}

fn probe_incomplete(message: &str) -> ImportDiagnostic {
    blocking(
        "pdf_probe_budget_incomplete",
        &format!("bounded PDF evidence could not be completed: {message}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ImportAdapter, ImportDocument, ImportPlan, OriginClass, PlanRequest, PortablePath,
        SourceArtifact, WorkerRequest, WorkerResponse, probe_source_bytes,
    };

    #[derive(Clone, Copy)]
    struct PdfProbeAdapter;

    impl ImportAdapter for PdfProbeAdapter {
        fn descriptor(&self) -> AdapterDescriptor {
            AdapterDescriptor {
                adapter_id: "test.pdf-probe".to_owned(),
                adapter_version: "1".to_owned(),
                supported_format: SourceFormat::Pdf,
            }
        }

        fn probe(
            &self,
            source: &SourceArtifact,
            evidence: &mut ProbeReader<'_>,
            limits: &ImportLimits,
        ) -> Result<FormatProbe, ImportError> {
            derive_docling_pdf_probe(
                source,
                evidence,
                limits,
                self.descriptor(),
                true,
                "available",
            )
        }

        fn plan(
            &self,
            _source: &SourceArtifact,
            _probe: &FormatProbe,
            _request: PlanRequest,
            _limits: ImportLimits,
        ) -> Result<ImportPlan, ImportError> {
            Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "test probe has no worker plan",
            ))
        }

        fn worker_request(
            &self,
            _source: &SourceArtifact,
            _plan: &ImportPlan,
            _source_locator: PortablePath,
        ) -> Result<WorkerRequest, ImportError> {
            unreachable!()
        }

        fn map_worker_response(
            &self,
            _source: &SourceArtifact,
            _plan: &ImportPlan,
            _response: WorkerResponse,
        ) -> Result<ImportDocument, ImportError> {
            unreachable!()
        }
    }

    fn probe(bytes: &[u8]) -> FormatProbe {
        let limits = ImportLimits::default();
        let source =
            SourceArtifact::from_bytes("fixture.pdf", OriginClass::TestFixture, bytes, &limits)
                .expect("source");
        probe_source_bytes(&PdfProbeAdapter, &source, bytes, &limits).expect("probe")
    }

    fn classic_pdf(objects: &[(u32, &str)], trailer_extra: &str) -> Vec<u8> {
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let max_object = objects.iter().map(|(object, _)| *object).max().unwrap_or(1);
        let mut offsets = BTreeMap::new();
        for (object, body) in objects {
            offsets.insert(*object, bytes.len());
            bytes.extend_from_slice(format!("{object} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        let xref = bytes.len();
        bytes.extend_from_slice(format!("xref\n0 {}\n", max_object + 1).as_bytes());
        bytes.extend_from_slice(b"0000000000 65535 f \n");
        for object in 1..=max_object {
            if let Some(offset) = offsets.get(&object) {
                bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
            } else {
                bytes.extend_from_slice(b"0000000000 00000 f \n");
            }
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R {trailer_extra} >>\nstartxref\n{xref}\n%%EOF\n",
                max_object + 1
            )
            .as_bytes(),
        );
        bytes
    }

    #[test]
    fn complete_classic_pdf_is_the_only_safe_state() {
        let bytes = classic_pdf(&[(1, "<< /Type /Catalog >>")], "");
        let result = probe(&bytes);
        assert!(result.safe_to_plan, "{:?}", result.diagnostics);
        assert_eq!(result.encryption, EncryptionState::NotEncrypted);
        assert!(!result.active_content_detected);
        assert!(!result.evidence.segments.is_empty());
    }

    #[test]
    fn encryption_in_the_tail_trailer_is_never_missed_by_a_prefix_probe() {
        let padding = format!(
            "<< /Length 70000 >>\nstream\n{}\nendstream",
            "x".repeat(70_000)
        );
        let bytes = classic_pdf(
            &[(1, "<< /Type /Catalog >>"), (2, padding.as_str())],
            "/Encrypt 9 0 R",
        );
        let result = probe(&bytes);
        assert_eq!(result.encryption, EncryptionState::PasswordRequired);
        assert!(!result.safe_to_plan);
        assert!(
            result
                .evidence
                .segments
                .iter()
                .any(|segment| segment.offset > 0)
        );
    }

    #[test]
    fn late_indirect_actions_and_encoded_names_are_blocking() {
        for action in [
            "<< /Type /Catalog /OpenAction 2 0 R >>",
            "<< /Type /Catalog /AA 2 0 R >>",
            "<< /Type /Catalog /OpenAction 2 0 R >>",
        ] {
            let bytes = classic_pdf(
                &[
                    (1, action),
                    (2, "<< /S /Java#53cript /JS (app.alert\\(1\\)) >>"),
                ],
                "",
            );
            let result = probe(&bytes);
            assert!(result.active_content_detected);
            assert!(!result.safe_to_plan);
        }
        let launch = classic_pdf(
            &[
                (1, "<< /Type /Catalog >>"),
                (2, "<< /S /Launch /F (tool.exe) >>"),
            ],
            "",
        );
        assert!(probe(&launch).active_content_detected);
    }

    #[test]
    fn incremental_prev_chain_uses_the_latest_active_object_revision() {
        let mut bytes = classic_pdf(&[(1, "<< /Type /Catalog >>")], "");
        let previous_xref = find_last_decimal_after(&bytes, b"startxref\n");
        let new_catalog = bytes.len();
        bytes.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /OpenAction 2 0 R >>\nendobj\n");
        let action = bytes.len();
        bytes.extend_from_slice(b"2 0 obj\n<< /S /JavaScript /JS (late) >>\nendobj\n");
        let xref = bytes.len();
        bytes.extend_from_slice(
            format!(
                "xref\n1 2\n{new_catalog:010} 00000 n \n{action:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R /Prev {previous_xref} >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        let result = probe(&bytes);
        assert!(result.active_content_detected);
        assert!(!result.safe_to_plan);
        assert!(
            result
                .signature_evidence
                .iter()
                .any(|item| item.contains("2 revisions"))
        );
    }

    #[test]
    fn xref_and_object_streams_remain_unknown_and_fail_closed() {
        let object_stream = classic_pdf(
            &[
                (1, "<< /Type /Catalog >>"),
                (
                    2,
                    "<< /Type /ObjStm /N 1 /First 4 /Length 4 >>\nstream\nnoop\nendstream",
                ),
            ],
            "",
        );
        let result = probe(&object_stream);
        assert_eq!(result.encryption, EncryptionState::Unknown);
        assert!(!result.safe_to_plan);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "pdf_unsupported_structure")
        );

        let mut xref_stream = b"%PDF-1.7\n".to_vec();
        let offset = xref_stream.len();
        xref_stream.extend_from_slice(
            b"1 0 obj\n<< /Type /XRef /Size 2 /Root 2 0 R /Length 0 >>\nstream\n\nendstream\nendobj\n",
        );
        xref_stream.extend_from_slice(format!("startxref\n{offset}\n%%EOF\n").as_bytes());
        let result = probe(&xref_stream);
        assert_eq!(result.encryption, EncryptionState::Unknown);
        assert!(!result.safe_to_plan);
    }

    #[test]
    fn malformed_stream_tail_cannot_hide_late_action_syntax() {
        let bytes = classic_pdf(
            &[
                (1, "<< /Type /Catalog >>"),
                (
                    2,
                    "<< /Length 0 >>\nstream\n\nendstream /OpenAction 3 0 R\nendobj",
                ),
            ],
            "",
        );
        let result = probe(&bytes);
        assert_eq!(result.encryption, EncryptionState::Unknown);
        assert!(!result.safe_to_plan);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "pdf_unsupported_structure")
        );
    }

    fn find_last_decimal_after(bytes: &[u8], marker: &[u8]) -> u64 {
        let at = rfind_bytes(bytes, marker).expect("marker") + marker.len();
        let mut cursor = at;
        parse_decimal(bytes, &mut cursor).expect("decimal")
    }
}
