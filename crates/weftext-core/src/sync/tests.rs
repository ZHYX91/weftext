use super::*;
use crate::fresh;

struct Fixture {
    workspace: String,
    replica: String,
    node: ObjectKey,
    other: ObjectKey,
    blobs: BTreeMap<String, Vec<u8>>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            workspace: fresh(),
            replica: fresh(),
            node: ObjectKey::Node(fresh()),
            other: ObjectKey::Node(fresh()),
            blobs: BTreeMap::new(),
        }
    }
    fn state(&mut self, text: &str) -> ObjectState {
        let blob = hash(text.as_bytes());
        self.blobs.insert(blob.clone(), text.as_bytes().to_vec());
        ObjectState {
            file_revisions: BTreeMap::from([("Note.typ".into(), fresh())]),
            parent: None,
            name: "Note".into(),
            retired: false,
            files: BTreeMap::from([("Note.typ".into(), blob)]),
        }
    }
    fn change(&mut self, object: ObjectKey, before: &[&str], text: &str) -> Change {
        Change {
            object,
            before: before.iter().map(|s| (*s).into()).collect(),
            revision: fresh(),
            after: self.state(text),
        }
    }
    fn manifest(&self, parents: &[&str], changes: Vec<Change>) -> Manifest {
        Manifest {
            version: 1,
            workspace_id: self.workspace.clone(),
            replica_id: self.replica.clone(),
            operation_id: fresh(),
            parents: parents.iter().map(|s| (*s).into()).collect(),
            changes,
            dependencies: Vec::new(),
        }
    }
    fn graph(&self, genesis: &Manifest) -> ReplicaGraph {
        ReplicaGraph::new(&self.workspace, &genesis.id().unwrap()).unwrap()
    }
    fn offer(&self, graph: &mut ReplicaGraph, manifest: &Manifest) -> String {
        for (id, blob) in &self.blobs {
            graph.offer_blob(id, blob.clone()).unwrap();
        }
        let id = manifest.id().unwrap();
        assert_eq!(
            graph.offer_manifest(&id, manifest.clone()).unwrap(),
            Admission::Accepted
        );
        id
    }
}

#[test]
fn reordered_child_and_missing_blob_never_become_heads() {
    let mut f = Fixture::new();
    let a = f.change(f.node.clone(), &[], "base");
    let genesis = f.manifest(&[], vec![a.clone()]);
    let root = genesis.id().unwrap();
    let b = f.change(f.node.clone(), &[&a.revision], "offline change");
    let child = f.manifest(&[&root], vec![b.clone()]);
    let child_id = child.id().unwrap();
    let mut graph = f.graph(&genesis);
    assert!(matches!(
        graph.offer_manifest(&child_id, child.clone()).unwrap(),
        Admission::Staged { .. }
    ));
    assert!(graph.heads().is_empty());
    graph.offer_manifest(&root, genesis).unwrap();
    let base_blob = a.after.files.values().next().unwrap();
    graph
        .offer_blob(base_blob, f.blobs[base_blob].clone())
        .unwrap();
    assert_eq!(graph.heads(), BTreeSet::from([root.clone()]));
    assert!(graph.view_at(&BTreeSet::from([child_id.clone()])).is_err());
    let child_blob = b.after.files.values().next().unwrap();
    graph
        .offer_blob(child_blob, f.blobs[child_blob].clone())
        .unwrap();
    assert_eq!(graph.heads(), BTreeSet::from([child_id.clone()]));
    assert_eq!(
        graph.offer_manifest(&child_id, child).unwrap(),
        Admission::Accepted
    );
    assert_eq!(graph.objects().unwrap()[&f.node].versions[0].state, b.after);
}

#[test]
fn concurrent_edits_preserve_both_sides_common_base_and_unrelated_progress() {
    let mut f = Fixture::new();
    let base = f.change(f.node.clone(), &[], "base");
    let other = f.change(f.other.clone(), &[], "other base");
    let genesis = f.manifest(&[], vec![base.clone(), other.clone()]);
    let mut graph = f.graph(&genesis);
    let root = f.offer(&mut graph, &genesis);
    let alice = f.change(f.node.clone(), &[&base.revision], "Alice offline");
    let bob = f.change(f.node.clone(), &[&base.revision], "Bob offline");
    let ma = f.manifest(&[&root], vec![alice.clone()]);
    let mut mb = f.manifest(&[&root], vec![bob.clone()]);
    mb.replica_id = fresh();
    let ca = f.offer(&mut graph, &ma);
    let cb = f.offer(&mut graph, &mb);
    let view = graph.objects().unwrap();
    assert_eq!(view[&f.node].versions.len(), 2);
    assert_eq!(view[&f.node].common_bases[0].revision, base.revision);
    assert_eq!(view[&f.other].versions.len(), 1);
    let next_other = f.change(
        f.other.clone(),
        &[&other.revision],
        "Unrelated work continues",
    );
    let next = f.manifest(&[&ca, &cb], vec![next_other.clone()]);
    let next_id = f.offer(&mut graph, &next);
    assert_eq!(graph.objects().unwrap()[&f.node].versions.len(), 2);
    let merge = f.change(
        f.node.clone(),
        &[&alice.revision, &bob.revision],
        "Explicit both-side merge",
    );
    let resolved = f.manifest(&[&next_id], vec![merge.clone()]);
    f.offer(&mut graph, &resolved);
    assert_eq!(
        graph.objects().unwrap()[&f.node].versions[0].revision,
        merge.revision
    );
    assert_eq!(
        graph.objects().unwrap()[&f.other].versions[0].revision,
        next_other.revision
    );
    assert!(graph.manifest(&ca).is_some());
    assert!(graph.manifest(&cb).is_some());
}

#[test]
fn reverse_delivery_has_the_same_conflict_not_a_different_winner() {
    let mut f = Fixture::new();
    let base = f.change(f.node.clone(), &[], "base");
    let genesis = f.manifest(&[], vec![base.clone()]);
    let root = genesis.id().unwrap();
    let a = f.change(f.node.clone(), &[&base.revision], "left");
    let b = f.change(f.node.clone(), &[&base.revision], "right");
    let ma = f.manifest(&[&root], vec![a]);
    let mb = f.manifest(&[&root], vec![b]);
    let mut first = f.graph(&genesis);
    let mut second = f.graph(&genesis);
    for m in [&genesis, &ma, &mb] {
        f.offer(&mut first, m);
    }
    for (id, blob) in &f.blobs {
        second.offer_blob(id, blob.clone()).unwrap();
    }
    for m in [&mb, &ma, &genesis] {
        second.offer_manifest(&m.id().unwrap(), m.clone()).unwrap();
    }
    assert_eq!(first.heads(), second.heads());
    assert_eq!(first.objects().unwrap(), second.objects().unwrap());
}

#[test]
fn merge_must_name_all_versions_even_when_one_side_has_identical_bytes() {
    let mut f = Fixture::new();
    let base = f.change(f.node.clone(), &[], "base");
    let genesis = f.manifest(&[], vec![base.clone()]);
    let mut graph = f.graph(&genesis);
    let root = f.offer(&mut graph, &genesis);
    let a = f.change(f.node.clone(), &[&base.revision], "identical");
    let b = f.change(f.node.clone(), &[&base.revision], "identical");
    let ca = f.offer(&mut graph, &f.manifest(&[&root], vec![a.clone()]));
    let cb = f.offer(&mut graph, &f.manifest(&[&root], vec![b.clone()]));
    assert_eq!(graph.objects().unwrap()[&f.node].versions.len(), 2);
    let bad = f.change(
        f.node.clone(),
        &[&a.revision],
        "discarded Bob without review",
    );
    let manifest = f.manifest(&[&ca, &cb], vec![bad]);
    for (id, blob) in &f.blobs {
        graph.offer_blob(id, blob.clone()).unwrap();
    }
    assert!(matches!(
        graph
            .offer_manifest(&manifest.id().unwrap(), manifest)
            .unwrap(),
        Admission::Rejected { .. }
    ));
    assert_eq!(graph.heads(), BTreeSet::from([ca, cb]));
}

#[test]
fn move_edit_and_retire_edit_conflict_and_keep_original_content() {
    for retire in [false, true] {
        let mut f = Fixture::new();
        let base = f.change(f.node.clone(), &[], "base");
        let genesis = f.manifest(&[], vec![base.clone()]);
        let mut graph = f.graph(&genesis);
        let root = f.offer(&mut graph, &genesis);
        let mut structural = base.clone();
        structural.before = BTreeSet::from([base.revision.clone()]);
        structural.revision = fresh();
        structural.after.retired = retire;
        if !retire {
            structural.after.parent = Some(ObjectKey::Directory(fresh()));
        }
        let edit = f.change(f.node.clone(), &[&base.revision], "new draft");
        f.offer(&mut graph, &f.manifest(&[&root], vec![structural.clone()]));
        f.offer(&mut graph, &f.manifest(&[&root], vec![edit.clone()]));
        let view = graph.objects().unwrap();
        assert_eq!(view[&f.node].versions.len(), 2);
        assert!(
            view[&f.node]
                .versions
                .iter()
                .any(|v| v.state == structural.after)
        );
        assert!(view[&f.node].versions.iter().any(|v| v.state == edit.after));
        assert_eq!(view[&f.node].common_bases[0].state, base.after);
    }
}

#[test]
fn multi_object_transaction_is_not_half_admitted() {
    let mut f = Fixture::new();
    let a = f.change(f.node.clone(), &[], "source and annotations");
    let b = f.change(f.other.clone(), &[], "dependent other note");
    let manifest = f.manifest(&[], vec![a.clone(), b.clone()]);
    let id = manifest.id().unwrap();
    let mut graph = f.graph(&manifest);
    let blob = a.after.files.values().next().unwrap();
    graph.offer_blob(blob, f.blobs[blob].clone()).unwrap();
    graph.offer_manifest(&id, manifest).unwrap();
    assert!(graph.objects().unwrap().is_empty());
    let blob = b.after.files.values().next().unwrap();
    graph.offer_blob(blob, f.blobs[blob].clone()).unwrap();
    assert_eq!(graph.objects().unwrap().len(), 2);
}

#[test]
fn schema_dependency_is_explicit_and_detects_concurrent_change() {
    let mut f = Fixture::new();
    let base = f.change(f.node.clone(), &[], "base");
    let mut schema = f.change(ObjectKey::Schema, &[], "schema v1");
    let revision = schema.after.file_revisions.remove("Note.typ").unwrap();
    schema
        .after
        .file_revisions
        .insert(".weftext-schema.json".into(), revision);
    let blob = schema.after.files.remove("Note.typ").unwrap();
    schema
        .after
        .files
        .insert(".weftext-schema.json".into(), blob);
    let genesis = f.manifest(&[], vec![base.clone(), schema.clone()]);
    let mut graph = f.graph(&genesis);
    let root = f.offer(&mut graph, &genesis);
    let edit = f.change(f.node.clone(), &[&base.revision], "uses schema v1");
    let mut source_commit = f.manifest(&[&root], vec![edit]);
    source_commit.dependencies.push(Dependency {
        object: ObjectKey::Schema,
        revisions: BTreeSet::from([schema.revision.clone()]),
    });
    let source_id = f.offer(&mut graph, &source_commit);
    assert!(graph.unsettled_dependencies(&source_id).unwrap().is_empty());
    let mut next_schema = schema.clone();
    next_schema.before = BTreeSet::from([schema.revision]);
    next_schema.revision = fresh();
    f.offer(&mut graph, &f.manifest(&[&root], vec![next_schema]));
    assert_eq!(
        graph.unsettled_dependencies(&source_id).unwrap(),
        vec![ObjectKey::Schema]
    );
}

#[test]
fn tampering_wrong_workspace_and_operation_reuse_never_replace_original() {
    let mut f = Fixture::new();
    let base = f.change(f.node.clone(), &[], "original");
    let manifest = f.manifest(&[], vec![base]);
    let mut graph = f.graph(&manifest);
    let id = f.offer(&mut graph, &manifest);
    let mut changed = manifest.clone();
    changed.changes[0].after.name = "Tampered".into();
    assert!(graph.offer_manifest(&id, changed.clone()).is_err());
    assert!(
        graph
            .offer_manifest(&changed.id().unwrap(), changed)
            .is_err()
    );
    let mut other = manifest.clone();
    other.workspace_id = fresh();
    assert!(graph.offer_manifest(&other.id().unwrap(), other).is_err());
    assert!(
        graph
            .offer_blob(&hash(b"original"), b"replaced".to_vec())
            .is_err()
    );
    assert_eq!(graph.manifest(&id).unwrap(), &manifest);
}

#[test]
fn fabricated_predecessor_and_reused_revision_are_rejected() {
    let mut f = Fixture::new();
    let base = f.change(f.node.clone(), &[], "base");
    let genesis = f.manifest(&[], vec![base.clone()]);
    let mut graph = f.graph(&genesis);
    let root = f.offer(&mut graph, &genesis);
    for reuse in [false, true] {
        let mut change = f.change(f.node.clone(), &[&base.revision], "forged");
        if reuse {
            change.revision = base.revision.clone();
            change.before.clear();
        } else {
            change.before = BTreeSet::from([fresh()]);
        }
        let m = f.manifest(&[&root], vec![change]);
        for (id, b) in &f.blobs {
            graph.offer_blob(id, b.clone()).unwrap();
        }
        assert!(matches!(
            graph.offer_manifest(&m.id().unwrap(), m).unwrap(),
            Admission::Rejected { .. }
        ));
    }
    assert_eq!(graph.heads(), BTreeSet::from([root]));
}

#[test]
fn paths_are_portable_and_never_host_paths_or_runtime_state() {
    let mut f = Fixture::new();
    for path in [
        "../secret",
        "a/../secret",
        "C:/secret",
        "/etc/passwd",
        "a\\b",
        "NUL.txt",
        "a/CON",
        "a.",
        "a ",
        "a//b",
        ".weftext-workspace.json",
        "hosted-policy.json",
    ] {
        let mut change = f.change(f.node.clone(), &[], "bytes");
        let blob = change.after.files.remove("Note.typ").unwrap();
        change.after.files.insert(path.into(), blob);
        let revision = change.after.file_revisions.remove("Note.typ").unwrap();
        change.after.file_revisions.insert(path.into(), revision);
        assert!(f.manifest(&[], vec![change]).id().is_err(), "{path}");
    }
    let mut change = f.change(f.node.clone(), &[], "bytes");
    change.after.files.insert("note.typ".into(), hash(b"other"));
    change
        .after
        .file_revisions
        .insert("note.typ".into(), fresh());
    assert!(f.manifest(&[], vec![change]).id().is_err());
}

#[test]
fn no_private_job_role_and_no_unknown_manifest_fields() {
    assert!(
        serde_json::from_value::<ObjectKey>(serde_json::json!({"kind":"job","id":fresh()}))
            .is_err()
    );
    let mut f = Fixture::new();
    let change = f.change(f.node.clone(), &[], "bytes");
    let mut value = serde_json::to_value(f.manifest(&[], vec![change])).unwrap();
    value["approval"] = serde_json::json!("execution authority");
    assert!(serde_json::from_value::<Manifest>(value).is_err());
}

#[test]
fn pairing_pins_genesis_even_if_an_unrelated_root_arrives_first() {
    let mut f = Fixture::new();
    let birth = f.change(f.node.clone(), &[], "correct origin");
    let genesis = f.manifest(&[], vec![birth]);
    let other_birth = f.change(f.other.clone(), &[], "unrelated origin");
    let unrelated = f.manifest(&[], vec![other_birth]);
    let mut graph = f.graph(&genesis);
    for (id, bytes) in &f.blobs {
        graph.offer_blob(id, bytes.clone()).unwrap();
    }
    assert!(matches!(
        graph
            .offer_manifest(&unrelated.id().unwrap(), unrelated)
            .unwrap(),
        Admission::Rejected { .. }
    ));
    assert!(graph.heads().is_empty());
    let id = f.offer(&mut graph, &genesis);
    assert_eq!(graph.heads(), BTreeSet::from([id]));
}

#[test]
fn partial_transfer_fetches_dependencies_and_both_conflicts_but_not_unrelated_bytes() {
    let mut f = Fixture::new();
    let genesis = f.manifest(&[], vec![]);
    let mut graph = f.graph(&genesis);
    let root = f.offer(&mut graph, &genesis);
    let birth = f.change(f.node.clone(), &[], "selected base");
    let other = f.change(f.other.clone(), &[], "large unselected media");
    let base = f.offer(&mut graph, &f.manifest(&[&root], vec![birth.clone()]));
    let irrelevant = f.offer(&mut graph, &f.manifest(&[&root], vec![other.clone()]));
    let left = f.change(f.node.clone(), &[&birth.revision], "offline left");
    let right = f.change(f.node.clone(), &[&birth.revision], "offline right");
    f.offer(&mut graph, &f.manifest(&[&base], vec![left]));
    f.offer(&mut graph, &f.manifest(&[&base], vec![right]));
    let bundle = graph
        .export_objects(&BTreeSet::from([f.node.clone()]))
        .unwrap();
    assert!(!bundle.manifests.contains_key(&irrelevant));
    assert!(
        !bundle
            .blobs
            .contains_key(other.after.files.values().next().unwrap())
    );
    let mut receiver = f.graph(&genesis);
    // Manifests before bytes, in arbitrary digest order, is a valid transport.
    for (id, manifest) in bundle.manifests {
        receiver.offer_manifest(&id, manifest).unwrap();
    }
    assert!(receiver.objects().unwrap().is_empty());
    for (id, bytes) in bundle.blobs {
        receiver.offer_blob(&id, bytes).unwrap();
    }
    let received = receiver.objects().unwrap();
    assert_eq!(received[&f.node], graph.objects().unwrap()[&f.node]);
    assert!(!received.contains_key(&f.other));
    assert!(receiver.export_objects(&BTreeSet::from([f.other])).is_err());
}

#[test]
fn partial_selection_includes_the_whole_compound_transaction() {
    let mut f = Fixture::new();
    let genesis = f.manifest(&[], vec![]);
    let mut graph = f.graph(&genesis);
    let root = f.offer(&mut graph, &genesis);
    let first = f.change(f.node.clone(), &[], "source");
    let second = f.change(f.other.clone(), &[], "required paired effect");
    f.offer(
        &mut graph,
        &f.manifest(&[&root], vec![first, second.clone()]),
    );
    let bundle = graph
        .export_objects(&BTreeSet::from([f.node.clone()]))
        .unwrap();
    assert!(
        bundle
            .blobs
            .contains_key(second.after.files.values().next().unwrap())
    );
    assert_eq!(
        bundle
            .manifests
            .values()
            .map(|m| m.changes.len())
            .sum::<usize>(),
        2
    );
}

#[test]
fn several_hundred_offline_versions_keep_exact_head_and_common_base() {
    let mut f = Fixture::new();
    let birth = f.change(f.node.clone(), &[], "initial");
    let genesis = f.manifest(&[], vec![birth.clone()]);
    let mut graph = f.graph(&genesis);
    let root = f.offer(&mut graph, &genesis);
    let mut last = root.clone();
    let mut revision = birth.revision.clone();
    for i in 0..256 {
        let next = f.change(
            f.node.clone(),
            &[&revision],
            &format!("offline version {i}"),
        );
        revision = next.revision.clone();
        last = f.offer(&mut graph, &f.manifest(&[&last], vec![next]));
    }
    assert_eq!(graph.heads(), BTreeSet::from([last]));
    let concurrent = f.change(
        f.node.clone(),
        &[&birth.revision],
        "other replica remained offline",
    );
    f.offer(&mut graph, &f.manifest(&[&root], vec![concurrent]));
    let view = graph.objects().unwrap();
    assert_eq!(view[&f.node].versions.len(), 2);
    assert_eq!(view[&f.node].common_bases[0].revision, birth.revision);
}

#[test]
fn file_revision_cannot_rebind_other_bytes_or_other_owner() {
    let mut f = Fixture::new();
    let base = f.change(f.node.clone(), &[], "original");
    let genesis = f.manifest(&[], vec![base.clone()]);
    let mut graph = f.graph(&genesis);
    let root = f.offer(&mut graph, &genesis);
    let mut bad = f.change(f.node.clone(), &[&base.revision], "changed bytes");
    bad.after.file_revisions = base.after.file_revisions.clone();
    let manifest = f.manifest(&[&root], vec![bad]);
    for (id, blob) in &f.blobs {
        graph.offer_blob(id, blob.clone()).unwrap();
    }
    assert!(matches!(
        graph
            .offer_manifest(&manifest.id().unwrap(), manifest)
            .unwrap(),
        Admission::Rejected { .. }
    ));
    let mut bad = f.change(f.other.clone(), &[], "original");
    bad.after.file_revisions = base.after.file_revisions.clone();
    let manifest = f.manifest(&[&root], vec![bad]);
    assert!(matches!(
        graph
            .offer_manifest(&manifest.id().unwrap(), manifest)
            .unwrap(),
        Admission::Rejected { .. }
    ));
    assert_eq!(
        graph.objects().unwrap()[&f.node].versions[0].revision,
        base.revision
    );
    assert!(!graph.objects().unwrap().contains_key(&f.other));
}
