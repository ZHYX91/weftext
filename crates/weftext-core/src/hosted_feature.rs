//! Durable actor intent and reviewed preview around the existing Core writer.
//! The host authorizes participants and filters this trusted result for delivery.
use super::*;
use std::collections::BTreeMap;
use weftext_messages::UiMessage;
#[path = "hosted_structure.rs"]
mod structure;
pub use structure::{ActorStructureNode, ActorStructureReceipt, FeatureTreeWitness};
#[path = "hosted_navigation.rs"]
mod navigation;
pub use navigation::HostedNavigationIntent;
use navigation::NavigationFeature;
#[path = "hosted_prepared_read.rs"]
mod prepared_read;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorFeatureStatus {
    pub operation_id: String,
    pub resource: String,
    pub status: String,
    pub plan_id: Option<String>,
    /// Host author preview plus fixed safe import losses, never a copied Core preview.
    pub preview: serde_json::Value,
    pub intent_kind: String,
    /// Stable refs allocated by the original Core plan, never filesystem paths.
    pub resource_refs: Vec<String>,
    /// Original allocated or continued identities, never guessed from a title.
    pub node_refs: Vec<weftext_knowledge::NodeRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub navigation_ref: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FeatureIntentKind {
    UpdateFiles,
    ImportDocx,
    ImportNative,
    ImportBibliography,
    ImportRecognized,
    ConfigureStyle,
    ImportResource,
    NewNode,
    RenameNode,
    MoveNode,
    CopyNode,
    TemplateNode,
    TrashNode,
    RestoreNode,
    AdmitDirectory,
    CreateTag,
    RenameTag,
    MoveTag,
}
impl FeatureIntentKind {
    fn from_intent(intent: &OperationIntent) -> Result<Self> {
        match intent {
            OperationIntent::UpdateFiles { .. } => Ok(Self::UpdateFiles),
            OperationIntent::ImportDocx { .. } | OperationIntent::ImportDocxWithOptions { .. } => {
                Ok(Self::ImportDocx)
            }
            OperationIntent::ImportNative { .. } => Ok(Self::ImportNative),
            OperationIntent::ImportBibliography { .. } => Ok(Self::ImportBibliography),
            OperationIntent::ImportRecognized { .. } => Ok(Self::ImportRecognized),
            OperationIntent::ConfigureStyle { .. } => Ok(Self::ConfigureStyle),
            OperationIntent::ImportResource { .. } => Ok(Self::ImportResource),
            OperationIntent::NewNode { .. } => Ok(Self::NewNode),
            OperationIntent::RenameNode { .. } => Ok(Self::RenameNode),
            OperationIntent::MoveNode { .. } => Ok(Self::MoveNode),
            OperationIntent::CopyNode { .. } => Ok(Self::CopyNode),
            OperationIntent::TemplateNode { .. } => Ok(Self::TemplateNode),
            OperationIntent::TrashNode { .. } => Ok(Self::TrashNode),
            OperationIntent::RestoreNode { .. } => Ok(Self::RestoreNode),
            _ => Err(CoreError::new(
                "unsupportedIntent",
                UiMessage::error_core_hosted_feature_hosted_reviewed_features_require_supported_author_or_structure_intent(),
            )),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::UpdateFiles => "updateFiles",
            Self::ImportDocx => "importDocx",
            Self::ImportNative => "importNative",
            Self::ImportBibliography => "importBibliography",
            Self::ImportRecognized => "importRecognized",
            Self::ConfigureStyle => "configureStyle",
            Self::ImportResource => "importResource",
            Self::NewNode => "newNode",
            Self::RenameNode => "renameNode",
            Self::MoveNode => "moveNode",
            Self::CopyNode => "copyNode",
            Self::TemplateNode => "templateNode",
            Self::TrashNode => "trashNode",
            Self::RestoreNode => "restoreNode",
            Self::AdmitDirectory => "admitDirectory",
            Self::CreateTag => "createTag",
            Self::RenameTag => "renameTag",
            Self::MoveTag => "moveTag",
        }
    }
    fn has_losses(self) -> bool {
        matches!(
            self,
            Self::ImportDocx
                | Self::ImportNative
                | Self::ImportBibliography
                | Self::ImportRecognized
                | Self::CopyNode
                | Self::TemplateNode
        )
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FeatureOperation {
    preview: serde_json::Value,
    #[serde(default)]
    docx_import: bool,
    #[serde(default)]
    import_losses: Option<Vec<SafeFeatureLoss>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    intent_kind: Option<FeatureIntentKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    destination_slot: Option<HostedDestination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    navigation: Option<NavigationFeature>,
}
impl FeatureOperation {
    pub(super) fn is_empty_fence(&self) -> bool {
        self.preview.is_null()
            && !self.docx_import
            && self.import_losses.is_none()
            && self.intent_kind.is_none()
            && self.destination_slot.is_none()
            && self.navigation.is_none()
    }
    fn kind(&self) -> FeatureIntentKind {
        self.intent_kind.unwrap_or(if self.docx_import {
            FeatureIntentKind::ImportDocx
        } else {
            FeatureIntentKind::UpdateFiles
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OutputClaim {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) library: Option<ActorLibraryBinding>,
    pub(super) actor: String,
    pub(super) fingerprint: String,
}

/// Only finite, safe loss meanings are retained by the hosted review. Private
/// paths, author values and source diagnostics remain in the original Core Plan.
/// This is an authorization projection of producer facts, not a translation of
/// error text. Presentation is always generated from the single message catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum SafeFeatureLoss {
    #[serde(rename = "word-layout")]
    WordLayout,
    #[serde(rename = "word-list-numbering")]
    WordListNumbering,
    #[serde(rename = "word-format-revision")]
    WordFormatRevision,
    #[serde(rename = "word-comment-range")]
    WordCommentRange,
    #[serde(rename = "word-comment-unlocated")]
    WordCommentUnlocated,
    #[serde(rename = "word-nested-revision")]
    WordNestedRevision,
    #[serde(rename = "word-link")]
    WordLink,
    #[serde(rename = "word-image-format")]
    WordImageFormat,
    #[serde(rename = "word-image-layout")]
    WordImageLayout,
    #[serde(rename = "word-image-missing")]
    WordImageMissing,
    #[serde(rename = "word-field")]
    WordField,
    #[serde(rename = "word-element")]
    WordElement,
    #[serde(rename = "word-block")]
    WordBlock,
    #[serde(rename = "word-table-layout")]
    WordTableLayout,
    #[serde(rename = "docx-import-loss")]
    DocxImportLoss,
    #[serde(rename = "generatedBibliographyNotInstalled")]
    GeneratedBibliographyNotInstalled,
    #[serde(rename = "resourceModuleContextPreserved")]
    ResourceModuleContextPreserved,
    #[serde(rename = "settingsNotApplied")]
    SettingsNotApplied,
    #[serde(rename = "unregisteredFileOmitted")]
    UnregisteredFileOmitted,
    #[serde(rename = "content-import-loss")]
    ContentImportLoss,
    #[serde(rename = "structure-copy-loss")]
    StructureCopyLoss,
}
impl SafeFeatureLoss {
    fn code(self) -> &'static str {
        match self {
            Self::WordLayout => "word-layout",
            Self::WordListNumbering => "word-list-numbering",
            Self::WordFormatRevision => "word-format-revision",
            Self::WordCommentRange => "word-comment-range",
            Self::WordCommentUnlocated => "word-comment-unlocated",
            Self::WordNestedRevision => "word-nested-revision",
            Self::WordLink => "word-link",
            Self::WordImageFormat => "word-image-format",
            Self::WordImageLayout => "word-image-layout",
            Self::WordImageMissing => "word-image-missing",
            Self::WordField => "word-field",
            Self::WordElement => "word-element",
            Self::WordBlock => "word-block",
            Self::WordTableLayout => "word-table-layout",
            Self::DocxImportLoss => "docx-import-loss",
            Self::GeneratedBibliographyNotInstalled => "generatedBibliographyNotInstalled",
            Self::ResourceModuleContextPreserved => "resourceModuleContextPreserved",
            Self::SettingsNotApplied => "settingsNotApplied",
            Self::UnregisteredFileOmitted => "unregisteredFileOmitted",
            Self::ContentImportLoss => "content-import-loss",
            Self::StructureCopyLoss => "structure-copy-loss",
        }
    }
    fn explanation(self) -> UiMessage {
        match self {
            Self::WordLayout => UiMessage::loss_core_hosted_feature_word_layout(),
            Self::WordListNumbering => UiMessage::loss_core_hosted_feature_word_list_numbering(),
            Self::WordFormatRevision => UiMessage::loss_core_hosted_feature_word_format_revision(),
            Self::WordCommentRange => UiMessage::loss_core_hosted_feature_word_comment_target(),
            Self::WordCommentUnlocated => UiMessage::loss_core_hosted_feature_word_comment_target(),
            Self::WordNestedRevision => UiMessage::loss_core_hosted_feature_word_nested_revision(),
            Self::WordLink => UiMessage::loss_core_hosted_feature_word_link(),
            Self::WordImageFormat => UiMessage::loss_core_hosted_feature_word_image_format(),
            Self::WordImageLayout => UiMessage::loss_core_hosted_feature_word_image_layout(),
            Self::WordImageMissing => UiMessage::loss_core_hosted_feature_word_image_missing(),
            Self::WordField => UiMessage::loss_core_hosted_feature_word_field(),
            Self::WordElement => UiMessage::loss_core_hosted_feature_word_structure(),
            Self::WordBlock => UiMessage::loss_core_hosted_feature_word_structure(),
            Self::WordTableLayout => UiMessage::loss_core_hosted_feature_word_table_layout(),
            Self::DocxImportLoss => UiMessage::loss_core_hosted_feature_docx_import(),
            Self::GeneratedBibliographyNotInstalled => {
                UiMessage::loss_core_hosted_feature_generated_bibliography()
            }
            Self::ResourceModuleContextPreserved => {
                UiMessage::loss_core_hosted_feature_resource_module_context()
            }
            Self::SettingsNotApplied => UiMessage::loss_core_hosted_feature_settings(),
            Self::UnregisteredFileOmitted => {
                UiMessage::loss_core_hosted_feature_unregistered_files()
            }
            Self::ContentImportLoss => UiMessage::loss_core_hosted_feature_content_import(),
            Self::StructureCopyLoss => UiMessage::loss_core_hosted_feature_structure_copy(),
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicFeatureLoss {
    code: SafeFeatureLoss,
    #[serde(flatten)]
    description: ErrorDescription,
    product_diagnostics: Vec<ErrorDescription>,
}
fn public_feature_losses(losses: &[SafeFeatureLoss]) -> Result<serde_json::Value> {
    Ok(serde_json::to_value(
        losses
            .iter()
            .map(|loss| PublicFeatureLoss {
                code: *loss,
                description: loss.explanation().into(),
                product_diagnostics: Vec::new(),
            })
            .collect::<Vec<_>>(),
    )?)
}
fn import_losses(preview: &OperationPreview) -> Vec<SafeFeatureLoss> {
    let mut losses = BTreeMap::new();
    for loss in &preview.losses {
        let safe = match loss.code.as_str() {
            "word-layout" => SafeFeatureLoss::WordLayout,
            "word-list-numbering" => SafeFeatureLoss::WordListNumbering,
            "word-format-revision" => SafeFeatureLoss::WordFormatRevision,
            "word-comment-range" => SafeFeatureLoss::WordCommentRange,
            "word-comment-unlocated" => SafeFeatureLoss::WordCommentUnlocated,
            "word-nested-revision" => SafeFeatureLoss::WordNestedRevision,
            "word-link" => SafeFeatureLoss::WordLink,
            "word-image-format" => SafeFeatureLoss::WordImageFormat,
            "word-image-layout" => SafeFeatureLoss::WordImageLayout,
            "word-image-missing" => SafeFeatureLoss::WordImageMissing,
            "word-field" => SafeFeatureLoss::WordField,
            "word-element" => SafeFeatureLoss::WordElement,
            "word-block" => SafeFeatureLoss::WordBlock,
            "word-table-layout" => SafeFeatureLoss::WordTableLayout,
            _ => SafeFeatureLoss::DocxImportLoss,
        };
        losses.insert(safe.code(), safe);
    }
    losses.into_values().collect()
}
fn feature_losses(preview: &OperationPreview, kind: FeatureIntentKind) -> Vec<SafeFeatureLoss> {
    if kind == FeatureIntentKind::ImportDocx {
        return import_losses(preview);
    }
    if preview.losses.is_empty() {
        return Vec::new();
    }
    if matches!(
        kind,
        FeatureIntentKind::ImportNative
            | FeatureIntentKind::ImportBibliography
            | FeatureIntentKind::ImportRecognized
    ) {
        let mut losses = BTreeMap::new();
        for loss in &preview.losses {
            let safe = match loss.code.as_str() {
                "generatedBibliographyNotInstalled" => {
                    SafeFeatureLoss::GeneratedBibliographyNotInstalled
                }
                "resourceModuleContextPreserved" => SafeFeatureLoss::ResourceModuleContextPreserved,
                "settingsNotApplied" => SafeFeatureLoss::SettingsNotApplied,
                "unregisteredFileOmitted" => SafeFeatureLoss::UnregisteredFileOmitted,
                _ => SafeFeatureLoss::ContentImportLoss,
            };
            losses.insert(safe.code(), safe);
        }
        losses.into_values().collect()
    } else {
        vec![SafeFeatureLoss::StructureCopyLoss]
    }
}

pub(super) fn conflict() -> CoreError {
    CoreError::new(
        "operationConflict",
        UiMessage::error_core_hosted_feature_original_reviewed_actor_resource_or_intent_differs(),
    )
}

pub(crate) fn valid_resource(resource: &str) -> Result<()> {
    if resource.is_empty() || resource.len() > 256 || resource.chars().any(char::is_control) {
        return Err(CoreError::new(
            "invalidResource",
            UiMessage::error_core_hosted_definition_bounded_resource_identity_required(),
        ));
    }
    Ok(())
}

pub(super) fn valid_fingerprint(fingerprint: &str) -> Result<()> {
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(CoreError::new(
            "invalidIntent",
            UiMessage::error_core_hosted_definition_lowercase_s_h_a256_author_intent_fingerprint_required(),
        ));
    }
    Ok(())
}

impl Core {
    /// Read only an exact original creation member. The host separately grants
    /// this operation's result read; the ActorOperation proves its ownership.
    pub fn actor_feature_open_original_node(
        &mut self,
        resource: &str,
        operation: &str,
        owner: &weftext_knowledge::NodeRef,
    ) -> Result<OpenedDocument> {
        let record = self
            .actor_feature_record(resource, operation)?
            .ok_or_else(conflict)?;
        let status = self
            .actor_feature_status(resource, operation)?
            .ok_or_else(conflict)?;
        if status.status != "committed" || !status.node_refs.iter().any(|node| node == owner) {
            return Err(conflict());
        }
        self.reopen_original_operation_member(&record.core_operation_id, owner)
    }
    /// Describe shared-role writes from the immutable original actor Plan.
    /// Hosts must authorize these effects separately from destination slots.
    pub fn actor_feature_shared_writes(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Vec<String>> {
        let record = self
            .actor_feature_record(resource, operation)?
            .ok_or_else(conflict)?;
        let transaction = self
            .operations
            .get(&record.core_operation_id)
            .and_then(|p| p.transaction.as_ref())
            .ok_or_else(conflict)?;
        let root = Path::new(&transaction.request.root);
        let mut roles = std::collections::BTreeSet::new();
        for change in self.prepared_operation_files(&record.core_operation_id)? {
            if change.before.as_ref() == Some(&change.after) {
                continue;
            }
            let path = Path::new(&change.path);
            if path == root.join(".weftext-schema.json") {
                roles.insert("schema.write".to_owned());
            }
            if path == root.join(".weftext-tags.json") {
                let before = change
                    .before
                    .as_ref()
                    .map(|b| crate::directory_identity::decode_historical_tag_document(b))
                    .transpose()?;
                let after =
                    crate::directory_identity::decode_historical_tag_document(&change.after)?;
                let old = before
                    .iter()
                    .flat_map(|d| &d.tags)
                    .map(|t| (&t.tag_id, t))
                    .collect::<std::collections::BTreeMap<_, _>>();
                for tag in &after.tags {
                    if let Some(previous) = old.get(&tag.tag_id) {
                        if previous.display_name != tag.display_name {
                            roles.insert("tags.rename".to_owned());
                        }
                        if previous.parent_tag_id != tag.parent_tag_id {
                            roles.insert("tags.move".to_owned());
                        }
                    } else {
                        roles.insert("tags.create".to_owned());
                    }
                }
                let after_ids = after
                    .tags
                    .iter()
                    .map(|t| &t.tag_id)
                    .collect::<std::collections::BTreeSet<_>>();
                if old.keys().any(|id| !after_ids.contains(id)) {
                    return Err(CoreError::new(
                        "unsupportedIntent",
                        UiMessage::error_core_directory_identity_tag_deletion_not_supported(),
                    ));
                }
                roles.insert("tags.read".to_owned());
            }
        }
        Ok(roles.into_iter().collect())
    }

    fn actor_output_bytes(claim: &OutputClaim) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(claim)?;
        if bytes.len() > 4096 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_hosted_feature_output_claim_exceeds_durable_read_budget(),
            ));
        }
        Ok(bytes)
    }

    fn actor_output_path(&self, operation: &str) -> Result<PathBuf> {
        // Canonical names prevent alternate UUID spellings from bypassing a fence.
        valid_operation_id(operation)?;
        if Uuid::parse_str(operation).unwrap().to_string() != operation {
            return Err(CoreError::new(
                "invalidOperationId",
                UiMessage::error_core_hosted_feature_canonical_u_u_i_d_required(),
            ));
        }
        self.actor_record(operation)?;
        Ok(self
            .draft_directory("operations")
            .join("outputs")
            .join(format!("{operation}.json")))
    }

    /// A durable output request fence contains no generated bytes or paths.
    /// A host may deliver only its retained artifact after an existing claim.
    pub fn actor_output_fingerprint(&self, operation: &str) -> Result<Option<String>> {
        let file = match File::open(self.actor_output_path(operation)?) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let mut bytes = vec![];
        file.take(4097).read_to_end(&mut bytes)?;
        if bytes.len() > 4096 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_hosted_feature_output_request_record_exceeds_budget(),
            ));
        }
        let claim: OutputClaim = serde_json::from_slice(&bytes)?;
        self.check_actor_library(&claim.library)?;
        if self.actor.as_ref() != Some(&claim.actor) {
            return Err(conflict());
        }
        valid_fingerprint(&claim.fingerprint)?;
        Ok(Some(claim.fingerprint))
    }

    /// true claims a new original request; false is the same existing request.
    /// No restart, failure, or cache expiry releases this original request ID.
    pub fn actor_output_claim(&mut self, operation: &str, fingerprint: &str) -> Result<bool> {
        valid_fingerprint(fingerprint)?;
        let path = self.actor_output_path(operation)?;
        let directory = path.parent().unwrap();
        fs::create_dir_all(directory)?;
        let _guard = ParentGuard::acquire(&directory.join(".scope"))?;
        if let Some(original) = self.actor_output_fingerprint(operation)? {
            if original != fingerprint {
                return Err(conflict());
            }
            return Ok(false);
        }
        journal::private_create(
            &path,
            &Self::actor_output_bytes(&OutputClaim {
                library: self.actor_library.clone(),
                actor: self.actor.clone().unwrap(),
                fingerprint: fingerprint.into(),
            })?,
        )?;
        Ok(true)
    }

    pub(super) fn actor_feature_record(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorOperation>> {
        valid_resource(resource)?;
        let Some(record) =
            self.read_actor_operation(operation, resource, MAX_SOURCE_BYTES + 4096)?
        else {
            return Ok(None);
        };
        if self.actor.as_ref() != Some(&record.actor)
            || record.resource != resource
            || record.feature.is_none()
            || record.field.is_some()
            || record.history.is_some()
            || record.owner_backup.is_some()
            || record.definition.is_some()
            || !record.save_definition_reservations.is_empty()
            || record
                .feature
                .as_ref()
                .is_some_and(|f| f.docx_import != (f.kind() == FeatureIntentKind::ImportDocx))
            || record.feature.as_ref().is_some_and(|f| {
                matches!(
                    f.kind(),
                    FeatureIntentKind::AdmitDirectory
                        | FeatureIntentKind::CreateTag
                        | FeatureIntentKind::RenameTag
                        | FeatureIntentKind::MoveTag
                ) != f.navigation.is_some()
                    || f.navigation
                        .as_ref()
                        .is_some_and(|n| n.kind() != f.kind() || f.destination_slot.is_some())
            })
        {
            return Err(conflict());
        }
        valid_operation_id(&record.core_operation_id)?;
        Ok(Some(record))
    }

    pub fn actor_feature_status(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorFeatureStatus>> {
        let Some(mut record) = self.actor_feature_record(resource, operation)? else {
            return Ok(None);
        };
        if let Some(HostedDestination::UniqueChild(binding)) =
            record.feature.as_ref().unwrap().destination_slot.as_ref()
            && binding.external_operation_id() != operation
        {
            return Err(conflict());
        }
        let detail = match self.operation_details(&record.core_operation_id) {
            Ok(detail) => {
                let transaction = self.operations[&record.core_operation_id]
                    .transaction
                    .as_ref()
                    .unwrap();
                let feature = record.feature.as_ref().unwrap();
                let valid_intent = if let Some(navigation) = &feature.navigation {
                    matches!(
                        transaction.request.intent,
                        OperationIntent::UpdateFiles { .. }
                    ) && transaction.hosted_destination.is_none()
                        && navigation.matches_plan(self, &record.core_operation_id)?
                } else {
                    FeatureIntentKind::from_intent(&transaction.request.intent)
                        .is_ok_and(|kind| kind == feature.kind())
                        && transaction.hosted_destination == feature.destination_slot
                };
                if !valid_intent {
                    return Err(conflict());
                }
                Some(detail)
            }
            Err(error)
                if error.code == "operationMissing"
                    && !self.operations.contains_key(&record.core_operation_id)
                    && !self
                        .journal_path(&record.core_operation_id, "plan")
                        .exists() =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        let status = match detail.as_ref().map(|p| p.status.as_str()) {
            Some("committed") => "committed",
            Some("recoveryPending") => "recoveryPending",
            Some("resolved") => {
                let resolution: serde_json::Value = serde_json::from_slice(&fs::read(
                    self.journal_path(&record.core_operation_id, "resolution"),
                )?)?;
                if resolution["action"] == "cancelPrepared" {
                    "cancelled"
                } else {
                    "resolved"
                }
            }
            Some("prepared") if !record.cancelled => "prepared",
            Some("prepared") | None if record.cancelled => "cancelled",
            None => "notCommitted",
            _ => {
                return Err(CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_feature_unknown_original_feature_operation_state(),
                ));
            }
        };
        if record.feature.as_ref().unwrap().kind().has_losses()
            && record.feature.as_ref().unwrap().import_losses.is_none()
            && let Some(detail) = &detail
        {
            // Freeze the safe semantic facts before first delivery, including a
            // crash between Core prepare and this write. A UI language change
            // changes presentation, never the reviewed Plan or this record.
            let kind = record.feature.as_ref().unwrap().kind();
            record.feature.as_mut().unwrap().import_losses = Some(feature_losses(detail, kind));
            private_replace(
                &self.actor_record(operation)?,
                &self.actor_operation_bytes(&record)?,
            )?;
        }
        let feature = record.feature.unwrap();
        let kind = feature.kind();
        let resource_refs = if matches!(
            kind,
            FeatureIntentKind::ImportResource
                | FeatureIntentKind::ConfigureStyle
                | FeatureIntentKind::ImportRecognized
        ) && matches!(status, "prepared" | "committed" | "recoveryPending")
        {
            detail
                .as_ref()
                .map(|p| {
                    p.resource_map
                        .values()
                        .cloned()
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };
        let navigation_ref = if detail.is_some() {
            feature
                .navigation
                .as_ref()
                .map(|n| n.target(self, &record.core_operation_id))
                .transpose()?
        } else {
            None
        };
        let mut preview = feature.preview;
        if kind.has_losses() {
            let object = preview.as_object_mut().ok_or_else(conflict)?;
            object.insert(
                "losses".into(),
                public_feature_losses(&feature.import_losses.unwrap_or_default())?,
            );
        }
        let node_refs = detail
            .as_ref()
            .and_then(|p| {
                self.operations
                    .get(&record.core_operation_id)?
                    .workspace_id
                    .as_ref()
                    .map(|workspace| {
                        p.node_map
                            .values()
                            .chain(p.new_node_id.iter())
                            .cloned()
                            .collect::<std::collections::BTreeSet<_>>()
                            .into_iter()
                            .map(|node_id| weftext_knowledge::NodeRef {
                                workspace_id: workspace.clone(),
                                node_id,
                            })
                            .collect()
                    })
            })
            .unwrap_or_default();
        Ok(Some(ActorFeatureStatus {
            operation_id: operation.into(),
            resource: resource.into(),
            status: status.into(),
            plan_id: detail.map(|p| p.plan_id),
            preview,
            intent_kind: if record.cancelled && record.fingerprint.is_empty() {
                "unknown".into()
            } else {
                kind.name().into()
            },
            resource_refs,
            node_refs,
            navigation_ref,
        }))
    }

    /// A retry reads only the original durable record and Core plan; no current
    /// source, new dependencies, or newly generated preview can replace it.
    pub fn actor_feature_retry(
        &self,
        resource: &str,
        operation: &str,
        fingerprint: &str,
    ) -> Result<Option<ActorFeatureStatus>> {
        valid_fingerprint(fingerprint)?;
        let Some(record) = self.actor_feature_record(resource, operation)? else {
            return Ok(None);
        };
        if !(record.cancelled && record.fingerprint.is_empty()) && record.fingerprint != fingerprint
        {
            return Err(conflict());
        }
        self.actor_feature_status(resource, operation)
    }

    /// Bind the original authenticated intent and author preview before Core
    /// prepares any filesystem participant. A crash or prepare failure in this
    /// gap stays notCommitted; its retry cannot regenerate a different plan.
    pub fn actor_feature_prepare(
        &mut self,
        resource: &str,
        external_operation_id: &str,
        fingerprint: &str,
        request: OperationRequest,
        preview: serde_json::Value,
        dependencies: &[String],
    ) -> Result<ActorFeatureStatus> {
        self.actor_feature_prepare_with_witnesses(
            resource,
            external_operation_id,
            fingerprint,
            request,
            preview,
            dependencies,
            &[],
        )
    }
    /// Persist original read-only dependencies in the same operation Plan.
    /// No witness becomes a writable snapshot basis. Absent dependencies use
    /// the existing predecision absence check, not a sustained absence lock.
    #[allow(clippy::too_many_arguments)]
    pub fn actor_feature_prepare_with_witnesses(
        &mut self,
        resource: &str,
        external_operation_id: &str,
        fingerprint: &str,
        request: OperationRequest,
        preview: serde_json::Value,
        dependencies: &[String],
        witnesses: &[&SnapshotWitness],
    ) -> Result<ActorFeatureStatus> {
        self.actor_feature_prepare_inner(
            resource,
            external_operation_id,
            fingerprint,
            request,
            preview,
            dependencies,
            witnesses,
            &[],
            None,
            None,
        )
    }
    /// A complete source-tree read capability is separate from source read and
    /// a copy action. The host must authorize it before capture_feature_tree.
    #[allow(clippy::too_many_arguments)]
    pub fn actor_feature_prepare_with_tree_witness(
        &mut self,
        resource: &str,
        external_operation_id: &str,
        fingerprint: &str,
        request: OperationRequest,
        preview: serde_json::Value,
        dependencies: &[String],
        witnesses: &[&SnapshotWitness],
        tree: &FeatureTreeWitness,
    ) -> Result<ActorFeatureStatus> {
        self.actor_feature_prepare_inner(
            resource,
            external_operation_id,
            fingerprint,
            request,
            preview,
            dependencies,
            witnesses,
            &[],
            Some(tree),
            None,
        )
    }
    /// Bind the separately approved destination slot to the original actor
    /// intent and Plan. Source operation anchor and destination are independent.
    /// transfer_witnesses carry explicitly authorized source inputs; namespace
    /// witnesses protect the decision but never grant transfer target reading.
    #[allow(clippy::too_many_arguments)]
    pub fn actor_feature_prepare_structure(
        &mut self,
        resource: &str,
        external_operation_id: &str,
        fingerprint: &str,
        request: OperationRequest,
        preview: serde_json::Value,
        dependencies: &[String],
        witnesses: &[&SnapshotWitness],
        transfer_witnesses: &[&SnapshotWitness],
        destination_slot: Option<&HostedDestination>,
        tree: Option<&FeatureTreeWitness>,
    ) -> Result<ActorFeatureStatus> {
        let kind = FeatureIntentKind::from_intent(&request.intent)?;
        if !matches!(
            kind,
            FeatureIntentKind::NewNode
                | FeatureIntentKind::RenameNode
                | FeatureIntentKind::MoveNode
                | FeatureIntentKind::CopyNode
                | FeatureIntentKind::TemplateNode
                | FeatureIntentKind::TrashNode
                | FeatureIntentKind::RestoreNode
        ) {
            return Err(CoreError::new(
                "unsupportedIntent",
                UiMessage::error_core_hosted_feature_only_structural_intents_bind_destination_slot(
                ),
            ));
        }
        if (kind == FeatureIntentKind::TrashNode) != destination_slot.is_none() {
            return Err(CoreError::new(
                "invalidIntent",
                UiMessage::error_core_hosted_feature_live_structural_output_requires_independently_approved_destination_slot(),
            ));
        }
        if let Some(destination) = destination_slot {
            valid_resource(destination.slot())?;
        }
        if let Some(HostedDestination::UniqueChild(binding)) = destination_slot
            && binding.external_operation_id() != external_operation_id
        {
            return Err(conflict());
        }
        self.actor_feature_prepare_inner(
            resource,
            external_operation_id,
            fingerprint,
            request,
            preview,
            dependencies,
            witnesses,
            transfer_witnesses,
            tree,
            destination_slot,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn actor_feature_prepare_inner(
        &mut self,
        resource: &str,
        external_operation_id: &str,
        fingerprint: &str,
        mut request: OperationRequest,
        preview: serde_json::Value,
        dependencies: &[String],
        witnesses: &[&SnapshotWitness],
        transfer_witnesses: &[&SnapshotWitness],
        tree: Option<&FeatureTreeWitness>,
        destination_slot: Option<&HostedDestination>,
    ) -> Result<ActorFeatureStatus> {
        self.actor_library_for_root(Path::new(&request.root))?;
        let intent_kind = FeatureIntentKind::from_intent(&request.intent)?;
        if let Some(record) = self.actor_feature_record(resource, external_operation_id)?
            && !(record.cancelled && record.fingerprint.is_empty())
            && (record.feature.as_ref().unwrap().kind() != intent_kind
                || record.feature.as_ref().unwrap().destination_slot.as_ref() != destination_slot)
        {
            return Err(conflict());
        }
        if let Some(status) =
            self.actor_feature_retry(resource, external_operation_id, fingerprint)?
        {
            return Ok(status);
        }
        if matches!(
            intent_kind,
            FeatureIntentKind::CopyNode | FeatureIntentKind::TemplateNode
        ) && tree.is_none()
        {
            return Err(CoreError::new(
                "basisRequired",
                UiMessage::error_core_hosted_feature_hosted_copy_and_template_require_explicitly_authorized_complete_tree_witness(),
            ));
        }
        if tree.is_some()
            && !matches!(
                intent_kind,
                FeatureIntentKind::CopyNode | FeatureIntentKind::TemplateNode
            )
        {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_hosted_feature_tree_witness_belongs_only_to_copy_or_template(
                ),
            ));
        }
        if !transfer_witnesses.is_empty() && tree.is_none() {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_hosted_feature_transfer_source_capabilities_belong_only_to_copy_or_template(),
            ));
        }
        let docx_import = matches!(
            request.intent,
            OperationIntent::ImportDocx { .. } | OperationIntent::ImportDocxWithOptions { .. }
        );
        if let OperationIntent::ImportDocxWithOptions {
            bytes,
            options,
            originals,
            conversion_losses,
            ..
        } = &request.intent
        {
            let extra = originals.iter().try_fold(bytes.len(), |total, original| {
                total.checked_add(original.bytes.len())
            });
            if originals.len() > 4
                || extra.is_none_or(|total| total > 16 * 1024 * 1024)
                || serde_json::to_vec(options)?.len() > 1024 * 1024
                || conversion_losses.len() > 128
                || serde_json::to_vec(conversion_losses)?.len() > 256 * 1024
            {
                return Err(CoreError::new(
                    "featureBudget",
                    UiMessage::error_core_hosted_feature_word_options_originals_and_conversion_losses_exceed_bounded_actor_import_budget(),
                ));
            }
        }
        let over_budget = match &mut request.intent {
            OperationIntent::UpdateFiles { changes } => {
                changes.is_empty()
                    || changes.len() > 64
                    || changes.iter().any(|c| c.path.len() > 64 * 1024)
                    || changes
                        .iter()
                        .try_fold(0usize, |total, change| {
                            total
                                .checked_add(change.before.as_ref().map_or(0, Vec::len))?
                                .checked_add(change.after.len())
                        })
                        .is_none_or(|bytes| bytes > 32 * 1024 * 1024)
            }
            OperationIntent::ImportNative {
                parent,
                name,
                bytes,
                allow_losses,
            }
            | OperationIntent::ImportBibliography {
                parent,
                name,
                bytes,
                allow_losses,
                ..
            }
            | OperationIntent::ImportRecognized {
                parent,
                name,
                bytes,
                allow_losses,
                ..
            }
            | OperationIntent::ImportDocx {
                parent,
                name,
                bytes,
                allow_losses,
            }
            | OperationIntent::ImportDocxWithOptions {
                parent,
                name,
                bytes,
                allow_losses,
                ..
            } => {
                // Preparing reports all losses. Acceptance belongs to commit of
                // the exact frozen plan, never to a pre-checked upload checkbox.
                *allow_losses = true;
                valid_name(name)?;
                if !preview.is_object() {
                    return Err(CoreError::new(
                        "invalidIntent",
                        UiMessage::error_core_hosted_feature_import_preview_must_object(),
                    ));
                }
                parent.len() > 64 * 1024 || bytes.is_empty() || bytes.len() > 16 * 1024 * 1024
            }
            OperationIntent::ConfigureStyle {
                base_id, source, ..
            } => {
                valid_operation_id(base_id)?;
                source.len() > MAX_SOURCE_BYTES
            }
            OperationIntent::ImportResource {
                node,
                name,
                bytes,
                manifest_base_id,
                resource_base_id,
            } => {
                valid_name(name)?;
                valid_operation_id(manifest_base_id)?;
                valid_operation_id(resource_base_id)?;
                node.len() > 64 * 1024 || bytes.len() > 32 * 1024 * 1024
            }
            OperationIntent::NewNode {
                parent,
                name,
                source,
            } => {
                valid_name(name)?;
                parent.len() > 64 * 1024
                    || source.as_ref().is_some_and(|s| s.len() > MAX_SOURCE_BYTES)
            }
            OperationIntent::RenameNode { source, name } => {
                valid_name(name)?;
                source.len() > 64 * 1024
            }
            OperationIntent::MoveNode { source, parent } => {
                source.len() > 64 * 1024 || parent.len() > 64 * 1024
            }
            OperationIntent::CopyNode {
                source,
                parent,
                name,
                allow_losses,
            } => {
                valid_name(name)?;
                *allow_losses = true;
                source.len() > 64 * 1024 || parent.len() > 64 * 1024
            }
            OperationIntent::TemplateNode {
                source,
                parent,
                name,
                instantiated_source,
                source_base_id,
            } => {
                valid_name(name)?;
                valid_operation_id(source_base_id)?;
                source.len() > 64 * 1024
                    || parent.len() > 64 * 1024
                    || instantiated_source.len() > MAX_SOURCE_BYTES
            }
            OperationIntent::TrashNode { source } => source.len() > 64 * 1024,
            OperationIntent::RestoreNode {
                trash_operation_id,
                node_id,
                parent,
                name,
            } => {
                valid_name(name)?;
                valid_operation_id(trash_operation_id)?;
                if let Some(id) = node_id {
                    valid_operation_id(id)?;
                }
                parent.len() > 64 * 1024
            }
            _ => {
                return Err(CoreError::new(
                    "unsupportedIntent",
                    UiMessage::error_core_hosted_feature_hosted_reviewed_features_require_one_of_supported_typed_intents(),
                ));
            }
        };
        if over_budget
            || dependencies.len() > 64
            || witnesses.len() > 2048
            || transfer_witnesses.len() > 64
            || request.root.len() > 64 * 1024
        {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_hosted_feature_hosted_review_exceeds_explicit_participant_or_byte_budget(),
            ));
        }
        for dependency in dependencies {
            valid_operation_id(dependency)?;
        }
        if intent_kind.has_losses() && !preview.is_object() {
            return Err(CoreError::new(
                "invalidIntent",
                UiMessage::error_core_hosted_feature_loss_bearing_preview_must_object(),
            ));
        }
        check_size(&serde_json::to_vec(&preview)?)?;
        let record = ActorOperation {
            scope: self.actor_input_scope_for_root(Path::new(&request.root))?,
            library: self.actor_library.clone(),
            actor: self.actor.clone().unwrap(),
            resource: resource.into(),
            fingerprint: fingerprint.into(),
            core_operation_id: fresh(),
            cancelled: false,
            field: None,
            feature: Some(FeatureOperation {
                preview,
                docx_import,
                import_losses: None,
                intent_kind: Some(intent_kind),
                destination_slot: destination_slot.cloned(),
                navigation: None,
            }),
            definition: None,
            save_definition_reservations: vec![],
            history: None,
            owner_backup: None,
        };
        journal::private_create(
            &self.actor_record(external_operation_id)?,
            &self.actor_operation_bytes(&record)?,
        )?;
        request.operation_id = record.core_operation_id.clone();
        let mut canonical = serde_json::json!({
            "kind":"hostedReviewedFeature","actor":record.actor,"resource":record.resource,"fingerprint":record.fingerprint,
        });
        if let Some(destination) = destination_slot {
            canonical["destination"] = serde_json::to_value(destination)?;
        }
        if tree.is_none() && destination_slot.is_none() {
            self.prepare_dependent_feature_operation_with_witnesses(
                request,
                &canonical,
                dependencies,
                witnesses,
            )?;
        } else {
            self.prepare_dependent_feature_operation_with_tree(
                request,
                &canonical,
                dependencies,
                witnesses,
                tree,
                transfer_witnesses,
                destination_slot,
            )?;
        }
        self.actor_feature_status(resource, external_operation_id)?
            .ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_feature_missing_original_feature_operation(),
                )
            })
    }

    pub fn actor_feature_commit(
        &mut self,
        resource: &str,
        operation: &str,
        plan_id: &str,
    ) -> Result<ActorFeatureStatus> {
        self.actor_feature_commit_with_losses(resource, operation, plan_id, false)
    }

    /// Import losses require an explicit choice at the exact original plan.
    /// This flag has no effect on review proposals or terminal receipts.
    pub fn actor_feature_commit_with_losses(
        &mut self,
        resource: &str,
        operation: &str,
        plan_id: &str,
        accept_losses: bool,
    ) -> Result<ActorFeatureStatus> {
        valid_operation_id(plan_id)?;
        let record = self
            .actor_feature_record(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    UiMessage::error_core_hosted_feature_prepare_original_reviewed_operation_first(
                    ),
                )
            })?;
        let status = self.actor_feature_status(resource, operation)?.unwrap();
        if status.plan_id.as_deref() != Some(plan_id) {
            return Err(CoreError::new(
                "planConflict",
                UiMessage::error_core_hosted_feature_commit_exact_original_reviewed_plan(),
            ));
        }
        // Committed receipts are idempotent. Resolved, cancelled, and installed
        // plans are never re-executed through this hosted wrapper.
        if status.status != "prepared" {
            return Ok(status);
        }
        if record.feature.as_ref().unwrap().kind().has_losses()
            && !accept_losses
            && status.preview["losses"]
                .as_array()
                .is_some_and(|losses| !losses.is_empty())
        {
            return Err(CoreError::new(
                "copyLoss",
                UiMessage::error_core_hosted_feature_explicitly_accept_original_import_losses_before_commit(),
            ));
        }
        self.execute_operation(&record.core_operation_id, plan_id)?;
        self.actor_feature_status(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_feature_missing_original_feature_operation(),
                )
            })
    }

    /// End an unknown or never-installed original proposal durably. Once the
    /// install marker exists, status/recovery remain authoritative and no
    /// tombstone may cancel or mask possibly committed filesystem effects.
    pub fn actor_feature_settle(
        &mut self,
        resource: &str,
        operation: &str,
    ) -> Result<ActorFeatureStatus> {
        let path = self.actor_record(operation)?;
        let mut record = match self.actor_feature_record(resource, operation)? {
            Some(record) => record,
            None => ActorOperation {
                scope: self.actor_attempt_input_scope()?,
                library: self.actor_library.clone(),
                actor: self.actor.clone().unwrap(),
                resource: resource.into(),
                fingerprint: String::new(),
                core_operation_id: fresh(),
                cancelled: true,
                field: None,
                feature: Some(FeatureOperation {
                    preview: serde_json::Value::Null,
                    docx_import: false,
                    import_losses: None,
                    intent_kind: None,
                    destination_slot: None,
                    navigation: None,
                }),
                definition: None,
                save_definition_reservations: vec![],
                history: None,
                owner_backup: None,
            },
        };
        if let Some(status) = self.actor_feature_status(resource, operation)? {
            if matches!(
                status.status.as_str(),
                "committed" | "recoveryPending" | "resolved" | "cancelled"
            ) {
                return Ok(status);
            }
            if status.status == "prepared" {
                // Resolving Core first prevents any direct original-plan replay,
                // including a crash before the actor tombstone is persisted.
                self.resolve_record(&record.core_operation_id, "cancelPrepared")?;
            }
            record.cancelled = true;
            private_replace(&path, &self.actor_operation_bytes(&record)?)?;
        } else {
            journal::private_create(&path, &self.actor_operation_bytes(&record)?)?;
        }
        self.actor_feature_status(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_feature_missing_settled_feature_operation(),
                )
            })
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_feature_tests.rs"]
mod tests;
