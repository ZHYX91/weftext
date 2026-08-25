use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;
use weftext_core::{
    TASK_IMPORT_PROFILE_ID, TaskId, TaskImportDiagnosticCode, TaskImportDialect,
    TaskImportDocumentInput, TaskImportEditKind, TaskImportSettings, TaskImportSettingsError,
    TaskImportStatusMapping, TaskImportStatusType, analyze_query_source, analyze_task_source,
    plan_task_import, validate_task_import_plan,
};

fn obsidian_settings() -> TaskImportSettings {
    TaskImportSettings {
        dialect: TaskImportDialect::ObsidianTasksEmojiV1,
        plugin_version: Some("8.2.0".to_owned()),
        global_filter: Some("#task".to_owned()),
        indentation_width: 4,
        statuses: vec![
            status(' ', "Todo", TaskImportStatusType::Todo),
            status('x', "Done", TaskImportStatusType::Done),
            status('/', "In Progress", TaskImportStatusType::InProgress),
            status('-', "Cancelled", TaskImportStatusType::Cancelled),
            status('?', "Not a task", TaskImportStatusType::NonTask),
        ],
    }
}

fn status(symbol: char, name: &str, status_type: TaskImportStatusType) -> TaskImportStatusMapping {
    TaskImportStatusMapping {
        symbol,
        name: name.to_owned(),
        status_type,
    }
}

#[test]
fn workspace_preview_resolves_ids_dependencies_statuses_and_queries() {
    let documents = vec![
        TaskImportDocumentInput {
            locator: "Project.md".to_owned(),
            source: concat!(
                "# Project\n",
                "- [ ] #task Plain\n",
                "    - [/] #task Work 🔺 📅 2026-09-05 🔁 every week on Monday, Friday 🆔 work\n",
                "- [ ] Not imported\n",
                "- [?] #task Informational\n",
                "```tasks\n",
                "not done\n",
                "due before tomorrow\n",
                "sort by due\n",
                "limit to 20 tasks\n",
                "```\n",
            )
            .to_owned(),
        },
        TaskImportDocumentInput {
            locator: "Project/Done.md".to_owned(),
            source: "- [-] #task Cancelled ❌ 2026-09-01 ⛔ work 🆔 cancelled\n".to_owned(),
        },
    ];

    let plan = plan_task_import(&documents, obsidian_settings()).expect("preview");
    assert_eq!(plan.profile, TASK_IMPORT_PROFILE_ID);
    assert!(plan.is_committable(), "{:?}", plan.diagnostics);
    assert_eq!(plan.identities.len(), 2);
    assert_eq!(
        plan.identities
            .iter()
            .filter_map(|identity| identity.legacy_id.as_deref())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["cancelled", "work"])
    );
    assert_ne!(plan.identities[0].task_id, plan.identities[1].task_id);

    let project = &plan.documents[0];
    assert!(project.proposed_source.contains("* [ ] Plain\n"));
    assert!(project.proposed_source.contains("** [ ] Work task:[id="));
    assert!(project.proposed_source.contains("phase=in-progress"));
    assert!(project.proposed_source.contains("priority=highest"));
    assert!(project.proposed_source.contains("due=2026-09-05"));
    assert!(
        project
            .proposed_source
            .contains("rrule=\"FREQ=WEEKLY;BYDAY=MO,FR\",repeat-from=due")
    );
    assert!(project.proposed_source.contains("- \\[ ] Not imported\n"));
    assert!(
        project
            .proposed_source
            .contains("- \\[?] #task Informational\n")
    );
    assert!(
        project
            .proposed_source
            .contains("[.weftext-query,version=1,view=task-list]\n....\n")
    );
    assert!(
        project
            .proposed_source
            .contains("where task.closed = false and task.due < context.today + P1D\n")
    );
    assert!(
        project
            .proposed_source
            .contains("order by task.due asc nulls last\n")
    );
    assert!(project.proposed_source.contains("limit 20\n"));
    let queries = analyze_query_source(&project.proposed_source);
    assert!(queries.diagnostics.is_empty(), "{:?}", queries.diagnostics);
    assert_eq!(queries.blocks.len(), 1);
    assert!(queries.blocks[0].plan.is_some());
    assert!(
        project
            .edits
            .iter()
            .any(|edit| edit.kind == TaskImportEditKind::ExcludedChecklist)
    );

    let cancelled = &plan.documents[1].proposed_source;
    let work_id = plan
        .identities
        .iter()
        .find(|identity| identity.legacy_id.as_deref() == Some("work"))
        .expect("work mapping")
        .task_id;
    assert!(cancelled.contains("* [x] Cancelled task:[id="));
    assert!(cancelled.contains("resolution=cancelled"));
    assert!(cancelled.contains("closed=2026-09-01"));
    assert!(cancelled.contains(&format!("depends-on=\"{work_id}\"")));
    assert!(
        analyze_task_source(&project.proposed_source)
            .diagnostics
            .is_empty()
    );
    assert!(
        analyze_query_source(&project.proposed_source)
            .diagnostics
            .is_empty()
    );
}

#[test]
fn blockers_preserve_affected_source_and_cover_cross_document_identity() {
    let documents = vec![
        TaskImportDocumentInput {
            locator: "A.md".to_owned(),
            source: concat!(
                "- [ ] #task First 🆔 repeated\n",
                "- [ ] #task Bad date 📅 2026-02-30\n",
                "- [ ] #task Unsupported 🔁 every weekday 📅 2026-09-01\n",
                "- [ ] #task Reminder ⏰ 2026-09-01 10:00\n",
                "- [ ] #task Self ⛔ self 🆔 self\n",
                "- [ ] #task Cycle A ⛔ cycle-b 🆔 cycle-a\n",
                "```tasks\n",
                "filter by function task.isDone\n",
                "```\n",
            )
            .to_owned(),
        },
        TaskImportDocumentInput {
            locator: "B.md".to_owned(),
            source: concat!(
                "- [ ] #task Second 🆔 repeated\n",
                "- [ ] #task Dependent ⛔ repeated\n",
                "- [ ] #task Cycle B ⛔ cycle-a 🆔 cycle-b\n",
                "- [!] #task Unknown\n",
            )
            .to_owned(),
        },
    ];
    let plan = plan_task_import(&documents, obsidian_settings()).expect("blocked preview");
    assert!(!plan.is_committable());
    let codes = plan
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<BTreeSet<_>>();
    assert!(codes.contains(&TaskImportDiagnosticCode::DuplicateLegacyId));
    assert!(codes.contains(&TaskImportDiagnosticCode::AmbiguousDependency));
    assert!(codes.contains(&TaskImportDiagnosticCode::InvalidDate));
    assert!(codes.contains(&TaskImportDiagnosticCode::UnsupportedRecurrence));
    assert!(codes.contains(&TaskImportDiagnosticCode::UnsupportedMetadata));
    assert!(codes.contains(&TaskImportDiagnosticCode::UnknownStatus));
    assert!(codes.contains(&TaskImportDiagnosticCode::UnsafeQueryInstruction));
    assert!(codes.contains(&TaskImportDiagnosticCode::SelfDependency));
    assert!(codes.contains(&TaskImportDiagnosticCode::DependencyCycle));
    assert!(
        plan.documents[0]
            .proposed_source
            .contains("- [ ] #task Bad date 📅 2026-02-30")
    );
    assert!(
        plan.documents[0]
            .proposed_source
            .contains("filter by function task.isDone")
    );
}

#[test]
fn plain_markdown_and_protected_ranges_are_exact_and_identity_free() {
    let source = concat!(
        "---\n",
        "examples:\n",
        "  - [ ] frontmatter literal\n",
        "---\n",
        "- [ ] 中文\r\n",
        "    + [X] العربية\n",
        "```text\n",
        "- [ ] literal\n",
        "```\r",
        "<!--\n",
        "- [ ] html comment\n",
        "-->\n",
        "> - [ ] quoted",
    );
    let plan = plan_task_import(
        &[TaskImportDocumentInput {
            locator: "mixed.md".to_owned(),
            source: source.to_owned(),
        }],
        TaskImportSettings::markdown_checklist_v1(4),
    )
    .expect("Markdown preview");
    assert!(plan.is_committable(), "{:?}", plan.diagnostics);
    assert!(plan.identities.is_empty());
    assert_eq!(
        plan.documents[0].proposed_source,
        concat!(
            "---\n",
            "examples:\n",
            "  - [ ] frontmatter literal\n",
            "---\n",
            "* [ ] 中文\r\n",
            "** [x] العربية\n",
            "```text\n",
            "- [ ] literal\n",
            "```\r",
            "<!--\n",
            "- [ ] html comment\n",
            "-->\n",
            "> - [ ] quoted",
        )
    );
}

#[test]
fn settings_and_locators_fail_closed_before_planning() {
    let mut settings = obsidian_settings();
    settings.plugin_version = Some(" ".to_owned());
    assert_eq!(
        plan_task_import(&[], settings),
        Err(TaskImportSettingsError::MissingPluginVersion)
    );

    let documents = vec![
        TaskImportDocumentInput {
            locator: "same.md".to_owned(),
            source: String::new(),
        },
        TaskImportDocumentInput {
            locator: "same.md".to_owned(),
            source: String::new(),
        },
    ];
    assert!(matches!(
        plan_task_import(&documents, obsidian_settings()),
        Err(TaskImportSettingsError::DuplicateLocator(locator)) if locator == "same.md"
    ));
}

#[test]
fn frozen_review_validation_reuses_exact_identity_mappings_without_minting() {
    let documents = vec![TaskImportDocumentInput {
        locator: "任务.md".to_owned(),
        source: "- [ ] #task 编写 📅 2026-09-05 🆔 write\r\n".to_owned(),
    }];
    let plan = plan_task_import(&documents, obsidian_settings()).expect("frozen preview");
    validate_task_import_plan(&documents, &plan).expect("exact reviewed plan");

    let mut changed_source = documents.clone();
    changed_source[0].source.push_str("concurrent bytes\n");
    assert_eq!(
        validate_task_import_plan(&changed_source, &plan),
        Err(TaskImportSettingsError::InvalidReviewedPlan)
    );

    let mut changed_identity = plan.clone();
    changed_identity.identities[0].task_id = TaskId::new();
    assert_eq!(
        validate_task_import_plan(&documents, &changed_identity),
        Err(TaskImportSettingsError::InvalidReviewedPlan)
    );

    let mut changed_edit = plan.clone();
    changed_edit.documents[0]
        .proposed_source
        .push_str("forged\n");
    assert_eq!(
        validate_task_import_plan(&documents, &changed_edit),
        Err(TaskImportSettingsError::InvalidReviewedPlan)
    );
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureManifest {
    profile: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCase {
    id: String,
    settings: TaskImportSettings,
    documents: Vec<FixtureDocument>,
    committable: bool,
    identities: usize,
    #[serde(default)]
    codes: Vec<String>,
    #[serde(default)]
    contains: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureDocument {
    locator: String,
    source: String,
}

#[test]
fn machine_readable_import_corpus_is_frozen() {
    let root = fixture_root();
    let source = fs::read_to_string(root.join("manifest.json")).expect("fixture manifest");
    let manifest: FixtureManifest = serde_json::from_str(&source).expect("parse fixture manifest");
    assert_eq!(manifest.profile, TASK_IMPORT_PROFILE_ID);
    assert!(manifest.cases.len() >= 4);
    for case in manifest.cases {
        let documents = case
            .documents
            .into_iter()
            .map(|document| TaskImportDocumentInput {
                locator: document.locator,
                source: document.source,
            })
            .collect::<Vec<_>>();
        let plan = plan_task_import(&documents, case.settings)
            .unwrap_or_else(|error| panic!("fixture {} settings: {error}", case.id));
        assert_eq!(plan.is_committable(), case.committable, "{}", case.id);
        assert_eq!(plan.identities.len(), case.identities, "{}", case.id);
        let actual_codes = plan
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic_code(diagnostic.code))
            .collect::<BTreeSet<_>>();
        for code in case.codes {
            assert!(actual_codes.contains(code.as_str()), "{}: {code}", case.id);
        }
        let combined = plan
            .documents
            .iter()
            .map(|document| document.proposed_source.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for expected in case.contains {
            assert!(combined.contains(&expected), "{}: {expected}", case.id);
        }
    }
}

#[test]
fn import_settings_schema_is_closed_and_versioned() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/task-import-v1.schema.json"),
    )
    .expect("task import schema");
    let schema: Value = serde_json::from_str(&source).expect("parse task import schema");
    assert_eq!(
        schema["$id"],
        "https://weftext.org/schemas/task-import-v1.schema.json"
    );
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["dialect"]["enum"],
        serde_json::json!(["markdown_checklist_v1", "obsidian_tasks_emoji_v1"])
    );
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/task-import-v1")
}

const fn diagnostic_code(code: TaskImportDiagnosticCode) -> &'static str {
    match code {
        TaskImportDiagnosticCode::MisalignedIndentation => "misaligned_indentation",
        TaskImportDiagnosticCode::UnknownStatus => "unknown_status",
        TaskImportDiagnosticCode::AmbiguousGlobalFilter => "ambiguous_global_filter",
        TaskImportDiagnosticCode::DuplicateMetadata => "duplicate_metadata",
        TaskImportDiagnosticCode::InvalidDate => "invalid_date",
        TaskImportDiagnosticCode::InvalidState => "invalid_state",
        TaskImportDiagnosticCode::UnsupportedMetadata => "unsupported_metadata",
        TaskImportDiagnosticCode::UnsupportedRecurrence => "unsupported_recurrence",
        TaskImportDiagnosticCode::MissingRecurrenceAnchor => "missing_recurrence_anchor",
        TaskImportDiagnosticCode::DuplicateLegacyId => "duplicate_legacy_id",
        TaskImportDiagnosticCode::UnresolvedDependency => "unresolved_dependency",
        TaskImportDiagnosticCode::AmbiguousDependency => "ambiguous_dependency",
        TaskImportDiagnosticCode::SelfDependency => "self_dependency",
        TaskImportDiagnosticCode::DependencyCycle => "dependency_cycle",
        TaskImportDiagnosticCode::EmptyStructuredDescription => "empty_structured_description",
        TaskImportDiagnosticCode::UnterminatedTasksQuery => "unterminated_tasks_query",
        TaskImportDiagnosticCode::UnsafeQueryInstruction => "unsafe_query_instruction",
        TaskImportDiagnosticCode::UnsupportedQueryInstruction => "unsupported_query_instruction",
        TaskImportDiagnosticCode::TargetValidation => "target_validation",
    }
}
