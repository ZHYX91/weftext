use super::*;

#[test]
fn definition_admission_uses_only_original_node_birth_source_after_clone_and_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("sender");
    let replica = temp.path().join("receiver");
    let data = temp.path().join("receiver-private");
    let mut sender = Core::new(temp.path().join("sender-private")).unwrap();
    sender.create_library(&root).unwrap();
    let note = sender.create_note(&root, &root, "Note").unwrap();
    let owner = note.node_id.unwrap();
    let schema = weftext_data::Schema::bundled().unwrap();
    let config = sender.read_config(&root, ConfigRole::Schema).unwrap();
    let saved = sender
        .prepare_config_save(
            &root,
            ConfigRole::Schema,
            config.bytes,
            serde_json::to_vec(&schema).unwrap(),
            &fresh(),
        )
        .unwrap();
    sender
        .execute_operation(&saved.operation_id, &saved.plan_id)
        .unwrap();
    let config = sender.read_config(&root, ConfigRole::Schema).unwrap();
    let binding = weftext_data::SavedQueryBinding {
        schema: config.role_ref,
        revision: config.revision.unwrap(),
        language: "weftext.query.cel.v1".into(),
    };
    let document = sender.open(&note.path, Some(root.clone())).unwrap();
    let basis = sender
        .read_snapshot(&root, std::slice::from_ref(&note.path))
        .unwrap()
        .files
        .remove(0);
    let operation = fresh();
    let key = sender
        .reserve_definition_key(&root, &note.path, &basis.base_id, &operation)
        .unwrap();
    assert_eq!(
        sender
            .pending_definition_operation(&root, Path::new(&note.path), &key)
            .unwrap(),
        Some(operation.clone())
    );
    let query = weftext_data::QueryPlan {
        select: vec![weftext_data::QueryColumn {
            name: "value".into(),
            expr: "1".into(),
        }],
        ..Default::default()
    };
    let source = weftext_data::upsert_saved_query_with_context(
        &document.source,
        None,
        &key,
        "weftext-query-original",
        &binding,
        &query,
        &schema,
        &weftext_data::FrozenAdvancedInput::default(),
    )
    .unwrap()
    .source;
    sender
        .save(SaveRequest {
            session_id: document.session_id,
            base_id: document.base_id,
            operation_id: operation,
            source: source.clone(),
        })
        .unwrap();
    assert_eq!(
        sender
            .pending_definition_operation(&root, Path::new(&note.path), &key)
            .unwrap(),
        None
    );
    let workspace_id = sender.require_workspace(&root).unwrap().workspace_id;
    let mut copied_definitions = Vec::new();
    for template in [false, true] {
        let name = if template { "FromTemplate" } else { "Copied" };
        let intent = if template {
            let snapshot = sender
                .read_snapshot(&root, std::slice::from_ref(&note.path))
                .unwrap();
            OperationIntent::TemplateNode {
                source: note.path.clone(),
                parent: display(&root),
                name: name.into(),
                instantiated_source: source.clone(),
                source_base_id: snapshot.files[0].base_id.clone(),
            }
        } else {
            OperationIntent::CopyNode {
                source: display(&root.join("Note")),
                parent: display(&root),
                name: name.into(),
                allow_losses: false,
            }
        };
        let copied = sender
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&root),
                intent,
            })
            .unwrap();
        sender
            .execute_operation(&copied.operation_id, &copied.plan_id)
            .unwrap();
        let copied_owner = weftext_knowledge::NodeRef {
            workspace_id: workspace_id.clone(),
            node_id: copied.node_map[&owner].clone(),
        };
        let copied_source =
            fs::read_to_string(root.join(name).join(format!("{name}.typ"))).unwrap();
        let definitions =
            weftext_knowledge::definition_slots(&copied_source, &copied_owner).unwrap();
        assert_eq!(definitions.len(), 1);
        let copied_key = definitions[0].key.clone();
        assert_ne!(copied_key, key);
        assert!(
            sender
                .is_definition_key_admitted(&root, &copied_owner.node_id, &copied_key)
                .unwrap()
        );
        copied_definitions.push((copied_owner.node_id, copied_key));
    }
    drop(sender);
    let mut sender = Core::new(temp.path().join("sender-private")).unwrap();
    for (copied_owner, copied_key) in copied_definitions {
        assert!(
            sender
                .is_definition_key_admitted(&root, &copied_owner, &copied_key)
                .unwrap()
        );
    }
    // A registered resource is preserved verbatim, even when its contents claim
    // the same Node metadata and contain an otherwise well-formed definition.
    let resource_key = fresh();
    let resource = source.replace(&key, &resource_key);
    let imported = sender
        .prepare_import_resource(
            &root,
            &note.path,
            "definition-resource.typ",
            resource.as_bytes().to_vec(),
            &fresh(),
        )
        .unwrap();
    sender
        .execute_operation(&imported.operation_id, &imported.plan_id)
        .unwrap();
    let identity = sender.enable_sync(&root, &fresh()).unwrap().identity;
    let objects = BTreeSet::from([sync::ObjectKey::Node(owner.clone()), sync::ObjectKey::Rules]);
    let mut receiver = Core::new(&data).unwrap();
    receiver
        .receive_sync(
            &identity,
            sender.export_sync_objects(&identity, &objects).unwrap(),
        )
        .unwrap();
    let cloned = receiver
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&replica),
            intent: OperationIntent::CloneSync { identity, objects },
        })
        .unwrap();
    receiver
        .execute_operation(&cloned.operation_id, &cloned.plan_id)
        .unwrap();
    for restart in [false, true] {
        if restart {
            drop(receiver);
            receiver = Core::new(&data).unwrap();
        }
        assert_eq!(
            fs::read_to_string(replica.join("Note/Note.typ")).unwrap(),
            source
        );
        assert_eq!(
            fs::read_to_string(replica.join("Note/definition-resource.typ")).unwrap(),
            resource
        );
        assert!(
            receiver
                .is_definition_key_admitted(&replica, &owner, &key)
                .unwrap()
        );
        assert!(
            !receiver
                .is_definition_key_admitted(&replica, &owner, &resource_key)
                .unwrap()
        );
        assert!(
            !receiver
                .is_definition_key_admitted(&replica, &fresh(), &key)
                .unwrap()
        );
    }
    let renamed = receiver
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&replica),
            intent: OperationIntent::RenameNode {
                source: display(&replica.join("Note")),
                name: "Renamed".into(),
            },
        })
        .unwrap();
    receiver
        .execute_operation(&renamed.operation_id, &renamed.plan_id)
        .unwrap();
    drop(receiver);
    let receiver = Core::new(&data).unwrap();
    assert!(!replica.join("Note").exists());
    assert!(
        receiver
            .is_definition_key_admitted(&replica, &owner, &key)
            .unwrap()
    );
    assert!(
        !receiver
            .is_definition_key_admitted(&replica, &owner, &resource_key)
            .unwrap()
    );
}
