//! One pure source proposal for transferring exact list occurrences. Persistence,
//! current authorization and contextual Type/relation validation belong to App/Core.
use super::*;
use typst_syntax::SyntaxKind;

struct ListField<'a> {
    field: &'a Field,
    values: &'a [NativeValue],
}

/// Transfer an inclusive source ordinal range into a different existing list.
/// `to` is an insertion boundary in the unchanged destination list (0..=len).
/// The returned single patch covers both author expressions and retains every
/// byte between them. Callers must validate the whole proposal against their
/// frozen Type/relation context before applying it once to the original Draft.
pub fn patch_list_transfer(
    source: &str,
    schema: &Schema,
    from_field: &str,
    to_field: &str,
    start: usize,
    end: usize,
    to: usize,
) -> Result<FieldPatch> {
    let projection = project(source, schema);
    if projection.qualification != "qualified" || !projection.diagnostics.is_empty() {
        return Err(DataError::new(
            UiMessage::error_data_projection_control_types_unqualified_or_conflicting(),
        ));
    }
    let from = transfer_field(&projection, schema, from_field)?;
    let into = transfer_field(&projection, schema, to_field)?;
    if from.field.id == into.field.id {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_different_fields_required(),
        ));
    }
    let Shape::List {
        items: from_items, ..
    } = schema.resolve(&from.field.shape)
    else {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_existing_lists_required(),
        ));
    };
    let Shape::List {
        items: into_items, ..
    } = schema.resolve(&into.field.shape)
    else {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_existing_lists_required(),
        ));
    };
    // A valid value in a broader target domain is not permission to reinterpret
    // its semantic type. No text conversion or first-match union is introduced.
    if serde_json::to_value(schema.resolve(from_items))?
        != serde_json::to_value(schema.resolve(into_items))?
    {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_item_shapes_differ(),
        ));
    }
    if start > end || end >= from.values.len() || to > into.values.len() {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_ordinal_invalid(),
        ));
    }
    let declaration = declaration(source, "weftext")?;
    let from_node = root_field(&declaration, &from.field.key).ok_or_else(|| {
        DataError::new(UiMessage::error_data_field_transfer_existing_lists_required())
    })?;
    let into_node = root_field(&declaration, &into.field.key).ok_or_else(|| {
        DataError::new(UiMessage::error_data_field_transfer_existing_lists_required())
    })?;
    let from_items_raw = exact_items(source, from_node, from.values.len())?;
    let into_items_raw = exact_items(source, into_node, into.values.len())?;
    let count = end - start + 1;
    let mut from_values = Vec::with_capacity(from.values.len() - count);
    from_values.extend_from_slice(&from.values[..start]);
    from_values.extend_from_slice(&from.values[end + 1..]);
    let mut into_values = Vec::with_capacity(into.values.len().saturating_add(count));
    into_values.extend_from_slice(&into.values[..to]);
    into_values.extend_from_slice(&from.values[start..=end]);
    into_values.extend_from_slice(&into.values[to..]);
    let from_value = NativeValue::Array(from_values);
    let into_value = NativeValue::Array(into_values);
    validate_transferred_field(schema, from.field, &from_value)?;
    validate_transferred_field(schema, into.field, &into_value)?;

    let from_raw = from_items_raw[..start]
        .iter()
        .chain(from_items_raw[end + 1..].iter())
        .copied()
        .collect::<Vec<_>>();
    let into_raw = into_items_raw[..to]
        .iter()
        .chain(from_items_raw[start..=end].iter())
        .chain(into_items_raw[to..].iter())
        .copied()
        .collect::<Vec<_>>();
    let mut replacements = [
        (from_node, render_exact_array(source, from_node, &from_raw)),
        (into_node, render_exact_array(source, into_node, &into_raw)),
    ];
    replacements.sort_by_key(|(node, _)| node.range.from);
    let [(first, first_text), (last, last_text)] = replacements;
    if first.range.to > last.range.from {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_readback_mismatch(),
        ));
    }
    let from_byte = first.range.from;
    let to_byte = last.range.to;
    let text = format!(
        "{first_text}{}{last_text}",
        &source[first.range.to..last.range.from]
    );
    let patched = format!("{}{text}{}", &source[..from_byte], &source[to_byte..]);
    if patched.len() > native::NATIVE_MAX_BYTES {
        return Err(DataError::budget(
            UiMessage::error_data_projection_structured_source_byte_budget_exceeded(),
        ));
    }
    let after = project(&patched, schema);
    if after.qualification != "qualified"
        || !after.diagnostics.is_empty()
        || after.id != projection.id
        || after.types != projection.types
        || after.resolved_types != projection.resolved_types
    {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_readback_mismatch(),
        ));
    }
    for (field, expected) in [(from.field, &from_value), (into.field, &into_value)] {
        let valid = after.fields.iter().find(|value| value.field_id == field.id);
        if !valid.is_some_and(|value| {
            value.writable
                && value.unbound.is_empty()
                && value
                    .cell
                    .native()
                    .is_some_and(|actual| native_equal(actual, expected))
        }) {
            return Err(DataError::new(
                UiMessage::error_data_field_transfer_readback_mismatch(),
            ));
        }
    }
    Ok(FieldPatch {
        from: from_byte,
        to: to_byte,
        text,
        source: patched,
    })
}

fn transfer_field<'a>(
    projection: &'a NoteProjection,
    schema: &'a Schema,
    key: &str,
) -> Result<ListField<'a>> {
    let field = schema.field(key).ok_or_else(|| {
        DataError::new(UiMessage::error_data_projection_selected_field_id_not_installed())
    })?;
    let projected = projection
        .fields
        .iter()
        .find(|projected| projected.field_id == field.id)
        .ok_or_else(|| {
            DataError::new(
                UiMessage::error_data_projection_selected_field_not_contributed_by_current_types(),
            )
        })?;
    if !projected.unbound.is_empty() {
        return Err(DataError::new(
            UiMessage::error_data_projection_unknown_author_members_read_only(),
        ));
    }
    match &projected.cell {
        Cell::Value {
            value: NativeValue::Array(values),
        } if projected.writable && projected.source_range.is_some() => {
            Ok(ListField { field, values })
        }
        _ => Err(DataError::new(
            UiMessage::error_data_field_transfer_existing_lists_required(),
        )),
    }
}

fn validate_transferred_field(schema: &Schema, field: &Field, value: &NativeValue) -> Result<()> {
    let validation = schema.validate(&field.shape, value);
    if !validation.is_valid() || !validation.unbound.is_empty() {
        return Err(DataError::new(
            UiMessage::error_data_projection_field_edit_validation_failed(
                weftext_messages::params::ErrorDataProjectionFieldEditValidationFailed {
                    field: field.id.clone(),
                    state: "invalid".into(),
                },
            ),
        )
        .with_diagnostics(
            validation
                .diagnostics
                .into_iter()
                .chain(validation.unavailable)
                .collect(),
        ));
    }
    Ok(())
}

fn array_node(mut node: &NativeNode) -> &NativeNode {
    while let Some(frame) = &node.frame {
        node = frame;
    }
    node
}

fn exact_items<'a>(source: &'a str, outer: &NativeNode, length: usize) -> Result<Vec<&'a str>> {
    let node = array_node(outer);
    if node.kind != SyntaxKind::Array || node.children.len() != length {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_existing_lists_required(),
        ));
    }
    let trivia_safe = |text: &str| text.chars().all(|c| c.is_whitespace() || c == ',');
    let mut at = node.range.from + 1;
    let mut items = Vec::with_capacity(length);
    for (index, (path, child)) in node.children.iter().enumerate() {
        if path != &(ValuePath::Index { index })
            || child.range.from < at
            || child.range.to > node.range.to - 1
            || !trivia_safe(&source[at..child.range.from])
        {
            return Err(DataError::new(
                UiMessage::error_data_field_transfer_trivia_uncertified(),
            ));
        }
        items.push(&source[child.range.from..child.range.to]);
        at = child.range.to;
    }
    if !trivia_safe(&source[at..node.range.to - 1]) {
        return Err(DataError::new(
            UiMessage::error_data_field_transfer_trivia_uncertified(),
        ));
    }
    Ok(items)
}

fn render_exact_array(source: &str, outer: &NativeNode, items: &[&str]) -> String {
    let node = array_node(outer);
    let inner = if items.is_empty() {
        "()".into()
    } else {
        format!("({},)", items.join(", "))
    };
    format!(
        "{}{inner}{}",
        &source[outer.range.from..node.range.from],
        &source[node.range.to..outer.range.to]
    )
}
