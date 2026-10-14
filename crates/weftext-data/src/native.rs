use crate::{DataError, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashSet, str::FromStr};
use typst_syntax::{
    Source, SyntaxKind, SyntaxNode,
    ast::{self, AstNode},
};
pub(crate) const NATIVE_MAX_BYTES: usize = 8 * 1024 * 1024;
const NATIVE_MAX_NODES: usize = 100_000;
const NATIVE_MAX_DEPTH: usize = 64;

/// All numbers use exact text in the transport, including i64 and binary64.
/// Records use ordered entries, avoiding JSON object-order ambiguity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum NativeValue {
    None,
    Text(String),
    Bool(bool),
    Int(String),
    Float(String),
    Decimal(String),
    Date(String),
    Time(String),
    LocalDatetime(String),
    Duration(String),
    Array(Vec<NativeValue>),
    Record(Vec<NativeMember>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeMember {
    pub key: String,
    pub value: NativeValue,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ValuePath {
    Member { key: String },
    Index { index: usize },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteRange {
    pub from: usize,
    pub to: usize,
}
#[derive(Debug, Clone)]
pub(crate) struct NativeNode {
    pub kind: SyntaxKind,
    pub value: Option<NativeValue>,
    pub range: ByteRange,
    pub children: Vec<(ValuePath, NativeNode)>,
    pub reason: Option<String>,
    pub commas: Vec<ByteRange>,
    pub frame: Option<Box<NativeNode>>,
    pub entries: Vec<(String, ByteRange)>,
    pub empty_colon: Option<ByteRange>,
}
impl NativeValue {
    pub fn member(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Record(a) => a.iter().find(|v| v.key == key).map(|v| &v.value),
            _ => None,
        }
    }
    pub fn text(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn integer(&self) -> Option<i64> {
        if let Self::Int(s) = self {
            s.parse().ok()
        } else {
            None
        }
    }
    pub fn array(&self) -> Option<&[Self]> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn record(&self) -> Option<&[NativeMember]> {
        if let Self::Record(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn at(&self, path: &[ValuePath]) -> Option<&Self> {
        let mut v = self;
        for p in path {
            v = match p {
                ValuePath::Member { key } => v.member(key)?,
                ValuePath::Index { index } => v.array()?.get(*index)?,
            };
        }
        Some(v)
    }
    pub fn to_typst(&self) -> Result<String> {
        check_native_budget(self)?;
        let text = self.to_typst_inner()?;
        if text.len() > NATIVE_MAX_BYTES {
            return Err(DataError("Native writer output budget exceeded".into()));
        }
        Ok(text)
    }
    fn to_typst_inner(&self) -> Result<String> {
        Ok(match self {
            Self::None => "none".into(),
            Self::Text(s) => typst_string(s),
            Self::Bool(b) => b.to_string(),
            Self::Int(s) => {
                let v: i64 = s
                    .parse()
                    .map_err(|_| DataError("Integer outside i64".into()))?;
                if v == i64::MIN {
                    format!("int({})", typst_string(s))
                } else {
                    v.to_string()
                }
            }
            Self::Float(s) => {
                let v: f64 = s.parse().map_err(|_| DataError("Invalid float".into()))?;
                if !v.is_finite() {
                    return Err(DataError("Nonfinite float".into()));
                }
                let mut out = v.to_string();
                if !out.contains(['.', 'e', 'E']) {
                    out.push_str(".0");
                }
                out
            }
            Self::Decimal(s) => {
                rust_decimal::Decimal::from_str(s).map_err(|_| {
                    DataError("Decimal exceeds native range; use schema exact_decimal text".into())
                })?;
                format!("decimal({})", typst_string(s))
            }
            Self::Date(s) => {
                let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map_err(|_| DataError("Invalid civil date".into()))?;
                use chrono::Datelike;
                format!(
                    "datetime(year: {}, month: {}, day: {})",
                    d.year(),
                    d.month(),
                    d.day()
                )
            }
            Self::Time(s) => {
                let t = NaiveTime::parse_from_str(s, "%H:%M:%S")
                    .map_err(|_| DataError("Invalid civil time".into()))?;
                use chrono::Timelike;
                format!(
                    "datetime(hour: {}, minute: {}, second: {})",
                    t.hour(),
                    t.minute(),
                    t.second()
                )
            }
            Self::LocalDatetime(s) => {
                let d = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .map_err(|_| DataError("Invalid local datetime".into()))?;
                use chrono::{Datelike, Timelike};
                format!(
                    "datetime(year: {}, month: {}, day: {}, hour: {}, minute: {}, second: {})",
                    d.year(),
                    d.month(),
                    d.day(),
                    d.hour(),
                    d.minute(),
                    d.second()
                )
            }
            Self::Duration(s) => {
                let v: i64 = s
                    .parse()
                    .map_err(|_| DataError("Invalid integer seconds".into()))?;
                format!("duration(seconds: {v})")
            }
            Self::Array(a) => {
                if a.is_empty() {
                    "()".into()
                } else {
                    format!(
                        "({},)",
                        a.iter()
                            .map(Self::to_typst_inner)
                            .collect::<Result<Vec<_>>>()?
                            .join(", ")
                    )
                }
            }
            Self::Record(a) => {
                let mut seen = HashSet::new();
                let entries = a
                    .iter()
                    .map(|v| {
                        if !seen.insert(&v.key) {
                            return Err(DataError("Duplicate record member".into()));
                        }
                        Ok(format!(
                            "{}: {}",
                            typst_string(&v.key),
                            v.value.to_typst_inner()?
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                if entries.is_empty() {
                    "(:)".into()
                } else {
                    format!("({})", entries.join(", "))
                }
            }
        })
    }
}
pub(crate) fn check_native_budget(value: &NativeValue) -> Result<()> {
    let mut todo = vec![(value, 0usize)];
    let mut nodes = 0usize;
    let mut bytes = 0usize;
    while let Some((value, depth)) = todo.pop() {
        nodes += 1;
        if nodes > NATIVE_MAX_NODES || depth > NATIVE_MAX_DEPTH {
            return Err(DataError("Native value traversal budget exceeded".into()));
        }
        match value {
            NativeValue::Array(values) => {
                todo.extend(values.iter().map(|value| (value, depth + 1)))
            }
            NativeValue::Record(values) => {
                for member in values {
                    bytes = bytes.saturating_add(member.key.len());
                    todo.push((&member.value, depth + 1));
                }
            }
            NativeValue::Text(text)
            | NativeValue::Int(text)
            | NativeValue::Float(text)
            | NativeValue::Decimal(text)
            | NativeValue::Date(text)
            | NativeValue::Time(text)
            | NativeValue::LocalDatetime(text)
            | NativeValue::Duration(text) => bytes = bytes.saturating_add(text.len()),
            _ => {}
        }
        if bytes > NATIVE_MAX_BYTES || todo.len() > NATIVE_MAX_NODES {
            return Err(DataError("Native value byte/node budget exceeded".into()));
        }
    }
    Ok(())
}
pub fn typst_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
/// Decode one native expression using the official parser. Standard constructor
/// binding must be certified by the enclosing declaration before using this.
pub fn decode_native_expression(expression: &str) -> Result<NativeValue> {
    if expression.len() > NATIVE_MAX_BYTES {
        return Err(DataError("Native expression byte budget exceeded".into()));
    }
    let source = Source::detached(format!("#metadata(({expression},))"));
    if !source.root().errors_and_warnings().0.is_empty() {
        return Err(DataError("Invalid Typst syntax".into()));
    }
    let call = source
        .root()
        .children()
        .find_map(|n| n.cast::<ast::FuncCall>())
        .ok_or_else(|| DataError("Missing expression".into()))?;
    let first = call
        .args()
        .items()
        .next()
        .and_then(|a| match a {
            ast::Arg::Pos(e) => e.to_untyped().cast::<ast::Array>(),
            _ => None,
        })
        .and_then(|a| a.items().next());
    let Some(ast::ArrayItem::Pos(e)) = first else {
        return Err(DataError("Expression cannot be decoded".into()));
    };
    let decoded = decode_node(e.to_untyped(), &source);
    decoded.value.ok_or_else(|| {
        DataError(
            decoded
                .reason
                .unwrap_or_else(|| "Computed or unsupported native expression".into()),
        )
    })
}
pub(crate) fn decode_node(node: &SyntaxNode, source: &Source) -> NativeNode {
    let mut remaining = NATIVE_MAX_NODES;
    decode_node_at(node, source, 0, &mut remaining)
}
fn decode_node_at(
    node: &SyntaxNode,
    source: &Source,
    depth: usize,
    remaining: &mut usize,
) -> NativeNode {
    let range = source.find(node.span()).map(|n| n.range()).unwrap_or(0..0);
    let mut out = NativeNode {
        kind: node.kind(),
        value: None,
        range: ByteRange {
            from: range.start,
            to: range.end,
        },
        children: vec![],
        reason: None,
        commas: node
            .children()
            .filter(|n| n.kind() == SyntaxKind::Comma)
            .filter_map(|n| source.find(n.span()).map(|n| n.range()))
            .map(|r| ByteRange {
                from: r.start,
                to: r.end,
            })
            .collect(),
        frame: None,
        entries: vec![],
        empty_colon: node
            .children()
            .find(|n| n.kind() == SyntaxKind::Colon)
            .and_then(|n| source.find(n.span()))
            .map(|n| ByteRange {
                from: n.range().start,
                to: n.range().end,
            }),
    };
    if depth > NATIVE_MAX_DEPTH || *remaining == 0 {
        out.reason = Some("native-decoder traversal budget exceeded".into());
        return out;
    }
    *remaining -= 1;
    let value = (|| -> Option<NativeValue> {
        Some(match node.kind() {
            SyntaxKind::None => NativeValue::None,
            SyntaxKind::Str => NativeValue::Text(node.cast::<ast::Str>()?.get().to_string()),
            SyntaxKind::Bool => NativeValue::Bool(node.cast::<ast::Bool>()?.get()),
            SyntaxKind::Int => NativeValue::Int(node.cast::<ast::Int>()?.get().to_string()),
            SyntaxKind::Float => {
                let v = node.cast::<ast::Float>()?.get();
                if !v.is_finite() {
                    return None;
                }
                NativeValue::Float(v.to_string())
            }
            SyntaxKind::Parenthesized => {
                let inner = decode_node_at(
                    node.cast::<ast::Parenthesized>()?.expr().to_untyped(),
                    source,
                    depth + 1,
                    remaining,
                );
                if inner.value.is_none() {
                    out.reason = inner.reason.clone();
                }
                out.children = inner.children.clone();
                out.commas = inner.commas.clone();
                out.entries = inner.entries.clone();
                out.empty_colon = inner.empty_colon.clone();
                let value = inner.value.clone()?;
                out.frame = Some(Box::new(inner));
                value
            }
            SyntaxKind::Unary => {
                let u = node.cast::<ast::Unary>()?;
                let child = decode_node_at(u.expr().to_untyped(), source, depth + 1, remaining);
                if child.value.is_none() {
                    out.reason = child.reason.clone();
                }
                let v = child.value?;
                match (u.op(), v) {
                    (ast::UnOp::Neg, NativeValue::Int(s)) => {
                        NativeValue::Int(s.parse::<i64>().ok()?.checked_neg()?.to_string())
                    }
                    (ast::UnOp::Pos, v @ NativeValue::Int(_)) => v,
                    (ast::UnOp::Neg, NativeValue::Float(s)) => {
                        NativeValue::Float((-s.parse::<f64>().ok()?).to_string())
                    }
                    (ast::UnOp::Pos, v @ NativeValue::Float(_)) => v,
                    _ => return None,
                }
            }
            SyntaxKind::Array => {
                let mut values = vec![];
                for (i, item) in node.cast::<ast::Array>()?.items().enumerate() {
                    let ast::ArrayItem::Pos(e) = item else {
                        return None;
                    };
                    let child = decode_node_at(e.to_untyped(), source, depth + 1, remaining);
                    if child.value.is_none() {
                        out.reason = child.reason.clone();
                    }
                    let v = child.value.clone();
                    out.children.push((ValuePath::Index { index: i }, child));
                    values.push(v?);
                }
                NativeValue::Array(values)
            }
            SyntaxKind::Dict => {
                let mut values = vec![];
                let mut seen = HashSet::new();
                for item in node.cast::<ast::Dict>()?.items() {
                    let (key, e, entry) = match item {
                        ast::DictItem::Named(n) => {
                            (n.name().get().to_string(), n.expr(), n.to_untyped())
                        }
                        ast::DictItem::Keyed(n) => (
                            n.key().to_untyped().cast::<ast::Str>()?.get().to_string(),
                            n.expr(),
                            n.to_untyped(),
                        ),
                        _ => return None,
                    };
                    if !seen.insert(key.clone()) {
                        out.reason = Some("duplicate-record-key".into());
                        return None;
                    }
                    let child = decode_node_at(e.to_untyped(), source, depth + 1, remaining);
                    if child.value.is_none() {
                        out.reason = child.reason.clone();
                    }
                    let range = source.find(entry.span())?.range();
                    out.entries.push((
                        key.clone(),
                        ByteRange {
                            from: range.start,
                            to: range.end,
                        },
                    ));
                    let v = child.value.clone();
                    out.children
                        .push((ValuePath::Member { key: key.clone() }, child));
                    values.push(NativeMember { key, value: v? });
                }
                NativeValue::Record(values)
            }
            SyntaxKind::FuncCall => {
                constructor(node.cast::<ast::FuncCall>()?, source, depth, remaining)?
            }
            _ => return None,
        })
    })();
    out.value = value;
    if out.value.is_none() && out.reason.is_none() {
        out.reason = Some("computed-expression".into());
    }
    out
}
fn constructor(
    call: ast::FuncCall<'_>,
    source: &Source,
    depth: usize,
    remaining: &mut usize,
) -> Option<NativeValue> {
    let name = call.callee().to_untyped().cast::<ast::Ident>()?.get();
    let args: Vec<_> = call.args().items().collect();
    if matches!(name.as_str(), "int" | "decimal") {
        let [ast::Arg::Pos(e)] = args.as_slice() else {
            return None;
        };
        let s = e.to_untyped().cast::<ast::Str>()?.get().to_string();
        return match name.as_str() {
            "int" => Some(NativeValue::Int(s.parse::<i64>().ok()?.to_string())),
            _ => {
                rust_decimal::Decimal::from_str(&s).ok()?;
                Some(NativeValue::Decimal(s))
            }
        };
    }
    let mut named = std::collections::BTreeMap::new();
    for a in args {
        let ast::Arg::Named(n) = a else {
            return None;
        };
        if named
            .insert(
                n.name().get().to_string(),
                decode_node_at(n.expr().to_untyped(), source, depth + 1, remaining)
                    .value?
                    .integer()?,
            )
            .is_some()
        {
            return None;
        }
    }
    if name == "duration" {
        if named.keys().any(|k| {
            !matches!(
                k.as_str(),
                "seconds" | "minutes" | "hours" | "days" | "weeks"
            )
        }) {
            return None;
        }
        let mut total = 0i64;
        for (k, n) in named {
            let factor = match k.as_str() {
                "minutes" => 60,
                "hours" => 3600,
                "days" => 86400,
                "weeks" => 604800,
                _ => 1,
            };
            total = total.checked_add(n.checked_mul(factor)?)?;
        }
        return Some(NativeValue::Duration(total.to_string()));
    }
    if name != "datetime"
        || named.keys().any(|k| {
            !matches!(
                k.as_str(),
                "year" | "month" | "day" | "hour" | "minute" | "second"
            )
        })
    {
        return None;
    }
    let date =
        if named.contains_key("year") || named.contains_key("month") || named.contains_key("day") {
            Some(NaiveDate::from_ymd_opt(
                i32::try_from(*named.get("year")?).ok()?,
                u32::try_from(*named.get("month")?).ok()?,
                u32::try_from(*named.get("day")?).ok()?,
            )?)
        } else {
            None
        };
    let time = if named.contains_key("hour")
        || named.contains_key("minute")
        || named.contains_key("second")
    {
        Some(NaiveTime::from_hms_opt(
            u32::try_from(*named.get("hour")?).ok()?,
            u32::try_from(*named.get("minute")?).ok()?,
            u32::try_from(*named.get("second")?).ok()?,
        )?)
    } else {
        None
    };
    match (date, time) {
        (Some(d), Some(t)) => Some(NativeValue::LocalDatetime(
            d.and_time(t).format("%Y-%m-%dT%H:%M:%S").to_string(),
        )),
        (Some(d), None) => Some(NativeValue::Date(d.format("%Y-%m-%d").to_string())),
        (None, Some(t)) => Some(NativeValue::Time(t.format("%H:%M:%S").to_string())),
        _ => None,
    }
}
/// Arbitrary-precision decimal codec; no conversion to binary64 occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactDecimal {
    pub coefficient: BigInt,
    pub scale: u32,
}
impl ExactDecimal {
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() || s.len() > 4096 {
            return Err(DataError("Invalid exact decimal".into()));
        }
        let (negative, rest) = s.strip_prefix('-').map_or((false, s), |r| (true, r));
        let parts: Vec<_> = rest.split('.').collect();
        if parts.len() > 2
            || parts[0].is_empty()
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(DataError("Invalid exact decimal".into()));
        }
        let scale = parts.get(1).map_or(0, |p| p.len()) as u32;
        let mut coefficient =
            BigInt::from_str(&parts.concat()).map_err(|_| DataError("Invalid decimal".into()))?;
        if negative {
            coefficient = -coefficient;
        }
        Ok(Self { coefficient, scale })
    }
    pub fn compare(&self, other: &Self) -> Ordering {
        let scale = self.scale.max(other.scale);
        (&self.coefficient * BigInt::from(10).pow(scale - self.scale))
            .cmp(&(&other.coefficient * BigInt::from(10).pow(scale - other.scale)))
    }
    pub fn add(&self, other: &Self) -> Self {
        let scale = self.scale.max(other.scale);
        Self {
            coefficient: &self.coefficient * BigInt::from(10).pow(scale - self.scale)
                + &other.coefficient * BigInt::from(10).pow(scale - other.scale),
            scale,
        }
    }
    pub fn spelling(&self) -> String {
        let s = self.coefficient.to_string();
        let (sign, digits) = s.strip_prefix('-').map_or(("", s.as_str()), |r| ("-", r));
        if self.scale == 0 {
            return s;
        }
        let padded = format!("{:0>width$}", digits, width = self.scale as usize + 1);
        let at = padded.len() - self.scale as usize;
        format!("{sign}{}.{}", &padded[..at], &padded[at..])
    }
}
