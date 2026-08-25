use std::fs;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;

use weftext_core::{
    PHYSICAL_TREE_INVENTORY_SCHEMA, PhysicalInventoryError, WORKSPACE_TRANSACTION_LEASE_FILE_NAME,
    acquire_workspace_transaction_lease, capture_disjoint_external_physical_tree,
    capture_stable_physical_tree, capture_stable_workspace_physical_inventory,
    verify_disjoint_external_physical_tree,
};

fn locators(inventory: &weftext_core::PhysicalTreeInventory) -> Vec<&str> {
    inventory
        .entries()
        .iter()
        .map(|entry| entry.locator().as_str())
        .collect()
}

fn write_representative_tree(root: &Path) {
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".git/config"), b"physical git bytes\n").unwrap();
    fs::create_dir_all(root.join("ignored/empty")).unwrap();
    fs::write(root.join("ignored/secret.bin"), b"ignored\0bytes").unwrap();
    fs::create_dir_all(root.join("Node")).unwrap();
    fs::write(root.join("Node/weftext.annotations.json"), b"{}\n").unwrap();
    fs::create_dir_all(root.join("Template")).unwrap();
    fs::write(root.join("Template/weftext.template.json"), b"{}\n").unwrap();
    fs::create_dir_all(root.join(".weftext-trash/_weftext.items/item/payload/empty")).unwrap();
    fs::write(
        root.join(".weftext-trash/_weftext.items/item/_weftext.trash-item.json"),
        b"trash manifest bytes\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(
        root.join("nested/.__weftext-transaction-workspace-user-resource"),
        b"ordinary nested payload",
    )
    .unwrap();
    fs::write(root.join("zero.bin"), []).unwrap();
}

#[test]
fn workspace_capture_is_complete_sorted_path_free_and_excludes_only_the_lease() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Workspace");
    fs::create_dir(&root).unwrap();
    write_representative_tree(&root);
    let lease = acquire_workspace_transaction_lease(&root).unwrap();

    let inventory = capture_stable_workspace_physical_inventory(&lease).unwrap();
    let actual = locators(&inventory);
    let mut sorted = actual.clone();
    sorted.sort_unstable();
    assert_eq!(actual, sorted);
    for required in [
        ".git",
        ".git/config",
        ".weftext-trash/_weftext.items/item/_weftext.trash-item.json",
        ".weftext-trash/_weftext.items/item/payload/empty",
        "ignored/empty",
        "ignored/secret.bin",
        "Node/weftext.annotations.json",
        "Template/weftext.template.json",
        "nested/.__weftext-transaction-workspace-user-resource",
        "zero.bin",
    ] {
        assert!(actual.contains(&required), "missing {required}: {actual:?}");
    }
    assert!(!actual.contains(&WORKSPACE_TRANSACTION_LEASE_FILE_NAME));
    assert_eq!(inventory.binding().schema, PHYSICAL_TREE_INVENTORY_SCHEMA);
    inventory.binding().validate().unwrap();
    let binding_json = serde_json::to_vec(inventory.binding()).unwrap();
    let reopened: weftext_core::PhysicalInventoryBinding =
        serde_json::from_slice(&binding_json).unwrap();
    assert_eq!(&reopened, inventory.binding());
    let debug = format!("{inventory:?}");
    assert!(!debug.contains(root.to_str().unwrap()));
    assert!(!debug.contains("secret.bin"));

    drop(lease);
    let exact = capture_stable_physical_tree(&root).unwrap();
    assert!(
        locators(&exact).contains(&WORKSPACE_TRANSACTION_LEASE_FILE_NAME),
        "an exact non-workspace capture has no implicit exclusions"
    );
}

#[test]
fn digest_binds_creation_order_names_kinds_empty_directories_and_bytes() {
    let temporary = tempfile::tempdir().unwrap();
    let left = temporary.path().join("left");
    let right = temporary.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    fs::create_dir(left.join("empty")).unwrap();
    fs::write(left.join("b.bin"), b"BB").unwrap();
    fs::write(left.join("a.bin"), b"AA").unwrap();
    fs::write(right.join("a.bin"), b"AA").unwrap();
    fs::write(right.join("b.bin"), b"BB").unwrap();
    fs::create_dir(right.join("empty")).unwrap();

    let baseline = capture_stable_physical_tree(&left).unwrap();
    let same = capture_stable_physical_tree(&right).unwrap();
    assert_eq!(
        baseline.binding().sha256,
        "8d3897ed6835c5d6336a1f434dfc81c9bdcc26d6d77a0586551c3874e7bd88b6"
    );
    assert_eq!(baseline.binding(), same.binding());
    assert_eq!(baseline.entries(), same.entries());

    fs::write(right.join("a.bin"), b"AZ").unwrap();
    assert_ne!(
        baseline.binding(),
        capture_stable_physical_tree(&right).unwrap().binding()
    );
    fs::write(right.join("a.bin"), b"AA").unwrap();
    fs::rename(right.join("b.bin"), right.join("c.bin")).unwrap();
    assert_ne!(
        baseline.binding(),
        capture_stable_physical_tree(&right).unwrap().binding()
    );
    fs::rename(right.join("c.bin"), right.join("b.bin")).unwrap();
    fs::remove_file(right.join("b.bin")).unwrap();
    fs::create_dir(right.join("b.bin")).unwrap();
    assert_ne!(
        baseline.binding(),
        capture_stable_physical_tree(&right).unwrap().binding()
    );
}

#[test]
fn root_transaction_evidence_blocks_but_nested_transaction_like_payload_is_inventoried() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Workspace");
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("nested")).unwrap();
    fs::write(
        root.join("nested/.__weftext-transaction-workspace-user.bin"),
        b"payload",
    )
    .unwrap();
    let lease = acquire_workspace_transaction_lease(&root).unwrap();
    let inventory = capture_stable_workspace_physical_inventory(&lease).unwrap();
    assert!(locators(&inventory).contains(&"nested/.__weftext-transaction-workspace-user.bin"));

    fs::create_dir(root.join(".__weftext-transaction-workspace-interrupted")).unwrap();
    assert!(matches!(
        capture_stable_workspace_physical_inventory(&lease),
        Err(PhysicalInventoryError::UnfinishedTransaction(locator))
            if locator.as_str() == ".__weftext-transaction-workspace-interrupted"
    ));
}

#[test]
fn renamed_or_replaced_lease_anchor_never_matches_the_held_lease_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Workspace");
    fs::create_dir(&root).unwrap();
    let lease = acquire_workspace_transaction_lease(&root).unwrap();
    let anchor = root.join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME);
    let displaced = root.join("displaced-lease-anchor");

    fs::rename(&anchor, &displaced).unwrap();
    assert!(matches!(
        capture_stable_workspace_physical_inventory(&lease),
        Err(PhysicalInventoryError::LeaseAnchorMismatch)
    ));

    fs::write(&anchor, []).unwrap();
    assert!(matches!(
        capture_stable_workspace_physical_inventory(&lease),
        Err(PhysicalInventoryError::LeaseAnchorMismatch)
    ));
}

#[test]
fn disjoint_external_tree_is_revalidated_and_tampering_is_detected() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("Workspace");
    let external = temporary.path().join("External");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(&external).unwrap();
    fs::create_dir(workspace.join("empty")).unwrap();
    fs::create_dir(external.join("empty")).unwrap();
    fs::write(workspace.join("bytes.bin"), b"same bytes").unwrap();
    fs::write(external.join("bytes.bin"), b"same bytes").unwrap();
    let lease = acquire_workspace_transaction_lease(&workspace).unwrap();
    let source = capture_stable_workspace_physical_inventory(&lease).unwrap();

    let captured = capture_disjoint_external_physical_tree(&lease, &external).unwrap();
    assert_eq!(captured.binding(), source.binding());
    let verified =
        verify_disjoint_external_physical_tree(&lease, &external, source.binding()).unwrap();
    assert_eq!(verified.binding(), source.binding());
    assert!(!format!("{verified:?}").contains(external.to_str().unwrap()));
    verified.revalidate(&lease).unwrap();

    fs::write(external.join("bytes.bin"), b"tampered!").unwrap();
    assert!(matches!(
        verified.revalidate(&lease),
        Err(PhysicalInventoryError::BindingMismatch)
    ));

    let nested = workspace.join("nested-external");
    fs::create_dir(&nested).unwrap();
    assert!(matches!(
        verify_disjoint_external_physical_tree(&lease, &nested, source.binding()),
        Err(PhysicalInventoryError::ExternalTreeNotDisjoint)
    ));
    assert!(matches!(
        verify_disjoint_external_physical_tree(&lease, temporary.path(), source.binding()),
        Err(PhysicalInventoryError::ExternalTreeNotDisjoint)
    ));
}

#[test]
fn external_tree_hard_linked_to_workspace_bytes_is_not_independent_recovery_evidence() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("Workspace");
    let external = temporary.path().join("External");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(&external).unwrap();
    fs::create_dir(workspace.join("empty")).unwrap();
    fs::create_dir(external.join("empty")).unwrap();
    fs::write(workspace.join("bytes.bin"), b"shared object bytes").unwrap();
    fs::hard_link(workspace.join("bytes.bin"), external.join("bytes.bin")).unwrap();
    let lease = acquire_workspace_transaction_lease(&workspace).unwrap();
    let source = capture_stable_workspace_physical_inventory(&lease).unwrap();

    assert!(matches!(
        verify_disjoint_external_physical_tree(&lease, &external, source.binding()),
        Err(PhysicalInventoryError::ExternalTreeNotDisjoint)
    ));
}

#[test]
fn unsafe_locator_forms_are_rejected() {
    for invalid in [
        "",
        "/absolute",
        "trailing/",
        "a//b",
        ".",
        "..",
        "a/../b",
        "a\\b",
    ] {
        assert!(matches!(
            weftext_core::PhysicalLocator::parse(invalid),
            Err(PhysicalInventoryError::InvalidLocator)
        ));
    }
}

#[cfg(unix)]
#[test]
fn symlink_and_non_utf8_entries_fail_closed() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("root");
    let outside = temporary.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    symlink(&outside, root.join("linked")).unwrap();
    assert!(matches!(
        capture_stable_physical_tree(&root),
        Err(PhysicalInventoryError::LinkedOrReparse(_))
    ));
    fs::remove_file(root.join("linked")).unwrap();
    match fs::write(root.join(OsString::from_vec(vec![0xff])), b"bytes") {
        Ok(()) => assert!(matches!(
            capture_stable_physical_tree(&root),
            Err(PhysicalInventoryError::NonUtf8Path)
        )),
        Err(error) => {
            #[cfg(target_os = "macos")]
            assert_eq!(
                error.raw_os_error(),
                Some(92),
                "macOS must reject the invalid byte sequence before inventory: {error}"
            );
            #[cfg(not(target_os = "macos"))]
            panic!("create non-UTF-8 inventory fixture: {error}");
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_junction_fails_closed() {
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("root");
    let outside = temporary.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    let junction = root.join("junction");
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(matches!(
        capture_stable_physical_tree(&root),
        Err(PhysicalInventoryError::LinkedOrReparse(_))
    ));
}

#[cfg(windows)]
#[test]
fn windows_unc_alias_is_rejected_by_directory_identity_when_available() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("Workspace");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(workspace.join("empty")).unwrap();
    let lease = acquire_workspace_transaction_lease(&workspace).unwrap();
    let workspace_text = workspace.to_str().unwrap();
    let drive = workspace_text.as_bytes()[0] as char;
    let relative = &workspace_text[3..];
    let alias = PathBuf::from(format!(r"\\localhost\{drive}$\{relative}"));
    if fs::metadata(&alias).is_err() {
        return;
    }
    let workspace_canonical = fs::canonicalize(&workspace).unwrap();
    let alias_canonical = fs::canonicalize(&alias).unwrap();
    assert!(!workspace_canonical.starts_with(&alias_canonical));
    assert!(!alias_canonical.starts_with(&workspace_canonical));

    assert!(matches!(
        capture_disjoint_external_physical_tree(&lease, &alias),
        Err(PhysicalInventoryError::ExternalTreeNotDisjoint)
    ));
}
