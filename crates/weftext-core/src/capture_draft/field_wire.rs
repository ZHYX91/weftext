//! Validate one unfinished native field without decoding its lone UTF-16
//! author strings into Rust String or interpreting a Schema/default/template.
use super::*;
use serde::de::{Deserialize, Error, MapAccess, Visitor};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

struct Members(BTreeMap<String, Box<RawValue>>);

pub(super) fn value_path<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<weftext_data::ValuePath>, D::Error> {
    #[derive(serde::Deserialize)]
    #[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
    enum Step {
        Member { key: String },
        Index { index: usize },
    }
    let steps = Vec::<Step>::deserialize(deserializer)?;
    if steps.len() > 32 {
        return Err(D::Error::custom("Capture path exceeds depth budget"));
    }
    Ok(steps
        .into_iter()
        .map(|step| match step {
            Step::Member { key } => weftext_data::ValuePath::Member { key },
            Step::Index { index } => weftext_data::ValuePath::Index { index },
        })
        .collect())
}

pub(super) fn valid_native<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<weftext_data::NativeValue, D::Error> {
    let raw = Box::<RawValue>::deserialize(deserializer)?;
    let mut nodes = 0;
    let mut bytes = 0;
    let mut context = Context {
        field: "",
        raw: &[],
        nodes: &mut nodes,
        bytes: &mut bytes,
        incomplete: false,
    };
    native(&raw, &mut Vec::new(), 0, &mut context).map_err(D::Error::custom)?;
    serde_json::from_str(raw.get()).map_err(D::Error::custom)
}
impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = Members;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a closed native value object")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Members, M::Error> {
                let mut fields = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    if fields.contains_key(&key) {
                        return Err(M::Error::custom("duplicate native member"));
                    }
                    if fields.len() == 2 {
                        return Err(M::Error::custom("unknown native member"));
                    }
                    fields.insert(key, map.next_value::<Box<RawValue>>()?);
                }
                Ok(Members(fields))
            }
        }
        deserializer.deserialize_map(Fields)
    }
}
fn fields(raw: &str, expected: &[&str]) -> Result<Members> {
    let members: Members = serde_json::from_str(raw)
        .map_err(|_| invalid("Incomplete field contains duplicate or invalid native members"))?;
    if members.0.len() != expected.len() || expected.iter().any(|key| !members.0.contains_key(*key))
    {
        return Err(invalid(
            "Incomplete field contains unknown or missing native members",
        ));
    }
    Ok(members)
}
fn string_units(raw: &RawValue) -> Result<Vec<u16>> {
    let raw = raw.get();
    if !raw.starts_with('"') || !raw.ends_with('"') {
        return Err(invalid("Native scalar input must be a string"));
    }
    // RawValue already checked JSON grammar. Decode escapes to code units so
    // an unpaired escaped surrogate is retained rather than replaced/rejected.
    let mut chars = raw[1..raw.len() - 1].chars();
    let mut result = Vec::new();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            let mut units = [0; 2];
            result.extend_from_slice(ch.encode_utf16(&mut units));
            continue;
        }
        let escaped = chars
            .next()
            .ok_or_else(|| invalid("Incomplete JSON escape"))?;
        if escaped == 'u' {
            let mut unit = 0u16;
            for _ in 0..4 {
                unit = (unit << 4)
                    | chars
                        .next()
                        .and_then(|ch| ch.to_digit(16))
                        .ok_or_else(|| invalid("Invalid UTF-16 escape"))?
                        as u16;
            }
            result.push(unit);
        } else {
            result.push(match escaped {
                '"' => 34,
                '\\' => 92,
                '/' => 47,
                'b' => 8,
                'f' => 12,
                'n' => 10,
                'r' => 13,
                't' => 9,
                _ => return Err(invalid("Invalid JSON string escape")),
            });
        }
    }
    Ok(result)
}
struct Context<'a> {
    field: &'a str,
    raw: &'a [CaptureRawInput],
    nodes: &'a mut usize,
    bytes: &'a mut usize,
    incomplete: bool,
}
fn native(
    raw: &RawValue,
    path: &mut Vec<weftext_data::ValuePath>,
    depth: usize,
    context: &mut Context<'_>,
) -> Result<()> {
    *context.nodes += 1;
    if depth > 32 || *context.nodes > 100_000 {
        return Err(budget());
    }
    let initial: Members = serde_json::from_str(raw.get())
        .map_err(|_| invalid("Incomplete field is not a closed native value"))?;
    let kind: String = serde_json::from_str(
        initial
            .0
            .get("kind")
            .ok_or_else(|| invalid("Native field kind is missing"))?
            .get(),
    )
    .map_err(|_| invalid("Native field kind must be an identifier"))?;
    if kind == "none" {
        fields(raw.get(), &["kind"])?;
        return Ok(());
    }
    let members = fields(raw.get(), &["kind", "value"])?;
    let value = &members.0["value"];
    match kind.as_str() {
        "bool" => {
            serde_json::from_str::<bool>(value.get())
                .map_err(|_| invalid("Native boolean is invalid"))?;
        }
        "text" | "int" | "float" | "decimal" | "date" | "time" | "localDatetime" | "duration" => {
            let units = string_units(value)?;
            add_input_bytes(context.bytes, units.len() * 2)?;
            if String::from_utf16(&units).is_err() {
                if !context.raw.iter().any(|raw| {
                    raw.field_id == context.field
                        && matches!(raw.binding, CaptureRawBinding::Exact)
                        && raw.value_path.as_slice() == path.as_slice()
                        && raw.text.utf16 == units
                }) {
                    return Err(invalid(
                        "Incomplete native scalar has no exact original control address",
                    ));
                }
                context.incomplete = true;
            }
        }
        "array" => {
            let items: Vec<Box<RawValue>> =
                serde_json::from_str(value.get()).map_err(|_| invalid("Native list is invalid"))?;
            for (index, item) in items.iter().enumerate() {
                path.push(weftext_data::ValuePath::Index { index });
                native(item, path, depth + 1, context)?;
                path.pop();
            }
        }
        "record" => {
            let items: Vec<Box<RawValue>> = serde_json::from_str(value.get())
                .map_err(|_| invalid("Native record is invalid"))?;
            let mut keys = BTreeSet::new();
            for item in items {
                let entry = fields(item.get(), &["key", "value"])?;
                let units = string_units(&entry.0["key"])?;
                let key = String::from_utf16(&units).map_err(|_| invalid("Native member identifiers must be complete; retain unfinished key input separately"))?;
                if !keys.insert(key.clone()) {
                    return Err(invalid("Native record keys are duplicated"));
                }
                add_input_bytes(context.bytes, key.len())?;
                path.push(weftext_data::ValuePath::Member { key });
                native(&entry.0["value"], path, depth + 1, context)?;
                path.pop();
            }
        }
        _ => return Err(invalid("Incomplete field has an unknown native kind")),
    }
    Ok(())
}
pub(super) fn validate(
    wire: &CaptureText,
    field: &str,
    raw: &[CaptureRawInput],
    nodes: &mut usize,
    bytes: &mut usize,
) -> Result<()> {
    text_valid(wire)?;
    add_input_bytes(bytes, wire.utf16.len() * 2)?;
    let source = wire.utf8()?;
    let value: Box<RawValue> = serde_json::from_str(&source)
        .map_err(|_| invalid("Incomplete field wire is invalid JSON"))?;
    let mut context = Context {
        field,
        raw,
        nodes,
        bytes,
        incomplete: false,
    };
    native(&value, &mut Vec::new(), 0, &mut context)?;
    if !context.incomplete {
        return Err(invalid("A complete native field must use Valid state"));
    }
    Ok(())
}
