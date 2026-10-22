//! Private, not-yet-created author input. The operation journal alone owns effects.
//! Raw text uses UTF-16 code units, so an unfinished JS string is not silently lost.
use super::*;
use hosted::ActorLibraryBinding;
use std::collections::{BTreeMap, BTreeSet};
#[path = "capture_draft/field_wire.rs"]
mod field_wire;
use weftext_messages::UiMessage;

pub const MAX_CAPTURE_RECORD_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CAPTURE_DOMAIN_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_CAPTURE_RECORDS: usize = 128;
const MAX_CAPTURE_HEADER_BYTES: usize = 2048;
const MAX_CAPTURE_PREFIX_BYTES: usize = 4096;
const MAX_CAPTURE_WIRE_BYTES: usize = 3 * 1024 * 1024;
const MAX_CAPTURE_TEXT_UNITS: usize = 2 * 1024 * 1024;
const MAX_CAPTURE_REVISION: u64 = 9_007_199_254_740_991;
const CAPTURE_RECORD_VERSION: u32 = 2;
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
            "captureTextIncomplete", UiMessage::error_core_capture_draft_unpaired_u_t_f16_input_retained_complete_it_before_creating_u_t_f8_author_source(),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureRawInput {
    pub field_id: String,
    #[serde(deserialize_with = "field_wire::value_path")]
    pub value_path: Vec<weftext_data::ValuePath>,
    pub control_role: String,
    pub text: CaptureText,
    pub binding: CaptureRawBinding,
    pub author: Option<CaptureRawAuthor>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureRawBinding {
    Exact,
    CopyOnly,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureRawProvenance {
    pub producer_id: String,
    pub input_id: String,
    pub revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureRawAuthor {
    pub text: CaptureText,
    pub provenance: CaptureRawProvenance,
}
/// Pure, copy-only author proposal: no protected control baseline. The actual
/// browser collector proves the source; a claimed producer never grants access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureAuthorRawInput {
    pub field_id: String,
    #[serde(deserialize_with = "field_wire::value_path")]
    pub value_path: Vec<weftext_data::ValuePath>,
    pub control_role: String,
    pub text: CaptureText,
    pub provenance: CaptureRawProvenance,
}
impl CaptureRawInput {
    fn author_only(self) -> Option<CaptureAuthorRawInput> {
        self.author.map(|author| CaptureAuthorRawInput {
            field_id: self.field_id,
            value_path: self.value_path,
            control_role: self.control_role,
            text: author.text,
            provenance: author.provenance,
        })
    }
}
impl CaptureAuthorRawInput {
    fn copy_only(self) -> CaptureRawInput {
        CaptureRawInput {
            field_id: self.field_id,
            value_path: self.value_path,
            control_role: self.control_role,
            text: self.text.clone(),
            binding: CaptureRawBinding::CopyOnly,
            author: Some(CaptureRawAuthor {
                text: self.text,
                provenance: self.provenance,
            }),
        }
    }
}
/// A retired producer path is retained verbatim, never silently attached to the
/// new list index, union branch or changed Schema field occupying that address.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureDetachedRawInput {
    pub retirement_id: String,
    pub reason: CaptureRawRetirement,
    pub input: CaptureRawInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureDetachedAuthorInput {
    pub retirement_id: String,
    pub reason: CaptureRawRetirement,
    pub input: CaptureAuthorRawInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureRawRetirement {
    Structure,
    Schema,
    Producer,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFieldInput {
    pub field_id: String,
    pub value_state: CaptureFieldValueState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CaptureFieldValueState {
    Valid {
        #[serde(deserialize_with = "field_wire::valid_native")]
        value: weftext_data::NativeValue,
    },
    /// One private exact native-field tree, never a Schema/Base/business form.
    Incomplete { wire: CaptureText },
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
/// The actual creation DTO is shared by HTTP dispatch and original delivery.
/// It is not rebuilt from today's Schema, current input or a cached result path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureCreationRequest {
    pub root: String,
    pub parent: String,
    pub name: String,
    pub types: Vec<String>,
    pub title: Option<String>,
    #[serde(default, deserialize_with = "capture_fields")]
    pub fields: BTreeMap<String, weftext_data::NativeValue>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub body_mode: CaptureBodyMode,
    pub form_basis: Option<CaptureFormBasis>,
    pub operation_id: String,
}
fn capture_fields<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, weftext_data::NativeValue>, D::Error> {
    use serde::de::{Error, MapAccess, Visitor};
    #[derive(Deserialize)]
    #[serde(transparent)]
    struct StrictNative(
        #[serde(deserialize_with = "field_wire::valid_native")] weftext_data::NativeValue,
    );
    struct Fields;
    impl<'de> Visitor<'de> for Fields {
        type Value = BTreeMap<String, weftext_data::NativeValue>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("exact unique Capture fields")
        }
        fn visit_map<M: MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut fields = BTreeMap::new();
            while let Some(key) = map.next_key::<String>()? {
                if fields.len() >= 64 || fields.contains_key(&key) {
                    return Err(M::Error::custom("Capture field budget or duplicate key"));
                }
                fields.insert(key, map.next_value::<StrictNative>()?.0);
            }
            Ok(fields)
        }
    }
    deserializer.deserialize_map(Fields)
}
/// Pure owner input is separate from every complete private control value.
/// Restoring/sampling a control does not manufacture keyboard provenance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureAuthorTexts {
    pub name: Vec<CaptureRawAuthor>,
    pub title: Vec<CaptureRawAuthor>,
    pub body: Vec<CaptureRawAuthor>,
    pub root_input: Vec<CaptureRawAuthor>,
    pub parent_input: Vec<CaptureRawAuthor>,
    pub parameters_text: Vec<CaptureRawAuthor>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureInput {
    pub name: CaptureText,
    pub title: CaptureText,
    pub body: CaptureText,
    pub body_mode: CaptureBodyMode,
    pub root_input: CaptureText,
    pub parent_input: CaptureText,
    pub parameters_text: CaptureText,
    pub author_texts: CaptureAuthorTexts,
    pub author_body_mode: Option<CaptureBodyMode>,
    pub template_id: Option<String>,
    pub type_ids: Vec<String>,
    pub fields: Vec<CaptureFieldInput>,
    pub raw_inputs: Vec<CaptureRawInput>,
    pub detached_raw_inputs: Vec<CaptureDetachedRawInput>,
    pub form_basis: Option<CaptureFormBasis>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureAuthorInput {
    pub texts: CaptureAuthorTexts,
    pub body_mode: Option<CaptureBodyMode>,
    pub raw_inputs: Vec<CaptureAuthorRawInput>,
    pub detached_raw_inputs: Vec<CaptureDetachedAuthorInput>,
}
impl CaptureAuthorInput {
    fn install(self, input: &mut CaptureInput) -> Result<()> {
        merge_author_texts(&mut input.author_texts, self.texts)?;
        input.author_body_mode = self.body_mode;
        // Only the currently authorized full mutation can replace the complete
        // candidate or its exact private control values. A pure owner checkpoint
        // remains available after revoke without changing that protected state.
        let mut retained = Vec::new();
        for author in self.raw_inputs {
            if let Some(raw) = input.raw_inputs.iter_mut().find(|raw| {
                raw.field_id == author.field_id
                    && raw.value_path == author.value_path
                    && raw.control_role == author.control_role
            }) {
                if let Some(old) = &raw.author {
                    if old.provenance.producer_id != author.provenance.producer_id
                        || old.provenance.input_id != author.provenance.input_id
                    {
                        retained.push(raw.clone());
                    } else if author.provenance.revision < old.provenance.revision
                        || author.provenance.revision == old.provenance.revision
                            && author.text != old.text
                    {
                        return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_raw_author_proof_revision_differs()));
                    }
                }
                raw.author = Some(CaptureRawAuthor {
                    text: author.text,
                    provenance: author.provenance,
                });
            } else {
                input.raw_inputs.push(author.copy_only());
            }
        }
        for author in self.detached_raw_inputs {
            if let Some(entry) = input.detached_raw_inputs.iter_mut().find(|entry| {
                entry.retirement_id == author.retirement_id
                    && entry.input.field_id == author.input.field_id
                    && entry.input.value_path == author.input.value_path
                    && entry.input.control_role == author.input.control_role
            }) {
                if let Some(old) = &entry.input.author
                    && (old.provenance.producer_id != author.input.provenance.producer_id
                        || old.provenance.input_id != author.input.provenance.input_id
                        || author.input.provenance.revision < old.provenance.revision
                        || author.input.provenance.revision == old.provenance.revision
                            && author.input.text != old.text)
                {
                    return Err(invalid(
                            weftext_messages::UiMessage::error_core_capture_draft_retired_capture_proof_identity_or_revision_differs(),
                        ));
                }
                entry.input.author = Some(CaptureRawAuthor {
                    text: author.input.text,
                    provenance: author.input.provenance,
                });
            } else {
                input.detached_raw_inputs.push(CaptureDetachedRawInput {
                    retirement_id: author.retirement_id,
                    reason: author.reason,
                    input: author.input.copy_only(),
                });
            }
        }
        for raw in retained {
            let proof = raw.author.as_ref().unwrap();
            if !input.detached_raw_inputs.iter().any(|entry| {
                entry.input.author.as_ref().is_some_and(|author| {
                    author.provenance.producer_id == proof.provenance.producer_id
                        && author.provenance.input_id == proof.provenance.input_id
                }) && entry.input.field_id == raw.field_id
                    && entry.input.value_path == raw.value_path
                    && entry.input.control_role == raw.control_role
            }) {
                input.detached_raw_inputs.push(CaptureDetachedRawInput {
                    retirement_id: fresh(),
                    reason: CaptureRawRetirement::Producer,
                    input: raw,
                });
            }
        }
        Ok(())
    }
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
impl CaptureLibrary {
    /// Shape validation is not original ownership proof. Capture retains its
    /// physical volume-GUID coordinate; file-save scope uses the original
    /// admission locator. A future consumer must qualify that conversion.
    fn validate_shape(&self) -> Result<()> {
        match self {
            Self::DeviceUnassigned => Ok(()),
            Self::Bound { binding } => file_save_scope::QualifiedLibrary {
                root: binding.root.clone(),
                root_identity: binding.root_identity.clone(),
                workspace_id: binding.workspace_id.clone().ok_or_else(|| {
                    invalid(UiMessage::error_core_capture_draft_qualified_library_owner_required())
                })?,
            }
            .validate(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CaptureScopeHeader {
    pub owner: CaptureOwner,
    pub library: CaptureLibrary,
}
/// A bounded view of the record's existing first two fields, not a second
/// persisted version or an author-body decoder. Routing precedes version checks.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct CaptureRecordPrefix {
    pub scope: CaptureScopeHeader,
    pub version: u32,
}
impl CaptureRecordPrefix {
    pub(crate) fn require_supported_version(&self) -> Result<()> {
        if self.version != CAPTURE_RECORD_VERSION {
            return Err(invalid(
                weftext_messages::UiMessage::error_capture_version_unsupported(
                    weftext_messages::params::ErrorCaptureVersionUnsupported {
                        actual: self.version.to_string(),
                        expected: CAPTURE_RECORD_VERSION.to_string(),
                    },
                ),
            ));
        }
        Ok(())
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureMutationKind {
    Create,
    Input,
    AuthorInput,
    Checkpoint,
    Target,
    Freeze,
    Advance,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureMutation {
    pub kind: CaptureMutationKind,
    pub mutation_id: String,
    pub fingerprint: String,
    pub resulting_revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureInputCut {
    Full,
    AuthorOnly,
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
    pub input_cut: CaptureInputCut,
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
/// Only input supplied to this draft's author controls. This projection has no
/// Schema candidates, template identity, target paths or frozen protected wire.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureAuthorRecovery {
    pub draft_id: String,
    pub revision: u64,
    pub input_revision: u64,
    pub input_cut: CaptureInputCut,
    pub last_mutation: CaptureMutation,
    pub texts: CaptureAuthorTexts,
    pub body_mode: Option<CaptureBodyMode>,
    pub raw_inputs: Vec<CaptureAuthorRawInput>,
    pub detached_raw_inputs: Vec<CaptureDetachedAuthorInput>,
    pub pending: Option<CapturePendingIdentity>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturePendingIdentity {
    pub operation_id: String,
    pub request_kind: String,
    pub input_revision: u64,
}
impl From<CaptureDraft> for CaptureAuthorRecovery {
    fn from(draft: CaptureDraft) -> Self {
        let input = draft.current_input;
        Self {
            draft_id: draft.draft_id,
            revision: draft.revision,
            input_revision: draft.input_revision,
            input_cut: draft.input_cut,
            last_mutation: draft.last_mutation,
            texts: input.author_texts,
            body_mode: input.author_body_mode,
            raw_inputs: input
                .raw_inputs
                .into_iter()
                .filter_map(CaptureRawInput::author_only)
                .collect(),
            detached_raw_inputs: input
                .detached_raw_inputs
                .into_iter()
                .filter_map(|entry| {
                    Some(CaptureDetachedAuthorInput {
                        retirement_id: entry.retirement_id,
                        reason: entry.reason,
                        input: entry.input.author_only()?,
                    })
                })
                .collect(),
            pending: draft.pending.map(|pending| CapturePendingIdentity {
                operation_id: pending.operation_id,
                request_kind: pending.request_kind,
                input_revision: pending.input_revision,
            }),
        }
    }
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
    Checkpoint {
        expected_revision: u64,
    },
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
    Advance {
        expected_revision: u64,
        operation_id: String,
        reset_input: bool,
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
    fn kind(&self) -> CaptureMutationKind {
        match self {
            Self::Checkpoint { .. } => CaptureMutationKind::Checkpoint,
            Self::Create { .. } => CaptureMutationKind::Create,
            Self::Input { .. } => CaptureMutationKind::Input,
            Self::Target { .. } => CaptureMutationKind::Target,
            Self::Freeze { .. } => CaptureMutationKind::Freeze,
            Self::Advance { .. } => CaptureMutationKind::Advance,
        }
    }

    fn fingerprint(&self) -> Result<String> {
        match self {
            Self::Create { input, .. } | Self::Input { input, .. } => input_valid(input)?,
            Self::Freeze { request, .. } => pending_valid(request, request.input_revision)?,
            _ => {}
        }
        match self {
            Self::Checkpoint { expected_revision } => {
                capture_fingerprint(&(expected_revision, "checkpoint"))
            }
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
                capture_location_key(Path::new(&target.root))?,
                capture_location_key(Path::new(&target.parent))?,
                &target.hosted_slot,
            )),
            Self::Input {
                expected_revision,
                input,
            } => full_input_fingerprint(*expected_revision, input),
            Self::Target {
                expected_revision,
                target,
            } => capture_fingerprint(&(
                expected_revision,
                "target",
                capture_location_key(Path::new(&target.root))?,
                capture_location_key(Path::new(&target.parent))?,
                &target.hosted_slot,
            )),
            Self::Freeze {
                expected_revision,
                request,
            } => capture_fingerprint(&(expected_revision, "freeze", request)),
            Self::Advance {
                expected_revision,
                operation_id,
                reset_input,
            } => capture_fingerprint(&(expected_revision, "advance", operation_id, reset_input)),
        }
    }
}
// Locations are hints, not capabilities. Windows separator spelling must not
// make a lost-response query differ from the retained root/parent hint.
fn capture_location_key(path: &Path) -> Result<String> {
    let path = display(&absolute_path(path)?);
    #[cfg(windows)]
    {
        Ok(path.replace('\\', "/"))
    }
    #[cfg(not(windows))]
    {
        Ok(path)
    }
}
fn capture_fingerprint(command: &impl Serialize) -> Result<String> {
    Ok(hash(&serde_json::to_vec(command)?))
}
fn full_input_fingerprint(expected: u64, input: &CaptureInput) -> Result<String> {
    Ok(hash(&operations::canonical_json_bytes(
        &serde_json::to_value((expected, "input", input))?,
    )?))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFlushReceipt {
    pub draft_id: String,
    pub record_revision: u64,
    pub input_revision: u64,
    pub input_cut: CaptureInputCut,
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

fn invalid(message: UiMessage) -> CoreError {
    CoreError::new("captureRecordUnavailable", message)
}
fn budget() -> CoreError {
    CoreError::new(
        "budget",
        UiMessage::error_core_capture_draft_capture_recovery_exceeds_bounded_record_or_owner_domain_budget(),
    )
}
fn canonical_id(id: &str) -> Result<()> {
    if Uuid::parse_str(id).is_ok_and(|id_value| id_value.hyphenated().to_string() == id) {
        Ok(())
    } else {
        Err(invalid(
            UiMessage::error_core_capture_draft_capture_identifiers_require_canonical_lowercase_u_u_i_ds(),
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
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_revision_exhausted(),
        ));
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
fn input_token_valid(id: &str) -> bool {
    !id.is_empty() && id.len() <= 512 && !id.chars().any(char::is_control)
}
fn raw_inputs_valid(
    raw: &[CaptureRawInput],
    detached: &[CaptureDetachedRawInput],
    bytes: &mut usize,
) -> Result<()> {
    if raw.len().saturating_add(detached.len()) > 1024 {
        return Err(budget());
    }
    let mut live = BTreeSet::new();
    let mut retired = BTreeSet::new();
    for entry in raw.iter().chain(detached.iter().map(|entry| &entry.input)) {
        if entry.value_path.len() > 32 {
            return Err(budget());
        }
        if !input_token_valid(&entry.field_id) || !input_token_valid(&entry.control_role) {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_raw_input_address_is_invalid()));
        }
        text_valid(&entry.text)?;
        add_input_bytes(bytes, entry.text.utf16.len() * 2)?;
        if let Some(author) = &entry.author {
            provenance_valid(&author.provenance)?;
            text_valid(&author.text)?;
            add_input_bytes(bytes, author.text.utf16.len() * 2)?;
        }
        for part in &entry.value_path {
            match part {
                weftext_data::ValuePath::Member { key } => add_input_bytes(bytes, key.len())?,
                weftext_data::ValuePath::Index { index }
                    if *index as u64 > MAX_CAPTURE_REVISION =>
                {
                    return Err(invalid(UiMessage::error_core_capture_draft_capture_path_index_not_exact_u_i_integer()));
                }
                _ => {}
            }
        }
    }
    for entry in raw {
        if !live.insert(serde_json::to_vec(&(
            &entry.field_id,
            &entry.value_path,
            &entry.control_role,
        ))?) {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_live_raw_input_address_is_duplicated()));
        }
    }
    for entry in detached {
        canonical_id(&entry.retirement_id)?;
        if !retired.insert(&entry.retirement_id) {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_retired_raw_identity_is_duplicated()));
        }
    }
    Ok(())
}
fn provenance_valid(provenance: &CaptureRawProvenance) -> Result<()> {
    canonical_id(&provenance.producer_id)?;
    if provenance.input_id.len() != 32
        || !provenance
            .input_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || provenance.revision > MAX_CAPTURE_REVISION
    {
        return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_author_provenance_is_invalid()));
    }
    Ok(())
}
fn author_input_valid(input: &CaptureAuthorInput) -> Result<()> {
    let mut bytes = 0usize;
    author_texts_valid(&input.texts, &mut bytes)?;
    if input
        .raw_inputs
        .len()
        .saturating_add(input.detached_raw_inputs.len())
        > 1024
    {
        return Err(budget());
    }
    // Borrow untrusted text; do not duplicate a potentially oversized request
    // just to construct the private exact/copy-only representation.
    let mut live = BTreeSet::new();
    let mut retired = BTreeSet::new();
    for entry in input
        .raw_inputs
        .iter()
        .chain(input.detached_raw_inputs.iter().map(|entry| &entry.input))
    {
        if entry.value_path.len() > 32 {
            return Err(budget());
        }
        if !input_token_valid(&entry.field_id) || !input_token_valid(&entry.control_role) {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_author_input_address_is_invalid()));
        }
        provenance_valid(&entry.provenance)?;
        text_valid(&entry.text)?;
        add_input_bytes(&mut bytes, entry.text.utf16.len() * 2)?;
        for part in &entry.value_path {
            match part {
                weftext_data::ValuePath::Member { key } => add_input_bytes(&mut bytes, key.len())?,
                weftext_data::ValuePath::Index { index }
                    if *index as u64 > MAX_CAPTURE_REVISION =>
                {
                    return Err(invalid(UiMessage::error_core_capture_draft_capture_path_index_not_exact_u_i_integer()));
                }
                _ => {}
            }
        }
    }
    for entry in &input.raw_inputs {
        if !live.insert(serde_json::to_vec(&(
            &entry.field_id,
            &entry.value_path,
            &entry.control_role,
        ))?) {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_author_input_address_is_duplicated()));
        }
    }
    for entry in &input.detached_raw_inputs {
        canonical_id(&entry.retirement_id)?;
        if !retired.insert(&entry.retirement_id) {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_retired_author_identity_is_duplicated()));
        }
    }
    Ok(())
}
fn author_texts_valid(texts: &CaptureAuthorTexts, bytes: &mut usize) -> Result<()> {
    let slots = [
        &texts.name,
        &texts.title,
        &texts.body,
        &texts.root_input,
        &texts.parent_input,
        &texts.parameters_text,
    ];
    if slots.iter().map(|slot| slot.len()).sum::<usize>() > 1024 {
        return Err(budget());
    }
    for slot in slots {
        let mut identities = BTreeSet::new();
        for author in slot {
            provenance_valid(&author.provenance)?;
            if !identities.insert((&author.provenance.producer_id, &author.provenance.input_id)) {
                return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_top_level_author_identity_is_duplicated()));
            }
            text_valid(&author.text)?;
            add_input_bytes(bytes, author.text.utf16.len() * 2)?;
        }
    }
    Ok(())
}
fn merge_author_texts(current: &mut CaptureAuthorTexts, next: CaptureAuthorTexts) -> Result<()> {
    for (current, next) in [
        (&mut current.name, next.name),
        (&mut current.title, next.title),
        (&mut current.body, next.body),
        (&mut current.root_input, next.root_input),
        (&mut current.parent_input, next.parent_input),
        (&mut current.parameters_text, next.parameters_text),
    ] {
        for author in next {
            if let Some(old) = current.iter_mut().find(|old| {
                old.provenance.producer_id == author.provenance.producer_id
                    && old.provenance.input_id == author.provenance.input_id
            }) {
                if author.provenance.revision < old.provenance.revision
                    || author.provenance.revision == old.provenance.revision
                        && author.text != old.text
                {
                    return Err(invalid(
                        weftext_messages::UiMessage::error_core_capture_draft_capture_author_proof_revision_differs_from_retained_input(),
                    ));
                }
                *old = author;
            } else {
                current.push(author);
            }
        }
    }
    Ok(())
}
fn input_valid(input: &CaptureInput) -> Result<()> {
    // A lower bound on JSON size rejects impossible Core inputs before hashing
    // or serializing a large typed value. Final canonical bytes remain decisive.
    let mut input_bytes = 0usize;
    author_texts_valid(&input.author_texts, &mut input_bytes)?;
    for text in [
        &input.name,
        &input.title,
        &input.body,
        &input.root_input,
        &input.parent_input,
        &input.parameters_text,
    ] {
        text_valid(text)?;
        add_input_bytes(&mut input_bytes, text.utf16.len() * 2)?;
    }
    if input.type_ids.len() > 32
        || input.fields.len() > 64
        || input.raw_inputs.len() + input.detached_raw_inputs.len() > 1024
    {
        return Err(budget());
    }
    let id_valid = input_token_valid;
    let mut types = BTreeSet::new();
    if input
        .type_ids
        .iter()
        .any(|id| !id_valid(id) || !types.insert(id))
    {
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_type_i_ds_invalid_or_duplicated(),
        ));
    }
    let mut fields = BTreeSet::new();
    if input
        .fields
        .iter()
        .any(|field| !id_valid(&field.field_id) || !fields.insert(&field.field_id))
    {
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_field_ids_invalid_or_duplicated(),
        ));
    }
    if input.template_id.as_ref().is_some_and(|id| !id_valid(id)) {
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_template_identity_invalid(),
        ));
    }
    let mut nodes = 0usize;
    for field in &input.fields {
        match &field.value_state {
            CaptureFieldValueState::Valid { value } => {
                native_valid(value, 0, &mut nodes, &mut input_bytes)?
            }
            CaptureFieldValueState::Incomplete { wire } => field_wire::validate(
                wire,
                &field.field_id,
                &input.raw_inputs,
                &mut nodes,
                &mut input_bytes,
            )?,
        }
    }
    raw_inputs_valid(
        &input.raw_inputs,
        &input.detached_raw_inputs,
        &mut input_bytes,
    )?;
    if let Some(basis) = &input.form_basis
        && (!basis.schema_ref.starts_with("weftext-config://")
            || !basis.schema_ref.ends_with("/schema")
            || basis.schema_ref.len() > 160
            || !digest_valid(&basis.semantic_digest))
    {
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_form_basis_invalid(),
        ));
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
                    return Err(invalid(
                        UiMessage::error_core_capture_draft_capture_native_members_duplicated(),
                    ));
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
    header.library.validate_shape()?;
    if let CaptureOwner::Actor { actor } = &header.owner
        && (actor.is_empty() || actor.len() > 256)
    {
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_actor_invalid(),
        ));
    }
    match &header.library {
        CaptureLibrary::DeviceUnassigned if header.owner != CaptureOwner::Local => {
            return Err(invalid(
                UiMessage::error_core_capture_draft_only_local_capture_can_device_unassigned(),
            ));
        }
        CaptureLibrary::Bound { binding }
            if !Path::new(&binding.root).is_absolute()
                || binding
                    .workspace_id
                    .as_deref()
                    .is_none_or(|id| canonical_id(id).is_err()) =>
        {
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_library_provenance_invalid(),
            ));
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
            UiMessage::error_core_capture_draft_frozen_capture_wire_request_binding_or_input_revision_invalid(),
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
    let prefix = read_capture_prefix(&mut std::io::Cursor::new(bytes))?;
    validate_capture_header_path(path, data_dir, &prefix.scope)?;
    prefix.require_supported_version()?;
    if bytes.len() > MAX_CAPTURE_RECORD_BYTES {
        return Err(budget());
    }
    let draft: CaptureDraft = serde_json::from_slice(bytes).map_err(|_| {
        invalid(UiMessage::error_core_capture_draft_capture_record_malformed_original_bytes_were_preserved())
    })?;
    if serde_json::to_vec(&draft)? != bytes {
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_record_not_exact_closed_canonical_encoding(
            ),
        ));
    }
    header_valid(&draft.scope)?;
    canonical_id(&draft.draft_id)?;
    canonical_id(&draft.last_mutation.mutation_id)?;
    if draft.scope != prefix.scope
        || draft.version != prefix.version
        || draft.revision > MAX_CAPTURE_REVISION
        || draft.revision == 0
        || draft.last_mutation.resulting_revision != draft.revision
        || !digest_valid(&draft.last_mutation.fingerprint)
        || draft.input_revision == 0
        || draft.input_revision > draft.revision
        || capture_record_path(data_dir, &draft.scope.owner, &draft.draft_id)? != path
    {
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_version_namespace_or_revision_invalid(),
        ));
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
                    return Err(invalid(
                        UiMessage::error_core_capture_draft_capture_parent_belongs_to_another_library(),
                    ));
                }
            }
            if target
                .hosted_slot
                .as_ref()
                .is_some_and(|slot| slot.is_empty() || slot.len() > 256)
            {
                return Err(invalid(
                    UiMessage::error_core_capture_draft_capture_approved_slot_invalid(),
                ));
            }
            if let Some(basis) = &draft.current_input.form_basis
                && basis.schema_ref
                    != format!(
                        "weftext-config://{}/schema",
                        binding.workspace_id.as_deref().unwrap()
                    )
            {
                return Err(invalid(
                    UiMessage::error_core_capture_draft_capture_schema_belongs_to_another_library(),
                ));
            }
        }
        _ => {
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_target_and_original_scope_differ(),
            ));
        }
    }
    if let Some(pending) = &draft.pending {
        if draft.target.is_none() {
            return Err(invalid(
                UiMessage::error_core_capture_draft_unassigned_capture_cannot_have_submission(),
            ));
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
            UiMessage::error_core_capture_draft_frozen_request_does_not_name_original_capture_target(),
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
#[derive(Clone, Copy, Debug)]
enum WireIdentityFailure {
    DuplicateMember,
    ExcessMembers,
    OperationMismatch,
    OperationMissing,
}
impl WireIdentityFailure {
    fn explanation(self) -> UiMessage {
        match self {
            Self::DuplicateMember => UiMessage::error_core_capture_draft_wire_member_duplicate(),
            Self::ExcessMembers => UiMessage::error_core_capture_draft_wire_members_exceeded(),
            Self::OperationMismatch => {
                UiMessage::error_core_capture_draft_wire_operation_mismatch()
            }
            Self::OperationMissing => UiMessage::error_core_capture_draft_wire_operation_missing(),
        }
    }
}
impl std::fmt::Display for WireIdentityFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A private Serde stop marker; the per-parse typed side channel below
        // consumes it before any diagnostic can leave this function.
        std::fmt::Debug::fmt(self, f)
    }
}
fn wire_identity(wire: &str, expected: &str) -> Result<WireIdentity> {
    struct Metadata<'a> {
        expected: &'a str,
        failure: &'a mut Option<WireIdentityFailure>,
    }
    impl Metadata<'_> {
        fn stop<E: serde::de::Error>(&mut self, failure: WireIdentityFailure) -> E {
            *self.failure = Some(failure);
            E::custom(failure)
        }
    }
    impl<'de> serde::de::Visitor<'de> for Metadata<'_> {
        type Value = WireIdentity;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a JSON object")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            mut self,
            mut map: M,
        ) -> std::result::Result<WireIdentity, M::Error> {
            let mut keys = BTreeSet::new();
            let mut found = false;
            let mut identity = WireIdentity {
                root: None,
                parent: None,
                resource: None,
                destination: None,
            };
            while let Some(key) = map.next_key::<String>()? {
                if keys.contains(&key) {
                    return Err(self.stop(WireIdentityFailure::DuplicateMember));
                }
                if keys.len() == 64 {
                    return Err(self.stop(WireIdentityFailure::ExcessMembers));
                }
                keys.insert(key.clone());
                match key.as_str() {
                    "operationId" => {
                        if map.next_value::<String>()? != self.expected {
                            return Err(self.stop(WireIdentityFailure::OperationMismatch));
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
                return Err(self.stop(WireIdentityFailure::OperationMissing));
            }
            Ok(identity)
        }
    }
    let mut failure = None;
    let mut decoder = serde_json::Deserializer::from_str(wire);
    let parsed = serde::de::Deserializer::deserialize_map(
        &mut decoder,
        Metadata {
            expected,
            failure: &mut failure,
        },
    );
    if let Some(failure) = failure {
        return Err(invalid(failure.explanation()));
    }
    let identity = parsed.map_err(|error| {
        invalid(
            UiMessage::error_core_capture_draft_frozen_capture_wire_or_original_identity_invalid(),
        )
        .diagnostic(DiagnosticOrigin::Json, error.to_string())
    })?;
    decoder.end().map_err(|error| {
        invalid(UiMessage::error_core_capture_draft_frozen_capture_wire_contains_trailing_data())
            .diagnostic(DiagnosticOrigin::Json, error.to_string())
    })?;
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
        .ok_or_else(|| invalid(UiMessage::error_core_capture_draft_invalid_capture_filename()))?;
    if capture_record_path(data_dir, &header.owner, id)? != path {
        return Err(invalid(
            UiMessage::error_core_capture_draft_capture_header_owner_differs_from_namespace(),
        ));
    }
    Ok(())
}

fn read_capture_prefix_byte(reader: &mut impl Read, prefix: &mut Vec<u8>) -> Result<u8> {
    if prefix.len() == MAX_CAPTURE_PREFIX_BYTES {
        return Err(budget());
    }
    let mut byte = [0];
    reader
        .read_exact(&mut byte)
        .map_err(|_| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_metadata_prefix_is_truncated()))?;
    prefix.push(byte[0]);
    Ok(byte[0])
}
fn read_capture_prefix_non_whitespace(reader: &mut impl Read, prefix: &mut Vec<u8>) -> Result<u8> {
    loop {
        let byte = read_capture_prefix_byte(reader, prefix)?;
        if !matches!(byte, b' ' | b'\r' | b'\n' | b'\t') {
            return Ok(byte);
        }
    }
}

/// Read only the first scope object and following numeric version, including
/// its terminating delimiter, never a byte of an author payload. Unsupported
/// versions remain routable metadata until a consumer selects their scope.
/// The same guarded reader remains in the caller's ownership through routing.
pub(crate) fn read_capture_prefix(reader: &mut (impl Read + Seek)) -> Result<CaptureRecordPrefix> {
    let original = reader.stream_position()?;
    reader.seek(SeekFrom::Start(0))?;
    let result = (|| {
        let mut prefix = Vec::new();
        let mut quoted = false;
        let mut escaped = false;
        let mut depth = 0usize;
        let mut value_start = None;
        let scope = loop {
            let b = read_capture_prefix_byte(reader, &mut prefix)?;
            if value_start.is_none() {
                if b == b':' && !quoted {
                    let head = std::str::from_utf8(&prefix[..prefix.len() - 1]).map_err(|_| {
                        invalid(UiMessage::error_core_capture_draft_invalid_capture_scope_header())
                    })?;
                    let key = head
                        .trim_start_matches([' ', '\r', '\n', '\t'])
                        .strip_prefix('{')
                        .ok_or_else(|| {
                            invalid(
                                UiMessage::error_core_capture_draft_capture_scope_must_first_field(
                                ),
                            )
                        })?
                        .trim_matches(|c: char| matches!(c, ' ' | '\r' | '\n' | '\t'));
                    if serde_json::from_str::<String>(key).ok().as_deref() != Some("scope") {
                        return Err(invalid(
                            UiMessage::error_core_capture_draft_capture_scope_must_first_field(),
                        ));
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
                        return Err(invalid(
                            UiMessage::error_core_capture_draft_capture_scope_object_missing(),
                        ));
                    }
                    depth -= 1;
                    if depth == 0 {
                        let header: CaptureScopeHeader = serde_json::from_slice(&prefix[start..])
                            .map_err(|_| {
                            invalid(
                                UiMessage::error_core_capture_draft_capture_scope_header_invalid(),
                            )
                        })?;
                        header_valid(&header)?;
                        if serde_json::to_vec(&header)? != prefix[start..] {
                            return Err(invalid(
                                UiMessage::error_core_capture_draft_capture_scope_metadata_has_unknown_or_noncanonical_members(),
                            ));
                        }
                        break header;
                    }
                }
                _ if depth == 0 && !b.is_ascii_whitespace() => {
                    return Err(invalid(
                        UiMessage::error_core_capture_draft_capture_scope_must_object(),
                    ));
                }
                _ => {}
            }
        };
        if read_capture_prefix_non_whitespace(reader, &mut prefix)? != b','
            || read_capture_prefix_non_whitespace(reader, &mut prefix)? != b'"'
        {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_version_must_follow_scope()));
        }
        for expected in b"version\"" {
            if read_capture_prefix_byte(reader, &mut prefix)? != *expected {
                return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_version_must_be_the_second_field()));
            }
        }
        if read_capture_prefix_non_whitespace(reader, &mut prefix)? != b':' {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_version_metadata_is_invalid()));
        }
        let mut byte = read_capture_prefix_non_whitespace(reader, &mut prefix)?;
        let mut version_bytes = Vec::new();
        while byte.is_ascii_digit() {
            version_bytes.push(byte);
            byte = read_capture_prefix_byte(reader, &mut prefix)?;
        }
        if matches!(byte, b' ' | b'\r' | b'\n' | b'\t') {
            byte = read_capture_prefix_non_whitespace(reader, &mut prefix)?;
        }
        if !matches!(byte, b',' | b'}') {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_version_requires_an_unsigned_integer()));
        }
        let version = serde_json::from_slice::<u32>(&version_bytes)
            .map_err(|_| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_version_requires_an_unsigned_integer()))?;
        Ok(CaptureRecordPrefix { scope, version })
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
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_owner_differs(),
            ));
        }
        if self.actor.is_some() {
            match (&scope.library, &self.actor_library) {
                (CaptureLibrary::Bound { binding }, Some(actual)) if binding == actual => {}
                _ => {
                    return Err(invalid(
                        UiMessage::error_core_capture_draft_capture_actor_library_differs(),
                    ));
                }
            }
        }
        Ok(())
    }
    fn capture_read_selected(
        &self,
        id: &str,
        require_library: bool,
    ) -> Result<(CaptureDraft, Vec<u8>)> {
        let path = capture_record_path(&self.data_dir, &self.capture_owner(), id)?;
        let guard = ParentGuard::acquire(&self.backend, &path)?;
        let mut file = guard.open_read_guard_file()?;
        let prefix = read_capture_prefix(&mut file)?;
        if require_library {
            self.check_capture_scope(&prefix.scope)?;
        } else if prefix.scope.owner != self.capture_owner() {
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_owner_differs(),
            ));
        }
        prefix.require_supported_version()?;
        if file.metadata()?.len() > MAX_CAPTURE_RECORD_BYTES as u64 {
            return Err(budget());
        }
        let mut bytes = Vec::new();
        file.take(MAX_CAPTURE_RECORD_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let draft = validate_capture_record(&path, &self.data_dir, &bytes)?;
        if draft.scope != prefix.scope || draft.version != prefix.version {
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_scope_changed_during_read(),
            ));
        }
        Ok((draft, bytes))
    }
    fn capture_read(&self, id: &str) -> Result<(CaptureDraft, Vec<u8>)> {
        self.capture_read_selected(id, true)
    }
    pub fn read_capture_draft(&self, id: &str) -> Result<CaptureDraft> {
        self.capture_read(id).map(|(draft, _)| draft)
    }
    /// Independent local result delivery for an existing Capture surface. This
    /// does not prepare/execute an effect, change pending or reuse a Session.
    /// The original committed member helper owns Birth/materialized identity,
    /// current placement and post-open owner checks.
    pub fn open_capture_result(&mut self, id: &str, operation_id: &str) -> Result<OpenedDocument> {
        self.sync_local_owner()?;
        canonical_id(operation_id)?;
        let draft = self.read_capture_draft(id)?;
        let pending = draft
            .pending
            .as_ref()
            .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_has_no_original_result_to_deliver()))?;
        if pending.operation_id != operation_id || pending.request_kind != "capture" {
            return Err(invalid(
                weftext_messages::UiMessage::error_core_capture_draft_capture_result_differs_from_its_original_pending_request(),
            ));
        }
        let request: CaptureCreationRequest = serde_json::from_str(&pending.wire)?;
        if request.operation_id != operation_id
            || self
                .feature_operation_status(
                    operation_id,
                    &serde_json::json!({"kind":"capture","request":request}),
                )?
                .is_none()
        {
            return Err(invalid(
                weftext_messages::UiMessage::error_core_capture_draft_original_capture_request_has_no_matching_canonical_plan(),
            ));
        }
        let CaptureLibrary::Bound { binding } = &draft.scope.library else {
            return Err(invalid(
                weftext_messages::UiMessage::error_core_capture_draft_unassigned_capture_cannot_deliver_a_library_result(),
            ));
        };
        let target = draft
            .target
            .as_ref()
            .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_has_no_original_library_target()))?;
        if target.hosted_slot.is_some() {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_hosted_result_delivery_requires_its_actor_wrapper()));
        }
        let root = absolute_path(Path::new(&target.root_hint))?;
        let guard = ParentGuard::acquire(&self.backend, &root.join(".capture-result-root"))?;
        let workspace = self.require_workspace(&root)?;
        if guard.directory_identity()? != binding.root_identity
            || display(&guard.directory_coordinates()?) != binding.root
            || binding.workspace_id.as_deref() != Some(&workspace.workspace_id)
        {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_original_capture_library_is_unavailable()));
        }
        let plan = self
            .operations
            .get(operation_id)
            .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_original_capture_plan_is_unavailable()))?;
        let transaction = plan
            .transaction
            .as_ref()
            .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_original_capture_is_not_a_node_transaction()))?;
        if plan.workspace_id != binding.workspace_id
            || Path::new(&transaction.request.root) != root
            || !matches!(transaction.request.intent, OperationIntent::NewNode { .. })
            || transaction.births().len() != 1
        {
            return Err(invalid(
                weftext_messages::UiMessage::error_core_capture_draft_original_capture_result_belongs_to_another_kind_or_library(),
            ));
        }
        let owner = weftext_knowledge::NodeRef {
            workspace_id: workspace.workspace_id,
            node_id: transaction.births()[0].node_id.clone(),
        };
        // Formal P0 delivery port; no old-path or Session fallback is retained.
        self.reopen_original_operation_member(operation_id, &owner)
    }
    /// A trusted actor wrapper may return this owner's actual input even after
    /// its former library grant is retired. It cannot obtain the private record.
    pub fn read_capture_author_recovery(&self, id: &str) -> Result<CaptureAuthorRecovery> {
        self.capture_read_selected(id, false)
            .map(|(draft, _)| draft.into())
    }
    pub fn list_capture_drafts(&self) -> Result<Vec<CaptureDraft>> {
        self.list_capture_drafts_selected(true)
    }
    pub fn list_capture_author_recovery(&self) -> Result<Vec<CaptureAuthorRecovery>> {
        self.list_capture_drafts_selected(false)
            .map(|drafts| drafts.into_iter().map(Into::into).collect())
    }
    fn list_capture_drafts_selected(&self, require_library: bool) -> Result<Vec<CaptureDraft>> {
        let directory = self.draft_directory(CAPTURE_DIRECTORY);
        let guard = ParentGuard::acquire(&self.backend, &directory.join(".capture-list"))?;
        let (entries, complete) = guard.directory_entries(MAX_CAPTURE_RECORDS)?;
        if !complete {
            return Err(budget());
        }
        let mut selected_ids = Vec::new();
        let mut selected_bytes = 0u64;
        for (name, attrs) in entries {
            if !attrs.is_file() {
                return Err(invalid(
                    UiMessage::error_core_capture_draft_invalid_capture_namespace_member(),
                ));
            }
            let path = directory.join(name);
            let file_guard = ParentGuard::acquire(&self.backend, &path)?;
            let mut file = file_guard.open_read_guard_file()?;
            let prefix = read_capture_prefix(&mut file)?;
            validate_capture_header_path(&path, &self.data_dir, &prefix.scope)?;
            let selected = if require_library {
                self.check_capture_scope(&prefix.scope).is_ok()
            } else {
                prefix.scope.owner == self.capture_owner()
            };
            if selected {
                prefix.require_supported_version()?;
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
            let (draft, bytes) = self.capture_read_selected(&id, require_library)?;
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
    /// Read-only original-command proof without renewing the former library
    /// grant. The trusted hosted wrapper receives only the author's projection.
    pub fn capture_author_command_status(
        &self,
        id: &str,
        mutation_id: &str,
        command: &CaptureMutationCommand,
    ) -> Result<(String, CaptureAuthorRecovery)> {
        canonical_id(mutation_id)?;
        let (draft, _) = self.capture_read_selected(id, false)?;
        let fingerprint = command.fingerprint()?;
        let status = if draft.last_mutation.mutation_id != mutation_id {
            "supersededOrUnknown"
        } else if draft.last_mutation.kind == command.kind()
            && draft.last_mutation.fingerprint == fingerprint
        {
            "applied"
        } else {
            "differentCommand"
        };
        Ok((status.into(), draft.into()))
    }
    /// Hosted creation chooses private target coordinates on the server. A
    /// lost-response query reconstructs those original coordinates from this
    /// owner's retained draft, never from today's changed slot or client paths.
    pub fn capture_hosted_create_status(
        &self,
        id: &str,
        mutation_id: &str,
        slot: &str,
        input: CaptureInput,
    ) -> Result<(String, CaptureAuthorRecovery)> {
        canonical_id(mutation_id)?;
        input_valid(&input)?;
        let (draft, _) = self.capture_read_selected(id, false)?;
        let target = draft
            .target
            .as_ref()
            .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_has_no_original_target()))?;
        if target.hosted_slot.as_deref() != Some(slot) {
            return Err(invalid(
                weftext_messages::UiMessage::error_core_capture_draft_capture_original_slot_differs(
                ),
            ));
        }
        let parent = match &target.parent {
            CaptureParent::LibraryRoot => target.root_hint.clone(),
            CaptureParent::Managed { path_hint, .. } => path_hint.clone(),
        };
        let command = CaptureMutationCommand::Create {
            input,
            target: Some(CaptureMutationTarget {
                root: target.root_hint.clone(),
                parent,
                hosted_slot: Some(slot.into()),
            }),
        };
        self.capture_author_command_status(id, mutation_id, &command)
    }
    fn capture_write(&self, draft: &CaptureDraft, create: bool) -> Result<CaptureDraft> {
        self.capture_write_selected(draft, create, true)
    }
    fn capture_write_selected(
        &self,
        draft: &CaptureDraft,
        create: bool,
        require_library: bool,
    ) -> Result<CaptureDraft> {
        if require_library {
            self.check_capture_scope(&draft.scope)?;
        } else if draft.scope.owner != self.capture_owner() {
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_owner_differs(),
            ));
        }
        let path = capture_record_path(&self.data_dir, &draft.scope.owner, &draft.draft_id)?;
        let bytes = serde_json::to_vec(draft)?;
        validate_capture_record(&path, &self.data_dir, &bytes)?;
        let directory = self.draft_directory(CAPTURE_DIRECTORY);
        let guard = ParentGuard::acquire(&self.backend, &directory.join(".capture-budget"))?;
        let (entries, complete) = guard.directory_entries(MAX_CAPTURE_RECORDS)?;
        if !complete {
            return Err(budget());
        }
        let mut total = bytes.len() as u64;
        let mut count = 1usize;
        for (name, attrs) in entries {
            if !attrs.is_file() {
                return Err(invalid(
                    UiMessage::error_core_capture_draft_capture_namespace_contains_unsupported_member(),
                ));
            }
            let existing = directory.join(name);
            if existing == path {
                continue;
            }
            let existing_guard = ParentGuard::acquire(&self.backend, &existing)?;
            let mut file = existing_guard.open_read_guard_file()?;
            let prefix = read_capture_prefix(&mut file)?;
            let id = existing
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    invalid(UiMessage::error_core_capture_draft_invalid_capture_filename())
                })?;
            if capture_record_path(&self.data_dir, &prefix.scope.owner, id)? != existing
                || prefix.scope.owner != draft.scope.owner
            {
                return Err(invalid(
                    UiMessage::error_core_capture_draft_capture_owner_namespace_differs(),
                ));
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
        let stage_guard = ParentGuard::acquire(&self.backend, &stage)?;
        let mut file = stage_guard.open_file(true, true)?;
        #[cfg(all(test, windows))]
        hosted::definition_test_failpoint("captureStageCreated")?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(all(test, windows))]
        hosted::definition_test_failpoint("captureStageFlushed")?;
        if create {
            backend::publish_private_create(&self.backend, &stage, &path)?;
        } else {
            backend::publish_private_replace(&self.backend, &stage, &path)?;
        }
        let (readback, actual) = self.capture_read_selected(&draft.draft_id, require_library)?;
        if actual != bytes {
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_durable_readback_differs(),
            ));
        }
        Ok(readback)
    }
    pub fn capture_storage_status(&self) -> Result<CaptureStorageStatus> {
        let directory = self.capture_staging_directory();
        let guard =
            ParentGuard::acquire(&self.backend, &directory.join(".capture-staging-budget"))?;
        let (entries, complete) = guard.directory_entries(MAX_CAPTURE_RECORDS)?;
        if !complete {
            return Err(budget());
        }
        let mut status = CaptureStorageStatus {
            retained_stages: 0,
            retained_stage_bytes: 0,
        };
        for (name, attrs) in entries {
            if !attrs.is_file() {
                return Err(invalid(
                    UiMessage::error_core_capture_draft_invalid_capture_staging_member(),
                ));
            }
            let name = name.to_str().ok_or_else(|| {
                invalid(UiMessage::error_core_capture_draft_invalid_capture_staging_name())
            })?;
            let parts = name.split('.').collect::<Vec<_>>();
            if parts.len() != 3 || parts[2] != "stage" {
                return Err(invalid(
                    UiMessage::error_core_capture_draft_unrecognized_capture_staging_member(),
                ));
            }
            canonical_id(parts[0])?;
            canonical_id(parts[1])?;
            let path = directory.join(name);
            let file_guard = ParentGuard::acquire(&self.backend, &path)?;
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
                UiMessage::error_core_capture_draft_hosted_capture_creation_requires_qualified_library_target(),
            ));
        }
        let fingerprint = hash(&serde_json::to_vec(&(0u64, "create", &input))?);
        let path = capture_record_path(&self.data_dir, &self.capture_owner(), id)?;
        if self.backend.entry_exists(&path)? {
            let draft = self.read_capture_draft(id)?;
            return Self::capture_replay(draft, mutation_id, &fingerprint)?.ok_or_else(|| {
                invalid(UiMessage::error_core_capture_draft_capture_i_d_already_exists())
            });
        }
        let draft = CaptureDraft {
            scope: CaptureScopeHeader {
                owner: self.capture_owner(),
                library: CaptureLibrary::DeviceUnassigned,
            },
            version: CAPTURE_RECORD_VERSION,
            draft_id: id.into(),
            revision: 1,
            input_revision: 1,
            input_cut: CaptureInputCut::Full,
            last_mutation: CaptureMutation {
                kind: CaptureMutationKind::Create,
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
            capture_location_key(root)?,
            capture_location_key(parent)?,
            &hosted_slot,
        ))?);
        let path = capture_record_path(&self.data_dir, &self.capture_owner(), id)?;
        if self.backend.entry_exists(&path)? {
            let draft = self.read_capture_draft(id)?;
            return Self::capture_replay(draft, mutation_id, &fingerprint)?.ok_or_else(|| {
                invalid(UiMessage::error_core_capture_draft_capture_i_d_already_exists())
            });
        }
        let (library, target) = self.capture_target(root, parent, hosted_slot)?;
        let draft = CaptureDraft {
            scope: CaptureScopeHeader {
                owner: self.capture_owner(),
                library,
            },
            version: CAPTURE_RECORD_VERSION,
            draft_id: id.into(),
            revision: 1,
            input_revision: 1,
            input_cut: CaptureInputCut::Full,
            last_mutation: CaptureMutation {
                kind: CaptureMutationKind::Create,
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
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_mutation_id_belongs_to_different_input(
                ),
            ));
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
                UiMessage::error_core_capture_draft_capture_changed_in_another_writer_old_and_new_input_must_retained(),
            ));
        }
        Ok(Ok(draft))
    }
    fn capture_finish(
        &self,
        mut draft: CaptureDraft,
        mutation_id: &str,
        fingerprint: String,
        kind: CaptureMutationKind,
    ) -> Result<CaptureDraft> {
        draft.revision = next_revision(draft.revision)?;
        draft.last_mutation = CaptureMutation {
            kind,
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
        let fingerprint = full_input_fingerprint(expected, &input)?;
        let mut draft = match self.capture_edit(id, expected, mutation_id, &fingerprint)? {
            Ok(draft) => draft,
            Err(replay) => return Ok(replay),
        };
        draft.input_revision = next_revision(draft.input_revision)?;
        draft.current_input = input;
        draft.input_cut = CaptureInputCut::Full;
        self.capture_finish(draft, mutation_id, fingerprint, CaptureMutationKind::Input)
    }
    /// Persist only this author's raw input after resource grants or the former
    /// library disappear. No target, Schema candidate or pending wire is editable.
    pub fn update_capture_author_input(
        &mut self,
        id: &str,
        expected: u64,
        mutation_id: &str,
        input: CaptureAuthorInput,
    ) -> Result<CaptureAuthorRecovery> {
        canonical_id(mutation_id)?;
        author_input_valid(&input)?;
        let fingerprint = capture_fingerprint(&(expected, "authorInput", &input))?;
        let (mut draft, _) = self.capture_read_selected(id, false)?;
        if let Some(replay) = Self::capture_replay(draft.clone(), mutation_id, &fingerprint)? {
            return Ok(replay.into());
        }
        if draft.revision != expected {
            return Err(CoreError::new(
                "captureConflict",
                weftext_messages::UiMessage::error_core_capture_draft_capture_changed_before_owner_input_persistence_preserve_both_inputs(),
            ));
        }
        input.install(&mut draft.current_input)?;
        input_valid(&draft.current_input)?;
        draft.input_revision = next_revision(draft.input_revision)?;
        draft.input_cut = CaptureInputCut::AuthorOnly;
        draft.revision = next_revision(draft.revision)?;
        draft.last_mutation = CaptureMutation {
            kind: CaptureMutationKind::AuthorInput,
            mutation_id: mutation_id.into(),
            fingerprint,
            resulting_revision: draft.revision,
        };
        self.capture_write_selected(&draft, false, false)
            .map(Into::into)
    }
    /// Fence an explicitly disposed private-input proposal by advancing only
    /// this same record's CAS revision. This is not an author operation and
    /// cannot clear, cancel or settle the separate frozen creation effect.
    pub fn checkpoint_capture_input(
        &mut self,
        id: &str,
        expected: u64,
        mutation_id: &str,
    ) -> Result<CaptureAuthorRecovery> {
        canonical_id(mutation_id)?;
        let fingerprint = capture_fingerprint(&(expected, "checkpoint"))?;
        let (mut draft, _) = self.capture_read_selected(id, false)?;
        if let Some(replay) = Self::capture_replay(draft.clone(), mutation_id, &fingerprint)? {
            return Ok(replay.into());
        }
        if draft.revision != expected {
            return Err(CoreError::new(
                "captureConflict",
                weftext_messages::UiMessage::error_core_capture_draft_capture_changed_before_private_proposal_checkpoint_query_the_original_mutation_and(),
            ));
        }
        draft.revision = next_revision(draft.revision)?;
        draft.last_mutation = CaptureMutation {
            kind: CaptureMutationKind::Checkpoint,
            mutation_id: mutation_id.into(),
            fingerprint,
            resulting_revision: draft.revision,
        };
        self.capture_write_selected(&draft, false, false)
            .map(Into::into)
    }
    pub fn capture_author_mutation_status(
        &self,
        id: &str,
        mutation_id: &str,
        expected: u64,
        input: &CaptureAuthorInput,
    ) -> Result<(String, CaptureAuthorRecovery)> {
        canonical_id(mutation_id)?;
        author_input_valid(input)?;
        let (draft, _) = self.capture_read_selected(id, false)?;
        let fingerprint = capture_fingerprint(&(expected, "authorInput", input))?;
        let status = if draft.last_mutation.mutation_id != mutation_id {
            "supersededOrUnknown"
        } else if draft.last_mutation.kind == CaptureMutationKind::AuthorInput
            && draft.last_mutation.fingerprint == fingerprint
            && draft.revision == expected + 1
            && draft.input_cut == CaptureInputCut::AuthorOnly
        {
            "applied"
        } else {
            "differentCommand"
        };
        Ok((status.into(), draft.into()))
    }
    /// Pure metadata lookup for the original full-input mutation. A digest is
    /// only an association with this owner's record, never an execution grant.
    /// Unknown/superseded remains unknown; it does not certify non-acceptance.
    pub fn capture_input_mutation_status(
        &self,
        id: &str,
        mutation_id: &str,
        expected: u64,
        fingerprint: &str,
    ) -> Result<(String, CaptureAuthorRecovery)> {
        canonical_id(mutation_id)?;
        if expected == 0 || expected > MAX_CAPTURE_REVISION || !digest_valid(fingerprint) {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_original_full_input_mutation_identity_is_invalid()));
        }
        let (draft, _) = self.capture_read_selected(id, false)?;
        let status = if draft.last_mutation.mutation_id != mutation_id {
            "supersededOrUnknown"
        } else if draft.last_mutation.kind == CaptureMutationKind::Input
            && draft.last_mutation.fingerprint == fingerprint
            && draft.revision == expected + 1
            && draft.input_cut == CaptureInputCut::Full
        {
            "applied"
        } else {
            "differentCommand"
        };
        Ok((status.into(), draft.into()))
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
            capture_location_key(root)?,
            capture_location_key(parent)?,
            &hosted_slot,
        ))?);
        let mut draft = match self.capture_edit(id, expected, mutation_id, &fingerprint)? {
            Ok(draft) => draft,
            Err(replay) => return Ok(replay),
        };
        if draft.pending.is_some() {
            return Err(invalid(
                UiMessage::error_core_capture_draft_pending_capture_target_immutable(),
            ));
        }
        let (library, target) = self.capture_target(root, parent, hosted_slot)?;
        if let CaptureLibrary::Bound { binding: original } = &draft.scope.library
            && library
                != (CaptureLibrary::Bound {
                    binding: original.clone(),
                })
        {
            return Err(invalid(
                UiMessage::error_core_capture_draft_use_separate_capture_draft_for_another_library(
                ),
            ));
        }
        draft.scope.library = library;
        draft.target = Some(target);
        draft.input_revision = next_revision(draft.input_revision)?;
        self.capture_finish(draft, mutation_id, fingerprint, CaptureMutationKind::Target)
    }
    fn capture_target(
        &self,
        root: &Path,
        parent: &Path,
        hosted_slot: Option<String>,
    ) -> Result<(CaptureLibrary, CaptureTarget)> {
        let root = absolute_path(root)?;
        let parent = absolute_path(parent)?;
        let guard = ParentGuard::acquire(&self.backend, &root.join(".capture-root"))?;
        let parent_guard = ParentGuard::acquire(&self.backend, &parent.join(".capture-parent"))?;
        let workspace = self.require_workspace(&root)?;
        let binding = ActorLibraryBinding {
            root: display(&guard.directory_coordinates()?),
            root_identity: guard.directory_identity()?,
            workspace_id: Some(workspace.workspace_id),
        };
        if self.actor.is_some() && self.actor_library.as_ref() != Some(&binding) {
            return Err(invalid(
                UiMessage::error_core_capture_draft_trusted_capture_library_differs(),
            ));
        }
        let selected = if parent == root {
            CaptureParent::LibraryRoot
        } else {
            let selected = self.capture_navigation_target(&root, &parent)?;
            CaptureParent::Managed {
                reference: selected.target_ref.ok_or_else(|| {
                    invalid(
                        UiMessage::error_core_capture_draft_capture_parent_needs_admitted_stable_identity(),
                    )
                })?,
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
            || draft.input_cut != CaptureInputCut::Full
            || request.input_revision != draft.input_revision
        {
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_already_has_pending_input_or_frozen_revision_differs(),
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
        self.capture_finish(draft, mutation_id, fingerprint, CaptureMutationKind::Freeze)
    }
    pub fn resolve_capture_target(&self, id: &str) -> Result<CaptureResolvedTarget> {
        self.resolve_capture_target_record(&self.read_capture_draft(id)?)
    }
    /// Explicitly leave a terminal original attempt. The journal is consulted
    /// here; a client success/error flag can never release an unknown attempt.
    /// A later current input cut is preserved even when reset was requested.
    pub fn advance_capture_submission(
        &mut self,
        id: &str,
        expected: u64,
        mutation_id: &str,
        operation_id: &str,
        reset_input: bool,
    ) -> Result<CaptureDraft> {
        canonical_id(operation_id)?;
        let fingerprint = capture_fingerprint(&(expected, "advance", operation_id, reset_input))?;
        let mut draft = match self.capture_edit(id, expected, mutation_id, &fingerprint)? {
            Ok(draft) => draft,
            Err(replay) => return Ok(replay),
        };
        let pending = draft.pending.as_ref().ok_or_else(|| {
            invalid(
                weftext_messages::UiMessage::error_core_capture_draft_no_original_capture_attempt(),
            )
        })?;
        if pending.operation_id != operation_id {
            return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_original_operation_differs()));
        }
        let committed = if self.actor.is_some() {
            let slot = draft
                .target
                .as_ref()
                .and_then(|target| target.hosted_slot.as_deref())
                .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_original_actor_slot_is_unavailable()))?;
            let status = self
                .actor_feature_status(slot, operation_id)?
                .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_original_operation_is_still_unknown()))?;
            if !matches!(
                status.status.as_str(),
                "committed" | "cancelled" | "resolved"
            ) || !matches!(
                status.intent_kind.as_str(),
                "newNode" | "templateNode" | "unknown"
            ) {
                return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_original_operation_is_not_terminal()));
            }
            status.status == "committed"
        } else if let Some(plan) = self.operations.get(operation_id) {
            let target = draft
                .target
                .as_ref()
                .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_target_is_unavailable()))?;
            let transaction = plan
                .transaction
                .as_ref()
                .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_operation_is_not_a_new_note_transaction()))?;
            if transaction.request.root != target.root_hint
                || !matches!(transaction.request.intent, OperationIntent::NewNode { .. })
            {
                return Err(invalid(
                    weftext_messages::UiMessage::error_core_capture_draft_capture_original_operation_belongs_to_another_target_or_kind(),
                ));
            }
            if !self.committed(operation_id)? && !self.resolved(operation_id)? {
                return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_original_operation_is_not_terminal()));
            }
            self.committed(operation_id)?
        } else {
            let resolution = self
                .resolution_record(operation_id)?
                .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_original_operation_is_still_unknown()))?;
            if !resolution.attempt.is_some_and(|attempt| {
                attempt.request_kind == pending.request_kind
                    && attempt.wire_digest == pending.wire_digest
            }) {
                return Err(invalid(weftext_messages::UiMessage::error_core_capture_draft_capture_original_fence_belongs_to_another_request()));
            }
            false
        };
        if committed && reset_input && draft.input_revision == pending.input_revision {
            // Retain the author's selected types/template; clear exactly the
            // consumed text/field cut, never newer edits or retired raw input.
            let current = &draft.current_input;
            draft.current_input = CaptureInput {
                type_ids: current.type_ids.clone(),
                template_id: current.template_id.clone(),
                form_basis: current.form_basis.clone(),
                body_mode: current.body_mode.clone(),
                detached_raw_inputs: current.detached_raw_inputs.clone(),
                ..Default::default()
            };
            draft.input_revision = next_revision(draft.input_revision)?;
        }
        draft.pending = None;
        self.capture_finish(
            draft,
            mutation_id,
            fingerprint,
            CaptureMutationKind::Advance,
        )
    }
    fn resolve_capture_target_record(&self, draft: &CaptureDraft) -> Result<CaptureResolvedTarget> {
        let target = draft.target.as_ref().ok_or_else(|| {
            invalid(UiMessage::error_core_capture_draft_select_actual_capture_library_first())
        })?;
        let CaptureLibrary::Bound { binding } = &draft.scope.library else {
            return Err(invalid(
                UiMessage::error_core_capture_draft_capture_not_bound_to_library(),
            ));
        };
        let root = absolute_path(Path::new(&target.root_hint))?;
        let guard = ParentGuard::acquire(&self.backend, &root.join(".capture-resolve"))?;
        let workspace = self.require_workspace(&root)?;
        if guard.directory_identity()? != binding.root_identity
            || workspace.workspace_id != binding.workspace_id.as_deref().unwrap()
            || display(&guard.directory_coordinates()?) != binding.root
        {
            return Err(invalid(
                UiMessage::error_core_capture_draft_original_capture_library_was_replaced_or_moved(
                ),
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
                let path = PathBuf::from(resolved.path.ok_or_else(|| {
                    invalid(
                        UiMessage::error_core_capture_draft_original_capture_parent_unavailable(),
                    )
                })?);
                // Both NodeRef and DirectoryRef resolve to their actual container.
                let parent_guard =
                    ParentGuard::acquire(&self.backend, &path.join(".capture-parent"))?;
                if hash(&serde_json::to_vec(&parent_guard.directory_identity()?)?)
                    != *identity_digest
                {
                    return Err(invalid(
                        UiMessage::error_core_capture_draft_original_capture_parent_container_was_replaced(),
                    ));
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
        self.capture_flush_selected(id, expected, input_revision, mutation_id, true)
    }
    pub fn capture_author_flush_receipt(
        &self,
        id: &str,
        expected: u64,
        input_revision: u64,
        mutation_id: &str,
    ) -> Result<CaptureFlushReceipt> {
        self.capture_flush_selected(id, expected, input_revision, mutation_id, false)
    }
    fn capture_flush_selected(
        &self,
        id: &str,
        expected: u64,
        input_revision: u64,
        mutation_id: &str,
        require_library: bool,
    ) -> Result<CaptureFlushReceipt> {
        let (draft, bytes) = self.capture_read_selected(id, require_library)?;
        if require_library && draft.input_cut != CaptureInputCut::Full {
            return Err(invalid(
                weftext_messages::UiMessage::error_core_capture_draft_owner_only_recovery_is_not_a_complete_form_acknowledgement(),
            ));
        }
        if draft.revision != expected
            || draft.input_revision != input_revision
            || draft.last_mutation.mutation_id != mutation_id
        {
            return Err(CoreError::new(
                "captureConflict",
                UiMessage::error_core_capture_draft_capture_changed_before_close_no_close_permit_was_issued(),
            ));
        }
        Ok(CaptureFlushReceipt {
            draft_id: draft.draft_id,
            record_revision: draft.revision,
            input_revision: draft.input_revision,
            input_cut: draft.input_cut,
            last_mutation_id: draft.last_mutation.mutation_id,
            pending_operation_id: draft.pending.map(|pending| pending.operation_id),
            record_digest: hash(&bytes),
        })
    }
}

#[cfg(all(test, windows))]
#[path = "capture_draft/tests.rs"]
mod tests;

#[cfg(test)]
mod wire_boundary_tests {
    use super::*;

    const OPERATION: &str = "00000000-0000-4000-8000-000000000001";

    #[test]
    fn typed_metadata_failures_never_escape_as_raw_serde_diagnostics() {
        for (wire, explanation) in [
            (
                format!(r#"{{"operationId":"{OPERATION}","operationId":"{OPERATION}"}}"#),
                UiMessage::error_core_capture_draft_wire_member_duplicate(),
            ),
            (
                format!(r#"{{"operationId":"{OPERATION}","opaque":0,"opaque":1}}"#),
                UiMessage::error_core_capture_draft_wire_member_duplicate(),
            ),
            (
                r#"{"operationId":"other"}"#.into(),
                UiMessage::error_core_capture_draft_wire_operation_mismatch(),
            ),
            (
                r#"{"body":"\ud800\u0000"}"#.into(),
                UiMessage::error_core_capture_draft_wire_operation_missing(),
            ),
        ] {
            let original = wire.clone();
            let error = wire_identity(&wire, OPERATION).err().unwrap();
            assert_eq!(error.code, "captureRecordUnavailable");
            assert_eq!(error.explanation, explanation);
            assert!(error.diagnostics.is_empty());
            assert!(error.causes.as_slice().is_empty());
            assert_eq!(wire, original);
        }
    }

    #[test]
    fn exact_member_budget_stops_before_excess_author_payload_and_does_not_leak_parse_state() {
        let mut members = vec![format!(r#""operationId":"{OPERATION}""#)];
        for index in 0..63 {
            members.push(format!(r#""opaque{index}":0"#));
        }
        let boundary = format!("{{{}}}", members.join(","));
        assert!(wire_identity(&boundary, OPERATION).is_ok());
        // The 65th member's value is deliberately malformed. The original
        // bounded metadata reader must stop before attempting to read it.
        let excess = format!("{{{},\"overflow\":", members.join(","));
        let error = wire_identity(&excess, OPERATION).err().unwrap();
        assert_eq!(
            error.explanation,
            UiMessage::error_core_capture_draft_wire_members_exceeded()
        );
        assert!(error.diagnostics.is_empty());
        assert!(wire_identity(&boundary, OPERATION).is_ok());
        let missing = wire_identity("{}", OPERATION).err().unwrap();
        assert_eq!(
            missing.explanation,
            UiMessage::error_core_capture_draft_wire_operation_missing()
        );
    }

    #[test]
    fn ignored_original_payload_preserves_lone_utf16_nul_and_nested_duplicate_members() {
        let wire = format!(
            r#"{{"operationId":"{OPERATION}","root":"original-root","parent":"original-parent","body":"\ud800\u0000\udc00","opaque":{{"same":"\ud800","same":1}}}}"#
        );
        let original = wire.clone();
        let identity = wire_identity(&wire, OPERATION).unwrap();
        assert_eq!(identity.root.as_deref(), Some("original-root"));
        assert_eq!(identity.parent.as_deref(), Some("original-parent"));
        assert_eq!(wire, original);
        // A generic Value decoder would reject this author's original string;
        // it is deliberately not the wire-identity admission contract.
        assert!(serde_json::from_str::<serde_json::Value>(&wire).is_err());
    }

    #[test]
    fn genuine_json_syntax_type_and_trailing_errors_keep_attributed_original_diagnostics() {
        for wire in [
            r#"{"operationId":"#,
            r#"{"operationId":7}"#,
            r#"{"operationId":"00000000-0000-4000-8000-000000000001","root":7}"#,
            "[]",
            r#"{"operationId":"00000000-0000-4000-8000-000000000001"} trailing"#,
        ] {
            let error = wire_identity(wire, OPERATION).err().unwrap();
            assert_eq!(error.code, "captureRecordUnavailable");
            assert_eq!(error.diagnostics.len(), 1);
            assert_eq!(error.diagnostics[0].origin, DiagnosticOrigin::Json);
            assert!(!error.diagnostics[0].text.is_empty());
            assert!(error.causes.as_slice().is_empty());
        }
    }
}
