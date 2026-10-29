//! A committed resource batch can propose one exact Draft edit. This adapter
//! never saves an author source, rolls back an import, or allocates another op.
use crate::{
    ApiError, ApiResult, AppState, Location, RequestJson, core_call, location,
    writing_api::{check_snapshot, validate_source},
};
use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use weftext_engine::{BlockInsert, EditIntent, EditOutput, EditRequest, SourcePatch};
use weftext_messages::UiMessage;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum InsertMode {
    Auto,
    Links,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ImportInsertRequest {
    session_id: String,
    operation_id: String,
    source: String,
    serial: u64,
    source_hash: String,
    mode: InsertMode,
}

fn same_path(a: &Path, b: &Path) -> bool {
    #[cfg(windows)]
    {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

fn check_location(state: &AppState, session_id: &str, original: &Location) -> Result<(), ApiError> {
    let current = location(state, session_id)?;
    if !same_path(&current.root, &original.root) || !same_path(&current.path, &original.path) {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "stale_draft",
            UiMessage::error_api_writing_editor_basis_expired(),
        ));
    }
    Ok(())
}

pub(super) async fn prepare_insert(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<ImportInsertRequest>,
) -> ApiResult {
    // Consume a previously observed exact editor basis, as /api/edit does. The
    // operation ID conveys neither a current editor basis nor destination rights.
    check_snapshot(
        &state,
        &request.session_id,
        &request.source,
        request.serial,
        Some(&request.source_hash),
    )?;
    let original = location(&state, &request.session_id)?;
    let captured = original.clone();
    let operation_id = request.operation_id.clone();
    let (imports, bytes) = core_call(&state, move |core| {
        core.resource_import_insertion_inputs(&captured.root, &captured.path, &operation_id)
    })
    .await?;
    check_location(&state, &request.session_id, &original)?;
    check_snapshot(
        &state,
        &request.session_id,
        &request.source,
        request.serial,
        Some(&request.source_hash),
    )?;
    let source = request.source.clone();
    let mode = request.mode;
    let block_inputs = imports
        .iter()
        .zip(bytes)
        .map(|(item, bytes)| (item.name.clone(), item.resource_ref.clone(), bytes))
        .collect::<Vec<_>>();
    let edited = crate::work::spawn_blocking(move || {
        let blocks = block_inputs
            .into_iter()
            .map(|(name, resource_ref, bytes)| resource_block(name, resource_ref, &bytes, mode))
            .collect();
        append_blocks(&source, blocks)
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_api_writing_edit_incomplete()))?
    .map_err(|error| {
        ApiError::from_engine(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_edit", error)
    })?;
    validate_source(&edited.source, None)?;
    // Byte inspection and Engine work can be delayed. Requalify the original
    // committed owner's exact manifest/resources before returning a proposal.
    let captured = original.clone();
    let operation_id = request.operation_id.clone();
    core_call(&state, move |core| {
        core.validate_resource_import_insertion(&captured.root, &captured.path, &operation_id)
    })
    .await?;
    check_location(&state, &request.session_id, &original)?;
    check_snapshot(
        &state,
        &request.session_id,
        &request.source,
        request.serial,
        Some(&request.source_hash),
    )?;
    let mut edit = crate::value(edited)?;
    edit["serial"] = json!(request.serial);
    Ok(Json(
        json!({"operationId":request.operation_id,"imports":imports,"edit":edit}),
    ))
}

fn resource_block(
    name: String,
    resource_ref: String,
    bytes: &[u8],
    mode: InsertMode,
) -> BlockInsert {
    if matches!(mode, InsertMode::Auto) && reliable_image(&name, bytes) {
        BlockInsert::Image {
            path: name.clone(),
            alt: name,
        }
    } else {
        BlockInsert::Link {
            target: resource_ref,
            text: name,
        }
    }
}

fn reliable_image(name: &str, bytes: &[u8]) -> bool {
    // Native Typst chooses a known format from the suffix before sniffing the
    // bytes. A decodable PNG named .jpg/.pdf is therefore not a proven image.
    let expected = match Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => image::ImageFormat::Png,
        Some("jpg" | "jpeg") => image::ImageFormat::Jpeg,
        _ => return false,
    };
    if image::guess_format(bytes).ok() != Some(expected) {
        return false;
    }
    matches!(
        super::metadata_from_bytes(bytes),
        super::ResourceMetadata::Image { .. }
    )
}

/// Each block is certified by Engine at its official body insertion. Only the
/// resulting contiguous insertion is published, so the client applies one Undo
/// unit even when endnotes follow the body or labels contain multibyte text.
fn append_blocks(
    source: &str,
    blocks: Vec<BlockInsert>,
) -> Result<EditOutput, weftext_engine::EngineError> {
    let unavailable = || {
        weftext_engine::EngineError::new(UiMessage::error_engine_editor_body_insertion_unavailable())
    };
    if blocks.is_empty() {
        return Err(unavailable());
    }
    let original_map = weftext_engine::inspect_editor(source);
    let original_at = original_map
        .body_insertion
        .as_ref()
        .ok_or_else(unavailable)?
        .from;
    let mut current = source.to_owned();
    let mut latest = None;
    for block in blocks {
        if matches!(&block, BlockInsert::Link { .. })
            && !weftext_engine::standard_binding_available(&current, "link")
        {
            return Err(unavailable());
        }
        let map = weftext_engine::inspect_editor(&current);
        let at = map.body_insertion.as_ref().ok_or_else(unavailable)?.from;
        let edited = weftext_engine::edit(
            &current,
            &EditRequest {
                source_hash: map.source_hash,
                intent: EditIntent::InsertBlock { at, block },
            },
        )?;
        // Never compress a replacement, a moved source suffix, or an arbitrary
        // editor projection into an insertion by assuming a UTF-8/UTF-16 offset.
        if edited.patches.len() != 1
            || edited.patches[0].from != at
            || edited.patches[0].to != at
            || !edited.patches[0].expected.is_empty()
            || !edited.source.starts_with(&source[..original_at])
            || !edited.source.ends_with(&source[original_at..])
        {
            return Err(unavailable());
        }
        current = edited.source.clone();
        latest = Some(edited);
    }
    let mut output = latest.ok_or_else(unavailable)?;
    let inserted_len = output
        .source
        .len()
        .checked_sub(source.len())
        .ok_or_else(unavailable)?;
    let utf16_at = source[..original_at].encode_utf16().count();
    output.patches = vec![SourcePatch {
        from: original_at,
        to: original_at,
        utf16_from: utf16_at,
        utf16_to: utf16_at,
        expected: String::new(),
        insert: output.source[original_at..original_at + inserted_len].into(),
    }];
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(name: &str, id: &str) -> BlockInsert {
        BlockInsert::Link {
            target: format!("weftext-resource://a/{id}"),
            text: name.into(),
        }
    }

    #[test]
    fn ordered_unicode_batch_is_one_original_source_patch() {
        let source = "正文😀";
        let output = append_blocks(
            source,
            vec![link("第一 []", "first"), link("第二 #", "second")],
        )
        .unwrap();
        assert_eq!(output.patches.len(), 1);
        let patch = &output.patches[0];
        assert_eq!((patch.from, patch.to), (source.len(), source.len()));
        assert_eq!((patch.utf16_from, patch.utf16_to), (4, 4));
        assert_eq!(patch.expected, "");
        assert_eq!(format!("{source}{}", patch.insert), output.source);
        assert!(output.source.find("/first").unwrap() < output.source.find("/second").unwrap());
        assert!(output.map.syntax_valid);
    }

    #[test]
    fn body_end_is_engine_insertion_before_native_endnotes() {
        let source = "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n正文#endnote(id: \"a\")[尾注]\n#endnotes()";
        let at = weftext_engine::inspect_editor(source)
            .body_insertion
            .unwrap()
            .from;
        assert!(at < source.len());
        let output = append_blocks(
            source,
            vec![link("first", "first"), link("second", "second")],
        )
        .unwrap();
        assert_eq!(output.patches[0].from, at);
        let mut actual = source.to_string();
        actual.insert_str(at, &output.patches[0].insert);
        assert_eq!(actual, output.source);
        assert!(output.source.ends_with("#endnotes()"));
    }

    #[test]
    fn unavailable_native_bindings_and_unverified_source_reject_whole_batch() {
        for source in [
            "#let link = x => x\n正文",
            "#show: x => x\n正文",
            "正文 // trailing",
        ] {
            assert!(
                append_blocks(source, vec![link("file", "one")]).is_err(),
                "{source}"
            );
        }
        assert!(append_blocks("正文", vec![]).is_err());
        assert!(
            append_blocks(
                "#let image = x => x\n正文",
                vec![BlockInsert::Image {
                    path: "a.png".into(),
                    alt: "a".into()
                }]
            )
            .is_err()
        );
    }

    #[test]
    fn auto_image_profile_requires_decodable_untransformed_png_or_jpeg() {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert!(reliable_image("matching.png", bytes.get_ref()));
        assert!(reliable_image("matching.PNG", bytes.get_ref()));
        let mut jpeg = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut jpeg, image::ImageFormat::Jpeg)
            .unwrap();
        assert!(reliable_image("matching.jpg", jpeg.get_ref()));
        assert!(reliable_image("matching.jpeg", jpeg.get_ref()));
        for (name, original) in [
            ("mismatch.jpg", bytes.get_ref()),
            ("mismatch.pdf", bytes.get_ref()),
            ("mismatch.png", jpeg.get_ref()),
        ] {
            assert!(!reliable_image(name, original));
            assert!(
                matches!(resource_block(name.into(), "weftext-resource://owner/original".into(), original, InsertMode::Auto), BlockInsert::Link { target, text } if target == "weftext-resource://owner/original" && text == name)
            );
        }
        let mut gif = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut gif, image::ImageFormat::Gif)
            .unwrap();
        assert!(!reliable_image("animation.gif", gif.get_ref()));
        assert!(!reliable_image("broken.png", b"\x89PNG\r\n\x1a\ncorrupt"));
        assert!(!reliable_image("original.txt", b"ordinary original"));
    }

    #[test]
    fn insert_request_cannot_add_path_target_or_an_implicit_mode() {
        let request = json!({"sessionId":"s","operationId":"op","source":"x","serial":1,"sourceHash":"h","mode":"links"});
        assert!(serde_json::from_value::<ImportInsertRequest>(request.clone()).is_ok());
        let mut path = request.clone();
        path["path"] = json!("C:/not-an-authority");
        assert!(serde_json::from_value::<ImportInsertRequest>(path).is_err());
        let mut mode = request;
        mode["mode"] = json!("saveAuthor");
        assert!(serde_json::from_value::<ImportInsertRequest>(mode).is_err());
    }

    #[cfg(windows)]
    fn fixture(
        commit: bool,
    ) -> (
        tempfile::TempDir,
        AppState,
        weftext_core::OpenedDocument,
        String,
    ) {
        use crate::CoreTestFixture;
        use std::{
            collections::HashMap,
            sync::{Arc, Mutex},
        };
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = weftext_core::Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let operation_id = uuid::Uuid::new_v4().to_string();
        let preview = core
            .prepare_import_resources(
                &root,
                &note.path,
                vec![
                    weftext_core::ResourceImportUpload {
                        name: "图 像😀.png".into(),
                        bytes: png.into_inner(),
                    },
                    weftext_core::ResourceImportUpload {
                        name: "原件.txt".into(),
                        bytes: b"original preserved".to_vec(),
                    },
                ],
                &operation_id,
            )
            .unwrap();
        if commit {
            core.execute_operation(&operation_id, &preview.plan_id)
                .unwrap();
        }
        let state = AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from("fixture-token"),
            port: 0,
            locations: Arc::new(Mutex::new(HashMap::from([(
                note.session_id.clone(),
                Location {
                    root,
                    path: note.path.clone().into(),
                },
            )]))),
            renders: Default::default(),
            editor_versions: Default::default(),
            exports: Arc::new(Mutex::new(crate::export_api::ExportStore::default())),
            native_status: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(crate::remote_sync::Store::default())),
            initial: None,
        };
        crate::writing_api::observe_snapshot(&state, &note.session_id, &note.source, 1, None)
            .unwrap();
        (temp, state, note, operation_id)
    }

    #[cfg(windows)]
    fn request(
        note: &weftext_core::OpenedDocument,
        operation_id: &str,
        mode: InsertMode,
    ) -> ImportInsertRequest {
        ImportInsertRequest {
            session_id: note.session_id.clone(),
            operation_id: operation_id.into(),
            source: note.source.clone(),
            serial: 1,
            source_hash: weftext_engine::source_hash(&note.source),
            mode,
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn committed_batch_proposes_one_draft_patch_without_saving_or_importing_again() {
        let (_temp, state, note, operation_id) = fixture(true);
        let original_results = state
            .core
            .lock()
            .unwrap()
            .resource_import_results(&operation_id)
            .unwrap();
        let Json(result) = prepare_insert(
            State(state.clone()),
            RequestJson(request(&note, &operation_id, InsertMode::Auto)),
        )
        .await
        .unwrap();
        assert_eq!(result["edit"]["patches"].as_array().unwrap().len(), 1);
        assert_eq!(result["edit"]["serial"], 1);
        let updated = result["edit"]["source"].as_str().unwrap();
        assert!(updated.contains("#image(\"图 像😀.png\""), "{updated}");
        assert!(updated.contains(&original_results[1].resource_ref));
        assert!(!updated.contains("#image(\"weftext-resource://"));
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
        let current = state
            .core
            .lock()
            .unwrap()
            .resource_import_results(&operation_id)
            .unwrap();
        assert_eq!(
            current
                .iter()
                .map(|item| &item.resource_ref)
                .collect::<Vec<_>>(),
            original_results
                .iter()
                .map(|item| &item.resource_ref)
                .collect::<Vec<_>>()
        );
        assert!(
            state
                .core
                .lock()
                .unwrap()
                .committed_operation_receipt(&operation_id)
                .unwrap()
                .is_some()
        );

        let Json(links) = prepare_insert(
            State(state.clone()),
            RequestJson(request(&note, &operation_id, InsertMode::Links)),
        )
        .await
        .unwrap();
        assert!(
            !links["edit"]["source"]
                .as_str()
                .unwrap()
                .contains("#image(")
        );
        assert_eq!(links["imports"].as_array().unwrap().len(), 2);
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn uncommitted_import_is_not_delivered_as_a_draft_and_no_leaf_is_created() {
        let (_temp, state, note, operation_id) = fixture(false);
        let error = prepare_insert(
            State(state),
            RequestJson(request(&note, &operation_id, InsertMode::Auto)),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "operationPending");
        let directory = Path::new(&note.path).parent().unwrap();
        assert!(!directory.join("图 像😀.png").exists());
        assert!(!directory.join("weftext.resources.json").exists());
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn later_input_and_aba_reject_delivery_but_keep_original_commit() {
        use std::future::Future;
        let (_temp, state, note, operation_id) = fixture(true);
        crate::writing_api::observe_snapshot(&state, &note.session_id, "later", 2, None).unwrap();
        let error = prepare_insert(
            State(state.clone()),
            RequestJson(request(&note, &operation_id, InsertMode::Links)),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "stale_draft");
        crate::writing_api::observe_snapshot(&state, &note.session_id, &note.source, 3, None)
            .unwrap();
        let error = prepare_insert(
            State(state.clone()),
            RequestJson(request(&note, &operation_id, InsertMode::Links)),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "stale_draft");
        // Poll through the initial exact check while the real Core read waits
        // for its lock. This makes the subsequent ABA a late result scenario,
        // rather than only rejecting an already stale request on entry.
        crate::writing_api::observe_snapshot(&state, &note.session_id, &note.source, 4, None)
            .unwrap();
        let mut input = request(&note, &operation_id, InsertMode::Links);
        input.serial = 4;
        let pending = {
            let _hold = state.core.lock().unwrap();
            let mut pending = Box::pin(prepare_insert(State(state.clone()), RequestJson(input)));
            let mut context = std::task::Context::from_waker(std::task::Waker::noop());
            assert!(pending.as_mut().poll(&mut context).is_pending());
            crate::writing_api::observe_snapshot(&state, &note.session_id, "later again", 5, None)
                .unwrap();
            crate::writing_api::observe_snapshot(&state, &note.session_id, &note.source, 6, None)
                .unwrap();
            pending
        };
        assert_eq!(pending.await.unwrap_err().code, "stale_draft");
        assert!(
            state
                .core
                .lock()
                .unwrap()
                .committed_operation_receipt(&operation_id)
                .unwrap()
                .is_some()
        );
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn changed_import_bytes_and_wrong_destination_cannot_use_old_plan_locators() {
        let (_temp, state, note, operation_id) = fixture(true);
        let other = {
            use crate::CoreTestFixture;
            let root = Path::new(&note.root);
            state
                .core
                .lock()
                .unwrap()
                .create_note_fixture(root, root, "Other")
                .unwrap()
        };
        state.locations.lock().unwrap().insert(
            note.session_id.clone(),
            Location {
                root: other.root.into(),
                path: other.path.into(),
            },
        );
        assert!(
            prepare_insert(
                State(state.clone()),
                RequestJson(request(&note, &operation_id, InsertMode::Links))
            )
            .await
            .is_err()
        );
        state.locations.lock().unwrap().insert(
            note.session_id.clone(),
            Location {
                root: note.root.clone().into(),
                path: note.path.clone().into(),
            },
        );
        std::fs::write(
            Path::new(&note.path).parent().unwrap().join("原件.txt"),
            b"external replacement",
        )
        .unwrap();
        assert!(
            prepare_insert(
                State(state.clone()),
                RequestJson(request(&note, &operation_id, InsertMode::Links))
            )
            .await
            .is_err()
        );
        assert!(
            state
                .core
                .lock()
                .unwrap()
                .committed_operation_receipt(&operation_id)
                .unwrap()
                .is_some()
        );
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn unproved_import_bindings_reject_both_modes_and_retain_resource_commit() {
        let (_temp, state, note, operation_id) = fixture(true);
        for (index, import) in [
            "#import \"module.typ\": *",
            "#import \"module.typ\" as link",
            "#import \"module.typ\": helper as link",
        ]
        .into_iter()
        .enumerate()
        {
            let source = format!("{import}\n{}", note.source);
            let serial = index as u64 + 2;
            crate::writing_api::observe_snapshot(&state, &note.session_id, &source, serial, None)
                .unwrap();
            for mode in [InsertMode::Auto, InsertMode::Links] {
                let mut input = request(&note, &operation_id, mode);
                input.source = source.clone();
                input.serial = serial;
                input.source_hash = weftext_engine::source_hash(&source);
                let error = prepare_insert(State(state.clone()), RequestJson(input))
                    .await
                    .unwrap_err();
                assert_eq!(error.code, "unsupported_edit");
            }
        }
        assert!(
            state
                .core
                .lock()
                .unwrap()
                .committed_operation_receipt(&operation_id)
                .unwrap()
                .is_some()
        );
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn safe_named_imports_and_native_endnotes_keep_one_resource_draft_patch() {
        let (_temp, state, note, operation_id) = fixture(true);
        for (index, source) in [
            format!("#import \"module.typ\": helper\n{}", note.source),
            format!("#import \"module.typ\" as helper\n{}", note.source),
            format!(
                "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n{}\n#endnotes()",
                note.source
            ),
            format!("#set image(format: \"jpg\")\n{}", note.source),
        ]
        .into_iter()
        .enumerate()
        {
            let serial = index as u64 + 2;
            crate::writing_api::observe_snapshot(&state, &note.session_id, &source, serial, None)
                .unwrap();
            let at = weftext_engine::inspect_editor(&source)
                .body_insertion
                .unwrap()
                .from;
            for mode in [InsertMode::Auto, InsertMode::Links] {
                let mut input = request(&note, &operation_id, mode);
                input.source = source.clone();
                input.serial = serial;
                input.source_hash = weftext_engine::source_hash(&source);
                let Json(result) = prepare_insert(State(state.clone()), RequestJson(input))
                    .await
                    .unwrap();
                let patches = result["edit"]["patches"].as_array().unwrap();
                assert_eq!(patches.len(), 1);
                assert_eq!(patches[0]["from"], at);
                assert_eq!(patches[0]["to"], at);
                let updated = result["edit"]["source"].as_str().unwrap();
                assert_eq!(
                    updated.contains("#image("),
                    matches!(mode, InsertMode::Auto)
                );
                if matches!(mode, InsertMode::Auto) {
                    assert!(updated.contains(", format: auto, alt:"));
                }
                if index == 2 {
                    assert!(at < source.len());
                    assert!(updated.ends_with("#endnotes()"));
                }
            }
        }
        assert!(
            state
                .core
                .lock()
                .unwrap()
                .committed_operation_receipt(&operation_id)
                .unwrap()
                .is_some()
        );
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
    }

    #[cfg(windows)]
    fn batch_request(
        note: &weftext_core::OpenedDocument,
        operation_id: &str,
        files: &[(&str, &[u8])],
    ) -> super::super::BatchImportRequest {
        use base64::Engine;
        super::super::BatchImportRequest {
            session_id: note.session_id.clone(),
            operation_id: operation_id.into(),
            files: files
                .iter()
                .map(|(name, bytes)| super::super::BatchImportFile {
                    name: (*name).into(),
                    bytes: base64::engine::general_purpose::STANDARD.encode(bytes),
                })
                .collect(),
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn batch_prepare_and_status_preserve_order_ids_and_original_request_without_installation()
    {
        let (_temp, state, note, _) = fixture(true);
        let directory = Path::new(&note.path).parent().unwrap();
        let manifest = std::fs::read(directory.join("weftext.resources.json")).unwrap();
        let operation_id = uuid::Uuid::new_v4().to_string();
        let files = [("z.txt", b"z".as_slice()), ("a.txt", b"a".as_slice())];
        let Json(first) = super::super::prepare_import_batch(
            State(state.clone()),
            RequestJson(batch_request(&note, &operation_id, &files)),
        )
        .await
        .unwrap();
        assert_eq!(first["operation"]["kind"], "importResources");
        assert_eq!(first["imports"][0]["name"], "z.txt");
        assert_eq!(first["imports"][1]["name"], "a.txt");
        let Json(repeated) = super::super::prepare_import_batch(
            State(state.clone()),
            RequestJson(batch_request(&note, &operation_id, &files)),
        )
        .await
        .unwrap();
        assert_eq!(first, repeated);
        let Json(status) = super::super::import_batch_status(
            State(state.clone()),
            RequestJson(super::super::BatchImportStatusRequest {
                session_id: note.session_id.clone(),
                operation_id: operation_id.clone(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(first, status);
        let error = super::super::prepare_import_batch(
            State(state.clone()),
            RequestJson(batch_request(
                &note,
                &operation_id,
                &[("z.txt", b"changed"), ("a.txt", b"a")],
            )),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "operationConflict");
        assert!(!directory.join("z.txt").exists());
        assert!(!directory.join("a.txt").exists());
        assert_eq!(
            std::fs::read(directory.join("weftext.resources.json")).unwrap(),
            manifest
        );
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn batch_api_rejects_last_item_conflict_invalid_base64_empty_and_duplicate_without_partial_files()
     {
        let (_temp, state, note, _) = fixture(true);
        let directory = Path::new(&note.path).parent().unwrap();
        let manifest = std::fs::read(directory.join("weftext.resources.json")).unwrap();
        let candidates = [
            vec![
                ("first.txt", b"first".as_slice()),
                ("原件.txt", b"conflict".as_slice()),
            ],
            vec![],
            vec![
                ("same.txt", b"first".as_slice()),
                ("same.txt", b"second".as_slice()),
            ],
        ];
        for files in candidates {
            let operation_id = uuid::Uuid::new_v4().to_string();
            assert!(
                super::super::prepare_import_batch(
                    State(state.clone()),
                    RequestJson(batch_request(&note, &operation_id, &files))
                )
                .await
                .is_err()
            );
            assert!(!directory.join("first.txt").exists());
            assert!(!directory.join("same.txt").exists());
            assert_eq!(
                std::fs::read(directory.join("weftext.resources.json")).unwrap(),
                manifest
            );
        }
        let operation_id = uuid::Uuid::new_v4().to_string();
        let mut invalid = batch_request(
            &note,
            &operation_id,
            &[("first.txt", b"first"), ("last.txt", b"last")],
        );
        invalid.files[1].bytes = "not base64!".into();
        let error = super::super::prepare_import_batch(State(state), RequestJson(invalid))
            .await
            .unwrap_err();
        assert_eq!(error.code, "invalid_request");
        assert!(!directory.join("first.txt").exists());
        assert!(!directory.join("last.txt").exists());
        assert_eq!(
            std::fs::read(directory.join("weftext.resources.json")).unwrap(),
            manifest
        );
        assert_eq!(std::fs::read_to_string(&note.path).unwrap(), note.source);
    }
}
