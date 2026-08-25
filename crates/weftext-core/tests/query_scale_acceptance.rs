use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use weftext_core::{
    CalendarDate, QueryAccessScope, QueryEvaluationContext, QueryExecutionBinding,
    QueryWorkspaceIndex, TaskNodeTemporal, analyze_query_source,
};

const CHILD_NODE_COUNT: usize = 500;
const CHECKLISTS_PER_NODE: usize = 19;
const EXPECTED_TASK_COUNT: usize = CHILD_NODE_COUNT * (CHECKLISTS_PER_NODE + 1);
const DEBUG_REBUILD_BUDGET: Duration = Duration::from_secs(30);
const DEBUG_EXECUTION_BUDGET: Duration = Duration::from_secs(3);

#[test]
fn ten_thousand_task_workspace_stays_within_the_declared_debug_budget() {
    let temporary = tempdir().expect("large-workspace fixture root");
    let workspace = temporary.path().join("Workspace");
    fs::create_dir(&workspace).expect("workspace directory");
    fs::write(workspace.join(".weftext-format"), "weftext.asciidoc.v1\n")
        .expect("exact workspace marker");
    write_node(&workspace, "Workspace", &canonical_uuid(1), "= Workspace\n");
    for node_index in 0..CHILD_NODE_COUNT {
        let name = format!("Node-{node_index:04}");
        let day = node_index % 28 + 1;
        let mut body = format!(
            "= {name}\n:weftext-task: v1\n:weftext-task-state: todo\n:weftext-task-due: 2026-09-{day:02}\n\n"
        );
        for task_index in 0..CHECKLISTS_PER_NODE {
            let ordinal = node_index * CHECKLISTS_PER_NODE + task_index;
            writeln!(&mut body, "* [ ] Checklist {ordinal:05}")
                .expect("format canonical checklist");
        }
        write_node(
            &workspace.join(&name),
            &name,
            &canonical_uuid(node_index + 2),
            &body,
        );
    }

    let rebuild_started = Instant::now();
    let index = QueryWorkspaceIndex::rebuild(&workspace).expect("large query index");
    let rebuild_elapsed = rebuild_started.elapsed();
    assert!(
        rebuild_elapsed <= DEBUG_REBUILD_BUDGET,
        "debug index rebuild exceeded {DEBUG_REBUILD_BUDGET:?}: {rebuild_elapsed:?}"
    );

    let source = concat!(
        "[.weftext-query,version=1,view=task-list]\n",
        "....\n",
        "from tasks as task\n",
        "scope workspace\n",
        "where task.closed = false\n",
        "select task.kind, task.title, task.due, task.owner_node.path\n",
        "order by task.due asc nulls last, task.owner_node.path asc, task.title asc\n",
        "limit 1000\n",
        "...."
    );
    let analysis = analyze_query_source(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    let plan = analysis.blocks[0].plan.as_ref().expect("typed query plan");
    let access = QueryAccessScope::complete(index.node_ids());
    let context = QueryEvaluationContext::new(
        CalendarDate::new(2026, 8, 24).expect("fixed date"),
        TaskNodeTemporal::parse("2026-08-24T09:30:00+08:00").expect("fixed instant"),
        "Asia/Shanghai".to_owned(),
        "zh-CN".to_owned(),
        QueryExecutionBinding {
            node_id: None,
            heading: None,
        },
    )
    .expect("fixed query context");

    let first_started = Instant::now();
    let first = index
        .execute(plan, &access, &context)
        .expect("large deterministic query");
    let first_elapsed = first_started.elapsed();
    assert!(
        first_elapsed <= DEBUG_EXECUTION_BUDGET,
        "debug query exceeded {DEBUG_EXECUTION_BUDGET:?}: {first_elapsed:?}"
    );
    assert_eq!(first.total_before_limit, EXPECTED_TASK_COUNT);
    assert_eq!(first.rows.len(), 1_000);
    assert!(first.truncated);

    let repeat_started = Instant::now();
    let repeat = index
        .execute(plan, &access, &context)
        .expect("repeat large deterministic query");
    let repeat_elapsed = repeat_started.elapsed();
    assert!(
        repeat_elapsed <= DEBUG_EXECUTION_BUDGET,
        "repeat debug query exceeded {DEBUG_EXECUTION_BUDGET:?}: {repeat_elapsed:?}"
    );
    assert_eq!(repeat, first);
}

fn write_node(directory: &Path, name: &str, node_id: &str, body: &str) {
    fs::create_dir_all(directory).expect("node directory");
    let source = format!("---\nweftext:\n  id: \"{node_id}\"\n---\n{body}");
    fs::write(directory.join(format!("{name}.adoc")), source).expect("canonical node source");
}

fn canonical_uuid(value: usize) -> String {
    format!("20000000-0000-4000-8000-{value:012x}")
}
