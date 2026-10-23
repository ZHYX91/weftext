//! Static, field-only projections. No body evaluation or hidden field decode.
//! The caller owns authorization, admitted identity, currentness and persistence.
use super::*;
use crate::query_providers::SemanticDependencies;

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

/// Direct control metadata only. Never decode unknown authored field values or
/// compile a body to qualify the separately granted types/relation products.
pub fn narrow_query_metadata(
    source: &str,
    schema: &Schema,
) -> Result<(String, Vec<String>, Vec<String>)> {
    let control = declaration_selected(source, "weftext", Some(&BTreeSet::new()))?;
    let (id, types) = controls(&control)?;
    let (resolved, validation) = schema.expand_types(&types);
    if !validation.is_valid() {
        return Err(unavailable());
    }
    Ok((id, types, resolved.into_iter().collect()))
}

fn unavailable() -> DataError {
    DataError::unavailable(
        weftext_messages::UiMessage::error_data_narrow_narrow_field_profile_unavailable(),
    )
}

/// Only these Type validators have a complete, context-free Field closure.
/// Keep discovery and write preflight on the same rule, before reading values.
fn write_constraint_fields(schema: &Schema, constraint: &Constraint) -> Result<BTreeSet<String>> {
    if schema
        .validators
        .iter()
        .find(|v| v.id == constraint.validator)
        .is_none_or(|v| !matches!(v.operation.as_str(), "event" | "task-reminder"))
    {
        return Err(unavailable());
    }
    constraint
        .inputs
        .values()
        .map(|selector| match selector {
            Selector::Field { id, .. } => Ok(id.clone()),
            _ => Err(unavailable()),
        })
        .collect()
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
        result.extend(write_constraint_fields(schema, c)?);
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
            if !write_constraint_fields(schema, c)?.is_subset(field_ids) {
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
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_narrow_selected_field_constraint_rejected_proposal(),
            ));
        }
    }
    Ok(patch)
}
