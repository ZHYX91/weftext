use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::{TempDir, tempdir};
use weftext_core::{
    CreatedNode, LocalTaskRebaselineAuthority, NodeId, TASK_REBASELINE_MAX_BLOCKERS,
    TASK_REBASELINE_MAX_PLAN_JSON_BYTES, TASK_REBASELINE_MAX_TOTAL_EVIDENCE_BYTES,
    TaskNodePriority, TaskNodeState, TaskRebaselineAnnotationInventory, TaskRebaselineBlockerCode,
    TaskRebaselineError, TaskRebaselineOccurrenceDisposition, TaskRebaselinePlan,
    TaskRebaselineQueryDisposition, TaskRebaselineSourceKind,
    capture_local_task_rebaseline_authority, commit_workspace_transaction, create_child_node,
    create_workspace, decode_task_rebaseline_plan_json, plan_task_rebaseline, plan_trash_node_at,
    revalidate_task_rebaseline_plan, validate_task_rebaseline_plan,
};

const TASK_A: &str = "11111111-1111-4111-8111-111111111111";
const TASK_B: &str = "22222222-2222-4222-8222-222222222222";
const TASK_C: &str = "33333333-3333-4333-8333-333333333333";
const TASK_D: &str = "44444444-4444-4444-8444-444444444444";
const FIXTURE_NODE_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const TRASH_TIME: &str = "2026-08-25T12:00:00+08:00";

struct Fixture {
    _temporary: TempDir,
    root: CreatedNode,
    source: CreatedNode,
}

fn fixture(source: &str) -> Fixture {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("Tasks")).expect("workspace");
    let child = create_child_node(&root.path, "Source").expect("source node");
    fs::write(
        &child.document_path,
        source.replace(FIXTURE_NODE_ID, &child.id.to_string()),
    )
    .expect("fixture source");
    Fixture {
        _temporary: temporary,
        root,
        source: child,
    }
}

fn managed_source(node: &CreatedNode, title: &str, body: &str) -> String {
    format!(
        "---\nweftext:\n  id: \"{}\"\n---\n= {title}\n\n{body}",
        node.id
    )
}

fn blocker_codes(plan: &TaskRebaselinePlan) -> Vec<TaskRebaselineBlockerCode> {
    plan.blockers.iter().map(|blocker| blocker.code).collect()
}

fn legacy_id(index: usize) -> String {
    format!("00000000-0000-4000-8000-{index:012x}")
}

fn refresh_plan_digest(plan: &mut TaskRebaselinePlan) {
    plan.plan_digest.clear();
    let bytes = serde_json::to_vec(plan).expect("unsigned plan JSON");
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.task-rebaseline.plan/v1\0");
    hasher.update(bytes);
    plan.plan_digest = format!("{:x}", hasher.finalize());
}

fn context_mapping_source(count: usize) -> String {
    let suffix = "x".repeat(80);
    let mut body = String::new();
    for index in 1..=count {
        writeln!(
            &mut body,
            "* [ ] {{ctx}} Mapping {index:05} {suffix} task:[id={}]",
            legacy_id(index)
        )
        .expect("write mapping source");
    }
    managed_source(&create_dummy_node(), "Source", &body)
}

fn dependency_chain_source(count: usize) -> String {
    let mut body = format!("* [ ] CON task:[id={}]\n", legacy_id(1));
    for index in 2..=count {
        writeln!(
            &mut body,
            "* [ ] Chain {index} task:[id={},depends-on=\"{}\"]",
            legacy_id(index),
            legacy_id(index - 1)
        )
        .expect("write dependency source");
    }
    managed_source(&create_dummy_node(), "Source", &body)
}

fn indexed_valid_task_source(count: usize) -> String {
    let mut body = String::new();
    for index in 1..=count {
        writeln!(
            &mut body,
            "* [ ] Indexed task {index} task:[id={}]",
            legacy_id(index)
        )
        .expect("write indexed valid task");
    }
    managed_source(&create_dummy_node(), "Source", &body)
}

fn indexed_invalid_task_source(count: usize) -> String {
    let mut body = String::new();
    for index in 1..=count {
        writeln!(
            &mut body,
            "* [ ] Invalid task {index} task:[id={},priority=not-a-priority]",
            legacy_id(index)
        )
        .expect("write indexed invalid task");
    }
    managed_source(&create_dummy_node(), "Source", &body)
}

fn tree_bytes(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, current: &Path, entries: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        let mut children = fs::read_dir(current)
            .expect("read workspace tree")
            .collect::<Result<Vec<_>, _>>()
            .expect("workspace entries");
        children.sort_by_key(fs::DirEntry::file_name);
        for child in children {
            let path = child.path();
            let relative = path
                .strip_prefix(root)
                .expect("relative path")
                .to_path_buf();
            let file_type = child.file_type().expect("entry type");
            if file_type.is_dir() {
                entries.insert(relative, None);
                visit(root, &path, entries);
            } else {
                entries.insert(relative, Some(fs::read(path).expect("file bytes")));
            }
        }
    }

    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn assert_valid_field_preview(plan: &TaskRebaselinePlan) {
    let map = plan
        .identity_map
        .iter()
        .map(|mapping| (mapping.old_task_id.to_string(), mapping.generated_node_id))
        .collect::<BTreeMap<_, _>>();
    for (old, generated) in &map {
        assert_ne!(old, &generated.to_string());
    }
    let preview = plan.source_previews.first().expect("source preview");
    assert!(matches!(
        preview.annotations,
        TaskRebaselineAnnotationInventory::ConfirmedAbsent
    ));
    assert!(preview.original_source.contains("\r\n"));
    assert_eq!(preview.proposals.len(), 3);
    assert!(!preview.proposed_source.contains("task:["));
    assert_eq!(preview.proposed_source.matches("node:").count(), 3);

    let first = preview
        .proposals
        .iter()
        .find(|proposal| proposal.old_task_id.to_string() == TASK_A)
        .expect("first task proposal");
    assert_eq!(first.fields.state, TaskNodeState::InProgress);
    assert_eq!(first.fields.priority, Some(TaskNodePriority::High));
    assert_eq!(first.fields.created.as_deref(), Some("2026-08-01"));
    assert_eq!(first.fields.start.as_deref(), Some("2026-08-23"));
    assert_eq!(
        first.fields.scheduled.as_deref(),
        Some("2026-08-24T09:30:00+08:00")
    );
    assert_eq!(first.fields.due.as_deref(), Some("2026-09-01"));
    assert_eq!(first.fields.depends_on, [map[TASK_B]]);
    assert!(first.proposed_task_source.contains("= 发布计划\n"));
    assert!(
        first
            .proposed_task_source
            .contains(":weftext-task-depends-on: ")
    );

    let cancelled = preview
        .proposals
        .iter()
        .find(|proposal| proposal.old_task_id.to_string() == TASK_B)
        .expect("cancelled proposal");
    assert_eq!(cancelled.fields.state, TaskNodeState::Cancelled);
    assert_eq!(
        cancelled.fields.closed.as_deref(),
        Some("2026-08-25T10:00:00Z")
    );
    let checked_without_closed = preview
        .proposals
        .iter()
        .find(|proposal| proposal.old_task_id.to_string() == TASK_C)
        .expect("checked proposal");
    assert_eq!(
        checked_without_closed.fields.state,
        TaskNodeState::Completed
    );
    assert_eq!(checked_without_closed.fields.closed, None);
    assert!(
        !checked_without_closed
            .proposed_task_source
            .contains("weftext-task-closed")
    );
}

fn assert_closed_reviewed_contract(
    authority: &LocalTaskRebaselineAuthority,
    plan: &TaskRebaselinePlan,
    root: &Path,
) {
    let revalidated =
        revalidate_task_rebaseline_plan(authority, plan).expect("stable reviewed mapping");
    assert_eq!(&revalidated, plan);
    validate_task_rebaseline_plan(authority, plan).expect("valid reviewed preview");

    let serialized = serde_json::to_value(plan).expect("serialize plan");
    let serialized_text = serde_json::to_string(plan).expect("plan JSON");
    assert!(!serialized_text.contains(root.to_string_lossy().as_ref()));
    assert_eq!(
        decode_task_rebaseline_plan_json(serialized_text.as_bytes()).expect("bounded plan decode"),
        *plan
    );
    let round_trip: TaskRebaselinePlan =
        serde_json::from_value(serialized.clone()).expect("closed DTO round trip");
    assert_eq!(&round_trip, plan);
    let mut unknown = serialized.clone();
    unknown
        .as_object_mut()
        .expect("plan object")
        .insert("alternateParentNodeId".to_owned(), Value::Null);
    assert!(serde_json::from_value::<TaskRebaselinePlan>(unknown).is_err());
    let mut tampered = serialized;
    tampered["identityMap"][0]["destinationPortableName"] = Value::String("tampered".to_owned());
    let tampered: TaskRebaselinePlan = serde_json::from_value(tampered).expect("shape-valid plan");
    assert_eq!(
        validate_task_rebaseline_plan(authority, &tampered),
        Err(TaskRebaselineError::InvalidReviewedPlan)
    );
}

#[test]
fn valid_workspace_preview_maps_every_field_and_revalidation_reuses_fresh_ids() {
    let source = include_str!("../../../tests/fixtures/task-rebaseline-v1/valid-fields.adoc")
        .replace('\n', "\r\n");
    let fixture = fixture(&source);
    let before = tree_bytes(&fixture.root.path);
    let authority =
        capture_local_task_rebaseline_authority(&fixture.root.path).expect("local authority");
    assert!(!format!("{authority:?}").contains(fixture.root.path.to_string_lossy().as_ref()));

    let plan = plan_task_rebaseline(&authority).expect("read-only preview");
    assert!(plan.conversion_ready(), "{:#?}", plan.blockers);
    assert!(!plan.committable);
    assert!(plan.preview_only);
    assert!(plan.blockers.is_empty());
    assert_eq!(plan.identity_map.len(), 3);
    assert_eq!(plan.occurrences.len(), 3);
    assert_eq!(
        tree_bytes(&fixture.root.path),
        before,
        "planning wrote bytes"
    );
    assert_valid_field_preview(&plan);
    assert_closed_reviewed_contract(&authority, &plan, &fixture.root.path);
    assert_eq!(
        tree_bytes(&fixture.root.path),
        before,
        "validation wrote bytes"
    );
}

#[test]
fn schema_and_fixture_manifest_pin_the_closed_package_one_contract() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/task-rebaseline-v1.schema.json"
    ))
    .expect("valid task rebaseline schema JSON");
    assert_eq!(
        schema["$id"],
        "https://weftext.org/schemas/task-rebaseline-v1.schema.json"
    );
    assert_eq!(schema["additionalProperties"], false);
    for definition in [
        "preState",
        "blocker",
        "occurrence",
        "identityMapping",
        "taskFields",
        "proposal",
        "sourcePreview",
        "query",
    ] {
        assert_eq!(
            schema["$defs"][definition]["additionalProperties"], false,
            "open schema definition: {definition}"
        );
    }
    assert_eq!(schema["properties"]["committable"]["const"], false);
    assert_eq!(
        schema["properties"]["annotationReplicaCompleteness"]["const"],
        "complete_local_workspace"
    );
    assert_eq!(
        schema["properties"]["blockers"]["maxItems"],
        u64::try_from(TASK_REBASELINE_MAX_BLOCKERS).expect("blocker limit")
    );
    assert_eq!(
        schema["$defs"]["preState"]["properties"]["externalSnapshotBinding"]["const"],
        "not_provided"
    );
    assert!(
        schema["$defs"]["query"]["properties"]["disposition"]["enum"]
            .as_array()
            .expect("closed query disposition enum")
            .contains(&Value::String("canonical_unchanged".to_owned()))
    );
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/task-rebaseline-v1/manifest.json"
    ))
    .expect("valid fixture manifest");
    assert_eq!(manifest["profile"], "weftext.task-rebaseline/v1");
    assert_eq!(manifest["cases"].as_array().expect("cases").len(), 7);
}

#[test]
fn duplicate_cycle_recurrence_malformed_and_protected_evidence_block_without_edits() {
    let blocked = include_str!("../../../tests/fixtures/task-rebaseline-v1/blocked-inputs.adoc");
    let cycle = format!(
        "\n* [ ] Cycle C task:[id={TASK_C},depends-on=\"{TASK_D}\"]\n\
         * [ ] Cycle D task:[id={TASK_D},depends-on=\"{TASK_C}\"]\n"
    );
    let fixture = fixture(&format!("{blocked}{cycle}"));
    let original = fs::read_to_string(&fixture.source.document_path).expect("source bytes");
    let authority =
        capture_local_task_rebaseline_authority(&fixture.root.path).expect("local authority");
    let plan = plan_task_rebaseline(&authority).expect("blocked preview");
    let codes = blocker_codes(&plan);
    for code in [
        TaskRebaselineBlockerCode::DuplicateLegacyTaskId,
        TaskRebaselineBlockerCode::DependencyCycle,
        TaskRebaselineBlockerCode::RecurrenceUnsupported,
        TaskRebaselineBlockerCode::MalformedMacroResidue,
    ] {
        assert!(
            codes.contains(&code),
            "missing {code:?}: {:#?}",
            plan.blockers
        );
    }
    let protected = plan
        .occurrences
        .iter()
        .find(|occurrence| {
            occurrence.raw_macro.contains(TASK_C)
                && occurrence.disposition == TaskRebaselineOccurrenceDisposition::ProtectedLiteral
        })
        .expect("protected literal inventory");
    assert!(protected.blocker_codes.is_empty());
    assert!(!plan.conversion_ready());
    let preview = plan
        .source_previews
        .first()
        .expect("blocked source preview");
    assert_eq!(preview.proposed_source, original);
    assert!(preview.proposals.is_empty());
    assert_eq!(
        fs::read_to_string(&fixture.source.document_path).expect("unchanged source"),
        original
    );
}

#[test]
fn unresolved_ambiguous_self_dependencies_and_reserved_names_are_typed() {
    let missing = "99999999-9999-4999-8999-999999999999";
    let fixture = fixture(&managed_source(
        &create_dummy_node(),
        "Source",
        &format!(
            "* [ ] Missing target task:[id={TASK_A},depends-on=\"{missing}\"]\n\
             * [ ] Duplicate target A task:[id={TASK_B}]\n\
             * [ ] Duplicate target B task:[id={TASK_B}]\n\
             * [ ] Ambiguous source task:[id={TASK_C},depends-on=\"{TASK_B}\"]\n\
             * [ ] Self edge task:[id={TASK_D},depends-on=\"{TASK_D}\"]\n\
             * [ ] CON task:[id=55555555-5555-4555-8555-555555555555]\n"
        ),
    ));
    let authority =
        capture_local_task_rebaseline_authority(&fixture.root.path).expect("local authority");
    let plan = plan_task_rebaseline(&authority).expect("dependency diagnostics");
    let codes = blocker_codes(&plan);
    for code in [
        TaskRebaselineBlockerCode::UnresolvedDependency,
        TaskRebaselineBlockerCode::AmbiguousDependency,
        TaskRebaselineBlockerCode::SelfDependency,
        TaskRebaselineBlockerCode::DestinationNameUnavailable,
    ] {
        assert!(
            codes.contains(&code),
            "missing {code:?}: {:#?}",
            plan.blockers
        );
    }
    assert!(!plan.conversion_ready());
}

#[test]
fn nested_context_annotation_query_and_unicode_destination_conflicts_are_typed() {
    let fixture = fixture(&managed_source(
        &create_dummy_node(),
        "Source",
        &format!(
            "* [ ] Parent task:[id={TASK_A}]\n\
             ** [ ] Child task:[id={TASK_B}]\n\
             * [ ] Relative task:[id={TASK_C}]\n\
             +\n\
             image::relative.png[]\n\n\
             [query,source=tasks,view=task-list]\n\
             ....\n\
             where state = \"todo\"\n\
             ....\n"
        ),
    ));
    // `managed_source` above uses a stable fixture placeholder which `fixture` replaces.
    let collision_source =
        create_child_node(&fixture.root.path, "CollisionSource").expect("collision source");
    fs::write(
        &collision_source.document_path,
        managed_source(
            &collision_source,
            "CollisionSource",
            &format!("* [ ] Straße task:[id={TASK_D}]\n"),
        ),
    )
    .expect("collision source bytes");
    create_child_node(&collision_source.path, "STRASSE").expect("Unicode collision child");
    let authority =
        capture_local_task_rebaseline_authority(&fixture.root.path).expect("local authority");
    let plan = plan_task_rebaseline(&authority).expect("conservative preview");
    let codes = blocker_codes(&plan);
    assert!(codes.contains(&TaskRebaselineBlockerCode::NestedStructuredBranchOverlap));
    assert!(codes.contains(&TaskRebaselineBlockerCode::RelativeLocator));
    assert!(
        codes.contains(&TaskRebaselineBlockerCode::DestinationCollision),
        "{:#?}",
        plan.blockers
    );
    assert!(codes.contains(&TaskRebaselineBlockerCode::QueryPopulationEquivalenceUnproven));
    assert_eq!(plan.queries.len(), 1);
    assert_eq!(
        plan.queries[0].disposition,
        TaskRebaselineQueryDisposition::ConversionBlocked
    );
}

#[test]
fn private_legacy_query_inventory_keeps_duplicate_source_mentions_in_both_orders() {
    let fixture = fixture(&managed_source(
        &create_dummy_node(),
        "Source",
        &format!(
            concat!(
                "* [ ] Legacy task task:[id={}]\n\n",
                "[query,source=tasks,source=nodes]\n....\nwhere state = \"todo\"\n....\n\n",
                "[query,source=nodes,source=tasks]\n....\nwhere state = \"todo\"\n....\n",
            ),
            TASK_A,
        ),
    ));
    let authority =
        capture_local_task_rebaseline_authority(&fixture.root.path).expect("local authority");
    let plan = plan_task_rebaseline(&authority).expect("legacy inventory");
    assert_eq!(plan.queries.len(), 2);
    assert!(plan.queries.iter().all(|query| {
        query.disposition == TaskRebaselineQueryDisposition::InvalidBlocked
            && query.raw_source.starts_with("[query,")
    }));
    assert_eq!(
        plan.blockers
            .iter()
            .filter(|blocker| blocker.code == TaskRebaselineBlockerCode::InvalidTaskQuery)
            .count(),
        2
    );
}

#[test]
fn retired_canonical_unchanged_shape_decodes_but_cannot_be_revalidated() {
    let fixture = fixture(&managed_source(
        &create_dummy_node(),
        "Source",
        &format!(
            concat!(
                "* [ ] Legacy task task:[id={}]\n\n",
                "[query,source=tasks]\n....\nwhere state = \"todo\"\n....\n",
            ),
            TASK_A,
        ),
    ));
    let authority =
        capture_local_task_rebaseline_authority(&fixture.root.path).expect("local authority");
    let plan = plan_task_rebaseline(&authority).expect("legacy query inventory");
    assert_eq!(
        plan.queries[0].disposition,
        TaskRebaselineQueryDisposition::ConversionBlocked
    );
    let mut value = serde_json::to_value(&plan).expect("plan JSON value");
    value["queries"][0]["disposition"] = Value::String("canonical_unchanged".to_owned());
    let decoded: TaskRebaselinePlan =
        serde_json::from_value(value).expect("v1 canonical_unchanged remains deserializable");
    assert_eq!(
        decoded.queries[0].disposition,
        TaskRebaselineQueryDisposition::CanonicalUnchanged
    );
    assert_eq!(
        revalidate_task_rebaseline_plan(&authority, &decoded),
        Err(TaskRebaselineError::InvalidReviewedPlan)
    );
}

#[test]
fn valid_trash_payload_macros_require_restore_and_retained_ids_remain_occupied() {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("TrashTasks")).expect("workspace");
    let trashed = create_child_node(&root.path, "Trashed").expect("trash source");
    fs::write(
        &trashed.document_path,
        managed_source(
            &trashed,
            "Trashed",
            &format!("* [ ] Restore first task:[id={TASK_A}]\n"),
        ),
    )
    .expect("trash source bytes");
    commit_workspace_transaction(
        &plan_trash_node_at(&root.path, trashed.id, TRASH_TIME).expect("trash plan"),
    )
    .expect("trash commit");

    let active = create_child_node(&root.path, "Active").expect("active source");
    fs::write(
        &active.document_path,
        managed_source(
            &active,
            "Active",
            &format!("* [ ] Active task:[id={TASK_B}]\n"),
        ),
    )
    .expect("active source bytes");
    let authority = capture_local_task_rebaseline_authority(&root.path).expect("local authority");
    let plan = plan_task_rebaseline(&authority).expect("Trash-aware preview");
    assert!(blocker_codes(&plan).contains(&TaskRebaselineBlockerCode::TrashRestoreRequired));
    let trash_occurrence = plan
        .occurrences
        .iter()
        .find(|occurrence| occurrence.source_kind == TaskRebaselineSourceKind::TrashPayloadDocument)
        .expect("Trash occurrence");
    assert_eq!(
        trash_occurrence.disposition,
        TaskRebaselineOccurrenceDisposition::TrashRestoreRequired
    );
    assert_eq!(trash_occurrence.source_node_id, trashed.id);
    let generated = plan
        .identity_map
        .iter()
        .find(|mapping| mapping.old_task_id.to_string() == TASK_B)
        .expect("active mapping")
        .generated_node_id;
    assert_ne!(generated, trashed.id);
    assert_ne!(generated.to_string(), TASK_B);
    assert!(!plan.committable);
}

#[test]
fn failed_canonical_analysis_freezes_active_old_identity_without_trusting_protection() {
    let source = include_str!("../../../tests/fixtures/task-rebaseline-v1/parser-failed.adoc");
    let fixture = fixture(source);
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("failed-parser inventory");
    let occurrence = plan.occurrences.first().expect("legacy occurrence");
    assert_eq!(
        occurrence.old_task_id.map(|id| id.to_string()).as_deref(),
        Some(TASK_A)
    );
    assert_eq!(
        occurrence.disposition,
        TaskRebaselineOccurrenceDisposition::Blocked
    );
    assert!(
        occurrence
            .blocker_codes
            .contains(&TaskRebaselineBlockerCode::ParserAlignmentUnproven)
    );
    assert!(plan.identity_map.is_empty());
    assert!(blocker_codes(&plan).contains(&TaskRebaselineBlockerCode::ParserAlignmentUnproven));
}

#[test]
fn attached_table_abort_freezes_raw_id_and_rejects_reviewed_generated_id_collision() {
    let fixture = fixture(include_str!(
        "../../../tests/fixtures/task-rebaseline-v1/parser-abort-table.adoc"
    ));
    let valid = create_child_node(&fixture.root.path, "Valid").expect("valid source node");
    fs::write(
        &valid.document_path,
        managed_source(
            &valid,
            "Valid",
            &format!("* [ ] Convertible task:[id={TASK_B}]\n"),
        ),
    )
    .expect("valid task source");
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("parser-abort inventory");
    let aborted = plan
        .occurrences
        .iter()
        .find(|occurrence| occurrence.source_node_id == fixture.source.id)
        .expect("aborted source occurrence");
    assert_eq!(
        aborted.old_task_id.map(|id| id.to_string()).as_deref(),
        Some(TASK_A)
    );
    assert!(
        aborted
            .blocker_codes
            .contains(&TaskRebaselineBlockerCode::ParserAlignmentUnproven)
    );

    let mut reviewed = plan;
    let mapping = reviewed
        .identity_map
        .iter_mut()
        .find(|mapping| mapping.old_task_id.to_string() == TASK_B)
        .expect("valid task mapping");
    mapping.generated_node_id = TASK_A.parse::<NodeId>().expect("legacy task UUIDv4");
    refresh_plan_digest(&mut reviewed);
    assert_eq!(
        revalidate_task_rebaseline_plan(&authority, &reviewed),
        Err(TaskRebaselineError::InvalidReviewedPlan)
    );
}

#[test]
fn failed_canonical_analysis_in_trash_keeps_old_identity_and_requires_restore() {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("FailedTrash")).expect("workspace");
    let trashed = create_child_node(&root.path, "Trashed").expect("Trash source");
    let source = include_str!("../../../tests/fixtures/task-rebaseline-v1/parser-abort-table.adoc")
        .replace(FIXTURE_NODE_ID, &trashed.id.to_string());
    fs::write(&trashed.document_path, source).expect("failed parser source");
    commit_workspace_transaction(
        &plan_trash_node_at(&root.path, trashed.id, TRASH_TIME).expect("trash plan"),
    )
    .expect("trash commit");
    let authority =
        capture_local_task_rebaseline_authority(&root.path).expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("Trash failed-parser inventory");
    let occurrence = plan
        .occurrences
        .iter()
        .find(|item| item.source_kind == TaskRebaselineSourceKind::TrashPayloadDocument)
        .expect("Trash occurrence");
    assert_eq!(
        occurrence.old_task_id.map(|id| id.to_string()).as_deref(),
        Some(TASK_A)
    );
    assert!(
        occurrence
            .blocker_codes
            .contains(&TaskRebaselineBlockerCode::ParserAlignmentUnproven)
    );
    assert!(
        occurrence
            .blocker_codes
            .contains(&TaskRebaselineBlockerCode::TrashRestoreRequired)
    );
}

#[test]
fn principal_context_and_unicode_reserved_names_are_parser_owned_blockers() {
    let source = include_str!(
        "../../../tests/fixtures/task-rebaseline-v1/principal-context-and-reserved.adoc"
    );
    let fixture = fixture(source);
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("principal-context inventory");
    let codes = blocker_codes(&plan);
    for code in [
        TaskRebaselineBlockerCode::RelativeLocator,
        TaskRebaselineBlockerCode::DocumentContextDependency,
        TaskRebaselineBlockerCode::DestinationNameUnavailable,
    ] {
        assert!(
            codes.contains(&code),
            "missing {code:?}: {:#?}",
            plan.blockers
        );
    }
}

#[test]
fn prospective_content_rules_and_nfc_casefold_collisions_block_destinations() {
    let blocked = fixture(include_str!(
        "../../../tests/fixtures/task-rebaseline-v1/content-boundary.adoc"
    ));
    fs::write(
        blocked.root.path.join(".weftext-rules"),
        include_str!("../../../tests/fixtures/task-rebaseline-v1/content-boundary.rules"),
    )
    .expect("content rules");
    let authority = capture_local_task_rebaseline_authority(&blocked.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("content-boundary inventory");
    assert!(blocker_codes(&plan).contains(&TaskRebaselineBlockerCode::DestinationContentBoundary));

    let collision = fixture(include_str!(
        "../../../tests/fixtures/task-rebaseline-v1/nfc-collision.adoc"
    ));
    create_child_node(&collision.source.path, "CAFÉ").expect("NFC/casefold collision child");
    let authority = capture_local_task_rebaseline_authority(&collision.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("collision inventory");
    assert!(blocker_codes(&plan).contains(&TaskRebaselineBlockerCode::DestinationCollision));
}

#[test]
fn bounded_decoder_rejects_duplicate_keys_noncanonical_ids_and_byte_ceiling() {
    let fixture = fixture(include_str!(
        "../../../tests/fixtures/task-rebaseline-v1/valid-fields.adoc"
    ));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("valid plan");
    let json = serde_json::to_string(&plan).expect("plan JSON");
    let duplicate = format!("{{\"schema\":\"weftext.task-rebaseline/v1\",{}", &json[1..]);
    assert_eq!(
        decode_task_rebaseline_plan_json(duplicate.as_bytes()),
        Err(TaskRebaselineError::InvalidReviewedPlan)
    );
    let uppercase_id = fixture.source.id.to_string().to_uppercase();
    let noncanonical = json.replacen(&fixture.source.id.to_string(), &uppercase_id, 1);
    assert_eq!(
        decode_task_rebaseline_plan_json(noncanonical.as_bytes()),
        Err(TaskRebaselineError::InvalidReviewedPlan)
    );
    let mut hosted = serde_json::to_value(&plan).expect("plan value");
    hosted["annotationReplicaCompleteness"] = Value::String("complete_hosted_workspace".to_owned());
    assert_eq!(
        decode_task_rebaseline_plan_json(
            &serde_json::to_vec(&hosted).expect("hosted-completeness JSON")
        ),
        Err(TaskRebaselineError::InvalidReviewedPlan)
    );
    let oversized = vec![b' '; TASK_REBASELINE_MAX_PLAN_JSON_BYTES + 1];
    assert_eq!(
        decode_task_rebaseline_plan_json(&oversized),
        Err(TaskRebaselineError::ResourceLimitExceeded(
            "reviewed plan JSON bytes"
        ))
    );
}

#[test]
fn repeated_long_line_evidence_hits_total_ceiling_before_raw_item_clones() {
    let filler = "x".repeat(TASK_REBASELINE_MAX_TOTAL_EVIDENCE_BYTES / 2 + 1_024);
    let source = managed_source(
        &create_dummy_node(),
        "Source",
        &format!("* [ ] {filler} task:[id={TASK_A}] task:[id={TASK_B}]\n"),
    );
    let fixture = fixture(&source);
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    assert_eq!(
        plan_task_rebaseline(&authority),
        Err(TaskRebaselineError::ResourceLimitExceeded(
            "total occurrence evidence bytes"
        ))
    );
}

#[test]
fn many_protected_macros_are_inventoried_by_the_linear_physical_line_scanner() {
    let mut body = String::from("....\n");
    for index in 1..=2_000 {
        writeln!(&mut body, "literal task:[id={}]", legacy_id(index))
            .expect("write protected macro source");
    }
    body.push_str("....\n");
    let fixture = fixture(&managed_source(&create_dummy_node(), "Source", &body));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("many-macro inventory");
    assert_eq!(plan.occurrences.len(), 2_000);
    assert!(plan.occurrences.iter().all(|occurrence| {
        occurrence.disposition == TaskRebaselineOccurrenceDisposition::ProtectedLiteral
    }));
}

#[test]
fn many_document_annotation_observation_uses_each_authorized_node_path() {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("ManySidecars")).expect("workspace");
    for index in 1..=64 {
        let node =
            create_child_node(&root.path, &format!("Source{index:03}")).expect("source node");
        fs::write(
            &node.document_path,
            managed_source(
                &node,
                &format!("Source{index:03}"),
                &format!("* [ ] Task {index} task:[id={}]\n", legacy_id(index)),
            ),
        )
        .expect("task source");
    }
    let authority =
        capture_local_task_rebaseline_authority(&root.path).expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("many-document annotation inventory");
    assert_eq!(plan.source_previews.len(), 64);
    assert!(plan.source_previews.iter().all(|preview| matches!(
        preview.annotations,
        TaskRebaselineAnnotationInventory::ConfirmedAbsent
    )));
}

#[test]
fn long_blocked_dependency_chain_propagates_with_reverse_edge_queue() {
    let fixture = fixture(&dependency_chain_source(1_000));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("long dependency-chain inventory");
    assert!(blocker_codes(&plan).contains(&TaskRebaselineBlockerCode::DestinationNameUnavailable));
    assert_eq!(
        plan.blockers
            .iter()
            .filter(|blocker| blocker.code == TaskRebaselineBlockerCode::UnresolvedDependency)
            .count(),
        999
    );
}

#[test]
fn aggregate_mapping_and_blocker_payload_hits_exact_json_budget() {
    let fixture = fixture(&context_mapping_source(9_000));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    assert_eq!(
        plan_task_rebaseline(&authority),
        Err(TaskRebaselineError::ResourceLimitExceeded(
            "reviewed plan JSON bytes"
        ))
    );
}

#[test]
fn blocker_count_limit_fails_before_an_eleventh_thousand_blocker_is_retained() {
    let mut body = String::new();
    for _ in 0..=TASK_REBASELINE_MAX_BLOCKERS {
        body.push_str("residue task:[broken]\n");
    }
    let fixture = fixture(&managed_source(&create_dummy_node(), "Source", &body));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    assert_eq!(
        plan_task_rebaseline(&authority),
        Err(TaskRebaselineError::ResourceLimitExceeded("blocker count"))
    );
}

#[test]
fn many_valid_candidates_use_range_indexed_parser_alignment() {
    let fixture = fixture(&indexed_valid_task_source(2_000));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("indexed valid candidate plan");
    assert_eq!(plan.occurrences.len(), 2_000);
    assert_eq!(plan.identity_map.len(), 2_000);
    assert!(plan.conversion_ready);
}

#[test]
fn many_invalid_candidates_use_range_indexed_diagnostics() {
    let fixture = fixture(&indexed_invalid_task_source(2_000));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("indexed invalid candidate plan");
    assert_eq!(plan.occurrences.len(), 2_000);
    assert_eq!(
        plan.blockers
            .iter()
            .filter(|blocker| blocker.code == TaskRebaselineBlockerCode::InvalidLegacyTask)
            .count(),
        2_000
    );
}

#[test]
fn lexical_occurrence_limit_precedes_canonical_and_legacy_analysis() {
    let raw = format!("task:[id={TASK_A}]\n").repeat(100_001);
    let fixture = fixture(&managed_source(&create_dummy_node(), "Source", &raw));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    assert_eq!(
        plan_task_rebaseline(&authority),
        Err(TaskRebaselineError::ResourceLimitExceeded(
            "legacy lexical task start count"
        ))
    );
}

#[test]
fn destination_content_rule_work_budget_precedes_candidate_rule_product() {
    let fixture = fixture(&indexed_valid_task_source(64));
    let mut rules = String::from("weftext-content-rules-v1\n");
    for index in 0..500 {
        writeln!(&mut rules, "ignore nowhere/{index:04}/**").expect("write content rule");
    }
    fs::write(fixture.root.path.join(".weftext-rules"), rules).expect("content rules");
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    assert_eq!(
        plan_task_rebaseline(&authority),
        Err(TaskRebaselineError::ResourceLimitExceeded(
            "destination content-rule match work"
        ))
    );
}

#[test]
fn embedded_lexical_task_start_storm_is_rejected_before_analysis() {
    let body = format!("{}]\n", "task:[".repeat(300_000));
    assert!(body.len() < 2 * 1024 * 1024);
    let fixture = fixture(&managed_source(&create_dummy_node(), "Source", &body));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    assert_eq!(
        plan_task_rebaseline(&authority),
        Err(TaskRebaselineError::ResourceLimitExceeded(
            "legacy task analysis scan work"
        ))
    );
}

#[test]
fn trash_attribute_separator_storm_is_rejected_before_analysis() {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("TrashAttributeStorm")).expect("workspace");
    let trashed = create_child_node(&root.path, "Trashed").expect("Trash source");
    let attributes = "unknown=value,".repeat(10_001);
    let body = format!("* [ ] Storm task:[id={TASK_A},{attributes}last=value]\n");
    fs::write(
        &trashed.document_path,
        managed_source(&trashed, "Trashed", &body),
    )
    .expect("attribute-storm source");
    commit_workspace_transaction(
        &plan_trash_node_at(&root.path, trashed.id, TRASH_TIME).expect("trash plan"),
    )
    .expect("trash commit");
    let authority =
        capture_local_task_rebaseline_authority(&root.path).expect("complete local authority");
    assert_eq!(
        plan_task_rebaseline(&authority),
        Err(TaskRebaselineError::ResourceLimitExceeded(
            "legacy task metadata separator work"
        ))
    );
}

#[test]
fn active_literal_dependency_space_storm_is_one_bounded_invalid_dependency() {
    let depends_on = " ".repeat(2_000_000);
    let body = format!("* [ ] Storm task:[id={TASK_A},depends-on=\"{depends_on}\"]\n");
    let fixture = fixture(&managed_source(&create_dummy_node(), "Source", &body));
    let authority = capture_local_task_rebaseline_authority(&fixture.root.path)
        .expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("bounded literal-space plan");
    assert_eq!(plan.occurrences.len(), 1);
    assert_eq!(
        plan.blockers
            .iter()
            .filter(|blocker| blocker.code == TaskRebaselineBlockerCode::InvalidLegacyTask)
            .count(),
        1
    );
    assert_eq!(
        plan.occurrences[0]
            .old_task_id
            .map(|id| id.to_string())
            .as_deref(),
        Some(TASK_A)
    );
}

#[test]
fn trash_escaped_dependency_space_storm_uses_the_same_bounded_parser() {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("TrashDependencyStorm")).expect("workspace");
    let trashed = create_child_node(&root.path, "Trashed").expect("Trash source");
    let depends_on = "\\u0020".repeat(300_000);
    let body = format!("* [ ] Storm task:[id={TASK_A},depends-on=\"{depends_on}\"]\n");
    fs::write(
        &trashed.document_path,
        managed_source(&trashed, "Trashed", &body),
    )
    .expect("escaped dependency-storm source");
    commit_workspace_transaction(
        &plan_trash_node_at(&root.path, trashed.id, TRASH_TIME).expect("trash plan"),
    )
    .expect("trash commit");
    let authority =
        capture_local_task_rebaseline_authority(&root.path).expect("complete local authority");
    let plan = plan_task_rebaseline(&authority).expect("bounded escaped-space Trash plan");
    let occurrence = plan
        .occurrences
        .iter()
        .find(|occurrence| occurrence.source_kind == TaskRebaselineSourceKind::TrashPayloadDocument)
        .expect("Trash dependency storm occurrence");
    assert_eq!(
        occurrence.old_task_id.map(|id| id.to_string()).as_deref(),
        Some(TASK_A)
    );
    assert_eq!(
        occurrence.disposition,
        TaskRebaselineOccurrenceDisposition::TrashRestoreRequired
    );
}

fn create_dummy_node() -> CreatedNode {
    CreatedNode {
        id: FIXTURE_NODE_ID.parse().expect("fixture node ID"),
        path: PathBuf::new(),
        document_path: PathBuf::new(),
    }
}
