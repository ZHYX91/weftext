use crate::{
    native::{NativeNode, decode_node},
    *,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use typst_syntax::{
    Source,
    ast::{self, AstNode},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase", deny_unknown_fields)]
pub enum Cell {
    Value {
        value: NativeValue,
    },
    Absent,
    Null,
    Invalid {
        diagnostics: Vec<ErrorDescription>,
        value: Option<NativeValue>,
    },
    Unavailable {
        kind: DataErrorKind,
        reason: ErrorDescription,
        diagnostics: Vec<ErrorDescription>,
    },
}
impl Cell {
    pub fn error_diagnostics(&self) -> Vec<ErrorDescription> {
        match self {
            Self::Invalid { diagnostics, .. } => diagnostics.clone(),
            Self::Unavailable {
                reason,
                diagnostics,
                ..
            } => {
                let mut all = vec![reason.clone()];
                all.extend(diagnostics.iter().cloned());
                all
            }
            _ => Vec::new(),
        }
    }
    pub fn is_native_budget(&self) -> bool {
        matches!(
            self,
            Self::Unavailable {
                kind: DataErrorKind::NativeBudget,
                ..
            }
        )
    }
    pub fn unavailable(reason: impl Into<ErrorDescription>) -> Self {
        Self::Unavailable {
            kind: DataErrorKind::Unavailable,
            reason: reason.into(),
            diagnostics: Vec::new(),
        }
    }
    pub fn unavailable_error(error: DataError) -> Self {
        let (kind, reason, details) = error.into_parts();
        Self::Unavailable {
            kind,
            reason,
            diagnostics: details
                .map(DataErrorDetails::into_diagnostics)
                .unwrap_or_default(),
        }
    }
    pub fn unavailable_group(
        kind: DataErrorKind,
        reason: impl Into<ErrorDescription>,
        diagnostics: Vec<ErrorDescription>,
    ) -> Self {
        Self::Unavailable {
            kind,
            reason: reason.into(),
            diagnostics,
        }
    }
    pub fn value(value: NativeValue) -> Self {
        Self::Value { value }
    }
    pub fn native(&self) -> Option<&NativeValue> {
        if let Self::Value { value } = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn state(&self) -> &str {
        match self {
            Self::Value { .. } => "value",
            Self::Absent => "absent",
            Self::Null => "null",
            Self::Invalid { .. } => "invalid",
            Self::Unavailable { .. } => "unavailable",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldProjection {
    pub field_id: String,
    pub key: String,
    pub cell: Cell,
    pub source_range: Option<ByteRange>,
    pub raw: Option<String>,
    pub writable: bool,
    pub required: bool,
    pub unbound: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownField {
    pub key: String,
    pub raw: String,
    pub range: ByteRange,
    pub value: Option<NativeValue>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteProjection {
    pub qualification: String,
    pub id: Option<String>,
    pub types: Vec<String>,
    pub resolved_types: Vec<String>,
    pub fields: Vec<FieldProjection>,
    pub unknown_fields: Vec<UnknownField>,
    pub diagnostics: Vec<ErrorDescription>,
    pub budget_exhausted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldPatch {
    pub from: usize,
    pub to: usize,
    pub text: String,
    pub source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedRefSlot {
    pub from: usize,
    pub to: usize,
    pub target: String,
    pub kind: String,
    pub field_id: String,
    pub path: Vec<ValuePath>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedRefClosure {
    pub slots: Vec<TypedRefSlot>,
    pub complete: bool,
    /// The strict full-copy proof after excluding only unbound root members.
    pub complete_without_unbound_root_fields: bool,
    pub reasons: Vec<ErrorDescription>,
}
/// References proved inside the declared field Shapes only. Unbound author
/// members are preserved outside this claim, even if they contain URI text.
/// This neither establishes target identity nor replaces `validate_note` for
/// required fields, Type constraints and the caller's frozen resource context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredTypedRefClosure {
    pub slots: Vec<TypedRefSlot>,
    pub complete: bool,
    pub reasons: Vec<ErrorDescription>,
    pub unbound: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchTerms {
    pub terms: Vec<String>,
    pub complete: bool,
    pub reasons: Vec<ErrorDescription>,
}
/// Project the installed name/alias semantics into mention dictionary terms.
/// Permanent FieldIds select the semantics; author keys, roles and source text
/// never cause an unrelated field to become a name. The caller adds its
/// producer-qualified native title separately.
pub fn search_terms(source: &str, schema: &Schema) -> SearchTerms {
    let projection = project(source, schema);
    let mut out = SearchTerms {
        terms: vec![],
        complete: projection.qualification == "qualified" && projection.diagnostics.is_empty(),
        reasons: projection.diagnostics.clone(),
    };
    if projection.qualification != "qualified" {
        out.reasons.push(
            weftext_messages::UiMessage::error_data_projection_name_dictionary_control_unavailable(
            )
            .into(),
        );
        return out;
    }
    let mut seen = BTreeSet::new();
    for field_id in ["core/aliases", "contacts/names"] {
        let Some(projected) = projection.fields.iter().find(|f| f.field_id == field_id) else {
            continue;
        };
        let field = schema.fields.iter().find(|f| f.id == field_id).unwrap();
        let recognized_shape = match schema.resolve(&field.shape) {
            Shape::List { items, .. } => match (field_id, schema.resolve(items)) {
                ("core/aliases", Shape::Scalar { semantic }) => semantic == "nonempty_text",
                ("contacts/names", Shape::Record { members, .. }) => {
                    members.get("text").is_some_and(|m| {
                        m.required
                            && !m.nullable
                            && matches!(schema.resolve(&m.shape), Shape::Scalar { semantic } if semantic == "nonempty_text")
                    })
                }
                _ => false,
            },
            _ => false,
        };
        if !recognized_shape {
            out.complete = false;
            out.reasons.push(
                weftext_messages::UiMessage::error_data_projection_installed_name_semantics_unavailable(weftext_messages::params::ErrorDataProjectionInstalledNameSemanticsUnavailable {
                    field_id: (field_id).to_string(),
                })
                .into(),
            );
            continue;
        }
        match &projected.cell {
            Cell::Absent | Cell::Null => {}
            Cell::Value { value } => {
                for item in value.array().unwrap() {
                    let term = if field_id == "core/aliases" {
                        item.text().unwrap()
                    } else {
                        item.member("text").unwrap().text().unwrap()
                    };
                    if seen.insert(term.to_owned()) {
                        out.terms.push(term.to_owned());
                    }
                }
                if !projected.unbound.is_empty() {
                    out.complete = false;
                    out.reasons
                        .push(weftext_messages::UiMessage::error_data_projection_unknown_name_members_preserved(weftext_messages::params::ErrorDataProjectionUnknownNameMembersPreserved {field_id: (field_id).to_string()}).into());
                }
            }
            Cell::Invalid { .. } | Cell::Unavailable { .. } => {
                out.complete = false;
                out.reasons.push(
                    weftext_messages::UiMessage::error_data_projection_name_field_state_unavailable(weftext_messages::params::ErrorDataProjectionNameFieldStateUnavailable {
                        field_id: (field_id).to_string(),
                        cell_state: (projected.cell.state()).to_string(),
                    })
                    .into(),
                );
            }
        }
    }
    out
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidationContext {
    pub owner: Option<String>,
    pub resources: Vec<ResourceMetadata>,
    pub resources_complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMetadata {
    pub resource_ref: String,
    pub owner: String,
    pub lifecycle: String,
}
/// Full typed validation is independent of legal-source saving/compilation.
/// Callers supply only Core-authorized frozen context slots; absent resource
/// metadata never becomes a guessed success or an ambient filesystem read.
pub fn validate_note(source: &str, schema: &Schema, context: &ValidationContext) -> Validation {
    let projection = project(source, schema);
    let mut out = Validation::valid();
    if projection.qualification != "qualified" {
        out.unavailable.push(
            weftext_messages::UiMessage::error_data_projection_root_control_unavailable().into(),
        );
        return out;
    }
    out.diagnostics.extend(projection.diagnostics.clone());
    for field in &projection.fields {
        if field.required && matches!(field.cell, Cell::Absent) {
            out.diagnostics.push(
                weftext_messages::UiMessage::error_data_projection_required_field_absent(
                    weftext_messages::params::ErrorDataProjectionRequiredFieldAbsent {
                        field_id: (field.field_id).to_string(),
                    },
                )
                .into(),
            );
        }
        match &field.cell {
            Cell::Invalid { diagnostics, .. } => {
                out.diagnostics.push(
                    UiMessage::error_data_projection_field_value_invalid(
                        weftext_messages::params::ErrorDataProjectionFieldValueInvalid {
                            field: field.field_id.clone(),
                        },
                    )
                    .into(),
                );
                out.diagnostics.extend(diagnostics.iter().cloned());
            }
            Cell::Unavailable {
                reason,
                diagnostics,
                ..
            } => {
                out.unavailable.push(
                    UiMessage::error_data_projection_field_value_unavailable(
                        weftext_messages::params::ErrorDataProjectionFieldValueUnavailable {
                            field: field.field_id.clone(),
                        },
                    )
                    .into(),
                );
                out.unavailable.push(reason.clone());
                out.unavailable.extend(diagnostics.iter().cloned());
            }
            _ => {}
        }
    }
    for t in schema
        .types
        .iter()
        .filter(|t| projection.resolved_types.contains(&t.id))
    {
        for c in &t.constraints {
            merge_validation(
                &mut out,
                validate_type_constraint(c, &projection, schema, context),
            );
        }
    }
    out
}
pub fn validate_field_change(
    before: &str,
    after: &str,
    schema: &Schema,
    field_id: &str,
    context: &ValidationContext,
) -> Validation {
    let original = project(before, schema);
    let proposed = project(after, schema);
    let mut out = Validation::valid();
    if original.qualification != "qualified"
        || proposed.qualification != "qualified"
        || original.id != proposed.id
        || original.types != proposed.types
    {
        out.unavailable
            .push(weftext_messages::UiMessage::error_data_projection_field_edit_must_preserve_qualified_controls().into());
        return out;
    }
    let Some(field) = schema.fields.iter().find(|f| f.id == field_id) else {
        out.unavailable.push(
            weftext_messages::UiMessage::error_data_projection_selected_field_id_not_installed()
                .into(),
        );
        return out;
    };
    let Some(value) = proposed.fields.iter().find(|f| f.field_id == field.id) else {
        out.unavailable
            .push(weftext_messages::UiMessage::error_data_projection_selected_field_not_contributed_by_current_types().into());
        return out;
    };
    match &value.cell {
        Cell::Invalid { diagnostics, .. } => out.diagnostics.extend(diagnostics.clone()),
        Cell::Unavailable {
            reason,
            diagnostics,
            ..
        } => {
            out.unavailable.push(reason.clone());
            out.unavailable.extend(diagnostics.iter().cloned());
        }
        _ => {}
    }
    for t in schema
        .types
        .iter()
        .filter(|t| proposed.resolved_types.contains(&t.id))
    {
        for c in &t.constraints {
            if c.inputs
                .values()
                .any(|s| matches!(s,Selector::Field{id,..}if id==field_id))
            {
                merge_validation(
                    &mut out,
                    validate_type_constraint(c, &proposed, schema, context),
                );
            }
        }
    }
    out
}
fn merge_validation(out: &mut Validation, v: Validation) {
    out.diagnostics.extend(v.diagnostics);
    out.unavailable.extend(v.unavailable);
    out.unbound.extend(v.unbound);
}
pub(crate) fn select_values<'a>(
    value: &'a NativeValue,
    path: &[SchemaPath],
    out: &mut Vec<&'a NativeValue>,
) {
    if path.is_empty() {
        out.push(value);
        return;
    }
    match &path[0] {
        SchemaPath::Member { key } => {
            if let Some(v) = value.member(key) {
                select_values(v, &path[1..], out);
            }
        }
        SchemaPath::Each => {
            if let Some(a) = value.array() {
                for v in a {
                    select_values(v, &path[1..], out);
                }
            }
        }
    }
}
fn constraint_values<'a>(
    c: &Constraint,
    slot: &str,
    projection: &'a NoteProjection,
) -> std::result::Result<Vec<&'a NativeValue>, ErrorDescription> {
    let Some(Selector::Field { id, path }) = c.inputs.get(slot) else {
        return Err(UiMessage::error_data_projection_constraint_field_selector_required().into());
    };
    let field = projection
        .fields
        .iter()
        .find(|f| &f.field_id == id)
        .ok_or_else(|| {
            ErrorDescription::new(
                UiMessage::error_data_projection_constraint_field_closure_missing(),
            )
        })?;
    match &field.cell {
        Cell::Absent => Ok(vec![]),
        Cell::Value { value } => {
            let mut out = vec![];
            select_values(value, path, &mut out);
            Ok(out)
        }
        _ => Err(UiMessage::error_data_projection_constraint_input_not_value(
            weftext_messages::params::ErrorDataProjectionConstraintInputNotValue {
                field: id.clone(),
            },
        )
        .into()),
    }
}
fn validate_type_constraint(
    c: &Constraint,
    p: &NoteProjection,
    schema: &Schema,
    context: &ValidationContext,
) -> Validation {
    let mut out = Validation::valid();
    let Some(v) = schema.validators.iter().find(|v| v.id == c.validator) else {
        out.unavailable
            .push(weftext_messages::UiMessage::error_data_projection_validator_missing().into());
        return out;
    };
    match v.operation.as_str() {
        "owned-resources" => {
            let values = match constraint_values(c, "values", p) {
                Ok(v) => v,
                Err(e) => {
                    out.unavailable.push(e);
                    return out;
                }
            };
            for value in values {
                let Some(reference) = value.text() else {
                    out.diagnostics
                        .push(weftext_messages::UiMessage::error_data_projection_resource_selector_not_scalar_ref().into());
                    continue;
                };
                let metadata = context
                    .resources
                    .iter()
                    .filter(|m| m.resource_ref == reference)
                    .collect::<Vec<_>>();
                if metadata.len() != 1 {
                    if context.resources_complete {
                        out.diagnostics
                            .push(weftext_messages::UiMessage::error_data_projection_required_resource_not_exact_live_admitted_object().into());
                    } else {
                        out.unavailable
                            .push(weftext_messages::UiMessage::error_data_projection_resource_metadata_slot_unavailable().into());
                    }
                    continue;
                }
                if metadata[0].lifecycle != "live" {
                    out.diagnostics.push(
                        weftext_messages::UiMessage::error_data_projection_resource_not_live().into(),
                    );
                }
                if c.parameters["owner_required"].as_bool() == Some(true) {
                    match &context.owner {
                        Some(owner) if &metadata[0].owner == owner => {}
                        Some(_) => out
                            .diagnostics
                            .push(weftext_messages::UiMessage::error_data_projection_resource_belongs_to_different_author_owner().into()),
                        None => out
                            .unavailable
                            .push(weftext_messages::UiMessage::error_data_projection_owner_context_slot_unavailable().into()),
                    }
                }
            }
        }
        "task-reminder" => {
            let reminder = match constraint_values(c, "reminder", p) {
                Ok(v) => v,
                Err(e) => {
                    out.unavailable.push(e);
                    return out;
                }
            };
            let due = match constraint_values(c, "due", p) {
                Ok(v) => v,
                Err(e) => {
                    out.unavailable.push(e);
                    return out;
                }
            };
            if reminder.len() > 1 || due.len() > 1 {
                out.diagnostics
                    .push(UiMessage::error_data_task_reminder_selector_cardinality().into());
                return out;
            }
            let resolution = crate::resolve_task_reminder(
                reminder.first().copied(),
                due.first().copied(),
                schema,
            );
            match resolution.validation() {
                Cell::Invalid { diagnostics, .. } => out.diagnostics.extend(diagnostics),
                Cell::Unavailable { reason, diagnostics, .. } => { out.unavailable.push(reason); out.unavailable.extend(diagnostics); },
                _ => {}
            }
        }
        "event" => {
            let recurrence = match constraint_values(c, "recurrence", p) {
                Ok(v) => v,
                Err(e) => {
                    out.unavailable.push(e);
                    return out;
                }
            };
            let range = match constraint_values(c, "range", p) {
                Ok(v) => v,
                Err(e) => {
                    out.unavailable.push(e);
                    return out;
                }
            };
            if range.len() != 1 {
                out.unavailable
                    .push(weftext_messages::UiMessage::error_data_projection_required_event_range_unavailable().into());
                return out;
            }
            match crate::validate_event(range[0], recurrence.first().copied(), schema) {
                Cell::Invalid { diagnostics, .. } => out.diagnostics.extend(diagnostics),
                Cell::Unavailable {
                    reason,
                    diagnostics,
                    ..
                } => {
                    out.unavailable.push(reason);
                    out.unavailable.extend(diagnostics);
                }
                _ => {}
            }
        }
        _ => out.unavailable.push(
            weftext_messages::UiMessage::error_data_projection_unsupported_type_validator_invocation()
                .into(),
        ),
    }
    out
}
struct Declaration {
    node: NativeNode,
    close: usize,
}
fn declaration(source: &str, label: &str) -> Result<Declaration> {
    declaration_selected(source, label, None)
}
fn declaration_selected(
    source: &str,
    label: &str,
    selected: Option<&BTreeSet<String>>,
) -> Result<Declaration> {
    if source.len() > native::NATIVE_MAX_BYTES {
        return Err(DataError::budget(
            weftext_messages::UiMessage::error_data_projection_structured_source_byte_budget_exceeded(),
        ));
    }
    let certified =
        weftext_engine::certify_metadata_declaration(source, label).ok_or_else(|| {
            DataError::unavailable(weftext_messages::UiMessage::error_data_projection_direct_metadata_binding_unavailable_or_ambiguous())
        })?;
    let parsed = Source::detached(source);
    let call = parsed
        .root()
        .children()
        .filter_map(|n| n.cast::<ast::FuncCall>())
        .find(|c| {
            parsed
                .find(c.to_untyped().span())
                .is_some_and(|n| n.range().start == certified.from)
        })
        .ok_or_else(|| {
            DataError::new(
                weftext_messages::UiMessage::error_data_projection_certified_ast_declaration_missing(),
            )
        })?;
    let args: Vec<_> = call.args().items().collect();
    let [ast::Arg::Pos(e)] = args.as_slice() else {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_root_metadata_requires_one_dictionary(),
        ));
    };
    let dict = e.to_untyped().cast::<ast::Dict>().ok_or_else(|| {
        DataError::new(
            weftext_messages::UiMessage::error_data_projection_root_metadata_not_direct_dictionary(
            ),
        )
    })?;
    let mut seen = BTreeSet::new();
    let mut children = vec![];
    for item in dict.items() {
        let (k, v) = match item {
            ast::DictItem::Named(n) => (n.name().get().to_string(), n.expr()),
            ast::DictItem::Keyed(n) => (
                n.key()
                    .to_untyped()
                    .cast::<ast::Str>()
                    .ok_or_else(|| {
                        DataError::new(
                            weftext_messages::UiMessage::error_data_projection_dynamic_dictionary_key(),
                        )
                    })?
                    .get()
                    .to_string(),
                n.expr(),
            ),
            _ => {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_projection_root_metadata_spread_unsupported(),
                ));
            }
        };
        if !seen.insert(k.clone()) {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_projection_duplicate_decoded_root_key(),
            ));
        }
        // A narrow producer inspects root keys/control bindings, but does not
        // decode, validate, or evaluate unselected author values.
        if selected.is_some_and(|keys| !keys.contains(&k)) && k != "id" && k != "types" {
            continue;
        }
        children.push((
            ValuePath::Member { key: k },
            decode_node(v.to_untyped(), &parsed),
        ));
    }
    let range = parsed.find(dict.to_untyped().span()).unwrap().range();
    let node = NativeNode {
        kind: typst_syntax::SyntaxKind::Dict,
        value: None,
        range: ByteRange {
            from: range.start,
            to: range.end,
        },
        children,
        reason: None,
        reason_kind: None,
        commas: vec![],
        frame: None,
        entries: vec![],
        empty_colon: None,
    };
    Ok(Declaration {
        node,
        close: range.end - 1,
    })
}
fn root_field<'a>(d: &'a Declaration, key: &str) -> Option<&'a NativeNode> {
    d.node
        .children
        .iter()
        .find(|(p, _)| matches!(p,ValuePath::Member{key:k}if k==key))
        .map(|(_, v)| v)
}
fn controls(d: &Declaration) -> Result<(String, Vec<String>)> {
    let id = root_field(d, "id")
        .filter(|n| n.kind == typst_syntax::SyntaxKind::Str)
        .and_then(|n| n.value.as_ref())
        .and_then(NativeValue::text)
        .filter(|s| uuid_v4(s))
        .ok_or_else(|| {
            DataError::new(
                weftext_messages::UiMessage::error_data_projection_id_requires_direct_lowercase_uuid_v4(),
            )
        })?;
    let types = match root_field(d, "types") {
        None => vec![],
        Some(n) => {
            if n.kind != typst_syntax::SyntaxKind::Array
                || n.children
                    .iter()
                    .any(|(_, n)| n.kind != typst_syntax::SyntaxKind::Str)
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_projection_types_requires_direct_array_and_strings(),
                ));
            }
            n.value
                .as_ref()
                .and_then(NativeValue::array)
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_projection_types_requires_direct_string_array()))?
                .iter()
                .map(|v| {
                    v.text()
                        .map(str::to_owned)
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_projection_types_contains_non_string()))
                })
                .collect::<Result<Vec<_>>>()?
        }
    };
    Ok((id.into(), types))
}
/// Project independently static fields even when an adjacent field is computed.
pub fn project(source: &str, schema: &Schema) -> NoteProjection {
    project_selected(source, schema, None)
}
fn project_selected(
    source: &str,
    schema: &Schema,
    selected: Option<&BTreeSet<String>>,
) -> NoteProjection {
    let d = match declaration_selected(source, "weftext", selected) {
        Ok(d) => d,
        Err(e) => {
            return NoteProjection {
                qualification: "unavailable".into(),
                id: None,
                types: vec![],
                resolved_types: vec![],
                fields: vec![],
                unknown_fields: vec![],
                budget_exhausted: matches!(
                    e.kind(),
                    DataErrorKind::NativeBudget | DataErrorKind::Budget
                ),
                diagnostics: e.into_diagnostics(),
            };
        }
    };
    let (id, types) = match controls(&d) {
        Ok(v) => v,
        Err(e) => {
            return NoteProjection {
                qualification: "invalid-control".into(),
                id: None,
                types: vec![],
                resolved_types: vec![],
                fields: vec![],
                unknown_fields: vec![],
                budget_exhausted: matches!(
                    e.kind(),
                    DataErrorKind::NativeBudget | DataErrorKind::Budget
                ),
                diagnostics: e.into_diagnostics(),
            };
        }
    };
    let (resolved, check) = schema.expand_types(&types);
    let mut diagnostics = check.diagnostics;
    diagnostics.extend(check.unavailable);
    let fields = schema
        .applicable_fields(&resolved)
        .into_iter()
        .filter(|(field, _)| selected.is_none_or(|keys| keys.contains(&field.key)))
        .map(|(field, required)| {
            let node = root_field(&d, &field.key);
            let mut unbound = vec![];
            let cell = match node {
                None => Cell::Absent,
                Some(n) => match &n.value {
                    None => Cell::Unavailable {
                        kind: n.reason_kind.unwrap_or(DataErrorKind::Unavailable),
                        reason: n.reason.clone().unwrap_or_else(|| {
                            UiMessage::error_data_native_expression_computed().into()
                        }),
                        diagnostics: vec![],
                    },
                    Some(NativeValue::None) if field.nullable => Cell::Null,
                    Some(value) => {
                        let validation = schema.validate(&field.shape, value);
                        unbound = validation.unbound;
                        if !validation.diagnostics.is_empty() {
                            Cell::Invalid {
                                diagnostics: validation.diagnostics,
                                value: Some(value.clone()),
                            }
                        } else if !validation.unavailable.is_empty() {
                            Cell::unavailable_group(
                                DataErrorKind::Unavailable,
                                UiMessage::error_data_projection_validation_unavailable(),
                                validation.unavailable,
                            )
                        } else {
                            Cell::value(value.clone())
                        }
                    }
                },
            };
            let writable =
                node.is_none_or(|n| n.value.is_some()) && !matches!(cell, Cell::Unavailable { .. });
            FieldProjection {
                field_id: field.id.clone(),
                key: field.key.clone(),
                cell,
                source_range: node.map(|n| n.range.clone()),
                raw: node.and_then(|n| source.get(n.range.from..n.range.to).map(str::to_owned)),
                writable,
                required,
                unbound,
            }
        })
        .collect::<Vec<_>>();
    let budget_exhausted = fields.iter().any(|field| field.cell.is_native_budget());
    let known: BTreeSet<_> = fields.iter().map(|f| f.key.as_str()).collect();
    let unknown_fields = d
        .node
        .children
        .iter()
        .filter_map(|(p, n)| {
            let ValuePath::Member { key } = p else {
                return None;
            };
            if matches!(key.as_str(), "id" | "types") || known.contains(key.as_str()) {
                return None;
            }
            Some(UnknownField {
                key: key.clone(),
                raw: source.get(n.range.from..n.range.to).unwrap_or("").into(),
                range: n.range.clone(),
                value: n.value.clone(),
            })
        })
        .collect();
    NoteProjection {
        qualification: "qualified".into(),
        id: Some(id),
        types,
        resolved_types: resolved.into_iter().collect(),
        fields,
        unknown_fields,
        diagnostics,
        budget_exhausted,
    }
}
fn node_at<'a>(node: &'a NativeNode, path: &[ValuePath]) -> Option<&'a NativeNode> {
    let mut n = node;
    for p in path {
        n = &n.children.iter().find(|(k, _)| k == p)?.1;
    }
    Some(n)
}
pub fn patch_field(
    source: &str,
    schema: &Schema,
    key: &str,
    path: &[ValuePath],
    replacement: &NativeValue,
) -> Result<FieldPatch> {
    patch_field_selected(source, schema, key, path, replacement, None)
}
fn patch_field_selected(
    source: &str,
    schema: &Schema,
    key: &str,
    path: &[ValuePath],
    replacement: &NativeValue,
    selected: Option<&BTreeSet<String>>,
) -> Result<FieldPatch> {
    let projection = project_selected(source, schema, selected);
    if projection.qualification != "qualified" || !projection.diagnostics.is_empty() {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_control_types_unqualified_or_conflicting(),
        ));
    }
    let field = projection
        .fields
        .iter()
        .find(|f| f.field_id == key || selected.is_none() && f.key == key)
        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_projection_field_not_contributed_by_current_types()))?;
    if !field.writable {
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_projection_computed_or_provider_unavailable_field_read_only(),
        ));
    }
    let d = declaration_selected(source, "weftext", selected)?;
    if let Some(node) = root_field(&d, &field.key)
        && let Some(value) = &node.value
    {
        editable_shape_at(
            schema,
            &schema
                .fields
                .iter()
                .find(|f| f.id == field.field_id)
                .unwrap()
                .shape,
            value,
            path,
        )?;
    }
    let text = replacement.to_typst()?;
    // Reparse writer output through the official AST rather than trusting DTOs.
    let readback = decode_native_expression(&text)?;
    if &readback != replacement && !native_equal(&readback, replacement) {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_native_writer_exact_readback_failed(
            ),
        ));
    }
    let (from, to, text) = match root_field(&d, &field.key) {
        Some(n) => {
            let selected = node_at(n, path)
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_projection_value_path_has_no_exact_author_expression()))?;
            if selected.value.is_none() {
                return Err(DataError::new(weftext_messages::UiMessage::error_data_projection_computed_value_overwrite_forbidden()));
            }
            let selected_shape = editable_shape_at(
                schema,
                &schema
                    .fields
                    .iter()
                    .find(|f| f.id == field.field_id)
                    .unwrap()
                    .shape,
                n.value.as_ref().unwrap(),
                path,
            )?;
            let text = preserving_writer(selected, replacement, selected_shape, schema, source)?;
            (selected.range.from, selected.range.to, text)
        }
        None if path.is_empty() => {
            let prefix = if d.node.children.is_empty() { "" } else { ", " };
            let before = &source[..d.close];
            let prefix = if before.trim_end().ends_with(',') {
                " "
            } else {
                prefix
            };
            (
                d.close,
                d.close,
                format!("{prefix}{}: {text}", typst_string(&field.key)),
            )
        }
        _ => {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_projection_missing_nested_member_not_writable_slot(),
            ));
        }
    };
    let patched = format!("{}{}{}", &source[..from], text, &source[to..]);
    let after = project_selected(&patched, schema, selected);
    if after.qualification != "qualified"
        || after.id != projection.id
        || after.types != projection.types
    {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_field_edit_changed_control_authority(
            ),
        ));
    }
    let checked = after
        .fields
        .iter()
        .find(|f| f.field_id == field.field_id)
        .unwrap();
    if matches!(
        checked.cell,
        Cell::Invalid { .. } | Cell::Unavailable { .. }
    ) {
        let kind = match &checked.cell {
            Cell::Unavailable { kind, .. } => *kind,
            _ => DataErrorKind::Invalid,
        };
        return Err(DataError::with_kind(
            kind,
            UiMessage::error_data_projection_field_edit_validation_failed(
                weftext_messages::params::ErrorDataProjectionFieldEditValidationFailed {
                    field: field.field_id.clone(),
                    state: checked.cell.state().into(),
                },
            ),
        )
        .with_diagnostics(checked.cell.error_diagnostics()));
    }
    let exact = checked
        .cell
        .native()
        .and_then(|value| value.at(path))
        .is_some_and(|value| native_equal(value, replacement))
        || path.is_empty()
            && matches!(checked.cell, Cell::Null)
            && matches!(replacement, NativeValue::None);
    if !exact {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_field_patch_native_value_or_order_mismatch(),
        ));
    }
    Ok(FieldPatch {
        from,
        to,
        text,
        source: patched,
    })
}
fn preserving_writer(
    node: &NativeNode,
    replacement: &NativeValue,
    shape: &Shape,
    schema: &Schema,
    source: &str,
) -> Result<String> {
    let raw = &source[node.range.from..node.range.to];
    if node.value.as_ref() == Some(replacement) {
        return Ok(raw.into());
    }
    if let Some(frame) = &node.frame {
        let text = preserving_writer(frame, replacement, shape, schema, source)?;
        return Ok(apply_inner(source, node, vec![(frame.range.clone(), text)]));
    }
    let shape = schema.resolve(shape);
    let new_shape = if let Shape::Union {
        variants,
        discriminator,
    } = shape
    {
        schema.resolve(schema.union_variant(variants, discriminator.as_ref(), replacement)?)
    } else {
        shape
    };
    let shape = if let Shape::Union {
        variants,
        discriminator,
    } = shape
    {
        node.value
            .as_ref()
            .and_then(|v| {
                schema
                    .union_variant(variants, discriminator.as_ref(), v)
                    .ok()
            })
            .map(|s| schema.resolve(s))
            .unwrap_or(shape)
    } else {
        shape
    };
    match (&node.value, replacement, shape) {
        (
            Some(NativeValue::Record(before)),
            NativeValue::Record(after),
            Shape::Record { members, .. },
        ) => {
            let Shape::Record {
                members: new_members,
                ..
            } = new_shape
            else {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_projection_record_branch_requires_record_target(),
                ));
            };
            preserving_record(node, before, after, members, new_members, schema, source)
        }
        (
            Some(NativeValue::Array(before)),
            NativeValue::Array(after),
            Shape::List { items, .. },
        ) => preserving_array(node, before, after, items, schema, source),
        (
            Some(NativeValue::Record(_) | NativeValue::Array(_)),
            NativeValue::Record(_) | NativeValue::Array(_),
            _,
        ) => Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_structural_edit_cannot_preserve_untouched_slots(),
        )),
        _ => replacement.to_typst(),
    }
}
fn apply_inner(source: &str, node: &NativeNode, mut changes: Vec<(ByteRange, String)>) -> String {
    changes.sort_by_key(|(r, _)| (r.from, r.to));
    let mut output = String::new();
    let mut at = node.range.from;
    for (range, text) in changes {
        output.push_str(&source[at..range.from]);
        output.push_str(&text);
        at = range.to;
    }
    output.push_str(&source[at..node.range.to]);
    output
}
fn preserving_record(
    node: &NativeNode,
    before: &[NativeMember],
    after: &[NativeMember],
    members: &BTreeMap<String, Member>,
    new_members: &BTreeMap<String, Member>,
    schema: &Schema,
    source: &str,
) -> Result<String> {
    let existing = after
        .iter()
        .filter(|m| before.iter().any(|old| old.key == m.key))
        .map(|m| &m.key)
        .collect::<Vec<_>>();
    let original_order = before
        .iter()
        .filter(|m| after.iter().any(|new| new.key == m.key))
        .map(|m| &m.key)
        .collect::<Vec<_>>();
    if existing != original_order {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_record_member_reorder_requires_source_edit(),
        ));
    }
    let removed = before
        .iter()
        .map(|old| !after.iter().any(|new| new.key == old.key))
        .collect::<Vec<_>>();
    let mut changes = vec![];
    for (index, old) in before.iter().enumerate() {
        if removed[index] && !members.contains_key(&old.key) {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_projection_unknown_author_members_read_only(
                ),
            ));
        }
        let Some(new) = after.iter().find(|new| new.key == old.key) else {
            continue;
        };
        if old.value == new.value {
            continue;
        }
        if !members.contains_key(&old.key) {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_projection_unknown_author_members_read_only(
                ),
            ));
        }
        let member = new_members.get(&old.key).ok_or_else(|| {
            DataError::new(
                weftext_messages::UiMessage::error_data_projection_changed_member_absent_from_new_branch(),
            )
        })?;
        let child = &node
            .children
            .iter()
            .find(|(path, _)| matches!(path, ValuePath::Member { key } if key == &old.key))
            .unwrap()
            .1;
        changes.push((
            child.range.clone(),
            preserving_writer(child, &new.value, &member.shape, schema, source)?,
        ));
    }
    let mut removed_commas = BTreeSet::new();
    for (index, _) in removed.iter().enumerate().filter(|(_, removed)| **removed) {
        let from = node.entries[index].1.from;
        let to = node.entries[index].1.to;
        let next_comma = node.commas.iter().find(|c| {
            c.from >= to
                && node
                    .entries
                    .get(index + 1)
                    .is_none_or(|(_, entry)| c.to <= entry.from)
        });
        let comma = if let Some(comma) = next_comma {
            Some(comma)
        } else if index > 0 {
            Some(
                node.commas
                    .iter()
                    .rev()
                    .find(|c| c.to <= from)
                    .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_projection_certified_member_separator_missing()))?,
            )
        } else {
            None
        };
        changes.push((ByteRange { from, to }, String::new()));
        if let Some(comma) = comma
            && removed_commas.insert((comma.from, comma.to))
        {
            changes.push((comma.clone(), String::new()));
        }
    }
    let additions = after
        .iter()
        .filter(|new| !before.iter().any(|old| old.key == new.key))
        .map(|new| {
            let member = new_members
                .get(&new.key)
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_projection_unbound_member_addition_forbidden()))?;
            if !schema
                .validate(&member.shape, &new.value)
                .unbound
                .is_empty()
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_projection_unbound_nested_member_creation_forbidden(),
                ));
            }
            Ok((
                new.key.clone(),
                format!("{}: {}", typst_string(&new.key), new.value.to_typst()?),
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let retained = before
        .iter()
        .enumerate()
        .filter(|(i, _)| !removed[*i])
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if !additions.is_empty() {
        if let Some(colon) = &node.empty_colon {
            changes.push((colon.clone(), String::new()));
        }
        let mut pending = vec![];
        for member in after {
            if let Some(text) = additions.get(&member.key) {
                pending.push(text.clone());
            } else if !pending.is_empty() {
                let from = node
                    .entries
                    .iter()
                    .find(|(key, _)| key == &member.key)
                    .unwrap()
                    .1
                    .from;
                changes.push((
                    ByteRange { from, to: from },
                    format!("{}, ", pending.join(", ")),
                ));
                pending.clear();
            }
        }
        if !pending.is_empty() {
            let trailing = retained.last().is_some_and(|index| {
                node.commas.iter().any(|comma| {
                    comma.from >= node.entries[*index].1.to
                        && !changes
                            .iter()
                            .any(|(range, _)| range.from <= comma.from && comma.to <= range.to)
                })
            });
            let prefix = if retained.is_empty() || trailing {
                " "
            } else {
                ", "
            };
            let close = node.range.to - 1;
            changes.push((
                ByteRange {
                    from: close,
                    to: close,
                },
                format!("{prefix}{}", pending.join(", ")),
            ));
        }
    } else if retained.is_empty() && !before.is_empty() {
        let open = node.range.from + 1;
        changes.push((
            ByteRange {
                from: open,
                to: open,
            },
            ":".into(),
        ));
    }
    Ok(apply_inner(source, node, changes))
}
fn preserving_array(
    node: &NativeNode,
    before: &[NativeValue],
    after: &[NativeValue],
    items: &Shape,
    schema: &Schema,
    source: &str,
) -> Result<String> {
    if before.len().saturating_mul(after.len()) > 1_000_000 {
        return Err(DataError::budget(
            weftext_messages::UiMessage::error_data_projection_array_transformation_work_budget_exceeded(),
        ));
    }
    let mut consumed = BTreeSet::new();
    let mut origins = vec![None; after.len()];
    // Reserve unchanged expressions before rebasing changed items, so a move
    // cannot take the preservation basis of a later unchanged duplicate.
    for (position, value) in after.iter().enumerate() {
        let exact = before
            .get(position)
            .filter(|old| *old == value)
            .map(|_| position)
            .filter(|index| !consumed.contains(index))
            .or_else(|| {
                before
                    .iter()
                    .enumerate()
                    .find(|(index, old)| !consumed.contains(index) && *old == value)
                    .map(|(index, _)| index)
            });
        if let Some(index) = exact {
            origins[position] = Some(index);
            consumed.insert(index);
        }
    }
    let mut texts = vec![];
    for (position, value) in after.iter().enumerate() {
        if let Some(origin) = origins[position] {
            let range = &node.children[origin].1.range;
            texts.push(source[range.from..range.to].to_owned());
            continue;
        }
        let mut candidates = (0..before.len())
            .filter(|index| !consumed.contains(index))
            .collect::<Vec<_>>();
        candidates.sort_by_key(|index| (*index != position, *index));
        let preserved = candidates.iter().find_map(|index| {
            preserving_writer(&node.children[*index].1, value, items, schema, source)
                .ok()
                .map(|text| (*index, text))
        });
        if let Some((origin, text)) = preserved {
            consumed.insert(origin);
            texts.push(text);
        } else {
            if !candidates.is_empty() {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_projection_array_item_preservation_basis_uncertified(),
                ));
            }
            if !schema.validate(items, value).unbound.is_empty()
                || candidates
                    .iter()
                    .any(|index| !schema.validate(items, &before[*index]).unbound.is_empty())
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_projection_array_cannot_overwrite_or_invent_unknown_members(),
                ));
            }
            texts.push(value.to_typst()?);
        }
    }
    let mut changes = node
        .children
        .iter()
        .take(after.len())
        .zip(&texts)
        .map(|((_, child), text)| (child.range.clone(), text.clone()))
        .collect::<Vec<_>>();
    let mut removed_commas = BTreeSet::new();
    for (index, (_, child)) in node.children.iter().enumerate().skip(after.len()) {
        changes.push((child.range.clone(), String::new()));
        let next = node.commas.iter().find(|comma| {
            comma.from >= child.range.to
                && node
                    .children
                    .get(index + 1)
                    .is_none_or(|(_, next)| comma.to <= next.range.from)
        });
        let comma = next.or_else(|| {
            node.commas
                .iter()
                .rev()
                .find(|comma| comma.to <= child.range.from)
        });
        if let Some(comma) = comma
            && removed_commas.insert((comma.from, comma.to))
        {
            changes.push((comma.clone(), String::new()));
        }
    }
    if after.len() > before.len() {
        let trailing = node
            .children
            .last()
            .is_some_and(|(_, last)| node.commas.iter().any(|comma| comma.from >= last.range.to));
        let prefix = if before.is_empty() || trailing {
            " "
        } else {
            ", "
        };
        let close = node.range.to - 1;
        changes.push((
            ByteRange {
                from: close,
                to: close,
            },
            format!("{prefix}{},", texts[before.len()..].join(", ")),
        ));
    } else if after.len() == 1 {
        let last = &node.children[0].1.range;
        let trailing = node.commas.iter().any(|comma| {
            comma.from >= last.to && !removed_commas.contains(&(comma.from, comma.to))
        });
        if !trailing {
            changes.push((
                ByteRange {
                    from: last.to,
                    to: last.to,
                },
                ",".into(),
            ));
        }
    }
    Ok(apply_inner(source, node, changes))
}
fn editable_shape_at<'a>(
    schema: &'a Schema,
    shape: &'a Shape,
    value: &NativeValue,
    path: &[ValuePath],
) -> Result<&'a Shape> {
    let shape = schema.resolve(shape);
    if path.is_empty() {
        return Ok(shape);
    }
    let shape = if let Shape::Union {
        variants,
        discriminator,
    } = shape
    {
        schema.resolve(schema.union_variant(variants, discriminator.as_ref(), value)?)
    } else {
        shape
    };
    match (shape, &path[0]) {
        (Shape::Record { members, .. }, ValuePath::Member { key }) => {
            let member = members
                .get(key)
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_projection_unbound_author_member_writer_unavailable()))?;
            let value = value.member(key).ok_or_else(|| {
                DataError::new(
                    weftext_messages::UiMessage::error_data_projection_nested_source_member_missing(
                    ),
                )
            })?;
            editable_shape_at(schema, &member.shape, value, &path[1..])
        }
        (Shape::List { items, .. }, ValuePath::Index { index }) => {
            let value = value.array().and_then(|a| a.get(*index)).ok_or_else(|| {
                DataError::new(
                    weftext_messages::UiMessage::error_data_projection_invalid_item_ordinal(),
                )
            })?;
            editable_shape_at(schema, items, value, &path[1..])
        }
        _ => Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_typed_path_schema_mismatch(),
        )),
    }
}
fn native_equal(a: &NativeValue, b: &NativeValue) -> bool {
    match (a, b) {
        (NativeValue::Float(a), NativeValue::Float(b)) => {
            a.parse::<f64>().ok().map(f64::to_bits) == b.parse::<f64>().ok().map(f64::to_bits)
        }
        (NativeValue::Array(a), NativeValue::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| native_equal(a, b))
        }
        (NativeValue::Record(a), NativeValue::Record(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(a, b)| a.key == b.key && native_equal(&a.value, &b.value))
        }
        _ => a == b,
    }
}
pub fn patch_types(source: &str, schema: &Schema, types: &[String]) -> Result<FieldPatch> {
    let (_, validation) = schema.expand_types(types);
    if !validation.is_valid() {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_type_composition_invalid_or_conflicting(),
        ));
    }
    let d = declaration(source, "weftext")?;
    let (id, _) = controls(&d)?;
    let text =
        NativeValue::Array(types.iter().cloned().map(NativeValue::Text).collect()).to_typst()?;
    let (from, to, text) = match root_field(&d, "types") {
        Some(n) if n.value.is_some() => (n.range.from, n.range.to, text),
        None => (d.close, d.close, format!(", types: {text}")),
        _ => {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_projection_computed_types(),
            ));
        }
    };
    let patched = format!("{}{}{}", &source[..from], text, &source[to..]);
    let after = project(&patched, schema);
    if after.id.as_deref() != Some(&id)
        || after.fields.iter().any(|f| {
            matches!(f.cell, Cell::Invalid { .. }) || f.required && matches!(f.cell, Cell::Absent)
        })
    {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_type_change_requires_same_draft_field_repair(),
        ));
    }
    Ok(FieldPatch {
        from,
        to,
        text,
        source: patched,
    })
}
/// Returns only refs declared by Shapes, with exact official AST source slots.
pub fn typed_ref_slots(source: &str, schema: &Schema) -> Result<Vec<TypedRefSlot>> {
    let closure = typed_ref_closure(source, schema)?;
    if !closure.complete {
        return Err(DataError::unavailable(
            UiMessage::error_data_projection_typed_reference_closure_incomplete(),
        )
        .with_diagnostics(closure.reasons));
    }
    Ok(closure.slots)
}
pub fn typed_ref_closure(source: &str, schema: &Schema) -> Result<TypedRefClosure> {
    let d = declaration(source, "weftext")?;
    let (_, tokens) = controls(&d)?;
    let (types, validation) = schema.expand_types(&tokens);
    if !validation.is_valid() {
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_projection_type_closure_unavailable(),
        ));
    }
    let mut out = vec![];
    let mut reasons = vec![];
    let mut complete_without_unbound_root_fields = true;
    let projection = project(source, schema);
    if !projection.unknown_fields.is_empty() {
        reasons.push(
            weftext_messages::UiMessage::error_data_projection_unbound_root_outside_typed_closure()
                .into(),
        );
    }
    for (field, _) in schema.applicable_fields(&types) {
        if let Some(n) = root_field(&d, &field.key) {
            if let Some(v) = &n.value {
                let check = schema.validate(&field.shape, v);
                if !check.is_valid() {
                    complete_without_unbound_root_fields = false;
                    reasons.push(weftext_messages::UiMessage::error_data_projection_field_typed_value_unavailable(weftext_messages::params::ErrorDataProjectionFieldTypedValueUnavailable {field: field.id.clone()}).into());
                    continue;
                }
                if !check.unbound.is_empty() {
                    complete_without_unbound_root_fields = false;
                    reasons.push(
                        weftext_messages::UiMessage::error_data_projection_field_author_members_unbound(weftext_messages::params::ErrorDataProjectionFieldAuthorMembersUnbound {
                            field: field.id.clone(),
                        })
                        .into(),
                    );
                }
                ref_walk(schema, &field.shape, n, &field.id, &mut vec![], &mut out)?;
            } else {
                complete_without_unbound_root_fields = false;
                reasons.push(
                    weftext_messages::UiMessage::error_data_projection_field_ref_closure_computed(
                        weftext_messages::params::ErrorDataProjectionFieldRefClosureComputed {
                            field: field.id.clone(),
                        },
                    )
                    .into(),
                );
            }
        }
    }
    Ok(TypedRefClosure {
        slots: out,
        complete: reasons.is_empty(),
        complete_without_unbound_root_fields,
        reasons,
    })
}
/// Discover only exact official AST literals belonging to declared Shapes.
/// Unlike the full-copy closure, legal unbound members do not invalidate this
/// limited proof. Their contents are never searched for possible references.
/// Computed/invalid known values remain incomplete; a legal nullable `none`
/// contributes no reference. Source bytes are never changed.
pub fn declared_typed_ref_closure(
    source: &str,
    schema: &Schema,
) -> Result<DeclaredTypedRefClosure> {
    let d = declaration(source, "weftext")?;
    let (_, tokens) = controls(&d)?;
    let (types, validation) = schema.expand_types(&tokens);
    if !validation.is_valid() {
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_projection_type_closure_unavailable(),
        ));
    }
    let projection = project(source, schema);
    let mut unbound = projection
        .unknown_fields
        .iter()
        .map(|field| field.key.clone())
        .collect::<Vec<_>>();
    let mut slots = vec![];
    let mut reasons = vec![];
    for (field, _) in schema.applicable_fields(&types) {
        let Some(node) = root_field(&d, &field.key) else {
            continue;
        };
        let Some(value) = node.value.as_ref() else {
            reasons.push(
                weftext_messages::UiMessage::error_data_projection_field_ref_closure_computed(
                    weftext_messages::params::ErrorDataProjectionFieldRefClosureComputed {
                        field: field.id.clone(),
                    },
                )
                .into(),
            );
            continue;
        };
        if field.nullable && matches!(value, NativeValue::None) {
            continue;
        }
        let validation = schema.validate(&field.shape, value);
        unbound.extend(
            validation
                .unbound
                .iter()
                .map(|path| format!("{}.{path}", field.id)),
        );
        if !validation.is_valid() {
            reasons.push(
                weftext_messages::UiMessage::error_data_projection_field_typed_value_unavailable(
                    weftext_messages::params::ErrorDataProjectionFieldTypedValueUnavailable {
                        field: field.id.clone(),
                    },
                )
                .into(),
            );
            continue;
        }
        ref_walk(
            schema,
            &field.shape,
            node,
            &field.id,
            &mut vec![],
            &mut slots,
        )?;
    }
    Ok(DeclaredTypedRefClosure {
        slots,
        complete: reasons.is_empty(),
        reasons,
        unbound,
    })
}
fn ref_walk(
    schema: &Schema,
    shape: &Shape,
    node: &NativeNode,
    field: &str,
    path: &mut Vec<ValuePath>,
    out: &mut Vec<TypedRefSlot>,
) -> Result<()> {
    if let Some(frame) = &node.frame {
        return ref_walk(schema, shape, frame, field, path, out);
    }
    match schema.resolve(shape) {
        Shape::Scalar { semantic }
            if matches!(semantic.as_str(), "node_ref" | "resource_ref" | "tag_ref") =>
        {
            if let Some(target) = node.value.as_ref().and_then(NativeValue::text) {
                out.push(TypedRefSlot {
                    from: node.range.from,
                    to: node.range.to,
                    target: target.into(),
                    kind: semantic.clone(),
                    field_id: field.into(),
                    path: path.clone(),
                });
            }
        }
        Shape::Record { members, .. } => {
            for (p, n) in &node.children {
                if let ValuePath::Member { key } = p
                    && let Some(m) = members.get(key)
                {
                    path.push(p.clone());
                    ref_walk(schema, &m.shape, n, field, path, out)?;
                    path.pop();
                }
            }
        }
        Shape::List { items, .. } => {
            for (p, n) in &node.children {
                path.push(p.clone());
                ref_walk(schema, items, n, field, path, out)?;
                path.pop();
            }
        }
        Shape::Union {
            variants,
            discriminator,
        } => {
            if let Some(v) = &node.value {
                let selected = schema.union_variant(variants, discriminator.as_ref(), v)?;
                ref_walk(schema, selected, node, field, path, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}
/// Build explicit creation source with Core-supplied identity. No default is
/// fabricated on read, and this function cannot allocate/admit an identity.
/// Inputs are exact FieldIds; native keys are only output addresses.
pub fn capture_source(
    id: &str,
    types: &[String],
    fields: &BTreeMap<String, NativeValue>,
    body: &str,
    schema: &Schema,
) -> Result<String> {
    if !uuid_v4(id) {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_projection_core_allocated_uuid_v4_required(),
        ));
    }
    let (resolved, validation) = schema.expand_types(types);
    if !validation.is_valid() {
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_projection_type_composition_unavailable(),
        ));
    }
    let applicable = schema.applicable_fields(&resolved);
    let mut entries = vec![
        format!("id: {}", typst_string(id)),
        format!(
            "types: {}",
            NativeValue::Array(types.iter().cloned().map(NativeValue::Text).collect())
                .to_typst()?
        ),
    ];
    for key in fields.keys() {
        if !applicable.iter().any(|(f, _)| f.id == *key) {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_projection_capture_field_unbound(
                    weftext_messages::params::ErrorDataProjectionCaptureFieldUnbound {
                        field_key: (key).to_string(),
                    },
                ),
            ));
        }
    }
    for (field, required) in applicable {
        let default = field
            .default
            .as_ref()
            .filter(|d| d["kind"].as_str() == Some("native"))
            .map(|d| {
                crate::decode_native_expression(
                    d["source"]
                        .as_str()
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_projection_native_default_source_missing()))?,
                )
            })
            .transpose()?;
        let value = fields.get(&field.id).or(default.as_ref());
        if let Some(v) = value {
            let check = schema.validate(&field.shape, v);
            if !check.is_valid() && !(field.nullable && matches!(v, NativeValue::None)) {
                let mut diagnostics = check.diagnostics;
                diagnostics.extend(check.unavailable);
                return Err(DataError::new(
                    UiMessage::error_data_projection_capture_field_validation_failed(
                        weftext_messages::params::ErrorDataProjectionCaptureFieldValidationFailed {
                            field: field.key.clone(),
                        },
                    ),
                )
                .with_diagnostics(diagnostics));
            }
            entries.push(format!("{}: {}", typst_string(&field.key), v.to_typst()?));
        } else if required {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_projection_required_capture_field_missing(
                    weftext_messages::params::ErrorDataProjectionRequiredCaptureFieldMissing {
                        field_key: (field.key).to_string(),
                    },
                ),
            ));
        }
    }
    Ok(format!(
        "#metadata(({})) <weftext>\n{}",
        entries.join(", "),
        body
    ))
}
/// Identity-free creation template. Only Core may add and admit the final ID.
pub fn capture_template(
    types: &[String],
    fields: &BTreeMap<String, NativeValue>,
    body: &str,
    schema: &Schema,
) -> Result<String> {
    let validation_id = "00000000-0000-4000-8000-000000000000";
    let source = capture_source(validation_id, types, fields, body, schema)?;
    let prefix = format!("#metadata((id: {}, ", typst_string(validation_id));
    let tail = source.strip_prefix(&prefix).ok_or_else(|| {
        DataError::new(
            weftext_messages::UiMessage::error_data_projection_creation_template_writer_mismatch(),
        )
    })?;
    Ok(format!("#metadata(({tail}"))
}

#[path = "narrow.rs"]
mod narrow;
pub use narrow::*;

#[cfg(test)]
mod reminder_constraint_tests {
    use super::*;
    #[test]
    fn reminder_constraint_rejects_multi_value_selector_instead_of_ignoring_later_input() {
        let schema = Schema::bundled().unwrap();
        let mut constraint = schema
            .types
            .iter()
            .find(|t| t.id == "tasks/task")
            .unwrap()
            .constraints
            .iter()
            .find(|c| c.validator == "temporal/task-reminder")
            .unwrap()
            .clone();
        constraint.inputs.insert(
            "due".into(),
            Selector::Field {
                id: "tasks/due".into(),
                path: vec![SchemaPath::Each],
            },
        );
        let source = "#metadata((id: \"6fc0e4b4-8a30-4dfe-b093-0d334f24d8ad\", types: (\"task\",), status: \"open\")) <weftext>";
        let mut projection = project(source, &schema);
        projection
            .fields
            .iter_mut()
            .find(|f| f.field_id == "tasks/due")
            .unwrap()
            .cell = Cell::Value {
            value: NativeValue::Array(vec![
                NativeValue::Text("2026-10-23T09:00:00Z".into()),
                NativeValue::Text("invalid later due".into()),
            ]),
        };
        let result = validate_type_constraint(
            &constraint,
            &projection,
            &schema,
            &ValidationContext::default(),
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.explanation
                    == UiMessage::error_data_task_reminder_selector_cardinality()),
            "{result:?}"
        );
    }
}
