use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
type SourceSlots = Vec<(ExactRange, String)>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CopyFileKind {
    Source,
    Resource,
    Annotations,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedRefSlot {
    pub from: usize,
    pub to: usize,
    pub target: String,
    pub kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionReferenceSlots {
    pub owner: NodeRef,
    pub key: String,
    pub revision: Option<String>,
    pub owner_slot: ExactRange,
    pub key_slot: ExactRange,
    pub revision_slot: Option<ExactRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedQueryReferenceSlots {
    pub typed_refs: Vec<TypedRefSlot>,
    pub definition_references: Vec<DefinitionReferenceSlots>,
    #[serde(skip)]
    tag_expressions: Vec<(ExactRange, String)>,
    #[serde(skip)]
    tag_defaults: Vec<(ExactRange, String)>,
    #[serde(skip)]
    schema_bindings: Vec<(ExactRange, ExactRange, String, String)>,
}

/// Slot discovery follows the data engine's qualified saved Query structures,
/// then binds those exact controls to official AST literals. CEL expressions,
/// arbitrary records, and URI-looking text never become rewrite slots.
pub fn saved_query_reference_slots(
    source: &str,
    schema: &weftext_data::Schema,
    owner: &NodeRef,
) -> Result<SavedQueryReferenceSlots> {
    let mut slots = saved_query_slots(source, owner, Some(schema))?;
    slots.typed_refs.extend(
        slots
            .tag_defaults
            .drain(..)
            .map(|(range, reference)| TypedRefSlot {
                from: range.from,
                to: range.to,
                target: reference,
                kind: "tag_ref".into(),
            }),
    );
    slots.typed_refs.sort_by_key(|slot| (slot.from, slot.to));
    Ok(slots)
}

fn saved_query_slots(
    source: &str,
    owner: &NodeRef,
    schema: Option<&weftext_data::Schema>,
) -> Result<SavedQueryReferenceSlots> {
    use typst_syntax::{
        Source,
        ast::{self, AstNode},
    };
    let definitions = weftext_data::saved_queries(source)
        .map_err(|e| KnowledgeError::new("definitionQualification", e.to_string()))?;
    let parsed = Source::detached(source);
    let metadata = weftext_engine::inspect_metadata(source)
        .metadata
        .ok_or_else(|| {
            KnowledgeError::new("definitionOwner", "Managed source identity is unavailable")
        })?;
    let identity = metadata
        .fields
        .iter()
        .find(|f| f.key == "id")
        .ok_or_else(|| {
            KnowledgeError::new("definitionOwner", "Managed source has no native identity")
        })?;
    if !verify_string_slot(
        source,
        &ExactRange::new(source, identity.from, identity.to)?,
        &owner.node_id,
    ) {
        return Err(KnowledgeError::new(
            "definitionOwner",
            "Saved references must belong to the exact managed source owner",
        ));
    }
    fn members<'a>(expr: ast::Expr<'a>) -> Result<BTreeMap<String, ast::Expr<'a>>> {
        let dict = expr.to_untyped().cast::<ast::Dict>().ok_or_else(|| {
            KnowledgeError::new(
                "definitionReferenceSlot",
                "An authoritative control must be a direct dictionary",
            )
        })?;
        let mut out = BTreeMap::new();
        for item in dict.items() {
            let (key, value) = match item {
                ast::DictItem::Named(n) => (n.name().get().to_string(), n.expr()),
                ast::DictItem::Keyed(n) => (
                    n.key()
                        .to_untyped()
                        .cast::<ast::Str>()
                        .ok_or_else(|| {
                            KnowledgeError::new("definitionReferenceSlot", "Computed control key")
                        })?
                        .get()
                        .to_string(),
                    n.expr(),
                ),
                _ => {
                    return Err(KnowledgeError::new(
                        "definitionReferenceSlot",
                        "Spread control dictionary",
                    ));
                }
            };
            if out.insert(key, value).is_some() {
                return Err(KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Duplicate control key",
                ));
            }
        }
        Ok(out)
    }
    fn member<'a>(members: &BTreeMap<String, ast::Expr<'a>>, key: &str) -> Result<ast::Expr<'a>> {
        members.get(key).copied().ok_or_else(|| {
            KnowledgeError::new(
                "definitionReferenceSlot",
                format!("Missing authoritative control {key}"),
            )
        })
    }
    fn text_slot(
        expr: ast::Expr<'_>,
        expected: &str,
        parsed: &Source,
        source: &str,
    ) -> Result<ExactRange> {
        if !expr
            .to_untyped()
            .cast::<ast::Str>()
            .is_some_and(|s| s.get() == expected)
        {
            return Err(KnowledgeError::new(
                "definitionReferenceSlot",
                "Control must match one exact native string literal",
            ));
        }
        let range = parsed
            .find(expr.to_untyped().span())
            .ok_or_else(|| {
                KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Control has no exact source range",
                )
            })?
            .range();
        ExactRange::new(source, range.start, range.end)
    }
    fn node_ref(value: &str) -> Option<NodeRef> {
        let normalized = crate::links::normalized_uri_scheme(value);
        let rest = normalized.strip_prefix("weftext-node://")?;
        let parts: Vec<_> = rest.split('/').collect();
        let [workspace, node] = parts.as_slice() else {
            return None;
        };
        (uuid::Uuid::parse_str(workspace).is_ok() && uuid::Uuid::parse_str(node).is_ok()).then(
            || NodeRef {
                workspace_id: (*workspace).into(),
                node_id: (*node).into(),
            },
        )
    }
    fn visit(
        plan: &weftext_data::QueryPlan,
        expr: ast::Expr<'_>,
        parsed: &Source,
        source: &str,
        out: &mut SavedQueryReferenceSlots,
        depth: usize,
        schema: Option<&weftext_data::Schema>,
    ) -> Result<()> {
        if depth > 64 {
            return Err(KnowledgeError::new(
                "definitionReferenceBudget",
                "Saved Query traversal exceeds the explicit depth budget",
            ));
        }
        let fields = members(expr)?;
        fn array(expr: ast::Expr<'_>) -> Result<Vec<ast::Expr<'_>>> {
            expr.to_untyped()
                .cast::<ast::Array>()
                .ok_or_else(|| {
                    KnowledgeError::new(
                        "definitionReferenceSlot",
                        "A Query array must have exact native members",
                    )
                })?
                .items()
                .map(|item| match item {
                    ast::ArrayItem::Pos(expr) => Ok(expr),
                    _ => Err(KnowledgeError::new(
                        "definitionReferenceSlot",
                        "Spread Query array",
                    )),
                })
                .collect()
        }
        fn expression(
            expr: ast::Expr<'_>,
            expected: &str,
            parsed: &Source,
            source: &str,
            out: &mut SavedQueryReferenceSlots,
        ) -> Result<()> {
            out.tag_expressions
                .push((text_slot(expr, expected, parsed, source)?, expected.into()));
            Ok(())
        }
        fn columns(
            expr: ast::Expr<'_>,
            columns: &[weftext_data::QueryColumn],
            parsed: &Source,
            source: &str,
            out: &mut SavedQueryReferenceSlots,
        ) -> Result<()> {
            let entries = array(expr)?;
            if entries.len() != columns.len() {
                return Err(KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Query columns no longer match their exact plan",
                ));
            }
            for (expr, column) in entries.into_iter().zip(columns) {
                expression(
                    member(&members(expr)?, "expr")?,
                    &column.expr,
                    parsed,
                    source,
                    out,
                )?;
            }
            Ok(())
        }
        struct DefaultContext<'a> {
            schema: &'a weftext_data::Schema,
            parsed: &'a Source,
            source: &'a str,
        }
        fn defaults(
            shape: &weftext_data::Shape,
            value: &weftext_data::NativeValue,
            expr: ast::Expr<'_>,
            context: &DefaultContext<'_>,
            out: &mut SavedQueryReferenceSlots,
            depth: usize,
        ) -> Result<()> {
            if depth > 64 {
                return Err(KnowledgeError::new(
                    "definitionReferenceBudget",
                    "Query default shape exceeds depth budget",
                ));
            }
            use weftext_data::{NativeValue, Shape};
            let DefaultContext {
                schema,
                parsed,
                source,
            } = context;
            fn contains_tag(shape: &Shape, schema: &weftext_data::Schema, depth: usize) -> bool {
                if depth > 64 {
                    return true;
                }
                match schema.resolve(shape) {
                    Shape::Scalar { semantic } => semantic == "tag_ref",
                    Shape::List { items, .. } => contains_tag(items, schema, depth + 1),
                    Shape::Record { members, .. } => members
                        .values()
                        .any(|member| contains_tag(&member.shape, schema, depth + 1)),
                    Shape::Union { variants, .. } => variants
                        .iter()
                        .any(|variant| contains_tag(variant, schema, depth + 1)),
                    _ => false,
                }
            }
            if !contains_tag(shape, schema, depth) {
                return Ok(());
            }
            match (schema.resolve(shape), value) {
                (Shape::Scalar { semantic }, NativeValue::Text(reference))
                    if semantic == "tag_ref" =>
                {
                    out.tag_defaults.push((
                        text_slot(expr, reference, parsed, source)?,
                        reference.clone(),
                    ));
                }
                (Shape::List { items, .. }, NativeValue::Array(values)) => {
                    let entries = array(expr)?;
                    if entries.len() != values.len() {
                        return Err(KnowledgeError::new(
                            "definitionReferenceSlot",
                            "Query list default differs from its exact native value",
                        ));
                    }
                    for (expr, value) in entries.into_iter().zip(values) {
                        defaults(items, value, expr, context, out, depth + 1)?;
                    }
                }
                (
                    Shape::Record {
                        members: shapes, ..
                    },
                    NativeValue::Record(values),
                ) => {
                    let fields = members(expr)?;
                    for value in values {
                        if let Some(member_shape) = shapes.get(&value.key) {
                            defaults(
                                &member_shape.shape,
                                &value.value,
                                member(&fields, &value.key)?,
                                context,
                                out,
                                depth + 1,
                            )?;
                        }
                    }
                }
                (Shape::Union { variants, .. }, _) => {
                    let choices: Vec<_> = variants
                        .iter()
                        .filter(|variant| schema.validate(variant, value).is_valid())
                        .collect();
                    if choices.len() != 1 {
                        return Err(KnowledgeError::new(
                            "definitionReferenceSlot",
                            "A Query default requires one unambiguous typed union variant",
                        ));
                    }
                    defaults(choices[0], value, expr, context, out, depth + 1)?;
                }
                _ => {}
            }
            Ok(())
        }
        if let Some(filter) = &plan.r#where {
            expression(member(&fields, "where")?, filter, parsed, source, out)?;
        }
        if !plan.select.is_empty() {
            columns(
                member(&fields, "select")?,
                &plan.select,
                parsed,
                source,
                out,
            )?;
        }
        if !plan.order.is_empty() {
            let orders = array(member(&fields, "order")?)?;
            if orders.len() != plan.order.len() {
                return Err(KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Query order no longer matches its exact plan",
                ));
            }
            for (expr, order) in orders.into_iter().zip(&plan.order) {
                expression(
                    member(&members(expr)?, "expr")?,
                    &order.expr,
                    parsed,
                    source,
                    out,
                )?;
            }
        }
        if let Some(unnest) = &plan.unnest {
            expression(
                member(&members(member(&fields, "unnest")?)?, "array")?,
                &unnest.array,
                parsed,
                source,
                out,
            )?;
        }
        if let Some(group) = &plan.group {
            let group_fields = members(member(&fields, "group")?)?;
            if !group.keys.is_empty() {
                columns(
                    member(&group_fields, "keys")?,
                    &group.keys,
                    parsed,
                    source,
                    out,
                )?;
            }
            let aggregates = array(member(&group_fields, "aggregates")?)?;
            if aggregates.len() != group.aggregates.len() {
                return Err(KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Query aggregates no longer match their exact plan",
                ));
            }
            for (expr, aggregate) in aggregates.into_iter().zip(&group.aggregates) {
                if let Some(expected) = &aggregate.expr {
                    expression(
                        member(&members(expr)?, "expr")?,
                        expected,
                        parsed,
                        source,
                        out,
                    )?;
                }
            }
        }
        if let Some(directory) = &plan.scope.directory
            && node_ref(directory).is_some()
        {
            let scope = members(member(&fields, "scope")?)?;
            let range = text_slot(member(&scope, "directory")?, directory, parsed, source)?;
            out.typed_refs.push(TypedRefSlot {
                from: range.from,
                to: range.to,
                target: directory.clone(),
                kind: "node_ref".into(),
            });
        }
        let Some(advanced) = &plan.advanced else {
            return Ok(());
        };
        let fields = members(member(&fields, "advanced")?)?;
        if let Some(distinct) = &advanced.distinct {
            columns(member(&fields, "distinct")?, distinct, parsed, source, out)?;
        }
        if !advanced.parameters.is_empty() {
            let parameters = array(member(&fields, "parameters")?)?;
            if parameters.len() != advanced.parameters.len() {
                return Err(KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Query parameters no longer match their exact plan",
                ));
            }
            for (expr, parameter) in parameters.into_iter().zip(&advanced.parameters) {
                if let (Some(schema), Some(value)) = (schema, &parameter.default) {
                    defaults(
                        &parameter.shape,
                        value,
                        member(&members(expr)?, "default")?,
                        &DefaultContext {
                            schema,
                            parsed,
                            source,
                        },
                        out,
                        0,
                    )?;
                }
            }
        }
        if let Some(call) = &advanced.call {
            let controls = members(member(&fields, "call")?)?;
            if !call.arguments.is_empty() {
                let arguments = members(member(&controls, "arguments")?)?;
                for (name, expected) in &call.arguments {
                    expression(member(&arguments, name)?, expected, parsed, source, out)?;
                }
            }
            let referenced_owner = node_ref(&call.owner).ok_or_else(|| {
                KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Saved-call owner is not a full typed NodeRef",
                )
            })?;
            let owner_slot = text_slot(member(&controls, "owner")?, &call.owner, parsed, source)?;
            let selection = members(member(&controls, "selection")?)?;
            let (key, revision) = match &call.selection {
                weftext_data::DefinitionSelection::Current { key } => (key, None),
                weftext_data::DefinitionSelection::Pinned { key, revision } => {
                    (key, Some(revision))
                }
            };
            let key_slot = text_slot(member(&selection, "key")?, key, parsed, source)?;
            let revision_slot = revision
                .map(|r| text_slot(member(&selection, "revision")?, r, parsed, source))
                .transpose()?;
            out.typed_refs.push(TypedRefSlot {
                from: owner_slot.from,
                to: owner_slot.to,
                target: call.owner.clone(),
                kind: "node_ref".into(),
            });
            out.definition_references.push(DefinitionReferenceSlots {
                owner: referenced_owner,
                key: key.clone(),
                revision: revision.cloned(),
                owner_slot,
                key_slot,
                revision_slot,
            });
        }
        if !advanced.union_all.is_empty() {
            let array = member(&fields, "unionAll")?
                .to_untyped()
                .cast::<ast::Array>()
                .ok_or_else(|| {
                    KnowledgeError::new(
                        "definitionReferenceSlot",
                        "Union producer must be a direct array",
                    )
                })?;
            let branches: Vec<_> = array
                .items()
                .map(|item| match item {
                    ast::ArrayItem::Pos(expr) => Ok(expr),
                    _ => Err(KnowledgeError::new(
                        "definitionReferenceSlot",
                        "Spread producer array",
                    )),
                })
                .collect::<Result<_>>()?;
            if branches.len() != advanced.union_all.len() {
                return Err(KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Union source no longer matches its parsed plan",
                ));
            }
            for (plan, expr) in advanced.union_all.iter().zip(branches) {
                visit(plan, expr, parsed, source, out, depth + 1, schema)?;
            }
        }
        if let Some(related) = &advanced.related {
            let related_fields = members(member(&fields, "related")?)?;
            visit(
                &related.from,
                member(&related_fields, "from")?,
                parsed,
                source,
                out,
                depth + 1,
                schema,
            )?;
        }
        Ok(())
    }
    let mut out = SavedQueryReferenceSlots {
        typed_refs: vec![],
        definition_references: vec![],
        tag_expressions: vec![],
        tag_defaults: vec![],
        schema_bindings: vec![],
    };
    for definition in definitions {
        let certified = weftext_engine::certify_metadata_declaration(source, &definition.label)
            .ok_or_else(|| {
                KnowledgeError::new(
                    "definitionQualification",
                    "Saved Query declaration is not direct standard native metadata",
                )
            })?;
        let call = parsed
            .root()
            .children()
            .filter_map(|n| n.cast::<ast::FuncCall>())
            .find(|call| {
                parsed
                    .find(call.to_untyped().span())
                    .is_some_and(|n| n.range().start == certified.from)
            })
            .ok_or_else(|| {
                KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Saved Query declaration has no exact native call",
                )
            })?;
        let expr = call
            .args()
            .items()
            .find_map(|arg| {
                if let ast::Arg::Pos(expr) = arg {
                    Some(expr)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                KnowledgeError::new(
                    "definitionReferenceSlot",
                    "Saved Query declaration has no exact dictionary",
                )
            })?;
        let binding = members(member(&members(expr)?, "binding")?)?;
        out.schema_bindings.push((
            text_slot(
                member(&binding, "schema")?,
                &definition.binding.schema,
                &parsed,
                source,
            )?,
            text_slot(
                member(&binding, "revision")?,
                &definition.binding.revision,
                &parsed,
                source,
            )?,
            definition.binding.schema.clone(),
            definition.binding.revision.clone(),
        ));
        visit(&definition.plan, expr, &parsed, source, &mut out, 0, schema)?;
    }
    out.typed_refs.sort_by_key(|s| (s.from, s.to));
    out.definition_references.sort_by_key(|s| s.owner_slot.from);
    Ok(out)
}

pub fn definition_reference_slots(
    source: &str,
    schema: &weftext_data::Schema,
    owner: &NodeRef,
) -> Result<Vec<DefinitionReferenceSlots>> {
    Ok(saved_query_reference_slots(source, schema, owner)?.definition_references)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyFile {
    pub source_path: String,
    pub target_path: String,
    pub before: Vec<u8>,
    pub kind: CopyFileKind,
    pub owner: Option<NodeRef>,
    #[serde(default)]
    pub typed_refs: Vec<TypedRefSlot>,
    /// Schema-qualified current/pinned definition reference records, including
    /// actual owner/key slots. Arbitrary matching strings never enter this list.
    #[serde(default)]
    pub definition_references: Vec<DefinitionReferenceSlots>,
    /// The host has installed the schema and enumerated every typed reference
    /// slot. A mere absence of slots never certifies this flag.
    pub typed_refs_complete: bool,
}
#[derive(Debug, Clone)]
pub struct CopyInput {
    pub files: Vec<CopyFile>,
    pub node_map: BTreeMap<NodeRef, NodeRef>,
    pub resource_map: BTreeMap<String, String>,
    /// Complete typed TagRef keys. Empty for same-library copies; a foreign
    /// import must supply mappings created by the target Core operation.
    pub tag_map: BTreeMap<String, String>,
    /// Exact source schema role/revision to an equivalent frozen target basis.
    pub schema_map: BTreeMap<(String, String), (String, String)>,
    pub annotation_map: BTreeMap<String, String>,
    pub definition_map: BTreeMap<String, String>,
    pub revision_map: BTreeMap<NodeRef, String>,
    pub resource_revision_map: BTreeMap<String, String>,
    pub inventory: InventorySnapshot,
    pub allow_losses: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreservedOriginal {
    pub source_path: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyPlan {
    pub files: Vec<PlannedFile>,
    pub losses: Vec<Loss>,
    pub preserved_originals: Vec<PreservedOriginal>,
}

pub fn definition_identity(owner: &NodeRef, key: &str) -> String {
    format!("{}/{}/{}", owner.workspace_id, owner.node_id, key)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionSlot {
    pub identity: String,
    pub key: String,
    pub range: ExactRange,
}
/// Native direct metadata declarations with complete static trees. Unknown
/// application kinds/computed definitions reject complete copy qualification.
pub fn definition_slots(source: &str, owner: &NodeRef) -> Result<Vec<DefinitionSlot>> {
    use typst_syntax::{
        LinkedNode,
        ast::{self, AstNode},
    };
    let parsed = typst_syntax::Source::detached(source);
    let qualified = weftext_data::saved_queries(source)
        .map_err(|e| KnowledgeError::new("definitionQualification", e.to_string()))?;
    fn static_node(node: &typst_syntax::SyntaxNode) -> bool {
        match node.kind() {
            typst_syntax::SyntaxKind::Str
            | typst_syntax::SyntaxKind::Int
            | typst_syntax::SyntaxKind::Float
            | typst_syntax::SyntaxKind::Bool
            | typst_syntax::SyntaxKind::None => true,
            typst_syntax::SyntaxKind::Array => node.cast::<ast::Array>().is_some_and(|a| {
                a.items()
                    .all(|i| matches!(i,ast::ArrayItem::Pos(e) if static_node(e.to_untyped())))
            }),
            typst_syntax::SyntaxKind::Dict => node.cast::<ast::Dict>().is_some_and(|d| {
                d.items().all(|i| match i {
                    ast::DictItem::Named(n) => static_node(n.expr().to_untyped()),
                    ast::DictItem::Keyed(n) => {
                        n.key().to_untyped().cast::<ast::Str>().is_some()
                            && static_node(n.expr().to_untyped())
                    }
                    _ => false,
                })
            }),
            _ => false,
        }
    }
    let root = LinkedNode::new(parsed.root());
    let children: Vec<_> = root.children().collect();
    let mut slots = vec![];
    let mut keys = BTreeSet::new();
    let mut labels = BTreeSet::new();
    for (index, node) in children.iter().enumerate() {
        let Some(call) = node.get().cast::<ast::FuncCall>() else {
            continue;
        };
        if !call
            .callee()
            .to_untyped()
            .cast::<ast::Ident>()
            .is_some_and(|i| i.get() == "metadata")
        {
            continue;
        }
        let args: Vec<_> = call.args().items().collect();
        let [ast::Arg::Pos(expr)] = args.as_slice() else {
            continue;
        };
        let Some(dict) = expr.to_untyped().cast::<ast::Dict>() else {
            continue;
        };
        let mut fields = BTreeMap::new();
        for item in dict.items() {
            let (key, expr) = match item {
                ast::DictItem::Named(n) => (n.name().get().to_string(), n.expr()),
                ast::DictItem::Keyed(n) => {
                    let Some(key) = n.key().to_untyped().cast::<ast::Str>() else {
                        return Err(KnowledgeError::new(
                            "unknownDefinition",
                            "Dynamic application dictionary key",
                        ));
                    };
                    (key.get().to_string(), n.expr())
                }
                _ => {
                    return Err(KnowledgeError::new(
                        "unknownDefinition",
                        "Spread application dictionary",
                    ));
                }
            };
            if fields.insert(key, expr).is_some() {
                return Err(KnowledgeError::new(
                    "unknownDefinition",
                    "Duplicate application dictionary key",
                ));
            }
        }
        let Some(kind) = fields
            .get("kind")
            .and_then(|e| e.to_untyped().cast::<ast::Str>())
            .map(|s| s.get().to_string())
        else {
            continue;
        };
        if !kind.starts_with("weftext.") {
            continue;
        }
        if kind == "weftext.citations" {
            continue;
        }
        if !weftext_engine::standard_binding_available(source, "metadata") {
            return Err(KnowledgeError::new(
                "definitionBinding",
                "Application definition binding is not proven",
            ));
        }
        if kind != "weftext.query" || !static_node(dict.to_untyped()) {
            return Err(KnowledgeError::new(
                "unknownDefinition",
                "Unknown or computed application definition",
            ));
        }
        let key_expr = fields
            .get("key")
            .ok_or_else(|| KnowledgeError::new("definitionKey", "Definition key is missing"))?;
        let key = key_expr
            .to_untyped()
            .cast::<ast::Str>()
            .ok_or_else(|| {
                KnowledgeError::new("definitionKey", "Definition key is not a native literal")
            })?
            .get()
            .to_string();
        if uuid::Uuid::parse_str(&key).is_err() || !keys.insert(key.clone()) {
            return Err(KnowledgeError::new(
                "definitionKey",
                "Invalid or duplicate definition key",
            ));
        }
        let label = children
            .iter()
            .skip(index + 1)
            .find(|n| !n.kind().is_trivia())
            .and_then(|n| n.get().cast::<ast::Label>())
            .ok_or_else(|| {
                KnowledgeError::new(
                    "definitionLabel",
                    "Direct application definition label is missing",
                )
            })?
            .get()
            .to_string();
        if !label.starts_with("weftext-query-") || !labels.insert(label) {
            return Err(KnowledgeError::new(
                "definitionLabel",
                "Invalid or duplicate application definition label",
            ));
        }
        if !qualified.iter().any(|definition| definition.key == key) {
            return Err(KnowledgeError::new(
                "definitionQualification",
                "Definition is not a qualified closed native saved query",
            ));
        }
        let range = parsed
            .find(key_expr.to_untyped().span())
            .ok_or_else(|| {
                KnowledgeError::new("definitionSlot", "Definition key has no exact source slot")
            })?
            .range();
        slots.push(DefinitionSlot {
            identity: definition_identity(owner, &key),
            key,
            range: ExactRange::new(source, range.start, range.end)?,
        });
    }
    Ok(slots)
}

fn encode_string(value: &str) -> Result<String> {
    weftext_engine::encode_string(value).map_err(|e| KnowledgeError::new("encoding", e.to_string()))
}
fn fresh_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .is_ok_and(|id| id.get_version_num() == 4 && id.to_string() == value)
}
fn file_url(path: &str) -> Option<String> {
    if path.starts_with("file:") {
        url::Url::parse(path).ok().map(|u| u.to_string())
    } else {
        url::Url::from_file_path(path).ok().map(|u| u.to_string())
    }
}
fn verify_string_slot(source: &str, range: &ExactRange, expected: &str) -> bool {
    if !range.verify(source) {
        return false;
    }
    let parsed = typst_syntax::Source::detached(source);
    fn walk(node: &typst_syntax::LinkedNode<'_>, range: &ExactRange, expected: &str) -> bool {
        node.range() == (range.from..range.to)
            && node
                .get()
                .cast::<typst_syntax::ast::Str>()
                .is_some_and(|s| s.get() == expected)
            || node.children().any(|child| walk(&child, range, expected))
    }
    walk(
        &typst_syntax::LinkedNode::new(parsed.root()),
        range,
        expected,
    )
}

fn verify_node_reference_slot(source: &str, range: &ExactRange, expected: &str) -> bool {
    let Ok(weftext_data::NativeValue::Text(value)) =
        weftext_data::decode_native_expression(&range.raw)
    else {
        return false;
    };
    crate::links::normalized_uri_scheme(&value) == expected
        && verify_string_slot(source, range, &value)
}

fn native_resource_url(source: &NodeSnapshot, path: &str) -> Option<String> {
    use std::path::{Component, Path, PathBuf};
    if path.is_empty()
        || path.contains(':')
        || path.contains('\\')
        || path.chars().any(char::is_control)
    {
        return None;
    }
    let source_file = url::Url::parse(&source.path).ok()?.to_file_path().ok()?;
    let mut base = source_file.parent()?.to_path_buf();
    if path.starts_with('/') {
        base = source_file.clone();
        for _ in source.library_path.split('/') {
            base.pop();
        }
    }
    let joined = base.join(path.trim_start_matches('/'));
    let mut normalized = PathBuf::new();
    for component in Path::new(&joined).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    url::Url::from_file_path(normalized)
        .ok()
        .map(|u| u.to_string())
}
fn native_relative_file(source_target: &str, resource_target: &str) -> Option<String> {
    let source = url::Url::parse(&file_url(source_target)?)
        .ok()?
        .to_file_path()
        .ok()?;
    let target = url::Url::parse(&file_url(resource_target)?)
        .ok()?
        .to_file_path()
        .ok()?;
    let from: Vec<_> = source.parent()?.components().collect();
    let to: Vec<_> = target.components().collect();
    if from.first() != to.first() {
        return None;
    }
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut result: Vec<String> = (common..from.len()).map(|_| "..".into()).collect();
    result.extend(
        to[common..]
            .iter()
            .map(|c| c.as_os_str().to_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?,
    );
    Some(result.join("/"))
}
fn copy_body_references(
    source: &NodeSnapshot,
    file: &CopyFile,
    files: &[CopyFile],
    inventory: &InventorySnapshot,
) -> Result<(SourceSlots, Vec<Loss>)> {
    use typst_syntax::{
        LinkedNode, SyntaxKind,
        ast::{self, AstNode},
    };
    let parsed = typst_syntax::Source::detached(&source.source);
    let native = native_links(&source.source);
    let bindings: BTreeMap<_, _> = [
        "image",
        "read",
        "csv",
        "json",
        "yaml",
        "toml",
        "xml",
        "cbor",
        "bibliography",
    ]
    .into_iter()
    .map(|name| {
        (
            name.to_string(),
            weftext_engine::standard_binding_available(&source.source, name),
        )
    })
    .collect();
    let mut patches = vec![];
    let mut losses = vec![];
    fn walk(
        node: &LinkedNode<'_>,
        source: &NodeSnapshot,
        file: &CopyFile,
        context: (
            &[CopyFile],
            &InventorySnapshot,
            &[NativeLink],
            &BTreeMap<String, bool>,
        ),
        patches: &mut Vec<(ExactRange, String)>,
        losses: &mut Vec<Loss>,
    ) -> Result<()> {
        let (files, inventory, native, bindings) = context;
        let mut lose = |code: &str, detail: &str| {
            losses.push(Loss {
                path: file.source_path.clone(),
                code: code.into(),
                detail: detail.into(),
            })
        };
        if matches!(
            node.kind(),
            SyntaxKind::ModuleInclude | SyntaxKind::ModuleImport
        ) {
            lose(
                "unqualifiedIncludeImport",
                "Native include/import needs a qualified complete dependency map; its original source is preserved",
            );
        }
        if let Some(binding) = node.get().cast::<ast::LetBinding>()
            && let Some(init) = binding.init()
            && init.to_untyped().cast::<ast::Ident>().is_some_and(|i| {
                matches!(
                    i.get().as_str(),
                    "link"
                        | "image"
                        | "read"
                        | "csv"
                        | "json"
                        | "yaml"
                        | "toml"
                        | "xml"
                        | "cbor"
                        | "bibliography"
                )
            })
        {
            lose(
                "aliasedNativeReference",
                "A function-valued native link/resource alias has no complete qualified destination map; original source is preserved",
            );
        }
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && call.callee().to_untyped().cast::<ast::Ident>().is_none()
        {
            fn mentions_reference(node: &typst_syntax::SyntaxNode) -> bool {
                node.cast::<ast::Ident>().is_some_and(|i| {
                    matches!(
                        i.get().as_str(),
                        "link"
                            | "image"
                            | "read"
                            | "csv"
                            | "json"
                            | "yaml"
                            | "toml"
                            | "xml"
                            | "cbor"
                            | "bibliography"
                    )
                }) || node.children().any(mentions_reference)
            }
            if mentions_reference(call.callee().to_untyped()) {
                lose(
                    "indirectNativeReference",
                    "Indirect native link/resource calls have no exact destination-slot proof; original source is preserved",
                );
            }
        }
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && let Some(name) = call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .map(|i| i.get().to_string())
        {
            if name == "link" && !native.iter().any(|link| link.from == node.range().start) {
                lose(
                    "unmappedLinkDestination",
                    "Computed or unverified link syntax has no proven copy destination slot; original bytes remain unchanged",
                );
            }
            if matches!(
                name.as_str(),
                "image"
                    | "read"
                    | "csv"
                    | "json"
                    | "yaml"
                    | "toml"
                    | "xml"
                    | "cbor"
                    | "bibliography"
            ) {
                if bindings.get(&name) != Some(&true) {
                    lose(
                        "unverifiedResourceCall",
                        "Native resource function binding is not proven",
                    );
                } else if let Some(expr) = call.args().items().find_map(|arg| {
                    if let ast::Arg::Pos(expr) = arg {
                        Some(expr)
                    } else {
                        None
                    }
                }) {
                    if let Some(path) = expr.to_untyped().cast::<ast::Str>() {
                        let old = native_resource_url(source, &path.get());
                        if let Some(copied) = files.iter().find(|candidate| {
                            file_url(&candidate.source_path) == old && old.is_some()
                        }) {
                            let address=native_relative_file(&file.target_path,&copied.target_path).ok_or_else(||KnowledgeError::new("resourcePath","A copied native resource cannot round-trip between these volumes"))?;
                            let slot = node
                                .find(expr.to_untyped().span())
                                .ok_or_else(|| {
                                    KnowledgeError::new(
                                        "resourceSlot",
                                        "Native resource path has no exact AST slot",
                                    )
                                })?
                                .range();
                            patches.push((
                                ExactRange::new(&source.source, slot.start, slot.end)?,
                                encode_string(&address)?,
                            ));
                        } else if let Some(resource) = inventory
                            .resources
                            .iter()
                            .find(|resource| Some(&resource.path) == old.as_ref())
                        {
                            lose(
                                "externalResource",
                                "Native resource remains an explicit external dependency because its bytes were not re-owned in the selection",
                            );
                            if let Some(address) =
                                native_relative_file(&file.target_path, &resource.path)
                            {
                                let slot = node.find(expr.to_untyped().span()).unwrap().range();
                                patches.push((
                                    ExactRange::new(&source.source, slot.start, slot.end)?,
                                    encode_string(&address)?,
                                ));
                            }
                        } else {
                            lose(
                                "unresolvedResource",
                                "Native resource path is outside the qualified copy closure; original bytes remain unchanged",
                            );
                        }
                    } else {
                        lose(
                            "computedResource",
                            "Computed resource input lacks a complete exact dependency map; original bytes remain unchanged",
                        );
                    }
                }
            }
        }
        for child in node.children() {
            walk(&child, source, file, context, patches, losses)?;
        }
        Ok(())
    }
    walk(
        &LinkedNode::new(parsed.root()),
        source,
        file,
        (files, inventory, &native, &bindings),
        &mut patches,
        &mut losses,
    )?;
    Ok((patches, losses))
}
fn apply_slots(source: &str, mut patches: Vec<(ExactRange, String)>) -> Result<String> {
    patches.sort_by_key(|(range, _)| (range.from, range.to));
    patches.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    for pair in patches.windows(2) {
        if pair[0].0.to > pair[1].0.from {
            return Err(KnowledgeError::new(
                "overlappingSlots",
                "Copy slots overlap; do not guess which mapping wins",
            ));
        }
    }
    let mut after = source.to_string();
    for (range, replacement) in patches.into_iter().rev() {
        if !range.verify(source) {
            return Err(KnowledgeError::new(
                "staleSlot",
                "A copy source slot does not match the frozen exact bytes",
            ));
        }
        after.replace_range(range.from..range.to, &replacement);
    }
    if !typst_syntax::Source::detached(&after)
        .root()
        .errors_and_warnings()
        .0
        .is_empty()
    {
        return Err(KnowledgeError::new(
            "invalidSource",
            "Rewritten copy does not parse as native Typst",
        ));
    }
    Ok(after)
}
fn remap_target(
    mut target: AnnotationTarget,
    nodes: &BTreeMap<NodeRef, NodeRef>,
    resources: &BTreeMap<String, String>,
    revisions: &BTreeMap<String, String>,
) -> AnnotationTarget {
    match &mut target {
        AnnotationTarget::WholeDocument { node } => {
            if let Some(mapped) = nodes.get(node) {
                *node = mapped.clone();
            }
        }
        AnnotationTarget::SourceRange { .. } | AnnotationTarget::SourceElement { .. } => {}
        AnnotationTarget::WholeResource {
            resource,
            revision_id,
        }
        | AnnotationTarget::PdfRegion {
            resource,
            revision_id,
            ..
        }
        | AnnotationTarget::PdfTextRange {
            resource,
            revision_id,
            ..
        }
        | AnnotationTarget::ImageRegion {
            resource,
            revision_id,
            ..
        }
        | AnnotationTarget::MediaInterval {
            resource,
            revision_id,
            ..
        } => {
            if let Some(mapped) = resources.get(resource) {
                if let Some(revision) = revisions.get(resource) {
                    *revision_id = revision.clone();
                }
                *resource = mapped.clone();
            }
        }
    }
    target
}

/// Allocations are supplied once by Core, in ordered maps. This planner never
/// allocates IDs, writes files, or modifies arbitrary UUID-like strings.
pub fn plan_copy(input: CopyInput) -> Result<CopyPlan> {
    let mut target_paths = BTreeSet::new();
    let mut source_owners = BTreeSet::new();
    for file in &input.files {
        if file.target_path == file.source_path || !target_paths.insert(&file.target_path) {
            return Err(KnowledgeError::new(
                "copyDestination",
                "Independent copies require unique new destinations",
            ));
        }
        if matches!(file.kind, CopyFileKind::Source)
            && file
                .owner
                .as_ref()
                .is_some_and(|owner| !source_owners.insert(owner))
        {
            return Err(KnowledgeError::new(
                "copyInstances",
                "One source cannot be copied twice through one node mapping; use separate full source/result maps",
            ));
        }
    }
    let fresh_ids: BTreeSet<_> = input.node_map.values().collect();
    if fresh_ids.len() != input.node_map.len()
        || input
            .node_map
            .values()
            .any(|node| !fresh_uuid(&node.workspace_id) || !fresh_uuid(&node.node_id))
        || input
            .node_map
            .keys()
            .any(|owner| !source_owners.contains(owner))
        || input.node_map.iter().any(|(old, new)| {
            old == new || input.inventory.nodes.iter().any(|node| &node.owner == new)
        })
    {
        return Err(KnowledgeError::new(
            "copyIdentity",
            "Every copied note must receive one independent fresh identity",
        ));
    }
    fn tag_identity(reference: &str) -> Option<(&str, &str)> {
        let (workspace, id) = reference.strip_prefix("weftext-tag://")?.split_once('/')?;
        (fresh_uuid(workspace) && fresh_uuid(id)).then_some((workspace, id))
    }
    let mut mapped_tags = BTreeSet::new();
    for (source, target) in &input.tag_map {
        if tag_identity(source).is_none()
            || tag_identity(target).is_none()
            || source == target
            || !mapped_tags.insert(target)
            || input.tag_map.contains_key(target)
        {
            return Err(KnowledgeError::new(
                "tagMap",
                "Tag mappings require complete typed references and distinct target identities",
            ));
        }
    }
    let resource_ids: BTreeSet<_> = input.resource_map.values().collect();
    if resource_ids.len() != input.resource_map.len()
        || input
            .resource_map
            .iter()
            .any(|(old, new)| old == new || input.resource_map.contains_key(new))
    {
        return Err(KnowledgeError::new(
            "copyIdentity",
            "Each copied resource requires a fresh distinct identity",
        ));
    }
    for old in input.resource_map.keys() {
        let resource = input
            .inventory
            .resources
            .iter()
            .find(|r| &r.key == old)
            .ok_or_else(|| {
                KnowledgeError::new(
                    "resourceClosure",
                    "A mapped resource is not in the authorized inventory",
                )
            })?;
        if !input.files.iter().any(|f| {
            matches!(f.kind, CopyFileKind::Resource)
                && file_url(&f.source_path).as_deref() == Some(resource.path.as_str())
        }) {
            return Err(KnowledgeError::new(
                "resourceClosure",
                "Mapped resource bytes are missing from the complete selection",
            ));
        }
    }
    let definition_keys: BTreeSet<_> = input.definition_map.values().collect();
    if definition_keys.len() != input.definition_map.len()
        || input.definition_map.values().any(|new| {
            input
                .definition_map
                .keys()
                .any(|old| old.rsplit('/').next() == Some(new.as_str()))
        })
    {
        return Err(KnowledgeError::new(
            "definitionMap",
            "Copied definitions require distinct fresh UUID keys",
        ));
    }
    let annotation_ids: BTreeSet<_> = input.annotation_map.values().collect();
    if annotation_ids.len() != input.annotation_map.len()
        || input
            .annotation_map
            .iter()
            .any(|(old, new)| old == new || input.annotation_map.contains_key(new))
    {
        return Err(KnowledgeError::new(
            "copyIdentity",
            "Annotation IDs must be fresh and unique",
        ));
    }
    let mut plan = CopyPlan {
        files: vec![],
        losses: vec![],
        preserved_originals: vec![],
    };
    let mut source_edits: BTreeMap<NodeRef, Vec<(ExactRange, String)>> = BTreeMap::new();
    let mut source_after: BTreeMap<NodeRef, String> = BTreeMap::new();
    // All source rewrites precede annotation relocation, independently of input
    // array order. Files in the result are sorted by target path afterwards.
    let mut ordered: Vec<_> = input.files.iter().collect();
    ordered.sort_by_key(|file| {
        (
            matches!(file.kind, CopyFileKind::Annotations),
            file.target_path.clone(),
        )
    });
    for file in ordered {
        let mut lose = |code: &str, detail: &str| {
            plan.losses.push(Loss {
                path: file.source_path.clone(),
                code: code.into(),
                detail: detail.into(),
            });
        };
        let after = match file.kind {
            CopyFileKind::Resource => file.before.clone(),
            CopyFileKind::Source => {
                let source = std::str::from_utf8(&file.before).map_err(|_| {
                    KnowledgeError::new("encoding", "Copy source is not UTF-8; retain the original")
                })?;
                let owner = file.owner.as_ref().ok_or_else(|| {
                    KnowledgeError::new(
                        "copyOwner",
                        "A managed source requires its exact original owner",
                    )
                })?;
                let original = input
                    .inventory
                    .nodes
                    .iter()
                    .find(|node| &node.owner == owner && node.source.as_bytes() == file.before)
                    .ok_or_else(|| {
                        KnowledgeError::new(
                            "copySnapshot",
                            "Copy source differs from the authorized frozen inventory",
                        )
                    })?;
                let new_owner = input.node_map.get(owner).ok_or_else(|| {
                    KnowledgeError::new("copyMap", "No fresh node mapping for this source")
                })?;
                let inspected = weftext_engine::inspect_metadata(source);
                let metadata = inspected.metadata.ok_or_else(|| {
                    KnowledgeError::new(
                        "copyIdentity",
                        "Native root metadata does not have a proven slot",
                    )
                })?;
                let id = metadata
                    .fields
                    .iter()
                    .find(|field| {
                        field.key == "id"
                            && field.value.as_ref().and_then(serde_json::Value::as_str)
                                == Some(owner.node_id.as_str())
                    })
                    .ok_or_else(|| {
                        KnowledgeError::new(
                            "copyIdentity",
                            "Original native id does not match the owned source",
                        )
                    })?;
                let mut patches = vec![(
                    ExactRange::new(source, id.from, id.to)?,
                    encode_string(&new_owner.node_id)?,
                )];
                let (resource_patches, reference_losses) =
                    copy_body_references(original, file, &input.files, &input.inventory)?;
                patches.extend(resource_patches);
                for loss in reference_losses {
                    lose(&loss.code, &loss.detail);
                }
                let definitions = match definition_slots(source, owner) {
                    Ok(slots) => slots,
                    Err(error) if input.allow_losses => {
                        lose("unknownDefinitions", &error.message);
                        vec![]
                    }
                    Err(error) => return Err(error),
                };
                for slot in definitions {
                    let new_key=input.definition_map.get(&slot.identity).ok_or_else(||KnowledgeError::new("definitionMap","Every copied application definition needs a frozen fresh owner/key mapping"))?;
                    if new_key == &slot.key || !fresh_uuid(new_key) {
                        return Err(KnowledgeError::new(
                            "definitionMap",
                            "A copied definition requires a fresh UUID key",
                        ));
                    }
                    patches.push((slot.range, encode_string(new_key)?));
                }
                if !file.typed_refs_complete {
                    lose(
                        "unknownReferences",
                        "The installed schema has not proved complete typed-ref slots; originals are preserved and unknown references remain unchanged",
                    );
                }
                if !input.tag_map.is_empty()
                    || input
                        .node_map
                        .get(owner)
                        .is_some_and(|target| target.workspace_id != owner.workspace_id)
                {
                    let slots = saved_query_slots(source, owner, None)?;
                    for (schema_range, revision_range, reference, revision) in slots.schema_bindings
                    {
                        let (target, target_revision) = input.schema_map.get(&(reference,revision)).ok_or_else(|| KnowledgeError::new("schemaMap", "A foreign Query requires an exact equivalent target schema role and revision"))?;
                        patches.push((schema_range, encode_string(target)?));
                        patches.push((revision_range, encode_string(target_revision)?));
                    }
                    for (range, expression) in slots.tag_expressions {
                        let after = weftext_data::remap_cel_tag_literals(
                            &expression,
                            &input.tag_map,
                            &owner.workspace_id,
                        )
                        .map_err(|e| KnowledgeError::new("tagMap", e.to_string()))?;
                        if after != expression {
                            patches.push((range, encode_string(&after)?));
                        }
                    }
                    for (range, reference) in slots.tag_defaults {
                        if let Some(target) = input.tag_map.get(&reference) {
                            patches.push((range, encode_string(target)?));
                        } else if reference
                            .strip_prefix("weftext-tag://")
                            .and_then(|r| r.split_once('/'))
                            .is_some_and(|(workspace, _)| workspace == owner.workspace_id)
                        {
                            return Err(KnowledgeError::new(
                                "tagMap",
                                "A typed source Query tag default requires its exact complete import mapping",
                            ));
                        }
                    }
                }
                for slot in &file.typed_refs {
                    if file.definition_references.iter().any(|reference| {
                        reference.revision.is_some()
                            && reference.owner_slot.from == slot.from
                            && reference.owner_slot.to == slot.to
                    }) {
                        continue;
                    }
                    let range = ExactRange::new(source, slot.from, slot.to)?;
                    if !verify_string_slot(source, &range, &slot.target) {
                        return Err(KnowledgeError::new(
                            "typedRefSlot",
                            "Typed reference must be one exact official native string slot",
                        ));
                    }
                    let replacement = match slot.kind.as_str() {
                        "node_ref" | "nodeRef" | "ref" => {
                            let resolved = resolve_link(&slot.target, original, &input.inventory);
                            let old = resolved.target.ok_or_else(|| {
                                KnowledgeError::new(
                                    "copyRef",
                                    "A typed node reference cannot be resolved exactly",
                                )
                            })?;
                            if resolved.fragment.is_some() {
                                return Err(KnowledgeError::new(
                                    "copyRef",
                                    "Typed node references do not accept body fragments",
                                ));
                            }
                            input.node_map.get(&old).map(|new| {
                                format!("weftext-node://{}/{}", new.workspace_id, new.node_id)
                            })
                        }
                        "resource_ref" | "resourceRef" => {
                            resolve_link(&slot.target, original, &input.inventory)
                                .resource
                                .and_then(|resource| input.resource_map.get(&resource).cloned())
                        }
                        "tag_ref" | "tagRef" => {
                            let normalized = crate::links::normalized_uri_scheme(&slot.target);
                            if let Some(mapped) = input.tag_map.get(normalized.as_ref()) {
                                Some(mapped.clone())
                            } else {
                                let local = normalized
                                    .strip_prefix("weftext-tag://")
                                    .and_then(|r| r.split_once('/'))
                                    .is_some_and(|(workspace, id)| {
                                        workspace == original.owner.workspace_id
                                            && uuid::Uuid::parse_str(id).is_ok()
                                    });
                                let same_workspace =
                                    input.node_map.get(&original.owner).is_some_and(|target| {
                                        target.workspace_id == original.owner.workspace_id
                                    });
                                if local && !same_workspace {
                                    return Err(KnowledgeError::new(
                                        "tagMap",
                                        "A foreign import requires an exact target mapping for every source-workspace typed TagRef",
                                    ));
                                }
                                None
                            }
                        }
                        _ => {
                            lose(
                                "unknownReferenceKind",
                                "Unknown typed reference kind is preserved without guessing",
                            );
                            None
                        }
                    };
                    if let Some(replacement) = replacement {
                        patches.push((range, encode_string(&replacement)?));
                    }
                }
                for reference in &file.definition_references {
                    let old_owner = format!(
                        "weftext-node://{}/{}",
                        reference.owner.workspace_id, reference.owner.node_id
                    );
                    if !verify_node_reference_slot(source, &reference.owner_slot, &old_owner)
                        || !verify_string_slot(source, &reference.key_slot, &reference.key)
                    {
                        return Err(KnowledgeError::new(
                            "definitionReferenceSlot",
                            "Definition reference owner/key slots are not exact native strings",
                        ));
                    }
                    if let Some(revision) = &reference.revision {
                        if !reference
                            .revision_slot
                            .as_ref()
                            .is_some_and(|slot| verify_string_slot(source, slot, revision))
                        {
                            return Err(KnowledgeError::new(
                                "definitionReferenceSlot",
                                "Pinned definition revision is not the exact native string slot",
                            ));
                        }
                        lose(
                            "pinnedHistoricalDefinition",
                            "Pinned owner/key/revision retains its original historical triple as an explicit external dependency",
                        );
                        continue;
                    }
                    if reference.revision_slot.is_some() {
                        return Err(KnowledgeError::new(
                            "definitionReferenceSlot",
                            "Current definition cannot carry an unexplained historical revision slot",
                        ));
                    }
                    if let Some(new_owner) = input.node_map.get(&reference.owner) {
                        let new_key=input.definition_map.get(&definition_identity(&reference.owner,&reference.key)).ok_or_else(||KnowledgeError::new("definitionMap","A current reference needs the same frozen owner/key mapping as its definition"))?;
                        patches.push((
                            reference.owner_slot.clone(),
                            encode_string(&format!(
                                "weftext-node://{}/{}",
                                new_owner.workspace_id, new_owner.node_id
                            ))?,
                        ));
                        patches.push((reference.key_slot.clone(), encode_string(new_key)?));
                    }
                }
                for link in native_links(source) {
                    let Some(_) = &link.destination_slot else {
                        continue;
                    };
                    let resolution = resolve_link(&link.target, original, &input.inventory);
                    if matches!(resolution.status.as_str(), "external" | "inDocument") {
                        continue;
                    }
                    if resolution.status != "resolved" {
                        lose(
                            "unresolvedLink",
                            "A link is not uniquely resolved in the original context; its original bytes are preserved",
                        );
                        continue;
                    }
                    if let Some(old) = resolution.target {
                        let new = input.node_map.get(&old).unwrap_or(&old);
                        let mut destination =
                            format!("weftext-node://{}/{}", new.workspace_id, new.node_id);
                        if let Some(fragment) = resolution.fragment {
                            destination.push('#');
                            destination.push_str(
                                &url::form_urlencoded::byte_serialize(fragment.as_bytes())
                                    .collect::<String>()
                                    .replace('+', "%20"),
                            );
                        }
                        patches.push(crate::links::destination_patch(
                            source,
                            &link,
                            &destination,
                        )?);
                    } else if let Some(resource) = resolution.resource {
                        if input.resource_map.contains_key(&resource) {
                            if resolution.form == Some(LinkForm::Id) {
                                patches.push(crate::links::destination_patch(
                                    source,
                                    &link,
                                    &input.resource_map[&resource],
                                )?);
                                continue;
                            }
                            let old_path = input
                                .inventory
                                .resources
                                .iter()
                                .find(|r| r.key == resource)
                                .map(|r| r.path.as_str());
                            let copied = input
                                .files
                                .iter()
                                .find(|f| {
                                    matches!(f.kind, CopyFileKind::Resource)
                                        && file_url(&f.source_path).as_deref() == old_path
                                })
                                .ok_or_else(|| {
                                    KnowledgeError::new(
                                        "resourceClosure",
                                        "Mapped resource has no complete copied bytes",
                                    )
                                })?;
                            let address = file_url(&copied.target_path).ok_or_else(|| {
                                KnowledgeError::new(
                                    "resourcePath",
                                    "Resource target needs a platform absolute path",
                                )
                            })?;
                            patches.push(crate::links::destination_patch(source, &link, &address)?);
                        } else {
                            lose(
                                "externalResource",
                                "Resource was not re-owned by the copy selection; original link is retained as an explicit external dependency",
                            );
                        }
                    }
                }
                patches.sort_by_key(|(range, _)| (range.from, range.to));
                patches.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
                let after = apply_slots(source, patches.clone())?;
                source_edits.insert(owner.clone(), patches);
                source_after.insert(owner.clone(), after.clone());
                after.into_bytes()
            }
            CopyFileKind::Annotations => {
                let document = decode_annotations(&file.before)?;
                // Copying source bytes changes offset preimages/identity. Without
                // Core's exact candidate revision and copy edit map, discussion
                // is preserved against historical targets and remains stale.
                let mut value: serde_json::Value = serde_json::from_slice(&file.before).unwrap();
                for (index, record) in document.records.iter().enumerate() {
                    let new_id =
                        input
                            .annotation_map
                            .get(&record.annotation_id)
                            .ok_or_else(|| {
                                KnowledgeError::new(
                                    "annotationMap",
                                    "Every copied annotation needs a fresh mapping",
                                )
                            })?;
                    let item = &mut value["annotations"][index];
                    item["annotationId"] = new_id.clone().into();
                    if let Some(parent) = &record.reply_to {
                        item["replyTo"] = input
                            .annotation_map
                            .get(parent)
                            .ok_or_else(|| {
                                KnowledgeError::new("annotationMap", "Reply closure is incomplete")
                            })?
                            .clone()
                            .into();
                    }
                    if record.target.validate().is_err() {
                        lose(
                            "invalidCopiedTarget",
                            "Invalid target evidence remains untouched with the copied discussion",
                        );
                        item["copyTargetState"] = "stale".into();
                        if let Some(suggestion) =
                            item.get_mut("suggestion").filter(|s| !s.is_null())
                        {
                            suggestion["requiresReconfirmation"] = true.into();
                        }
                    } else if let Some(anchor) = record.target.anchor() {
                        let mapped = (|| -> Option<SourceAnchor> {
                            let old = input.inventory.nodes.iter().find(|n| {
                                n.owner == anchor.node
                                    && n.revision_id == anchor.revision_id
                                    && anchor.source_range.verify(&n.source)
                            })?;
                            let owner = input.node_map.get(&anchor.node)?;
                            let revision = input.revision_map.get(&anchor.node)?;
                            let edits = source_edits.get(&old.owner)?;
                            let after = source_after.get(&old.owner)?;
                            let mut delta = 0isize;
                            for (range, replacement) in edits {
                                if range.from < anchor.source_range.to
                                    && anchor.source_range.from < range.to
                                {
                                    return None;
                                }
                                if range.to <= anchor.source_range.from {
                                    delta = delta.checked_add(
                                        replacement.len() as isize
                                            - (range.to - range.from) as isize,
                                    )?;
                                }
                            }
                            let from = anchor.source_range.from.checked_add_signed(delta)?;
                            let to = anchor.source_range.to.checked_add_signed(delta)?;
                            let range = ExactRange::new(after, from, to).ok()?;
                            if range.raw != anchor.source_range.raw {
                                return None;
                            }
                            Some(SourceAnchor {
                                node: owner.clone(),
                                revision_id: revision.clone(),
                                source_range: range,
                                visible_excerpt: anchor.visible_excerpt.clone(),
                            })
                        })();
                        if let Some(anchor) = mapped {
                            let target = match &record.target {
                                AnnotationTarget::SourceElement { element_role, .. } => {
                                    AnnotationTarget::SourceElement {
                                        anchor,
                                        element_role: element_role.clone(),
                                    }
                                }
                                _ => AnnotationTarget::SourceRange { anchor },
                            };
                            item["target"] = serde_json::to_value(target).unwrap();
                        } else {
                            lose(
                                "staleCopiedAnchor",
                                "Discussion keeps its exact historical target because no continuous exact copy map could prove the new range",
                            );
                            item["copyTargetState"] = "stale".into();
                        }
                        if let Some(suggestion) =
                            item.get_mut("suggestion").filter(|s| !s.is_null())
                        {
                            suggestion["requiresReconfirmation"] = true.into();
                        }
                    } else if record.target.resource_basis().is_some_and(|(resource, _)| {
                        input.resource_map.contains_key(resource)
                            && !input.resource_revision_map.contains_key(resource)
                    }) {
                        lose(
                            "staleCopiedResourceAnchor",
                            "The copied discussion keeps its original resource target because no committed candidate resource revision was provided",
                        );
                        item["copyTargetState"] = "stale".into();
                    } else {
                        item["target"] = serde_json::to_value(remap_target(
                            record.target.clone(),
                            &input.node_map,
                            &input.resource_map,
                            &input.resource_revision_map,
                        ))
                        .unwrap();
                    }
                }
                let bytes = serde_json::to_vec_pretty(&value)
                    .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?;
                decode_annotations(&bytes)?;
                bytes
            }
        };
        plan.files.push(PlannedFile {
            path: file.target_path.clone(),
            before: None,
            after,
        });
    }
    plan.files.sort_by(|a, b| a.path.cmp(&b.path));
    if !plan.losses.is_empty() {
        if !input.allow_losses {
            return Err(KnowledgeError::new(
                "copyLosses",
                serde_json::to_string(&plan.losses).unwrap(),
            ));
        }
        plan.preserved_originals = input
            .files
            .iter()
            .map(|file| PreservedOriginal {
                source_path: file.source_path.clone(),
                bytes: file.before.clone(),
            })
            .collect();
    }
    Ok(plan)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TemplateValue {
    Null,
    Text(String),
    Bool(bool),
    Int(i64),
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TemplateSlotKind {
    Markup,
    String,
    Bool,
    Int,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSlot {
    pub name: String,
    pub range: ExactRange,
    pub kind: TemplateSlotKind,
    pub default: Option<TemplateValue>,
    pub nullable: bool,
}

/// A registry entry refers to a managed source revision, without retaining a
/// second authoritative source document. Host settings own persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source_node: NodeRef,
    pub source_revision: String,
    pub slots: Vec<TemplateSlot>,
}

pub fn validate_template_definition(
    definition: &TemplateDefinition,
    source: &NodeSnapshot,
) -> Result<()> {
    if definition.id.is_empty() || definition.title.trim().is_empty() {
        return Err(KnowledgeError::new(
            "templateDefinition",
            "A template needs an identity and title",
        ));
    }
    if definition.source_node != source.owner || definition.source_revision != source.revision_id {
        return Err(KnowledgeError::new(
            "staleTemplate",
            "The registered source revision is no longer current",
        ));
    }
    validate_template_slots(&source.source, &definition.slots)
}

/// Registration/instantiation accepts only exact official literal slots. Root
/// identity/type controls remain Core operations rather than template inputs.
pub fn validate_template_slots(source: &str, slots: &[TemplateSlot]) -> Result<()> {
    use typst_syntax::{LinkedNode, SyntaxKind};
    use unicode_segmentation::UnicodeSegmentation;
    let parsed = typst_syntax::Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return Err(KnowledgeError::new(
            "invalidTemplate",
            "Template source does not parse",
        ));
    }
    let metadata = weftext_engine::inspect_metadata(source).metadata;
    let editor = weftext_engine::inspect_editor(source);
    let boundaries: BTreeSet<_> = source
        .grapheme_indices(true)
        .map(|(byte, _)| byte)
        .chain(std::iter::once(source.len()))
        .collect();
    let mut names = BTreeSet::new();
    let mut ranges = vec![];
    fn literal(node: &LinkedNode<'_>, range: &ExactRange, kind: TemplateSlotKind) -> bool {
        let expected = match kind {
            TemplateSlotKind::String => SyntaxKind::Str,
            TemplateSlotKind::Bool => SyntaxKind::Bool,
            TemplateSlotKind::Int => SyntaxKind::Int,
            TemplateSlotKind::Markup => return false,
        };
        node.kind() == expected && node.range() == (range.from..range.to)
            || node.children().any(|child| literal(&child, range, kind))
    }
    for slot in slots {
        if slot.name.is_empty()
            || !names.insert(&slot.name)
            || !slot.range.verify(source)
            || !boundaries.contains(&slot.range.from)
            || !boundaries.contains(&slot.range.to)
        {
            return Err(KnowledgeError::new(
                "templateSlot",
                "Parameter names and whole-grapheme exact source slots must be unique and current",
            ));
        }
        if metadata.as_ref().is_some_and(|m| {
            m.fields
                .iter()
                .filter(|f| matches!(f.key.as_str(), "id" | "types"))
                .any(|field| slot.range.from < field.to && field.from < slot.range.to)
        }) {
            return Err(KnowledgeError::new(
                "templateIdentity",
                "Template parameters cannot rewrite managed identity/type controls",
            ));
        }
        let valid = match slot.kind {
            TemplateSlotKind::Markup => {
                editor.unavailable_reason.is_none()
                    && (editor.literal_regions.iter().any(|region| {
                        region.range.writable
                            && region.range.from <= slot.range.from
                            && slot.range.to <= region.range.to
                    }) || editor.title.status == "writable"
                        && editor.title.encoding.as_deref() == Some("markup")
                        && editor.title.range.as_ref().is_some_and(|range| {
                            range.writable
                                && range.from <= slot.range.from
                                && slot.range.to <= range.to
                        }))
            }
            _ => {
                literal(&LinkedNode::new(parsed.root()), &slot.range, slot.kind)
                    || slot.nullable && {
                        fn none(node: &LinkedNode<'_>, range: &ExactRange) -> bool {
                            node.kind() == SyntaxKind::None
                                && node.range() == (range.from..range.to)
                                || node.children().any(|child| none(&child, range))
                        }
                        none(&LinkedNode::new(parsed.root()), &slot.range)
                    }
            }
        };
        if !valid {
            return Err(KnowledgeError::new(
                "templateSlot",
                "Parameter does not select one exact official writable literal slot",
            ));
        }
        if let Some(default) = &slot.default {
            let valid = match (slot.kind, default) {
                (_, TemplateValue::Null) => {
                    slot.nullable && !matches!(slot.kind, TemplateSlotKind::Markup)
                }
                (TemplateSlotKind::Markup | TemplateSlotKind::String, TemplateValue::Text(_))
                | (TemplateSlotKind::Bool, TemplateValue::Bool(_))
                | (TemplateSlotKind::Int, TemplateValue::Int(_)) => true,
                _ => false,
            };
            if !valid {
                return Err(KnowledgeError::new(
                    "templateDefault",
                    "The declared default does not have the parameter's exact type",
                ));
            }
        }
        ranges.push((slot.range.from, slot.range.to));
    }
    ranges.sort();
    if ranges
        .windows(2)
        .any(|pair| pair[0].1 > pair[1].0 || pair[0] == pair[1])
    {
        return Err(KnowledgeError::new(
            "templateSlot",
            "Parameter source slots overlap",
        ));
    }
    Ok(())
}
/// Missing input uses a declared default; explicit null never uses the default.
/// Values enter exact typed slots as native literals, without executing input.
pub fn instantiate_template(
    source: &str,
    slots: &[TemplateSlot],
    parameters: &BTreeMap<String, TemplateValue>,
) -> Result<String> {
    validate_template_slots(source, slots)?;
    let names: BTreeSet<_> = slots.iter().map(|slot| &slot.name).collect();
    if parameters.keys().any(|name| !names.contains(name)) {
        return Err(KnowledgeError::new(
            "unknownParameter",
            "An input does not correspond to a declared template slot",
        ));
    }
    let mut patches = vec![];
    for slot in slots {
        let value = parameters
            .get(&slot.name)
            .or(slot.default.as_ref())
            .ok_or_else(|| {
                KnowledgeError::new(
                    "missingParameter",
                    format!("Template parameter {} is required", slot.name),
                )
            })?;
        let encoded = match (slot.kind, value) {
            (_, TemplateValue::Null)
                if slot.nullable && !matches!(slot.kind, TemplateSlotKind::Markup) =>
            {
                "none".into()
            }
            (TemplateSlotKind::Markup, TemplateValue::Text(text)) => {
                weftext_engine::encode_literal(text)
                    .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?
            }
            (TemplateSlotKind::String, TemplateValue::Text(text)) => encode_string(text)?,
            (TemplateSlotKind::Bool, TemplateValue::Bool(value)) => value.to_string(),
            (TemplateSlotKind::Int, TemplateValue::Int(value)) => value.to_string(),
            _ => {
                return Err(KnowledgeError::new(
                    "parameterType",
                    format!("Wrong type for template parameter {}", slot.name),
                ));
            }
        };
        patches.push((slot.range.clone(), encoded));
    }
    apply_slots(source, patches)
}
