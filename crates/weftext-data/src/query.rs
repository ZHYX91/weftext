use crate::*;
use cel::{
    Context, ExecutionError, Program, Value as CelValue,
    common::{
        ast::{EntryExpr, Expr, IdedExpr, LiteralValue},
        traits::{Comparer, Container, Indexer, Iterable, Sizer},
        types::{
            CelBool, CelDouble, CelInt, CelList, CelMapKey, CelNull, CelString, Type as CelType,
        },
        value::{BuiltinRef, CowVal, StaticVal, Val},
    },
};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering as AtomicOrdering},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryScope {
    pub directory: Option<String>,
    #[serde(default = "yes")]
    pub descendants: bool,
    #[serde(default = "live")]
    pub lifecycle: String,
}
impl Default for QueryScope {
    fn default() -> Self {
        Self {
            directory: None,
            descendants: true,
            lifecycle: live(),
        }
    }
}
fn yes() -> bool {
    true
}
fn live() -> String {
    "live".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryColumn {
    pub name: String,
    pub expr: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryOrder {
    pub expr: String,
    #[serde(default)]
    pub descending: bool,
    #[serde(default = "last")]
    pub nulls: String,
}
fn last() -> String {
    "last".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryUnnest {
    pub array: String,
    pub binding: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryAggregate {
    pub name: String,
    pub op: String,
    pub expr: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryGroup {
    pub keys: Vec<QueryColumn>,
    pub aggregates: Vec<QueryAggregate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryPlan {
    #[serde(default)]
    pub scope: QueryScope,
    #[serde(default)]
    pub types: Vec<String>,
    pub r#where: Option<String>,
    #[serde(default)]
    pub select: Vec<QueryColumn>,
    #[serde(default)]
    pub order: Vec<QueryOrder>,
    pub take: Option<usize>,
    pub group: Option<QueryGroup>,
    pub unnest: Option<QueryUnnest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced: Option<QueryAdvanced>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenNote {
    pub entity_key: String,
    pub path: String,
    pub source: String,
    pub revision: String,
    pub lifecycle: String,
    pub title: Option<String>,
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub products: Option<NoteProducts>,
    /// Trusted in-process field producer only; wire JSON cannot install this.
    /// Values are keyed by canonical FieldId, never author keys or source ranges.
    #[serde(skip)]
    pub authorized_fields: Option<BTreeMap<String, Cell>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteProducts {
    pub compile_identity: String,
    pub target: String,
    pub title: Cell,
    pub body: Cell,
    pub headings: Cell,
}

/// Trusted producer metadata. Wire JSON cannot install this capability; the
/// hosted adapter authorizes identity/types products and relation read effects.
#[derive(Debug, Clone, Default)]
pub struct AuthorizedQueryMetadata {
    pub identity: Option<String>,
    pub types: Option<Vec<String>>,
    pub resolved_types: Vec<String>,
}

pub(crate) fn query_projection(
    note: &FrozenNote,
    schema: &Schema,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
) -> NoteProjection {
    let Some(fields) = &note.authorized_fields else {
        return project(&note.source, schema);
    };
    let qualified = metadata.get(&note.entity_key);
    NoteProjection {
        qualification: "qualified".into(),
        id: None,
        types: qualified.and_then(|m| m.types.clone()).unwrap_or_default(),
        resolved_types: qualified
            .map(|m| m.resolved_types.clone())
            .unwrap_or_default(),
        fields: schema
            .fields
            .iter()
            .map(|f| FieldProjection {
                field_id: f.id.clone(),
                key: f.key.clone(),
                cell: fields.get(&f.id).cloned().unwrap_or(Cell::Unavailable {
                    reason: "authorized-field-unavailable".into(),
                }),
                source_range: None,
                raw: None,
                writable: false,
                required: false,
                unbound: vec![],
            })
            .collect(),
        unknown_fields: vec![],
        diagnostics: vec![],
    }
}

pub(crate) fn query_note_bytes(
    note: &FrozenNote,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
) -> Result<usize> {
    let mut bytes = if let Some(fields) = &note.authorized_fields {
        serde_json::to_vec(fields)?.len()
    } else {
        note.source.len()
    };
    if let Some(products) = &note.products {
        bytes = bytes.saturating_add(serde_json::to_vec(products)?.len());
    }
    if let Some(metadata) = metadata.get(&note.entity_key) {
        bytes = bytes.saturating_add(metadata.identity.as_ref().map_or(0, String::len));
        for name in metadata
            .types
            .iter()
            .flatten()
            .chain(&metadata.resolved_types)
        {
            bytes = bytes.saturating_add(name.len());
        }
    }
    Ok(bytes)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenQueryInput {
    pub cut_id: String,
    pub schema_revision: String,
    pub coverage: String,
    pub freshness: String,
    pub notes: Vec<FrozenNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced: Option<FrozenAdvancedInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRow {
    pub key: String,
    pub entity_key: Option<String>,
    pub path: Option<String>,
    pub revision: Option<String>,
    pub cells: BTreeMap<String, Cell>,
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub cut_id: String,
    pub schema_revision: String,
    pub rows: Vec<QueryRow>,
    pub coverage: String,
    pub freshness: String,
    pub diagnostics: Vec<String>,
    pub total_known: usize,
    pub limited: bool,
    pub aggregate_exact: bool,
    pub runtime: String,
    pub terminal_schema: BTreeMap<String, QueryValueType>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryBudget {
    pub max_steps: u64,
    pub max_rows: usize,
    pub max_source_bytes: usize,
}
impl Default for QueryBudget {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_rows: 100_000,
            max_source_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum QueryValueType {
    Scalar(String),
    List(Box<QType>),
    Record(BTreeMap<String, QType>),
    Map(Box<QType>),
    Union(Vec<QType>),
    Any,
}
pub(crate) type QType = QueryValueType;
pub(crate) fn scalar(s: &str) -> QType {
    QType::Scalar(s.into())
}
pub(crate) fn shape_type(schema: &Schema, s: &Shape) -> QType {
    match schema.resolve(s) {
        Shape::Scalar { semantic } => scalar(match semantic.as_str() {
            "nonempty_text" | "phone" | "email" | "identifier" | "doi" | "url" | "language_tag"
            | "citation_key" | "config_revision" => "text",
            s => s,
        }),
        Shape::Enum { .. } => scalar("text"),
        Shape::List { items, .. } => QType::List(Box::new(shape_type(schema, items))),
        Shape::Record { members, .. } => QType::Record(
            members
                .iter()
                .map(|(k, m)| (k.clone(), shape_type(schema, &m.shape)))
                .collect(),
        ),
        Shape::Union { variants, .. } => {
            QType::Union(variants.iter().map(|v| shape_type(schema, v)).collect())
        }
        _ => unreachable!(),
    }
}
pub(crate) fn compatible(a: &QType, b: &QType) -> bool {
    a == b
        || matches!(a, QType::Any)
        || matches!(b, QType::Any)
        || match (a, b) {
            (QType::Union(v), b) => v.iter().any(|a| compatible(a, b)),
            (a, QType::Union(v)) => v.iter().any(|b| compatible(a, b)),
            (QType::List(a), QType::List(b)) => compatible(a, b),
            _ => false,
        }
}
pub(crate) fn symbols(schema: &Schema) -> Result<BTreeMap<String, QType>> {
    let mut map = BTreeMap::from([
        ("self".into(), scalar("node_ref")),
        ("title".into(), scalar("text")),
        ("body".into(), scalar("text")),
        ("types".into(), QType::List(Box::new(scalar("text")))),
        (
            "headings".into(),
            QType::List(Box::new(QType::Record(BTreeMap::from([
                ("level".into(), scalar("int")),
                ("text".into(), scalar("text")),
                ("generated".into(), scalar("bool")),
                ("displayAssociation".into(), scalar("text")),
            ])))),
        ),
    ]);
    for f in &schema.fields {
        if map
            .insert(f.key.clone(), shape_type(schema, &f.shape))
            .is_some()
        {
            return Err(DataError(
                "Schema field conflicts with reserved Query product".into(),
            ));
        }
    }
    Ok(map)
}
pub(crate) fn literal_text(e: &IdedExpr) -> Option<String> {
    match &e.expr {
        Expr::Literal(LiteralValue::String(s)) => Some(s.inner().to_string()),
        _ => None,
    }
}
fn field_name(e: &IdedExpr, schema: &Schema) -> Option<String> {
    match &e.expr {
        Expr::Ident(n) => schema
            .fields
            .iter()
            .find(|f| &f.key == n)
            .map(|f| f.key.clone()),
        Expr::Call(c) if c.func_name == "field" && c.target.is_none() && c.args.len() == 1 => {
            let id = literal_text(&c.args[0])?;
            schema
                .fields
                .iter()
                .find(|f| f.id == id)
                .map(|f| f.key.clone())
        }
        _ => None,
    }
}
fn typecheck(
    e: &IdedExpr,
    schema: &Schema,
    vars: &BTreeMap<String, QType>,
    depth: usize,
) -> Result<QType> {
    if depth > 64 {
        return Err(DataError("CEL expression nesting budget".into()));
    }
    let child = |e: &IdedExpr| typecheck(e, schema, vars, depth + 1);
    Ok(match &e.expr {
        Expr::Literal(l) => match l {
            LiteralValue::Boolean(_) => scalar("bool"),
            LiteralValue::Double(v) => {
                if !v.into_inner().is_finite() {
                    return Err(DataError("Nonfinite float literal".into()));
                }
                scalar("float")
            }
            LiteralValue::Int(_) => scalar("int"),
            LiteralValue::String(_) => scalar("text"),
            LiteralValue::Null => scalar("none"),
            _ => return Err(DataError("CEL profile excludes uint/bytes".into())),
        },
        Expr::Ident(n) => vars
            .get(n)
            .cloned()
            .ok_or_else(|| DataError(format!("Unknown CEL symbol {n}")))?,
        Expr::Select(s) => {
            let t = child(&s.operand)?;
            let member = |t: &QType| match t {
                QType::Record(m) => m.get(&s.field).cloned(),
                QType::Map(v) => Some(*v.clone()),
                _ => None,
            };
            let result = member(&t)
                .or_else(|| {
                    if let QType::Union(v) = &t {
                        let found: Vec<_> = v.iter().filter_map(member).collect();
                        if found.is_empty() {
                            None
                        } else {
                            Some(QType::Union(found))
                        }
                    } else {
                        None
                    }
                })
                .ok_or_else(|| DataError(format!("Unknown typed record member {}", s.field)))?;
            if s.test { scalar("bool") } else { result }
        }
        Expr::List(l) => {
            let types = l.elements.iter().map(child).collect::<Result<Vec<_>>>()?;
            let item = types.first().cloned().unwrap_or(QType::Any);
            if types.iter().any(|t| !compatible(t, &item)) {
                return Err(DataError(
                    "CEL list elements require one declared item type".into(),
                ));
            }
            QType::List(Box::new(item))
        }
        Expr::Map(m) => {
            let mut members = BTreeMap::new();
            for entry in &m.entries {
                let EntryExpr::MapEntry(v) = &entry.expr else {
                    return Err(DataError("Struct values excluded".into()));
                };
                let k = literal_text(&v.key)
                    .ok_or_else(|| DataError("Only string-key records in Query profile".into()))?;
                if members.insert(k, child(&v.value)?).is_some() {
                    return Err(DataError("Duplicate CEL record member".into()));
                }
            }
            if members.is_empty() {
                QType::Map(Box::new(QType::Any))
            } else {
                QType::Record(members)
            }
        }
        Expr::Comprehension(c) => {
            let iter = child(&c.iter_range)?;
            let item = match iter {
                QType::List(t) => *t,
                QType::Record(_) | QType::Map(_) => scalar("text"),
                _ => return Err(DataError("Comprehension needs a typed list/record".into())),
            };
            let mut inner = vars.clone();
            inner.insert(c.iter_var.clone(), item.clone());
            if let Some(n) = &c.iter_var2 {
                inner.insert(n.clone(), item);
            }
            let initial = child(&c.accu_init)?;
            inner.insert(c.accu_var.clone(), initial.clone());
            typecheck(&c.loop_cond, schema, &inner, depth + 1)?;
            let step = typecheck(&c.loop_step, schema, &inner, depth + 1)?;
            inner.insert(c.accu_var.clone(), step);
            typecheck(&c.result, schema, &inner, depth + 1)?
        }
        Expr::Call(c) => {
            if c.func_name == "field" {
                let name = field_name(e, schema)
                    .ok_or_else(|| DataError("field requires a literal existing FieldId".into()))?;
                return vars.get(&name).cloned().ok_or_else(|| {
                    DataError("Field is unavailable in this expression scope".into())
                });
            }
            if matches!(
                c.func_name.as_str(),
                "isAbsent" | "isNull" | "isInvalid" | "isUnavailable"
            ) {
                if c.target.is_some()
                    || c.args.len() != 1
                    || field_name(&c.args[0], schema).is_none()
                {
                    return Err(DataError(
                        "Cell predicate requires a direct field reference".into(),
                    ));
                }
                return Ok(scalar("bool"));
            }
            if matches!(
                c.func_name.as_str(),
                "decimal"
                    | "exactDecimal"
                    | "date"
                    | "time"
                    | "localDatetime"
                    | "instant"
                    | "nodeRef"
                    | "resourceRef"
                    | "tagRef"
                    | "duration"
                    | "exactSeconds"
            ) {
                if c.target.is_some() || c.args.len() != 1 || literal_text(&c.args[0]).is_none() {
                    return Err(DataError(
                        "Typed constructor requires one literal string".into(),
                    ));
                }
                let (semantic, _) =
                    typed_constant(&c.func_name, literal_text(&c.args[0]).unwrap())?;
                return Ok(scalar(semantic));
            }
            let mut args = vec![];
            if let Some(t) = &c.target {
                args.push(child(t)?);
            }
            args.extend(c.args.iter().map(child).collect::<Result<Vec<_>>>()?);
            match c.func_name.as_str() {
                "_&&_" | "_||_" => {
                    if args.len() != 2 || args.iter().any(|v| !compatible(v, &scalar("bool"))) {
                        return Err(DataError("Logical operands must be bool".into()));
                    }
                    scalar("bool")
                }
                "!_" | "@not_strictly_false" => {
                    if args.len() != 1 || !compatible(&args[0], &scalar("bool")) {
                        return Err(DataError("Boolean operand required".into()));
                    }
                    scalar("bool")
                }
                "_?_:_" => {
                    if args.len() != 3
                        || !compatible(&args[0], &scalar("bool"))
                        || !compatible(&args[1], &args[2])
                    {
                        return Err(DataError("Conditional branches/types disagree".into()));
                    }
                    if matches!(args[1], QType::Any) {
                        args[2].clone()
                    } else {
                        args[1].clone()
                    }
                }
                "_==_" | "_!=_" | "_<_" | "_<=_" | "_>_" | "_>=_" => {
                    if args.len() != 2 || !compatible(&args[0], &args[1]) {
                        return Err(DataError(
                            "Query forbids implicit scalar/numeric/date conversion".into(),
                        ));
                    }
                    scalar("bool")
                }
                "_+_" | "_-_" | "_*_" | "_/_" | "_%_" => {
                    if args.len() != 2 || !compatible(&args[0], &args[1]) {
                        return Err(DataError("Arithmetic types disagree".into()));
                    }
                    if let (QType::List(a), QType::List(b)) = (&args[0], &args[1]) {
                        QType::List(if matches!(a.as_ref(), QType::Any) {
                            b.clone()
                        } else {
                            a.clone()
                        })
                    } else {
                        args[0].clone()
                    }
                }
                "-_" => {
                    if args.len() != 1
                        || !matches!(args[0],QType::Scalar(ref s)if s=="int"||s=="float")
                    {
                        return Err(DataError("Unary numeric operand".into()));
                    }
                    args[0].clone()
                }
                "_[_]" => {
                    if args.len() != 2 {
                        return Err(DataError("Index arity".into()));
                    }
                    match &args[0] {
                        QType::List(a) if compatible(&args[1], &scalar("int")) => *a.clone(),
                        QType::Map(v) => *v.clone(),
                        QType::Record(m) => {
                            let key = c.args.last().and_then(literal_text).ok_or_else(|| {
                                DataError("Typed record index requires literal key".into())
                            })?;
                            m.get(&key)
                                .cloned()
                                .ok_or_else(|| DataError("Unknown record key".into()))?
                        }
                        _ => return Err(DataError("Index type mismatch".into())),
                    }
                }
                "_in_" => {
                    if args.len() != 2 {
                        return Err(DataError("in arity".into()));
                    }
                    let valid = match &args[1] {
                        QType::List(item) => compatible(&args[0], item),
                        QType::Record(_) | QType::Map(_) => compatible(&args[0], &scalar("text")),
                        _ => false,
                    };
                    if !valid {
                        return Err(DataError(
                            "in requires the declared list item or text record-key type".into(),
                        ));
                    }
                    scalar("bool")
                }
                "size" => {
                    if args.len() != 1
                        || !(matches!(args[0], QType::List(_) | QType::Record(_) | QType::Map(_))
                            || args[0] == scalar("text"))
                    {
                        return Err(DataError("size requires string/list/record".into()));
                    }
                    scalar("int")
                }
                "contains" | "startsWith" | "endsWith" => {
                    if args.len() != 2 || args.iter().any(|v| !compatible(v, &scalar("text"))) {
                        return Err(DataError("String predicate requires text".into()));
                    }
                    scalar("bool")
                }
                "expandRecurrence" => {
                    if c.target.is_some() || args.len() != 3 {
                        return Err(DataError(
                            "expandRecurrence requires event, window and explicit budget".into(),
                        ));
                    }
                    if !schema
                        .providers
                        .iter()
                        .any(|p| p.implementation.id == "weftext.temporal.rules.v2")
                    {
                        return Err(DataError(
                            "Temporal provider is not installed in the baseline".into(),
                        ));
                    }
                    recurrence_result_type()
                }
                "containsDate" | "rangeOverlap" | "compareTemporal" => {
                    if c.target.is_some() || args.len() != 2 {
                        return Err(DataError(
                            "Temporal provider call requires two declared inputs".into(),
                        ));
                    }
                    if !schema
                        .providers
                        .iter()
                        .any(|p| p.implementation.id == "weftext.temporal.rules.v2")
                    {
                        return Err(DataError(
                            "Temporal provider is not installed in the baseline".into(),
                        ));
                    }
                    if c.func_name == "compareTemporal" {
                        scalar("int")
                    } else {
                        scalar("bool")
                    }
                }
                "nextCivilDay" | "rangeDuration" | "duringDuration" | "periodBounds" => {
                    if c.target.is_some() || args.len() != 1 {
                        return Err(DataError(
                            "Temporal provider call requires one declared input".into(),
                        ));
                    }
                    if !schema
                        .providers
                        .iter()
                        .any(|p| p.implementation.id == "weftext.temporal.rules.v2")
                    {
                        return Err(DataError(
                            "Temporal provider is not installed in the baseline".into(),
                        ));
                    }
                    if c.func_name == "nextCivilDay" {
                        scalar("date")
                    } else if c.func_name == "periodBounds" {
                        QType::Record(BTreeMap::from([
                            ("start".into(), scalar("date")),
                            ("end_exclusive".into(), scalar("date")),
                        ]))
                    } else {
                        duration_result_type()
                    }
                }
                _ => {
                    return Err(DataError(format!(
                        "Function {} is not registered in weftext.query.cel.v1",
                        c.func_name
                    )));
                }
            }
        }
        _ => {
            return Err(DataError(
                "CEL profile excludes dynamic/struct syntax".into(),
            ));
        }
    })
}
pub(crate) struct Compiled {
    ast: IdedExpr,
    pub(crate) ty: QType,
}
type GroupBucket = (Vec<Cell>, Vec<Vec<Option<Cell>>>);
pub(crate) fn compile(
    expr: &str,
    schema: &Schema,
    vars: &BTreeMap<String, QType>,
) -> Result<Compiled> {
    if expr.len() > 16384 {
        return Err(DataError("CEL source exceeds profile budget".into()));
    }
    let p = Program::compile(expr).map_err(|e| DataError(e.to_string()))?;
    let mut ast = p.expression().clone();
    alpha_rename(&mut ast, &BTreeMap::new(), &mut 0);
    let ty = typecheck(&ast, schema, vars, 0)?;
    Ok(Compiled { ast, ty })
}
pub(crate) fn alpha_rename(
    ast: &mut IdedExpr,
    names: &BTreeMap<String, String>,
    counter: &mut usize,
) {
    match &mut ast.expr {
        Expr::Ident(name) => {
            if let Some(mapped) = names.get(name) {
                *name = mapped.clone();
            }
        }
        Expr::Comprehension(c) => {
            alpha_rename(&mut c.iter_range, names, counter);
            alpha_rename(&mut c.accu_init, names, counter);
            let mut local = names.clone();
            let mut rename = |name: &mut String| {
                let mapped = format!("@weftext_local_{}", *counter);
                *counter += 1;
                local.insert(name.clone(), mapped.clone());
                *name = mapped;
            };
            rename(&mut c.iter_var);
            if let Some(name) = &mut c.iter_var2 {
                rename(name);
            }
            rename(&mut c.accu_var);
            for e in [&mut c.loop_cond, &mut c.loop_step, &mut c.result] {
                alpha_rename(e, &local, counter);
            }
        }
        _ => {
            let _ = visit_children_mut(ast, |e| {
                alpha_rename(e, names, counter);
                Ok(())
            });
        }
    }
}

#[derive(Debug)]
struct ExactAtom {
    value: NativeValue,
    semantic: String,
    ty: CelType,
}
impl ExactAtom {
    fn new(value: NativeValue, semantic: &str) -> Self {
        Self {
            value,
            semantic: semantic.into(),
            ty: CelType::new_opaque_type(format!("weftext.{semantic}")),
        }
    }
}
impl Clone for ExactAtom {
    fn clone(&self) -> Self {
        Self::new(self.value.clone(), &self.semantic)
    }
}
impl StaticVal for ExactAtom {}
impl Val for ExactAtom {
    fn get_type(&self) -> &CelType {
        &self.ty
    }
    fn cel_type() -> &'static CelType {
        panic!("ExactAtom type varies with schema semantic")
    }
    fn clone_as_boxed<'v>(&self) -> Box<dyn Val + 'v> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
    fn as_comparer(&self) -> Option<&dyn Comparer> {
        Some(self)
    }
    fn equals(&self, other: &dyn Val) -> bool {
        other.downcast_ref::<Self>().is_some_and(|v| {
            self.semantic == v.semantic && self.compare(other) == Ok(Ordering::Equal)
        })
    }
}
impl Comparer for ExactAtom {
    fn compare(&self, rhs: &dyn Val) -> std::result::Result<Ordering, ExecutionError> {
        let other = rhs
            .downcast_ref::<Self>()
            .filter(|o| o.semantic == self.semantic)
            .ok_or_else(|| ExecutionError::UnexpectedType {
                got: rhs.get_type().name().into(),
                want: self.ty.name().into(),
            })?;
        let result = match self.semantic.as_str() {
            "exact_decimal" | "exact_seconds" => self
                .value
                .text()
                .zip(other.value.text())
                .and_then(|(a, b)| ExactDecimal::parse(a).ok().zip(ExactDecimal::parse(b).ok()))
                .map(|(a, b)| a.compare(&b)),
            "instant" => self
                .value
                .text()
                .zip(other.value.text())
                .and_then(|(a, b)| ExactInstant::parse(a).ok().zip(ExactInstant::parse(b).ok()))
                .map(|(a, b)| a.compare(&b)),
            "node_ref" | "resource_ref" | "tag_ref" => self
                .value
                .text()
                .zip(other.value.text())
                .map(|(a, b)| a.cmp(b)),
            _ => compare_native(&self.value, &other.value).ok(),
        };
        result.ok_or_else(|| ExecutionError::FunctionError {
            function: "compare".into(),
            message: "Exact same-type comparison unavailable".into(),
        })
    }
}
/// An ordinary CEL map adapter with deterministic authored member order.
/// Macro filter/map clones this exact structure instead of rebuilding HashMaps.
#[derive(Debug, Clone)]
struct OrderedRecord {
    members: Vec<(CelString<'static>, Box<dyn Val>)>,
}
struct OrderedRecordConstructor {
    schema: Arc<Schema>,
}
impl cel::extractors::IntoFunction<OrderedRecordConstructor> for OrderedRecordConstructor {
    fn into_function(
        self,
    ) -> Box<
        dyn for<'c, 'v> Fn(
                &mut cel::FunctionContext<'c, 'v>,
            ) -> std::result::Result<CowVal<'c, 'v>, ExecutionError>
            + Send
            + Sync,
    > {
        fn owned(value: &dyn Val, schema: &Schema) -> Result<Box<dyn Val>> {
            if let Some(atom) = value.downcast_ref::<ExactAtom>() {
                return Ok(Box::new(atom.clone()));
            }
            if let Some(record) = value.downcast_ref::<OrderedRecord>() {
                return Ok(Box::new(record.clone()));
            }
            if let BuiltinRef::List(list) = value.as_builtin() {
                return Ok(Box::new(CelList::from(
                    list.inner()
                        .iter()
                        .map(|v| owned(v.as_ref(), schema))
                        .collect::<Result<Vec<_>>>()?,
                )));
            }
            Ok(to_cel(&from_cel(value)?, schema, None))
        }
        Box::new(move |context| {
            if !context.args.len().is_multiple_of(2) {
                return Err(context.error("Malformed internal record constructor"));
            }
            let mut members = vec![];
            for pair in context.args.as_chunks::<2>().0 {
                let BuiltinRef::String(key) = pair[0].as_builtin() else {
                    return Err(context.error("Record key must be text"));
                };
                members.push((
                    CelString::from(key.inner().to_string()),
                    owned(pair[1].as_ref(), &self.schema).map_err(|e| context.error(e.0))?,
                ));
            }
            Ok(CowVal::Owned(Box::new(OrderedRecord { members })))
        })
    }
}
impl StaticVal for OrderedRecord {}
impl Val for OrderedRecord {
    fn get_type(&self) -> &CelType {
        &cel::common::types::MAP_TYPE
    }
    fn cel_type() -> &'static CelType {
        &cel::common::types::MAP_TYPE
    }
    fn clone_as_boxed<'v>(&self) -> Box<dyn Val + 'v> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
    fn as_indexer<'b, 'v>(&'b self) -> Option<&'b (dyn Indexer + 'v)>
    where
        Self: 'v,
    {
        Some(self)
    }
    fn as_iterable<'b, 'v>(&'b self) -> Option<&'b (dyn Iterable + 'v)>
    where
        Self: 'v,
    {
        Some(self)
    }
    fn as_sizer(&self) -> Option<&dyn Sizer> {
        Some(self)
    }
    fn as_container(&self) -> Option<&dyn Container> {
        Some(self)
    }
    fn equals(&self, other: &dyn Val) -> bool {
        other.downcast_ref::<Self>().is_some_and(|o| {
            self.members.len() == o.members.len()
                && self.members.iter().all(|(k, v)| {
                    o.members
                        .iter()
                        .find(|(ok, _)| ok.inner() == k.inner())
                        .is_some_and(|(_, ov)| v.equals(ov.as_ref()))
                })
        })
    }
}
impl Indexer for OrderedRecord {
    fn get<'b, 'v>(&'b self, key: &dyn Val) -> std::result::Result<CowVal<'b, 'v>, ExecutionError>
    where
        Self: 'v,
    {
        let BuiltinRef::String(s) = key.as_builtin() else {
            return Err(ExecutionError::UnexpectedType {
                got: key.get_type().name().into(),
                want: "string".into(),
            });
        };
        self.members
            .iter()
            .find(|(k, _)| k.inner() == s.inner())
            .map(|(_, v)| CowVal::Borrowed(v.as_ref()))
            .ok_or_else(|| ExecutionError::NoSuchKey(Arc::new(s.inner().to_string())))
    }
    fn steal<'v>(
        self: Box<Self>,
        key: &dyn Val,
    ) -> std::result::Result<Box<dyn Val + 'v>, ExecutionError>
    where
        Self: 'v,
    {
        self.get(key).map(|v| v.clone_as_boxed())
    }
}
impl Sizer for OrderedRecord {
    fn size(&self) -> CelInt {
        CelInt::from(self.members.len() as i64)
    }
}
impl Container for OrderedRecord {
    fn contains(&self, key: &dyn Val) -> std::result::Result<bool, ExecutionError> {
        match self.get(key) {
            Ok(_) => Ok(true),
            Err(ExecutionError::NoSuchKey(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}
struct RecordKeys<'a> {
    members: &'a [(CelString<'static>, Box<dyn Val>)],
    index: usize,
}
impl<'b, 'v> cel::common::traits::Iterator<'b, 'v> for RecordKeys<'b> {
    fn next(&mut self) -> Option<&'b (dyn Val + 'v)> {
        let item = self.members.get(self.index)?;
        self.index += 1;
        Some(&item.0)
    }
}
impl Iterable for OrderedRecord {
    fn iter<'b, 'v>(&'b self) -> Box<dyn cel::common::traits::Iterator<'b, 'v> + 'b>
    where
        Self: 'v,
    {
        Box::new(RecordKeys {
            members: &self.members,
            index: 0,
        })
    }
}
fn to_cel(v: &NativeValue, schema: &Schema, shape: Option<&Shape>) -> Box<dyn Val> {
    let selected = shape.map(|s| schema.resolve(s));
    let selected = if let Some(Shape::Union {
        variants,
        discriminator,
    }) = selected
    {
        schema
            .union_variant(variants, discriminator.as_ref(), v)
            .ok()
            .map(|s| schema.resolve(s))
    } else {
        selected
    };
    if let Some(Shape::Scalar { semantic }) = selected
        && matches!(
            semantic.as_str(),
            "exact_decimal" | "exact_seconds" | "instant" | "node_ref" | "resource_ref" | "tag_ref"
        )
    {
        return Box::new(ExactAtom::new(v.clone(), semantic));
    }
    match v {
        NativeValue::None => Box::new(CelNull),
        NativeValue::Text(s) => Box::new(CelString::from(s.clone())),
        NativeValue::Bool(v) => Box::new(CelBool::from(*v)),
        NativeValue::Int(s) => Box::new(CelInt::from(s.parse::<i64>().unwrap())),
        NativeValue::Float(s) => Box::new(CelDouble::from(s.parse::<f64>().unwrap())),
        NativeValue::Decimal(_) => Box::new(ExactAtom::new(v.clone(), "decimal")),
        NativeValue::Date(_) => Box::new(ExactAtom::new(v.clone(), "date")),
        NativeValue::Time(_) => Box::new(ExactAtom::new(v.clone(), "time")),
        NativeValue::LocalDatetime(_) => Box::new(ExactAtom::new(v.clone(), "local_datetime")),
        NativeValue::Duration(_) => Box::new(ExactAtom::new(v.clone(), "duration")),
        NativeValue::Array(a) => {
            let items = if let Some(Shape::List { items, .. }) = selected {
                Some(items.as_ref())
            } else {
                None
            };
            Box::new(CelList::from(
                a.iter()
                    .map(|v| to_cel(v, schema, items))
                    .collect::<Vec<_>>(),
            ))
        }
        NativeValue::Record(a) => {
            let members = if let Some(Shape::Record { members, .. }) = selected {
                Some(members)
            } else {
                None
            };
            Box::new(OrderedRecord {
                members: a
                    .iter()
                    .map(|m| {
                        (
                            CelString::from(m.key.clone()),
                            to_cel(
                                &m.value,
                                schema,
                                members.and_then(|a| a.get(&m.key)).map(|m| &m.shape),
                            ),
                        )
                    })
                    .collect(),
            })
        }
    }
}
fn from_cel(v: &dyn Val) -> Result<NativeValue> {
    if let Some(a) = v.downcast_ref::<ExactAtom>() {
        return Ok(a.value.clone());
    }
    if let Some(record) = v.downcast_ref::<OrderedRecord>() {
        return Ok(NativeValue::Record(
            record
                .members
                .iter()
                .map(|(key, value)| {
                    Ok(NativeMember {
                        key: key.inner().to_string(),
                        value: from_cel(value.as_ref())?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        ));
    }
    if let Some(v) = v.downcast_ref::<CelBool>() {
        return Ok(NativeValue::Bool(v.into_inner()));
    }
    if let Some(v) = v.downcast_ref::<CelInt>() {
        return Ok(NativeValue::Int(v.into_inner().to_string()));
    }
    if let Some(v) = v.downcast_ref::<CelDouble>() {
        if !v.into_inner().is_finite() {
            return Err(DataError("Nonfinite float result".into()));
        }
        return Ok(NativeValue::Float(v.into_inner().to_string()));
    }
    if v.downcast_ref::<CelNull>().is_some() {
        return Ok(NativeValue::None);
    }
    match v.as_builtin() {
        BuiltinRef::String(v) => Ok(NativeValue::Text(v.inner().to_string())),
        BuiltinRef::List(a) => Ok(NativeValue::Array(
            a.inner()
                .iter()
                .map(|v| from_cel(v.as_ref()))
                .collect::<Result<Vec<_>>>()?,
        )),
        BuiltinRef::Map(a) => {
            let mut out = vec![];
            for (k, v) in a.inner() {
                let CelMapKey::String(k) = k else {
                    return Err(DataError("Result record key must be text".into()));
                };
                out.push(NativeMember {
                    key: k.inner().to_string(),
                    value: from_cel(v.as_ref())?,
                });
            }
            out.sort_by(|a, b| a.key.cmp(&b.key));
            Ok(NativeValue::Record(out))
        }
        _ => Err(DataError("Unsupported CEL result type".into())),
    }
}
fn typed_constant(name: &str, text: String) -> Result<(&'static str, NativeValue)> {
    let (semantic, value) = match name {
        "decimal" => ("decimal", NativeValue::Decimal(text)),
        "exactDecimal" => ("exact_decimal", NativeValue::Text(text)),
        "date" => ("date", NativeValue::Date(text)),
        "time" => ("time", NativeValue::Time(text)),
        "localDatetime" => ("local_datetime", NativeValue::LocalDatetime(text)),
        "instant" => ("instant", NativeValue::Text(text)),
        "nodeRef" => ("node_ref", NativeValue::Text(text)),
        "resourceRef" => ("resource_ref", NativeValue::Text(text)),
        "tagRef" => ("tag_ref", NativeValue::Text(text)),
        "duration" => ("duration", NativeValue::Duration(text)),
        "exactSeconds" => ("exact_seconds", NativeValue::Text(text)),
        _ => return Err(DataError("Unknown typed constant constructor".into())),
    };
    if !scalar_valid(semantic, &value) {
        return Err(DataError("Invalid typed Query constant".into()));
    }
    Ok((semantic, value))
}
fn bool_expr(v: bool) -> Expr {
    Expr::Literal(LiteralValue::Boolean(CelBool::from(v)))
}
fn bind_ast(
    ast: &mut IdedExpr,
    schema: &Schema,
    cells: &BTreeMap<String, Cell>,
    context: &mut Context,
    counter: &mut usize,
) -> Result<()> {
    if let Expr::Call(c) = &ast.expr {
        if c.func_name == "field" {
            ast.expr = Expr::Ident(
                field_name(ast, schema).ok_or_else(|| DataError("Unresolved field".into()))?,
            );
            return Ok(());
        }
        if matches!(
            c.func_name.as_str(),
            "isAbsent" | "isNull" | "isInvalid" | "isUnavailable"
        ) {
            let name = field_name(&c.args[0], schema).unwrap();
            let state = cells.get(&name).map(Cell::state).unwrap_or("absent");
            let expected = match c.func_name.as_str() {
                "isAbsent" => "absent",
                "isNull" => "null",
                "isInvalid" => "invalid",
                _ => "unavailable",
            };
            ast.expr = bool_expr(state == expected);
            return Ok(());
        }
        if matches!(
            c.func_name.as_str(),
            "decimal"
                | "exactDecimal"
                | "date"
                | "time"
                | "localDatetime"
                | "instant"
                | "nodeRef"
                | "resourceRef"
                | "tagRef"
                | "duration"
                | "exactSeconds"
        ) {
            let s = literal_text(&c.args[0]).unwrap();
            let (semantic, value) = typed_constant(&c.func_name, s)?;
            let name = format!("@weftext_constant_{}", *counter);
            *counter += 1;
            context.add_variable_as_val(&name, Box::new(ExactAtom::new(value, semantic)));
            ast.expr = Expr::Ident(name);
            return Ok(());
        }
    }
    visit_children_mut(ast, |e| bind_ast(e, schema, cells, context, counter))?;
    if let Expr::Map(map) = &ast.expr {
        let mut args = vec![];
        for entry in &map.entries {
            let EntryExpr::MapEntry(entry) = &entry.expr else {
                return Err(DataError("Only static string-key records".into()));
            };
            args.push(entry.key.clone());
            args.push(entry.value.clone());
        }
        let program = Program::compile("weftext_record()").map_err(|e| DataError(e.to_string()))?;
        let Expr::Call(mut call) = program.expression().expr.clone() else {
            unreachable!()
        };
        call.func_name = "@weftext_ordered_record".into();
        call.args = args;
        ast.expr = Expr::Call(call);
    }
    Ok(())
}
pub(crate) fn visit_children_mut(
    ast: &mut IdedExpr,
    mut f: impl FnMut(&mut IdedExpr) -> Result<()>,
) -> Result<()> {
    match &mut ast.expr {
        Expr::Call(c) => {
            if let Some(t) = &mut c.target {
                f(t)?;
            }
            for a in &mut c.args {
                f(a)?;
            }
        }
        Expr::Select(s) => f(&mut s.operand)?,
        Expr::List(l) => {
            for e in &mut l.elements {
                f(e)?;
            }
        }
        Expr::Map(m) => {
            for e in &mut m.entries {
                if let EntryExpr::MapEntry(e) = &mut e.expr {
                    f(&mut e.key)?;
                    f(&mut e.value)?;
                }
            }
        }
        Expr::Comprehension(c) => {
            for e in [
                &mut c.iter_range,
                &mut c.accu_init,
                &mut c.loop_cond,
                &mut c.loop_step,
                &mut c.result,
            ] {
                f(e)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn ast_cost(ast: &IdedExpr, max_collection: u64) -> u64 {
    let mut a = ast.clone();
    let mut cost = 1u64;
    let mut children = vec![];
    let _ = visit_children_mut(&mut a, |e| {
        children.push(e.clone());
        Ok(())
    });
    for c in children {
        cost = cost.saturating_add(ast_cost(&c, max_collection));
    }
    if matches!(ast.expr, Expr::Comprehension(_)) {
        cost = cost.saturating_mul(max_collection.max(1));
    }
    cost
}
fn literal_collection_bound(ast: &IdedExpr) -> u64 {
    let own = match &ast.expr {
        Expr::List(a) => a.elements.len() as u64,
        Expr::Map(m) => m.entries.len() as u64,
        _ => 0,
    };
    let mut a = ast.clone();
    let mut bound = own;
    let _ = visit_children_mut(&mut a, |e| {
        bound = bound.max(literal_collection_bound(e));
        Ok(())
    });
    bound
}
fn max_collection(v: &NativeValue) -> usize {
    match v {
        NativeValue::Array(a) => a.len().max(a.iter().map(max_collection).max().unwrap_or(0)),
        NativeValue::Record(a) => a.len().max(
            a.iter()
                .map(|m| max_collection(&m.value))
                .max()
                .unwrap_or(0),
        ),
        _ => 0,
    }
}
pub(crate) fn evaluate(
    compiled: &Compiled,
    cells: &BTreeMap<String, Cell>,
    schema: &Schema,
    budget: &mut u64,
    env: &QueryRuntime,
) -> Result<Cell> {
    // Decode limits are host failures in the static read closure, not CEL
    // errors that boolean absorption can turn into a complete result.
    fn consumes_native_budget(
        ast: &IdedExpr,
        cells: &BTreeMap<String, Cell>,
        schema: &Schema,
    ) -> bool {
        let name = field_name(ast, schema).or_else(|| {
            if let Expr::Ident(n) = &ast.expr {
                Some(n.clone())
            } else {
                None
            }
        });
        if name.as_ref().and_then(|n|cells.get(n)).is_some_and(|c|matches!(c,Cell::Unavailable{reason}if reason.contains("native-decoder")&&reason.contains("budget"))){return true;}
        let mut ast = ast.clone();
        let mut consumed = false;
        let _ = visit_children_mut(&mut ast, |child| {
            consumed |= consumes_native_budget(child, cells, schema);
            Ok(())
        });
        consumed
    }
    if consumes_native_budget(&compiled.ast, cells, schema) {
        env.native_budget_partial
            .store(true, AtomicOrdering::Relaxed);
    }
    let max = cells
        .values()
        .filter_map(Cell::native)
        .map(max_collection)
        .max()
        .unwrap_or(1) as u64;
    let max = max.max(literal_collection_bound(&compiled.ast));
    let cost = ast_cost(&compiled.ast, max);
    if cost > *budget {
        return Err(DataError("host-budget-exhausted".into()));
    }
    *budget -= cost;
    if let Expr::Ident(name) = &compiled.ast.expr
        && let Some(cell) = cells.get(name)
    {
        return Ok(cell.clone());
    }
    if let Some(name) = field_name(&compiled.ast, schema)
        && let Some(cell) = cells.get(&name)
    {
        return Ok(cell.clone());
    }
    let mut context = Context::with_env(env.env.clone());
    context
        .add_function::<OrderedRecordConstructor, _>(
            "@weftext_ordered_record",
            OrderedRecordConstructor {
                schema: env.schema.clone(),
            },
        )
        .map_err(|e| DataError(e.to_string()))?;
    for (name, arity) in TEMPORAL_CALLS {
        context
            .add_function::<ProviderFunction, _>(
                name,
                ProviderFunction {
                    name,
                    arity: *arity,
                    schema: env.schema.clone(),
                    partial: env.provider_partial.clone(),
                    trace: env.provider_trace.clone(),
                    failed: env.provider_failed.clone(),
                },
            )
            .map_err(|e| DataError(e.to_string()))?;
    }
    let self_shape = Shape::Scalar {
        semantic: "node_ref".into(),
    };
    for (name, cell) in cells {
        match cell {
            Cell::Value { value } => context.add_variable_as_val(
                name,
                to_cel(
                    value,
                    schema,
                    env.parameter_shapes
                        .get(name)
                        .or_else(|| schema.field(name).map(|f| &f.shape))
                        .or_else(|| (name == "self").then_some(&self_shape)),
                ),
            ),
            Cell::Null => context.add_variable_as_val(name, Box::new(CelNull)),
            _ => {}
        }
    }
    let mut ast = compiled.ast.clone();
    bind_ast(&mut ast, schema, cells, &mut context, &mut 0)?;
    match CelValue::resolve_val(&ast, &context) {
        Ok(v) => match from_cel(v.as_ref()) {
            Ok(v) => Ok(if matches!(v, NativeValue::None) {
                Cell::Null
            } else {
                Cell::value(v)
            }),
            Err(e) => Ok(Cell::Unavailable { reason: e.0 }),
        },
        Err(e) => Ok(Cell::Unavailable {
            reason: e.to_string(),
        }),
    }
}
/// Evaluates a frozen terminal; callers must publish its freshness through Core.
/// Passing per-file observations as a cut does not create snapshot guarantees.
const TEMPORAL_CALLS: &[(&str, usize)] = &[
    ("containsDate", 2),
    ("rangeOverlap", 2),
    ("compareTemporal", 2),
    ("nextCivilDay", 1),
    ("rangeDuration", 1),
    ("duringDuration", 1),
    ("periodBounds", 1),
    ("expandRecurrence", 3),
];
fn duration_result_type() -> QType {
    QType::Union(vec![
        QType::Record(BTreeMap::from([
            ("value".into(), scalar("int")),
            ("unit".into(), scalar("text")),
            ("calendar".into(), scalar("text")),
            ("version".into(), scalar("text")),
        ])),
        QType::Record(BTreeMap::from([
            ("value".into(), scalar("exact_seconds")),
            ("unit".into(), scalar("text")),
        ])),
    ])
}
fn recurrence_result_type() -> QType {
    QType::Record(BTreeMap::from([
        ("status".into(), scalar("text")),
        ("diagnostics".into(), QType::List(Box::new(scalar("text")))),
        ("steps".into(), scalar("int")),
        ("providers".into(), QType::List(Box::new(scalar("text")))),
        (
            "occurrences".into(),
            QType::List(Box::new(QType::Record(BTreeMap::from([
                (
                    "original_onset".into(),
                    QType::Union(vec![scalar("date"), scalar("instant")]),
                ),
                ("range".into(), QType::Map(Box::new(QType::Any))),
                ("replacement".into(), scalar("bool")),
                ("title".into(), scalar("text")),
                ("status".into(), scalar("text")),
                ("note".into(), scalar("text")),
            ])))),
        ),
    ]))
}
pub(crate) struct QueryRuntime {
    env: Arc<cel::Env>,
    schema: Arc<Schema>,
    pub(crate) parameter_shapes: BTreeMap<String, Shape>,
    pub(crate) provider_partial: Arc<AtomicBool>,
    pub(crate) native_budget_partial: Arc<AtomicBool>,
    provider_trace: Option<crate::query_providers::ProviderTrace>,
    provider_failed: Arc<AtomicBool>,
}
pub(crate) fn query_env_traced(
    schema: &Schema,
    trace: Option<&crate::query_providers::ProviderTrace>,
) -> Result<QueryRuntime> {
    Ok(QueryRuntime {
        env: Arc::new(cel::Env::stdlib()),
        schema: Arc::new(schema.clone()),
        parameter_shapes: BTreeMap::new(),
        provider_partial: Arc::new(AtomicBool::new(false)),
        native_budget_partial: Arc::new(AtomicBool::new(false)),
        provider_trace: trace.cloned(),
        provider_failed: Arc::new(AtomicBool::new(false)),
    })
}

/// Only the already authorized original CEL evaluator computes row-free calls.
/// A subtree with a row/terminal free variable is left for its real occurrence.
pub(crate) fn preflight_provider_expression(
    compiled: &Compiled,
    parameters: &BTreeMap<String, Cell>,
    schema: &Schema,
    remaining: &mut u64,
    env: &QueryRuntime,
) -> Result<()> {
    fn contains(ast: &IdedExpr) -> bool {
        if matches!(&ast.expr,Expr::Call(c) if crate::query_providers::PROVIDER_CALLS.iter().any(|(n,_)| *n==c.func_name))
        {
            return true;
        }
        let mut copy = ast.clone();
        let mut found = false;
        let _ = visit_children_mut(&mut copy, |child| {
            found |= contains(child);
            Ok(())
        });
        found
    }
    fn row_free(
        ast: &IdedExpr,
        parameters: &BTreeMap<String, Cell>,
        locals: &BTreeSet<String>,
    ) -> bool {
        match &ast.expr {
            Expr::Ident(name) => {
                locals.contains(name)
                    || parameters
                        .get(name)
                        .is_some_and(|c| matches!(c, Cell::Value { .. } | Cell::Null))
            }
            Expr::Call(c) if c.func_name == "field" => false,
            Expr::Comprehension(c) => {
                if !row_free(&c.iter_range, parameters, locals)
                    || !row_free(&c.accu_init, parameters, locals)
                {
                    return false;
                }
                let mut bound = locals.clone();
                bound.insert(c.iter_var.clone());
                bound.insert(c.accu_var.clone());
                if let Some(n) = &c.iter_var2 {
                    bound.insert(n.clone());
                }
                [&c.loop_cond, &c.loop_step, &c.result]
                    .iter()
                    .all(|e| row_free(e, parameters, &bound))
            }
            _ => {
                let mut copy = ast.clone();
                let mut valid = true;
                let _ = visit_children_mut(&mut copy, |child| {
                    valid &= row_free(child, parameters, locals);
                    Ok(())
                });
                valid
            }
        }
    }
    fn visit(
        ast: &IdedExpr,
        parameters: &BTreeMap<String, Cell>,
        schema: &Schema,
        vars: &BTreeMap<String, QType>,
        remaining: &mut u64,
        env: &QueryRuntime,
    ) -> Result<()> {
        if !contains(ast) {
            return Ok(());
        }
        if row_free(ast, parameters, &BTreeSet::new()) {
            let compiled = Compiled {
                ast: ast.clone(),
                ty: typecheck(ast, schema, vars, 0)?,
            };
            if !matches!(
                evaluate(&compiled, parameters, schema, remaining, env)?,
                Cell::Value { .. } | Cell::Null
            ) || env.provider_failed.load(AtomicOrdering::Relaxed)
            {
                return Err(DataError("Query provider unavailable".into()));
            }
            return Ok(());
        }
        if let Expr::Comprehension(c) = &ast.expr {
            visit(&c.iter_range, parameters, schema, vars, remaining, env)?;
            visit(&c.accu_init, parameters, schema, vars, remaining, env)?;
            let mut scoped = parameters.clone();
            scoped.remove(&c.iter_var);
            scoped.remove(&c.accu_var);
            if let Some(name) = &c.iter_var2 {
                scoped.remove(name);
            }
            for child in [&c.loop_cond, &c.loop_step, &c.result] {
                visit(child, &scoped, schema, vars, remaining, env)?;
            }
            return Ok(());
        }
        let mut copy = ast.clone();
        visit_children_mut(&mut copy, |child| {
            visit(child, parameters, schema, vars, remaining, env)
        })
    }
    let mut vars = symbols(schema)?;
    vars.extend(
        env.parameter_shapes
            .iter()
            .map(|(name, shape)| (name.clone(), shape_type(schema, shape))),
    );
    visit(&compiled.ast, parameters, schema, &vars, remaining, env)
}
struct ProviderFunction {
    name: &'static str,
    arity: usize,
    schema: Arc<Schema>,
    partial: Arc<AtomicBool>,
    trace: Option<crate::query_providers::ProviderTrace>,
    failed: Arc<AtomicBool>,
}
impl cel::extractors::IntoFunction<ProviderFunction> for ProviderFunction {
    fn into_function(
        self,
    ) -> Box<
        dyn for<'c, 'v> Fn(
                &mut cel::FunctionContext<'c, 'v>,
            ) -> std::result::Result<CowVal<'c, 'v>, ExecutionError>
            + Send
            + Sync,
    > {
        Box::new(move |context| {
            if context.args.len() != self.arity {
                return Err(context.error("Provider arity mismatch"));
            }
            let values = context
                .args
                .iter()
                .map(|v| from_cel(v.as_ref()))
                .collect::<Result<Vec<_>>>()
                .map_err(|e| context.error(e.0))?;
            if let Some(trace) = &self.trace {
                let mut dependencies = crate::query_providers::SemanticDependencies::default();
                dependencies
                    .call(&self.schema, self.name, &values)
                    .map_err(|_| context.error("Query provider unavailable"))?;
                trace
                    .record(&dependencies)
                    .map_err(|_| context.error("Query provider dependency budget exhausted"))?;
            }
            let cell = match self.name {
                "expandRecurrence" => {
                    let budget =
                        ExpansionBudget::from_native(&values[2]).map_err(|e| context.error(e.0))?;
                    let result = expand_event(&values[0], &values[1], &self.schema, budget);
                    if result.status == ExpansionStatus::Partial {
                        self.partial.store(true, AtomicOrdering::Relaxed);
                    }
                    match result.status {
                        ExpansionStatus::Complete | ExpansionStatus::Partial => {
                            Cell::value(result.native())
                        }
                        _ => Cell::Unavailable {
                            reason: result.diagnostics.join("; "),
                        },
                    }
                }
                "containsDate" => contains_date(&values[0], &values[1], &self.schema),
                "rangeOverlap" => range_overlap(&values[0], &values[1], &self.schema),
                "compareTemporal" => compare_temporal(&values[0], &values[1], &self.schema),
                "nextCivilDay" => next_civil_day(&values[0]),
                "rangeDuration" => range_duration(&values[0], &self.schema),
                "duringDuration" => during_duration(&values[0], &self.schema),
                _ => period_bounds(&values[0], &self.schema)
                    .map(Cell::value)
                    .unwrap_or_else(|e| Cell::Unavailable { reason: e.0 }),
            };
            if self.trace.is_some()
                && matches!(cell, Cell::Unavailable { .. } | Cell::Invalid { .. })
            {
                self.partial.store(true, AtomicOrdering::Relaxed);
                self.failed.store(true, AtomicOrdering::Relaxed);
            }
            match cell {
                Cell::Value { value } => {
                    let shape = match self.name {
                        "rangeDuration" | "duringDuration" => {
                            crate::query_advanced::type_shape(&duration_result_type())
                        }
                        "expandRecurrence" => {
                            crate::query_advanced::type_shape(&recurrence_result_type())
                        }
                        _ => None,
                    };
                    Ok(CowVal::Owned(to_cel(&value, &self.schema, shape.as_ref())))
                }
                Cell::Unavailable { reason } => Err(context.error(reason)),
                Cell::Invalid { diagnostics, .. } => Err(context.error(diagnostics.join("; "))),
                _ => Err(context.error("Non-value provider result")),
            }
        })
    }
}
pub fn execute_query(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
) -> Result<QueryResult> {
    execute_query_with_budget(input, plan, schema, &QueryBudget::default())
}
pub fn execute_query_with_budget(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    limits: &QueryBudget,
) -> Result<QueryResult> {
    execute_authorized_query(input, plan, schema, limits, &BTreeMap::new())
}
pub fn execute_authorized_query(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    limits: &QueryBudget,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
) -> Result<QueryResult> {
    crate::query_advanced::execute_advanced_query(input, plan, schema, limits, metadata)
}
/// Executes the same CEL/Runner with a bounded, server-private provider read set.
pub fn execute_authorized_query_with_provider_dependencies(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    limits: &QueryBudget,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
) -> Result<(QueryResult, QueryProviderDependencies)> {
    let trace = crate::query_providers::ProviderTrace::new();
    let result = crate::query_advanced::execute_advanced_query_traced(
        input,
        plan,
        schema,
        limits,
        metadata,
        Some(&trace),
    )?;
    Ok((result, trace.finish()?))
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_query_leaf(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    limits: &QueryBudget,
    parameters: &BTreeMap<String, Cell>,
    parameter_shapes: &BTreeMap<String, Shape>,
    remaining: &mut u64,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
) -> Result<QueryResult> {
    execute_query_leaf_traced(
        input,
        plan,
        schema,
        limits,
        parameters,
        parameter_shapes,
        remaining,
        metadata,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_query_leaf_traced(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    limits: &QueryBudget,
    parameters: &BTreeMap<String, Cell>,
    parameter_shapes: &BTreeMap<String, Shape>,
    remaining: &mut u64,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
    trace: Option<&crate::query_providers::ProviderTrace>,
) -> Result<QueryResult> {
    if !matches!(input.coverage.as_str(), "complete" | "partial")
        || !matches!(input.freshness.as_str(), "current" | "stale")
    {
        return Err(DataError("Invalid input coverage/freshness".into()));
    }
    if !matches!(plan.scope.lifecycle.as_str(), "" | "live" | "trash") {
        return Err(DataError("Unsupported lifecycle scope".into()));
    }
    let lifecycle = if plan.scope.lifecycle.is_empty() {
        "live"
    } else {
        &plan.scope.lifecycle
    };
    let mut vars = symbols(schema)?;
    for (name, shape) in parameter_shapes {
        if vars
            .insert(name.clone(), shape_type(schema, shape))
            .is_some()
        {
            return Err(DataError(
                "Parameter collides with a reserved product/field".into(),
            ));
        }
    }
    let unnest = plan
        .unnest
        .as_ref()
        .map(|u| {
            if vars.contains_key(&u.binding) || u.binding.is_empty() {
                return Err(DataError("Unnest binding collides with symbol".into()));
            }
            let c = compile(&u.array, schema, &vars)?;
            let QType::List(item) = &c.ty else {
                return Err(DataError("Unnest requires array".into()));
            };
            vars.insert(u.binding.clone(), *item.clone());
            Ok(c)
        })
        .transpose()?;
    let predicate = plan
        .r#where
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| compile(s, schema, &vars))
        .transpose()?;
    if predicate.as_ref().is_some_and(|p| p.ty != scalar("bool")) {
        return Err(DataError("where must return bool".into()));
    }
    let columns = if plan.select.is_empty() {
        vec![
            QueryColumn {
                name: "title".into(),
                expr: "title".into(),
            },
            QueryColumn {
                name: "types".into(),
                expr: "types".into(),
            },
        ]
    } else {
        plan.select.clone()
    };
    let selects = columns
        .iter()
        .map(|c| compile(&c.expr, schema, &vars))
        .collect::<Result<Vec<_>>>()?;
    let orders = plan
        .order
        .iter()
        .map(|o| {
            if !matches!(o.nulls.as_str(), "first" | "last") {
                return Err(DataError("Sort nulls must be first/last".into()));
            }
            let compiled=compile(&o.expr, schema, &vars)?;
            if !orderable(&compiled.ty){return Err(DataError("Sort needs one declared scalar comparator; record/array/precision unions need an explicit provider".into()));}
            Ok(compiled)
        })
        .collect::<Result<Vec<_>>>()?;
    let (group_keys, aggregates) = if let Some(g) = &plan.group {
        let k = g
            .keys
            .iter()
            .map(|k| compile(&k.expr, schema, &vars))
            .collect::<Result<Vec<_>>>()?;
        let a = g
            .aggregates
            .iter()
            .map(|a| {
                if !matches!(a.op.as_str(), "count" | "sum" | "min" | "max") {
                    return Err(DataError("Unknown aggregate".into()));
                }
                if a.op == "count" {
                    if a.expr.is_some(){return Err(DataError("count is the explicit row-count aggregate and takes no expression".into()));}
                    Ok(None)
                } else {
                    let compiled=compile(
                        a.expr
                            .as_deref()
                            .ok_or_else(|| DataError("Aggregate expression required".into()))?,
                        schema,
                        &vars,
                    )?;
                    if a.op=="sum"&&!matches!(&compiled.ty,QType::Scalar(s)if matches!(s.as_str(),"int"|"decimal"|"exact_decimal")){return Err(DataError("sum needs int or exact decimal without implicit promotion".into()));}
                    Ok(Some(compiled))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        (k, a)
    } else {
        (vec![], vec![])
    };
    let mut names = BTreeSet::new();
    for c in &columns {
        if c.name.is_empty() || !names.insert(&c.name) {
            return Err(DataError("Duplicate projection column".into()));
        }
    }
    let wanted = plan
        .types
        .iter()
        .map(|t| {
            schema
                .type_id(t)
                .map(str::to_owned)
                .ok_or_else(|| DataError("Unknown Query type".into()))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut notes: Vec<_> = input.notes.iter().collect();
    notes.sort_by(|a, b| a.entity_key.cmp(&b.entity_key));
    let mut rows = vec![];
    let mut diagnostics = vec![];
    let mut partial = input.coverage == "partial";
    let mut env = query_env_traced(schema, trace)?;
    env.parameter_shapes = parameter_shapes.clone();
    if trace.is_some() {
        for expression in unnest
            .iter()
            .chain(predicate.iter())
            .chain(&selects)
            .chain(&orders)
            .chain(&group_keys)
            .chain(aggregates.iter().flatten())
        {
            preflight_provider_expression(expression, parameters, schema, remaining, &env)?;
        }
    }
    let mut bytes = 0usize;
    let mut data_rows = vec![];
    'notes: for note in notes {
        if note.lifecycle != lifecycle || !in_directory(&note.path, &plan.scope) {
            continue;
        }
        bytes = bytes.saturating_add(query_note_bytes(note, metadata)?);
        if bytes > limits.max_source_bytes || data_rows.len() >= limits.max_rows {
            partial = true;
            diagnostics.push("host-budget-exhausted".into());
            break;
        }
        let projection = query_projection(note, schema, metadata);
        if projection.qualification != "qualified" {
            partial = true;
            diagnostics.push(format!(
                "{}: control projection unavailable",
                note.entity_key
            ));
            diagnostics.extend(
                projection
                    .diagnostics
                    .iter()
                    .filter(|reason| reason.contains("budget"))
                    .cloned(),
            );
            continue;
        }
        if !projection.diagnostics.is_empty() {
            partial = true;
            diagnostics.push(format!(
                "{}: type closure invalid/unavailable",
                note.entity_key
            ));
        }
        if !wanted.iter().all(|t| projection.resolved_types.contains(t)) {
            continue;
        }
        let mut cells = projection
            .fields
            .iter()
            .map(|f| (f.key.clone(), f.cell.clone()))
            .collect::<BTreeMap<_, _>>();
        // Fields outside the applicable Type closure are absent, not arbitrary
        // unknown authored members with a namesake key.
        for f in &schema.fields {
            cells.entry(f.key.clone()).or_insert(Cell::Absent);
        }
        cells.insert(
            "self".into(),
            Cell::value(NativeValue::Text(note.entity_key.clone())),
        );
        cells.insert(
            "types".into(),
            Cell::value(NativeValue::Array(
                projection
                    .types
                    .iter()
                    .cloned()
                    .map(NativeValue::Text)
                    .collect(),
            )),
        );
        if note.authorized_fields.is_some() {
            cells.insert(
                "self".into(),
                metadata
                    .get(&note.entity_key)
                    .and_then(|m| m.identity.clone())
                    .map(NativeValue::Text)
                    .map(Cell::value)
                    .unwrap_or(Cell::Unavailable {
                        reason: "identity-not-authorized".into(),
                    }),
            );
            cells.insert(
                "types".into(),
                metadata
                    .get(&note.entity_key)
                    .and_then(|m| m.types.clone())
                    .map(|types| {
                        Cell::value(NativeValue::Array(
                            types.into_iter().map(NativeValue::Text).collect(),
                        ))
                    })
                    .unwrap_or(Cell::Unavailable {
                        reason: "types-not-authorized".into(),
                    }),
            );
        }
        cells.insert(
            "title".into(),
            note.title
                .clone()
                .map(NativeValue::Text)
                .map(Cell::value)
                .unwrap_or(Cell::Unavailable {
                    reason: "qualified-title-product-unavailable".into(),
                }),
        );
        cells.insert(
            "body".into(),
            note.body
                .clone()
                .map(NativeValue::Text)
                .map(Cell::value)
                .unwrap_or(Cell::Unavailable {
                    reason: "qualified-body-product-unavailable".into(),
                }),
        );
        let products = note
            .products
            .as_ref()
            .filter(|p| p.target == "html" && !p.compile_identity.is_empty());
        if let Some(products) = products {
            cells.insert("title".into(), products.title.clone());
            cells.insert("body".into(), products.body.clone());
            cells.insert("headings".into(), products.headings.clone());
        } else {
            if note.products.is_some() {
                cells.insert(
                    "title".into(),
                    Cell::Unavailable {
                        reason: "qualified-html-product-unavailable".into(),
                    },
                );
                cells.insert(
                    "body".into(),
                    Cell::Unavailable {
                        reason: "qualified-html-product-unavailable".into(),
                    },
                );
            }
            cells.insert(
                "headings".into(),
                Cell::Unavailable {
                    reason: "qualified-html-headings-unavailable".into(),
                },
            );
        }
        cells.extend(parameters.clone());
        let expanded = if let (Some(u), Some(c)) = (&plan.unnest, &unnest) {
            match evaluate(c, &cells, schema, remaining, &env) {
                Ok(Cell::Value {
                    value: NativeValue::Array(a),
                }) => a
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let mut c = cells.clone();
                        c.insert(u.binding.clone(), Cell::value(v));
                        (format!("{}/item/{i}", note.entity_key), c)
                    })
                    .collect(),
                Ok(_) => {
                    partial = true;
                    diagnostics.push("unnest input undecided".into());
                    continue;
                }
                Err(e) => {
                    partial = true;
                    diagnostics.push(e.0);
                    break;
                }
            }
        } else {
            vec![(note.entity_key.clone(), cells)]
        };
        for (key, cells) in expanded {
            if data_rows.len() >= limits.max_rows {
                partial = true;
                diagnostics.push("host-budget-exhausted".into());
                break 'notes;
            }
            if let Some(p) = &predicate {
                match evaluate(p, &cells, schema, remaining, &env) {
                    Ok(Cell::Value {
                        value: NativeValue::Bool(true),
                    }) => {}
                    Ok(Cell::Value {
                        value: NativeValue::Bool(false),
                    }) => continue,
                    Ok(_) => {
                        partial = true;
                        diagnostics.push(format!("{key}: filter undecided"));
                        continue;
                    }
                    Err(e) => {
                        partial = true;
                        diagnostics.push(e.0);
                        break 'notes;
                    }
                }
            }
            let eval =
                |c: &Compiled, remaining: &mut u64| evaluate(c, &cells, schema, remaining, &env);
            let values = selects
                .iter()
                .map(|s| eval(s, remaining))
                .collect::<Result<Vec<_>>>();
            let order = orders
                .iter()
                .map(|s| eval(s, remaining))
                .collect::<Result<Vec<_>>>();
            let keys = group_keys
                .iter()
                .map(|s| eval(s, remaining))
                .collect::<Result<Vec<_>>>();
            let agg = aggregates
                .iter()
                .map(|s| s.as_ref().map(|s| eval(s, remaining)).transpose())
                .collect::<Result<Vec<_>>>();
            let (values, order, keys, agg) = match (values, order, keys, agg) {
                (Ok(v), Ok(o), Ok(k), Ok(a)) => (v, o, k, a),
                _ => {
                    partial = true;
                    diagnostics.push("host-budget-exhausted".into());
                    break 'notes;
                }
            };
            if order
                .iter()
                .any(|c| matches!(c, Cell::Unavailable { .. } | Cell::Invalid { .. }))
            {
                partial = true;
                diagnostics.push(format!("{key}: sort input undecided"));
            }
            let row = QueryRow {
                key,
                entity_key: Some(note.entity_key.clone()),
                path: Some(note.path.clone()),
                revision: Some(note.revision.clone()),
                cells: columns
                    .iter()
                    .zip(values)
                    .map(|(c, v)| (c.name.clone(), v))
                    .collect(),
                read_only: lifecycle == "trash" || plan.unnest.is_some(),
                lifecycle: Some(note.lifecycle.clone()),
            };
            data_rows.push((row, order, keys, agg));
        }
    }
    if let Some(group) = &plan.group {
        let mut grouped: BTreeMap<String, GroupBucket> = BTreeMap::new();
        for (_, _, keys, agg) in data_rows {
            if keys
                .iter()
                .any(|k| matches!(k, Cell::Invalid { .. } | Cell::Unavailable { .. }))
            {
                partial = true;
                diagnostics.push("group key undecided".into());
                continue;
            }
            let k = match canonical_cells(&keys, &group_keys) {
                Ok(k) => k,
                Err(e) => {
                    partial = true;
                    diagnostics.push(e.0);
                    continue;
                }
            };
            grouped
                .entry(k)
                .or_insert_with(|| (keys, vec![]))
                .1
                .push(agg);
        }
        if group.keys.is_empty() && grouped.is_empty() {
            grouped.insert("[]".into(), (vec![], vec![]));
        }
        for (key, (keys, values)) in grouped {
            let mut cells = group
                .keys
                .iter()
                .zip(keys)
                .map(|(k, v)| (k.name.clone(), v))
                .collect::<BTreeMap<_, _>>();
            for (i, aggregate) in group.aggregates.iter().enumerate() {
                let values = values.iter().map(|r| r[i].as_ref()).collect::<Vec<_>>();
                let cell = aggregate_cell(
                    &aggregate.op,
                    &values,
                    aggregates[i].as_ref().map(|c| &c.ty),
                );
                if matches!(cell, Cell::Invalid { .. } | Cell::Unavailable { .. }) {
                    partial = true;
                }
                cells.insert(aggregate.name.clone(), cell);
            }
            rows.push(QueryRow {
                key: format!("group:{key}"),
                entity_key: None,
                path: None,
                revision: None,
                cells,
                read_only: true,
                lifecycle: None,
            });
        }
    } else {
        data_rows.sort_by(|(a, ak, _, _), (b, bk, _, _)| {
            for (((a, b), order), compiled) in ak.iter().zip(bk).zip(&plan.order).zip(&orders) {
                let mut c = compare_cell(a, b, &order.nulls, &compiled.ty);
                if order.descending && a.native().is_some() && b.native().is_some() {
                    c = c.reverse();
                }
                if !c.is_eq() {
                    return c;
                }
            }
            a.key.cmp(&b.key)
        });
        rows = data_rows.into_iter().map(|(r, _, _, _)| r).collect();
    }
    if env.provider_partial.load(AtomicOrdering::Relaxed) {
        partial = true;
        diagnostics.push("temporal expansion partial".into());
    }
    if env.native_budget_partial.load(AtomicOrdering::Relaxed) {
        partial = true;
        diagnostics.push("native-decoder traversal budget exhausted in consumed field".into());
    }
    let total_known = rows.len();
    let limited = plan.take.is_some_and(|n| n < rows.len());
    if let Some(n) = plan.take {
        rows.truncate(n);
    }
    Ok(QueryResult {
        cut_id: input.cut_id.clone(),
        schema_revision: input.schema_revision.clone(),
        rows,
        coverage: if partial { "partial" } else { "complete" }.into(),
        freshness: input.freshness.clone(),
        diagnostics,
        total_known,
        limited,
        aggregate_exact: !partial,
        runtime: "cel-rust 0.15.0; weftext.query.cel.v1".into(),
        terminal_schema: if let Some(group) = &plan.group {
            group
                .keys
                .iter()
                .zip(&group_keys)
                .map(|(k, c)| (k.name.clone(), c.ty.clone()))
                .chain(group.aggregates.iter().zip(&aggregates).map(|(a, c)| {
                    (
                        a.name.clone(),
                        c.as_ref().map_or_else(|| scalar("int"), |c| c.ty.clone()),
                    )
                }))
                .collect()
        } else {
            columns
                .iter()
                .zip(&selects)
                .map(|(c, s)| (c.name.clone(), s.ty.clone()))
                .collect()
        },
    })
}
fn in_directory(path: &str, scope: &QueryScope) -> bool {
    let Some(dir) = &scope.directory else {
        return true;
    };
    let dir = dir.replace('\\', "/").trim_end_matches('/').to_string();
    let path = path.replace('\\', "/");
    let parent = path.rsplit_once('/').map_or("", |(p, _)| p);
    parent == dir
        || scope.descendants
            && parent
                .strip_prefix(&dir)
                .is_some_and(|rest| rest.starts_with('/'))
}
pub(crate) fn orderable(ty: &QType) -> bool {
    matches!(ty,QType::Scalar(s)if !matches!(s.as_str(),"none"|"schema_ref"))
}
fn compare_typed(a: &NativeValue, b: &NativeValue, ty: &QType) -> Result<Ordering> {
    match ty {
        QType::Scalar(s) if s == "exact_decimal" || s == "exact_seconds" => {
            Ok(ExactDecimal::parse(
                a.text()
                    .ok_or_else(|| DataError("Expected exact decimal".into()))?,
            )?
            .compare(&ExactDecimal::parse(
                b.text()
                    .ok_or_else(|| DataError("Expected exact decimal".into()))?,
            )?))
        }
        QType::Scalar(s) if s == "instant" => Ok(ExactInstant::parse(
            a.text()
                .ok_or_else(|| DataError("Expected instant".into()))?,
        )?
        .compare(&ExactInstant::parse(
            b.text()
                .ok_or_else(|| DataError("Expected instant".into()))?,
        )?)),
        _ => compare_native(a, b),
    }
}
pub(crate) fn compare_cell(a: &Cell, b: &Cell, nulls: &str, ty: &QType) -> Ordering {
    match (a.native(), b.native()) {
        (Some(a), Some(b)) => compare_typed(a, b, ty).unwrap_or(Ordering::Equal),
        (None, None) => a.state().cmp(b.state()),
        (None, Some(_)) => {
            if nulls == "first" {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (Some(_), None) => {
            if nulls == "first" {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
    }
}
pub(crate) fn canonical_cells(c: &[Cell], compiled: &[Compiled]) -> Result<String> {
    Ok(serde_json::to_string(
        &c.iter()
            .zip(compiled)
            .map(|(v, c)| match v {
                Cell::Value { value } => canonical_native(value, &c.ty),
                v => Ok(serde_json::to_string(v)?),
            })
            .collect::<Result<Vec<_>>>()?,
    )?)
}
pub(crate) fn canonical_native(v: &NativeValue, ty: &QType) -> Result<String> {
    if let QType::Union(variants) = ty {
        let matching = variants
            .iter()
            .filter(|t| matches!(t,QType::Scalar(s)if scalar_valid(s,v)))
            .collect::<Vec<_>>();
        if matching.len() == 1 {
            return canonical_native(v, matching[0]);
        }
        let structural = variants
            .iter()
            .filter(|t| {
                matches!(
                    (t, v),
                    (QType::Record(_), NativeValue::Record(_))
                        | (QType::List(_), NativeValue::Array(_))
                )
            })
            .collect::<Vec<_>>();
        if structural.len() == 1 {
            return canonical_native(v, structural[0]);
        }
        fn exact_text(t: &QType) -> bool {
            match t {
                QType::Scalar(s) => {
                    matches!(s.as_str(), "exact_decimal" | "exact_seconds" | "instant")
                }
                QType::List(t) | QType::Map(t) => exact_text(t),
                QType::Record(m) => m.values().any(exact_text),
                QType::Union(v) => v.iter().any(exact_text),
                _ => false,
            }
        }
        if variants.iter().any(exact_text) {
            return Err(DataError(
                "typed-union equality requires an available selected schema branch".into(),
            ));
        }
    }
    if matches!(ty,QType::Scalar(s)if matches!(s.as_str(),"decimal"|"exact_decimal"|"exact_seconds"))
    {
        let s = match v {
            NativeValue::Decimal(s) | NativeValue::Text(s) => s,
            _ => return Err(DataError("Invalid decimal key".into())),
        };
        let n = ExactDecimal::parse(s)?;
        let mut coefficient = n.coefficient;
        let mut scale = n.scale;
        while scale > 0
            && &coefficient % num_bigint::BigInt::from(10) == num_bigint::BigInt::from(0)
        {
            coefficient /= 10;
            scale -= 1;
        }
        return Ok(format!("decimal:{coefficient}:{scale}"));
    }
    if matches!(ty,QType::Scalar(s)if s=="instant") {
        let n = ExactInstant::parse(
            v.text()
                .ok_or_else(|| DataError("Invalid instant key".into()))?,
        )?;
        let fraction = canonical_native(
            &NativeValue::Text(n.fraction.spelling()),
            &scalar("exact_decimal"),
        )?;
        return Ok(format!("instant:{}:{fraction}", n.utc_seconds));
    }
    match v {
        NativeValue::Array(a) => {
            let item = if let QType::List(t) = ty {
                t.as_ref()
            } else {
                &QType::Any
            };
            Ok(format!(
                "array:{}",
                serde_json::to_string(
                    &a.iter()
                        .map(|v| canonical_native(v, item))
                        .collect::<Result<Vec<_>>>()?
                )?
            ))
        }
        NativeValue::Record(a) => {
            let mut members = BTreeMap::new();
            for m in a {
                let member = if let QType::Record(t) = ty {
                    t.get(&m.key).unwrap_or(&QType::Any)
                } else {
                    &QType::Any
                };
                members.insert(&m.key, canonical_native(&m.value, member)?);
            }
            Ok(format!("record:{}", serde_json::to_string(&members)?))
        }
        NativeValue::Decimal(_) => canonical_native(v, &scalar("decimal")),
        NativeValue::Float(s) if s.parse::<f64>().is_ok_and(|f| f == 0.0) => Ok("float:0".into()),
        _ => Ok(serde_json::to_string(v)?),
    }
}
pub(crate) fn aggregate_cell(op: &str, values: &[Option<&Cell>], ty: Option<&QType>) -> Cell {
    if op == "count" {
        return Cell::value(NativeValue::Int(values.len().to_string()));
    }
    let vals = values.iter().filter_map(|v| *v).collect::<Vec<_>>();
    if vals.iter().any(|v| {
        matches!(
            v,
            Cell::Unavailable { .. } | Cell::Invalid { .. } | Cell::Absent
        )
    }) {
        return Cell::Unavailable {
            reason: "aggregate input undecided".into(),
        };
    }
    let vals = vals.iter().filter_map(|v| v.native()).collect::<Vec<_>>();
    if vals.is_empty() {
        return Cell::Absent;
    }
    if op == "sum" {
        if matches!(ty,Some(QType::Scalar(s))if s=="exact_decimal") {
            let sum = vals
                .iter()
                .try_fold(ExactDecimal::parse("0").unwrap(), |a, v| {
                    v.text()
                        .and_then(|s| ExactDecimal::parse(s).ok())
                        .map(|b| a.add(&b))
                });
            return sum
                .map(|s| Cell::value(NativeValue::Text(s.spelling())))
                .unwrap_or(Cell::Unavailable {
                    reason: "exact decimal sum type mismatch".into(),
                });
        }
        let first = vals[0];
        return match first {
            NativeValue::Int(_) => {
                let sum = vals
                    .iter()
                    .try_fold(0i64, |a, v| a.checked_add(v.integer()?));
                sum.map(|s| Cell::value(NativeValue::Int(s.to_string())))
                    .unwrap_or(Cell::Invalid {
                        diagnostics: vec!["i64 sum overflow/type mismatch".into()],
                        value: None,
                    })
            }
            NativeValue::Decimal(_) => {
                let sum = vals
                    .iter()
                    .try_fold(ExactDecimal::parse("0").unwrap(), |a, v| match v {
                        NativeValue::Decimal(s) => ExactDecimal::parse(s).ok().map(|b| a.add(&b)),
                        _ => None,
                    });
                sum.map(|s| Cell::value(NativeValue::Decimal(s.spelling())))
                    .unwrap_or(Cell::Unavailable {
                        reason: "decimal sum type mismatch".into(),
                    })
            }
            _ => Cell::Invalid {
                diagnostics: vec![
                    "sum supports int/native exact decimal without conversion".into(),
                ],
                value: None,
            },
        };
    }
    let mut chosen = vals[0];
    for v in &vals[1..] {
        match compare_typed(v, chosen, ty.unwrap_or(&QType::Any)) {
            Ok(c) if op == "min" && c.is_lt() || op == "max" && c.is_gt() => chosen = v,
            Ok(_) => {}
            Err(_) => {
                return Cell::Unavailable {
                    reason: "aggregate comparator unavailable".into(),
                };
            }
        }
    }
    Cell::value(chosen.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedQuery {
    pub key: String,
    pub label: String,
    pub binding: SavedQueryBinding,
    pub plan: QueryPlan,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedQueryBinding {
    pub schema: String,
    pub revision: String,
    pub language: String,
}
/// Direct definitions remain owner-local. Current/pinned selection is resolved
/// from supplied immutable history and never falls back to a latest namesake.
pub fn saved_queries(source: &str) -> Result<Vec<SavedQuery>> {
    use typst_syntax::{
        Source,
        ast::{self, AstNode},
    };
    let parsed = Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return Err(DataError("Incomplete saved-definition source".into()));
    }
    let labels = parsed
        .root()
        .children()
        .filter_map(|n| n.cast::<ast::Label>())
        .map(|l| l.get().to_string())
        .filter(|l| l.starts_with("weftext-query-"))
        .collect::<Vec<_>>();
    let mut keys = BTreeSet::new();
    let mut out = vec![];
    for label in labels {
        let Some(range) = weftext_engine::certify_metadata_declaration(source, &label) else {
            return Err(DataError("Saved Query binding unavailable".into()));
        };
        let call = parsed
            .root()
            .children()
            .filter_map(|n| n.cast::<ast::FuncCall>())
            .find(|c| {
                parsed
                    .find(c.to_untyped().span())
                    .is_some_and(|n| n.range().start == range.from)
            })
            .ok_or_else(|| DataError("Missing Query call".into()))?;
        let expr = call
            .args()
            .items()
            .find_map(|a| {
                if let ast::Arg::Pos(e) = a {
                    Some(e)
                } else {
                    None
                }
            })
            .ok_or_else(|| DataError("Missing Query dictionary".into()))?;
        let dictionary = expr
            .to_untyped()
            .cast::<ast::Dict>()
            .ok_or_else(|| DataError("Saved Query requires a direct native dictionary".into()))?;
        for item in dictionary.items() {
            let (key, value) = match item {
                ast::DictItem::Named(n) => (n.name().get().to_string(), n.expr()),
                ast::DictItem::Keyed(n) => {
                    let key =
                        n.key().to_untyped().cast::<ast::Str>().ok_or_else(|| {
                            DataError("Saved Query key syntax is computed".into())
                        })?;
                    (key.get().to_string(), n.expr())
                }
                _ => return Err(DataError("Saved Query dictionary is computed".into())),
            };
            if matches!(key.as_str(), "kind" | "key")
                && value.to_untyped().cast::<ast::Str>().is_none()
            {
                return Err(DataError(
                    "Saved Query kind/key require direct string controls".into(),
                ));
            }
        }
        let value = crate::native::decode_node(expr.to_untyped(), &parsed)
            .value
            .ok_or_else(|| DataError("Saved Query is computed".into()))?;
        let record = value
            .record()
            .ok_or_else(|| DataError("Saved Query requires dictionary".into()))?;
        let allowed = [
            "kind", "key", "binding", "scope", "types", "where", "select", "order", "take",
            "group", "unnest", "advanced",
        ];
        if record.iter().any(|m| !allowed.contains(&m.key.as_str())) {
            return Err(DataError("Unknown saved Query member".into()));
        }
        if value.member("kind").and_then(NativeValue::text) != Some("weftext.query") {
            return Err(DataError("Incorrect Query role".into()));
        }
        let key = value
            .member("key")
            .and_then(NativeValue::text)
            .filter(|s| uuid_v4(s))
            .ok_or_else(|| DataError("Saved Query key must be UUIDv4".into()))?
            .to_string();
        if !keys.insert(key.clone()) {
            return Err(DataError("Duplicate saved Query key".into()));
        }
        let binding: SavedQueryBinding = serde_json::from_value(native_json(
            value
                .member("binding")
                .ok_or_else(|| DataError("Missing schema binding".into()))?,
        )?)?;
        if binding.language != "weftext.query.cel.v1"
            || !scalar_valid("schema_ref", &NativeValue::Text(binding.schema.clone()))
            || !scalar_valid(
                "config_revision",
                &NativeValue::Text(binding.revision.clone()),
            )
        {
            return Err(DataError("Unsupported saved Query baseline/profile".into()));
        }
        let mut json = crate::query_advanced::saved_plan_json(&value)?;
        json.as_object_mut()
            .unwrap()
            .retain(|k, _| !matches!(k.as_str(), "kind" | "key" | "binding"));
        if let Some(scope) = json
            .get_mut("scope")
            .and_then(serde_json::Value::as_object_mut)
            && scope.remove("library").is_some_and(|v| v != "current")
        {
            return Err(DataError(
                "Saved scope library must be current owner".into(),
            ));
        }
        let plan: QueryPlan = serde_json::from_value(json)?;
        out.push(SavedQuery {
            key,
            label,
            binding,
            plan,
        });
    }
    Ok(out)
}
fn native_json(v: &NativeValue) -> Result<serde_json::Value> {
    Ok(match v {
        NativeValue::None => serde_json::Value::Null,
        NativeValue::Text(s) => s.clone().into(),
        NativeValue::Bool(b) => (*b).into(),
        NativeValue::Int(s) => s
            .parse::<i64>()
            .map_err(|_| DataError("Invalid int".into()))?
            .into(),
        NativeValue::Array(a) => {
            serde_json::Value::Array(a.iter().map(native_json).collect::<Result<Vec<_>>>()?)
        }
        NativeValue::Record(a) => serde_json::Value::Object(
            a.iter()
                .map(|m| Ok((m.key.clone(), native_json(&m.value)?)))
                .collect::<Result<serde_json::Map<_, _>>>()?,
        ),
        _ => {
            return Err(DataError(
                "Saved Query structure requires native text/int/bool/record/list".into(),
            ));
        }
    })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum DefinitionSelection {
    Current { key: String },
    Pinned { key: String, revision: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionRevision {
    pub revision: String,
    pub source: String,
    pub current: bool,
}
pub fn select_definition(
    selection: &DefinitionSelection,
    history: &[DefinitionRevision],
) -> Result<SavedQuery> {
    let (key, revision) = match selection {
        DefinitionSelection::Current { key } => (key, None),
        DefinitionSelection::Pinned { key, revision } => (key, Some(revision)),
    };
    let selected = history
        .iter()
        .filter(|v| revision.map_or(v.current, |r| &v.revision == r))
        .collect::<Vec<_>>();
    if selected.len() != 1 {
        return Err(DataError(
            "definition_unavailable: exact revision not retained/ambiguous".into(),
        ));
    }
    saved_queries(&selected[0].source)?
        .into_iter()
        .find(|v| &v.key == key)
        .ok_or_else(|| DataError("definition_unavailable: key missing at selected revision".into()))
}

/// Insert a Core-reserved definition key or replace its exact direct declaration.
/// Core must verify the supplied binding revision is installed and preserve the
/// original key for updates. This transformation performs no filesystem IO.
pub fn upsert_saved_query(
    source: &str,
    existing_key: Option<&str>,
    key: &str,
    label: &str,
    binding: &SavedQueryBinding,
    plan: &QueryPlan,
    schema: &Schema,
) -> Result<FieldPatch> {
    upsert_saved_query_impl(
        source,
        existing_key,
        key,
        label,
        binding,
        plan,
        schema,
        None,
        true,
    )
}
/// The host supplies the exact callee/config closure frozen for this save.
#[allow(clippy::too_many_arguments)]
pub fn upsert_saved_query_with_context(
    source: &str,
    existing_key: Option<&str>,
    key: &str,
    label: &str,
    binding: &SavedQueryBinding,
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
) -> Result<FieldPatch> {
    upsert_saved_query_impl(
        source,
        existing_key,
        key,
        label,
        binding,
        plan,
        schema,
        Some(context),
        true,
    )
}
/// Reconstruct only a previously decided save. Core must compare the exact
/// returned bytes with its retained original B and replay the original opId /
/// fingerprint. This never validates against newly observed callee definitions.
pub fn upsert_saved_query_replay(
    source: &str,
    existing_key: Option<&str>,
    key: &str,
    label: &str,
    binding: &SavedQueryBinding,
    plan: &QueryPlan,
    schema: &Schema,
) -> Result<FieldPatch> {
    upsert_saved_query_impl(
        source,
        existing_key,
        key,
        label,
        binding,
        plan,
        schema,
        None,
        false,
    )
}
#[allow(clippy::too_many_arguments)]
fn upsert_saved_query_impl(
    source: &str,
    existing_key: Option<&str>,
    key: &str,
    label: &str,
    binding: &SavedQueryBinding,
    plan: &QueryPlan,
    schema: &Schema,
    context: Option<&FrozenAdvancedInput>,
    validate: bool,
) -> Result<FieldPatch> {
    if !uuid_v4(key) || existing_key.is_some_and(|old| old != key) {
        return Err(DataError(
            "Saved definition key must be Core-reserved and unchanged".into(),
        ));
    }
    if !label.starts_with("weftext-query-")
        || label.len() <= "weftext-query-".len()
        || !label
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(DataError(
            "Saved Query label requires reserved prefix and a native alphanumeric name".into(),
        ));
    }
    if binding.language != "weftext.query.cel.v1"
        || !scalar_valid("schema_ref", &NativeValue::Text(binding.schema.clone()))
        || !scalar_valid(
            "config_revision",
            &NativeValue::Text(binding.revision.clone()),
        )
    {
        return Err(DataError(
            "Saved Query requires an exact schema/profile binding".into(),
        ));
    }
    if validate {
        if let Some(context) = context {
            validate_query_plan_with_context(plan, schema, context)?;
        } else {
            validate_query_plan(plan, schema)?;
        }
    }
    let definitions = saved_queries(source)?;
    if definitions.iter().any(|d| d.key != key && d.label == label) {
        return Err(DataError(
            "Saved Query label conflicts with another definition".into(),
        ));
    }
    let mut json = serde_json::to_value(plan)?;
    let object = json.as_object_mut().unwrap();
    object.retain(|_, v| !v.is_null());
    object.insert("kind".into(), "weftext.query".into());
    object.insert("key".into(), key.into());
    object.insert("binding".into(), serde_json::to_value(binding)?);
    object
        .get_mut("scope")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("library".into(), "current".into());
    let mut native = crate::query_advanced::saved_plan_native(&json)?;
    // Optional fields are absent native members; never persist fabricated none.
    if let NativeValue::Record(members) = &mut native {
        members.retain(|m| !matches!(m.value, NativeValue::None));
    }
    let declaration = format!("metadata({}) <{}>", native.to_typst()?, label);
    let (from, to, text) = if let Some(old_key) = existing_key {
        let old = definitions
            .iter()
            .find(|d| d.key == old_key)
            .ok_or_else(|| DataError("Selected definition is not current in this source".into()))?;
        let range = weftext_engine::certify_metadata_declaration(source, &old.label)
            .ok_or_else(|| DataError("Definition writer binding unavailable".into()))?;
        (range.from, range.to, declaration)
    } else {
        if definitions.iter().any(|d| d.key == key) {
            return Err(DataError("Definition key already exists".into()));
        }
        // Keep root identity before every other application declaration. The
        // saved role's safe-prefix proof separately checks its static tree.
        let root =
            weftext_engine::certify_metadata_declaration(source, "weftext").ok_or_else(|| {
                DataError("Saved definition insertion requires qualified root control".into())
            })?;
        let from = root.to;
        (from, from, format!("\n#{declaration}\n"))
    };
    let patched = format!("{}{}{}", &source[..from], text, &source[to..]);
    let after = saved_queries(&patched)?;
    if !after.iter().any(|d| d.key == key && d.label == label) {
        return Err(DataError(
            "Saved Query writer failed official AST readback".into(),
        ));
    }
    // Existing root control must retain its qualification and identity.
    let before_root = project(source, schema);
    let after_root = project(&patched, schema);
    if before_root.qualification == "qualified"
        && (after_root.qualification != "qualified" || before_root.id != after_root.id)
    {
        return Err(DataError(
            "Definition edit invalidated root control qualification".into(),
        ));
    }
    Ok(FieldPatch {
        from,
        to,
        text,
        source: patched,
    })
}
pub fn validate_query_plan(plan: &QueryPlan, schema: &Schema) -> Result<()> {
    // Validation uses the exact same plan compiler as execution, without reading
    // any rows or claiming a current result. No body/title facts are supplied.
    let input = FrozenQueryInput {
        cut_id: "validation-only".into(),
        schema_revision: "validation-only".into(),
        coverage: "complete".into(),
        freshness: "stale".into(),
        notes: vec![],
        advanced: None,
    };
    crate::query_advanced::validate_advanced_plan(&input, plan, schema)
}
pub(crate) fn json_native(value: &serde_json::Value) -> Result<NativeValue> {
    Ok(match value {
        serde_json::Value::Null => NativeValue::None,
        serde_json::Value::Bool(b) => NativeValue::Bool(*b),
        serde_json::Value::String(s) => NativeValue::Text(s.clone()),
        serde_json::Value::Number(n) => NativeValue::Int(
            n.as_i64()
                .ok_or_else(|| DataError("Query structure accepts only i64 numeric values".into()))?
                .to_string(),
        ),
        serde_json::Value::Array(a) => {
            NativeValue::Array(a.iter().map(json_native).collect::<Result<Vec<_>>>()?)
        }
        serde_json::Value::Object(o) => NativeValue::Record(
            o.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(key, value)| {
                    Ok(NativeMember {
                        key: key.clone(),
                        value: json_native(value)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        ),
    })
}
