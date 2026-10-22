//! Derived unique-child author resources. No member registry or directory scan.
use super::*;

pub(crate) fn split_member<'a>(config: &Config, resource: &'a str) -> Option<(&'a str, &'a str)> {
    let (slot, node) = resource.split_once('/')?;
    (config.structure_slots.get(slot).is_some_and(Slot::unique)
        && uuid::Uuid::parse_str(node).is_ok_and(|id| id.to_string() == node))
    .then_some((slot, node))
}
fn entries(
    inner: &HostedInner,
    slot: &str,
) -> Result<Vec<weftext_core::HostedUniqueMember>, ApiError> {
    let configured = inner
        .config
        .structure_slots
        .get(slot)
        .ok_or_else(unavailable)?;
    let NamePolicy::UniqueChildren {
        prefix,
        max_members,
    } = &configured.name_policy
    else {
        return Err(unavailable());
    };
    let parent = inner
        .config
        .directories
        .get(&configured.parent)
        .ok_or_else(unavailable)?;
    let path = inner.config.root.join(&parent.path);
    inner
        .core
        .qualified_query_scope_at(&inner.config.root, &path, &parent.reference, "live")
        .map_err(|_| unavailable())?;
    inner
        .core
        .hosted_unique_child_members_at(&inner.config.root, &path, slot, prefix, *max_members)
        .map_err(core_error)
}
pub(crate) fn member_file(
    inner: &HostedInner,
    resource: &str,
) -> Result<Option<FileEntry>, ApiError> {
    let Some((slot, node)) = split_member(&inner.config, resource) else {
        return Ok(None);
    };
    let selected = entries(inner, slot)?
        .into_iter()
        .find(|m| m.owner.node_id == node)
        .ok_or_else(unavailable)?;
    let configured = &inner.config.structure_slots[slot];
    let parent = &inner.config.directories[&configured.parent];
    let relative = Path::new(&selected.relative_path);
    if relative
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(changed());
    }
    Ok(Some(FileEntry {
        path: Path::new(&parent.path)
            .join(relative)
            .to_string_lossy()
            .into_owned(),
        title: relative
            .file_stem()
            .ok_or_else(changed)?
            .to_string_lossy()
            .into_owned(),
        lifecycle: "live".into(),
        expected_node: Some(selected.owner),
        editable: true,
        importable: false,
        inputs: Vec::new(),
    }))
}
pub(crate) fn member_rights(
    inner: &HostedInner,
    actor: &str,
    resource: &str,
) -> Option<super::super::Rights> {
    let (slot, _) = split_member(&inner.config, resource)?;
    inner
        .policy
        .users
        .get(actor)
        .filter(|user| user.enabled)?
        .structure
        .destinations
        .get(slot)
        .map(|rights| rights.members.clone())
}
pub(crate) fn slot_member_ids(inner: &HostedInner, slot: &str) -> Result<Vec<String>, ApiError> {
    Ok(entries(inner, slot)?
        .into_iter()
        .map(|m| format!("{slot}/{}", m.owner.node_id))
        .collect())
}
pub(crate) fn member_ids(inner: &HostedInner, actor: &str) -> Result<Vec<String>, ApiError> {
    let user = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?;
    let mut ids = Vec::new();
    for (slot, rights) in &user.structure.destinations {
        if rights.members == super::super::Rights::default() {
            continue;
        }
        ids.extend(slot_member_ids(inner, slot)?);
        if ids.len() > 5000 {
            return Err(ApiError::bad(weftext_messages::UiMessage::error_api_members_authorized_structure_members_exceed_the5000_member_budget()));
        }
    }
    Ok(ids)
}
pub(crate) fn qualify_member(inner: &HostedInner, resource: &str) -> Result<(), ApiError> {
    let Some((slot, node)) = split_member(&inner.config, resource) else {
        return Ok(());
    };
    let configured = &inner.config.structure_slots[slot];
    let NamePolicy::UniqueChildren {
        prefix,
        max_members,
    } = &configured.name_policy
    else {
        return Err(unavailable());
    };
    let parent = &inner.config.directories[&configured.parent];
    let path = inner.config.root.join(&parent.path);
    inner
        .core
        .qualified_query_scope_at(&inner.config.root, &path, &parent.reference, "live")
        .map_err(|_| unavailable())?;
    let q = inner
        .core
        .qualified_hosted_unique_child_at(
            &inner.config.root,
            &path,
            slot,
            prefix,
            *max_members,
            node,
        )
        .map_err(|_| unavailable())?;
    inner
        .core
        .validate_query_node_witness(&inner.config.root, &q.witness)
        .map_err(|_| unavailable())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn fixture(read: bool) -> (tempfile::TempDir, HostedState, Auth) {
        let (temp, state, auth) = super::super::tests::fixture();
        let mut inner = state.inner.lock().unwrap();
        inner.config.structure_slots.insert(
            "captures".into(),
            Slot {
                parent: "root".into(),
                name_policy: NamePolicy::UniqueChildren {
                    prefix: "fragment-".into(),
                    max_members: 2,
                },
                types: BTreeSet::new(),
            },
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .structure
            .destinations
            .insert(
                "captures".into(),
                DestinationRights {
                    create: true,
                    receive: false,
                    members: super::super::super::Rights {
                        read,
                        write: read,
                        history: read,
                        ..Default::default()
                    },
                },
            );
        assert!(valid_config(&inner.config));
        assert!(valid_rights(
            &inner.config,
            &inner.policy.users["alice"].structure
        ));
        drop(inner);
        (temp, state, auth)
    }
    fn request(operation: &str) -> Prepare {
        Prepare {
            resource: "captures".into(),
            operation_id: operation.into(),
            kind: "create".into(),
            source: None,
            destination: Some("captures".into()),
            trash_operation_id: None,
            template: None,
            parameters: BTreeMap::new(),
            inputs: Vec::new(),
            capture: Some(Capture {
                body: "An authored fragment".into(),
                ..Default::default()
            }),
        }
    }
    fn commit(inner: &mut HostedInner, auth: &Auth, request: Prepare) -> Value {
        let prepared = prepare_inner(inner, auth, request.clone()).unwrap();
        commit_inner(
            inner,
            auth,
            Commit {
                resource: request.resource,
                operation_id: request.operation_id,
                plan_id: prepared["planId"].as_str().unwrap().into(),
                accept_losses: false,
            },
        )
        .unwrap()
    }

    #[test]
    fn unique_creation_has_no_implicit_read_and_replay_keeps_one_original_result() {
        let (_temp, state, auth) = fixture(false);
        let mut inner = state.inner.lock().unwrap();
        let operation = uuid::Uuid::new_v4().to_string();
        let committed = commit(&mut inner, &auth, request(&operation));
        assert_eq!(committed["status"], "committed");
        assert_eq!(committed["result"]["canOpen"], false);
        let resource = committed["result"]["resource"].as_str().unwrap();
        assert!(split_member(&inner.config, resource).is_some());
        assert!(inner.open("alice", resource).is_err());
        let file = inner.file(resource).unwrap();
        let bytes = std::fs::read(inner.config.root.join(&file.path)).unwrap();
        let replay = prepare_inner(&mut inner, &auth, request(&operation)).unwrap();
        assert_eq!(replay["operationId"], operation);
        assert_eq!(replay["result"], committed["result"]);
        assert_eq!(slot_member_ids(&inner, "captures").unwrap().len(), 1);
        assert_eq!(
            std::fs::read(inner.config.root.join(file.path)).unwrap(),
            bytes
        );
    }

    #[test]
    fn explicit_member_rights_use_original_owner_and_cannot_expand_receive() {
        let (_temp, state, auth) = fixture(true);
        let mut inner = state.inner.lock().unwrap();
        let operation = uuid::Uuid::new_v4().to_string();
        let committed = commit(&mut inner, &auth, request(&operation));
        assert_eq!(committed["result"]["canOpen"], true);
        let resource = committed["result"]["resource"].as_str().unwrap();
        let opened = inner.open("alice", resource).unwrap();
        assert!(
            opened["source"]
                .as_str()
                .unwrap()
                .contains("An authored fragment")
        );
        assert_eq!(
            inner.file(resource).unwrap().expected_node,
            Some(serde_json::from_value(committed["result"]["nodeRef"].clone()).unwrap())
        );
        let mut copied = request(&uuid::Uuid::new_v4().to_string());
        copied.kind = "copy".into();
        copied.source = Some("Shared".into());
        copied.capture = None;
        assert!(authorize(&inner, "alice", &copied).is_err());
        let mut old = serde_json::to_value(&inner.config.structure_slots["captures"]).unwrap();
        old.as_object_mut().unwrap().remove("namePolicy");
        old["name"] = "legacy".into();
        assert!(serde_json::from_value::<Slot>(old).is_err());
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .structure
            .destinations
            .get_mut("captures")
            .unwrap()
            .receive = true;
        assert!(!valid_rights(
            &inner.config,
            &inner.policy.users["alice"].structure
        ));
    }
}
