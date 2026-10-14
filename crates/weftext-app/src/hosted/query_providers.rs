//! User-owned Query capabilities for exact installed provider projections.
//! Provider records never become row-resource grants or public data tables.
use super::*;
use weftext_data::{QueryProviderAccess, QueryProviderRequirements, Schema};

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    pub(super) providers: BTreeSet<String>,
    #[serde(default)]
    pub(super) series_policies: BTreeSet<String>,
}
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Selection {
    #[serde(default, deserialize_with = "unique_fingerprints")]
    pub(super) providers: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "unique_fingerprints")]
    pub(super) series_policies: BTreeMap<String, String>,
}
pub(super) fn unique_fingerprints<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Map;
    impl<'de> serde::de::Visitor<'de> for Map {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("unique provider IDs and exact server fingerprints")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some((id, fingerprint)) = map.next_entry::<String, String>()? {
                if values.insert(id, fingerprint).is_some() {
                    return Err(serde::de::Error::custom("Duplicate provider selection"));
                }
                if values.len() > 64 {
                    return Err(serde::de::Error::custom("Provider selection budget"));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(Map)
}
fn bounded_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control)
}
pub(super) fn valid_rights(rights: &Rights) -> bool {
    rights.providers.len() <= 64
        && rights.series_policies.len() <= 64
        && rights
            .providers
            .iter()
            .chain(&rights.series_policies)
            .all(|id| bounded_id(id))
}
pub(super) fn budget(selection: &Selection) -> Result<(), ApiError> {
    if selection.providers.len() > 64
        || selection.series_policies.len() > 64
        || selection
            .providers
            .iter()
            .chain(&selection.series_policies)
            .any(|(id, fp)| !bounded_id(id) || !valid_digest(fp))
    {
        return Err(ApiError::bad("请选择有界的 Provider ID 和准确服务端指纹"));
    }
    Ok(())
}
pub(super) fn authorize(
    inner: &HostedInner,
    actor: &str,
    selection: &Selection,
) -> Result<(), ApiError> {
    let rights = &inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?
        .query_providers;
    if !selection
        .providers
        .keys()
        .all(|id| rights.providers.contains(id))
        || !selection
            .series_policies
            .keys()
            .all(|id| rights.series_policies.contains(id))
    {
        return Err(unavailable());
    }
    Ok(())
}
pub(super) fn merge(target: &mut Selection, incoming: &Selection) -> Result<(), ApiError> {
    for (to, from) in [
        (&mut target.providers, &incoming.providers),
        (&mut target.series_policies, &incoming.series_policies),
    ] {
        for (id, fingerprint) in from {
            if to.get(id).is_some_and(|old| old != fingerprint) {
                return Err(query::changed());
            }
            to.insert(id.clone(), fingerprint.clone());
        }
    }
    budget(target)
}
/// Extend an already field-restricted projection with only independently
/// authorized static effects, or the explicit execution capability selection.
pub(super) fn project(
    inner: &HostedInner,
    actor: &str,
    original: &Schema,
    projection: &mut Schema,
    demand: &QueryProviderRequirements,
    explicit: Option<&Selection>,
) -> Result<Selection, ApiError> {
    let required = Selection {
        providers: demand.providers.clone(),
        series_policies: demand.series_policies.clone(),
    };
    let selected = explicit.unwrap_or(&required);
    authorize(inner, actor, selected)?;
    for (id, fingerprint) in &selected.providers {
        let provider = original
            .providers
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(query::changed)?;
        if weftext_data::provider_fingerprint(provider).map_err(|_| query::changed())?
            != *fingerprint
        {
            return Err(query::changed());
        }
    }
    for (id, fingerprint) in &selected.series_policies {
        let policy = original
            .series_policies
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(query::changed)?;
        if weftext_data::series_policy_fingerprint(policy).map_err(|_| query::changed())?
            != *fingerprint
        {
            return Err(query::changed());
        }
    }
    if explicit.is_some()
        && (required
            .providers
            .keys()
            .any(|id| !selected.providers.contains_key(id))
            || required
                .series_policies
                .keys()
                .any(|id| !selected.series_policies.contains_key(id)))
    {
        return Err(query::invalid());
    }
    if required
        .providers
        .iter()
        .any(|(id, fp)| selected.providers.get(id) != Some(fp))
        || required
            .series_policies
            .iter()
            .any(|(id, fp)| selected.series_policies.get(id) != Some(fp))
    {
        return Err(query::changed());
    }
    let access = QueryProviderAccess {
        providers: selected.providers.keys().cloned().collect(),
        series_policies: selected.series_policies.keys().cloned().collect(),
    };
    let permitted = weftext_data::narrow_query_provider_schema(original, &access, demand)
        .map_err(|_| unavailable())?;
    projection.providers = permitted.providers;
    projection.validators = permitted.validators;
    projection.series_policies = permitted.series_policies;
    projection.definitions = permitted.definitions;
    Ok(selected.clone())
}
pub(super) fn catalog(schema: &Schema, rights: &Rights) -> Result<Value, ApiError> {
    let mut providers = vec![];
    let mut series = vec![];
    let mut unavailable_providers = vec![];
    let mut unavailable_series = vec![];
    let mut dependency_unavailable = false;
    for id in &rights.providers {
        if let Some(provider) = schema.providers.iter().find(|p| &p.id == id) {
            let mut data = serde_json::Map::new();
            for key in ["kind", "key", "version"] {
                if let Some(value) = provider.data.get(key) {
                    data.insert(key.into(), value.clone());
                }
            }
            let query_names: &[&str] = if provider.implementation.id == "weftext.temporal.rules.v2"
            {
                &[
                    "containsDate",
                    "rangeOverlap",
                    "compareTemporal",
                    "nextCivilDay",
                    "rangeDuration",
                    "duringDuration",
                    "periodBounds",
                    "expandRecurrence",
                ]
            } else {
                &[]
            };
            providers.push(json!({"id":id,"implementation":{"id":provider.implementation.id,"abi":provider.implementation.abi},
                "data":data,"limits":{"maxSteps":provider.limits.max_steps,"maxBytes":provider.limits.max_bytes,"maxExpansion":provider.limits.max_expansion},
                "fingerprint":weftext_data::provider_fingerprint(provider).map_err(|_|query::changed())?,"queryNames":query_names}));
        } else {
            unavailable_providers.push(json!({"id":id,"status":"unavailable"}));
        }
    }
    for id in &rights.series_policies {
        if let Some(policy) = schema.series_policies.iter().find(|p| &p.id == id) {
            let calendar = rights
                .providers
                .contains(&policy.calendar)
                .then_some(&policy.calendar);
            let zone = rights
                .providers
                .contains(&policy.zone)
                .then_some(&policy.zone);
            dependency_unavailable |= calendar.is_none() || zone.is_none();
            series.push(json!({"id":id,"key":policy.key,"kinds":policy.kinds,"rule":policy.rule,"calendar":calendar,"zone":zone,
                "status":if calendar.is_some()&&zone.is_some(){"available"}else{"unavailable"},
                "fingerprint":weftext_data::series_policy_fingerprint(policy).map_err(|_|query::changed())?}));
        } else {
            unavailable_series.push(json!({"id":id,"status":"unavailable"}));
        }
    }
    Ok(
        json!({"providers":providers,"seriesPolicies":series,"unavailableProviders":unavailable_providers,"unavailableSeriesPolicies":unavailable_series,
        "providerCoverage":if unavailable_providers.is_empty() && unavailable_series.is_empty() && !dependency_unavailable{"complete"}else{"partial"}}),
    )
}
pub(super) fn unavailable_catalog(rights: &Rights) -> Value {
    json!({"providers":[],"seriesPolicies":[],"unavailableProviders":rights.providers.iter().map(|id|json!({"id":id,"status":"unavailable"})).collect::<Vec<_>>(),
        "unavailableSeriesPolicies":rights.series_policies.iter().map(|id|json!({"id":id,"status":"unavailable"})).collect::<Vec<_>>(),
        "providerCoverage":if rights.providers.is_empty()&&rights.series_policies.is_empty(){"complete"}else{"partial"}})
}
/// Demand discovery for execution must see exactly the selected registry too;
/// otherwise guessed hidden literal keys could affect its errors or closure.
pub(super) fn selected_schema(
    inner: &HostedInner,
    actor: &str,
    original: &Schema,
    selected: &Selection,
) -> Result<Schema, ApiError> {
    authorize(inner, actor, selected)?;
    let mut schema = original.clone();
    for (id, fingerprint) in &selected.providers {
        let p = original
            .providers
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(query::changed)?;
        if weftext_data::provider_fingerprint(p).map_err(|_| query::changed())? != *fingerprint {
            return Err(query::changed());
        }
    }
    for (id, fingerprint) in &selected.series_policies {
        let p = original
            .series_policies
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(query::changed)?;
        if weftext_data::series_policy_fingerprint(p).map_err(|_| query::changed())? != *fingerprint
        {
            return Err(query::changed());
        }
    }
    schema
        .providers
        .retain(|p| selected.providers.contains_key(&p.id));
    schema
        .series_policies
        .retain(|p| selected.series_policies.contains_key(&p.id));
    schema
        .validators
        .retain(|v| selected.providers.contains_key(&v.provider));
    Ok(schema)
}
pub(super) fn delivery_schema(schema: &Schema) -> Schema {
    let mut safe = schema.clone();
    // The XLSX structure sheet is public output. Executable data tables and
    // parameter-only use/validator definitions stay in the private frozen cut.
    // Authorized fields already have fully expanded native Shapes.
    safe.providers.clear();
    safe.validators.clear();
    safe.series_policies.clear();
    safe.definitions.clear();
    safe
}
fn shape_references(
    schema: &Schema,
    shape: &weftext_data::Shape,
    names: &mut BTreeSet<String>,
    remaining: &mut usize,
) -> Result<(), ApiError> {
    *remaining = remaining.checked_sub(1).ok_or_else(unavailable)?;
    match shape {
        weftext_data::Shape::Use { name } => {
            if names.insert(name.clone()) {
                shape_references(
                    schema,
                    schema.definitions.get(name).ok_or_else(unavailable)?,
                    names,
                    remaining,
                )?;
            }
        }
        weftext_data::Shape::Record { members, .. } => {
            for member in members.values() {
                shape_references(schema, &member.shape, names, remaining)?;
            }
        }
        weftext_data::Shape::List { items, .. } => {
            shape_references(schema, items, names, remaining)?
        }
        weftext_data::Shape::Union { variants, .. } => {
            for variant in variants {
                shape_references(schema, variant, names, remaining)?;
            }
        }
        _ => {}
    }
    Ok(())
}
/// Named Schema definitions are read capabilities too. Only an actual selected
/// authorized field closure or an unchanged public bundled contract supplies
/// them; an arbitrary author-chosen name does not disclose private definitions.
pub(super) fn check_shape_names(
    schema: &Schema,
    fields: &BTreeSet<String>,
    plan: &weftext_data::QueryPlan,
) -> Result<(), ApiError> {
    let public = Schema::bundled().map_err(|_| unavailable())?;
    let mut field_names = BTreeSet::new();
    let mut remaining = 20_000;
    for field in schema.fields.iter().filter(|f| fields.contains(&f.id)) {
        shape_references(schema, &field.shape, &mut field_names, &mut remaining)?;
    }
    fn visit(
        schema: &Schema,
        public: &Schema,
        field_names: &BTreeSet<String>,
        plan: &weftext_data::QueryPlan,
        remaining: &mut usize,
    ) -> Result<(), ApiError> {
        *remaining = remaining.checked_sub(1).ok_or_else(unavailable)?;
        if let Some(advanced) = &plan.advanced {
            for parameter in &advanced.parameters {
                let mut names = BTreeSet::new();
                shape_references(schema, &parameter.shape, &mut names, remaining)?;
                for name in names.difference(field_names) {
                    let own = schema.definitions.get(name).ok_or_else(unavailable)?;
                    let bundled = public.definitions.get(name).ok_or_else(unavailable)?;
                    if serde_json::to_value(own).map_err(|_| unavailable())?
                        != serde_json::to_value(bundled).map_err(|_| unavailable())?
                    {
                        return Err(unavailable());
                    }
                    let mut closure = BTreeSet::new();
                    shape_references(public, bundled, &mut closure, remaining)?;
                    for member in closure {
                        if serde_json::to_value(
                            schema.definitions.get(&member).ok_or_else(unavailable)?,
                        )
                        .map_err(|_| unavailable())?
                            != serde_json::to_value(
                                public.definitions.get(&member).ok_or_else(unavailable)?,
                            )
                            .map_err(|_| unavailable())?
                        {
                            return Err(unavailable());
                        }
                    }
                }
            }
            for child in &advanced.union_all {
                visit(schema, public, field_names, child, remaining)?;
            }
            if let Some(related) = &advanced.related {
                visit(schema, public, field_names, &related.from, remaining)?;
            }
        }
        Ok(())
    }
    visit(schema, &public, &field_names, plan, &mut remaining)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capabilities_default_closed_and_duplicate_or_invalid_selections_are_rejected() {
        let rights: Rights = serde_json::from_value(json!({})).unwrap();
        assert!(rights.providers.is_empty() && rights.series_policies.is_empty());
        let fingerprint = digest(b"exact provider");
        assert!(serde_json::from_str::<Selection>(&format!(r#"{{"providers":{{"temporal/rules":"{fingerprint}","temporal/rules":"{fingerprint}"}}}}"#)).is_err());
        let mut selected = Selection::default();
        selected
            .providers
            .insert("temporal/rules".into(), "not-a-fingerprint".into());
        assert!(budget(&selected).is_err());
    }
    #[test]
    fn provider_identity_cannot_merge_two_versions_under_one_selected_id() {
        let mut first = Selection {
            providers: BTreeMap::from([("calendar/private".into(), digest(b"v1"))]),
            ..Default::default()
        };
        let second = Selection {
            providers: BTreeMap::from([("calendar/private".into(), digest(b"v2"))]),
            ..Default::default()
        };
        assert_eq!(
            merge(&mut first, &second).unwrap_err().status,
            StatusCode::CONFLICT
        );
    }
    #[test]
    fn catalog_redacts_tables_and_ungranted_series_dependencies() {
        let schema = Schema::bundled().unwrap();
        let policy = &schema.series_policies[0];
        let rights = Rights {
            series_policies: BTreeSet::from([policy.id.clone()]),
            ..Default::default()
        };
        let output = catalog(&schema, &rights).unwrap();
        assert!(output["providers"].as_array().unwrap().is_empty());
        assert_eq!(output["providerCoverage"], "partial");
        assert!(output["seriesPolicies"][0]["calendar"].is_null());
        assert!(output["seriesPolicies"][0]["zone"].is_null());
        assert_eq!(output["seriesPolicies"][0]["status"], "unavailable");
        let providers_only = Rights {
            providers: BTreeSet::from([policy.calendar.clone(), policy.zone.clone()]),
            ..Default::default()
        };
        let output = catalog(&schema, &providers_only).unwrap();
        assert!(output["seriesPolicies"].as_array().unwrap().is_empty());
        for provider in output["providers"].as_array().unwrap() {
            assert!(provider["data"].get("years").is_none());
            assert!(provider["data"].get("transitions").is_none());
            assert!(provider["data"].get("epoch_date").is_none());
        }
    }
    #[test]
    fn output_schema_keeps_expanded_authorized_fields_without_private_registries() {
        let original = Schema::bundled().unwrap();
        let fields = BTreeSet::from(["tasks/status".into()]);
        let mut frozen = weftext_data::narrow_query_schema(&original, &fields).unwrap();
        frozen.providers = original.providers.clone();
        frozen.validators = original.validators.clone();
        frozen.series_policies = original.series_policies.clone();
        frozen.definitions = original.definitions.clone();
        let before = serde_json::to_value(&frozen.fields).unwrap();
        let safe = delivery_schema(&frozen);
        assert_eq!(serde_json::to_value(&safe.fields).unwrap(), before);
        assert!(
            safe.providers.is_empty()
                && safe.validators.is_empty()
                && safe.series_policies.is_empty()
                && safe.definitions.is_empty()
        );
        assert!(!frozen.providers.is_empty() && !frozen.definitions.is_empty());
    }
    #[test]
    fn named_shapes_require_selected_field_closure_or_exact_public_contract() {
        let mut schema = Schema::bundled().unwrap();
        schema.definitions.insert(
            "PrivateDecision".into(),
            weftext_data::Shape::Enum {
                values: vec!["ONLY_INTERNAL_CHOICE".into()],
                open: false,
            },
        );
        let plan = |name: &str| {
            serde_json::from_value(json!({"select":[{"name":"value","expr":"wanted"}],
            "advanced":{"parameters":[{"name":"wanted","shape":{"kind":"use","name":name}}]}}))
            .unwrap()
        };
        for name in ["PrivateDecision", "NeverInstalledDecision"] {
            assert_eq!(
                check_shape_names(&schema, &BTreeSet::new(), &plan(name))
                    .unwrap_err()
                    .status,
                StatusCode::NOT_FOUND
            );
        }
        let public_name = schema
            .definitions
            .keys()
            .find(|name| name.to_ascii_lowercase().contains("calendardate"))
            .unwrap()
            .clone();
        check_shape_names(&schema, &BTreeSet::new(), &plan(&public_name)).unwrap();
        schema.definitions.insert(
            public_name.clone(),
            weftext_data::Shape::Enum {
                values: vec!["private replacement".into()],
                open: false,
            },
        );
        assert!(check_shape_names(&schema, &BTreeSet::new(), &plan(&public_name)).is_err());
        let mut field = schema
            .fields
            .iter()
            .find(|f| f.id == "tasks/status")
            .unwrap()
            .clone();
        field.shape = weftext_data::Shape::Use {
            name: "PrivateDecision".into(),
        };
        schema.fields.retain(|f| f.id != field.id);
        schema.fields.push(field);
        check_shape_names(
            &schema,
            &BTreeSet::from(["tasks/status".into()]),
            &plan("PrivateDecision"),
        )
        .unwrap();
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn hidden_and_missing_named_parameter_shapes_are_identical_before_execution_even_empty_scope()
     {
        let (_temp, state, auth, doc, _version, _source) =
            super::super::history_api::test_fixture();
        {
            let mut inner = state.inner.lock().unwrap();
            let rights = inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .grants
                .get_mut("Shared")
                .unwrap();
            rights.fields.insert(
                "tasks/status".into(),
                FieldRights {
                    read: true,
                    write: false,
                },
            );
            rights.query.definitions = true;
            let mut schema = Schema::bundled().unwrap();
            schema.definitions.insert(
                "PrivateDecision".into(),
                weftext_data::Shape::Enum {
                    values: vec!["ONLY_INTERNAL_CHOICE".into()],
                    open: false,
                },
            );
            let root = inner.config.root.clone();
            let operation = uuid::Uuid::new_v4().to_string();
            let plan = inner
                .core
                .prepare_config_save(
                    &root,
                    weftext_core::ConfigRole::Schema,
                    None,
                    serde_json::to_vec(&schema).unwrap(),
                    &operation,
                )
                .unwrap();
            inner
                .core
                .execute_operation(&operation, &plan.plan_id)
                .unwrap();
        }
        for resources in [BTreeSet::<String>::new(), BTreeSet::from(["Shared".into()])] {
            let mut refusal = None;
            for name in ["PrivateDecision", "NeverInstalledDecision"] {
                let req: query::QueryRequest = serde_json::from_value(json!({"fields":["tasks/status"],"resources":resources,
                    "parameters":{"wanted":{"kind":"text","value":"ONLY_INTERNAL_CHOICE"}},
                    "plan":{"select":[{"name":"value","expr":"wanted"}],"advanced":{"parameters":[{"name":"wanted","shape":{"kind":"use","name":name}}]}}})).unwrap();
                let error = query::execute(&state, &auth, req).await.unwrap_err();
                assert_eq!(error.status, StatusCode::BAD_REQUEST);
                let current = (error.status, error.code, error.message);
                assert_eq!(refusal.get_or_insert(current.clone()), &current);
            }
        }
        let mut refusal = None;
        let mut inner = state.inner.lock().unwrap();
        for name in ["PrivateDecision", "NeverInstalledDecision"] {
            let plan = serde_json::from_value(json!({"select":[{"name":"value","expr":"wanted"}],"advanced":{"parameters":[{"name":"wanted","shape":{"kind":"use","name":name}}]}})).unwrap();
            let result = query::validate_definition(
                &mut inner,
                &auth,
                query::DefinitionValidationRequest {
                    resource: "Shared",
                    source: doc["source"].as_str().unwrap(),
                    plan: &plan,
                    binding: None,
                    draft_keys: &BTreeSet::new(),
                },
            );
            let error = match result {
                Ok(_) => panic!("A private parameter shape was admitted"),
                Err(error) => error,
            };
            assert_eq!(error.status, StatusCode::NOT_FOUND);
            let current = (error.status, error.code, error.message);
            assert_eq!(refusal.get_or_insert(current.clone()), &current);
        }
    }
}
