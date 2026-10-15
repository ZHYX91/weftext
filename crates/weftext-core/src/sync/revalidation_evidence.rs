//! Read-only evidence capture for an already reviewed, explicitly supplied cut.
//! No locator discovery, author writes, operation staging or new admission is
//! performed. A returned read set is still subject to the original writer's
//! namespace/installed-cut checks and held commit/recovery guards.
use super::replica::{LocalObject, SyncPhysicalBinding};
use super::revalidation::{
    CheckedRevalidation, HistoricalSemanticInput, InvocationValidation, QueryValidationTrace,
    RequiredSemanticEvidence,
};
use super::*;
use crate::{
    AdmittedSnapshot, Core, Path, PathBuf, QualifiedQueryNode, QualifiedQueryScope,
    QualifiedResource, SnapshotWitness,
};
use weftext_knowledge::{AnnotationTarget, NodeRef};

/// These are original opaque Core qualifications, never client booleans.
pub(crate) enum ObjectQualification<'a> {
    Node(&'a QualifiedQueryNode),
    Directory(&'a QualifiedQueryScope),
    Configuration,
}
pub(crate) struct SuppliedObject<'a> {
    /// The original writer's exact canonical/marker/config file path.
    pub path: &'a Path,
    pub qualification: ObjectQualification<'a>,
}
pub(crate) struct SuppliedEvidenceCut<'a> {
    pub root: &'a Path,
    pub graph: &'a ReplicaGraph,
    pub installed: &'a BTreeMap<ObjectKey, LocalObject>,
    pub chosen: &'a BTreeMap<ObjectKey, Version>,
    /// From the original installed cut, not stamps freshly selected from disk.
    pub physical: &'a [SyncPhysicalBinding],
    /// Original file/parent observations, including negative dependencies.
    pub snapshots: &'a [SnapshotWitness],
    pub objects: &'a BTreeMap<ObjectKey, SuppliedObject<'a>>,
    pub resources: &'a [&'a QualifiedResource],
    /// Original Core admitted snapshots, not a deserialized FrozenQueryInput.
    /// Only needed for an actual row-dependent Query/provider invocation.
    pub query_snapshots: &'a [&'a AdmittedSnapshot],
    /// Exact final locators from the original application planner. These may
    /// differ from installed paths during the same original Compound action.
    pub after_locations: &'a BTreeMap<ObjectKey, PathBuf>,
    pub planned: Option<&'a crate::journal::Plan>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EvidenceGap {
    OriginalQualification,
    PlannedObjectMaterialization,
    PlannedResourceMaterialization,
    DefinitionAdmission,
    ExactHistoricalSource,
    OriginalNegativeConfiguration,
    LegacyRulesFeatureMerge,
    ExactAnnotationHistory,
    CompiledElementOrigin,
    ExactInvocationInputs,
}
pub(crate) struct UnfulfilledEvidence {
    pub obligation: RequiredSemanticEvidence,
    pub gap: EvidenceGap,
}
pub(crate) struct EvidenceCapture {
    witnesses: Vec<SnapshotWitness>,
    discharged: Vec<RequiredSemanticEvidence>,
    unfulfilled: Vec<UnfulfilledEvidence>,
    read_objects: BTreeSet<ObjectKey>,
    provider_dependencies: Vec<QueryValidationTrace>,
    query_membership: Option<BTreeSet<String>>,
    annotation_targets: Vec<super::revalidation::SyncAnnotationTargetStatus>,
}
pub(crate) struct EvidenceReadSet {
    pub witnesses: Vec<SnapshotWitness>,
    pub discharged: Vec<RequiredSemanticEvidence>,
    /// The writer must include these actual reads in the reviewed dependency
    /// and expected-version cut; supplied extra candidates are not approval.
    pub read_objects: BTreeSet<ObjectKey>,
    pub provider_dependencies: Vec<QueryValidationTrace>,
    /// Preserve in the original FrozenOperation.expected_membership guard.
    pub query_membership: Option<BTreeSet<String>>,
    pub annotation_targets: Vec<super::revalidation::SyncAnnotationTargetStatus>,
}
impl EvidenceCapture {
    pub(crate) fn unfulfilled(&self) -> &[UnfulfilledEvidence] {
        &self.unfulfilled
    }
    /// This is not a Plan/permission. The original writer must incorporate all
    /// bindings into its own frozen operation and retain its existing guards.
    pub(crate) fn into_read_set(self) -> Result<EvidenceReadSet> {
        if !self.unfulfilled.is_empty() {
            return Err(error(
                "syncEvidenceRequired",
                "Exact semantic evidence remains unfulfilled",
            ));
        }
        Ok(EvidenceReadSet {
            witnesses: self.witnesses,
            discharged: self.discharged,
            read_objects: self.read_objects,
            provider_dependencies: self.provider_dependencies,
            query_membership: self.query_membership,
            annotation_targets: self.annotation_targets,
        })
    }
}
fn error(code: &str, message: &str) -> CoreError {
    CoreError::new(code, message)
}
fn mismatch() -> CoreError {
    error(
        "syncEvidenceChanged",
        "Original evidence does not match the reviewed installed cut",
    )
}
fn same_locator(a: &Path, b: &Path) -> bool {
    // Pure controlled ASCII locator comparison; physical identity and parent
    // are independently supplied by opaque witnesses and checked below.
    crate::history::history_relative_path(a, b).is_some_and(|p| p.as_os_str().is_empty())
}
fn object_file(object: &ObjectKey, state: &ObjectState) -> Option<String> {
    Some(match object {
        ObjectKey::Node(_) => format!("{}.typ", state.name),
        ObjectKey::Directory(_) | ObjectKey::RootDirectory(_) => "weftext.directory.json".into(),
        ObjectKey::Schema => ".weftext-schema.json".into(),
        ObjectKey::Rules => ".weftext-rules".into(),
        ObjectKey::Settings => ".weftext-settings.json".into(),
        ObjectKey::Tags => ".weftext-tags.json".into(),
    })
}

/// Read the original writer's exact planned FileChanges for every existing
/// mutation. No output, path or identity is synthesized by semantic review.
pub(crate) fn original_outputs(
    transaction: &crate::operations::FrozenOperation,
) -> Vec<crate::operations::FileChange> {
    match &transaction.mutation {
        crate::operations::Mutation::Update { files } => files.clone(),
        crate::operations::Mutation::CreateTree { files, updates, .. } => {
            files.iter().chain(updates).cloned().collect()
        }
        crate::operations::Mutation::Relocate { .. }
        | crate::operations::Mutation::Compound { .. } => transaction.relocated_files(),
    }
}

struct Capture<'a, 'b> {
    core: &'a Core,
    cut: &'a SuppliedEvidenceCut<'b>,
    checked: &'a CheckedRevalidation,
    output: EvidenceCapture,
    objects: BTreeSet<ObjectKey>,
    planned_objects: BTreeSet<ObjectKey>,
}
impl Capture<'_, '_> {
    fn planned_after(&self, object: &ObjectKey) -> Result<bool> {
        let Some(plan) = self.cut.planned else {
            return Ok(false);
        };
        let (Some(application), Some(transaction), Some(chosen)) = (
            &plan.sync_application,
            &plan.transaction,
            self.cut.chosen.get(object),
        ) else {
            return Ok(false);
        };
        if application.identity.workspace_id != self.cut.graph.workspace_id
            || application.before.iter().find(|old| &old.object == object)
                != self.cut.installed.get(object)
        {
            return Err(mismatch());
        }
        let Some(after) = application
            .after
            .iter()
            .find(|after| &after.object == object)
        else {
            return Ok(false);
        };
        if after.state != chosen.state {
            return Err(mismatch());
        }
        let outputs = original_outputs(transaction);
        for (relative, digest) in &after.state.files {
            let path = Path::new(&after.directory).join(relative);
            if transaction.preview.revisions.get(&crate::display(&path))
                != after.state.file_revisions.get(relative)
                || !outputs.iter().any(|output| {
                    same_locator(Path::new(&output.path), &path)
                        && crate::hash(&output.after) == *digest
                })
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn planned_object(&self, object: &ObjectKey) -> Result<bool> {
        if !self.planned_after(object)? {
            return Ok(false);
        }
        let plan = self.cut.planned.unwrap();
        let application = plan.sync_application.as_ref().unwrap();
        let transaction = plan.transaction.as_ref().unwrap();
        let after = application
            .after
            .iter()
            .find(|after| &after.object == object)
            .unwrap();
        let main = object_file(object, &after.state).unwrap();
        let path = Path::new(&after.directory).join(main);
        if !original_outputs(transaction)
            .iter()
            .any(|file| file.before.is_none() && same_locator(Path::new(&file.path), &path))
        {
            return Ok(false);
        }
        Ok(match object {
            ObjectKey::Node(id) => transaction
                .births()
                .iter()
                .any(|birth| birth.node_id == *id && same_locator(Path::new(&birth.path), &path)),
            // The original marker/resource admission consumers establish the
            // receiver FI from their original materialized create handles.
            _ => true,
        })
    }
    fn planned_resource(&self, owner: &NodeRef, reference: &str, revision: &str) -> Result<bool> {
        let object = ObjectKey::Node(owner.node_id.clone());
        if !self.planned_after(&object)? {
            return Ok(false);
        }
        let chosen = &self.cut.chosen[&object];
        let resources = super::apply::validate_state(self.cut.graph, &object, &chosen.state)?;
        let id = reference.rsplit('/').next().ok_or_else(mismatch)?;
        let Some((relative, _)) = resources
            .iter()
            .find(|(_, resource)| resource.as_str() == id)
        else {
            return Ok(false);
        };
        if chosen
            .state
            .file_revisions
            .get(relative)
            .map(String::as_str)
            != Some(revision)
            || self
                .cut
                .installed
                .get(&object)
                .is_some_and(|old| old.state.files.contains_key(relative))
        {
            return Ok(false);
        }
        let plan = self.cut.planned.unwrap();
        let after = plan
            .sync_application
            .as_ref()
            .unwrap()
            .after
            .iter()
            .find(|after| after.object == object)
            .unwrap();
        let path = Path::new(&after.directory).join(relative);
        let transaction = plan.transaction.as_ref().unwrap();
        Ok(transaction
            .preview
            .resource_map
            .get(&crate::display(&path))
            .map(String::as_str)
            == Some(reference)
            && original_outputs(transaction)
                .iter()
                .any(|file| file.before.is_none() && same_locator(Path::new(&file.path), &path)))
    }
    fn received_definition_key(&self, owner: &NodeRef, key: &str) -> Result<bool> {
        let object = ObjectKey::Node(owner.node_id.clone());
        if !self.planned_after(&object)? {
            return Ok(false);
        }
        for id in &self.cut.graph.admitted {
            for change in self.cut.graph.manifests[id]
                .changes
                .iter()
                .filter(|change| change.object == object)
            {
                let relative = format!("{}.typ", change.after.name);
                let Some(bytes) = change
                    .after
                    .files
                    .get(&relative)
                    .and_then(|digest| self.cut.graph.blob(digest))
                else {
                    continue;
                };
                let source = crate::decode(bytes)?;
                if weftext_engine::inspect_metadata(&source).id.as_ref() != Some(&owner.node_id) {
                    return Err(mismatch());
                }
                if weftext_knowledge::definition_slots(&source, owner)
                    .is_ok_and(|slots| slots.iter().any(|slot| slot.key == key))
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    fn gap(&mut self, obligation: &RequiredSemanticEvidence, gap: EvidenceGap) {
        self.output.unfulfilled.push(UnfulfilledEvidence {
            obligation: obligation.clone(),
            gap,
        });
    }
    fn merge(&mut self, incoming: Vec<SnapshotWitness>) -> Result<()> {
        // Original checked merge supplies parent/stamp/FI/bytes/negative
        // consistency. Its current limit is 64 unique reads / 64 MiB, not the
        // larger sync transaction capacity; larger cuts are a shared API gap.
        Core::merge_feature_witnesses(&mut self.output.witnesses, incoming)
    }
    fn installed_snapshot(
        &self,
        object: &ObjectKey,
        relative: &str,
    ) -> Result<Option<SnapshotWitness>> {
        self.installed_snapshot_from(&self.output.witnesses, object, relative)
    }
    fn installed_snapshot_from(
        &self,
        witnesses: &[SnapshotWitness],
        object: &ObjectKey,
        relative: &str,
    ) -> Result<Option<SnapshotWitness>> {
        let Some(local) = self.cut.installed.get(object) else {
            return Ok(None);
        };
        let expected = Path::new(&local.directory).join(relative);
        let mut matching = witnesses
            .iter()
            .filter(|w| same_locator(Path::new(&w.binding.path), &expected));
        let Some(witness) = matching.next() else {
            return Ok(None);
        };
        if matching.next().is_some() {
            return Err(mismatch());
        }
        let captured = witness.binding.captured.as_ref().ok_or_else(mismatch)?;
        if !same_locator(Path::new(&captured.path), &expected)
            || local.state.files.get(relative) != Some(&crate::hash(&captured.bytes))
        {
            return Err(mismatch());
        }
        let physical = self
            .cut
            .physical
            .iter()
            .filter(|b| &b.object == object && b.relative_path == relative)
            .collect::<Vec<_>>();
        if physical.len() != 1 || physical[0].stamp != captured.stamp {
            return Err(mismatch());
        }
        if !self.installed_file_revision_matches(object, relative, captured)? {
            return Err(error(
                "syncEvidenceChanged",
                "Installed file revision lacks its original observation or replica initialization binding",
            ));
        }
        Ok(Some(witness.clone()))
    }
    fn installed_file_revision_matches(
        &self,
        object: &ObjectKey,
        relative: &str,
        captured: &crate::operations::CapturedFile,
    ) -> Result<bool> {
        let local = &self.cut.installed[object];
        if let Some(observed) = self.core.accepted_revision(captured) {
            // An existing observation never falls back to initialization.
            return Ok(local.state.file_revisions.get(relative) == Some(&observed));
        }
        // enable_sync can assign a portable genesis revision to an unchanged
        // file that has no separate accepted local observation (default Rules
        // are one example). Only its original committed physical setup binds
        // that number; None alone is neither admission nor permission to mint.
        let setup = self.core.sync_setup_for(self.cut.root)?;
        if setup.identity.workspace_id != self.cut.graph.workspace_id {
            return Err(mismatch());
        }
        let initial = setup
            .initial
            .iter()
            .filter(|entry| &entry.object == object)
            .collect::<Vec<_>>();
        let [initial] = initial.as_slice() else {
            return Ok(false);
        };
        let physical = setup
            .physical_bindings
            .iter()
            .filter(|binding| &binding.object == object && binding.relative_path == relative)
            .collect::<Vec<_>>();
        Ok(physical.len() == 1
            && physical[0].stamp == captured.stamp
            && same_locator(
                Path::new(&captured.path),
                &Path::new(&initial.directory).join(relative),
            )
            && initial.state.file_revisions.get(relative)
                == local.state.file_revisions.get(relative)
            && initial.state.files.get(relative) == Some(&crate::hash(&captured.bytes)))
    }
    fn object(
        &mut self,
        object: &ObjectKey,
        installed_revision: Option<&str>,
    ) -> Result<Option<EvidenceGap>> {
        let Some(local) = self.cut.installed.get(object) else {
            if installed_revision.is_none() && self.planned_object(object)? {
                self.planned_objects.insert(object.clone());
                return Ok(None);
            }
            return Ok(Some(EvidenceGap::PlannedObjectMaterialization));
        };
        if installed_revision != Some(local.revision.as_str()) || &local.object != object {
            return Err(mismatch());
        }
        if self.objects.contains(object) {
            return Ok(None);
        }
        let Some(supplied) = self.cut.objects.get(object) else {
            return Ok(Some(EvidenceGap::OriginalQualification));
        };
        let relative = object_file(object, &local.state).unwrap();
        let path = Path::new(&local.directory).join(&relative);
        if !same_locator(supplied.path, &path) {
            return Err(mismatch());
        }
        match (&supplied.qualification, object) {
            (ObjectQualification::Node(q), ObjectKey::Node(id)) => {
                let owner = NodeRef {
                    workspace_id: self.cut.graph.workspace_id.clone(),
                    node_id: id.clone(),
                };
                if q.owner != owner
                    || q.lifecycle != if local.state.retired { "trash" } else { "live" }
                {
                    return Err(mismatch());
                }
                self.core
                    .validate_query_node_witness(self.cut.root, &q.witness)?;
                let mut pool = vec![];
                match Core::merge_query_node_feature_witness(&mut pool, &q.witness) {
                    Ok(()) => {
                        if self
                            .installed_snapshot_from(&pool, object, &relative)?
                            .is_none()
                        {
                            return Err(mismatch());
                        }
                        self.merge(pool)?;
                    }
                    Err(e) if e.code == "basisRequired" => {
                        return Ok(Some(EvidenceGap::LegacyRulesFeatureMerge));
                    }
                    Err(e) => return Err(e),
                }
            }
            (
                ObjectQualification::Directory(q),
                ObjectKey::Directory(id) | ObjectKey::RootDirectory(id),
            ) => {
                let reference = format!("weftext-directory://{}/{id}", self.cut.graph.workspace_id);
                if q.target_ref != reference
                    || q.lifecycle != if local.state.retired { "trash" } else { "live" }
                {
                    return Err(mismatch());
                }
                self.core
                    .validate_query_scope_witness(self.cut.root, &q.witness)?;
                let mut pool = vec![];
                match Core::merge_query_scope_feature_witness(&mut pool, &q.witness) {
                    Ok(()) => {
                        if self
                            .installed_snapshot_from(&pool, object, &relative)?
                            .is_none()
                        {
                            return Err(mismatch());
                        }
                        self.merge(pool)?;
                    }
                    Err(e) if e.code == "basisRequired" => {
                        return Ok(Some(EvidenceGap::LegacyRulesFeatureMerge));
                    }
                    Err(e) => return Err(e),
                }
            }
            (
                ObjectQualification::Configuration,
                ObjectKey::Schema | ObjectKey::Rules | ObjectKey::Settings | ObjectKey::Tags,
            ) => {
                if !same_locator(supplied.path, &self.cut.root.join(&relative)) {
                    return Err(mismatch());
                }
            }
            _ => return Err(mismatch()),
        }
        let Some(snapshot) = self.installed_snapshot(object, &relative)? else {
            return Ok(Some(EvidenceGap::OriginalQualification));
        };
        let file = snapshot.binding.captured.as_ref().unwrap();
        if let ObjectKey::Node(id) = object {
            let birth = self
                .core
                .admitted_binding(&file.stamp.identity)
                .ok_or_else(mismatch)?;
            if birth.workspace_id != self.cut.graph.workspace_id
                || birth.node_id != *id
                || weftext_engine::inspect_metadata(&crate::decode(&file.bytes)?)
                    .id
                    .as_ref()
                    != Some(id)
            {
                return Err(mismatch());
            }
            self.core
                .check_history_placement(&birth, Path::new(&file.path))?;
        } else if let ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) = object {
            crate::directory_identity::validate_directory_marker(&file.bytes)?;
            if serde_json::from_slice::<serde_json::Value>(&file.bytes)?["directoryId"] != *id
                || !self.core.directory_admitted(
                    &self.cut.graph.workspace_id,
                    id,
                    &snapshot.binding.parent,
                )?
            {
                return Err(mismatch());
            }
        }
        self.objects.insert(object.clone());
        Ok(None)
    }
    fn ensure_object(&mut self, object: &ObjectKey) -> Result<Option<EvidenceGap>> {
        let revision = self.cut.installed.get(object).map(|v| v.revision.as_str());
        self.object(object, revision)
    }
    fn resource(
        &mut self,
        owner: &NodeRef,
        reference: &str,
        revision: &str,
    ) -> Result<Option<EvidenceGap>> {
        if owner.workspace_id != self.cut.graph.workspace_id {
            return Err(mismatch());
        }
        let object = ObjectKey::Node(owner.node_id.clone());
        if let Some(gap) = self.ensure_object(&object)? {
            return Ok(Some(gap));
        }
        if self.planned_resource(owner, reference, revision)? {
            return Ok(None);
        }
        let candidates = self
            .cut
            .resources
            .iter()
            .filter(|q| q.owner == *owner && q.resource_ref == reference)
            .copied()
            .collect::<Vec<_>>();
        let [q] = candidates.as_slice() else {
            return Ok(Some(EvidenceGap::OriginalQualification));
        };
        self.core
            .validate_resource_witness(self.cut.root, &q.witness)?;
        let mut pool = vec![];
        Core::merge_resource_witness(&mut pool, &q.witness)?;
        // Prove this qualifier's actual selected file against the original
        // installed stamp/manifest entry. Public descriptor fields are not proof.
        let local = &self.cut.installed[&object];
        let Some(manifest) =
            self.installed_snapshot_from(&pool, &object, "weftext.resources.json")?
        else {
            return Err(mismatch());
        };
        let manifest: serde_json::Value =
            serde_json::from_slice(&manifest.binding.captured.as_ref().unwrap().bytes)?;
        let prefix = format!("weftext-resource://{}/", self.cut.graph.workspace_id);
        let id = reference.strip_prefix(&prefix).ok_or_else(mismatch)?;
        super::uuid(id)?;
        let entries = manifest["resources"]
            .as_array()
            .filter(|_| manifest["version"] == 1)
            .ok_or_else(mismatch)?;
        let selected = entries
            .iter()
            .filter(|entry| entry["resourceId"] == id)
            .collect::<Vec<_>>();
        let [entry] = selected.as_slice() else {
            return Err(mismatch());
        };
        let relative = entry["path"].as_str().ok_or_else(mismatch)?;
        super::relative(relative)?;
        let Some(snapshot) = self.installed_snapshot_from(&pool, &object, relative)? else {
            return Err(mismatch());
        };
        let actual = snapshot.binding.captured.as_ref().unwrap();
        if q.content_hash != crate::hash(&actual.bytes)
            || q.bytes() != actual.bytes
            || q.revision.as_ref() != local.state.file_revisions.get(relative)
            || q.lifecycle != if local.state.retired { "trash" } else { "live" }
            || !self.core.admitted_resource(
                id,
                &owner.workspace_id,
                &owner.node_id,
                &actual.stamp.identity,
            )
        {
            return Err(mismatch());
        }
        self.merge(pool)?;
        if local.state.file_revisions.get(relative).map(String::as_str) != Some(revision) {
            if let Some(chosen) = self.cut.chosen.get(&object)
                && chosen
                    .state
                    .file_revisions
                    .get(relative)
                    .map(String::as_str)
                    == Some(revision)
                && super::apply::validate_state(self.cut.graph, &object, &chosen.state)?
                    .get(relative)
                    .map(String::as_str)
                    == Some(id)
            {
                // Original qualification establishes the existing owner/FI;
                // the same original writer replaces its exact captured bytes.
                // It does not require the incoming after bytes to exist early.
                return Ok(None);
            }
            return Ok(Some(EvidenceGap::PlannedResourceMaterialization));
        }
        Ok(None)
    }
    fn exact_file(
        &self,
        object: &ObjectKey,
        relative: &str,
        revision: &str,
    ) -> Result<Option<&[u8]>> {
        if let Some(version) = self.cut.chosen.get(object)
            && version
                .state
                .file_revisions
                .get(relative)
                .is_some_and(|r| r == revision)
        {
            let digest = version.state.files.get(relative).ok_or_else(mismatch)?;
            return Ok(self.cut.graph.blob(digest));
        }
        let historical = self
            .checked
            .historical_inputs
            .iter()
            .filter(|item| {
                &item.object == object
                    && item.file_revision == revision
                    && item.relative_path == relative
            })
            .collect::<Vec<_>>();
        let [input] = historical.as_slice() else {
            return Ok(None);
        };
        if !self.exact_historical_descriptor(input)? {
            return Err(mismatch());
        }
        Ok(self.cut.graph.blob(&input.digest))
    }
    fn exact_historical_descriptor(&self, input: &HistoricalSemanticInput) -> Result<bool> {
        for id in &self.cut.graph.admitted {
            for change in &self.cut.graph.manifests[id].changes {
                if change.object == input.object
                    && change.revision == input.object_revision
                    && change.after.file_revisions.get(&input.relative_path)
                        == Some(&input.file_revision)
                    && change.after.files.get(&input.relative_path) == Some(&input.digest)
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    fn source(&self, owner: &NodeRef, revision: &str) -> Result<Option<String>> {
        let object = ObjectKey::Node(owner.node_id.clone());
        let Some(version) = self.cut.chosen.get(&object) else {
            return Ok(None);
        };
        let relative = object_file(&object, &version.state).unwrap();
        let mut bytes = self.exact_file(&object, &relative, revision)?;
        if bytes.is_none() {
            // A pinned source can precede a rename. Use its exact received
            // canonical path descriptor, not the current namesake's path.
            let originals = self
                .checked
                .historical_inputs
                .iter()
                .filter(|input| input.object == object && input.file_revision == revision)
                .collect::<Vec<_>>();
            if let [input] = originals.as_slice() {
                if !self.exact_historical_descriptor(input)? {
                    return Err(mismatch());
                }
                bytes = self.cut.graph.blob(&input.digest);
            }
        }
        let Some(bytes) = bytes else { return Ok(None) };
        crate::check_size(bytes)?;
        let source = crate::decode(bytes)?;
        if owner.workspace_id != self.cut.graph.workspace_id
            || weftext_engine::inspect_metadata(&source).id.as_deref()
                != Some(owner.node_id.as_str())
        {
            return Err(mismatch());
        }
        Ok(Some(source))
    }
    fn definition(
        &mut self,
        owner: &NodeRef,
        key: &str,
        revision: &str,
    ) -> Result<Option<EvidenceGap>> {
        if let Some(gap) = self.ensure_object(&ObjectKey::Node(owner.node_id.clone()))? {
            return Ok(Some(gap));
        }
        if !self
            .core
            .is_definition_key_admitted(self.cut.root, &owner.node_id, key)?
            && !self.received_definition_key(owner, key)?
        {
            return Ok(Some(EvidenceGap::DefinitionAdmission));
        }
        let Some(source) = self.source(owner, revision)? else {
            return Ok(Some(EvidenceGap::ExactHistoricalSource));
        };
        let definitions = weftext_knowledge::definition_slots(&source, owner).map_err(|_| {
            error(
                "definitionUnavailable",
                "Exact admitted definition control is unavailable",
            )
        })?;
        if definitions.iter().filter(|d| d.key == key).count() != 1 {
            return Err(error(
                "definitionUnavailable",
                "Exact selected source lacks its admitted definition key",
            ));
        }
        Ok(None)
    }
    fn absent_configuration(&mut self, object: &ObjectKey) -> Result<Option<EvidenceGap>> {
        let relative = match object {
            ObjectKey::Schema => ".weftext-schema.json",
            ObjectKey::Rules => ".weftext-rules",
            _ => return Err(mismatch()),
        };
        if self.cut.installed.contains_key(object)
            || self.cut.chosen.contains_key(object)
            || self.cut.graph.objects()?.contains_key(object)
            || self.cut.graph.manifests.iter().any(|(id, m)| {
                m.changes.iter().any(|c| &c.object == object)
                    && matches!(self.cut.graph.status(id), Ok(Admission::Staged { .. }))
            })
        {
            return Err(mismatch());
        }
        if matches!(object, ObjectKey::Rules)
            && self.core.query_rules_ever_committed(self.cut.root)?
        {
            return Err(error(
                "rulesUnavailable",
                "Previously committed rules are missing; absence is not a legacy default",
            ));
        }
        let path = self.cut.root.join(relative);
        let witnesses = self
            .output
            .witnesses
            .iter()
            .filter(|w| same_locator(Path::new(&w.binding.path), &path))
            .collect::<Vec<_>>();
        let [witness] = witnesses.as_slice() else {
            return Ok(Some(EvidenceGap::OriginalNegativeConfiguration));
        };
        if witness.binding.captured.is_some() {
            return Err(mismatch());
        }
        self.core
            .validate_snapshot_witness(self.cut.root, &path, witness)?;
        Ok(None)
    }
    fn annotations(&mut self, owner: &NodeRef, revision: &str) -> Result<Option<EvidenceGap>> {
        let object = ObjectKey::Node(owner.node_id.clone());
        if let Some(gap) = self.ensure_object(&object)? {
            return Ok(Some(gap));
        }
        let Some(bytes) = self.exact_file(&object, "weftext.annotations.json", revision)? else {
            return Ok(Some(EvidenceGap::ExactAnnotationHistory));
        };
        let document = weftext_knowledge::decode_annotations(bytes).map_err(|_| {
            error(
                "annotationUnavailable",
                "Exact annotation input is malformed",
            )
        })?;
        for record in &document.records {
            let (state, reason) = self.annotation_status(owner, &record.target)?;
            self.output
                .annotation_targets
                .push(super::revalidation::SyncAnnotationTargetStatus {
                    owner: owner.clone(),
                    annotation_id: record.annotation_id.clone(),
                    state,
                    reason,
                });
        }
        Ok(None)
    }
    fn annotation_status(
        &mut self,
        owner: &NodeRef,
        target: &AnnotationTarget,
    ) -> Result<(String, String)> {
        let unavailable = |reason: &str| ("unavailable".into(), reason.into());
        let may_read = |node: &NodeRef| {
            node.workspace_id == self.cut.graph.workspace_id
                && (node == owner
                    || self
                        .checked
                        .dependencies
                        .iter()
                        .any(|d| d.object == ObjectKey::Node(node.node_id.clone())))
        };
        match target {
            AnnotationTarget::WholeDocument { node } => {
                if !may_read(node) {
                    return Ok(unavailable("preserved-target-not-in-authorized-read-cut"));
                }
                if self
                    .ensure_object(&ObjectKey::Node(node.node_id.clone()))?
                    .is_some()
                {
                    return Ok(unavailable("preserved-target-qualification-unavailable"));
                }
                Ok(("exact".into(), "original-qualified-target".into()))
            }
            AnnotationTarget::SourceRange { anchor } => {
                if !may_read(&anchor.node) {
                    return Ok(unavailable("preserved-target-not-in-authorized-read-cut"));
                }
                if self
                    .ensure_object(&ObjectKey::Node(anchor.node.node_id.clone()))?
                    .is_some()
                {
                    return Ok(unavailable("preserved-target-qualification-unavailable"));
                }
                let Some(source) = self.source(&anchor.node, &anchor.revision_id)? else {
                    return Ok(unavailable("preserved-exact-anchor-revision-unavailable"));
                };
                let snapshot = weftext_knowledge::NodeSnapshot {
                    owner: anchor.node.clone(),
                    path: String::new(),
                    library_path: String::new(),
                    revision_id: anchor.revision_id.clone(),
                    source,
                    title: None,
                    aliases: vec![],
                    labels: vec![],
                    labels_complete: false,
                };
                if weftext_knowledge::resolve_anchor(anchor, Some(&snapshot), &[]).status == "exact"
                {
                    Ok((
                        "exact".into(),
                        "preserved-original-anchor-revision-and-range".into(),
                    ))
                } else {
                    Ok(unavailable("preserved-anchor-awaits-explicit-reconnection"))
                }
            }
            AnnotationTarget::WholeResource {
                resource,
                revision_id,
            } => {
                let candidates = self
                    .cut
                    .resources
                    .iter()
                    .filter(|q| q.resource_ref == *resource && may_read(&q.owner))
                    .map(|q| q.owner.clone())
                    .collect::<Vec<_>>();
                let [target_owner] = candidates.as_slice() else {
                    return Ok(unavailable("preserved-resource-not-in-authorized-read-cut"));
                };
                if self
                    .resource(target_owner, resource, revision_id)?
                    .is_some()
                {
                    return Ok(unavailable("preserved-exact-resource-revision-unavailable"));
                }
                Ok((
                    "exact".into(),
                    "original-qualified-resource-revision".into(),
                ))
            }
            AnnotationTarget::SourceElement { .. } => {
                Ok(unavailable("preserved-compiled-element-origin-unavailable"))
            }
            AnnotationTarget::PdfRegion { .. }
            | AnnotationTarget::PdfTextRange { .. }
            | AnnotationTarget::ImageRegion { .. }
            | AnnotationTarget::MediaInterval { .. } => {
                Ok(unavailable("preserved-trusted-media-facts-unavailable"))
            }
        }
    }

    fn invocation(
        &mut self,
        owner: &NodeRef,
        key: &str,
        revision: &str,
    ) -> Result<Option<EvidenceGap>> {
        let candidates = self
            .checked
            .invocations
            .iter()
            .filter(|input| {
                input.owner == *owner && input.key == key && input.source_revision == revision
            })
            .cloned()
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(Some(EvidenceGap::ExactInvocationInputs));
        }
        for invocation in &candidates {
            if let Some(gap) = self.actual_invocation(invocation)? {
                return Ok(Some(gap));
            }
        }
        Ok(None)
    }

    fn actual_invocation(
        &mut self,
        invocation: &InvocationValidation,
    ) -> Result<Option<EvidenceGap>> {
        let lifecycles = weftext_data::required_query_lifecycles(
            &invocation.plan,
            &invocation.context.definitions,
        )
        .map_err(|_| {
            error(
                "queryInvocationUnavailable",
                "Actual Query lifecycles are unavailable",
            )
        })?;
        // The existing admitted cut guard is a live-membership guard. Do not
        // silently turn a Trash/current mixed invocation into a live input.
        if lifecycles.iter().any(|l| l == "trash") {
            return Ok(Some(EvidenceGap::ExactInvocationInputs));
        }
        let snapshots = self
            .cut
            .query_snapshots
            .iter()
            .copied()
            .filter(|snapshot| snapshot.inventory.status != "trashReadOnly")
            .collect::<Vec<_>>();
        let [snapshot] = snapshots.as_slice() else {
            return Ok(Some(EvidenceGap::ExactInvocationInputs));
        };
        if !snapshot.complete
            || snapshot.inventory.workspace_id.as_ref() != Some(&self.cut.graph.workspace_id)
            || !same_locator(Path::new(&snapshot.inventory.root), self.cut.root)
        {
            return Err(error(
                "queryInvocationUnavailable",
                "Actual Query requires its complete original admitted cut",
            ));
        }
        let Some(original) = self.core.admitted_cuts.get(&snapshot.cut_id) else {
            return Err(error(
                "cutExpired",
                "The original admitted Query cut is no longer retained",
            ));
        };
        let actual_bases = snapshot
            .files
            .iter()
            .chain(snapshot.schema.iter())
            .map(|file| file.base_id.clone())
            .collect::<BTreeSet<_>>();
        if actual_bases.len() != snapshot.files.len() + usize::from(snapshot.schema.is_some())
            || actual_bases != original.bases.iter().cloned().collect()
        {
            return Err(mismatch());
        }
        let members = snapshot
            .membership_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if members.len() != snapshot.membership_ids.len()
            || snapshot
                .files
                .iter()
                .filter_map(|f| f.node_id.clone())
                .collect::<BTreeSet<_>>()
                != members
            || snapshot.files.len() != members.len()
            || !members
                .iter()
                .cloned()
                .map(ObjectKey::Node)
                .collect::<BTreeSet<_>>()
                .is_subset(&invocation.members)
        {
            return Err(mismatch());
        }
        if self
            .output
            .query_membership
            .as_ref()
            .is_some_and(|old| old != &members)
        {
            return Err(mismatch());
        }
        let mut notes = Vec::new();
        for original_file in &snapshot.files {
            let id = original_file.node_id.as_ref().ok_or_else(mismatch)?;
            let object = ObjectKey::Node(id.clone());
            if let Some(gap) = self.ensure_object(&object)? {
                return Ok(Some(gap));
            }
            let installed = &self.cut.installed[&object];
            let relative = object_file(&object, &installed.state).unwrap();
            let Some(witness) = self.installed_snapshot(&object, &relative)? else {
                return Err(mismatch());
            };
            let captured = witness.binding.captured.as_ref().unwrap();
            if !same_locator(Path::new(&original_file.path), Path::new(&captured.path))
                || original_file.bytes.as_ref() != Some(&captured.bytes)
            {
                return Err(mismatch());
            }
            let Some(version) = self.cut.chosen.get(&object) else {
                return Err(mismatch());
            };
            if version.state.retired {
                // The selected after state has left live membership. Its
                // before source remains guarded but is not an after Query row.
                continue;
            }
            let revision = &version.state.file_revisions[&format!("{}.typ", version.state.name)];
            let Some(source) = self.source(
                &NodeRef {
                    workspace_id: self.cut.graph.workspace_id.clone(),
                    node_id: id.clone(),
                },
                revision,
            )?
            else {
                return Ok(Some(EvidenceGap::ExactHistoricalSource));
            };
            let Some(directory) = self.cut.after_locations.get(&object) else {
                return Ok(Some(EvidenceGap::ExactInvocationInputs));
            };
            notes.push(weftext_data::FrozenNote {
                entity_key: format!("weftext-node://{}/{id}", self.cut.graph.workspace_id),
                path: crate::display(&directory.join(format!("{}.typ", version.state.name))),
                source,
                revision: revision.clone(),
                lifecycle: "live".into(),
                title: None,
                body: None,
                products: None,
                authorized_fields: None,
            });
        }
        for object in invocation.members.iter().filter(|object| match object {
            ObjectKey::Node(id) => !members.contains(id),
            _ => true,
        }) {
            let ObjectKey::Node(id) = object else {
                return Err(mismatch());
            };
            let Some(version) = self.cut.chosen.get(object) else {
                return Err(mismatch());
            };
            if version.state.retired
                || self
                    .cut
                    .installed
                    .get(object)
                    .is_some_and(|old| !old.state.retired)
                || !self.planned_after(object)?
            {
                return Err(mismatch());
            }
            if let Some(gap) = self.ensure_object(object)? {
                return Ok(Some(gap));
            }
            let relative = format!("{}.typ", version.state.name);
            let revision = version.state.file_revisions[&relative].clone();
            let Some(source) = self.source(
                &NodeRef {
                    workspace_id: self.cut.graph.workspace_id.clone(),
                    node_id: id.clone(),
                },
                &revision,
            )?
            else {
                return Ok(Some(EvidenceGap::ExactHistoricalSource));
            };
            let Some(directory) = self.cut.after_locations.get(object) else {
                return Ok(Some(EvidenceGap::ExactInvocationInputs));
            };
            notes.push(weftext_data::FrozenNote {
                entity_key: format!("weftext-node://{}/{id}", self.cut.graph.workspace_id),
                path: crate::display(&directory.join(relative)),
                source,
                revision,
                lifecycle: "live".into(),
                title: None,
                body: None,
                products: None,
                authorized_fields: None,
            });
        }
        let products =
            weftext_data::query_products(&invocation.plan, &invocation.context.definitions)
                .map_err(|_| {
                    error(
                        "queryInvocationUnavailable",
                        "Actual Query products are unavailable",
                    )
                })?;
        if !products.is_empty() {
            return Ok(Some(EvidenceGap::CompiledElementOrigin));
        }
        let mut context = invocation.context.clone();
        for scope in weftext_data::required_query_scopes(&invocation.plan, &context.definitions)
            .map_err(|_| error("queryScopeUnavailable", "Actual Query scope is unavailable"))?
        {
            let (kind, workspace, id) = crate::directory_identity::parse_ref(&scope)?;
            if workspace != self.cut.graph.workspace_id {
                return Err(mismatch());
            }
            let object = if kind == "node" {
                ObjectKey::Node(id.into())
            } else if self
                .cut
                .chosen
                .contains_key(&ObjectKey::Directory(id.into()))
            {
                ObjectKey::Directory(id.into())
            } else {
                ObjectKey::RootDirectory(id.into())
            };
            if let Some(gap) = self.ensure_object(&object)? {
                return Ok(Some(gap));
            }
            let Some(location) = self.cut.after_locations.get(&object) else {
                return Ok(Some(EvidenceGap::ExactInvocationInputs));
            };
            context
                .scope_bindings
                .insert(scope, crate::display(location));
        }
        let input = weftext_data::FrozenQueryInput {
            cut_id: snapshot.cut_id.clone(),
            schema_revision: invocation.schema_baseline.revision.clone(),
            coverage: "complete".into(),
            freshness: "stale".into(),
            notes,
            advanced: Some(context),
        };
        let (result, dependencies) =
            weftext_data::execute_authorized_query_with_provider_dependencies(
                &input,
                &invocation.plan,
                &invocation.baseline,
                &weftext_data::QueryBudget::default(),
                &BTreeMap::new(),
            )
            .map_err(|_| {
                error(
                    "queryInvocationUnavailable",
                    "The exact actual Query/provider invocation is invalid",
                )
            })?;
        if result.coverage != "complete" || !result.diagnostics.is_empty() {
            return Err(error(
                "queryInvocationUnavailable",
                "The exact actual Query/provider invocation is incomplete",
            ));
        }
        self.output
            .provider_dependencies
            .push(QueryValidationTrace {
                owner: invocation.owner.clone(),
                key: invocation.key.clone(),
                source_revision: invocation.source_revision.clone(),
                root_baseline: invocation.schema_baseline.clone(),
                definition: invocation.definition.clone(),
                dependencies,
            });
        self.output.query_membership = Some(members);
        Ok(None)
    }
}

pub(crate) fn capture_evidence(
    core: &Core,
    cut: &SuppliedEvidenceCut<'_>,
    checked: &CheckedRevalidation,
) -> Result<EvidenceCapture> {
    if cut.chosen != &checked.states
        || checked.required_evidence.len() > MAX_FILES
        || cut.objects.len() > MAX_FILES
        || cut.resources.len() > MAX_FILES
    {
        return Err(mismatch());
    }
    if core.require_workspace(cut.root)?.workspace_id != cut.graph.workspace_id {
        return Err(mismatch());
    }
    if cut.objects.keys().any(|key| !cut.chosen.contains_key(key)) {
        return Err(mismatch());
    }
    if cut.resources.iter().any(|resource| {
        resource.owner.workspace_id != cut.graph.workspace_id
            || !cut
                .chosen
                .contains_key(&ObjectKey::Node(resource.owner.node_id.clone()))
    }) {
        return Err(mismatch());
    }
    let mut capture = Capture {
        core,
        cut,
        checked,
        output: EvidenceCapture {
            witnesses: vec![],
            discharged: vec![],
            unfulfilled: vec![],
            read_objects: BTreeSet::new(),
            provider_dependencies: vec![],
            query_membership: None,
            annotation_targets: vec![],
        },
        objects: BTreeSet::new(),
        planned_objects: BTreeSet::new(),
    };
    // Only supplied observations participate. Never capture a missing one now
    // and treat that newer observation as the reviewed original basis.
    for witness in cut.snapshots {
        core.validate_snapshot_witness(cut.root, Path::new(&witness.binding.path), witness)?;
        capture.merge(vec![witness.clone()])?;
    }
    for obligation in checked.evidence_obligations() {
        let gap = match obligation {
            RequiredSemanticEvidence::Object {
                object,
                installed_revision,
            } => capture.object(object, installed_revision.as_deref())?,
            RequiredSemanticEvidence::Resource {
                owner,
                reference,
                file_revision,
            } => capture.resource(owner, reference, file_revision)?,
            RequiredSemanticEvidence::Definition {
                owner,
                key,
                source_revision,
            } => capture.definition(owner, key, source_revision)?,
            RequiredSemanticEvidence::AbsentConfiguration { object } => {
                capture.absent_configuration(object)?
            }
            RequiredSemanticEvidence::AnnotationTargets {
                owner,
                annotation_file_revision,
            } => capture.annotations(owner, annotation_file_revision)?,
            RequiredSemanticEvidence::QueryInvocation {
                owner,
                key,
                source_revision,
            } => capture.invocation(owner, key, source_revision)?,
        };
        if let Some(gap) = gap {
            capture.gap(obligation, gap);
        } else {
            capture.output.discharged.push(obligation.clone());
        }
    }
    // The writer still holds/rechecks all bindings through stage/commit. This
    // final check detects change during capture without substituting new bytes.
    for object in &capture.objects {
        match &cut.objects[object].qualification {
            ObjectQualification::Node(q) => {
                core.validate_query_node_witness(cut.root, &q.witness)?
            }
            ObjectQualification::Directory(q) => {
                core.validate_query_scope_witness(cut.root, &q.witness)?
            }
            ObjectQualification::Configuration => (),
        }
    }
    for witness in &capture.output.witnesses {
        let path = Path::new(&witness.binding.path);
        let control = [
            ".weftext-workspace.json",
            ".weftext-format",
            ".weftext-mvp-commit.json",
        ]
        .iter()
        .any(|name| same_locator(path, &cut.root.join(name)));
        let planned_negative = witness.binding.captured.is_none()
            && cut.chosen.iter().any(|(object, version)| {
                matches!(object, ObjectKey::Schema | ObjectKey::Rules)
                    && !cut.installed.contains_key(object)
                    && object_file(object, &version.state)
                        .is_some_and(|relative| same_locator(path, &cut.root.join(relative)))
                    && capture.planned_object(object).unwrap_or(false)
                    && cut
                        .planned
                        .and_then(|plan| plan.transaction.as_ref())
                        .is_some_and(|transaction| {
                            transaction.write_parents.iter().any(|(original, parent)| {
                                *parent == witness.binding.parent
                                    && Path::new(original)
                                        .parent()
                                        .zip(path.parent())
                                        .is_some_and(|(a, b)| same_locator(a, b))
                            })
                        })
            });
        let negative = planned_negative || witness.binding.captured.is_none()
            && checked.required_evidence.iter().any(|obligation| {
                matches!(obligation, RequiredSemanticEvidence::AbsentConfiguration { object }
                if match object {
                    ObjectKey::Schema => same_locator(path, &cut.root.join(".weftext-schema.json")),
                    ObjectKey::Rules => same_locator(path, &cut.root.join(".weftext-rules")),
                    _ => false,
                })
            });
        let mut original = false;
        if !control && !negative {
            for (object, local) in cut
                .installed
                .iter()
                .filter(|(object, _)| cut.chosen.contains_key(*object))
            {
                for relative in local.state.files.keys() {
                    if same_locator(path, &Path::new(&local.directory).join(relative)) {
                        if capture
                            .installed_snapshot_from(
                                std::slice::from_ref(witness),
                                object,
                                relative,
                            )?
                            .is_none()
                        {
                            return Err(mismatch());
                        }
                        original = true;
                    }
                }
            }
            if !original {
                return Err(mismatch());
            }
        }
        core.validate_snapshot_witness(cut.root, Path::new(&witness.binding.path), witness)?;
    }
    capture.output.read_objects = capture
        .objects
        .union(&capture.planned_objects)
        .cloned()
        .collect();
    Ok(capture.output)
}

#[cfg(all(test, windows))]
#[path = "revalidation_evidence_tests.rs"]
mod tests;
