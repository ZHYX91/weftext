use super::*;

#[test]
fn ordinary_preview_captures_dependencies_and_current_input_without_writing_author_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("document");
    fs::create_dir(&root).unwrap();
    let source = root.join("note.typ");
    fs::write(&source, "Saved original").unwrap();
    fs::write(root.join("dependency.typ"), "Dependency original").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let opened = core.open(&source, Some(root.clone())).unwrap();
    let input = core
        .preview_compile_snapshot(&opened.session_id, "Current unsaved input")
        .unwrap();
    assert_eq!(input.source, "Current unsaved input");
    assert_eq!(input.files["note.typ"], b"Current unsaved input");
    assert_eq!(input.files["dependency.typ"], b"Dependency original");
    assert_eq!(fs::read(&source).unwrap(), b"Saved original");
    fs::write(root.join("dependency.typ"), "Late dependency").unwrap();
    assert_eq!(input.files["dependency.typ"], b"Dependency original");
    assert!(input.style.is_none());
    assert!(core.admitted_cuts.is_empty());
    assert!(core.compile_dependency_cuts.is_empty());
}

#[test]
fn managed_preview_releases_read_cuts_and_keeps_explicit_input() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let opened = core.create_note_fixture(&root, &root, "Preview").unwrap();
    let cuts = core.admitted_cuts.len();
    let dependencies = core.compile_dependency_cuts.len();
    let source = format!("{}\nCurrent input", opened.source);
    let input = core
        .preview_compile_snapshot(&opened.session_id, &source)
        .unwrap();
    assert_eq!(input.source, source);
    let relative = Path::new(&opened.path)
        .strip_prefix(&root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    assert_eq!(input.files[&relative], source.as_bytes());
    assert_eq!(core.admitted_cuts.len(), cuts);
    assert_eq!(core.compile_dependency_cuts.len(), dependencies);
    assert_eq!(fs::read_to_string(&opened.path).unwrap(), opened.source);
}

#[test]
fn preview_rejects_replaced_source_object_and_preserves_both_originals() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("document");
    fs::create_dir(&root).unwrap();
    let path = root.join("note.typ");
    fs::write(&path, "Original").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let opened = core.open(&path, Some(root.clone())).unwrap();
    fs::rename(&path, root.join("retained.typ")).unwrap();
    fs::write(&path, "Replacement").unwrap();
    let error = core
        .preview_compile_snapshot(&opened.session_id, "Unsaved input")
        .unwrap_err();
    assert_eq!(error.code, "scopeChanged");
    assert_eq!(fs::read(&path).unwrap(), b"Replacement");
    assert_eq!(fs::read(root.join("retained.typ")).unwrap(), b"Original");
}
