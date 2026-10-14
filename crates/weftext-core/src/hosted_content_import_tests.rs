use super::*;
use std::os::windows::fs::OpenOptionsExt;

const ACTOR: &str = "deployment:alice:content-import";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn hosted_content_import_generated_bibliography_loss_is_fixed_and_requires_original_plan_acceptance()
 {
    let mut f = Fixture::new();
    let note = f.core.create_note(&f.root, &f.root, "Note").unwrap();
    let mut package = weftext_export::exchange::read_native_package(
        &f.core.export_native_package(&f.root).unwrap(),
    )
    .unwrap();
    weftext_export::exchange::append_generated_bibliography(
        &mut package,
        note.node_id.as_ref().unwrap(),
        b"@article{work, title={Exact retained work}}\n".to_vec(),
    )
    .unwrap();
    package
        .manifest
        .losses
        .push(weftext_export::exchange::NativeLoss {
            path: "PRIVATE_SCHEMA_PATH".into(),
            code: "PRIVATE_UNKNOWN_CODE".into(),
            detail: "PRIVATE_PRODUCER_DETAILS".into(),
        });
    let bytes = weftext_export::exchange::write_native_package(&package).unwrap();
    let original = bytes.clone();
    let operation = fresh();
    let prepared = f
        .core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            core.actor_feature_prepare_content_import(
                "Bundle",
                &operation,
                FP,
                OperationRequest {
                    operation_id: fresh(),
                    root: display(&f.root),
                    intent: OperationIntent::ImportNative {
                        parent: display(&f.root),
                        name: "Bundle".into(),
                        bytes,
                        allow_losses: false,
                    },
                },
                serde_json::json!({"kind":"nativeImport"}),
                &[],
                "Bundle",
            )
        })
        .unwrap();
    let loss = prepared.preview["losses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|loss| loss["code"] == "generatedBibliographyNotInstalled")
        .unwrap();
    assert!(loss["message"].as_str().unwrap().contains("不安装"));
    let public = serde_json::to_string(&prepared).unwrap();
    for secret in [
        "PRIVATE_SCHEMA_PATH",
        "PRIVATE_UNKNOWN_CODE",
        "PRIVATE_PRODUCER_DETAILS",
        "weftext.citations.bib",
    ] {
        assert!(!public.contains(secret));
    }
    let reviewed = prepared.preview.clone();
    f.restart();
    f.core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            let retry = core.actor_feature_retry("Bundle", &operation, FP)?.unwrap();
            assert_eq!(retry.preview, reviewed);
            assert!(
                core.actor_feature_commit("Bundle", &operation, prepared.plan_id.as_ref().unwrap())
                    .is_err()
            );
            assert!(!f.root.join("Bundle").exists());
            core.actor_feature_commit_with_losses(
                "Bundle",
                &operation,
                prepared.plan_id.as_ref().unwrap(),
                true,
            )?;
            Ok(())
        })
        .unwrap();
    assert!(!f.root.join("Bundle/Note/weftext.citations.bib").exists());
    assert_eq!(
        fs::read(
            f.root
                .join("Bundle/.weftext-import-originals/original.weftext")
        )
        .unwrap(),
        original
    );
}

struct Fixture {
    temp: tempfile::TempDir,
    core: Core,
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("Library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        Self {
            temp,
            core,
            root,
            data,
        }
    }
    fn bibliography(&mut self, operation: &str) -> ActorFeatureStatus {
        let request = OperationRequest { operation_id: fresh(), root: display(&self.root),
            intent: OperationIntent::ImportBibliography { parent: display(&self.root), name: "Papers".into(), format: "cslJson".into(),
                bytes: br#"[{"type":"article-journal","id":"same","title":"One"},{"type":"article-journal","id":"same","title":"Two"}]"#.to_vec(), allow_losses: false } };
        self.core
            .with_actor_for_root(ACTOR, &self.root, |core| {
                core.actor_feature_prepare_content_import(
                    "Papers",
                    operation,
                    FP,
                    request,
                    serde_json::json!({"kind":"bibliographyImport"}),
                    &[],
                    "Papers",
                )
            })
            .unwrap()
    }
    fn commit(&mut self, operation: &str, prepared: &ActorFeatureStatus) {
        self.core
            .with_actor_for_root(ACTOR, &self.root, |core| {
                core.actor_feature_commit_with_losses(
                    "Papers",
                    operation,
                    prepared.plan_id.as_ref().unwrap(),
                    true,
                )
            })
            .unwrap();
    }
    fn restart(&mut self) {
        let replacement = Core::new(self.temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, replacement));
        self.core = Core::new(&self.data).unwrap();
    }
}

#[test]
fn content_import_bibliography_original_multinode_receipt_and_metadata_listing_survive_restart() {
    let mut f = Fixture::new();
    let operation = fresh();
    let prepared = f.bibliography(&operation);
    assert_eq!(prepared.intent_kind, "importBibliography");
    assert_eq!(prepared.node_refs.len(), 2);
    assert!(
        f.core
            .content_import_slot_members_at(&f.root, &f.root.join("Papers"), "Papers")
            .unwrap()
            .is_empty()
    );
    f.commit(&operation, &prepared);
    f.restart();
    let receipt = f
        .core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            core.actor_feature_content_import_receipt("Papers", &operation)
        })
        .unwrap()
        .unwrap();
    assert_eq!(receipt.nodes.len(), 2);
    let members = f
        .core
        .content_import_slot_members_at(&f.root, &f.root.join("Papers"), "Papers")
        .unwrap();
    assert_eq!(members.len(), 2);
    for member in &members {
        let path = f.root.join("Papers").join(&member.relative_path);
        assert_eq!(
            f.core
                .qualified_content_import_node_at(&f.root, &path, "Papers", &member.owner.node_id)
                .unwrap()
                .owner,
            member.owner
        );
    }
    // Listing is only original immutable symbols: exclusive source locks cannot
    // force it to read body bytes or make the independent reader need creator ID.
    let locks = members
        .iter()
        .map(|member| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(f.root.join("Papers").join(&member.relative_path))
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        f.core
            .content_import_slot_members_at(&f.root, &f.root.join("Papers"), "Papers")
            .unwrap()
            .len(),
        2
    );
    drop(locks);
    let path = f.root.join("Papers").join(&members[0].relative_path);
    fs::rename(&path, f.temp.path().join("held-source.typ")).unwrap();
    assert_eq!(
        f.core
            .content_import_slot_members_at(&f.root, &f.root.join("Papers"), "Papers")
            .unwrap()
            .len(),
        2
    );
    assert!(
        f.core
            .qualified_content_import_node_at(&f.root, &path, "Papers", &members[0].owner.node_id)
            .is_err()
    );
}

#[test]
fn content_import_slot_rejects_local_producer_uncommitted_and_samebytes_replacement() {
    let mut f = Fixture::new();
    let ordinary = f.core.create_note(&f.root, &f.root, "Unrelated").unwrap();
    assert_eq!(
        f.core
            .content_import_slot_members_at(&f.root, &f.root.join("Unrelated"), "Unrelated")
            .unwrap_err()
            .code,
        "slotUnavailable"
    );
    assert!(
        f.core
            .qualified_content_import_node_at(
                &f.root,
                Path::new(&ordinary.path),
                "Unrelated",
                ordinary.node_id.as_ref().unwrap()
            )
            .is_err()
    );
    let operation = fresh();
    let prepared = f.bibliography(&operation);
    assert!(
        f.core
            .qualified_content_import_node_at(
                &f.root,
                &f.root.join("Papers/paper-0001/paper-0001.typ"),
                "Papers",
                &prepared.node_refs[0].node_id
            )
            .is_err()
    );
    f.commit(&operation, &prepared);
    let member = f
        .core
        .content_import_slot_members_at(&f.root, &f.root.join("Papers"), "Papers")
        .unwrap()
        .remove(0);
    let path = f.root.join("Papers").join(&member.relative_path);
    let bytes = fs::read(&path).unwrap();
    fs::rename(&path, f.temp.path().join("original.typ")).unwrap();
    fs::write(&path, bytes).unwrap();
    assert!(
        f.core
            .qualified_content_import_node_at(&f.root, &path, "Papers", &member.owner.node_id)
            .is_err()
    );
    assert!(
        f.core
            .qualified_content_import_node_at(&f.root, &path, "AnotherSlot", &member.owner.node_id)
            .is_err()
    );
}

#[test]
fn content_import_native_keeps_original_plan_identity_losses_and_future_slot_binding() {
    let mut f = Fixture::new();
    f.core.create_note(&f.root, &f.root, "Note").unwrap();
    let bytes = f.core.export_native_package(&f.root).unwrap();
    let operation = fresh();
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::ImportNative {
            parent: display(&f.root),
            name: "Bundle".into(),
            bytes,
            allow_losses: false,
        },
    };
    let prepared = f
        .core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            core.actor_feature_prepare_content_import(
                "Bundle",
                &operation,
                FP,
                request,
                serde_json::json!({"kind":"nativeImport"}),
                &[],
                "Bundle",
            )
        })
        .unwrap();
    assert_eq!(prepared.intent_kind, "importNative");
    let original_refs = prepared.node_refs.clone();
    f.restart();
    let retry = f
        .core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            core.actor_feature_retry("Bundle", &operation, FP)
        })
        .unwrap()
        .unwrap();
    assert_eq!(retry.plan_id, prepared.plan_id);
    assert_eq!(retry.node_refs, original_refs);
    f.core
        .with_actor_for_root(ACTOR, &f.root, |core| {
            core.actor_feature_commit_with_losses(
                "Bundle",
                &operation,
                prepared.plan_id.as_ref().unwrap(),
                true,
            )
        })
        .unwrap();
    let members = f
        .core
        .content_import_slot_members_at(&f.root, &f.root.join("Bundle"), "Bundle")
        .unwrap();
    assert_eq!(members.len(), original_refs.len());
    for member in members {
        assert_eq!(
            f.core
                .qualified_content_import_node_at(
                    &f.root,
                    &f.root.join("Bundle").join(&member.relative_path),
                    "Bundle",
                    &member.owner.node_id
                )
                .unwrap()
                .owner,
            member.owner
        );
    }
}
