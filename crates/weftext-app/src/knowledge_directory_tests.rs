//! Real local Directory operations keep child documents in the existing
//! namespace-result contract; the shared editor/draft is never replaced here.
use super::*;
#[cfg(test)]
use crate::CoreTestFixture;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use weftext_core::{Core, OpenedDocument, SaveRequest};

struct Fixture {
    _temp: tempfile::TempDir,
    state: AppState,
    root: PathBuf,
    source: PathBuf,
    target: PathBuf,
    documents: Vec<Value>,
    contexts: Vec<Context>,
    histories: Vec<Vec<String>>,
}

fn admit_directory(core: &mut Core, root: &Path, directory: &Path) {
    fs::create_dir(directory).unwrap();
    let admission = core
        .prepare_directory_admission(root, directory, &uuid::Uuid::new_v4().to_string())
        .unwrap();
    core.execute_operation(&admission.preview.operation_id, &admission.preview.plan_id)
        .unwrap();
}

fn note(core: &mut Core, root: &Path, parent: &Path, name: &str) -> OpenedDocument {
    let opened = core.create_note_fixture(root, parent, name).unwrap();
    core.save(SaveRequest {
        session_id: opened.session_id,
        base_id: opened.base_id,
        source: format!("{}\nSaved {name}", opened.source),
        operation_id: uuid::Uuid::new_v4().to_string(),
    })
    .unwrap();
    core.open(opened.path, Some(root.to_path_buf())).unwrap()
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let source = root.join("Folder");
    admit_directory(&mut core, &root, &source);
    let nested = source.join("Nested");
    admit_directory(&mut core, &root, &nested);
    let target = root.join("Target");
    core.create_note_fixture(&root, &root, "Target").unwrap();
    let first = note(&mut core, &root, &source, "First");
    let second = note(&mut core, &root, &nested, "Second");
    let outside = note(&mut core, &root, &root, "FolderSibling");
    let mut contexts = Vec::new();
    let mut documents = Vec::new();
    let mut histories = Vec::new();
    for (index, document) in [first, second, outside].into_iter().enumerate() {
        let dirty = index < 2;
        let source = if dirty {
            format!("{}\nUnsaved draft {index}", document.source)
        } else {
            document.source.clone()
        };
        if dirty {
            core.persist_draft(&document.session_id, &source).unwrap();
        }
        histories.push(
            core.list_history(&document.session_id)
                .unwrap()
                .versions
                .into_iter()
                .map(|version| version.version_id)
                .collect(),
        );
        contexts.push(Context {
            session_id: document.session_id.clone(),
            source,
            serial: 7 + index as u64,
            base_id: document.base_id.clone(),
            dirty,
        });
        documents.push(value(document).unwrap());
    }
    let state = AppState {
        core: Arc::new(Mutex::new(core)),
        token: Arc::from("test-token"),
        port: 0,
        locations: Arc::new(Mutex::new(HashMap::new())),
        renders: Arc::new(Mutex::new(HashMap::new())),
        editor_versions: Arc::new(Mutex::new(HashMap::new())),
        exports: Arc::new(Mutex::new(crate::export_api::ExportStore::default())),
        render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
        sync_remote: Arc::new(Mutex::new(crate::remote_sync::Store::default())),
        initial: None,
    };
    for document in &documents {
        remember(&state, document).unwrap();
    }
    for context in &contexts {
        observe(&state, context).unwrap();
    }
    Fixture {
        _temp: temp,
        state,
        root,
        source,
        target,
        documents,
        contexts,
        histories,
    }
}

async fn relocate(fixture: &Fixture, context: Option<Context>) -> (Value, String) {
    let operation_id = uuid::Uuid::new_v4().to_string();
    let plan = prepare_operation(
        State(fixture.state.clone()),
        Json(OperationRequest {
            operation_id: operation_id.clone(),
            root: fixture.root.to_string_lossy().into_owned(),
            intent: OperationIntent::RelocateDirectory {
                source: fixture.source.to_string_lossy().into_owned(),
                parent: fixture.target.to_string_lossy().into_owned(),
                name: "Moved".into(),
            },
        }),
    )
    .await
    .unwrap()
    .0;
    let receipt = commit(
        State(fixture.state.clone()),
        Json(CommitRequest {
            operation_id: operation_id.clone(),
            plan_id: plan["planId"].as_str().unwrap().into(),
            context,
        }),
    )
    .await
    .unwrap()
    .0;
    (receipt, operation_id)
}

async fn assert_rebound(fixture: &Fixture, document: &Value, index: usize, relative: &str) {
    let expected = fixture.target.join("Moved").join(relative);
    assert_eq!(
        Path::new(document["path"].as_str().unwrap())
            .components()
            .collect::<Vec<_>>(),
        expected.components().collect::<Vec<_>>()
    );
    assert_eq!(document["source"], fixture.documents[index]["source"]);
    assert_eq!(document["nodeId"], fixture.documents[index]["nodeId"]);
    assert_eq!(document["recoveredDraft"], fixture.contexts[index].source);
    assert!(document["recoveredDraftConflict"].as_bool() == Some(false));
    let session_id = document["sessionId"].as_str().unwrap().to_owned();
    let history = core_call(&fixture.state, move |core| core.list_history(&session_id))
        .await
        .unwrap();
    assert_eq!(
        history
            .versions
            .into_iter()
            .map(|v| v.version_id)
            .collect::<Vec<_>>(),
        fixture.histories[index]
    );
    // Result consumption supplies a new Core session while leaving the
    // original live source/serial (and the UI's shared editor/Undo) intact.
    check_snapshot(
        &fixture.state,
        &fixture.contexts[index].session_id,
        &fixture.contexts[index].source,
        fixture.contexts[index].serial,
        None,
    )
    .unwrap();
}

#[tokio::test]
async fn directory_result_rebinds_current_and_nested_dirty_sessions_and_status() {
    let f = fixture();
    let (receipt, operation_id) = relocate(&f, Some(f.contexts[0].clone())).await;
    assert_eq!(receipt["kind"], "relocateDirectory");
    assert_eq!(receipt["documentDisposition"], "rebound");
    assert_rebound(&f, &receipt["document"], 0, "First/First.typ").await;
    let affected = receipt["affectedDocuments"].as_array().unwrap();
    assert_eq!(affected.len(), 1);
    assert_eq!(affected[0]["priorSessionId"], f.contexts[1].session_id);
    assert_eq!(affected[0]["documentDisposition"], "rebound");
    assert_rebound(&f, &affected[0]["document"], 1, "Nested/Second/Second.typ").await;
    assert!(!f.source.exists());
    assert!(!f.target.join("Moved/Moved.typ").exists());
    assert_eq!(
        location(&f.state, &f.contexts[2].session_id).unwrap().path,
        PathBuf::from(f.documents[2]["path"].as_str().unwrap())
    );
    check_snapshot(
        &f.state,
        &f.contexts[2].session_id,
        &f.contexts[2].source,
        f.contexts[2].serial,
        None,
    )
    .unwrap();
    let result = status(
        State(f.state.clone()),
        Json(StatusRequest {
            operation_id,
            context: Some(f.contexts[0].clone()),
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(result["status"], "committed");
    assert_rebound(&f, &result["document"], 0, "First/First.typ").await;
}

#[tokio::test]
async fn directory_result_without_context_maps_all_children_without_inventing_a_source_leaf() {
    let f = fixture();
    let (receipt, _) = relocate(&f, None).await;
    assert!(receipt.get("document").is_none());
    assert!(receipt.get("documentError").is_none());
    let affected = receipt["affectedDocuments"].as_array().unwrap();
    assert_eq!(affected.len(), 2);
    for (index, relative) in ["First/First.typ", "Nested/Second/Second.typ"]
        .into_iter()
        .enumerate()
    {
        let effect = affected
            .iter()
            .find(|effect| effect["priorSessionId"] == f.contexts[index].session_id)
            .unwrap();
        assert_eq!(effect["documentDisposition"], "rebound");
        assert_rebound(&f, &effect["document"], index, relative).await;
    }
    assert!(!f.target.join("Moved/Moved.typ").exists());
}
