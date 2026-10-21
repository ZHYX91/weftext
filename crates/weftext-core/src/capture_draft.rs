//! Private, not-yet-created author input. The operation journal alone owns effects.
//! Raw text uses UTF-16 code units, so an unfinished JS string is not silently lost.
use super::*;
use hosted::ActorLibraryBinding;
use std::collections::BTreeSet;

pub const MAX_CAPTURE_RECORD_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CAPTURE_DOMAIN_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_CAPTURE_RECORDS: usize = 128;
const MAX_CAPTURE_HEADER_BYTES: usize = 2048;
const MAX_CAPTURE_PREFIX_BYTES: usize = 4096;
const MAX_CAPTURE_WIRE_BYTES: usize = 3 * 1024 * 1024;
const MAX_CAPTURE_TEXT_UNITS: usize = 2 * 1024 * 1024;
const MAX_CAPTURE_REVISION: u64 = 9_007_199_254_740_991;
pub(crate) const CAPTURE_DIRECTORY: &str = "capture-drafts";
pub(crate) const CAPTURE_STAGING_DIRECTORY: &str = "capture-staging";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureText {
    /// Exact JS UTF-16, including unpaired surrogates. No lossy UTF-8 conversion.
    pub utf16: Vec<u16>,
}
impl CaptureText {
    pub fn from_text(text: &str) -> Self {
        Self {
            utf16: text.encode_utf16().collect(),
        }
    }
    pub fn utf8(&self) -> Result<String> {
        String::from_utf16(&self.utf16).map_err(|_| CoreError::new(
            "captureTextIncomplete", "Unpaired UTF-16 input is retained; complete it before creating UTF-8 author source",
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureRawInput {
    pub field_id: String,
    pub value_path: Vec<weftext_data::ValuePath>,
    pub control_role: String,
    pub text: CaptureText,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFieldInput {
    pub field_id: String,
    /// Optional validated candidate. Every author's control raw is retained separately.
    pub value: Option<weftext_data::NativeValue>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureBodyMode {
    Text,
    #[default]
    Markup,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFormBasis {
    pub schema_ref: String,
    pub semantic_digest: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureInput {
    pub name: CaptureText,
    pub title: CaptureText,
    pub body: CaptureText,
    pub body_mode: CaptureBodyMode,
    pub parent_input: CaptureText,
    pub parameters_text: CaptureText,
    pub template_id: Option<String>,
    pub type_ids: Vec<String>,
    pub fields: Vec<CaptureFieldInput>,
    pub raw_inputs: Vec<CaptureRawInput>,
    pub form_basis: Option<CaptureFormBasis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum CaptureOwner {
    Local,
    Actor { actor: String },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum CaptureLibrary {
    DeviceUnassigned,
    Bound { binding: ActorLibraryBinding },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CaptureScopeHeader {
    pub owner: CaptureOwner,
    pub library: CaptureLibrary,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CaptureParent {
    LibraryRoot,
    Managed {
        reference: String,
        path_hint: String,
        identity_digest: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureTarget {
    pub root_hint: String,
    pub parent: CaptureParent,
    /// Server-owned approved slot. This identifier is never a grant.
    pub hosted_slot: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenCaptureRequest {
    pub operation_id: String,
    pub request_kind: String,
    /// The original JSON.stringify wire, not a parsed Value or regenerated input.
    pub wire: String,
    pub wire_digest: String,
    pub input_revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureMutation {
    pub mutation_id: String,
    pub fingerprint: String,
    pub resulting_revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureDraft {
    // Keep this first: backup routing must never parse a foreign author body.
    pub(crate) scope: CaptureScopeHeader,
    pub version: u32,
    pub draft_id: String,
    pub revision: u64,
    pub input_revision: u64,
    pub last_mutation: CaptureMutation,
    pub target: Option<CaptureTarget>,
    pub current_input: CaptureInput,
    pub pending: Option<FrozenCaptureRequest>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMutationLookup {
    /// A negative lookup does not prove a superseded mutation was never applied.
    pub status: &'static str,
    pub draft: CaptureDraft,
}
/// Original mutation evidence for a read-only lost-response query. A mutation ID
/// alone cannot prove that the stored command is the one this caller submitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CaptureMutationCommand {
    Create {
        input: CaptureInput,
        target: Option<CaptureMutationTarget>,
    },
    Input {
        expected_revision: u64,
        input: CaptureInput,
    },
    Target {
        expected_revision: u64,
        target: CaptureMutationTarget,
    },
    Freeze {
        expected_revision: u64,
        request: FrozenCaptureRequest,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureMutationTarget {
    pub root: String,
    pub parent: String,
    pub hosted_slot: Option<String>,
}
impl CaptureMutationCommand {
    fn fingerprint(&self) -> Result<String> {
        match self {
            Self::Create { input, .. } | Self::Input { input, .. } => input_valid(input)?,
            Self::Freeze { request, .. } => pending_valid(request, request.input_revision)?,
            _ => {}
        }
        match self {
            Self::Create {
                input,
                target: None,
            } => capture_fingerprint(&(0u64, "create", input)),
            Self::Create {
                input,
                target: Some(target),
            } => capture_fingerprint(&(
                0u64,
                "createTargeted",
                input,
                &target.root,
                &target.parent,
                &target.hosted_slot,
            )),
            Self::Input {
                expected_revision,
                input,
            } => capture_fingerprint(&(expected_revision, "input", input)),
            Self::Target {
                expected_revision,
                target,
            } => capture_fingerprint(&(
                expected_revision,
                "target",
                &target.root,
                &target.parent,
                &target.hosted_slot,
            )),
            Self::Freeze {
                expected_revision,
                request,
            } => capture_fingerprint(&(expected_revision, "freeze", request)),
        }
    }
}
fn capture_fingerprint(command: &impl Serialize) -> Result<String> {
    Ok(hash(&serde_json::to_vec(command)?))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFlushReceipt {
    pub draft_id: String,
    pub record_revision: u64,
    pub input_revision: u64,
    pub last_mutation_id: String,
    pub pending_operation_id: Option<String>,
    pub record_digest: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResolvedTarget {
    pub root: String,
    pub parent: String,
    pub workspace_id: String,
    pub hosted_slot: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStorageStatus {
    /// Preserved unpublished writes, never parsed or promoted as authoritative drafts.
    pub retained_stages: usize,
    pub retained_stage_bytes: u64,
}

fn invalid(message: &str) -> CoreError {
    CoreError::new("captureRecordUnavailable", message)
}
fn budget() -> CoreError {
    CoreError::new(
        "budget",
        "Capture recovery exceeds its bounded record or owner-domain budget",
    )
}
fn canonical_id(id: &str) -> Result<()> {
    if Uuid::parse_str(id).is_ok_and(|id_value| id_value.hyphenated().to_string() == id) {
        Ok(())
    } else {
        Err(invalid(
            "Capture identifiers require canonical lowercase UUIDs",
        ))
    }
}
fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn next_revision(revision: u64) -> Result<u64> {
    if revision >= MAX_CAPTURE_REVISION {
        return Err(invalid("Capture revision exhausted"));
    }
    Ok(revision + 1)
}
fn text_valid(text: &CaptureText) -> Result<()> {
    if text.utf16.len() > MAX_CAPTURE_TEXT_UNITS {
        return Err(budget());
    }
    Ok(())
}
fn add_input_bytes(bytes: &mut usize, added: usize) -> Result<()> {
    *bytes = bytes.checked_add(added).ok_or_else(budget)?;
    if *bytes > MAX_CAPTURE_RECORD_BYTES {
        return Err(budget());
    }
    Ok(())
}
fn input_valid(input: &CaptureInput) -> Result<()> {
    // A lower bound on JSON size rejects impossible Core inputs before hashing
    // or serializing a large typed value. Final canonical bytes remain decisive.
    let mut input_bytes = 0usize;
    for text in [
        &input.name,
        &input.title,
        &input.body,
        &input.parent_input,
        &input.parameters_text,
    ] {
        text_valid(text)?;
        add_input_bytes(&mut input_bytes, text.utf16.len() * 2)?;
    }
    if input.type_ids.len() > 32 || input.fields.len() > 64 || input.raw_inputs.len() > 1024 {
        return Err(budget());
    }
    let id_valid =
        |id: &str| !id.is_empty() && id.len() <= 512 && !id.chars().any(char::is_control);
    let mut types = BTreeSet::new();
    if input
        .type_ids
        .iter()
        .any(|id| !id_valid(id) || !types.insert(id))
    {
        return Err(invalid("Capture type IDs are invalid or duplicated"));
    }
    let mut fields = BTreeSet::new();
    if input
        .fields
        .iter()
        .any(|field| !id_valid(&field.field_id) || !fields.insert(&field.field_id))
    {
        return Err(invalid("Capture FieldIds are invalid or duplicated"));
    }
    if input.template_id.as_ref().is_some_and(|id| !id_valid(id)) {
        return Err(invalid("Capture template identity is invalid"));
    }
    let mut nodes = 0usize;
    for field in &input.fields {
        if let Some(value) = &field.value {
            native_valid(value, 0, &mut nodes, &mut input_bytes)?;
        }
    }
    let mut raw = BTreeSet::new();
    for entry in &input.raw_inputs {
        if entry.value_path.len() > 32 {
            return Err(budget());
        }
        text_valid(&entry.text)?;
        add_input_bytes(&mut input_bytes, entry.text.utf16.len() * 2)?;
        for part in &entry.value_path {
            match part {
                weftext_data::ValuePath::Member { key } => {
                    add_input_bytes(&mut input_bytes, key.len())?
                }
                weftext_data::ValuePath::Index { index }
                    if *index as u64 > MAX_CAPTURE_REVISION =>
                {
                    return Err(invalid("Capture path index is not an exact UI integer"));
                }
                _ => {}
            }
        }
        if !id_valid(&entry.field_id)
            || !id_valid(&entry.control_role)
            || !raw.insert(serde_json::to_vec(&(
                &entry.field_id,
                &entry.value_path,
                &entry.control_role,
            ))?)
        {
            return Err(invalid(
                "Capture raw-input addresses are invalid or duplicated",
            ));
        }
    }
    if let Some(basis) = &input.form_basis
        && (!basis.schema_ref.starts_with("weftext-config://")
            || !basis.schema_ref.ends_with("/schema")
            || basis.schema_ref.len() > 160
            || !digest_valid(&basis.semantic_digest))
    {
        return Err(invalid("Capture form basis is invalid"));
    }
    Ok(())
}
fn native_valid(
    value: &weftext_data::NativeValue,
    depth: usize,
    nodes: &mut usize,
    bytes: &mut usize,
) -> Result<()> {
    *nodes += 1;
    if depth > 32 || *nodes > 100_000 {
        return Err(budget());
    }
    match value {
        weftext_data::NativeValue::Array(values) => {
            for value in values {
                native_valid(value, depth + 1, nodes, bytes)?;
            }
        }
        weftext_data::NativeValue::Record(members) => {
            let mut keys = BTreeSet::new();
            for member in members {
                if !keys.insert(&member.key) {
                    return Err(invalid("Capture native members are duplicated"));
                }
                add_input_bytes(bytes, member.key.len())?;
                native_valid(&member.value, depth + 1, nodes, bytes)?;
            }
        }
        weftext_data::NativeValue::Text(value)
        | weftext_data::NativeValue::Int(value)
        | weftext_data::NativeValue::Float(value)
        | weftext_data::NativeValue::Decimal(value)
        | weftext_data::NativeValue::Date(value)
        | weftext_data::NativeValue::Time(value)
        | weftext_data::NativeValue::LocalDatetime(value)
        | weftext_data::NativeValue::Duration(value) => add_input_bytes(bytes, value.len())?,
        _ => {}
    }
    Ok(())
}
fn header_valid(header: &CaptureScopeHeader) -> Result<()> {
    if let CaptureOwner::Actor { actor } = &header.owner
        && (actor.is_empty() || actor.len() > 256)
    {
        return Err(invalid("Capture actor is invalid"));
    }
    match &header.library {
        CaptureLibrary::DeviceUnassigned if header.owner != CaptureOwner::Local => {
            return Err(invalid("Only local capture can be device-unassigned"));
        }
        CaptureLibrary::Bound { binding }
            if !Path::new(&binding.root).is_absolute()
                || binding
                    .workspace_id
                    .as_deref()
                    .is_none_or(|id| canonical_id(id).is_err()) =>
        {
            return Err(invalid("Capture library provenance is invalid"));
        }
        _ => {}
    }
    if serde_json::to_vec(header)?.len() > MAX_CAPTURE_HEADER_BYTES {
        return Err(budget());
    }
    Ok(())
}
fn pending_valid(pending: &FrozenCaptureRequest, input_revision: u64) -> Result<()> {
    canonical_id(&pending.operation_id)?;
    if !matches!(
        pending.request_kind.as_str(),
        "capture" | "period" | "hostedStructure"
    ) || pending.wire.is_empty()
        || pending.wire.len() > MAX_CAPTURE_WIRE_BYTES
        || !digest_valid(&pending.wire_digest)
        || hash(pending.wire.as_bytes()) != pending.wire_digest
        || pending.input_revision == 0
        || pending.input_revision > input_revision
    {
        return Err(invalid(
            "Frozen capture wire, request binding or input revision is invalid",
        ));
    }
    // Deliberately do not parse wire as serde_json::Value: a rejected original
    // request may contain an escaped unpaired surrogate and still needs fencing.
    wire_identity(&pending.wire, &pending.operation_id)?;
    Ok(())
}
pub(crate) fn validate_capture_record(
    path: &Path,
    data_dir: &Path,
    bytes: &[u8],
) -> Result<CaptureDraft> {
    if bytes.len() > MAX_CAPTURE_RECORD_BYTES {
        return Err(budget());
    }
    let draft: CaptureDraft = serde_json::from_slice(bytes)
        .map_err(|_| invalid("Capture record is malformed; original bytes were preserved"))?;
    if serde_json::to_vec(&draft)? != bytes {
        return Err(invalid(
            "Capture record is not its exact closed canonical encoding",
        ));
    }
    header_valid(&draft.scope)?;
    canonical_id(&draft.draft_id)?;
    canonical_id(&draft.last_mutation.mutation_id)?;
    if draft.version != 1
        || draft.revision > MAX_CAPTURE_REVISION
        || draft.revision == 0
        || draft.last_mutation.resulting_revision != draft.revision
        || !digest_valid(&draft.last_mutation.fingerprint)
        || draft.input_revision == 0
        || draft.input_revision > draft.revision
        || capture_record_path(data_dir, &draft.scope.owner, &draft.draft_id)? != path
    {
        return Err(invalid("Capture version, namespace or revision is invalid"));
    }
    input_valid(&draft.current_input)?;
    match (&draft.scope.library, &draft.target) {
        (CaptureLibrary::DeviceUnassigned, None) => {}
        (CaptureLibrary::Bound { binding }, Some(target)) => {
            absolute_path(Path::new(&target.root_hint))?;
            if let CaptureParent::Managed {
                reference,
                path_hint,
                identity_digest,
            } = &target.parent
            {
                let workspace = binding.workspace_id.as_deref().unwrap();
                let (kind, reference_workspace, id) = directory_identity::parse_ref(reference)?;
                if !digest_valid(identity_digest)
                    || !matches!(kind, "node" | "directory")
                    || reference_workspace != workspace
                    || canonical_id(id).is_err()
                    || reference.len() > 180
                    || !absolute_path(Path::new(path_hint))?.starts_with(&target.root_hint)
                {
                    return Err(invalid("Capture parent belongs to another library"));
                }
            }
            if target
                .hosted_slot
                .as_ref()
                .is_some_and(|slot| slot.is_empty() || slot.len() > 256)
            {
                return Err(invalid("Capture approved slot is invalid"));
            }
            if let Some(basis) = &draft.current_input.form_basis
                && basis.schema_ref
                    != format!(
                        "weftext-config://{}/schema",
                        binding.workspace_id.as_deref().unwrap()
                    )
            {
                return Err(invalid("Capture Schema belongs to another library"));
            }
        }
        _ => return Err(invalid("Capture target and original scope differ")),
    }
    if let Some(pending) = &draft.pending {
        if draft.target.is_none() {
            return Err(invalid("Unassigned capture cannot have a submission"));
        }
        pending_valid(pending, draft.input_revision)?;
        pending_target_valid(draft.target.as_ref().unwrap(), pending)?;
    }
    Ok(draft)
}
fn pending_target_valid(target: &CaptureTarget, request: &FrozenCaptureRequest) -> Result<()> {
    let wire = wire_identity(&request.wire, &request.operation_id)?;
    let parent = match &target.parent {
        CaptureParent::LibraryRoot => &target.root_hint,
        CaptureParent::Managed { path_hint, .. } => path_hint,
    };
    match request.request_kind.as_str() {
        "capture" | "period"
            if wire.root.as_deref() == Some(target.root_hint.as_str())
                && wire.parent.as_deref() == Some(parent.as_str())
                && target.hosted_slot.is_none() =>
        {
            Ok(())
        }
        "hostedStructure"
            if target.hosted_slot.is_some()
                && wire.resource == target.hosted_slot
                && wire.destination == target.hosted_slot =>
        {
            Ok(())
        }
        _ => Err(invalid(
            "Frozen request does not name the original capture target",
        )),
    }
}

/// Inspect only typed top-level identity; all author payload uses IgnoredAny.
/// serde_json validates its syntax without decoding escaped lone UTF-16 strings.
struct WireIdentity {
    root: Option<String>,
    parent: Option<String>,
    resource: Option<String>,
    destination: Option<String>,
}
fn wire_identity(wire: &str, expected: &str) -> Result<WireIdentity> {
    struct Metadata<'a> {
        expected: &'a str,
    }
    impl<'de> serde::de::Visitor<'de> for Metadata<'_> {
        type Value = WireIdentity;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("the original request object with its exact operationId")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<WireIdentity, M::Error> {
            use serde::de::Error;
            let mut keys = BTreeSet::new();
            let mut found = false;
            let mut identity = WireIdentity {
                root: None,
                parent: None,
                resource: None,
                destination: None,
            };
            while let Some(key) = map.next_key::<String>()? {
                if keys.len() == 64 || !keys.insert(key.clone()) {
                    return Err(M::Error::custom(
                        "Original wire has duplicated or excessive top-level members",
                    ));
                }
                match key.as_str() {
                    "operationId" => {
                        if map.next_value::<String>()? != self.expected {
                            return Err(M::Error::custom(
                                "Frozen wire belongs to another operationId",
                            ));
                        }
                        found = true;
                    }
                    "root" => identity.root = Some(map.next_value()?),
                    "parent" => identity.parent = Some(map.next_value()?),
                    "resource" => identity.resource = Some(map.next_value()?),
                    "destination" => identity.destination = Some(map.next_value()?),
                    _ => {
                        map.next_value::<serde::de::IgnoredAny>()?;
                    }
                }
            }
            if !found {
                return Err(M::Error::custom("Original operationId is missing"));
            }
            Ok(identity)
        }
    }
    let mut decoder = serde_json::Deserializer::from_str(wire);
    let identity = serde::de::Deserializer::deserialize_map(&mut decoder, Metadata { expected })
        .map_err(|_| invalid("Frozen capture wire or original identity is invalid"))?;
    decoder
        .end()
        .map_err(|_| invalid("Frozen capture wire contains trailing data"))?;
    Ok(identity)
}
pub(crate) fn capture_record_path(
    data_dir: &Path,
    owner: &CaptureOwner,
    id: &str,
) -> Result<PathBuf> {
    canonical_id(id)?;
    let directory = match owner {
        CaptureOwner::Local => data_dir.join(CAPTURE_DIRECTORY),
        CaptureOwner::Actor { actor } => data_dir
            .join("actors")
            .join(hash(actor.as_bytes()))
            .join(CAPTURE_DIRECTORY),
    };
    Ok(directory.join(format!("{id}.json")))
}
pub(crate) fn validate_capture_header_path(
    path: &Path,
    data_dir: &Path,
    header: &CaptureScopeHeader,
) -> Result<()> {
    header_valid(header)?;
    let id = path
        .file_stem()
        .and_then(|id| id.to_str())
        .ok_or_else(|| invalid("Invalid capture filename"))?;
    if capture_record_path(data_dir, &header.owner, id)? != path {
        return Err(invalid("Capture header owner differs from namespace"));
    }
    Ok(())
}

/// Read exactly the first metadata object, never a byte of an author payload.
/// The same guarded reader remains in the caller's ownership through routing.
pub(crate) fn read_capture_header(reader: &mut (impl Read + Seek)) -> Result<CaptureScopeHeader> {
    let original = reader.stream_position()?;
    reader.seek(SeekFrom::Start(0))?;
    let result = (|| {
        let mut prefix = Vec::new();
        let mut quoted = false;
        let mut escaped = false;
        let mut depth = 0usize;
        let mut value_start = None;
        loop {
            if prefix.len() == MAX_CAPTURE_PREFIX_BYTES {
                return Err(budget());
            }
            let mut byte = [0];
            reader
                .read_exact(&mut byte)
                .map_err(|_| invalid("Capture scope header is truncated"))?;
            let b = byte[0];
            prefix.push(b);
            if value_start.is_none() {
                if b == b':' && !quoted {
                    let head = std::str::from_utf8(&prefix[..prefix.len() - 1])
                        .map_err(|_| invalid("Invalid capture scope header"))?;
                    let key = head
                        .trim_start_matches([' ', '\r', '\n', '\t'])
                        .strip_prefix('{')
                        .ok_or_else(|| invalid("Capture scope must be the first field"))?
                        .trim_matches(|c: char| matches!(c, ' ' | '\r' | '\n' | '\t'));
                    if serde_json::from_str::<String>(key).ok().as_deref() != Some("scope") {
                        return Err(invalid("Capture scope must be the first field"));
                    }
                    value_start = Some(prefix.len());
                } else if b == b'"' && !escaped {
                    quoted = !quoted;
                }
                escaped = quoted && b == b'\\' && !escaped;
                continue;
            }
            let start = value_start.unwrap();
            if prefix.len() - start > MAX_CAPTURE_HEADER_BYTES {
                return Err(budget());
            }
            if quoted {
                if b == b'"' && !escaped {
                    quoted = false;
                }
                escaped = b == b'\\' && !escaped;
                continue;
            }
            match b {
                b'"' => {
                    quoted = true;
                    escaped = false;
                }
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    if depth == 0 {
                        return Err(invalid("Capture scope object is missing"));
                    }
                    depth -= 1;
                    if depth == 0 {
                        let header: CaptureScopeHeader =
                            serde_json::from_slice(&prefix[start..])
                                .map_err(|_| invalid("Capture scope header is invalid"))?;
                        header_valid(&header)?;
                        if serde_json::to_vec(&header)? != prefix[start..] {
                            return Err(invalid(
                                "Capture scope metadata has unknown or noncanonical members",
                            ));
                        }
                        return Ok(header);
                    }
                }
                _ if depth == 0 && !b.is_ascii_whitespace() => {
                    return Err(invalid("Capture scope must be an object"));
                }
                _ => {}
            }
        }
    })();
    reader.seek(SeekFrom::Start(original))?;
    result
}

impl Core {
    fn capture_staging_directory(&self) -> PathBuf {
        self.data_dir.join(CAPTURE_STAGING_DIRECTORY).join(
            self.actor
                .as_ref()
                .map_or_else(|| "local".into(), |actor| hash(actor.as_bytes())),
        )
    }
    fn capture_owner(&self) -> CaptureOwner {
        self.actor
            .as_ref()
            .map_or(CaptureOwner::Local, |actor| CaptureOwner::Actor {
                actor: actor.clone(),
            })
    }
    fn check_capture_scope(&self, scope: &CaptureScopeHeader) -> Result<()> {
        if scope.owner != self.capture_owner() {
            return Err(invalid("Capture owner differs"));
        }
        if self.actor.is_some() {
            match (&scope.library, &self.actor_library) {
                (CaptureLibrary::Bound { binding }, Some(actual)) if binding == actual => {}
                _ => return Err(invalid("Capture actor library differs")),
            }
        }
        Ok(())
    }
    fn capture_read(&self, id: &str) -> Result<(CaptureDraft, Vec<u8>)> {
        let path = capture_record_path(&self.data_dir, &self.capture_owner(), id)?;
        let guard = ParentGuard::acquire(&path)?;
        let mut file = guard.open_read_guard_file()?;
        let header = read_capture_header(&mut file)?;
        self.check_capture_scope(&header)?;
        if file.metadata()?.len() > MAX_CAPTURE_RECORD_BYTES as u64 {
            return Err(budget());
        }
        let mut bytes = Vec::new();
        file.take(MAX_CAPTURE_RECORD_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let draft = validate_capture_record(&path, &self.data_dir, &bytes)?;
        if draft.scope != header {
            return Err(invalid("Capture scope changed during read"));
        }
        Ok((draft, bytes))
    }
    pub fn read_capture_draft(&self, id: &str) -> Result<CaptureDraft> {
        self.capture_read(id).map(|(draft, _)| draft)
    }
    pub fn list_capture_drafts(&self) -> Result<Vec<CaptureDraft>> {
        let directory = self.draft_directory(CAPTURE_DIRECTORY);
        let guard = ParentGuard::acquire(&directory.join(".capture-list"))?;
        let (entries, complete) = guard.directory_entries(MAX_CAPTURE_RECORDS)?;
        if !complete {
            return Err(budget());
        }
        let mut selected_ids = Vec::new();
        let mut selected_bytes = 0u64;
        for (name, attrs) in entries {
            if attrs & (0x10 | 0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000) != 0 {
                return Err(invalid("Invalid capture namespace member"));
            }
            let path = directory.join(name);
            let file_guard = ParentGuard::acquire(&path)?;
            let mut file = file_guard.open_read_guard_file()?;
            let header = read_capture_header(&mut file)?;
            validate_capture_header_path(&path, &self.data_dir, &header)?;
            let selected = self.check_capture_scope(&header).is_ok();
            if selected {
                let bytes = file.metadata()?.len();
                if bytes > MAX_CAPTURE_RECORD_BYTES as u64 {
                    return Err(budget());
                }
                selected_bytes = selected_bytes.checked_add(bytes).ok_or_else(budget)?;
                if selected_bytes > MAX_CAPTURE_DOMAIN_BYTES {
                    return Err(budget());
                }
            }
            drop(file);
            if selected {
                let id = path.file_stem().and_then(|id| id.to_str()).unwrap();
                selected_ids.push(id.to_owned());
            }
        }
        let mut drafts = Vec::new();
        let mut actual_bytes = 0u64;
        for id in selected_ids {
            let (draft, bytes) = self.capture_read(&id)?;
            actual_bytes = actual_bytes
                .checked_add(bytes.len() as u64)
                .ok_or_else(budget)?;
            if actual_bytes > MAX_CAPTURE_DOMAIN_BYTES {
                return Err(budget());
            }
            drafts.push(draft);
        }
        drafts.sort_by(|a, b| a.draft_id.cmp(&b.draft_id));
        Ok(drafts)
    }
    pub fn capture_mutation_status(
        &self,
        id: &str,
        mutation_id: &str,
        command: &CaptureMutationCommand,
    ) -> Result<CaptureMutationLookup> {
        canonical_id(mutation_id)?;
        let draft = self.read_capture_draft(id)?;
        let status = if draft.last_mutation.mutation_id == mutation_id {
            if draft.last_mutation.fingerprint == command.fingerprint()? {
                "applied"
            } else {
                "differentCommand"
            }
        } else {
            "supersededOrUnknown"
        };
        Ok(CaptureMutationLookup { status, draft })
    }
    fn capture_write(&self, draft: &CaptureDraft, create: bool) -> Result<CaptureDraft> {
        self.check_capture_scope(&draft.scope)?;
        let path = capture_record_path(&self.data_dir, &draft.scope.owner, &draft.draft_id)?;
        let bytes = serde_json::to_vec(draft)?;
        validate_capture_record(&path, &self.data_dir, &bytes)?;
        let directory = self.draft_directory(CAPTURE_DIRECTORY);
        let guard = ParentGuard::acquire(&directory.join(".capture-budget"))?;
        let (entries, complete) = guard.directory_entries(MAX_CAPTURE_RECORDS)?;
        if !complete {
            return Err(budget());
        }
        let mut total = bytes.len() as u64;
        let mut count = 1usize;
        for (name, attrs) in entries {
            if attrs & (0x10 | 0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000) != 0 {
                return Err(invalid("Capture namespace contains an unsupported member"));
            }
            let existing = directory.join(name);
            if existing == path {
                continue;
            }
            let existing_guard = ParentGuard::acquire(&existing)?;
            let mut file = existing_guard.open_read_guard_file()?;
            let header = read_capture_header(&mut file)?;
            let id = existing
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| invalid("Invalid capture filename"))?;
            if capture_record_path(&self.data_dir, &header.owner, id)? != existing
                || header.owner != draft.scope.owner
            {
                return Err(invalid("Capture owner namespace differs"));
            }
            total = total
                .checked_add(file.metadata()?.len())
                .ok_or_else(budget)?;
            count += 1;
        }
        let stages = self.capture_storage_status()?;
        total = total
            .checked_add(stages.retained_stage_bytes)
            .ok_or_else(budget)?;
        if count > MAX_CAPTURE_RECORDS
            || total > MAX_CAPTURE_DOMAIN_BYTES
            || stages.retained_stages >= MAX_CAPTURE_RECORDS
        {
            return Err(budget());
        }
        // Partial publication never appears in the authoritative enumeration.
        let stage =
            self.capture_staging_directory()
                .join(format!("{}.{}.stage", draft.draft_id, fresh()));
        let _stage_guard = ParentGuard::acquire(&stage)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stage)?;
        #[cfg(all(test, windows))]
        hosted::definition_test_failpoint("captureStageCreated")?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(all(test, windows))]
        hosted::definition_test_failpoint("captureStageFlushed")?;
        if create {
            backend::publish_private_create(&stage, &path)?;
        } else {
            backend::publish_private_replace(&stage, &path)?;
        }
        let (readback, actual) = self.capture_read(&draft.draft_id)?;
        if actual != bytes {
            return Err(invalid("Capture durable readback differs"));
        }
        Ok(readback)
    }
    pub fn capture_storage_status(&self) -> Result<CaptureStorageStatus> {
        let directory = self.capture_staging_directory();
        let guard = ParentGuard::acquire(&directory.join(".capture-staging-budget"))?;
        let (entries, complete) = guard.directory_entries(MAX_CAPTURE_RECORDS)?;
        if !complete {
            return Err(budget());
        }
        let mut status = CaptureStorageStatus {
            retained_stages: 0,
            retained_stage_bytes: 0,
        };
        for (name, attrs) in entries {
            if attrs & (0x10 | 0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000) != 0 {
                return Err(invalid("Invalid capture staging member"));
            }
            let name = name
                .to_str()
                .ok_or_else(|| invalid("Invalid capture staging name"))?;
            let parts = name.split('.').collect::<Vec<_>>();
            if parts.len() != 3 || parts[2] != "stage" {
                return Err(invalid("Unrecognized capture staging member"));
            }
            canonical_id(parts[0])?;
            canonical_id(parts[1])?;
            let path = directory.join(name);
            let file_guard = ParentGuard::acquire(&path)?;
            let file = file_guard.open_read_guard_file()?;
            let length = file.metadata()?.len();
            if length > MAX_CAPTURE_RECORD_BYTES as u64 {
                return Err(budget());
            }
            status.retained_stages += 1;
            status.retained_stage_bytes = status
                .retained_stage_bytes
                .checked_add(length)
                .ok_or_else(budget)?;
        }
        if status.retained_stage_bytes > MAX_CAPTURE_DOMAIN_BYTES {
            return Err(budget());
        }
        Ok(status)
    }
    pub fn create_capture_draft(
        &mut self,
        id: &str,
        mutation_id: &str,
        input: CaptureInput,
    ) -> Result<CaptureDraft> {
        canonical_id(id)?;
        canonical_id(mutation_id)?;
        input_valid(&input)?;
        if self.actor.is_some() {
            return Err(invalid(
                "Hosted capture creation requires a qualified library target",
            ));
        }
        let fingerprint = hash(&serde_json::to_vec(&(0u64, "create", &input))?);
        let path = capture_record_path(&self.data_dir, &self.capture_owner(), id)?;
        if path.exists() {
            let draft = self.read_capture_draft(id)?;
            return Self::capture_replay(draft, mutation_id, &fingerprint)?
                .ok_or_else(|| invalid("Capture ID already exists"));
        }
        let draft = CaptureDraft {
            scope: CaptureScopeHeader {
                owner: self.capture_owner(),
                library: CaptureLibrary::DeviceUnassigned,
            },
            version: 1,
            draft_id: id.into(),
            revision: 1,
            input_revision: 1,
            last_mutation: CaptureMutation {
                mutation_id: mutation_id.into(),
                fingerprint,
                resulting_revision: 1,
            },
            target: None,
            current_input: input,
            pending: None,
        };
        self.capture_write(&draft, true)
    }
    pub fn create_targeted_capture_draft(
        &mut self,
        id: &str,
        mutation_id: &str,
        input: CaptureInput,
        root: &Path,
        parent: &Path,
        hosted_slot: Option<String>,
    ) -> Result<CaptureDraft> {
        canonical_id(id)?;
        canonical_id(mutation_id)?;
        input_valid(&input)?;
        let fingerprint = hash(&serde_json::to_vec(&(
            0u64,
            "createTargeted",
            &input,
            display(root),
            display(parent),
            &hosted_slot,
        ))?);
        let path = capture_record_path(&self.data_dir, &self.capture_owner(), id)?;
        if path.exists() {
            let draft = self.read_capture_draft(id)?;
            return Self::capture_replay(draft, mutation_id, &fingerprint)?
                .ok_or_else(|| invalid("Capture ID already exists"));
        }
        let (library, target) = self.capture_target(root, parent, hosted_slot)?;
        let draft = CaptureDraft {
            scope: CaptureScopeHeader {
                owner: self.capture_owner(),
                library,
            },
            version: 1,
            draft_id: id.into(),
            revision: 1,
            input_revision: 1,
            last_mutation: CaptureMutation {
                mutation_id: mutation_id.into(),
                fingerprint,
                resulting_revision: 1,
            },
            target: Some(target),
            current_input: input,
            pending: None,
        };
        self.capture_write(&draft, true)
    }
    fn capture_replay(
        draft: CaptureDraft,
        mutation_id: &str,
        fingerprint: &str,
    ) -> Result<Option<CaptureDraft>> {
        if draft.last_mutation.mutation_id != mutation_id {
            return Ok(None);
        }
        if draft.last_mutation.fingerprint != fingerprint {
            return Err(invalid("Capture mutationId belongs to different input"));
        }
        Ok(Some(draft))
    }
    fn capture_edit(
        &self,
        id: &str,
        expected: u64,
        mutation_id: &str,
        fingerprint: &str,
    ) -> Result<std::result::Result<CaptureDraft, CaptureDraft>> {
        canonical_id(mutation_id)?;
        let draft = self.read_capture_draft(id)?;
        if let Some(replay) = Self::capture_replay(draft.clone(), mutation_id, fingerprint)? {
            return Ok(Err(replay));
        }
        if draft.revision != expected {
            return Err(CoreError::new(
                "captureConflict",
                "Capture changed in another writer; old and new input must be retained",
            ));
        }
        Ok(Ok(draft))
    }
    fn capture_finish(
        &self,
        mut draft: CaptureDraft,
        mutation_id: &str,
        fingerprint: String,
    ) -> Result<CaptureDraft> {
        draft.revision = next_revision(draft.revision)?;
        draft.last_mutation = CaptureMutation {
            mutation_id: mutation_id.into(),
            fingerprint,
            resulting_revision: draft.revision,
        };
        self.capture_write(&draft, false)
    }
    pub fn update_capture_input(
        &mut self,
        id: &str,
        expected: u64,
        mutation_id: &str,
        input: CaptureInput,
    ) -> Result<CaptureDraft> {
        input_valid(&input)?;
        let fingerprint = hash(&serde_json::to_vec(&(expected, "input", &input))?);
        let mut draft = match self.capture_edit(id, expected, mutation_id, &fingerprint)? {
            Ok(draft) => draft,
            Err(replay) => return Ok(replay),
        };
        draft.input_revision = next_revision(draft.input_revision)?;
        draft.current_input = input;
        self.capture_finish(draft, mutation_id, fingerprint)
    }
    /// A new library selection is allowed only before the first pending request.
    /// Selecting another library afterwards uses another draft; it never retags one.
    pub fn select_capture_target(
        &mut self,
        id: &str,
        expected: u64,
        mutation_id: &str,
        root: &Path,
        parent: &Path,
        hosted_slot: Option<String>,
    ) -> Result<CaptureDraft> {
        let fingerprint = hash(&serde_json::to_vec(&(
            expected,
            "target",
            display(root),
            display(parent),
            &hosted_slot,
        ))?);
        let mut draft = match self.capture_edit(id, expected, mutation_id, &fingerprint)? {
            Ok(draft) => draft,
            Err(replay) => return Ok(replay),
        };
        if draft.pending.is_some() {
            return Err(invalid("Pending capture target is immutable"));
        }
        let (library, target) = self.capture_target(root, parent, hosted_slot)?;
        if let CaptureLibrary::Bound { binding: original } = &draft.scope.library
            && library
                != (CaptureLibrary::Bound {
                    binding: original.clone(),
                })
        {
            return Err(invalid("Use a separate capture draft for another library"));
        }
        draft.scope.library = library;
        draft.target = Some(target);
        draft.input_revision = next_revision(draft.input_revision)?;
        self.capture_finish(draft, mutation_id, fingerprint)
    }
    fn capture_target(
        &self,
        root: &Path,
        parent: &Path,
        hosted_slot: Option<String>,
    ) -> Result<(CaptureLibrary, CaptureTarget)> {
        let root = absolute_path(root)?;
        let parent = absolute_path(parent)?;
        let guard = ParentGuard::acquire(&root.join(".capture-root"))?;
        let parent_guard = ParentGuard::acquire(&parent.join(".capture-parent"))?;
        let workspace = self.require_workspace(&root)?;
        let binding = ActorLibraryBinding {
            root: display(&guard.directory_coordinates()?),
            root_identity: guard.directory_identity()?,
            workspace_id: Some(workspace.workspace_id),
        };
        if self.actor.is_some() && self.actor_library.as_ref() != Some(&binding) {
            return Err(invalid("Trusted capture library differs"));
        }
        let selected = if parent == root {
            CaptureParent::LibraryRoot
        } else {
            let selected = self.capture_navigation_target(&root, &parent)?;
            CaptureParent::Managed {
                reference: selected
                    .target_ref
                    .ok_or_else(|| invalid("Capture parent needs an admitted stable identity"))?,
                path_hint: display(&parent),
                identity_digest: hash(&serde_json::to_vec(&parent_guard.directory_identity()?)?),
            }
        };
        Ok((
            CaptureLibrary::Bound { binding },
            CaptureTarget {
                root_hint: display(&root),
                parent: selected,
                hosted_slot,
            },
        ))
    }
    pub fn freeze_capture_submission(
        &mut self,
        id: &str,
        expected: u64,
        mutation_id: &str,
        request: FrozenCaptureRequest,
    ) -> Result<CaptureDraft> {
        let fingerprint = hash(&serde_json::to_vec(&(expected, "freeze", &request))?);
        let mut draft = match self.capture_edit(id, expected, mutation_id, &fingerprint)? {
            Ok(draft) => draft,
            Err(replay) => return Ok(replay),
        };
        if draft.pending.is_some()
            || draft.target.is_none()
            || request.input_revision != draft.input_revision
        {
            return Err(invalid(
                "Capture already has pending input or its frozen revision differs",
            ));
        }
        pending_valid(&request, draft.input_revision)?;
        let actual = self.resolve_capture_target_record(&draft)?;
        let target = draft.target.as_mut().unwrap();
        // The hint is frozen at submission time. A later rename must not silently
        // rewrite the original wire, while an earlier lawful move may be resolved.
        if let CaptureParent::Managed { path_hint, .. } = &mut target.parent {
            *path_hint = actual.parent;
        }
        pending_target_valid(target, &request)?;
        draft.pending = Some(request);
        self.capture_finish(draft, mutation_id, fingerprint)
    }
    pub fn resolve_capture_target(&self, id: &str) -> Result<CaptureResolvedTarget> {
        self.resolve_capture_target_record(&self.read_capture_draft(id)?)
    }
    fn resolve_capture_target_record(&self, draft: &CaptureDraft) -> Result<CaptureResolvedTarget> {
        let target = draft
            .target
            .as_ref()
            .ok_or_else(|| invalid("Select an actual capture library first"))?;
        let CaptureLibrary::Bound { binding } = &draft.scope.library else {
            return Err(invalid("Capture is not bound to a library"));
        };
        let root = absolute_path(Path::new(&target.root_hint))?;
        let guard = ParentGuard::acquire(&root.join(".capture-resolve"))?;
        let workspace = self.require_workspace(&root)?;
        if guard.directory_identity()? != binding.root_identity
            || workspace.workspace_id != binding.workspace_id.as_deref().unwrap()
            || display(&guard.directory_coordinates()?) != binding.root
        {
            return Err(invalid(
                "The original capture library was replaced or moved",
            ));
        }
        let parent = match &target.parent {
            CaptureParent::LibraryRoot => display(&root),
            CaptureParent::Managed {
                reference,
                identity_digest,
                ..
            } => {
                let resolved = self.resolve_navigation_target(&root, reference)?;
                let path = PathBuf::from(
                    resolved
                        .path
                        .ok_or_else(|| invalid("Original capture parent is unavailable"))?,
                );
                // Both NodeRef and DirectoryRef resolve to their actual container.
                let parent_guard = ParentGuard::acquire(&path.join(".capture-parent"))?;
                if hash(&serde_json::to_vec(&parent_guard.directory_identity()?)?)
                    != *identity_digest
                {
                    return Err(invalid("Original capture parent container was replaced"));
                }
                display(&path)
            }
        };
        Ok(CaptureResolvedTarget {
            root: display(&root),
            parent,
            workspace_id: workspace.workspace_id,
            hosted_slot: target.hosted_slot.clone(),
        })
    }
    /// Durable snapshot ACK, not a native close lock. The host must freeze input
    /// and reject this ACK if its surface epoch or in-memory edit serial changes.
    pub fn capture_flush_receipt(
        &self,
        id: &str,
        expected: u64,
        input_revision: u64,
        mutation_id: &str,
    ) -> Result<CaptureFlushReceipt> {
        let (draft, bytes) = self.capture_read(id)?;
        if draft.revision != expected
            || draft.input_revision != input_revision
            || draft.last_mutation.mutation_id != mutation_id
        {
            return Err(CoreError::new(
                "captureConflict",
                "Capture changed before close; no close permit was issued",
            ));
        }
        Ok(CaptureFlushReceipt {
            draft_id: draft.draft_id,
            record_revision: draft.revision,
            input_revision: draft.input_revision,
            last_mutation_id: draft.last_mutation.mutation_id,
            pending_operation_id: draft.pending.map(|pending| pending.operation_id),
            record_digest: hash(&bytes),
        })
    }
}

#[cfg(all(test, windows))]
#[path = "capture_draft/tests.rs"]
mod tests;
