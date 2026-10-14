use crate::{DataError, ExactDecimal, NativeValue, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    Scalar {
        semantic: String,
    },
    Enum {
        values: Vec<String>,
        open: bool,
    },
    List {
        items: Box<Shape>,
        min: usize,
        max: Option<usize>,
    },
    Record {
        members: BTreeMap<String, Member>,
        unknown: String,
        constraints: Vec<Constraint>,
    },
    Union {
        variants: Vec<Shape>,
        discriminator: Option<Discriminator>,
    },
    Use {
        name: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Discriminator {
    Member {
        keys: Vec<String>,
    },
    Tag {
        key: String,
    },
    RecordPresence {
        key: String,
        present: usize,
        absent: usize,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub shape: Shape,
    pub required: bool,
    pub nullable: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub id: String,
    pub key: String,
    pub shape: Shape,
    pub nullable: bool,
    pub default: Option<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Type {
    pub id: String,
    pub fields: Vec<FieldContribution>,
    pub requires: Vec<String>,
    pub conflicts: Vec<String>,
    pub constraints: Vec<Constraint>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldContribution {
    pub id: String,
    pub required: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constraint {
    pub validator: String,
    pub inputs: BTreeMap<String, Selector>,
    pub parameters: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selector {
    Local { path: Vec<SchemaPath> },
    Field { id: String, path: Vec<SchemaPath> },
    Context { name: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SchemaPath {
    Member { key: String },
    Each,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    pub id: String,
    pub implementation: Implementation,
    pub data: Value,
    pub limits: Limits,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Implementation {
    pub kind: String,
    pub id: String,
    pub abi: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub max_steps: u64,
    pub max_bytes: u64,
    pub max_expansion: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Validator {
    pub id: String,
    pub provider: String,
    pub operation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub id: String,
    pub source_type: String,
    pub field: String,
    pub target_path: Vec<SchemaPath>,
    pub target_domain: String,
    pub target_types: Value,
    pub direction: String,
    pub inverse: Option<String>,
    pub max_assertions: Option<usize>,
    pub reject_self: bool,
    pub graph_projection: String,
    pub cycle_policy: String,
    pub resolution: String,
    pub delete: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeriesPolicy {
    pub id: String,
    pub key: String,
    pub calendar: String,
    pub zone: String,
    pub kinds: Vec<String>,
    pub rule: String,
    pub scope: Value,
    pub multiplicity: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub version: u32,
    pub definitions: BTreeMap<String, Shape>,
    pub fields: Vec<Field>,
    pub types: Vec<Type>,
    pub aliases: BTreeMap<String, String>,
    pub relations: Vec<Relation>,
    pub validators: Vec<Validator>,
    pub providers: Vec<Provider>,
    pub series_policies: Vec<SeriesPolicy>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Validation {
    pub diagnostics: Vec<String>,
    pub unavailable: Vec<String>,
    pub unbound: Vec<String>,
}
impl Validation {
    pub fn valid() -> Self {
        Self {
            diagnostics: vec![],
            unavailable: vec![],
            unbound: vec![],
        }
    }
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty() && self.unavailable.is_empty()
    }
    fn merge(&mut self, other: Self) {
        self.diagnostics.extend(other.diagnostics);
        self.unavailable.extend(other.unavailable);
        self.unbound.extend(other.unbound);
    }
}
/// The trusted registration source is code, not schema-provided signatures.
/// Every dispatch below resolves a registered exact implementation/operation.
pub fn builtin_operations(id: &str) -> Option<(u32, &'static [&'static str])> {
    Some(match id {
        "weftext.core.rules.v1" => (
            1,
            &[
                "int-range",
                "decimal-range",
                "custom-code",
                "owned-resources",
            ],
        ),
        "weftext.temporal.rules.v2" => (
            2,
            &[
                "month-day",
                "calendar-date",
                "during",
                "range",
                "period",
                "recurrence",
                "exception",
                "event",
                "containsDate",
                "nextCivilDay",
                "rangeOverlap",
                "rangeDuration",
                "periodBounds",
                "expandRecurrence",
                "compareTemporal",
                "duringDuration",
            ],
        ),
        "weftext.units.rules.v1" => (1, &["quantity", "measurement"]),
        "weftext.calendar.gregorian.v2" | "weftext.calendar.table.v2" => (
            2,
            &[
                "validateDate",
                "periodBounds",
                "civilCandidates",
                "dateBounds",
            ],
        ),
        "weftext.zone.table.v1" => (1, &["toCivil", "toInstant"]),
        _ => return None,
    })
}
const SCALARS: &[&str] = &[
    "text",
    "nonempty_text",
    "phone",
    "email",
    "identifier",
    "doi",
    "url",
    "language_tag",
    "citation_key",
    "node_ref",
    "resource_ref",
    "tag_ref",
    "schema_ref",
    "config_revision",
    "bool",
    "int",
    "float",
    "decimal",
    "exact_decimal",
    "duration",
    "exact_seconds",
    "date",
    "time",
    "local_datetime",
    "instant",
];
impl Schema {
    pub fn from_author_json(bytes: &str) -> Result<Self> {
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(DataError(
                "Schema configuration exceeds installation byte budget".into(),
            ));
        }
        let value: UniqueJson = serde_json::from_str(bytes)?;
        let schema: Self = serde_json::from_value(value.0)?;
        schema.check_installation()?;
        Ok(schema)
    }
    pub fn bundled() -> Result<Self> {
        Self::from_author_json(include_str!(
            "../../../docs/design/examples/schema.authoring.json"
        ))
    }
    pub fn field(&self, id_or_key: &str) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| f.id == id_or_key || f.key == id_or_key)
    }
    pub fn type_id<'a>(&'a self, token: &'a str) -> Option<&'a str> {
        self.aliases.get(token).map(String::as_str).or_else(|| {
            self.types
                .iter()
                .find(|t| t.id == token)
                .map(|t| t.id.as_str())
        })
    }
    pub fn expand_types(&self, tokens: &[String]) -> (BTreeSet<String>, Validation) {
        let mut result = BTreeSet::new();
        let mut check = Validation::valid();
        for token in tokens {
            if let Some(id) = self.type_id(token) {
                if result.contains(id) {
                    check.diagnostics.push(format!("Duplicate type {token}"));
                }
                self.add_type(id, &mut result);
            } else {
                check.unavailable.push(format!("Unknown type {token}"));
            }
        }
        for id in &result {
            let t = self.types.iter().find(|t| &t.id == id).unwrap();
            for c in &t.conflicts {
                if result.contains(c) {
                    check
                        .diagnostics
                        .push(format!("Conflicting types {id} and {c}"));
                }
            }
        }
        (result, check)
    }
    fn add_type(&self, id: &str, out: &mut BTreeSet<String>) {
        if out.insert(id.into())
            && let Some(t) = self.types.iter().find(|t| t.id == id)
        {
            for r in &t.requires {
                self.add_type(r, out);
            }
        }
    }
    pub fn applicable_fields(&self, types: &BTreeSet<String>) -> Vec<(&Field, bool)> {
        let mut required = BTreeMap::new();
        for t in self.types.iter().filter(|t| types.contains(&t.id)) {
            for f in &t.fields {
                *required.entry(&f.id).or_insert(false) |= f.required;
            }
        }
        self.fields
            .iter()
            .filter_map(|f| required.get(&f.id).map(|r| (f, *r)))
            .collect()
    }
    pub fn resolve<'a>(&'a self, mut shape: &'a Shape) -> &'a Shape {
        while let Shape::Use { name } = shape {
            shape = &self.definitions[name];
        }
        shape
    }
    pub fn validate(&self, shape: &Shape, value: &NativeValue) -> Validation {
        if let Err(error) = crate::native::check_native_budget(value) {
            let mut unavailable = Validation::valid();
            unavailable.unavailable.push(error.0);
            return unavailable;
        }
        self.validate_at(shape, value, "$")
    }

    /// Validate a declared input shape against this exact projected baseline.
    /// Query projections intentionally omit author creation defaults and
    /// unrelated installation entries, so they are not schemas to reinstall.
    pub fn check_query_shape(&self, shape: &Shape) -> Result<()> {
        self.check_shape(shape, &mut vec![])
    }
    fn validate_at(&self, shape: &Shape, value: &NativeValue, path: &str) -> Validation {
        let shape = self.resolve(shape);
        let mut out = Validation::valid();
        match shape {
            Shape::Scalar { semantic } => {
                if !scalar_valid(semantic, value) {
                    out.diagnostics.push(format!("{path}: expected {semantic}"));
                }
            }
            Shape::Enum { values, open } => {
                if value
                    .text()
                    .is_none_or(|v| !*open && !values.iter().any(|s| s == v))
                {
                    out.diagnostics.push(format!("{path}: unknown enum value"));
                }
            }
            Shape::List { items, min, max } => match value.array() {
                Some(a) => {
                    if a.len() < *min || max.is_some_and(|m| a.len() > m) {
                        out.diagnostics.push(format!("{path}: list cardinality"));
                    }
                    for (i, v) in a.iter().enumerate() {
                        out.merge(self.validate_at(items, v, &format!("{path}[{i}]")));
                    }
                }
                None => out.diagnostics.push(format!("{path}: expected array")),
            },
            Shape::Record {
                members,
                unknown,
                constraints,
            } => match value.record() {
                Some(a) => {
                    for (k, m) in members {
                        match value.member(k) {
                            None if m.required => out
                                .diagnostics
                                .push(format!("{path}.{k}: required member absent")),
                            Some(NativeValue::None) if m.nullable => {}
                            Some(v) => {
                                out.merge(self.validate_at(&m.shape, v, &format!("{path}.{k}")))
                            }
                            _ => {}
                        }
                    }
                    for m in a {
                        if !members.contains_key(&m.key) {
                            if unknown == "reject" {
                                out.diagnostics
                                    .push(format!("{path}.{}: unknown member", m.key));
                            } else {
                                out.unbound.push(format!("{path}.{}", m.key));
                            }
                        }
                    }
                    if out.diagnostics.is_empty() {
                        for c in constraints {
                            out.merge(self.validate_constraint(c, value));
                        }
                    }
                }
                None => out.diagnostics.push(format!("{path}: expected record")),
            },
            Shape::Union {
                variants,
                discriminator,
            } => match self.union_variant(variants, discriminator.as_ref(), value) {
                Ok(v) => out.merge(self.validate_at(v, value, path)),
                Err(e) => out.diagnostics.push(format!("{path}: {}", e.0)),
            },
            Shape::Use { .. } => unreachable!(),
        }
        out
    }
    pub(crate) fn union_variant<'a>(
        &'a self,
        variants: &'a [Shape],
        discriminator: Option<&Discriminator>,
        v: &NativeValue,
    ) -> Result<&'a Shape> {
        let index = match discriminator {
            Some(Discriminator::Member { keys }) => {
                let matches: Vec<_> = keys
                    .iter()
                    .enumerate()
                    .filter(|(_, k)| v.member(k).is_some())
                    .map(|(i, _)| i)
                    .collect();
                if matches.len() != 1 {
                    return Err(DataError("Union requires exactly one branch member".into()));
                }
                matches[0]
            }
            Some(Discriminator::Tag { key }) => {
                let tag = v
                    .member(key)
                    .and_then(NativeValue::text)
                    .ok_or_else(|| DataError("Missing union tag".into()))?;
                let found:Vec<_>=variants.iter().enumerate().filter(|(_,s)|match self.resolve(s){Shape::Record{members,..}=>matches!(self.resolve(&members[key].shape),Shape::Enum{values,..}if values.iter().any(|x|x==tag)),_=>false}).map(|(i,_)|i).collect();
                if found.len() != 1 {
                    return Err(DataError("Ambiguous union tag".into()));
                }
                found[0]
            }
            Some(Discriminator::RecordPresence {
                key,
                present,
                absent,
            }) if v.record().is_some() => {
                if v.member(key).is_some() {
                    *present
                } else {
                    *absent
                }
            }
            _ => {
                let found: Vec<_> = variants
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| self.native_domain(s) == value_domain(v))
                    .map(|(i, _)| i)
                    .collect();
                if found.len() != 1 {
                    return Err(DataError("Ambiguous union native domain".into()));
                }
                found[0]
            }
        };
        variants
            .get(index)
            .ok_or_else(|| DataError("Invalid union discriminator".into()))
    }
    fn native_domain(&self, shape: &Shape) -> &str {
        match self.resolve(shape) {
            Shape::Scalar { semantic } => match semantic.as_str() {
                "bool" => "bool",
                "int" => "int",
                "float" => "float",
                "decimal" => "decimal",
                "duration" => "duration",
                "date" => "date",
                "time" => "time",
                "local_datetime" => "local_datetime",
                _ => "text",
            },
            Shape::Enum { .. } => "text",
            Shape::Record { .. } => "record",
            Shape::List { .. } => "array",
            _ => "union",
        }
    }
    fn check_installation(&self) -> Result<()> {
        if self.version != 1 {
            return Err(DataError("Unsupported schema version".into()));
        }
        unique(self.fields.iter().map(|f| f.id.as_str()), "field id")?;
        unique(self.fields.iter().map(|f| f.key.as_str()), "field key")?;
        unique(self.types.iter().map(|t| t.id.as_str()), "type")?;
        unique(self.providers.iter().map(|p| p.id.as_str()), "provider")?;
        unique(self.validators.iter().map(|v| v.id.as_str()), "validator")?;
        unique(self.relations.iter().map(|r| r.id.as_str()), "relation")?;
        unique(
            self.series_policies.iter().map(|p| p.id.as_str()),
            "series policy",
        )?;
        for (k, v) in &self.aliases {
            if k.is_empty() || !self.types.iter().any(|t| t.id == *v) {
                return Err(DataError("Unresolved type alias".into()));
            }
        }
        for (name, s) in &self.definitions {
            self.check_shape(s, &mut vec![name.as_str()])?;
        }
        for f in &self.fields {
            if matches!(f.key.as_str(), "id" | "types") || f.key.starts_with("weftext-") {
                return Err(DataError("Reserved field key".into()));
            }
            self.check_shape(&f.shape, &mut vec![])?;
            let default = f
                .default
                .as_ref()
                .ok_or_else(|| DataError("Every Field needs an explicit default policy".into()))?;
            match default.get("kind").and_then(Value::as_str) {
                Some("absent") => exact_members(default, &["kind"])?,
                Some("native") => {
                    exact_members(default, &["kind", "source"])?;
                    let native = crate::decode_native_expression(
                        default["source"]
                            .as_str()
                            .ok_or_else(|| DataError("Native default requires source".into()))?,
                    )?;
                    let check = self.validate(&f.shape, &native);
                    if !check.is_valid() && !(f.nullable && matches!(native, NativeValue::None)) {
                        return Err(DataError(
                            "Invalid/unavailable native creation default".into(),
                        ));
                    }
                }
                _ => return Err(DataError("Unknown Field default policy".into())),
            }
        }
        for t in &self.types {
            self.check_type(t, &mut vec![])?;
            for f in &t.fields {
                if self.field(&f.id).is_none() {
                    return Err(DataError("Unresolved type field".into()));
                }
            }
            for c in &t.conflicts {
                let other = self
                    .types
                    .iter()
                    .find(|x| &x.id == c)
                    .ok_or_else(|| DataError("Missing conflicting type".into()))?;
                if !other.conflicts.contains(&t.id) {
                    return Err(DataError("Asymmetric type conflict".into()));
                }
            }
            for c in &t.constraints {
                self.check_constraint(c)?;
                for selector in c.inputs.values() {
                    if let Selector::Field { id, .. } = selector {
                        let mut closure = BTreeSet::new();
                        self.add_type(&t.id, &mut closure);
                        if !self
                            .applicable_fields(&closure)
                            .iter()
                            .any(|(f, _)| f.id == *id)
                        {
                            return Err(DataError(
                                "Type validator reads a Field outside its requires closure".into(),
                            ));
                        }
                    }
                }
            }
        }
        for p in &self.providers {
            let Some((abi, _)) = builtin_operations(&p.implementation.id) else {
                return Err(DataError("Unknown builtin implementation".into()));
            };
            if p.implementation.kind != "builtin" || p.implementation.abi != abi {
                return Err(DataError("Builtin ABI mismatch".into()));
            }
            if p.limits.max_steps == 0
                || p.limits.max_steps > 10_000_000
                || p.limits.max_bytes == 0
                || p.limits.max_bytes > 64 * 1024 * 1024
                || p.limits.max_expansion > 100_000
            {
                return Err(DataError("Invalid provider budget".into()));
            }
            self.check_provider_data(p)?;
        }
        let mut temporal = false;
        let mut provider_bases = BTreeSet::new();
        for p in &self.providers {
            if p.implementation.id == "weftext.temporal.rules.v2" {
                if temporal {
                    return Err(DataError(
                        "Duplicate public temporal Query function registrations".into(),
                    ));
                }
                temporal = true;
            }
            let family = match p.implementation.id.as_str() {
                "weftext.calendar.gregorian.v2" | "weftext.calendar.table.v2" => "calendar",
                "weftext.zone.table.v1" => "zone",
                _ => continue,
            };
            if !provider_bases.insert((family, p.data["key"].as_str(), p.data["version"].as_str()))
            {
                return Err(DataError(
                    "Duplicate provider key/version within family".into(),
                ));
            }
        }
        for v in &self.validators {
            let p = self
                .providers
                .iter()
                .find(|p| p.id == v.provider)
                .ok_or_else(|| DataError("Missing validator provider".into()))?;
            if !builtin_operations(&p.implementation.id)
                .unwrap()
                .1
                .contains(&v.operation.as_str())
                || v.id
                    != format!(
                        "{}/{}",
                        v.provider.split('/').next().unwrap_or(""),
                        v.operation
                    )
            {
                return Err(DataError("Unregistered validator operation".into()));
            }
            let registered = match p.implementation.id.as_str() {
                "weftext.core.rules.v1" => [
                    "int-range",
                    "decimal-range",
                    "custom-code",
                    "owned-resources",
                ]
                .contains(&v.operation.as_str()),
                "weftext.temporal.rules.v2" => [
                    "month-day",
                    "calendar-date",
                    "during",
                    "range",
                    "period",
                    "recurrence",
                    "exception",
                    "event",
                ]
                .contains(&v.operation.as_str()),
                "weftext.units.rules.v1" => {
                    ["quantity", "measurement"].contains(&v.operation.as_str())
                }
                _ => false,
            };
            if !registered {
                return Err(DataError(
                    "Operation is not a registered validation signature".into(),
                ));
            }
        }
        for r in &self.relations {
            let t = self
                .types
                .iter()
                .find(|t| t.id == r.source_type)
                .ok_or_else(|| DataError("Missing relation source type".into()))?;
            if !t.fields.iter().any(|f| f.id == r.field) {
                return Err(DataError(
                    "Relation field is not owned by source type".into(),
                ));
            }
            let shape = &self
                .field(&r.field)
                .ok_or_else(|| DataError("Missing relation field".into()))?
                .shape;
            let (has_ref, maximum) = self.relation_path_info(shape, &r.target_path)?;
            if !has_ref || r.target_domain != "node_ref" || maximum != r.max_assertions {
                return Err(DataError(
                    "Relation path/domain/cardinality disagrees with its installed shape".into(),
                ));
            }
            if !matches!(r.direction.as_str(), "directed" | "symmetric")
                || !matches!(r.cycle_policy.as_str(), "allow" | "reject")
                || !matches!(r.graph_projection.as_str(), "general" | "ranked" | "family")
                || r.resolution != "live_on_typed_create_retain_dangling"
                || r.delete != "retain_explicit_cleanup"
                || (r.direction == "symmetric" && r.inverse.is_some())
            {
                return Err(DataError(
                    "Unregistered relation policy or directed symmetric inverse".into(),
                ));
            }
            match r.target_types["kind"].as_str() {
                Some("ordinary") => exact_members(&r.target_types, &["kind"])?,
                Some("any_of") => {
                    exact_members(&r.target_types, &["kind", "ids"])?;
                    let ids = r.target_types["ids"].as_array().ok_or_else(|| {
                        DataError("Relation target domain requires type IDs".into())
                    })?;
                    if ids.is_empty()
                        || ids.iter().any(|v| {
                            v.as_str()
                                .is_none_or(|id| !self.types.iter().any(|t| t.id == id))
                        })
                    {
                        return Err(DataError("Unresolved relation target type".into()));
                    }
                }
                _ => return Err(DataError("Unknown relation target policy".into())),
            }
        }
        unique(
            self.series_policies.iter().map(|p| p.key.as_str()),
            "series key",
        )?;
        for p in &self.series_policies {
            let calendar = self
                .providers
                .iter()
                .find(|v| v.id == p.calendar)
                .ok_or_else(|| DataError("Missing series calendar provider".into()))?;
            let zone = self
                .providers
                .iter()
                .find(|v| v.id == p.zone)
                .ok_or_else(|| DataError("Missing series zone provider".into()))?;
            if zone.implementation.id != "weftext.zone.table.v1"
                || !matches!(
                    (calendar.implementation.id.as_str(), p.rule.as_str()),
                    ("weftext.calendar.gregorian.v2", "iso8601")
                        | ("weftext.calendar.table.v2", "table-ordinal-v1")
                )
            {
                return Err(DataError("Unsupported series rule/provider family".into()));
            }
            unique(p.kinds.iter().map(String::as_str), "period kind")?;
            if p.kinds.is_empty()
                || p.kinds
                    .iter()
                    .any(|k| !matches!(k.as_str(), "day" | "week" | "month" | "quarter" | "year"))
                || !matches!(p.multiplicity.as_str(), "unique" | "many")
            {
                return Err(DataError("Unknown series kinds/multiplicity".into()));
            }
            match p.scope["kind"].as_str() {
                Some("library") => exact_members(&p.scope, &["kind"])?,
                Some("node_subtree") => {
                    exact_members(&p.scope, &["kind", "root"])?;
                    if !p.scope["root"]
                        .as_str()
                        .is_some_and(|s| scalar_valid("node_ref", &NativeValue::Text(s.into())))
                    {
                        return Err(DataError("Invalid series subtree root".into()));
                    }
                }
                _ => return Err(DataError("Unknown series scope".into())),
            }
        }
        Ok(())
    }
    fn check_type(&self, t: &Type, stack: &mut Vec<String>) -> Result<()> {
        if stack.len() > 64 {
            return Err(DataError("Type requires depth budget exceeded".into()));
        }
        if stack.contains(&t.id) {
            return Err(DataError("Type requires cycle".into()));
        }
        stack.push(t.id.clone());
        for id in &t.requires {
            let t = self
                .types
                .iter()
                .find(|t| &t.id == id)
                .ok_or_else(|| DataError("Missing required type".into()))?;
            self.check_type(t, stack)?;
        }
        stack.pop();
        Ok(())
    }
    fn check_shape<'a>(&'a self, s: &'a Shape, stack: &mut Vec<&'a str>) -> Result<()> {
        if stack.len() > 64 {
            return Err(DataError("Shape use depth budget exceeded".into()));
        }
        match s {
            Shape::Use { name } => {
                if stack.contains(&name.as_str()) {
                    return Err(DataError("Cyclic shape use".into()));
                }
                stack.push(name);
                self.check_shape(
                    self.definitions
                        .get(name)
                        .ok_or_else(|| DataError("Missing definition".into()))?,
                    stack,
                )?;
                stack.pop();
            }
            Shape::Scalar { semantic } => {
                if !SCALARS.contains(&semantic.as_str()) {
                    return Err(DataError("Unknown scalar codec".into()));
                }
            }
            Shape::Enum { values, .. } => {
                unique(values.iter().map(String::as_str), "enum")?;
                if values.is_empty() {
                    return Err(DataError("Empty enum".into()));
                }
            }
            Shape::List { items, min, max } => {
                if max.is_some_and(|v| v < *min) {
                    return Err(DataError("Invalid list cardinality".into()));
                }
                self.check_shape(items, stack)?;
            }
            Shape::Record {
                members,
                unknown,
                constraints,
            } => {
                if !matches!(unknown.as_str(), "preserve" | "reject") {
                    return Err(DataError("Unknown record policy".into()));
                }
                for m in members.values() {
                    self.check_shape(&m.shape, stack)?;
                }
                for c in constraints {
                    self.check_constraint(c)?;
                    for selector in c.inputs.values() {
                        match selector {
                            Selector::Local { path } => self.check_path(s, path)?,
                            _ => {
                                return Err(DataError(
                                    "Record validators can read only their local Shape".into(),
                                ));
                            }
                        }
                    }
                }
            }
            Shape::Union {
                variants,
                discriminator,
            } => {
                if variants.len() < 2 {
                    return Err(DataError("Union needs variants".into()));
                }
                for s in variants {
                    self.check_shape(s, stack)?;
                }
                if discriminator.is_none() {
                    unique(
                        variants.iter().map(|s| self.native_domain(s)),
                        "union domain",
                    )?;
                } else {
                    match discriminator.as_ref().unwrap() {
                        Discriminator::Member { keys } => {
                            if keys.len() != variants.len() {
                                return Err(DataError(
                                    "Member discriminator/variant count mismatch".into(),
                                ));
                            }
                            unique(
                                keys.iter().map(String::as_str),
                                "union member discriminator",
                            )?;
                            for (key, variant) in keys.iter().zip(variants) {
                                let Shape::Record { members, .. } = self.resolve(variant) else {
                                    return Err(DataError(
                                        "Member discriminator requires record variants".into(),
                                    ));
                                };
                                if !members.get(key).is_some_and(|m| m.required) {
                                    return Err(DataError(
                                        "Union branch discriminator must be required".into(),
                                    ));
                                }
                            }
                        }
                        Discriminator::Tag { key } => {
                            let mut tags = BTreeSet::new();
                            for variant in variants {
                                let Shape::Record { members, .. } = self.resolve(variant) else {
                                    return Err(DataError("Tagged union requires records".into()));
                                };
                                let member =
                                    members.get(key).filter(|m| m.required).ok_or_else(|| {
                                        DataError("Union tag must be a required member".into())
                                    })?;
                                let Shape::Enum {
                                    values,
                                    open: false,
                                } = self.resolve(&member.shape)
                                else {
                                    return Err(DataError("Union tags require fixed enums".into()));
                                };
                                if values.len() != 1 || !tags.insert(&values[0]) {
                                    return Err(DataError(
                                        "Union tags overlap or are not fixed".into(),
                                    ));
                                }
                            }
                        }
                        Discriminator::RecordPresence {
                            key,
                            present,
                            absent,
                        } => {
                            if present == absent
                                || *present >= variants.len()
                                || *absent >= variants.len()
                            {
                                return Err(DataError("Malformed record-presence variants".into()));
                            }
                            let Shape::Record { members, .. } = self.resolve(&variants[*present])
                            else {
                                return Err(DataError("Presence branch must be record".into()));
                            };
                            if !members.get(key).is_some_and(|m| m.required)
                                || !matches!(self.resolve(&variants[*absent]),Shape::Record{members,..}if !members.contains_key(key))
                            {
                                return Err(DataError(
                                    "Record-presence discriminator overlaps".into(),
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn check_constraint(&self, c: &Constraint) -> Result<()> {
        let v = self
            .validators
            .iter()
            .find(|v| v.id == c.validator)
            .ok_or_else(|| DataError("Unknown constraint validator".into()))?;
        let slots: &[&str] = match v.operation.as_str() {
            "custom-code" => &["code", "custom"],
            "owned-resources" => &["owner", "values", "metadata"],
            "event" => &["range", "recurrence"],
            _ => &["value"],
        };
        if c.inputs.keys().map(String::as_str).collect::<BTreeSet<_>>()
            != slots.iter().copied().collect()
        {
            return Err(DataError("Validator input slots mismatch".into()));
        }
        let params: &[&str] = match v.operation.as_str() {
            "int-range" => &["min", "max", "exclude"],
            "decimal-range" => &["min", "max"],
            "custom-code" => &["code"],
            "owned-resources" => &["owner_required"],
            _ => &[],
        };
        if c.parameters.as_object().is_none_or(|p| {
            p.keys().map(String::as_str).collect::<BTreeSet<_>>()
                != params.iter().copied().collect()
        }) {
            return Err(DataError("Validator parameters mismatch".into()));
        }
        let valid_parameters = match v.operation.as_str() {
            "int-range" => {
                c.parameters["min"]
                    .as_i64()
                    .zip(c.parameters["max"].as_i64())
                    .is_some_and(|(a, b)| a <= b)
                    && c.parameters["exclude"]
                        .as_array()
                        .is_some_and(|a| a.iter().all(|v| v.as_i64().is_some()))
            }
            "decimal-range" => c.parameters["min"]
                .as_str()
                .zip(c.parameters["max"].as_str())
                .is_some_and(|(a, b)| {
                    ExactDecimal::parse(a)
                        .ok()
                        .zip(ExactDecimal::parse(b).ok())
                        .is_some_and(|(a, b)| !a.compare(&b).is_gt())
                }),
            "custom-code" => c.parameters["code"].as_str().is_some_and(|s| !s.is_empty()),
            "owned-resources" => c.parameters["owner_required"].is_boolean(),
            _ => true,
        };
        if !valid_parameters {
            return Err(DataError(
                "Validator parameter types/ranges mismatch".into(),
            ));
        }
        for s in c.inputs.values() {
            match s {
                Selector::Context { name }
                    if !matches!(name.as_str(), "owner" | "resource_metadata") =>
                {
                    return Err(DataError("Unknown validator context".into()));
                }
                Selector::Field { id, path } => self.check_path(
                    &self
                        .field(id)
                        .ok_or_else(|| DataError("Unresolved constraint field".into()))?
                        .shape,
                    path,
                )?,
                _ => {}
            }
        }
        Ok(())
    }
    fn check_path(&self, s: &Shape, path: &[SchemaPath]) -> Result<()> {
        if path.is_empty() {
            return Ok(());
        }
        match (self.resolve(s), &path[0]) {
            (Shape::List { items, .. }, SchemaPath::Each) => self.check_path(items, &path[1..]),
            (Shape::Record { members, .. }, SchemaPath::Member { key }) => self.check_path(
                &members
                    .get(key)
                    .ok_or_else(|| DataError("Unresolved schema member path".into()))?
                    .shape,
                &path[1..],
            ),
            (Shape::Union { variants, .. }, _) => {
                let supported = variants.iter().any(|v| self.check_path(v, path).is_ok());
                if supported {
                    Ok(())
                } else {
                    Err(DataError("Unresolved union path".into()))
                }
            }
            _ => Err(DataError("Schema path type mismatch".into())),
        }
    }
    fn relation_path_info(
        &self,
        shape: &Shape,
        path: &[SchemaPath],
    ) -> Result<(bool, Option<usize>)> {
        let shape = self.resolve(shape);
        if let Shape::Union { variants, .. } = shape {
            let mut has_ref = false;
            let mut maximum = Some(0usize);
            for variant in variants {
                let (ref_branch, bound) = self.relation_path_info(variant, path)?;
                has_ref |= ref_branch;
                maximum = maximum.zip(bound).map(|(a, b)| a.max(b));
            }
            return Ok((has_ref, maximum));
        }
        match (shape, path.first()) {
            (Shape::Scalar { semantic }, None) if semantic == "node_ref" => Ok((true, Some(1))),
            (Shape::List { items, max, .. }, Some(SchemaPath::Each)) => {
                let (has_ref, nested) = self.relation_path_info(items, &path[1..])?;
                let maximum = max.zip(nested).and_then(|(a, b)| a.checked_mul(b));
                Ok((has_ref, maximum))
            }
            (Shape::Record { members, .. }, Some(SchemaPath::Member { key })) => {
                if let Some(member) = members.get(key) {
                    self.relation_path_info(&member.shape, &path[1..])
                } else if key == "ref" && path.len() == 1 && members.get("text").is_some_and(|m| m.required && !m.nullable && matches!(self.resolve(&m.shape), Shape::Scalar { semantic } if semantic == "nonempty_text")) {
                    // The schema's selected literal Target branch counts one
                    // assertion without inventing an endpoint for its text.
                    Ok((false, Some(1)))
                } else {
                    Err(DataError("Unresolved relation assertion path".into()))
                }
            }
            _ => Err(DataError(
                "Relation path must reach an exact node_ref slot".into(),
            )),
        }
    }
    fn check_provider_data(&self, p: &Provider) -> Result<()> {
        let kind = p
            .data
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| DataError("Provider data missing kind".into()))?;
        let want = match p.implementation.id.as_str() {
            "weftext.core.rules.v1" | "weftext.temporal.rules.v2" => "none",
            "weftext.units.rules.v1" => "unit-table",
            "weftext.calendar.gregorian.v2" => "gregorian",
            "weftext.calendar.table.v2" => "calendar-table",
            "weftext.zone.table.v1" => "zone-table",
            _ => "",
        };
        if kind != want {
            return Err(DataError("Provider data kind mismatch".into()));
        }
        let keys: &[&str] = match kind {
            "none" => &["kind"],
            "unit-table" => &["kind", "version", "units"],
            "gregorian" => &["kind", "key", "version", "min_year", "max_year"],
            "calendar-table" => &["kind", "key", "version", "epoch_date", "years"],
            "zone-table" => &[
                "kind",
                "key",
                "version",
                "from",
                "until",
                "initial_offset_seconds",
                "transitions",
            ],
            _ => return Err(DataError("Unknown provider data".into())),
        };
        exact_members(&p.data, keys)?;
        if kind == "gregorian"
            && p.data["min_year"]
                .as_i64()
                .zip(p.data["max_year"].as_i64())
                .is_none_or(|(a, b)| a < 1 || b > 9999 || a > b)
        {
            return Err(DataError("Invalid Gregorian coverage".into()));
        }
        if kind == "unit-table" {
            let units = p.data["units"]
                .as_array()
                .ok_or_else(|| DataError("Unit table requires units".into()))?;
            let mut seen = BTreeSet::new();
            for unit in units {
                exact_members(unit, &["key", "dimension"])?;
                if !unit["key"]
                    .as_str()
                    .is_some_and(|key| !key.is_empty() && seen.insert(key))
                    || !unit["dimension"].as_str().is_some_and(|d| !d.is_empty())
                {
                    return Err(DataError("Invalid/duplicate unit definition".into()));
                }
            }
        }
        if matches!(kind, "gregorian" | "calendar-table" | "zone-table")
            && ["key", "version"]
                .iter()
                .any(|k| p.data[*k].as_str().is_none_or(str::is_empty))
        {
            return Err(DataError(
                "Provider key/version must be nonempty exact strings".into(),
            ));
        }
        if kind == "calendar-table" {
            let epoch = p.data["epoch_date"]
                .as_str()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .ok_or_else(|| DataError("Calendar table needs real epoch date".into()))?;
            let years = p.data["years"]
                .as_array()
                .filter(|a| !a.is_empty())
                .ok_or_else(|| DataError("Calendar table needs finite years".into()))?;
            let mut previous = None;
            let mut days = 0i64;
            for year in years {
                exact_members(year, &["year", "months"])?;
                let y = year["year"]
                    .as_i64()
                    .ok_or_else(|| DataError("Calendar table year must be int".into()))?;
                if previous.is_some_and(|v: i64| v.checked_add(1) != Some(y)) {
                    return Err(DataError("Calendar table years must be consecutive".into()));
                }
                previous = Some(y);
                let months = year["months"]
                    .as_array()
                    .filter(|a| !a.is_empty())
                    .ok_or_else(|| DataError("Calendar year needs ordered months".into()))?;
                let mut seen = BTreeSet::new();
                for month in months {
                    exact_members(month, &["number", "leap", "days"])?;
                    let number = month["number"]
                        .as_i64()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| DataError("Month number must be positive".into()))?;
                    let leap = month["leap"]
                        .as_bool()
                        .ok_or_else(|| DataError("Month leap must be bool".into()))?;
                    let length = month["days"]
                        .as_i64()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| DataError("Month days must be positive".into()))?;
                    if !seen.insert((number, leap)) {
                        return Err(DataError("Duplicate table month number/leap".into()));
                    }
                    days = days
                        .checked_add(length)
                        .ok_or_else(|| DataError("Calendar table cumulative overflow".into()))?;
                }
            }
            if epoch
                .checked_add_days(chrono::Days::new(days as u64))
                .is_none()
            {
                return Err(DataError("Calendar table epoch/coverage overflow".into()));
            }
        }
        if kind == "zone-table" {
            let from = p.data["from"]
                .as_str()
                .map(crate::ExactInstant::parse)
                .transpose()?
                .ok_or_else(|| DataError("Zone coverage from required".into()))?;
            let until = p.data["until"]
                .as_str()
                .map(crate::ExactInstant::parse)
                .transpose()?
                .ok_or_else(|| DataError("Zone coverage until required".into()))?;
            if !from.compare(&until).is_lt() {
                return Err(DataError("Zone coverage must be ordered".into()));
            }
            let valid_offset =
                |v: &Value| v.as_i64().is_some_and(|n| (-86400..=86400).contains(&n));
            if !valid_offset(&p.data["initial_offset_seconds"]) {
                return Err(DataError("Invalid initial zone offset".into()));
            }
            let mut previous = from;
            for t in p.data["transitions"]
                .as_array()
                .ok_or_else(|| DataError("Zone transitions required".into()))?
            {
                exact_members(t, &["at", "offset_seconds"])?;
                let at = t["at"]
                    .as_str()
                    .map(crate::ExactInstant::parse)
                    .transpose()?
                    .ok_or_else(|| DataError("Zone transition instant required".into()))?;
                if !previous.compare(&at).is_lt()
                    || !at.compare(&until).is_lt()
                    || !valid_offset(&t["offset_seconds"])
                {
                    return Err(DataError(
                        "Zone transitions must be ordered, covered and bounded".into(),
                    ));
                }
                previous = at;
            }
        }
        Ok(())
    }
    fn validate_constraint(&self, c: &Constraint, local: &NativeValue) -> Validation {
        let mut out = Validation::valid();
        let Some(v) = self.validators.iter().find(|v| v.id == c.validator) else {
            out.unavailable.push("missing-validator".into());
            return out;
        };
        let get = |slot: &str| -> Option<&NativeValue> {
            match c.inputs.get(slot)? {
                Selector::Local { path } => select_one(local, path),
                _ => None,
            }
        };
        let bad = |out: &mut Validation, msg: &str| {
            out.diagnostics.push(format!("{}: {msg}", c.validator))
        };
        match v.operation.as_str() {
            "int-range" => {
                if let Some(value) = get("value") {
                    let n = value.integer();
                    let min = c.parameters["min"].as_i64();
                    let max = c.parameters["max"].as_i64();
                    if n.is_none()
                        || min.is_none()
                        || max.is_none()
                        || n < min
                        || n > max
                        || c.parameters["exclude"]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|x| x.as_i64() == n))
                    {
                        bad(&mut out, "integer out of range");
                    }
                }
            }
            "decimal-range" => {
                if let Some(value) = get("value") {
                    let s = match value {
                        NativeValue::Decimal(s) | NativeValue::Text(s) => s.as_str(),
                        _ => "",
                    };
                    let values = [
                        s,
                        c.parameters["min"].as_str().unwrap_or(""),
                        c.parameters["max"].as_str().unwrap_or(""),
                    ]
                    .map(ExactDecimal::parse);
                    match values {
                        [Ok(n), Ok(min), Ok(max)] => {
                            if n.compare(&min).is_lt() || n.compare(&max).is_gt() {
                                bad(&mut out, "decimal out of range");
                            }
                        }
                        _ => bad(&mut out, "invalid exact decimal"),
                    }
                }
            }
            "custom-code" => {
                if get("code").and_then(NativeValue::text) == c.parameters["code"].as_str()
                    && get("custom")
                        .and_then(NativeValue::text)
                        .is_none_or(str::is_empty)
                {
                    bad(&mut out, "custom code requires custom text");
                }
            }
            "month-day" => {
                if let Some(value) = get("value") {
                    let m = value
                        .member("month")
                        .and_then(NativeValue::integer)
                        .unwrap_or(0);
                    let d = value
                        .member("day")
                        .and_then(NativeValue::integer)
                        .unwrap_or(0);
                    if !(1..=12).contains(&m)
                        || !(1..=31).contains(&d)
                        || NaiveDate::from_ymd_opt(2000, m as u32, d as u32).is_none()
                    {
                        bad(&mut out, "invalid month-day");
                    }
                }
            }
            "calendar-date" => {
                if let Some(value) = get("value") {
                    validation_cell(crate::validate_calendar_endpoint(value, self), &mut out);
                }
            }
            "range" | "during" => {
                if let Some(value) = get("value") {
                    validation_cell(
                        crate::validate_interval(value, self, v.operation == "during"),
                        &mut out,
                    );
                }
            }
            "quantity" | "measurement" => {
                if let Some(value) = get("value") {
                    let quantity = if v.operation == "measurement" {
                        value.member("quantity").unwrap_or(value)
                    } else {
                        value
                    };
                    let unit = quantity.member("unit").and_then(NativeValue::text);
                    let provider = self.providers.iter().find(|p| p.id == v.provider);
                    let found = provider
                        .and_then(|p| p.data["units"].as_array())
                        .and_then(|a| a.iter().find(|u| u["key"].as_str() == unit));
                    if found.is_none() {
                        out.unavailable.push("unknown-unit".into());
                    }
                    if v.operation == "measurement" {
                        let expected = match value.member("kind").and_then(NativeValue::text) {
                            Some("height") => "length",
                            Some("weight") => "mass",
                            _ => "",
                        };
                        if found.is_some_and(|u| u["dimension"].as_str() != Some(expected)) {
                            bad(&mut out, "measurement unit dimension mismatch");
                        }
                    }
                    if let Some(p) = quantity.member("precision")
                        && let NativeValue::Decimal(s) | NativeValue::Text(s) = p
                        && ExactDecimal::parse(s)
                            .is_ok_and(|n| n.compare(&ExactDecimal::parse("0").unwrap()).is_lt())
                    {
                        bad(&mut out, "negative precision");
                    }
                }
            }
            "recurrence" => {
                if let Some(value) = get("value") {
                    validation_cell(crate::validate_recurrence(value, self), &mut out);
                }
            }
            "period" => {
                if let Some(value) = get("value") {
                    validation_cell(crate::validate_period(value, self), &mut out);
                }
            }
            "exception" => {
                if let Some(value) = get("value") {
                    validation_cell(crate::validate_exception(value, self), &mut out);
                }
            }
            "owned-resources" | "event" => out
                .unavailable
                .push("required Core field/context snapshot is not supplied".into()),
            _ => out.unavailable.push("operation unavailable".into()),
        }
        out
    }
}
fn validation_cell(cell: crate::Cell, out: &mut Validation) {
    match cell {
        crate::Cell::Invalid { diagnostics, .. } => out.diagnostics.extend(diagnostics),
        crate::Cell::Unavailable { reason } => out.unavailable.push(reason),
        _ => {}
    }
}
fn exact_members(value: &Value, keys: &[&str]) -> Result<()> {
    if value.as_object().is_none_or(|o| {
        o.keys().map(String::as_str).collect::<BTreeSet<_>>() != keys.iter().copied().collect()
    }) {
        return Err(DataError(
            "Unknown/missing configuration data members".into(),
        ));
    }
    Ok(())
}
fn unique<'a>(values: impl Iterator<Item = &'a str>, kind: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for v in values {
        if v.is_empty() || !seen.insert(v) {
            return Err(DataError(format!("Duplicate/empty {kind}")));
        }
    }
    Ok(())
}
pub(crate) fn select_one<'a>(v: &'a NativeValue, path: &[SchemaPath]) -> Option<&'a NativeValue> {
    let mut current = v;
    for p in path {
        match p {
            SchemaPath::Member { key } => current = current.member(key)?,
            SchemaPath::Each => return None,
        }
    }
    Some(current)
}
fn value_domain(v: &NativeValue) -> &str {
    match v {
        NativeValue::None => "none",
        NativeValue::Text(_) => "text",
        NativeValue::Bool(_) => "bool",
        NativeValue::Int(_) => "int",
        NativeValue::Float(_) => "float",
        NativeValue::Decimal(_) => "decimal",
        NativeValue::Date(_) => "date",
        NativeValue::Time(_) => "time",
        NativeValue::LocalDatetime(_) => "local_datetime",
        NativeValue::Duration(_) => "duration",
        NativeValue::Array(_) => "array",
        NativeValue::Record(_) => "record",
    }
}
pub fn uuid_v4(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b[14] == b'4'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b')
        && b.iter().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *c == b'-'
            } else {
                c.is_ascii_digit() || matches!(c, b'a'..=b'f')
            }
        })
}
fn uuid(s: &str) -> bool {
    uuid_v4(s)
}
pub fn scalar_valid(semantic: &str, v: &NativeValue) -> bool {
    match semantic {
        "bool" => matches!(v, NativeValue::Bool(_)),
        "int" => v.integer().is_some(),
        "float" => matches!(v,NativeValue::Float(s)if s.parse::<f64>().is_ok_and(f64::is_finite)),
        "decimal" => {
            matches!(v,NativeValue::Decimal(s)if s.parse::<rust_decimal::Decimal>().is_ok())
        }
        "exact_decimal" | "exact_seconds" => {
            matches!(v,NativeValue::Text(s)if ExactDecimal::parse(s).is_ok())
        }
        "duration" => matches!(v,NativeValue::Duration(s)if s.parse::<i64>().is_ok()),
        "date" => {
            matches!(v,NativeValue::Date(s)if NaiveDate::parse_from_str(s,"%Y-%m-%d").is_ok_and(|d|d.format("%Y-%m-%d").to_string()==*s))
        }
        "time" => {
            matches!(v,NativeValue::Time(s)if NaiveTime::parse_from_str(s,"%H:%M:%S").is_ok_and(|t|t.format("%H:%M:%S").to_string()==*s))
        }
        "local_datetime" => {
            matches!(v,NativeValue::LocalDatetime(s)if NaiveDateTime::parse_from_str(s,"%Y-%m-%dT%H:%M:%S").is_ok_and(|d|d.format("%Y-%m-%dT%H:%M:%S").to_string()==*s))
        }
        _ => v.text().is_some_and(|s| match semantic {
            "text" => true,
            "nonempty_text" | "phone" | "email" | "identifier" => !s.is_empty(),
            "config_revision" => uuid(s),
            "citation_key" => {
                s.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            }
            "doi" => {
                s.starts_with("10.")
                    && s.split_once('/').is_some_and(|(p, n)| {
                        p[3..].bytes().all(|b| b.is_ascii_digit()) && !n.is_empty()
                    })
                    && !s.chars().any(char::is_whitespace)
            }
            "url" => s.split_once("://").is_some_and(|(scheme, rest)| {
                matches!(scheme, "https" | "http" | "ftp")
                    && !rest.is_empty()
                    && !rest.chars().any(char::is_whitespace)
            }),
            "language_tag" => {
                !s.is_empty()
                    && s.split('-')
                        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric()))
            }
            "node_ref" | "resource_ref" | "tag_ref" => {
                let scheme = match semantic {
                    "node_ref" => "weftext-node://",
                    "resource_ref" => "weftext-resource://",
                    _ => "weftext-tag://",
                };
                s.strip_prefix(scheme)
                    .is_some_and(|r| r.split_once('/').is_some_and(|(w, n)| uuid(w) && uuid(n)))
            }
            "schema_ref" => s
                .strip_prefix("weftext-config://")
                .and_then(|r| r.strip_suffix("/schema"))
                .is_some_and(uuid),
            "instant" => crate::ExactInstant::parse(s).is_ok(),
            _ => false,
        }),
    }
}
pub fn compare_native(a: &NativeValue, b: &NativeValue) -> Result<std::cmp::Ordering> {
    use NativeValue::*;
    let order = match (a, b) {
        (Text(a), Text(b)) => a.cmp(b),
        (Bool(a), Bool(b)) => a.cmp(b),
        (Int(a), Int(b)) => a
            .parse::<i64>()
            .ok()
            .zip(b.parse::<i64>().ok())
            .map(|(a, b)| a.cmp(&b))
            .ok_or_else(|| DataError("invalid int".into()))?,
        (Float(a), Float(b)) => a
            .parse::<f64>()
            .ok()
            .zip(b.parse::<f64>().ok())
            .and_then(|(a, b)| a.partial_cmp(&b))
            .ok_or_else(|| DataError("invalid float".into()))?,
        (Decimal(a), Decimal(b)) => ExactDecimal::parse(a)?.compare(&ExactDecimal::parse(b)?),
        (Date(a), Date(b)) | (Time(a), Time(b)) | (LocalDatetime(a), LocalDatetime(b)) => a.cmp(b),
        (Duration(a), Duration(b)) => a
            .parse::<i64>()
            .ok()
            .zip(b.parse::<i64>().ok())
            .map(|(a, b)| a.cmp(&b))
            .ok_or_else(|| DataError("invalid duration".into()))?,
        _ => {
            return Err(DataError(
                "Values require explicit same-type comparator".into(),
            ));
        }
    };
    Ok(order)
}
struct UniqueJson(Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut m: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut out = serde_json::Map::new();
                while let Some((k, v)) = m.next_entry::<String, UniqueJson>()? {
                    if out.insert(k, v.0).is_some() {
                        return Err(serde::de::Error::custom("Duplicate JSON key"));
                    }
                }
                Ok(UniqueJson(Value::Object(out)))
            }
            fn visit_seq<S: SeqAccess<'de>>(
                self,
                mut s: S,
            ) -> std::result::Result<Self::Value, S::Error> {
                let mut a = vec![];
                while let Some(v) = s.next_element::<UniqueJson>()? {
                    a.push(v.0);
                }
                Ok(UniqueJson(Value::Array(a)))
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(
                    serde_json::Number::from_f64(v)
                        .ok_or_else(|| E::custom("nonfinite JSON"))?
                        .into(),
                ))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
        }
        d.deserialize_any(V)
    }
}
