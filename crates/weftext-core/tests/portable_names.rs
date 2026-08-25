use tempfile::tempdir;
use weftext_core::{
    MAX_PORTABLE_NODE_NAME_BYTES, WorkspaceError, create_child_node, create_workspace,
    suggest_portable_node_name,
};

#[test]
fn suggestion_normalizes_and_preserves_portable_unicode() {
    assert_eq!(
        suggest_portable_node_name("  Cafe\u{301} / 中文 🧑‍💻 .  "),
        Some("Café-中文-🧑‍💻".to_owned())
    );
    assert_eq!(
        suggest_portable_node_name(".Release.2026."),
        Some("Release.2026".to_owned())
    );
}

#[test]
fn suggestion_collapses_separators_without_truncating_or_suffixing() {
    assert_eq!(
        suggest_portable_node_name(" -- First\t/\\:*?\"<>|\u{a0}Second -- "),
        Some("First-Second".to_owned())
    );
    assert_eq!(suggest_portable_node_name(&"a".repeat(121)), None);
    assert_eq!(suggest_portable_node_name("..."), None);
}

#[test]
fn suggestion_refuses_reserved_names() {
    for title in [
        "CON",
        "con.txt",
        "LPT9.report",
        ".weftext-trash",
        ".WEFTEXT-FORMAT",
        "weftext.annotations.json",
        ".__weftext-transaction-plan",
        "_weftext.items",
    ] {
        assert_eq!(suggest_portable_node_name(title), None, "{title}");
    }
}

#[test]
fn authoritative_validation_enforces_utf8_byte_limit() {
    assert_eq!(MAX_PORTABLE_NODE_NAME_BYTES, 120);
    let temporary = tempdir().unwrap();
    let root = create_workspace(temporary.path().join("Workspace")).unwrap();

    create_child_node(&root.path, &"a".repeat(120)).expect("120 ASCII bytes");
    create_child_node(&root.path, &"界".repeat(40)).expect("120 multibyte UTF-8 bytes");
    assert_invalid_name(create_child_node(&root.path, &"a".repeat(121)));
    assert_invalid_name(create_child_node(
        &root.path,
        &format!("{}a", "界".repeat(40)),
    ));
}

#[test]
fn authoritative_validation_rejects_windows_and_weftext_names() {
    let temporary = tempdir().unwrap();
    let root = create_workspace(temporary.path().join("Workspace")).unwrap();

    for name in [
        "CON",
        "prn.txt",
        "AUX.log",
        "nul",
        "COM1.data",
        "lpt9.bin",
        ".weftext-trash",
        ".WEFTEXT-FORMAT",
        ".weftext-rules",
        "weftext.ANNOTATIONS.json",
        ".__WEFTEXT-TRANSACTION-plan",
        ".__weftext-resource-copy",
        "_weftext.items",
        "_WEFTEXT.trash-item.json",
        ".git",
    ] {
        assert_invalid_name(create_child_node(&root.path, name));
    }

    create_child_node(&root.path, "COM0").expect("only COM1 through COM9 are reserved");
    create_child_node(&root.path, "LPT10").expect("only LPT1 through LPT9 are reserved");
}

#[test]
fn authoritative_validation_rejects_boundaries_controls_and_bidi() {
    let temporary = tempdir().unwrap();
    let root = create_workspace(temporary.path().join("Workspace")).unwrap();

    for name in [
        " leading",
        "trailing\u{a0}",
        "trailing.",
        "a/b",
        "a\u{7f}b",
        "a\u{202e}b",
        "a\u{206a}b",
    ] {
        assert_invalid_name(create_child_node(&root.path, name));
    }

    create_child_node(&root.path, "עברית").expect("ordinary RTL letters remain portable");
    create_child_node(&root.path, "中文").expect("CJK letters remain portable");
    create_child_node(&root.path, "🧑‍💻").expect("emoji sequences remain portable");
    create_child_node(&root.path, "Internal space").expect("internal whitespace remains valid");
}

fn assert_invalid_name(result: Result<weftext_core::CreatedNode, WorkspaceError>) {
    let error = result.expect_err("portable node name must be rejected");
    assert!(matches!(error, WorkspaceError::InvalidName(_)));
}
