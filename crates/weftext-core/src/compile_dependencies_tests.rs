use super::*;

struct Fixture {
    _temp: tempfile::TempDir,
    core: Core,
    root: PathBuf,
    helper: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let root = temp.path().join("library");
        core.create_library(&root).unwrap();
        core.create_note(&root, &root, "Note").unwrap();
        let helper = root.join("Note/helper.typ");
        fs::write(&helper, "#let value = 1").unwrap();
        fs::write(root.join("unused.raw"), "unrelated").unwrap();
        Self {
            _temp: temp,
            core,
            root,
            helper,
        }
    }
    fn cut(&mut self) -> String {
        self.core
            .admitted_compile_snapshot(&self.root, "live")
            .unwrap()
            .snapshot
            .cut_id
    }
    fn witness(
        &mut self,
        cut: &str,
        paths: &[(&str, &str, &str)],
    ) -> Result<CompileDependencyWitness> {
        let deps = paths
            .iter()
            .map(|(path, kind, status)| CompileDependency::new(path, kind, status))
            .collect::<Result<Vec<_>>>()?;
        self.core
            .take_compile_dependency_witness(&self.root, cut, &deps)
    }
}

#[test]
fn actual_raw_input_uses_old_cut_and_ignores_unused_changes() {
    let mut f = Fixture::new();
    let cut = f.cut();
    fs::write(f.root.join("unused.raw"), "changed but not read").unwrap();
    let witness = f
        .witness(
            &cut,
            &[
                ("Note/helper.typ", "file", "loaded"),
                ("Note/helper.typ", "source", "loaded"),
            ],
        )
        .unwrap();
    assert_eq!(witness.dependency_count(), 1);
    assert!(!f.core.admitted_cuts.contains_key(&cut));
    assert!(!f.core.compile_dependency_cuts.contains_key(&cut));
    f.core
        .validate_compile_dependency_witness(&witness)
        .unwrap();
    fs::write(&f.helper, "#let value = 2").unwrap();
    assert!(
        f.core
            .validate_compile_dependency_witness(&witness)
            .is_err()
    );
    assert!(
        f.witness(&cut, &[("Note/helper.typ", "source", "loaded")])
            .is_err()
    );
}

#[test]
fn positive_identity_and_parent_replacement_are_not_matching_bytes() {
    let mut f = Fixture::new();
    let cut = f.cut();
    let witness = f
        .witness(&cut, &[("Note/helper.typ", "source", "loaded")])
        .unwrap();
    let raw = fs::read(&f.helper).unwrap();
    fs::rename(&f.helper, f.root.join("retained-helper.typ")).unwrap();
    fs::write(&f.helper, &raw).unwrap();
    assert!(
        f.core
            .validate_compile_dependency_witness(&witness)
            .is_err()
    );
    let cut = f.cut();
    let witness = f
        .witness(&cut, &[("Note/helper.typ", "source", "loaded")])
        .unwrap();
    fs::rename(f.root.join("Note"), f.root.join("retained-parent")).unwrap();
    fs::create_dir(f.root.join("Note")).unwrap();
    fs::rename(f.root.join("retained-parent/helper.typ"), &f.helper).unwrap();
    assert!(
        f.core
            .validate_compile_dependency_witness(&witness)
            .is_err()
    );
}

#[test]
fn unavailable_utf8_source_retains_positive_file_evidence() {
    let mut f = Fixture::new();
    fs::write(&f.helper, [0xff, 0xfe, 1]).unwrap();
    let cut = f.cut();
    let witness = f
        .witness(
            &cut,
            &[
                ("Note/helper.typ", "file", "loaded"),
                ("Note/helper.typ", "source", "unavailable"),
            ],
        )
        .unwrap();
    assert_eq!(witness.files.len(), 1);
    assert!(witness.absent.is_empty());
    f.core
        .validate_compile_dependency_witness(&witness)
        .unwrap();
    fs::write(&f.helper, [0xff, 0xfe, 2]).unwrap();
    assert!(
        f.core
            .validate_compile_dependency_witness(&witness)
            .is_err()
    );
}

#[test]
fn original_absence_is_not_repaired_by_new_snapshot_or_new_directory() {
    let mut f = Fixture::new();
    let old = f.cut();
    fs::write(f.root.join("late.typ"), "appeared after old cut").unwrap();
    let fresh = f.cut();
    assert!(
        f.witness(&old, &[("late.typ", "source", "unavailable")])
            .is_err()
    );
    let witness = f
        .witness(&fresh, &[("late.typ", "source", "loaded")])
        .unwrap();
    f.core
        .validate_compile_dependency_witness(&witness)
        .unwrap();
    let cut = f.cut();
    let witness = f
        .witness(&cut, &[("missing/deep/input.typ", "file", "unavailable")])
        .unwrap();
    assert_eq!(witness.absent[0].path, f.root.join("missing"));
    fs::create_dir(f.root.join("missing")).unwrap();
    assert!(
        f.core
            .validate_compile_dependency_witness(&witness)
            .is_err()
    );
}

#[test]
fn empty_original_directory_and_file_parent_blocker_are_exact_inputs() {
    let mut f = Fixture::new();
    fs::create_dir(f.root.join("empty")).unwrap();
    let cut = f.cut();
    let witness = f
        .witness(
            &cut,
            &[
                ("empty/missing.typ", "source", "unavailable"),
                ("empty", "file", "unavailable"),
                ("Note/helper.typ/child.typ", "source", "unavailable"),
            ],
        )
        .unwrap();
    assert_eq!(witness.directories.len(), 1);
    assert_eq!(witness.files.len(), 1);
    f.core
        .validate_compile_dependency_witness(&witness)
        .unwrap();
    fs::write(f.root.join("empty/unrelated.raw"), "allowed").unwrap();
    f.core
        .validate_compile_dependency_witness(&witness)
        .unwrap();
    fs::write(f.root.join("empty/missing.typ"), "late").unwrap();
    assert!(
        f.core
            .validate_compile_dependency_witness(&witness)
            .is_err()
    );
}

#[test]
fn virtual_inputs_require_exact_derived_kind_and_unoccupied_original_slot() {
    let mut f = Fixture::new();
    let cut = f.cut();
    assert!(
        f.witness(
            &cut,
            &[(".weftext-runtime-entry-exact.typ", "source", "loaded")]
        )
        .is_err()
    );
    let cut = f.cut();
    let witness = f
        .witness(
            &cut,
            &[(
                ".weftext-runtime-entry-exact.typ",
                "derived-source",
                "loaded",
            )],
        )
        .unwrap();
    f.core
        .validate_compile_dependency_witness(&witness)
        .unwrap();
    fs::write(
        f.root.join(".weftext-runtime-entry-exact.typ"),
        "new author collision",
    )
    .unwrap();
    assert!(
        f.core
            .validate_compile_dependency_witness(&witness)
            .is_err()
    );
    let cut = f.cut();
    assert!(
        f.witness(
            &cut,
            &[(
                ".weftext-runtime-entry-exact.typ",
                "derived-source",
                "loaded"
            )]
        )
        .is_err()
    );
    let cut = f.cut();
    assert!(
        f.witness(&cut, &[("Note/helper.typ", "derived-file", "loaded")])
            .is_err()
    );
}

#[test]
fn ordinary_or_foreign_cut_and_invalid_paths_cannot_create_read_authority() {
    let mut f = Fixture::new();
    let ordinary = f.core.admitted_snapshot(&f.root).unwrap();
    assert!(f.witness(&ordinary.cut_id, &[]).is_err());
    let other = f._temp.path().join("other");
    f.core.create_library(&other).unwrap();
    let cut = f.cut();
    assert!(
        f.core
            .take_compile_dependency_witness(&other, &cut, &[])
            .is_err()
    );
    for path in [
        "../escape.typ",
        "/outside.typ",
        "C:/outside.typ",
        "a/../b.typ",
        "a//b.typ",
        "a:stream",
        "",
    ] {
        assert!(
            CompileDependency::new(path, "file", "loaded").is_err(),
            "{path}"
        );
    }
    assert!(CompileDependency::new("x.typ", "unknown", "loaded").is_err());
    assert!(CompileDependency::new("x.typ", "source", "unknown").is_err());
}

#[test]
fn conflicting_original_duplicate_basis_is_never_silently_overwritten() {
    let mut f = Fixture::new();
    let cut = f.cut();
    let base = f.core.admitted_cuts[&cut]
        .bases
        .iter()
        .find(|base| {
            backend::namespace_path_compare(Path::new(&f.core.snapshots[*base].path), &f.helper)
                .unwrap()
                .is_eq()
        })
        .unwrap()
        .clone();
    let mut competing = f.core.snapshots[&base].clone();
    competing.captured.as_mut().unwrap().bytes.push(0);
    let another = fresh();
    f.core.snapshots.insert(another.clone(), competing);
    f.core
        .admitted_cuts
        .get_mut(&cut)
        .unwrap()
        .bases
        .push(another);
    assert!(
        f.witness(&cut, &[("Note/helper.typ", "source", "loaded")])
            .is_err()
    );
    assert!(!f.core.admitted_cuts.contains_key(&cut));
}

#[test]
fn positive_handles_coexist_through_callback_and_negative_appearance_is_rechecked() {
    let mut f = Fixture::new();
    let cut = f.cut();
    let witness = f
        .witness(
            &cut,
            &[
                ("Note/helper.typ", "source", "loaded"),
                ("late.typ", "file", "unavailable"),
            ],
        )
        .unwrap();
    let result = f.core.with_validated_compile_dependencies(&witness, |_| {
        assert!(fs::write(&f.helper, "blocked by original positive read").is_err());
        assert!(fs::rename(&f.helper, f.root.join("moved.typ")).is_err());
        fs::write(f.root.join("unused.raw"), "unrelated succeeds").unwrap();
        fs::write(
            f.root.join("late.typ"),
            "negative slots are not sustained locks",
        )
        .unwrap();
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&f.helper).unwrap(), "#let value = 1");
}
