//! Future bundle slots resolve only original committed hosted import facts.
use super::*;
use operations::Mutation;
use weftext_knowledge::NodeRef;
use weftext_messages::UiMessage;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentImportMember {
    pub owner: NodeRef,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorContentImportReceipt {
    pub operation_id: String,
    pub nodes: Vec<ContentImportMember>,
}

fn content_import(intent: &OperationIntent) -> bool {
    matches!(
        intent,
        OperationIntent::ImportNative { .. }
            | OperationIntent::ImportBibliography { .. }
            | OperationIntent::ImportRecognized { .. }
    )
}
fn same_path(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b)
        .is_some_and(|relative| relative.as_os_str().is_empty())
}

impl Core {
    #[allow(clippy::too_many_arguments)]
    pub fn actor_feature_prepare_content_import(
        &mut self,
        resource: &str,
        operation: &str,
        fingerprint: &str,
        request: OperationRequest,
        preview: serde_json::Value,
        witnesses: &[&SnapshotWitness],
        destination_slot: &str,
    ) -> Result<ActorFeatureStatus> {
        if !content_import(&request.intent) {
            return Err(CoreError::new(
                "unsupportedIntent",
                UiMessage::error_core_hosted_content_import_content_import_requires_native_or_bibliography_producer(),
            ));
        }
        feature::valid_resource(destination_slot)?;
        self.actor_feature_prepare_inner(
            resource,
            operation,
            fingerprint,
            request,
            preview,
            &[],
            witnesses,
            &[],
            None,
            Some(&HostedDestination::fixed(destination_slot)),
        )
    }

    /// Symbolic original producer facts only. It does not scan the target or
    /// open a member body, nor does it grant read access to any returned member.
    fn content_import_members(&self, plan: &journal::Plan) -> Result<Vec<ContentImportMember>> {
        let transaction = plan.transaction.as_ref().ok_or_else(feature::conflict)?;
        let Mutation::CreateTree {
            target,
            births,
            existing_target: false,
            files,
            ..
        } = &transaction.mutation
        else {
            return Err(feature::conflict());
        };
        let workspace = plan.workspace_id.as_ref().ok_or_else(feature::conflict)?;
        let actual = self.read_births(&plan.operation_id)?;
        let mut members = Vec::new();
        let mut metadata = 0usize;
        for birth in births {
            if !is_canonical(Path::new(&birth.path))
                || !files
                    .iter()
                    .any(|file| file.path == birth.path && file.before.is_none())
                || actual
                    .iter()
                    .filter(|b| b.node_id == birth.node_id && &b.workspace_id == workspace)
                    .count()
                    != 1
            {
                return Err(CoreError::new(
                    "privateEvidence",
                    UiMessage::error_core_hosted_content_import_original_import_birth_evidence_unavailable(),
                ));
            }
            let relative =
                crate::history::history_relative_path(Path::new(&birth.path), Path::new(target))
                    .ok_or_else(feature::conflict)?;
            if relative.as_os_str().is_empty()
                || relative
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(feature::conflict());
            }
            let relative_path = display(&relative).replace('\\', "/");
            metadata = metadata
                .checked_add(relative_path.len() + birth.node_id.len() + workspace.len())
                .ok_or_else(feature::conflict)?;
            if members.len() == 5000 || metadata > MAX_SOURCE_BYTES {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_hosted_content_import_import_member_metadata_exceeds_budget(),
                ));
            }
            members.push(ContentImportMember {
                owner: NodeRef {
                    workspace_id: workspace.clone(),
                    node_id: birth.node_id.clone(),
                },
                relative_path,
            });
        }
        Ok(members)
    }

    pub fn actor_feature_content_import_receipt(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorContentImportReceipt>> {
        self.backend.check_path(&self.data_dir)?;
        let Some(record) = self.actor_feature_record(resource, operation)? else {
            return Ok(None);
        };
        let Some(plan) = self.operations.get(&record.core_operation_id) else {
            return Ok(None);
        };
        if !content_import(
            &plan
                .transaction
                .as_ref()
                .ok_or_else(feature::conflict)?
                .request
                .intent,
        ) {
            return Err(feature::conflict());
        }
        if !self.committed(&record.core_operation_id)? {
            return Ok(None);
        }
        Ok(Some(ActorContentImportReceipt {
            operation_id: operation.into(),
            nodes: self.content_import_members(plan)?,
        }))
    }

    /// Only original Plan/Birth symbols, including retained unavailable members.
    /// Actual consumers must separately qualify each selected member and report
    /// absence/replacement as unavailable or partial, never complete-empty.
    pub fn content_import_slot_members_at(
        &self,
        root: &Path,
        target: &Path,
        slot: &str,
    ) -> Result<Vec<ContentImportMember>> {
        self.backend.check_path(&self.data_dir)?;
        feature::valid_resource(slot)?;
        let root = absolute_path(root)?;
        let target = absolute_path(target)?;
        if crate::history::history_relative_path(&target, &root).is_none() {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_hosted_content_import_import_slot_outside_configured_root(),
            ));
        }
        let workspace = self.require_workspace(&root)?;
        let mut members = Vec::new();
        let mut producers = 0usize;
        let mut metadata = 0usize;
        for plan in self
            .operations
            .values()
            .map(|p| -> Result<_> {
                Ok(
                    if {
                        self.committed(&p.operation_id)?
                            && p.workspace_id.as_ref() == Some(&workspace.workspace_id)
                    } {
                        Some(p)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            let Some(transaction) = &plan.transaction else {
                continue;
            };
            if !transaction
                .hosted_destination
                .as_ref()
                .is_some_and(|d| d.is_fixed_slot(slot))
                || !content_import(&transaction.request.intent)
            {
                continue;
            }
            let Mutation::CreateTree {
                target: original,
                existing_target: false,
                ..
            } = &transaction.mutation
            else {
                return Err(feature::conflict());
            };
            if !same_path(Path::new(original), &target) {
                continue;
            }
            producers += 1;
            for member in self.content_import_members(plan)? {
                if members.iter().any(|prior: &ContentImportMember| {
                    prior.owner == member.owner && prior.relative_path == member.relative_path
                }) {
                    continue;
                }
                metadata = metadata
                    .checked_add(
                        member.relative_path.len()
                            + member.owner.node_id.len()
                            + member.owner.workspace_id.len(),
                    )
                    .ok_or_else(feature::conflict)?;
                if members.len() == 5000 || metadata > MAX_SOURCE_BYTES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_hosted_content_import_import_slot_members_exceed_their_budget(),
                    ));
                }
                members.push(member);
            }
        }
        if producers == 0 {
            let _guard = ParentGuard::acquire(&self.backend, &target)?;
            if self.backend.entry_exists(&target)? {
                return Err(CoreError::new(
                        "slotUnavailable",
                        UiMessage::error_core_hosted_content_import_unrelated_object_occupies_future_import_slot(),
                    ));
            }
        }
        members.sort_by(|a, b| {
            a.relative_path
                .cmp(&b.relative_path)
                .then(a.owner.node_id.cmp(&b.owner.node_id))
        });
        Ok(members)
    }

    /// Exact original bundle location only. Moving a member does not transfer
    /// this future-slot capability to its new address; configure that address
    /// and expected owner independently instead.
    pub fn qualified_content_import_node_at(
        &self,
        root: &Path,
        source_path: &Path,
        slot: &str,
        node_id: &str,
    ) -> Result<QualifiedQueryNode> {
        self.backend.check_path(&self.data_dir)?;
        feature::valid_resource(slot)?;
        valid_operation_id(node_id)?;
        let root = absolute_path(root)?;
        let path = absolute_path(source_path)?;
        let workspace = self.require_workspace(&root)?;
        let placement_guard = ParentGuard::acquire(&self.backend, &path)?;
        let placement_file = placement_guard.open_placement_file()?;
        let identity = stamp(&placement_file)?.identity;
        let mut owner = None;
        for plan in self
            .operations
            .values()
            .map(|p| -> Result<_> {
                Ok(
                    if {
                        self.committed(&p.operation_id)?
                            && p.workspace_id.as_ref() == Some(&workspace.workspace_id)
                    } {
                        Some(p)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            let Some(transaction) = &plan.transaction else {
                continue;
            };
            if !transaction
                .hosted_destination
                .as_ref()
                .is_some_and(|d| d.is_fixed_slot(slot))
                || !content_import(&transaction.request.intent)
            {
                continue;
            }
            let Mutation::CreateTree {
                target,
                births,
                existing_target: false,
                ..
            } = &transaction.mutation
            else {
                return Err(feature::conflict());
            };
            if !births
                .iter()
                .any(|b| b.node_id == node_id && same_path(Path::new(&b.path), &path))
                || !self.read_births(&plan.operation_id)?.iter().any(|birth| {
                    birth.node_id == node_id
                        && birth.workspace_id == workspace.workspace_id
                        && birth.identity == identity
                })
            {
                continue;
            }
            if self.content_import_members(plan)?.iter().any(|member| {
                member.owner.node_id == node_id
                    && same_path(&Path::new(target).join(&member.relative_path), &path)
            }) {
                let found = NodeRef {
                    workspace_id: workspace.workspace_id.clone(),
                    node_id: node_id.into(),
                };
                if owner.as_ref().is_some_and(|prior| prior != &found) {
                    return Err(feature::conflict());
                }
                owner = Some(found);
            }
        }
        self.qualified_query_node_at(
            &root,
            &path,
            &owner.ok_or_else(|| {
                CoreError::new(
                    "identityPending",
                    UiMessage::error_core_hosted_content_import_member_has_no_original_committed_hosted_import_producer(),
                )
            })?,
            "live",
        )
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_content_import_tests.rs"]
mod tests;
