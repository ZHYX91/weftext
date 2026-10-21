use super::*;

const ALICE: &str = "deployment:alice:navigation";
const BOB: &str = "deployment:bob:navigation";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    state: PathBuf,
    core: Core,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let state = temp.path().join("state");
        let mut core = Core::new(&state).unwrap();
        core.create_library(&root).unwrap();
        Self {
            _temp: temp,
            root,
            state,
            core,
        }
    }
    fn prepare(
        &mut self,
        actor: &str,
        operation: &str,
        intent: HostedNavigationIntent,
    ) -> ActorFeatureStatus {
        self.core
            .with_actor(actor, |c| {
                c.actor_feature_prepare_navigation(
                    "navigation",
                    operation,
                    FP,
                    &self.root,
                    intent,
                    serde_json::json!({"summary":"original navigation"}),
                )
            })
            .unwrap()
    }
    fn commit(
        &mut self,
        actor: &str,
        operation: &str,
        status: &ActorFeatureStatus,
    ) -> ActorFeatureStatus {
        self.core
            .with_actor(actor, |c| {
                c.actor_feature_commit("navigation", operation, status.plan_id.as_deref().unwrap())
            })
            .unwrap()
    }
    fn restart(&mut self) {
        let placeholder = Core::new(self._temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, placeholder));
        self.core = Core::new(&self.state).unwrap();
    }
}

#[test]
fn navigation_directory_original_allocation_survives_retry_restart_and_commit() {
    let mut f = Fixture::new();
    let path = f.root.join("Container");
    fs::create_dir(&path).unwrap();
    let op = fresh();
    let intent = HostedNavigationIntent::AdmitDirectory {
        path: display(&path),
    };
    let prepared = f.prepare(ALICE, &op, intent.clone());
    assert_eq!(prepared.intent_kind, "admitDirectory");
    let reference = prepared.navigation_ref.clone().unwrap();
    assert!(!path.join("weftext.directory.json").exists());
    f.restart();
    let retry = f.prepare(ALICE, &op, intent.clone());
    assert_eq!(retry.plan_id, prepared.plan_id);
    assert_eq!(retry.navigation_ref.as_deref(), Some(reference.as_str()));
    let committed = f.commit(ALICE, &op, &retry);
    assert_eq!(committed.status, "committed");
    f.restart();
    assert_eq!(
        f.prepare(ALICE, &op, intent).navigation_ref.as_deref(),
        Some(reference.as_str())
    );
    let actual = f
        .core
        .resolve_navigation_target(&f.root, &reference)
        .unwrap();
    assert_eq!(actual.status, "managed");
    assert_eq!(actual.path.as_deref(), Some(display(&path).as_str()));
    assert!(
        f.core
            .with_actor(BOB, |c| c.actor_feature_status("navigation", &op))
            .unwrap()
            .is_none()
    );
}

#[test]
fn navigation_tag_same_names_and_actor_ids_remain_independent_and_rename_is_original() {
    let mut f = Fixture::new();
    let op = fresh();
    let basis = f.core.read_tag_registry(&f.root).unwrap();
    let first = f.prepare(
        ALICE,
        &op,
        HostedNavigationIntent::CreateTag {
            parent_ref: None,
            assignment: None,
            display_name: "同名标签".into(),
            base_id: basis.base_id,
        },
    );
    f.commit(ALICE, &op, &first);
    let basis = f.core.read_tag_registry(&f.root).unwrap();
    let second = f.prepare(
        BOB,
        &op,
        HostedNavigationIntent::CreateTag {
            parent_ref: None,
            assignment: None,
            display_name: "同名标签".into(),
            base_id: basis.base_id,
        },
    );
    f.commit(BOB, &op, &second);
    assert_ne!(first.navigation_ref, second.navigation_ref);
    let basis = f.core.read_tag_registry(&f.root).unwrap();
    assert_eq!(basis.tags.len(), 2);
    let rename = fresh();
    let intent = HostedNavigationIntent::RenameTag {
        reference: first.navigation_ref.clone().unwrap(),
        display_name: "改名标签".into(),
        base_id: basis.base_id,
    };
    let prepared = f.prepare(ALICE, &rename, intent.clone());
    f.restart();
    let retry = f.prepare(ALICE, &rename, intent);
    assert_eq!(retry.plan_id, prepared.plan_id);
    f.commit(ALICE, &rename, &retry);
    let tags = f.core.read_tag_registry(&f.root).unwrap();
    assert_eq!(
        tags.tags
            .iter()
            .find(|t| Some(&t.tag_ref) == first.navigation_ref.as_ref())
            .unwrap()
            .display_name,
        "改名标签"
    );
    assert_eq!(
        tags.tags
            .iter()
            .find(|t| Some(&t.tag_ref) == second.navigation_ref.as_ref())
            .unwrap()
            .display_name,
        "同名标签"
    );
}

#[test]
fn navigation_unknown_settlement_fences_late_prepare_and_different_intent_conflicts() {
    let mut f = Fixture::new();
    let path = f.root.join("Container");
    fs::create_dir(&path).unwrap();
    let unknown = fresh();
    f.core
        .with_actor(ALICE, |c| c.actor_feature_settle("navigation", &unknown))
        .unwrap();
    f.restart();
    let intent = HostedNavigationIntent::AdmitDirectory {
        path: display(&path),
    };
    assert_eq!(
        f.prepare(ALICE, &unknown, intent.clone()).status,
        "cancelled"
    );
    assert!(!path.join("weftext.directory.json").exists());
    let op = fresh();
    let original = f.prepare(ALICE, &op, intent);
    let error = f
        .core
        .with_actor(ALICE, |c| {
            c.actor_feature_prepare_navigation(
                "navigation",
                &op,
                FP,
                &f.root,
                HostedNavigationIntent::CreateTag {
                    parent_ref: None,
                    assignment: None,
                    display_name: "different".into(),
                    base_id: fresh(),
                },
                serde_json::json!({}),
            )
        })
        .unwrap_err();
    assert_eq!(error.code, "operationConflict");
    let error = f
        .core
        .with_actor(ALICE, |c| {
            c.actor_feature_prepare(
                "navigation",
                &op,
                FP,
                OperationRequest {
                    operation_id: op.clone(),
                    root: display(&f.root),
                    intent: OperationIntent::NewNode {
                        parent: display(&f.root),
                        name: "Other".into(),
                        source: None,
                    },
                },
                serde_json::json!({}),
                &[],
            )
        })
        .unwrap_err();
    assert_eq!(error.code, "operationConflict");
    assert_eq!(f.commit(ALICE, &op, &original).status, "committed");
}

#[test]
fn navigation_failed_prepare_keeps_original_actor_decision_without_retry_allocation() {
    let mut f = Fixture::new();
    let op = fresh();
    let intent = HostedNavigationIntent::CreateTag {
        parent_ref: None,
        assignment: None,
        display_name: String::new(),
        base_id: fresh(),
    };
    assert!(
        f.core
            .with_actor(ALICE, |c| c.actor_feature_prepare_navigation(
                "navigation",
                &op,
                FP,
                &f.root,
                intent.clone(),
                serde_json::json!({})
            ))
            .is_err()
    );
    f.restart();
    let retry = f.prepare(ALICE, &op, intent);
    assert_eq!(retry.status, "notCommitted");
    assert!(retry.navigation_ref.is_none());
    assert!(f.core.read_tag_registry(&f.root).unwrap().tags.is_empty());
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_feature_settle("navigation", &op))
            .unwrap()
            .status,
        "cancelled"
    );
}

#[test]
fn navigation_replaced_directory_cannot_inherit_original_reservation() {
    let mut f = Fixture::new();
    let path = f.root.join("Container");
    fs::create_dir(&path).unwrap();
    let op = fresh();
    let prepared = f.prepare(
        ALICE,
        &op,
        HostedNavigationIntent::AdmitDirectory {
            path: display(&path),
        },
    );
    let original = f.root.join("MovedOriginal");
    fs::rename(&path, &original).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(
        f.core
            .with_actor(ALICE, |c| c.actor_feature_commit(
                "navigation",
                &op,
                prepared.plan_id.as_deref().unwrap()
            ))
            .is_err()
    );
    assert!(!path.join("weftext.directory.json").exists());
    assert!(!original.join("weftext.directory.json").exists());
    let status = f
        .core
        .with_actor(ALICE, |c| c.actor_feature_status("navigation", &op))
        .unwrap()
        .unwrap();
    assert_eq!(status.navigation_ref, prepared.navigation_ref);
}

#[test]
fn retained_pre_hierarchy_navigation_records_keep_the_original_author_fingerprint() {
    let root = "D:/owned/library";
    let base = fresh();
    let old = serde_json::json!({"root":root,"intent":{"kind":"createTag","displayName":"original","baseId":base}});
    let legacy: NavigationFeature = serde_json::from_value(old).unwrap();
    assert_eq!(
        legacy.canonical(),
        serde_json::json!({"kind":"createTag","root":root,"displayName":"original","baseId":base})
    );
    let mut current = legacy.clone();
    current.original_author_intent = Some(current.current_canonical());
    assert_ne!(current.canonical(), legacy.canonical());
    let serialized = serde_json::to_vec(&current).unwrap();
    assert_eq!(
        serde_json::from_slice::<NavigationFeature>(&serialized)
            .unwrap()
            .canonical(),
        current.canonical()
    );
}
