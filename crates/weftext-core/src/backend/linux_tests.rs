//! Runs as the enrolled service identity, never as root. Every fixture is an
//! explicitly owned fresh child; rejected objects are retained outside the root.
use super::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

#[test]
fn ordinary_linux_paths_do_not_create_writer_authority() {
    assert!(BackendContext::local().is_err());

    let legacy = br#"{"volume":1,"index":2,"created":3}"#;
    assert!(serde_json::from_slice::<FileIdentity>(legacy).is_err());
    assert!(
        serde_json::from_slice::<FileIdentity>(
            br#"{"platform":"linux","filesystem":[1,2],"handle_type":1,"handle":[4]}"#
        )
        .is_ok()
    );
}

#[test]
#[ignore = "requires a fresh administrator enrollment revocation harness"]
fn administrator_revocation_fences_existing_private_reads_and_operation_status() {
    let enrollment = PathBuf::from(
        std::env::var_os("WEFTEXT_LINUX_TEST_FENCE_ENROLLMENT")
            .expect("fresh revocation enrollment"),
    );
    let signals = PathBuf::from(
        std::env::var_os("WEFTEXT_LINUX_TEST_FENCE_SIGNALS").expect("owned test signal directory"),
    );
    let backend = BackendContext::controlled_service(&enrollment).unwrap();
    let fixture = backend
        .root
        .path
        .join(format!("revocation-case-{}", crate::fresh()));
    backend.create_private_directory(&fixture).unwrap();
    let data = fixture.join("private");
    let root = fixture.join("library");
    let mut core = crate::Core::with_backend(&data, backend).unwrap();
    core.store_hosted_policy(b"original protected policy")
        .unwrap();
    core.create_library(&root).unwrap();
    let doc = core
        .create_note_fixture(&root, &root, "BeforeRevocation")
        .unwrap();
    let operation = crate::fresh();
    core.save(crate::SaveRequest {
        session_id: doc.session_id,
        base_id: doc.base_id,
        source: format!("{}\noriginal durable save", doc.source),
        operation_id: operation.clone(),
    })
    .unwrap();
    let style_basis = core.read_library_style(&root).unwrap().base_id;
    let snapshot_basis = core
        .read_snapshot(&root, std::slice::from_ref(&doc.path))
        .unwrap()
        .files[0]
        .base_id
        .clone();
    assert!(core.snapshot_witness(&snapshot_basis).is_ok());
    let guard_source = fixture.join("guard-source");
    let guard_target = fixture.join("guard-target");
    core.backend
        .create_private_directory(&guard_source)
        .unwrap();
    core.backend
        .create_private_directory(&guard_target)
        .unwrap();
    let guarded_path = guard_source.join("kept.bin");
    crate::journal::private_create(&core.backend, &guarded_path, b"original guard bytes").unwrap();
    let parent_guard = ParentGuard::acquire(&core.backend, &guarded_path).unwrap();
    let mut retained_file = parent_guard.open_read_guard_file().unwrap();
    let mut namespace =
        NamespaceGuard::acquire(&core.backend, &guard_source, &guard_target).unwrap();
    let mut union =
        MultiNamespaceGuard::acquire(&core.backend, &[(&guard_source, &guard_target)]).unwrap();
    let unretained = fixture.join("unretained");
    assert!(parent_guard.directory_identity().is_ok());
    assert!(parent_guard.directory_coordinates().is_ok());
    assert_eq!(parent_guard.directory_entries(10).unwrap().0.len(), 1);
    assert!(
        namespace
            .source_entry_exists(Path::new("kept.bin"))
            .unwrap()
    );
    assert_eq!(namespace.source_coordinates().unwrap(), guard_source);
    assert_eq!(
        union
            .namespace_coordinate(&core.backend, &guarded_path)
            .unwrap(),
        guarded_path
    );
    assert!(
        !namespace
            .source_entry_exists(Path::new("absent.bin"))
            .unwrap()
    );
    assert!(
        namespace
            .source_directory_snapshot(Path::new(""), 10)
            .unwrap()
            .2
    );
    assert!(union.directory_identity(&unretained).unwrap().is_none());
    assert!(union.directory_snapshot(&guard_source, 10).unwrap().2);
    assert!(union.retain_directory(&guard_source).is_ok());
    assert!(union.select(0).is_ok());
    let policy = data.join("hosted-policy.json");
    let commit = core.journal_path(&operation, "commit");
    let before = [
        std::fs::read(&policy).unwrap(),
        std::fs::read(&doc.path).unwrap(),
        std::fs::read(&commit).unwrap(),
    ];
    std::fs::write(
        signals.join("ready.json"),
        serde_json::to_vec(&serde_json::json!({"fixture":fixture,"operation":operation})).unwrap(),
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !signals.join("revoked").exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "administrator revocation did not arrive"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    for error in [
        parent_guard.directory_identity().err().unwrap(),
        parent_guard.directory_coordinates().err().unwrap(),
        parent_guard.directory_entries(10).err().unwrap(),
        stamp(&retained_file).err().unwrap(),
        attributes(&retained_file).err().unwrap(),
        file_coordinates(&retained_file).err().unwrap(),
        namespace.source_identity().err().unwrap(),
        namespace.source_coordinates().err().unwrap(),
        union
            .namespace_coordinate(&core.backend, &guarded_path)
            .err()
            .unwrap(),
        namespace.parent_identity().err().unwrap(),
        namespace
            .source_entry_exists(Path::new("kept.bin"))
            .err()
            .unwrap(),
        namespace
            .source_entry_exists(Path::new("absent.bin"))
            .err()
            .unwrap(),
        namespace
            .source_parent_identity(Path::new("kept.bin"))
            .err()
            .unwrap(),
        namespace
            .source_directory_snapshot(Path::new(""), 10)
            .err()
            .unwrap(),
        union.directory_identity(&unretained).err().unwrap(),
        union.directory_identity(&guard_source).err().unwrap(),
        union.directory_file(&guard_source).err().unwrap(),
        union.directory_snapshot(&guard_source, 10).err().unwrap(),
        union.source_parent_identity(0).err().unwrap(),
        union.target_parent_identity(0).err().unwrap(),
        union.retain_directory(&guard_source).err().unwrap(),
        union.select(0).err().unwrap(),
        crate::operations::read_blob(&mut retained_file)
            .err()
            .unwrap(),
    ] {
        assert_eq!(error.code, "controlledRootAuthority");
    }
    assert_eq!(
        core.hosted_policy().unwrap_err().code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.operation_status(&operation).unwrap_err().code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.operation_status(&crate::fresh()).unwrap_err().code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.snapshot_witness(&snapshot_basis).err().unwrap().code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.library_style_basis_retained_bytes(&style_basis)
            .unwrap_err()
            .code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.local_operation_root(&operation).unwrap_err().code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.store_hosted_policy(b"must never replace the original")
            .unwrap_err()
            .code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.open(&doc.path, Some(root)).unwrap_err().code,
        "controlledRootAuthority"
    );
    assert_eq!(std::fs::read(policy).unwrap(), before[0]);
    assert_eq!(std::fs::read(&doc.path).unwrap(), before[1]);
    assert_eq!(std::fs::read(commit).unwrap(), before[2]);
    assert_eq!(
        std::fs::read(guarded_path).unwrap(),
        b"original guard bytes"
    );
}

#[test]
#[ignore = "requires a fresh administrator enrollment revocation harness"]
fn administrator_revocation_fences_empty_recovery_catalog() {
    let enrollment = PathBuf::from(
        std::env::var_os("WEFTEXT_LINUX_TEST_FENCE_ENROLLMENT")
            .expect("fresh revocation enrollment"),
    );
    let signals = PathBuf::from(
        std::env::var_os("WEFTEXT_LINUX_TEST_FENCE_SIGNALS").expect("owned test signal directory"),
    );
    let backend = BackendContext::controlled_service(&enrollment).unwrap();
    let fixture = backend
        .root
        .path
        .join(format!("revocation-empty-{}", crate::fresh()));
    backend.create_private_directory(&fixture).unwrap();
    let core = crate::Core::with_backend(fixture.join("private"), backend).unwrap();
    assert!(core.operations.is_empty());
    assert!(core.recoveries().unwrap().is_empty());
    assert!(
        core.saved_operation_source(&crate::fresh())
            .unwrap()
            .is_none()
    );
    std::fs::write(
        signals.join("ready.json"),
        serde_json::to_vec(&serde_json::json!({"fixture":fixture,"empty":true})).unwrap(),
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !signals.join("revoked").exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "administrator revocation did not arrive"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(core.operations.is_empty());
    assert_eq!(
        core.recoveries().unwrap_err().code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.saved_operation_source(&crate::fresh())
            .err()
            .unwrap()
            .code,
        "controlledRootAuthority"
    );
    assert_eq!(
        core.local_operation_root(&crate::fresh()).unwrap_err().code,
        "controlledRootAuthority"
    );
}

#[test]
#[ignore = "requires three explicit administrator peer enrollments; run serially"]
fn distinct_cores_reject_other_authority_and_unknown_birth_never_becomes_ordinary() {
    use crate::test_support::{FixtureRoots, open_core};
    let roots = FixtureRoots::new();
    let a_data = roots.a_path().join("a-private");
    let mut a = open_core(&a_data).unwrap();
    let mut b = open_core(roots.b_path().join("b-private")).unwrap();
    let library = roots.a_path().join("library");
    a.create_library(&library).unwrap();
    let doc = a
        .create_note_fixture(&library, &library, "Original")
        .unwrap();
    let original_source = std::fs::read(&doc.path).unwrap();
    assert_eq!(
        b.open(&doc.path, Some(library.clone())).unwrap_err().code,
        "outsideRoot"
    );
    assert_eq!(
        b.create_library(roots.a_path().join("foreign-library"))
            .unwrap_err()
            .code,
        "outsideRoot"
    );
    assert!(!roots.a_path().join("foreign-library").exists());
    assert_eq!(std::fs::read(&doc.path).unwrap(), original_source);

    let plan = a
        .operations
        .values()
        .find(|plan| {
            plan.node_id.as_ref() == doc.node_id.as_ref()
                || plan.transaction.as_ref().is_some_and(|t| {
                    t.births()
                        .iter()
                        .any(|birth| Some(&birth.node_id) == doc.node_id.as_ref())
                })
        })
        .expect("the original committed Node Plan");
    let operation = plan.operation_id.clone();
    let single = a.journal_path(&operation, "birth");
    let plural = a.journal_path(&operation, "births");
    let birth = if single.exists() { single } else { plural };
    let original_birth = std::fs::read(&birth).unwrap();
    let retained = birth.with_extension(format!("{}-retained", crate::fresh()));
    std::fs::rename(&birth, &retained).unwrap();
    assert_eq!(
        a.open(&doc.path, Some(library.clone())).unwrap_err().code,
        "identityPending"
    );
    std::fs::write(&birth, b"{ damaged original birth").unwrap();
    assert!(a.open(&doc.path, Some(library.clone())).is_err());
    assert_eq!(std::fs::read(&doc.path).unwrap(), original_source);
    // Preserve the damaged evidence and restore only the known exact original.
    std::fs::rename(
        &birth,
        birth.with_extension(format!("{}-damaged", crate::fresh())),
    )
    .unwrap();
    std::fs::write(&birth, &original_birth).unwrap();
    drop(a);
    a = open_core(&a_data).unwrap();
    assert_eq!(
        a.open(&doc.path, Some(library)).unwrap().node_id,
        doc.node_id
    );
    let plan = a.journal_path(&operation, "plan");
    let retained_plan = plan.with_extension(format!("{}-retained", crate::fresh()));
    drop(a);
    std::fs::rename(&plan, &retained_plan).unwrap();
    let failure = open_core(&a_data)
        .err()
        .expect("orphan original commit must refuse admission");
    assert_eq!(failure.code, "resolutionUnavailable");
    assert_eq!(std::fs::read(&doc.path).unwrap(), original_source);
    std::fs::rename(&retained_plan, &plan).unwrap();
}

#[test]
#[ignore = "requires administrator enrollment, dedicated service UID and retained fixture directory"]
fn controlled_service_core_round_trip_and_negative_boundaries() {
    let enrollment = PathBuf::from(
        std::env::var_os("WEFTEXT_LINUX_TEST_ENROLLMENT").expect("explicit enrollment"),
    );
    let quarantine = PathBuf::from(
        std::env::var_os("WEFTEXT_LINUX_TEST_QUARANTINE")
            .expect("owned retained fixture directory"),
    );
    let alternate = PathBuf::from(
        std::env::var_os("WEFTEXT_LINUX_TEST_ALTERNATE_ENROLLMENT")
            .expect("administrator alternate enrollment"),
    );
    let bad_machine = PathBuf::from(
        std::env::var_os("WEFTEXT_LINUX_TEST_BAD_MACHINE_ENROLLMENT")
            .expect("administrator foreign machine enrollment"),
    );
    assert_ne!(
        unsafe { libc::geteuid() },
        0,
        "root cannot establish service acceptance"
    );
    assert_eq!(unsafe { libc::getgroups(0, std::ptr::null_mut()) }, 0);
    assert!(BackendContext::controlled_service(&bad_machine).is_err());
    let context = BackendContext::controlled_service(&enrollment).unwrap();
    assert!(
        BackendContext::controlled_service(&enrollment).is_err(),
        "one physical root has one writer across data directories"
    );
    let fixture = context.root.path.join(format!("core-{}", crate::fresh()));
    context.create_private_directory(&fixture).unwrap();
    assert!(quarantine.is_dir() && !quarantine.starts_with(&context.root.path));
    {
        let path = fixture.join("original.typ");
        let guard = ParentGuard::acquire(&context, &path).unwrap();
        let mut file = guard.open_file(true, true).unwrap();
        file.write_all("原始内容\n".as_bytes()).unwrap();
        file.sync_all().unwrap();
        let identity = stamp(&file).unwrap().identity;
        assert!(matches!(identity, FileIdentity::Linux { .. }));
        drop(file);
        let reader = guard.open_read_guard_file().unwrap();
        assert!(
            guard.open_file(true, false).is_err(),
            "retained read scope must exclude a byte writer"
        );
        drop(reader);
        assert!(guard.open_file(true, false).is_ok());
        let (entries, complete) = guard.directory_entries(0).unwrap();
        assert!(entries.is_empty() && !complete);
        let (entries, complete) = guard.directory_entries(1).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(complete && entries[0].1.is_file());
        let source = fixture.join("Source");
        let target = fixture.join("Target");
        context.create_private_directory(&source).unwrap();
        context.create_private_directory(&target).unwrap();
        let leaf_guard = ParentGuard::acquire(&context, &source.join("Source.typ")).unwrap();
        let mut leaf = leaf_guard.open_file(true, true).unwrap();
        leaf.write_all(b"preserve exact bytes").unwrap();
        leaf.sync_all().unwrap();
        let leaf_identity = stamp(&leaf).unwrap().identity;
        drop(leaf);
        let occupied_guard = ParentGuard::acquire(&context, &source.join("Occupied.typ")).unwrap();
        let mut occupied = occupied_guard.open_file(true, true).unwrap();
        occupied.write_all(b"foreign destination").unwrap();
        occupied.sync_all().unwrap();
        drop(occupied);
        let namespace = NamespaceGuard::acquire(&context, &source, &target).unwrap();
        let original_directory = namespace.source_identity().unwrap();
        let leaf = namespace.open_source_file("Source.typ", true).unwrap();
        assert!(namespace.rename_leaf(&leaf, "Occupied.typ").is_err());
        assert_eq!(
            std::fs::read(source.join("Occupied.typ")).unwrap(),
            b"foreign destination"
        );
        namespace.rename_leaf(&leaf, "Renamed.typ").unwrap();
        assert_eq!(stamp(&leaf).unwrap().identity, leaf_identity);
        drop(leaf);
        namespace.move_source("Moved").unwrap();
        assert_eq!(namespace.source_identity().unwrap(), original_directory);
        assert_eq!(
            stamp(&namespace.open_moved_file("Renamed.typ").unwrap())
                .unwrap()
                .identity,
            leaf_identity
        );
        // The same retained entry binding supports parking and final placement.
        drop(namespace);
        let moved = target.join("Moved");
        let union = MultiNamespaceGuard::acquire(&context, &[(&moved, &fixture)]).unwrap();
        union.move_source_to(0, &fixture.join("Parking")).unwrap();
        union.move_source_to(0, &fixture.join("Final")).unwrap();
        assert_eq!(
            union.directory_identity(&moved).unwrap(),
            Some(original_directory)
        );
        assert_eq!(
            std::fs::read(fixture.join("Final/Renamed.typ")).unwrap(),
            b"preserve exact bytes"
        );
    }
    // Strict rejection fixtures retain their original names and bytes. They
    // never become admitted data and are moved to an explicit evidence folder.
    {
        let rejected = fixture.join("rejections");
        context.create_private_directory(&rejected).unwrap();
        std::os::unix::fs::symlink("missing-target", rejected.join("link")).unwrap();
        assert!(
            ParentGuard::acquire(&context, &rejected.join("link"))
                .unwrap()
                .open_file(false, false)
                .is_err()
        );
        assert!(
            ParentGuard::acquire(&context, &rejected.join(".scan"))
                .unwrap()
                .directory_entries(100)
                .is_err()
        );
        let non_utf8 = rejected.join(OsString::from_vec(vec![0xff]));
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&non_utf8)
            .unwrap();
        assert!(ParentGuard::acquire(&context, &non_utf8).is_err());
        let original = rejected.join("hardlink-original");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&original)
            .unwrap();
        std::fs::hard_link(&original, rejected.join("hardlink-alias")).unwrap();
        assert!(
            ParentGuard::acquire(&context, &original)
                .unwrap()
                .open_file(false, false)
                .is_err()
        );
        let fifo = CString::new(rejected.join("fifo").as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert!(
            ParentGuard::acquire(&context, &rejected.join("fifo"))
                .unwrap()
                .open_file(false, false)
                .is_err()
        );
        assert!(
            ParentGuard::acquire(&context, &rejected.join(".scan"))
                .unwrap()
                .directory_entries(100)
                .is_err()
        );
        let retained = quarantine.join(format!("{}-rejections", crate::fresh()));
        assert!(!retained.exists());
        std::fs::rename(&rejected, &retained).unwrap();
        File::open(&fixture).unwrap().sync_all().unwrap();
        File::open(&quarantine).unwrap().sync_all().unwrap();
    }
    let data = fixture.join("private");
    let library = fixture.join("library");
    let mut core = crate::Core::with_backend(&data, context).unwrap();
    assert!(core.existing_device_identity().is_err());
    let device = core.device_identity().unwrap();
    assert!(device.is_current);
    assert_eq!(core.existing_device_identity().unwrap(), device);
    let device_path = core.device_record_path(crate::device_state::DeviceRecordKind::Identity);
    let device_bytes = std::fs::read(&device_path).unwrap();
    assert!(!device_bytes.windows(9).any(|bytes| bytes == b"keyFile\":"));
    let mut tampered = core.backend.seal_linux_device(b"binding").unwrap();
    tampered[0] ^= 1;
    assert!(
        core.backend
            .unseal_linux_device(&tampered)
            .unwrap()
            .is_none()
    );
    core.create_library(&library).unwrap();
    let doc = core
        .create_note_fixture(&library, &library, "Linux")
        .unwrap();
    let saved_source = format!("{}\n服务端保存😀\n", doc.source);
    let operation_id = crate::fresh();
    let saved = core
        .save(crate::SaveRequest {
            session_id: doc.session_id.clone(),
            base_id: doc.base_id,
            source: saved_source.clone(),
            operation_id: operation_id.clone(),
        })
        .unwrap();
    assert_eq!(std::fs::read_to_string(&doc.path).unwrap(), saved_source);
    assert_eq!(
        core.operation_status(&operation_id).unwrap().status,
        "committed"
    );
    let original_file = stamp(
        &ParentGuard::acquire(&core.backend, Path::new(&doc.path))
            .unwrap()
            .open_file(false, false)
            .unwrap(),
    )
    .unwrap()
    .identity;
    drop(core);
    let context = BackendContext::controlled_service(&enrollment).unwrap();
    let mut core = crate::Core::with_backend(&data, context).unwrap();
    assert_eq!(core.device_identity().unwrap().id, device.id);
    assert!(core.device_identity().unwrap().is_current);
    let reopened = core.open(&doc.path, Some(library.clone())).unwrap();
    assert_eq!(reopened.source, saved_source);
    assert_eq!(saved.receipt.operation_id, operation_id);
    assert_eq!(reopened.node_id, doc.node_id);
    assert_eq!(
        core.operation_status(&operation_id).unwrap().status,
        "committed"
    );
    assert_eq!(
        stamp(
            &ParentGuard::acquire(&core.backend, Path::new(&doc.path))
                .unwrap()
                .open_file(false, false)
                .unwrap()
        )
        .unwrap()
        .identity,
        original_file
    );
    drop(core);
    let context = BackendContext::controlled_service(&enrollment).unwrap();
    let copied_profile = fixture.join("copied-profile");
    let mut copied = crate::Core::with_backend(&copied_profile, context).unwrap();
    let copied_identity =
        copied.device_record_path(crate::device_state::DeviceRecordKind::Identity);
    crate::journal::private_create(&copied.backend, &copied_identity, &device_bytes).unwrap();
    let foreign = copied.device_identity().unwrap();
    assert_eq!(foreign.id, device.id);
    assert!(
        !foreign.is_current,
        "profile copy must not acquire the original device identity"
    );
    assert_eq!(std::fs::read(copied_identity).unwrap(), device_bytes);
    drop(copied);
    let context = BackendContext::controlled_service(&alternate).unwrap();
    let mut alternate_core = crate::Core::with_backend(&data, context).unwrap();
    let foreign = alternate_core.device_identity().unwrap();
    assert_eq!(foreign.id, device.id);
    assert!(
        !foreign.is_current,
        "another enrollment cannot adopt the original device binding"
    );
    assert_eq!(std::fs::read(device_path).unwrap(), device_bytes);
}

#[test]
fn linux_case_distinct_pending_targets_have_distinct_original_coordinates() {
    let a = Path::new("/enrolled/Documents/A.typ");
    let b = Path::new("/enrolled/Documents/a.typ");
    assert_ne!(
        namespace_path_compare(a, b).unwrap(),
        std::cmp::Ordering::Equal
    );
    assert_eq!(namespace_relative_path(b, a).unwrap(), None);
    assert_eq!(
        namespace_path_compare(a, Path::new("/enrolled/Documents/A.typ")).unwrap(),
        std::cmp::Ordering::Equal
    );
}
