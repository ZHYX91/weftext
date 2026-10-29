use super::*;

struct Fixture {
    temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    source: PathBuf,
    core: Core,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("Library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let source = core
            .create_note_fixture(&root, &root, "Note")
            .unwrap()
            .path
            .into();
        Self {
            temp,
            data,
            root,
            source,
            core,
        }
    }
    fn files(&self, names: &[&str]) -> Vec<ResourceImportUpload> {
        names
            .iter()
            .map(|name| ResourceImportUpload {
                name: (*name).into(),
                bytes: name.as_bytes().to_vec(),
            })
            .collect()
    }
    fn prepare(&mut self, names: &[&str], id: &str) -> Result<OperationPreview> {
        self.core
            .prepare_import_resources(&self.root, &self.source, self.files(names), id)
    }
    fn target(&self, name: &str) -> PathBuf {
        self.source.parent().unwrap().join(name)
    }
    fn restart(&mut self) {
        let placeholder = Core::new(self.temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, placeholder));
        self.core = Core::new(&self.data).unwrap();
    }
    fn untouched(&self, names: &[&str]) {
        for name in names {
            assert!(!self.target(name).exists(), "unexpected {name}");
        }
    }
}

#[test]
fn resource_batch_one_plan_ordered_results_survive_restart_and_original_retry() {
    let mut f = Fixture::new();
    let id = fresh();
    let plan = f.prepare(&["z.pdf", "a.png", "中.jpg"], &id).unwrap();
    assert_eq!(plan.kind, "importResources");
    assert_eq!(plan.changes.len(), 4);
    f.untouched(&["z.pdf", "a.png", "中.jpg", "weftext.resources.json"]);
    let proposed = f.core.resource_import_results(&id).unwrap();
    assert_eq!(
        proposed
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["z.pdf", "a.png", "中.jpg"]
    );
    assert!(
        f.core
            .validate_resource_import_insertion(&f.root, &f.source, &id)
            .is_err()
    );
    let source = fs::read(&f.source).unwrap();
    f.core.execute_operation(&id, &plan.plan_id).unwrap();
    assert_eq!(fs::read(&f.source).unwrap(), source);
    assert_eq!(
        f.core
            .validate_resource_import_insertion(&f.root, &f.source, &id)
            .unwrap(),
        proposed
    );
    f.restart();
    assert_eq!(f.core.resource_import_results(&id).unwrap(), proposed);
    let repeated = f.prepare(&["z.pdf", "a.png", "中.jpg"], &id).unwrap();
    assert_eq!(repeated.plan_id, plan.plan_id);
    assert_eq!(f.core.resource_import_results(&id).unwrap(), proposed);
    assert!(f.prepare(&["a.png", "z.pdf", "中.jpg"], &id).is_err());
    for item in proposed {
        assert_eq!(
            fs::read(f.target(&item.name)).unwrap(),
            item.name.as_bytes()
        );
        assert!(item.resource_ref.starts_with("weftext-resource://"));
        assert!(!item.exact_revision.is_empty());
    }
}

#[test]
fn resource_batch_last_conflict_and_duplicate_selection_create_nothing() {
    let mut f = Fixture::new();
    fs::write(f.target("occupied.pdf"), b"original").unwrap();
    assert_eq!(
        f.prepare(&["first.png", "occupied.pdf"], &fresh())
            .unwrap_err()
            .code,
        "exists"
    );
    f.untouched(&["first.png", "weftext.resources.json"]);
    assert_eq!(fs::read(f.target("occupied.pdf")).unwrap(), b"original");
    for names in [
        &[][..],
        &["same.png", "same.png"][..],
        &["A.png", "a.png"][..],
    ] {
        assert!(f.prepare(names, &fresh()).is_err());
    }
    f.untouched(&["same.png", "A.png", "weftext.resources.json"]);
    assert!(
        f.prepare(&["first.png", "weftext.resources.json"], &fresh())
            .is_err()
    );
    assert!(f.prepare(&["first.png", "Note.typ"], &fresh()).is_err());
    f.untouched(&["first.png", "weftext.resources.json"]);
}

#[test]
fn resource_batch_changed_owner_or_manifest_rejects_before_install() {
    let mut f = Fixture::new();
    let id = fresh();
    let plan = f.prepare(&["first.png", "last.pdf"], &id).unwrap();
    fs::write(&f.source, b"new author input").unwrap();
    assert!(f.core.execute_operation(&id, &plan.plan_id).is_err());
    f.untouched(&["first.png", "last.pdf", "weftext.resources.json"]);
    assert_eq!(fs::read(&f.source).unwrap(), b"new author input");

    let mut f = Fixture::new();
    let id = fresh();
    let plan = f.prepare(&["first.png", "last.pdf"], &id).unwrap();
    fs::write(f.target("weftext.resources.json"), b"external manifest").unwrap();
    assert!(f.core.execute_operation(&id, &plan.plan_id).is_err());
    f.untouched(&["first.png", "last.pdf"]);
    assert_eq!(
        fs::read(f.target("weftext.resources.json")).unwrap(),
        b"external manifest"
    );
}

#[test]
fn resource_batch_budget_and_manifest_capacity_are_cumulative() {
    let mut f = Fixture::new();
    let uploads = vec![
        ResourceImportUpload {
            name: "a.bin".into(),
            bytes: vec![0; 32 * 1024 * 1024],
        },
        ResourceImportUpload {
            name: "b.bin".into(),
            bytes: vec![0; 32 * 1024 * 1024 + 1],
        },
    ];
    assert_eq!(
        f.core
            .prepare_import_resources(&f.root, &f.source, uploads, &fresh())
            .unwrap_err()
            .code,
        "budget"
    );
    f.untouched(&["a.bin", "b.bin", "weftext.resources.json"]);
    let names: Vec<_> = (0..63).map(|index| format!("{index}.bin")).collect();
    let refs: Vec<_> = names.iter().map(String::as_str).collect();
    let id = fresh();
    let plan = f.prepare(&refs, &id).unwrap();
    f.core.execute_operation(&id, &plan.plan_id).unwrap();
    let before = fs::read(f.target("weftext.resources.json")).unwrap();
    assert_eq!(
        f.prepare(&["new-a.bin", "new-b.bin"], &fresh())
            .unwrap_err()
            .code,
        "budget"
    );
    f.untouched(&["new-a.bin", "new-b.bin"]);
    assert_eq!(
        fs::read(f.target("weftext.resources.json")).unwrap(),
        before
    );
}

#[test]
fn resource_batch_existing_sixty_four_descriptor_budget_includes_one_manifest() {
    let mut f = Fixture::new();
    let names: Vec<_> = (0..64).map(|index| format!("{index}.bin")).collect();
    let refs: Vec<_> = names.iter().map(String::as_str).collect();
    let id = fresh();
    let plan = f.prepare(&refs, &id).unwrap();
    assert_eq!(plan.resource_map.len(), 64);
    f.core.execute_operation(&id, &plan.plan_id).unwrap();
    assert_eq!(
        f.core
            .validate_resource_import_insertion(&f.root, &f.source, &id)
            .unwrap()
            .len(),
        64
    );
    let before = fs::read(f.target("weftext.resources.json")).unwrap();
    assert!(f.prepare(&["sixty-fifth.bin"], &fresh()).is_err());
    assert_eq!(
        fs::read(f.target("weftext.resources.json")).unwrap(),
        before
    );
    f.untouched(&["sixty-fifth.bin"]);
    let mut another = Fixture::new();
    let names: Vec<_> = (0..65).map(|index| format!("{index}.bin")).collect();
    let refs: Vec<_> = names.iter().map(String::as_str).collect();
    assert!(another.prepare(&refs, &fresh()).is_err());
    another.untouched(&["0.bin", "64.bin", "weftext.resources.json"]);
}

#[test]
fn resource_batch_delivery_requires_current_owner_manifest_and_exact_resources() {
    let mut f = Fixture::new();
    let id = fresh();
    let plan = f.prepare(&["first.png", "last.pdf"], &id).unwrap();
    f.core.execute_operation(&id, &plan.plan_id).unwrap();
    let other = PathBuf::from(
        f.core
            .create_note_fixture(&f.root, &f.root, "Other")
            .unwrap()
            .path,
    );
    assert!(
        f.core
            .validate_resource_import_insertion(&f.root, &other, &id)
            .is_err()
    );
    assert!(
        f.core
            .resource_import_preview_at(&f.root, &other, &id)
            .is_err()
    );
    fs::write(f.target("last.pdf"), b"external replacement").unwrap();
    assert!(
        f.core
            .validate_resource_import_insertion(&f.root, &f.source, &id)
            .is_err()
    );
    // Failure to deliver a Draft proposal never undoes the original commit.
    assert!(f.core.committed_operation_receipt(&id).unwrap().is_some());
    assert_eq!(fs::read(f.target("first.png")).unwrap(), b"first.png");
}

#[test]
fn resource_batch_cannot_reinitialize_previously_admitted_removed_manifest() {
    let mut f = Fixture::new();
    let id = fresh();
    let plan = f.prepare(&["first.png"], &id).unwrap();
    f.core.execute_operation(&id, &plan.plan_id).unwrap();
    fs::remove_file(f.target("weftext.resources.json")).unwrap();
    assert!(f.prepare(&["second.png", "third.pdf"], &fresh()).is_err());
    f.untouched(&["second.png", "third.pdf", "weftext.resources.json"]);
}

#[test]
fn resource_batch_delivery_manifest_witness_rejects_a_change_after_resource_cut() {
    let mut f = Fixture::new();
    let id = fresh();
    let plan = f.prepare(&["first.png", "last.pdf"], &id).unwrap();
    f.core.execute_operation(&id, &plan.plan_id).unwrap();
    let resources = f
        .core
        .managed_resource_snapshot(&f.root, &f.source)
        .unwrap();
    let witness = f
        .core
        .take_snapshot_witness(&resources[0].manifest_base_id)
        .unwrap();
    let revision = plan.revisions[&display(&f.target("weftext.resources.json"))].clone();
    f.core
        .validate_import_manifest(&f.root, &witness, Some(&revision))
        .unwrap();
    let manifest = f.target("weftext.resources.json");
    let mut bytes = fs::read(&manifest).unwrap();
    bytes.extend_from_slice(b"\n");
    fs::write(&manifest, bytes).unwrap();
    // Resource identities and bytes stayed equal; this must still fail for the
    // actual manifest cut, including a semantically identical external edit.
    assert!(
        f.core
            .validate_import_manifest(&f.root, &witness, Some(&revision))
            .is_err()
    );
    assert!(
        f.core
            .validate_resource_import_insertion(&f.root, &f.source, &id)
            .is_err()
    );
    assert!(f.core.committed_operation_receipt(&id).unwrap().is_some());
}

#[test]
fn resource_batch_draft_inputs_are_one_ordered_cut_without_retained_snapshot_payloads() {
    let mut f = Fixture::new();
    let id = fresh();
    let plan = f.prepare(&["z.pdf", "a.png"], &id).unwrap();
    f.core.execute_operation(&id, &plan.plan_id).unwrap();
    let retained = f.core.snapshots.len();
    for _ in 0..2 {
        let (imports, bytes) = f
            .core
            .resource_import_insertion_inputs(&f.root, &f.source, &id)
            .unwrap();
        assert_eq!(
            imports
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["z.pdf", "a.png"]
        );
        assert_eq!(bytes, vec![b"z.pdf".to_vec(), b"a.png".to_vec()]);
        assert_eq!(f.core.snapshots.len(), retained);
    }
    fs::write(f.target("a.png"), b"later").unwrap();
    assert!(
        f.core
            .resource_import_insertion_inputs(&f.root, &f.source, &id)
            .is_err()
    );
    assert_eq!(f.core.snapshots.len(), retained);
}
