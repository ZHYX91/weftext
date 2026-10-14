//! Static, field-only projections. No body evaluation or hidden field decode.
//! The caller owns authorization, admitted identity, currentness and persistence.
use super::*;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrowField {
    pub field_id: String,
    pub key: String,
    pub shape: Shape,
    pub nullable: bool,
    pub required: bool,
    pub cell: Cell,
    pub writable: bool,
}

pub struct NarrowProfile {
    pub node_id: String,
    pub fields: Vec<NarrowField>,
    /// Server-private semantic/preimage witness; never a client revision or ACL.
    pub witness: String,
    projection: NoteProjection,
    keys: BTreeSet<String>,
}

fn unavailable() -> DataError {
    DataError("Narrow field profile unavailable".into())
}

/// Determines write dependencies from control/schema syntax, before reading
/// author field values. The caller must authorize every returned FieldId.
pub fn narrow_write_fields(
    source: &str,
    schema: &Schema,
    field_id: &str,
) -> Result<BTreeSet<String>> {
    let control = declaration_selected(source, "weftext", Some(&BTreeSet::new()))?;
    let (_, types) = controls(&control)?;
    let (resolved, validation) = schema.expand_types(&types);
    if !validation.is_valid()
        || schema
            .relations
            .iter()
            .any(|r| r.field == field_id && resolved.contains(&r.source_type))
    {
        return Err(unavailable());
    }
    let mut result = BTreeSet::from([field_id.to_string()]);
    for c in schema
        .types
        .iter()
        .filter(|t| resolved.contains(&t.id))
        .flat_map(|t| &t.constraints)
        .filter(|c| {
            c.inputs
                .values()
                .any(|s| matches!(s, Selector::Field { id, .. } if id == field_id))
        })
    {
        if schema
            .validators
            .iter()
            .find(|v| v.id == c.validator)
            .is_none_or(|v| v.operation != "event")
        {
            return Err(unavailable());
        }
        for selector in c.inputs.values() {
            let Selector::Field { id, .. } = selector else {
                return Err(unavailable());
            };
            result.insert(id.clone());
        }
    }
    Ok(result)
}

/// Restrict the Query symbol/shape universe before CEL type checking. Removed
/// fields are neither readable cells nor discoverable schema diagnostics.
pub fn narrow_query_schema(schema: &Schema, field_ids: &BTreeSet<String>) -> Result<Schema> {
    if field_ids.is_empty() || field_ids.len() > 64 {
        return Err(unavailable());
    }
    let mut remaining = 20_000;
    let mut result = schema.clone();
    result.fields = field_ids
        .iter()
        .map(|id| {
            let mut field = schema
                .fields
                .iter()
                .find(|f| &f.id == id)
                .cloned()
                .ok_or_else(unavailable)?;
            field.shape = expanded_shape(schema, &field.shape, 0, &mut remaining)?;
            field.default = None;
            Ok(field)
        })
        .collect::<Result<_>>()?;
    result.definitions.clear();
    result.types.clear();
    result.aliases.clear();
    result.relations.clear();
    result.series_policies.clear();
    // A field grant does not grant the ambient calendar/zone/unit registry.
    // Provider-backed CEL calls require a separately authorized producer.
    result.validators.clear();
    result.providers.clear();
    Ok(result)
}

fn expanded_shape(
    schema: &Schema,
    shape: &Shape,
    depth: usize,
    remaining: &mut usize,
) -> Result<Shape> {
    if depth > 64 || *remaining == 0 {
        return Err(unavailable());
    }
    *remaining -= 1;
    Ok(match shape {
        Shape::Use { name } => expanded_shape(
            schema,
            schema.definitions.get(name).ok_or_else(unavailable)?,
            depth + 1,
            remaining,
        )?,
        Shape::List { items, min, max } => Shape::List {
            items: Box::new(expanded_shape(schema, items, depth + 1, remaining)?),
            min: *min,
            max: *max,
        },
        Shape::Record {
            members,
            unknown,
            constraints,
        } => {
            // Nested validators may only inspect the selected native value.
            // Resource/other-field/context slots need a separate authorized producer.
            if constraints.iter().any(|c| {
                c.inputs
                    .values()
                    .any(|s| !matches!(s, Selector::Local { .. }))
            }) {
                return Err(unavailable());
            }
            let members = members
                .iter()
                .map(|(key, member)| {
                    Ok((
                        key.clone(),
                        Member {
                            shape: expanded_shape(schema, &member.shape, depth + 1, remaining)?,
                            required: member.required,
                            nullable: member.nullable,
                        },
                    ))
                })
                .collect::<Result<_>>()?;
            Shape::Record {
                members,
                unknown: unknown.clone(),
                constraints: constraints.clone(),
            }
        }
        Shape::Union {
            variants,
            discriminator,
        } => Shape::Union {
            variants: variants
                .iter()
                .map(|s| expanded_shape(schema, s, depth + 1, remaining))
                .collect::<Result<_>>()?,
            discriminator: discriminator.clone(),
        },
        other => other.clone(),
    })
}

#[derive(Default, Serialize)]
struct SemanticDependencies {
    validators: BTreeMap<String, Validator>,
    providers: BTreeMap<String, Provider>,
    series_policies: BTreeMap<String, SeriesPolicy>,
    exact_provider_selections: BTreeMap<String, Option<String>>,
    series_selections: BTreeMap<String, Option<String>>,
}

impl SemanticDependencies {
    fn provider(&mut self, schema: &Schema, id: &str) -> Result<()> {
        let provider = schema
            .providers
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(unavailable)?;
        self.providers.insert(id.into(), provider.clone());
        Ok(())
    }

    fn exact_provider(&mut self, schema: &Schema, key: &str, version: &str, family: &str) {
        let provider = schema.providers.iter().find(|p| {
            let kind = p.data["kind"].as_str();
            (if family == "calendar" {
                matches!(kind, Some("gregorian" | "calendar-table"))
            } else {
                kind == Some("zone-table")
            }) && p.data["key"].as_str() == Some(key)
                && p.data["version"].as_str() == Some(version)
        });
        // Retain absence as a dependency too. This matches the executors'
        // exact family/key/version lookup and installed uniqueness rule.
        self.exact_provider_selections.insert(
            serde_json::to_string(&[family, key, version]).unwrap(),
            provider.map(|p| p.id.clone()),
        );
        if let Some(provider) = provider {
            self.providers.insert(provider.id.clone(), provider.clone());
        }
    }

    fn endpoint(&mut self, schema: &Schema, value: &NativeValue) {
        if let (Some(key), Some(version)) = (
            value.member("calendar").and_then(NativeValue::text),
            value.member("version").and_then(NativeValue::text),
        ) {
            self.exact_provider(schema, key, version, "calendar");
        }
    }

    fn recurrence(&mut self, schema: &Schema, value: &NativeValue) {
        for (family, version_key) in [("calendar", "calendar_version"), ("zone", "zone_version")] {
            if let (Some(key), Some(version)) = (
                value.member(family).and_then(NativeValue::text),
                value.member(version_key).and_then(NativeValue::text),
            ) {
                self.exact_provider(schema, key, version, family);
            }
        }
    }

    fn constraint(
        &mut self,
        schema: &Schema,
        constraint: &Constraint,
        local: Option<&NativeValue>,
        fields: Option<&BTreeMap<String, NativeValue>>,
    ) -> Result<()> {
        let validator = schema
            .validators
            .iter()
            .find(|v| v.id == constraint.validator)
            .ok_or_else(unavailable)?;
        self.validators
            .insert(validator.id.clone(), validator.clone());
        self.provider(schema, &validator.provider)?;
        let slot = |name: &str| -> Vec<&NativeValue> {
            match constraint.inputs.get(name) {
                Some(Selector::Local { path }) => local
                    .and_then(|v| crate::schema::select_one(v, path))
                    .into_iter()
                    .collect(),
                Some(Selector::Field { id, path }) => {
                    let value = fields.and_then(|fields| fields.get(id));
                    let mut values = vec![];
                    if let Some(value) = value {
                        select_values(value, path, &mut values);
                    }
                    values
                }
                _ => vec![],
            }
        };
        let name = if validator.operation == "event" {
            "recurrence"
        } else {
            "value"
        };
        for value in slot(name) {
            match validator.operation.as_str() {
                "calendar-date" => self.endpoint(schema, value),
                "range" | "during" => {
                    let end = if validator.operation == "during" {
                        "end"
                    } else {
                        "end_exclusive"
                    };
                    for key in ["start", end] {
                        if let Some(value) = value.member(key) {
                            self.endpoint(schema, value);
                        }
                    }
                }
                "recurrence" | "event" => self.recurrence(schema, value),
                "period" => {
                    if let Some(key) = value.member("series_key").and_then(NativeValue::text) {
                        let policy = schema.series_policies.iter().find(|p| p.key == key);
                        self.series_selections
                            .insert(key.into(), policy.map(|p| p.id.clone()));
                        let Some(policy) = policy else {
                            continue;
                        };
                        self.series_policies
                            .insert(policy.id.clone(), policy.clone());
                        self.provider(schema, &policy.calendar)?;
                        self.provider(schema, &policy.zone)?;
                    }
                }
                // These registered executors consume only their native input
                // and the directly named validator provider (units use its
                // table). Unknown future executors must declare a new closure.
                "int-range" | "decimal-range" | "custom-code" | "month-day" | "quantity"
                | "measurement" | "exception" => {}
                _ => return Err(unavailable()),
            }
        }
        Ok(())
    }

    fn shape(
        &mut self,
        schema: &Schema,
        shape: &Shape,
        value: Option<&NativeValue>,
        remaining: &mut usize,
    ) -> Result<()> {
        if *remaining == 0 {
            return Err(unavailable());
        }
        *remaining -= 1;
        match shape {
            Shape::Record {
                members,
                constraints,
                ..
            } => {
                for constraint in constraints {
                    self.constraint(schema, constraint, value, None)?;
                }
                for (key, member) in members {
                    self.shape(
                        schema,
                        &member.shape,
                        value.and_then(|v| v.member(key)),
                        remaining,
                    )?;
                }
            }
            Shape::List { items, .. } => {
                if let Some(values) = value.and_then(NativeValue::array) {
                    for value in values {
                        self.shape(schema, items, Some(value), remaining)?;
                    }
                } else {
                    self.shape(schema, items, None, remaining)?;
                }
            }
            Shape::Union {
                variants,
                discriminator,
            } => {
                if let Some(variant) = value.and_then(|v| {
                    schema
                        .union_variant(variants, discriminator.as_ref(), v)
                        .ok()
                }) {
                    self.shape(schema, variant, value, remaining)?;
                } else {
                    for variant in variants {
                        self.shape(schema, variant, None, remaining)?;
                    }
                }
            }
            Shape::Use { .. } => return Err(unavailable()),
            _ => {}
        }
        Ok(())
    }
}

/// Certifies keys/control bindings first, then decodes only the authorized
/// field closure. A write requires every affected Type-constraint input before
/// any selected value is read. `field_ids` must contain canonical FieldIds.
pub fn narrow_profile(
    source: &str,
    schema: &Schema,
    field_ids: &BTreeSet<String>,
    write: Option<&str>,
) -> Result<NarrowProfile> {
    if field_ids.is_empty() || field_ids.len() > 64 {
        return Err(unavailable());
    }
    let control = declaration_selected(source, "weftext", Some(&BTreeSet::new()))
        .map_err(|_| unavailable())?;
    let (id, types) = controls(&control).map_err(|_| unavailable())?;
    let (resolved, validation) = schema.expand_types(&types);
    if !validation.is_valid() {
        return Err(unavailable());
    }
    let applicable = schema.applicable_fields(&resolved);
    let mut keys = BTreeSet::new();
    let mut definitions = BTreeMap::new();
    let mut remaining = 20_000;
    for field_id in field_ids {
        let field = schema
            .fields
            .iter()
            .find(|f| &f.id == field_id)
            .ok_or_else(unavailable)?;
        if applicable.iter().any(|(f, _)| &f.id == field_id) {
            keys.insert(field.key.clone());
        }
        definitions.insert(
            field.id.clone(),
            expanded_shape(schema, &field.shape, 0, &mut remaining)?,
        );
    }
    let mut constraints = vec![];
    if let Some(selected) = write {
        if !field_ids.contains(selected)
            // Relations need separately authorized target/graph evidence.
            || schema.relations.iter().any(|r| r.field == selected && resolved.contains(&r.source_type))
        {
            return Err(unavailable());
        }
        for c in schema
            .types
            .iter()
            .filter(|t| resolved.contains(&t.id))
            .flat_map(|t| &t.constraints)
            .filter(|c| {
                c.inputs
                    .values()
                    .any(|s| matches!(s, Selector::Field { id, .. } if id == selected))
            })
        {
            if c.inputs
                .values()
                .any(|s| !matches!(s, Selector::Field { id, .. } if field_ids.contains(id)))
                || schema
                    .validators
                    .iter()
                    .find(|v| v.id == c.validator)
                    .is_none_or(|v| v.operation != "event")
            {
                return Err(unavailable());
            }
            constraints.push(c);
        }
    }
    // Retain only authorized static native inputs privately, including values
    // whose provider validation is unavailable. Those values still identify
    // negative calendar/zone/series dependencies; Cells intentionally omit them.
    let declaration =
        declaration_selected(source, "weftext", Some(&keys)).map_err(|_| unavailable())?;
    let native_values = applicable
        .iter()
        .filter(|(f, _)| field_ids.contains(&f.id))
        .filter_map(|(f, _)| {
            root_field(&declaration, &f.key)
                .and_then(|n| n.value.clone())
                .map(|v| (f.id.clone(), v))
        })
        .collect::<BTreeMap<_, _>>();
    let mut projection = project_selected(source, schema, Some(&keys));
    if projection.qualification != "qualified" || !projection.diagnostics.is_empty() {
        return Err(unavailable());
    }
    projection.unknown_fields.clear();
    let mut fields = projection
        .fields
        .iter_mut()
        .map(|p| {
            p.raw = None;
            p.source_range = None;
            let field = schema
                .fields
                .iter()
                .find(|f| f.id == p.field_id)
                .ok_or_else(unavailable)?;
            Ok(NarrowField {
                field_id: p.field_id.clone(),
                key: p.key.clone(),
                shape: definitions.remove(&p.field_id).ok_or_else(unavailable)?,
                nullable: field.nullable,
                required: p.required,
                cell: p.cell.clone(),
                writable: p.writable,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    // A requested field outside the qualified type's contribution is absent,
    // not a value borrowed from an unknown authored member with the same key.
    // This lets Query keep the authorized row and its actual Cell semantics.
    for (field_id, shape) in definitions {
        let field = schema
            .fields
            .iter()
            .find(|f| f.id == field_id)
            .ok_or_else(unavailable)?;
        fields.push(NarrowField {
            field_id,
            key: field.key.clone(),
            shape,
            nullable: field.nullable,
            required: false,
            cell: Cell::Absent,
            writable: false,
        });
    }
    fields.sort_by(|a, b| a.field_id.cmp(&b.field_id));
    // Only selected Shape constraints and affected Type constraints contribute
    // registry dependencies. Dynamic calendar/zone/series selections come from
    // these already authorized native inputs, never from an ambient registry.
    let mut dependencies = SemanticDependencies::default();
    let mut remaining = 200_000;
    for field in &fields {
        dependencies.shape(
            schema,
            &field.shape,
            native_values.get(&field.field_id),
            &mut remaining,
        )?;
    }
    for constraint in &constraints {
        dependencies.constraint(schema, constraint, None, Some(&native_values))?;
    }
    let witness = weftext_engine::source_hash(&serde_json::to_string(&serde_json::json!({
        "id":id,"types":types,"fields":fields,"constraints":constraints,
        "dependencies":dependencies,
    }))?);
    Ok(NarrowProfile {
        node_id: id,
        fields,
        witness,
        projection,
        keys,
    })
}

/// Writes a selected native value into *current* source after the caller has
/// compared its private witness. Never applies a client-supplied byte range.
pub fn patch_narrow_field(
    source: &str,
    schema: &Schema,
    field_ids: &BTreeSet<String>,
    field_id: &str,
    replacement: &NativeValue,
) -> Result<FieldPatch> {
    let before = narrow_profile(source, schema, field_ids, Some(field_id))?;
    let patch = patch_field_selected(
        source,
        schema,
        field_id,
        &[],
        replacement,
        Some(&before.keys),
    )?;
    let after = narrow_profile(&patch.source, schema, field_ids, Some(field_id))?;
    for constraint in schema
        .types
        .iter()
        .filter(|t| after.projection.resolved_types.contains(&t.id))
        .flat_map(|t| &t.constraints)
        .filter(|c| {
            c.inputs
                .values()
                .any(|s| matches!(s, Selector::Field { id, .. } if id == field_id))
        })
    {
        if !validate_type_constraint(
            constraint,
            &after.projection,
            schema,
            &ValidationContext::default(),
        )
        .is_valid()
        {
            return Err(DataError(
                "Selected field constraint rejected the proposal".into(),
            ));
        }
    }
    Ok(patch)
}
