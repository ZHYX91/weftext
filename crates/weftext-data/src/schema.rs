use crate::{DataError, ErrorDescription, ExactDecimal, NativeValue, Result, UiMessage};
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
    pub diagnostics: Vec<ErrorDescription>,
    pub unavailable: Vec<ErrorDescription>,
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
                "task-reminder",
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
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_schema_schema_installation_byte_budget_exceeded(),
            ));
        }
        let schema: Self = serde_json::from_value(parse_unique_json(bytes.as_bytes())?)?;
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
                    check.diagnostics.push(
                        weftext_messages::UiMessage::error_data_schema_duplicate_type(
                            weftext_messages::params::ErrorDataSchemaDuplicateType {
                                type_token: (token).to_string(),
                            },
                        )
                        .into(),
                    );
                }
                self.add_type(id, &mut result);
            } else {
                check.unavailable.push(
                    weftext_messages::UiMessage::error_data_schema_unknown_type(
                        weftext_messages::params::ErrorDataSchemaUnknownType {
                            type_token: (token).to_string(),
                        },
                    )
                    .into(),
                );
            }
        }
        for id in &result {
            let t = self.types.iter().find(|t| &t.id == id).unwrap();
            for c in &t.conflicts {
                if result.contains(c) {
                    check.diagnostics.push(
                        weftext_messages::UiMessage::error_data_schema_types_conflict(
                            weftext_messages::params::ErrorDataSchemaTypesConflict {
                                type_id: (id).to_string(),
                                conflicting_type_id: (c).to_string(),
                            },
                        )
                        .into(),
                    );
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
            unavailable.unavailable.extend(error.into_diagnostics());
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
                    out.diagnostics.push(
                        weftext_messages::UiMessage::error_data_schema_value_semantic_mismatch(
                            weftext_messages::params::ErrorDataSchemaValueSemanticMismatch {
                                value_path: (path).to_string(),
                                semantic: (semantic).to_string(),
                            },
                        )
                        .into(),
                    );
                }
            }
            Shape::Enum { values, open } => {
                if value
                    .text()
                    .is_none_or(|v| !*open && !values.iter().any(|s| s == v))
                {
                    out.diagnostics.push(
                        weftext_messages::UiMessage::error_data_schema_unknown_enum_value(
                            weftext_messages::params::ErrorDataSchemaUnknownEnumValue {
                                value_path: (path).to_string(),
                            },
                        )
                        .into(),
                    );
                }
            }
            Shape::List { items, min, max } => match value.array() {
                Some(a) => {
                    if a.len() < *min || max.is_some_and(|m| a.len() > m) {
                        out.diagnostics.push(weftext_messages::UiMessage::error_data_schema_list_cardinality_mismatch(weftext_messages::params::ErrorDataSchemaListCardinalityMismatch {value_path: (path).to_string()}).into());
                    }
                    for (i, v) in a.iter().enumerate() {
                        out.merge(self.validate_at(items, v, &format!("{path}[{i}]")));
                    }
                }
                None => out.diagnostics.push(
                    weftext_messages::UiMessage::error_data_schema_array_expected(
                        weftext_messages::params::ErrorDataSchemaArrayExpected {
                            value_path: (path).to_string(),
                        },
                    )
                    .into(),
                ),
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
                                .push(weftext_messages::UiMessage::error_data_schema_required_record_member_absent(weftext_messages::params::ErrorDataSchemaRequiredRecordMemberAbsent {value_path: (path).to_string(), member_key: (k).to_string()}).into()),
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
                                    .push(weftext_messages::UiMessage::error_data_schema_unknown_record_member(weftext_messages::params::ErrorDataSchemaUnknownRecordMember {value_path: (path).to_string(), member_key: (m.key).to_string()}).into());
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
                None => out.diagnostics.push(
                    weftext_messages::UiMessage::error_data_schema_record_expected(
                        weftext_messages::params::ErrorDataSchemaRecordExpected {
                            value_path: (path).to_string(),
                        },
                    )
                    .into(),
                ),
            },
            Shape::Union {
                variants,
                discriminator,
            } => match self.union_variant(variants, discriminator.as_ref(), value) {
                Ok(v) => out.merge(self.validate_at(v, value, path)),
                Err(error) => {
                    out.diagnostics.push(
                        UiMessage::error_data_schema_union_value_invalid_at_path(
                            weftext_messages::params::ErrorDataSchemaUnionValueInvalidAtPath {
                                path: path.into(),
                            },
                        )
                        .into(),
                    );
                    out.diagnostics.extend(error.into_diagnostics());
                }
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
                    return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_union_requires_exactly_one_branch_member()));
                }
                matches[0]
            }
            Some(Discriminator::Tag { key }) => {
                let tag = v.member(key).and_then(NativeValue::text).ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_schema_union_tag_missing(),
                    )
                })?;
                let found:Vec<_>=variants.iter().enumerate().filter(|(_,s)|match self.resolve(s){Shape::Record{members,..}=>matches!(self.resolve(&members[key].shape),Shape::Enum{values,..}if values.iter().any(|x|x==tag)),_=>false}).map(|(i,_)|i).collect();
                if found.len() != 1 {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_union_tag_ambiguous(),
                    ));
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
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_union_native_domain_ambiguous(),
                    ));
                }
                found[0]
            }
        };
        variants.get(index).ok_or_else(|| {
            DataError::new(
                weftext_messages::UiMessage::error_data_schema_invalid_union_discriminator(),
            )
        })
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
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_unsupported_schema_version(),
            ));
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
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_type_alias_unresolved(),
                ));
            }
        }
        for (name, s) in &self.definitions {
            self.check_shape(s, &mut vec![name.as_str()])?;
        }
        for f in &self.fields {
            if matches!(f.key.as_str(), "id" | "types") || f.key.starts_with("weftext-") {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_field_key_reserved(),
                ));
            }
            self.check_shape(&f.shape, &mut vec![])?;
            let default = f.default.as_ref().ok_or_else(|| {
                DataError::new(
                    weftext_messages::UiMessage::error_data_schema_field_default_policy_required(),
                )
            })?;
            match default.get("kind").and_then(Value::as_str) {
                Some("absent") => exact_members(default, &["kind"])?,
                Some("native") => {
                    exact_members(default, &["kind", "source"])?;
                    let native = crate::decode_native_expression(
                        default["source"]
                            .as_str()
                            .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_native_default_source_required()))?,
                    )?;
                    let check = self.validate(&f.shape, &native);
                    if !check.is_valid() && !(f.nullable && matches!(native, NativeValue::None)) {
                        return Err(DataError::new(
                            weftext_messages::UiMessage::error_data_schema_native_creation_default_invalid_or_unavailable(),
                        ));
                    }
                }
                _ => {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_unknown_field_default_policy(
                        ),
                    ));
                }
            }
        }
        for t in &self.types {
            self.check_type(t, &mut vec![])?;
            for f in &t.fields {
                if self.field(&f.id).is_none() {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_type_field_unresolved(),
                    ));
                }
            }
            for c in &t.conflicts {
                let other = self.types.iter().find(|x| &x.id == c).ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_schema_conflicting_type_missing(),
                    )
                })?;
                if !other.conflicts.contains(&t.id) {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_type_conflict_asymmetric(),
                    ));
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
                            return Err(DataError::new(
                                weftext_messages::UiMessage::error_data_schema_type_validator_reads_outside_requires_closure(),
                            ));
                        }
                    }
                }
            }
        }
        for p in &self.providers {
            let Some((abi, _)) = builtin_operations(&p.implementation.id) else {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_unknown_builtin_implementation(),
                ));
            };
            if p.implementation.kind != "builtin" || p.implementation.abi != abi {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_builtin_abi_mismatch(),
                ));
            }
            if p.limits.max_steps == 0
                || p.limits.max_steps > 10_000_000
                || p.limits.max_bytes == 0
                || p.limits.max_bytes > 64 * 1024 * 1024
                || p.limits.max_expansion > 100_000
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_invalid_provider_budget(),
                ));
            }
            self.check_provider_data(p)?;
        }
        let mut temporal = false;
        let mut provider_bases = BTreeSet::new();
        for p in &self.providers {
            if p.implementation.id == "weftext.temporal.rules.v2" {
                if temporal {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_duplicate_public_temporal_functions(),
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
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_duplicate_provider_key_version_in_family(),
                ));
            }
        }
        for v in &self.validators {
            let p = self
                .providers
                .iter()
                .find(|p| p.id == v.provider)
                .ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_schema_validator_provider_missing(),
                    )
                })?;
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
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_validator_operation_unregistered(
                    ),
                ));
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
                    "task-reminder",
                ]
                .contains(&v.operation.as_str()),
                "weftext.units.rules.v1" => {
                    ["quantity", "measurement"].contains(&v.operation.as_str())
                }
                _ => false,
            };
            if !registered {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_validation_signature_unregistered(),
                ));
            }
        }
        for r in &self.relations {
            let t = self
                .types
                .iter()
                .find(|t| t.id == r.source_type)
                .ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_schema_relation_source_type_missing(
                        ),
                    )
                })?;
            if !t.fields.iter().any(|f| f.id == r.field) {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_relation_field_not_owned_by_source_type(),
                ));
            }
            let shape = &self
                .field(&r.field)
                .ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_schema_relation_field_missing(),
                    )
                })?
                .shape;
            let (has_ref, maximum) = self.relation_path_info(shape, &r.target_path)?;
            if !has_ref || r.target_domain != "node_ref" || maximum != r.max_assertions {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_relation_shape_contract_mismatch(
                    ),
                ));
            }
            if !matches!(r.direction.as_str(), "directed" | "symmetric")
                || !matches!(r.cycle_policy.as_str(), "allow" | "reject")
                || !matches!(r.graph_projection.as_str(), "general" | "ranked" | "family")
                || r.resolution != "live_on_typed_create_retain_dangling"
                || r.delete != "retain_explicit_cleanup"
                || (r.direction == "symmetric" && r.inverse.is_some())
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_relation_policy_or_symmetric_inverse_invalid(),
                ));
            }
            match r.target_types["kind"].as_str() {
                Some("ordinary") => exact_members(&r.target_types, &["kind"])?,
                Some("any_of") => {
                    exact_members(&r.target_types, &["kind", "ids"])?;
                    let ids = r.target_types["ids"].as_array().ok_or_else(|| {
                        DataError::new(weftext_messages::UiMessage::error_data_schema_relation_target_type_ids_required())
                    })?;
                    if ids.is_empty()
                        || ids.iter().any(|v| {
                            v.as_str()
                                .is_none_or(|id| !self.types.iter().any(|t| t.id == id))
                        })
                    {
                        return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_relation_target_type_unresolved()));
                    }
                }
                _ => {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_unknown_relation_target_policy(),
                    ));
                }
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
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_series_calendar_provider_missing()))?;
            let zone = self
                .providers
                .iter()
                .find(|v| v.id == p.zone)
                .ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_schema_series_zone_provider_missing(
                        ),
                    )
                })?;
            if zone.implementation.id != "weftext.zone.table.v1"
                || !matches!(
                    (calendar.implementation.id.as_str(), p.rule.as_str()),
                    ("weftext.calendar.gregorian.v2", "iso8601")
                        | ("weftext.calendar.table.v2", "table-ordinal-v1")
                )
            {
                return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_unsupported_series_rule_or_provider_family()));
            }
            unique(p.kinds.iter().map(String::as_str), "period kind")?;
            if p.kinds.is_empty()
                || p.kinds
                    .iter()
                    .any(|k| !matches!(k.as_str(), "day" | "week" | "month" | "quarter" | "year"))
                || !matches!(p.multiplicity.as_str(), "unique" | "many")
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_unknown_series_kinds_or_multiplicity(),
                ));
            }
            match p.scope["kind"].as_str() {
                Some("library") => exact_members(&p.scope, &["kind"])?,
                Some("node_subtree") => {
                    exact_members(&p.scope, &["kind", "root"])?;
                    if !p.scope["root"]
                        .as_str()
                        .is_some_and(|s| scalar_valid("node_ref", &NativeValue::Text(s.into())))
                    {
                        return Err(DataError::new(
                            weftext_messages::UiMessage::error_data_schema_invalid_series_subtree_root(),
                        ));
                    }
                }
                _ => {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_unknown_series_scope(),
                    ));
                }
            }
        }
        Ok(())
    }
    fn check_type(&self, t: &Type, stack: &mut Vec<String>) -> Result<()> {
        if stack.len() > 64 {
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_schema_type_requires_depth_budget_exceeded(
                ),
            ));
        }
        if stack.contains(&t.id) {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_type_requires_cycle(),
            ));
        }
        stack.push(t.id.clone());
        for id in &t.requires {
            let t = self.types.iter().find(|t| &t.id == id).ok_or_else(|| {
                DataError::new(
                    weftext_messages::UiMessage::error_data_schema_required_type_missing(),
                )
            })?;
            self.check_type(t, stack)?;
        }
        stack.pop();
        Ok(())
    }
    fn check_shape<'a>(&'a self, s: &'a Shape, stack: &mut Vec<&'a str>) -> Result<()> {
        if stack.len() > 64 {
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_schema_shape_use_depth_budget_exceeded(),
            ));
        }
        match s {
            Shape::Use { name } => {
                if stack.contains(&name.as_str()) {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_shape_use_cycle(),
                    ));
                }
                stack.push(name);
                self.check_shape(
                    self.definitions.get(name).ok_or_else(|| {
                        DataError::new(
                            weftext_messages::UiMessage::error_data_schema_shape_definition_missing(
                            ),
                        )
                    })?,
                    stack,
                )?;
                stack.pop();
            }
            Shape::Scalar { semantic } => {
                if !SCALARS.contains(&semantic.as_str()) {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_unknown_scalar_codec(),
                    ));
                }
            }
            Shape::Enum { values, .. } => {
                unique(values.iter().map(String::as_str), "enum")?;
                if values.is_empty() {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_enum_empty(),
                    ));
                }
            }
            Shape::List { items, min, max } => {
                if max.is_some_and(|v| v < *min) {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_invalid_list_cardinality(),
                    ));
                }
                self.check_shape(items, stack)?;
            }
            Shape::Record {
                members,
                unknown,
                constraints,
            } => {
                if !matches!(unknown.as_str(), "preserve" | "reject") {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_unknown_record_policy(),
                    ));
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
                                return Err(DataError::new(
                                    weftext_messages::UiMessage::error_data_schema_record_validator_reads_outside_local_shape(),
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
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_union_variants_required(),
                    ));
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
                                return Err(DataError::new(
                                    weftext_messages::UiMessage::error_data_schema_member_discriminator_variant_count_mismatch(),
                                ));
                            }
                            unique(
                                keys.iter().map(String::as_str),
                                "union member discriminator",
                            )?;
                            for (key, variant) in keys.iter().zip(variants) {
                                let Shape::Record { members, .. } = self.resolve(variant) else {
                                    return Err(DataError::new(
                                        weftext_messages::UiMessage::error_data_schema_member_discriminator_requires_record_variants(),
                                    ));
                                };
                                if !members.get(key).is_some_and(|m| m.required) {
                                    return Err(DataError::new(
                                        weftext_messages::UiMessage::error_data_schema_union_branch_discriminator_must_be_required(),
                                    ));
                                }
                            }
                        }
                        Discriminator::Tag { key } => {
                            let mut tags = BTreeSet::new();
                            for variant in variants {
                                let Shape::Record { members, .. } = self.resolve(variant) else {
                                    return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_tagged_union_requires_records()));
                                };
                                let member =
                                    members.get(key).filter(|m| m.required).ok_or_else(|| {
                                        DataError::new(weftext_messages::UiMessage::error_data_schema_union_tag_must_be_required_member())
                                    })?;
                                let Shape::Enum {
                                    values,
                                    open: false,
                                } = self.resolve(&member.shape)
                                else {
                                    return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_union_tags_require_fixed_enums()));
                                };
                                if values.len() != 1 || !tags.insert(&values[0]) {
                                    return Err(DataError::new(
                                        weftext_messages::UiMessage::error_data_schema_union_tags_overlap_or_not_fixed(),
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
                                return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_record_presence_variants_malformed()));
                            }
                            let Shape::Record { members, .. } = self.resolve(&variants[*present])
                            else {
                                return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_presence_branch_requires_record()));
                            };
                            if !members.get(key).is_some_and(|m| m.required)
                                || !matches!(self.resolve(&variants[*absent]),Shape::Record{members,..}if !members.contains_key(key))
                            {
                                return Err(DataError::new(
                                    weftext_messages::UiMessage::error_data_schema_record_presence_discriminator_overlaps(),
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
            .ok_or_else(|| {
                DataError::new(
                    weftext_messages::UiMessage::error_data_schema_unknown_constraint_validator(),
                )
            })?;
        let slots: &[&str] = match v.operation.as_str() {
            "custom-code" => &["code", "custom"],
            "owned-resources" => &["owner", "values", "metadata"],
            "event" => &["range", "recurrence"],
            "task-reminder" => &["reminder", "due"],
            _ => &["value"],
        };
        if c.inputs.keys().map(String::as_str).collect::<BTreeSet<_>>()
            != slots.iter().copied().collect()
        {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_validator_input_slots_mismatch(),
            ));
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
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_validator_parameters_mismatch(),
            ));
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
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_validator_parameter_types_or_ranges_mismatch(),
            ));
        }
        for s in c.inputs.values() {
            match s {
                Selector::Context { name }
                    if !matches!(name.as_str(), "owner" | "resource_metadata") =>
                {
                    return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_unknown_validator_context()));
                }
                Selector::Field { id, path } => self.check_path(
                    &self
                        .field(id)
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_constraint_field_unresolved()))?
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
                    .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_schema_member_path_unresolved()))?
                    .shape,
                &path[1..],
            ),
            (Shape::Union { variants, .. }, _) => {
                let supported = variants.iter().any(|v| self.check_path(v, path).is_ok());
                if supported {
                    Ok(())
                } else {
                    Err(DataError::new(weftext_messages::UiMessage::error_data_schema_union_path_unresolved()))
                }
            }
            _ => Err(DataError::new(weftext_messages::UiMessage::error_data_schema_schema_path_type_mismatch())),
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
                    Err(DataError::new(weftext_messages::UiMessage::error_data_schema_relation_assertion_path_unresolved()))
                }
            }
            _ => Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_relation_path_requires_exact_node_ref_slot(),
            )),
        }
    }
    fn check_provider_data(&self, p: &Provider) -> Result<()> {
        let kind = p.data.get("kind").and_then(Value::as_str).ok_or_else(|| {
            DataError::new(
                weftext_messages::UiMessage::error_data_schema_provider_data_kind_missing(),
            )
        })?;
        let want = match p.implementation.id.as_str() {
            "weftext.core.rules.v1" | "weftext.temporal.rules.v2" => "none",
            "weftext.units.rules.v1" => "unit-table",
            "weftext.calendar.gregorian.v2" => "gregorian",
            "weftext.calendar.table.v2" => "calendar-table",
            "weftext.zone.table.v1" => "zone-table",
            _ => "",
        };
        if kind != want {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_provider_data_kind_mismatch(),
            ));
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
            _ => {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_unknown_provider_data(),
                ));
            }
        };
        exact_members(&p.data, keys)?;
        if kind == "gregorian"
            && p.data["min_year"]
                .as_i64()
                .zip(p.data["max_year"].as_i64())
                .is_none_or(|(a, b)| a < 1 || b > 9999 || a > b)
        {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_invalid_gregorian_coverage(),
            ));
        }
        if kind == "unit-table" {
            let units = p.data["units"].as_array().ok_or_else(|| {
                DataError::new(
                    weftext_messages::UiMessage::error_data_schema_unit_table_units_required(),
                )
            })?;
            let mut seen = BTreeSet::new();
            for unit in units {
                exact_members(unit, &["key", "dimension"])?;
                if !unit["key"]
                    .as_str()
                    .is_some_and(|key| !key.is_empty() && seen.insert(key))
                    || !unit["dimension"].as_str().is_some_and(|d| !d.is_empty())
                {
                    return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_unit_definition_invalid_or_duplicate()));
                }
            }
        }
        if matches!(kind, "gregorian" | "calendar-table" | "zone-table")
            && ["key", "version"]
                .iter()
                .any(|k| p.data[*k].as_str().is_none_or(str::is_empty))
        {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_provider_key_version_requires_exact_nonempty_strings(),
            ));
        }
        if kind == "calendar-table" {
            let epoch = p.data["epoch_date"]
                .as_str()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_calendar_table_epoch_date_required()))?;
            let years = p.data["years"]
                .as_array()
                .filter(|a| !a.is_empty())
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_calendar_table_finite_years_required()))?;
            let mut previous = None;
            let mut days = 0i64;
            for year in years {
                exact_members(year, &["year", "months"])?;
                let y = year["year"]
                    .as_i64()
                    .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_calendar_table_year_requires_integer()))?;
                if previous.is_some_and(|v: i64| v.checked_add(1) != Some(y)) {
                    return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_calendar_table_years_must_be_consecutive()));
                }
                previous = Some(y);
                let months = year["months"]
                    .as_array()
                    .filter(|a| !a.is_empty())
                    .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_calendar_year_ordered_months_required()))?;
                let mut seen = BTreeSet::new();
                for month in months {
                    exact_members(month, &["number", "leap", "days"])?;
                    let number = month["number"]
                        .as_i64()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_month_number_must_be_positive()))?;
                    let leap = month["leap"]
                        .as_bool()
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_month_leap_flag_requires_boolean()))?;
                    let length = month["days"].as_i64().filter(|n| *n > 0).ok_or_else(|| {
                        DataError::new(
                            weftext_messages::UiMessage::error_data_schema_month_days_must_be_positive(),
                        )
                    })?;
                    if !seen.insert((number, leap)) {
                        return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_duplicate_table_month_number_and_leap()));
                    }
                    days = days
                        .checked_add(length)
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_calendar_table_cumulative_overflow()))?;
                }
            }
            if epoch
                .checked_add_days(chrono::Days::new(days as u64))
                .is_none()
            {
                return Err(DataError::new(weftext_messages::UiMessage::error_data_schema_calendar_table_epoch_coverage_overflow()));
            }
        }
        if kind == "zone-table" {
            let from = p.data["from"]
                .as_str()
                .map(crate::ExactInstant::parse)
                .transpose()?
                .ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_schema_zone_coverage_start_required(
                        ),
                    )
                })?;
            let until = p.data["until"]
                .as_str()
                .map(crate::ExactInstant::parse)
                .transpose()?
                .ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_schema_zone_coverage_end_required(),
                    )
                })?;
            if !from.compare(&until).is_lt() {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_zone_coverage_must_be_ordered(),
                ));
            }
            let valid_offset =
                |v: &Value| v.as_i64().is_some_and(|n| (-86400..=86400).contains(&n));
            if !valid_offset(&p.data["initial_offset_seconds"]) {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_schema_invalid_initial_zone_offset(),
                ));
            }
            let mut previous = from;
            for t in p.data["transitions"].as_array().ok_or_else(|| {
                DataError::new(
                    weftext_messages::UiMessage::error_data_schema_zone_transitions_required(),
                )
            })? {
                exact_members(t, &["at", "offset_seconds"])?;
                let at = t["at"]
                    .as_str()
                    .map(crate::ExactInstant::parse)
                    .transpose()?
                    .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_schema_zone_transition_instant_required()))?;
                if !previous.compare(&at).is_lt()
                    || !at.compare(&until).is_lt()
                    || !valid_offset(&t["offset_seconds"])
                {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_schema_zone_transitions_order_coverage_or_bounds_invalid(),
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
            out.unavailable
                .push(weftext_messages::UiMessage::error_data_schema_validator_missing().into());
            return out;
        };
        let get = |slot: &str| -> Option<&NativeValue> {
            match c.inputs.get(slot)? {
                Selector::Local { path } => select_one(local, path),
                _ => None,
            }
        };
        let bad = |out: &mut Validation, explanation: UiMessage| {
            out.diagnostics.push(
                UiMessage::error_data_schema_validator_rejected(
                    weftext_messages::params::ErrorDataSchemaValidatorRejected {
                        validator: c.validator.clone(),
                    },
                )
                .into(),
            );
            out.diagnostics.push(explanation.into());
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
                        bad(
                            &mut out,
                            weftext_messages::UiMessage::error_data_schema_integer_out_of_range(),
                        );
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
                                bad(
                                    &mut out,
                                    weftext_messages::UiMessage::error_data_schema_decimal_out_of_range(),
                                );
                            }
                        }
                        _ => bad(
                            &mut out,
                            weftext_messages::UiMessage::error_data_schema_invalid_exact_decimal(),
                        ),
                    }
                }
            }
            "custom-code" => {
                if get("code").and_then(NativeValue::text) == c.parameters["code"].as_str()
                    && get("custom")
                        .and_then(NativeValue::text)
                        .is_none_or(str::is_empty)
                {
                    bad(
                        &mut out,
                        weftext_messages::UiMessage::error_data_schema_custom_code_requires_custom_text(),
                    );
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
                        bad(
                            &mut out,
                            weftext_messages::UiMessage::error_data_schema_invalid_month_day(),
                        );
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
                        out.unavailable.push(
                            weftext_messages::UiMessage::error_data_schema_unknown_unit().into(),
                        );
                    }
                    if v.operation == "measurement" {
                        let expected = match value.member("kind").and_then(NativeValue::text) {
                            Some("height") => "length",
                            Some("weight") => "mass",
                            _ => "",
                        };
                        if found.is_some_and(|u| u["dimension"].as_str() != Some(expected)) {
                            bad(&mut out, weftext_messages::UiMessage::error_data_schema_measurement_unit_dimension_mismatch());
                        }
                    }
                    if let Some(p) = quantity.member("precision")
                        && let NativeValue::Decimal(s) | NativeValue::Text(s) = p
                        && ExactDecimal::parse(s)
                            .is_ok_and(|n| n.compare(&ExactDecimal::parse("0").unwrap()).is_lt())
                    {
                        bad(
                            &mut out,
                            weftext_messages::UiMessage::error_data_schema_negative_precision(),
                        );
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
            "owned-resources" | "event" | "task-reminder" => out
                .unavailable
                .push(weftext_messages::UiMessage::error_data_schema_required_core_field_or_context_snapshot_missing().into()),
            _ => out.unavailable.push(
                weftext_messages::UiMessage::error_data_schema_validation_operation_unavailable()
                    .into(),
            ),
        }
        out
    }
}
fn validation_cell(cell: crate::Cell, out: &mut Validation) {
    match cell {
        crate::Cell::Invalid { diagnostics, .. } => out.diagnostics.extend(diagnostics),
        crate::Cell::Unavailable {
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
fn exact_members(value: &Value, keys: &[&str]) -> Result<()> {
    if value.as_object().is_none_or(|o| {
        o.keys().map(String::as_str).collect::<BTreeSet<_>>() != keys.iter().copied().collect()
    }) {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_schema_configuration_members_unknown_or_missing(
            ),
        ));
    }
    Ok(())
}
fn unique<'a>(values: impl Iterator<Item = &'a str>, kind: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for v in values {
        if v.is_empty() || !seen.insert(v) {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_identifier_duplicate_or_empty(
                    weftext_messages::params::ErrorDataSchemaIdentifierDuplicateOrEmpty {
                        identifier_kind: (kind).to_string(),
                    },
                ),
            ));
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
            .ok_or_else(|| {
                DataError::new(weftext_messages::UiMessage::error_data_schema_invalid_integer())
            })?,
        (Float(a), Float(b)) => a
            .parse::<f64>()
            .ok()
            .zip(b.parse::<f64>().ok())
            .and_then(|(a, b)| a.partial_cmp(&b))
            .ok_or_else(|| {
                DataError::new(weftext_messages::UiMessage::error_data_schema_invalid_float())
            })?,
        (Decimal(a), Decimal(b)) => ExactDecimal::parse(a)?.compare(&ExactDecimal::parse(b)?),
        (Date(a), Date(b)) | (Time(a), Time(b)) | (LocalDatetime(a), LocalDatetime(b)) => a.cmp(b),
        (Duration(a), Duration(b)) => a
            .parse::<i64>()
            .ok()
            .zip(b.parse::<i64>().ok())
            .map(|(a, b)| a.cmp(&b))
            .ok_or_else(|| {
                DataError::new(weftext_messages::UiMessage::error_data_schema_invalid_duration())
            })?,
        _ => {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_schema_explicit_same_type_comparator_required(),
            ));
        }
    };
    Ok(order)
}
/// Parse a bounded-by-the-caller JSON input once, with explicit owned validation
/// facts. Object order and serde_json's i64/u64/finite-f64 number model remain
/// unchanged; callers retain original bytes for exact author provenance.
pub fn parse_unique_json(bytes: &[u8]) -> Result<Value> {
    let parsed: UniqueJson = serde_json::from_slice(bytes)?;
    if let Some(issue) = parsed.issue {
        return Err(match issue {
            UniqueJsonIssue::DuplicateKey { key } => {
                DataError::new(UiMessage::error_data_schema_json_key_duplicate(
                    weftext_messages::params::ErrorDataSchemaJsonKeyDuplicate { key },
                ))
            }
            UniqueJsonIssue::NonfiniteNumber => {
                DataError::new(UiMessage::error_data_schema_json_number_nonfinite())
            }
        });
    }
    Ok(parsed.value)
}

struct UniqueJson {
    value: Value,
    issue: Option<UniqueJsonIssue>,
}
enum UniqueJsonIssue {
    DuplicateKey { key: String },
    NonfiniteNumber,
}
impl UniqueJson {
    fn plain(value: Value) -> Self {
        Self { value, issue: None }
    }
}
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
                let mut issue = None;
                while let Some((k, v)) = m.next_entry::<String, UniqueJson>()? {
                    if issue.is_none() {
                        issue = v.issue;
                    }
                    if out.contains_key(&k) && issue.is_none() {
                        issue = Some(UniqueJsonIssue::DuplicateKey { key: k.clone() });
                    }
                    out.insert(k, v.value);
                }
                Ok(UniqueJson {
                    value: Value::Object(out),
                    issue,
                })
            }
            fn visit_seq<S: SeqAccess<'de>>(
                self,
                mut s: S,
            ) -> std::result::Result<Self::Value, S::Error> {
                let mut a = vec![];
                let mut issue = None;
                while let Some(v) = s.next_element::<UniqueJson>()? {
                    if issue.is_none() {
                        issue = v.issue;
                    }
                    a.push(v.value);
                }
                Ok(UniqueJson {
                    value: Value::Array(a),
                    issue,
                })
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson::plain(v.into()))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson::plain(v.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson::plain(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson::plain(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson::plain(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                Ok(match serde_json::Number::from_f64(v) {
                    Some(value) => UniqueJson::plain(value.into()),
                    None => UniqueJson {
                        value: Value::Null,
                        issue: Some(UniqueJsonIssue::NonfiniteNumber),
                    },
                })
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson::plain(Value::Null))
            }
        }
        d.deserialize_any(V)
    }
}
