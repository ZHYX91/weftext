//! Provider capabilities and read effects for the existing CEL evaluator.
use crate::query::visit_children_mut;
use crate::*;
use cel::{
    Program,
    common::ast::{EntryExpr, Expr, IdedExpr, LiteralValue},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn unavailable() -> DataError {
    DataError("Query provider unavailable".into())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBaseline {
    pub schema: String,
    pub revision: String,
}

/// Server-private effects. `None` denotes the caller's root baseline.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryProviderRequirements {
    pub baseline: Option<ProviderBaseline>,
    pub provider_calls: BTreeSet<String>,
    pub providers: BTreeMap<String, String>,
    pub series_policies: BTreeMap<String, String>,
    pub validators: BTreeSet<String>,
    pub definitions: BTreeSet<String>,
    pub dynamic_selectors: BTreeSet<String>,
}

/// The host verifies grants and descriptor fingerprints before constructing
/// this capability. It is deliberately not a deserializable wire authority.
#[derive(Debug, Clone, Default)]
pub struct QueryProviderAccess {
    pub providers: BTreeSet<String>,
    pub series_policies: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryProviderDependency {
    pub baseline: Option<ProviderBaseline>,
    pub providers: BTreeMap<String, String>,
    pub series_policies: BTreeMap<String, String>,
    pub exact_provider_selections: BTreeMap<String, Option<String>>,
    pub series_selections: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryProviderDependencies {
    pub baselines: Vec<QueryProviderDependency>,
}

fn canonical(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(m) => {
            let sorted: BTreeMap<_, _> = m.iter().map(|(k, v)| (k.clone(), canonical(v))).collect();
            serde_json::to_value(sorted).unwrap()
        }
        serde_json::Value::Array(a) => serde_json::Value::Array(a.iter().map(canonical).collect()),
        v => v.clone(),
    }
}
fn record_fingerprint(kind: &str, record: &impl Serialize) -> Result<String> {
    let record = serde_json::to_value(record)?;
    let bytes = serde_json::to_vec(&canonical(&serde_json::json!({
        "profile":"weftext.query.cel.v1", "kind":kind, "record":record
    })))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
pub fn provider_fingerprint(provider: &Provider) -> Result<String> {
    record_fingerprint("provider", provider)
}
pub fn series_policy_fingerprint(policy: &SeriesPolicy) -> Result<String> {
    record_fingerprint("seriesPolicy", policy)
}

fn provider_family(provider: &Provider) -> Option<&'static str> {
    match provider.implementation.id.as_str() {
        "weftext.calendar.gregorian.v2" | "weftext.calendar.table.v2" => Some("calendar"),
        "weftext.zone.table.v1" => Some("zone"),
        _ => None,
    }
}

/// Project registries only; the caller keeps its existing field/type narrowing.
/// Dynamic candidates come exclusively from this explicit capability.
pub fn narrow_query_provider_schema(
    schema: &Schema,
    access: &QueryProviderAccess,
    requirements: &QueryProviderRequirements,
) -> Result<Schema> {
    let mut result = schema.clone();
    let mut providers: BTreeSet<_> = requirements.providers.keys().cloned().collect();
    let mut policies: BTreeSet<_> = requirements.series_policies.keys().cloned().collect();
    for (id, expected) in &requirements.providers {
        let provider = schema
            .providers
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(unavailable)?;
        if !access.providers.contains(id) || provider_fingerprint(provider)? != *expected {
            return Err(unavailable());
        }
    }
    for (id, expected) in &requirements.series_policies {
        let policy = schema
            .series_policies
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(unavailable)?;
        if !access.series_policies.contains(id) || series_policy_fingerprint(policy)? != *expected {
            return Err(unavailable());
        }
    }
    for provider in &schema.providers {
        if access.providers.contains(&provider.id)
            && provider_family(provider)
                .is_some_and(|family| requirements.dynamic_selectors.contains(family))
        {
            providers.insert(provider.id.clone());
        }
    }
    if requirements.dynamic_selectors.contains("series") {
        policies.extend(
            access
                .series_policies
                .iter()
                .filter(|id| schema.series_policies.iter().any(|p| &p.id == *id))
                .cloned(),
        );
    }
    result.providers.retain(|p| providers.contains(&p.id));
    result.series_policies.retain(|p| policies.contains(&p.id));
    result
        .validators
        .retain(|v| requirements.validators.contains(&v.id) && providers.contains(&v.provider));
    result
        .definitions
        .retain(|name, _| requirements.definitions.contains(name));
    Ok(result)
}

#[derive(Default, Serialize)]
pub(crate) struct SemanticDependencies {
    pub(crate) validators: BTreeMap<String, Validator>,
    pub(crate) providers: BTreeMap<String, Provider>,
    pub(crate) series_policies: BTreeMap<String, SeriesPolicy>,
    pub(crate) exact_provider_selections: BTreeMap<String, Option<String>>,
    pub(crate) series_selections: BTreeMap<String, Option<String>>,
}

impl SemanticDependencies {
    pub(crate) fn provider(&mut self, schema: &Schema, id: &str) -> Result<()> {
        let provider = schema
            .providers
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(unavailable)?;
        self.providers.insert(id.into(), provider.clone());
        Ok(())
    }

    pub(crate) fn exact_provider(
        &mut self,
        schema: &Schema,
        key: &str,
        version: &str,
        family: &str,
    ) {
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

    pub(crate) fn endpoint(&mut self, schema: &Schema, value: &NativeValue) {
        if let (Some(key), Some(version)) = (
            value.member("calendar").and_then(NativeValue::text),
            value.member("version").and_then(NativeValue::text),
        ) {
            self.exact_provider(schema, key, version, "calendar");
        }
    }

    pub(crate) fn recurrence(&mut self, schema: &Schema, value: &NativeValue) {
        for (family, version_key) in [("calendar", "calendar_version"), ("zone", "zone_version")] {
            if let (Some(key), Some(version)) = (
                value.member(family).and_then(NativeValue::text),
                value.member(version_key).and_then(NativeValue::text),
            ) {
                self.exact_provider(schema, key, version, family);
            }
        }
    }

    pub(crate) fn constraint(
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
                        crate::projection::select_values(value, path, &mut values);
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

    pub(crate) fn shape(
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
            Shape::Use { name } => {
                let shape = schema.definitions.get(name).ok_or_else(unavailable)?;
                self.shape(schema, shape, value, remaining)?;
            }
            _ => {}
        }
        Ok(())
    }
}

pub(crate) const PROVIDER_CALLS: &[(&str, usize)] = &[
    ("containsDate", 2),
    ("rangeOverlap", 2),
    ("compareTemporal", 2),
    ("nextCivilDay", 1),
    ("rangeDuration", 1),
    ("duringDuration", 1),
    ("periodBounds", 1),
    ("expandRecurrence", 3),
];
fn call_families(name: &str) -> &'static [&'static str] {
    match name {
        "periodBounds" => &["calendar", "zone", "series"],
        "expandRecurrence" => &["calendar", "zone"],
        "nextCivilDay" => &[],
        _ => &["calendar"],
    }
}
impl SemanticDependencies {
    pub(crate) fn call(&mut self, schema: &Schema, name: &str, args: &[NativeValue]) -> Result<()> {
        let (_, arity) = PROVIDER_CALLS
            .iter()
            .find(|(n, _)| *n == name)
            .ok_or_else(unavailable)?;
        if args.len() != *arity {
            return Err(unavailable());
        }
        let rule = schema
            .providers
            .iter()
            .find(|p| p.implementation.id == "weftext.temporal.rules.v2")
            .ok_or_else(unavailable)?;
        self.provider(schema, &rule.id)?;
        let endpoints = |deps: &mut Self, value: &NativeValue, end: &str| {
            for key in ["start", end] {
                if let Some(point) = value.member(key) {
                    deps.endpoint(schema, point);
                }
            }
        };
        match name {
            "compareTemporal" => {
                for value in args {
                    self.endpoint(schema, value);
                }
            }
            "containsDate" => {
                endpoints(self, &args[0], "end");
                self.endpoint(schema, &args[1]);
            }
            "rangeOverlap" => {
                for value in args {
                    endpoints(self, value, "end_exclusive");
                }
            }
            "rangeDuration" => endpoints(self, &args[0], "end_exclusive"),
            "duringDuration" => endpoints(self, &args[0], "end"),
            "periodBounds" => {
                if let Some(key) = args[0].member("series_key").and_then(NativeValue::text) {
                    let policy = schema.series_policies.iter().find(|p| p.key == key);
                    self.series_selections
                        .insert(key.into(), policy.map(|p| p.id.clone()));
                    if let Some(policy) = policy {
                        self.series_policies
                            .insert(policy.id.clone(), policy.clone());
                        // An absent selected provider is a negative dependency,
                        // rather than an ambient registry lookup or a fallback.
                        for id in [&policy.calendar, &policy.zone] {
                            if schema.providers.iter().any(|p| &p.id == id) {
                                self.provider(schema, id)?;
                            }
                        }
                    }
                }
                for (family, key, version) in [
                    ("calendar", "calendar", "calendar_version"),
                    ("zone", "zone", "zone_version"),
                ] {
                    if let (Some(key), Some(version)) = (
                        args[0].member(key).and_then(NativeValue::text),
                        args[0].member(version).and_then(NativeValue::text),
                    ) {
                        self.exact_provider(schema, key, version, family);
                    }
                }
            }
            "expandRecurrence" => {
                if let Some(recurrence) = args[0].member("recurrence") {
                    self.recurrence(schema, recurrence);
                }
                endpoints(self, &args[1], "end_exclusive");
                if let Some(range) = args[0].member("range") {
                    endpoints(self, range, "end_exclusive");
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct ProviderTrace {
    state: Arc<Mutex<BTreeMap<Option<ProviderBaseline>, QueryProviderDependency>>>,
    baseline: Option<ProviderBaseline>,
    failed: Arc<AtomicBool>,
}
impl ProviderTrace {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(BTreeMap::new())),
            baseline: None,
            failed: Arc::new(AtomicBool::new(false)),
        }
    }
    pub(crate) fn at(&self, baseline: Option<ProviderBaseline>) -> Self {
        Self {
            state: self.state.clone(),
            baseline,
            failed: self.failed.clone(),
        }
    }
    pub(crate) fn record(&self, dependencies: &SemanticDependencies) -> Result<()> {
        if self.failed.load(Ordering::Relaxed) {
            return Err(DataError(
                "Query provider dependency budget exhausted".into(),
            ));
        }
        let mut all = self.state.lock().map_err(|_| unavailable())?;
        let record = all
            .entry(self.baseline.clone())
            .or_insert_with(|| QueryProviderDependency {
                baseline: self.baseline.clone(),
                ..Default::default()
            });
        for (id, p) in &dependencies.providers {
            if !record.providers.contains_key(id) {
                record
                    .providers
                    .insert(id.clone(), provider_fingerprint(p)?);
            }
        }
        for (id, p) in &dependencies.series_policies {
            if !record.series_policies.contains_key(id) {
                record
                    .series_policies
                    .insert(id.clone(), series_policy_fingerprint(p)?);
            }
        }
        record
            .exact_provider_selections
            .extend(dependencies.exact_provider_selections.clone());
        record
            .series_selections
            .extend(dependencies.series_selections.clone());
        let count: usize = all
            .values()
            .map(|r| {
                r.providers.len()
                    + r.series_policies.len()
                    + r.exact_provider_selections.len()
                    + r.series_selections.len()
            })
            .sum();
        if count > 1024
            || serde_json::to_vec(&all.values().collect::<Vec<_>>())?.len() > 1024 * 1024
        {
            self.failed.store(true, Ordering::Relaxed);
            all.clear();
            return Err(DataError(
                "Query provider dependency budget exhausted".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn finish(&self) -> Result<QueryProviderDependencies> {
        if self.failed.load(Ordering::Relaxed) {
            return Err(DataError(
                "Query provider dependency budget exhausted".into(),
            ));
        }
        Ok(QueryProviderDependencies {
            baselines: self
                .state
                .lock()
                .map_err(|_| unavailable())?
                .values()
                .cloned()
                .collect(),
        })
    }
}

pub(crate) fn trace_parameters(
    plan: &QueryPlan,
    schema: &Schema,
    values: &BTreeMap<String, NativeValue>,
    trace: &ProviderTrace,
    budget: &mut u64,
) -> Result<()> {
    let mut dependencies = SemanticDependencies::default();
    let initial = (*budget).min(200_000) as usize;
    let mut remaining = initial;
    if let Some(advanced) = &plan.advanced {
        for parameter in &advanced.parameters {
            dependencies.shape(
                schema,
                &parameter.shape,
                values.get(&parameter.name).or(parameter.default.as_ref()),
                &mut remaining,
            )?;
        }
    }
    *budget = budget.saturating_sub((initial - remaining) as u64);
    trace.record(&dependencies)
}

fn known_native(
    ast: &IdedExpr,
    values: &BTreeMap<String, NativeValue>,
    depth: usize,
) -> Option<NativeValue> {
    if depth > 64 {
        return None;
    }
    match &ast.expr {
        Expr::Literal(LiteralValue::Boolean(v)) => Some(NativeValue::Bool(v.into_inner())),
        Expr::Literal(LiteralValue::Int(v)) => Some(NativeValue::Int(v.into_inner().to_string())),
        Expr::Literal(LiteralValue::Double(v)) => {
            Some(NativeValue::Float(v.into_inner().to_string()))
        }
        Expr::Literal(LiteralValue::String(v)) => Some(NativeValue::Text(v.inner().to_string())),
        Expr::Literal(LiteralValue::Null) => Some(NativeValue::None),
        Expr::Ident(name) => values.get(name).cloned(),
        Expr::Select(s) if !s.test => known_native(&s.operand, values, depth + 1)?
            .member(&s.field)
            .cloned(),
        Expr::List(a) => Some(NativeValue::Array(
            a.elements
                .iter()
                .map(|e| known_native(e, values, depth + 1))
                .collect::<Option<_>>()?,
        )),
        Expr::Map(m) => Some(NativeValue::Record(
            m.entries
                .iter()
                .map(|entry| {
                    let EntryExpr::MapEntry(entry) = &entry.expr else {
                        return None;
                    };
                    let NativeValue::Text(key) = known_native(&entry.key, values, depth + 1)?
                    else {
                        return None;
                    };
                    Some(NativeMember {
                        key,
                        value: known_native(&entry.value, values, depth + 1)?,
                    })
                })
                .collect::<Option<_>>()?,
        )),
        _ => None,
    }
}

fn partial_native(
    ast: &IdedExpr,
    values: &BTreeMap<String, NativeValue>,
    depth: usize,
) -> Option<NativeValue> {
    if depth > 64 {
        return None;
    }
    if let Some(value) = known_native(ast, values, depth) {
        return Some(value);
    }
    match &ast.expr {
        Expr::Map(map) => Some(NativeValue::Record(
            map.entries
                .iter()
                .map(|entry| {
                    let EntryExpr::MapEntry(entry) = &entry.expr else {
                        return None;
                    };
                    let NativeValue::Text(key) = known_native(&entry.key, values, depth + 1)?
                    else {
                        return None;
                    };
                    Some(NativeMember {
                        key,
                        value: partial_native(&entry.value, values, depth + 1)
                            .unwrap_or(NativeValue::None),
                    })
                })
                .collect::<Option<_>>()?,
        )),
        Expr::List(list) => Some(NativeValue::Array(
            list.elements
                .iter()
                .map(|a| partial_native(a, values, depth + 1).unwrap_or(NativeValue::None))
                .collect(),
        )),
        _ => None,
    }
}

fn shape_effects(
    schema: &Schema,
    shape: &Shape,
    value: Option<&NativeValue>,
    requirements: &mut QueryProviderRequirements,
    dependencies: &mut SemanticDependencies,
    remaining: &mut usize,
) -> Result<()> {
    if *remaining == 0 {
        return Err(unavailable());
    }
    *remaining -= 1;
    match shape {
        Shape::Use { name } => {
            requirements.definitions.insert(name.clone());
            shape_effects(
                schema,
                schema.definitions.get(name).ok_or_else(unavailable)?,
                value,
                requirements,
                dependencies,
                remaining,
            )?;
        }
        Shape::Record {
            members,
            constraints,
            ..
        } => {
            for constraint in constraints {
                dependencies.constraint(schema, constraint, value, None)?;
                if value.is_none() {
                    let operation = schema
                        .validators
                        .iter()
                        .find(|v| v.id == constraint.validator)
                        .ok_or_else(unavailable)?
                        .operation
                        .as_str();
                    let families = match operation {
                        "calendar-date" | "range" | "during" => &["calendar"][..],
                        "period" => &["calendar", "zone", "series"][..],
                        "recurrence" | "event" => &["calendar", "zone"][..],
                        _ => &[][..],
                    };
                    requirements
                        .dynamic_selectors
                        .extend(families.iter().map(|s| s.to_string()));
                }
            }
            for (name, member) in members {
                shape_effects(
                    schema,
                    &member.shape,
                    value.and_then(|v| v.member(name)),
                    requirements,
                    dependencies,
                    remaining,
                )?;
            }
        }
        Shape::List { items, .. } => {
            if let Some(values) = value.and_then(NativeValue::array) {
                for value in values {
                    shape_effects(
                        schema,
                        items,
                        Some(value),
                        requirements,
                        dependencies,
                        remaining,
                    )?;
                }
            } else {
                shape_effects(schema, items, None, requirements, dependencies, remaining)?;
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
                shape_effects(
                    schema,
                    variant,
                    value,
                    requirements,
                    dependencies,
                    remaining,
                )?;
            } else {
                for variant in variants {
                    shape_effects(schema, variant, None, requirements, dependencies, remaining)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Collect static symbols and selectors without computing any Provider data.
/// Required parameters that lack a value keep a structured dynamic requirement.
pub fn query_provider_requirements(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
) -> Result<Vec<QueryProviderRequirements>> {
    provider_requirements(plan, schema, context, false)
}
/// Runtime inputs already use an authorized registry projection. Missing exact
/// selections stay negative dependencies for the original evaluator's preflight.
pub fn query_provider_runtime_requirements(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
) -> Result<Vec<QueryProviderRequirements>> {
    provider_requirements(plan, schema, context, true)
}
fn provider_requirements(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
    allow_missing: bool,
) -> Result<Vec<QueryProviderRequirements>> {
    // Pure selector evidence must never become a fabricated execution value.
    #[derive(Clone, Default)]
    struct StaticValues {
        known: BTreeMap<String, NativeValue>,
        selectors: BTreeMap<String, NativeValue>,
    }
    struct State {
        requirements: BTreeMap<Option<ProviderBaseline>, QueryProviderRequirements>,
        nodes: usize,
        remaining: usize,
        allow_missing: bool,
    }
    fn expression(
        source: &str,
        schema: &Schema,
        values: &StaticValues,
        req: &mut QueryProviderRequirements,
        deps: &mut SemanticDependencies,
        remaining: &mut usize,
    ) -> Result<()> {
        if source.len() > 16384 {
            return Err(unavailable());
        }
        fn walk(
            ast: &IdedExpr,
            schema: &Schema,
            values: &StaticValues,
            req: &mut QueryProviderRequirements,
            deps: &mut SemanticDependencies,
            remaining: &mut usize,
        ) -> Result<()> {
            if *remaining == 0 {
                return Err(unavailable());
            }
            *remaining -= 1;
            if let Expr::Call(call) = &ast.expr
                && let Some((name, arity)) = PROVIDER_CALLS
                    .iter()
                    .find(|(name, _)| *name == call.func_name)
            {
                if call.target.is_some() || call.args.len() != *arity {
                    return Err(unavailable());
                }
                req.provider_calls.insert(name.to_string());
                let rule = schema
                    .providers
                    .iter()
                    .find(|p| p.implementation.id == "weftext.temporal.rules.v2")
                    .ok_or_else(unavailable)?;
                deps.provider(schema, &rule.id)?;
                if call
                    .args
                    .iter()
                    .any(|a| known_native(a, &values.known, 0).is_none())
                {
                    req.dynamic_selectors
                        .extend(call_families(name).iter().map(|s| s.to_string()));
                }
                let args = call
                    .args
                    .iter()
                    .map(|a| partial_native(a, &values.selectors, 0))
                    .collect::<Vec<_>>();
                // Every known argument contributes independently. An unknown
                // sibling never erases a private literal's static selection.
                deps.call(
                    schema,
                    name,
                    &args
                        .into_iter()
                        .map(|a| a.unwrap_or(NativeValue::None))
                        .collect::<Vec<_>>(),
                )?;
            }
            if let Expr::Comprehension(c) = &ast.expr {
                walk(&c.iter_range, schema, values, req, deps, remaining)?;
                walk(&c.accu_init, schema, values, req, deps, remaining)?;
                let mut scoped = values.clone();
                scoped.known.remove(&c.iter_var);
                scoped.selectors.remove(&c.iter_var);
                scoped.known.remove(&c.accu_var);
                scoped.selectors.remove(&c.accu_var);
                if let Some(name) = &c.iter_var2 {
                    scoped.known.remove(name);
                    scoped.selectors.remove(name);
                }
                for child in [&c.loop_cond, &c.loop_step, &c.result] {
                    walk(child, schema, &scoped, req, deps, remaining)?;
                }
                return Ok(());
            }
            let mut copy = ast.clone();
            visit_children_mut(&mut copy, |child| {
                walk(child, schema, values, req, deps, remaining)
            })
        }
        let program = Program::compile(source).map_err(|_| unavailable())?;
        walk(program.expression(), schema, values, req, deps, remaining)
    }
    fn visit(
        plan: &QueryPlan,
        schema: &Schema,
        context: &FrozenAdvancedInput,
        baseline: Option<ProviderBaseline>,
        supplied: &StaticValues,
        state: &mut State,
        depth: usize,
    ) -> Result<()> {
        state.nodes += 1;
        if depth > 64 || state.nodes > 1024 {
            return Err(unavailable());
        }
        let a = plan.advanced.clone().unwrap_or_default();
        let mut values = StaticValues::default();
        let mut req =
            state
                .requirements
                .remove(&baseline)
                .unwrap_or_else(|| QueryProviderRequirements {
                    baseline: baseline.clone(),
                    ..Default::default()
                });
        let mut deps = SemanticDependencies::default();
        for parameter in &a.parameters {
            let value = supplied
                .known
                .get(&parameter.name)
                .or(parameter.default.as_ref());
            let evidence = supplied.selectors.get(&parameter.name).or(value);
            if let Some(value) = value {
                values.known.insert(parameter.name.clone(), value.clone());
            }
            if let Some(evidence) = evidence {
                values
                    .selectors
                    .insert(parameter.name.clone(), evidence.clone());
            }
            shape_effects(
                schema,
                &parameter.shape,
                evidence,
                &mut req,
                &mut deps,
                &mut state.remaining,
            )?;
            if evidence.is_some() && value.is_none() {
                shape_effects(
                    schema,
                    &parameter.shape,
                    None,
                    &mut req,
                    &mut deps,
                    &mut state.remaining,
                )?;
            }
        }
        for source in plan
            .r#where
            .iter()
            .chain(plan.select.iter().map(|c| &c.expr))
            .chain(plan.order.iter().map(|o| &o.expr))
            .chain(plan.unnest.iter().map(|u| &u.array))
        {
            expression(
                source,
                schema,
                &values,
                &mut req,
                &mut deps,
                &mut state.remaining,
            )?;
        }
        if let Some(group) = &plan.group {
            for source in group
                .keys
                .iter()
                .map(|k| &k.expr)
                .chain(group.aggregates.iter().filter_map(|a| a.expr.as_ref()))
            {
                expression(
                    source,
                    schema,
                    &values,
                    &mut req,
                    &mut deps,
                    &mut state.remaining,
                )?;
            }
        }
        if let Some(keys) = &a.distinct {
            for key in keys {
                expression(
                    &key.expr,
                    schema,
                    &values,
                    &mut req,
                    &mut deps,
                    &mut state.remaining,
                )?;
            }
        }
        let mut call_values = StaticValues::default();
        if let Some(call) = &a.call {
            for (name, source) in &call.arguments {
                expression(
                    source,
                    schema,
                    &values,
                    &mut req,
                    &mut deps,
                    &mut state.remaining,
                )?;
                let program = Program::compile(source).map_err(|_| unavailable())?;
                if let Some(value) = known_native(program.expression(), &values.known, 0) {
                    call_values.known.insert(name.clone(), value);
                }
                if let Some(value) = partial_native(program.expression(), &values.selectors, 0) {
                    call_values.selectors.insert(name.clone(), value);
                }
            }
        }
        req.validators.extend(deps.validators.keys().cloned());
        if !state.allow_missing
            && (deps.exact_provider_selections.values().any(Option::is_none)
                || deps.series_selections.values().any(Option::is_none))
        {
            return Err(unavailable());
        }
        for (selector, selected) in &deps.exact_provider_selections {
            if selected.is_none()
                && let Ok(parts) = serde_json::from_str::<Vec<String>>(selector)
                && let Some(family) = parts.first()
            {
                req.dynamic_selectors.insert(family.clone());
            }
        }
        if deps.series_selections.values().any(Option::is_none) {
            req.dynamic_selectors.insert("series".into());
        }
        for (id, p) in deps.providers {
            req.providers.insert(id, provider_fingerprint(&p)?);
        }
        for (id, p) in deps.series_policies {
            req.series_policies
                .insert(id, series_policy_fingerprint(&p)?);
        }
        state.requirements.insert(baseline.clone(), req);
        for branch in &a.union_all {
            visit(
                branch,
                schema,
                context,
                baseline.clone(),
                &values,
                state,
                depth + 1,
            )?;
        }
        if let Some(related) = &a.related {
            visit(
                &related.from,
                schema,
                context,
                baseline.clone(),
                &values,
                state,
                depth + 1,
            )?;
        }
        if let Some(call) = &a.call {
            let owner = context
                .definitions
                .iter()
                .find(|o| o.owner == call.owner)
                .ok_or_else(unavailable)?;
            let definition = select_definition(&call.selection, &owner.history)?;
            let matches = context
                .schemas
                .iter()
                .filter(|s| {
                    s.schema == definition.binding.schema
                        && s.revision == definition.binding.revision
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(unavailable());
            }
            visit(
                &definition.plan,
                &matches[0].projection,
                context,
                Some(ProviderBaseline {
                    schema: matches[0].schema.clone(),
                    revision: matches[0].revision.clone(),
                }),
                &call_values,
                state,
                depth + 1,
            )?;
        }
        Ok(())
    }
    definition_dependencies(plan, &context.definitions)?;
    let mut state = State {
        requirements: BTreeMap::new(),
        nodes: 0,
        remaining: 200_000,
        allow_missing,
    };
    visit(
        plan,
        schema,
        context,
        None,
        &StaticValues {
            known: context.parameters.clone(),
            selectors: context.parameters.clone(),
        },
        &mut state,
        0,
    )?;
    Ok(state.requirements.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_trace_deduplicates_occurrences_and_latches_a_bounded_failure() {
        let trace = ProviderTrace::new();
        let mut dependency = SemanticDependencies::default();
        dependency
            .exact_provider_selections
            .insert("one".into(), None);
        for _ in 0..2048 {
            trace.record(&dependency).unwrap();
        }
        assert_eq!(
            trace.finish().unwrap().baselines[0]
                .exact_provider_selections
                .len(),
            1
        );
        for n in 0..1023 {
            dependency.exact_provider_selections.clear();
            dependency
                .exact_provider_selections
                .insert(n.to_string(), None);
            trace.record(&dependency).unwrap();
        }
        dependency
            .exact_provider_selections
            .insert("overflow".into(), None);
        assert!(trace.record(&dependency).is_err());
        assert!(trace.finish().is_err());
        assert!(trace.state.lock().unwrap().is_empty());
        dependency.exact_provider_selections.clear();
        assert!(trace.record(&dependency).is_err());
    }
    #[test]
    fn provider_trace_rejects_oversized_negative_selectors_without_retaining_them() {
        let trace = ProviderTrace::new();
        let mut dependency = SemanticDependencies::default();
        dependency
            .series_selections
            .insert("x".repeat(1024 * 1024), None);
        assert!(trace.record(&dependency).is_err());
        assert!(trace.state.lock().unwrap().is_empty());
        assert!(trace.finish().is_err());
    }
}
