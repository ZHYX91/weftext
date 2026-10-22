//! Typed navigation preparation keeps the original identity reservation and
//! canonical Plan fingerprint, wrapped by the existing actor operation record.
use super::*;
use weftext_messages::UiMessage;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HostedNavigationIntent {
    AdmitDirectory {
        path: String,
    },
    CreateTag {
        display_name: String,
        parent_ref: Option<String>,
        assignment: Option<crate::TagAssignment>,
        base_id: String,
    },
    RenameTag {
        reference: String,
        display_name: String,
        base_id: String,
    },
    MoveTag {
        reference: String,
        #[serde(deserialize_with = "weftext_data::deserialize_required_tag_parent")]
        parent_ref: Option<String>,
        base_id: String,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct NavigationFeature {
    root: String,
    intent: HostedNavigationIntent,
    /// Preserve the exact author fingerprint when reading real pre-hierarchy
    /// actor records. Historical outcomes do not authorize installing v1.
    #[serde(default)]
    original_author_intent: Option<serde_json::Value>,
}
impl NavigationFeature {
    pub(super) fn kind(&self) -> FeatureIntentKind {
        match self.intent {
            HostedNavigationIntent::AdmitDirectory { .. } => FeatureIntentKind::AdmitDirectory,
            HostedNavigationIntent::CreateTag { .. } => FeatureIntentKind::CreateTag,
            HostedNavigationIntent::RenameTag { .. } => FeatureIntentKind::RenameTag,
            HostedNavigationIntent::MoveTag { .. } => FeatureIntentKind::MoveTag,
        }
    }
    fn canonical(&self) -> serde_json::Value {
        if let Some(original) = &self.original_author_intent {
            return original.clone();
        }
        if let HostedNavigationIntent::CreateTag {
            display_name,
            base_id,
            ..
        } = &self.intent
        {
            return serde_json::json!({"kind":"createTag","root":self.root,"displayName":display_name,"baseId":base_id});
        }
        self.current_canonical()
    }
    fn current_canonical(&self) -> serde_json::Value {
        match &self.intent {
            HostedNavigationIntent::AdmitDirectory { path } => {
                serde_json::json!({"kind":"admitDirectory","root":self.root,"path":path})
            }
            HostedNavigationIntent::CreateTag {
                display_name,
                parent_ref,
                assignment,
                base_id,
            } => {
                serde_json::json!({"kind":"createTag","root":self.root,"displayName":display_name,"parentRef":parent_ref,"baseId":base_id,"assignment":assignment})
            }
            HostedNavigationIntent::RenameTag {
                reference,
                display_name,
                base_id,
            } => {
                serde_json::json!({"kind":"renameTag","root":self.root,"tagRef":reference,"displayName":display_name,"baseId":base_id})
            }
            HostedNavigationIntent::MoveTag {
                reference,
                parent_ref,
                base_id,
            } => {
                serde_json::json!({"kind":"moveTag","root":self.root,"tagRef":reference,"parentRef":parent_ref,"baseId":base_id})
            }
        }
    }
    pub(super) fn matches_plan(&self, core: &Core, operation: &str) -> Result<bool> {
        core.feature_operation_status(operation, &self.canonical())
            .map(|p| p.is_some())
    }
    pub(super) fn target(&self, core: &Core, operation: &str) -> Result<String> {
        core.original_navigation_target(operation, &self.canonical())
    }
    fn prepare(&self, core: &mut Core, operation: &str) -> Result<NavigationPreparation> {
        match &self.intent {
            HostedNavigationIntent::AdmitDirectory { path } => {
                core.prepare_directory_admission(&self.root, path, operation)
            }
            HostedNavigationIntent::CreateTag {
                display_name,
                parent_ref,
                assignment,
                base_id,
            } => core.prepare_create_tag_with_assignment(
                &self.root,
                display_name,
                parent_ref.as_deref(),
                base_id,
                assignment.clone(),
                operation,
            ),
            HostedNavigationIntent::RenameTag {
                reference,
                display_name,
                base_id,
            } => core.prepare_rename_tag(&self.root, reference, display_name, base_id, operation),
            HostedNavigationIntent::MoveTag {
                reference,
                parent_ref,
                base_id,
            } => core.prepare_move_tag(
                &self.root,
                reference,
                parent_ref.as_deref(),
                base_id,
                operation,
            ),
        }
    }
}

impl Core {
    /// The host separately authorizes the exact directory or shared tag role.
    /// Persist the actor binding before calling the original specialized
    /// planner. Retry/status never reruns a partially prepared allocation.
    pub fn actor_feature_prepare_navigation(
        &mut self,
        resource: &str,
        operation: &str,
        fingerprint: &str,
        root: impl AsRef<Path>,
        mut intent: HostedNavigationIntent,
        preview: serde_json::Value,
    ) -> Result<ActorFeatureStatus> {
        valid_resource(resource)?;
        valid_fingerprint(fingerprint)?;
        self.actor_library_for_root(root.as_ref())?;
        let root = display(&absolute_path(root.as_ref())?);
        if let HostedNavigationIntent::AdmitDirectory { path } = &mut intent {
            *path = display(&absolute_path(Path::new(path))?);
        }
        let mut navigation = NavigationFeature {
            root,
            intent,
            original_author_intent: None,
        };
        navigation.original_author_intent = Some(navigation.current_canonical());
        let bytes = serde_json::to_vec(&navigation)?;
        if bytes.len() > 256 * 1024 || !preview.is_object() {
            return Err(CoreError::new(
                "invalidIntent",
                UiMessage::error_core_hosted_navigation_navigation_requires_bounded_typed_input_and_author_preview(),
            ));
        }
        check_size(&serde_json::to_vec(&preview)?)?;
        if let Some(record) = self.actor_feature_record(resource, operation)?
            && !(record.cancelled && record.fingerprint.is_empty())
            && record.feature.as_ref().unwrap().navigation.as_ref() != Some(&navigation)
        {
            return Err(conflict());
        }
        if let Some(status) = self.actor_feature_retry(resource, operation, fingerprint)? {
            return Ok(status);
        }
        let record = ActorOperation {
            scope: self.actor_input_scope_for_root(Path::new(&navigation.root))?,
            library: self.actor_library.clone(),
            actor: self.actor.clone().unwrap(),
            resource: resource.into(),
            fingerprint: fingerprint.into(),
            core_operation_id: fresh(),
            cancelled: false,
            field: None,
            feature: Some(FeatureOperation {
                preview,
                docx_import: false,
                import_losses: None,
                intent_kind: Some(navigation.kind()),
                destination_slot: None,
                navigation: Some(navigation.clone()),
            }),
            definition: None,
            save_definition_reservations: vec![],
            history: None,
            owner_backup: None,
        };
        journal::private_create(
            &self.backend,
            &self.actor_record(operation)?,
            &self.actor_operation_bytes(&record)?,
        )?;
        navigation.prepare(self, &record.core_operation_id)?;
        self.actor_feature_status(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_navigation_missing_original_navigation_operation(),
                )
            })
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_navigation_tests.rs"]
mod tests;
