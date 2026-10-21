use serde::Deserialize;
#[path = "src/message_template.rs"]
mod message_template;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::PathBuf,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Catalog {
    version: u32,
    default_locale: String,
    locales: Vec<String>,
    #[serde(deserialize_with = "unique_map")]
    messages: BTreeMap<String, Message>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Message {
    #[serde(rename = "zh-CN")]
    zh: String,
    #[serde(rename = "en-US")]
    en: String,
    #[serde(deserialize_with = "unique_map")]
    params: BTreeMap<String, Param>,
}

fn unique_map<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, T>, D::Error> {
    struct UniqueMap<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for UniqueMap<T> {
        type Value = BTreeMap<String, T>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an object with unique catalog members")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some(key) = map.next_key::<String>()? {
                if values.contains_key(&key) {
                    return Err(serde::de::Error::custom("Duplicate catalog member"));
                }
                values.insert(key, map.next_value()?);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(UniqueMap(std::marker::PhantomData))
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Param {
    Text,
    Integer,
}

fn identifiers(key: &str) -> (String, String) {
    let mut variant = String::new();
    let mut field = String::new();
    for word in key.split('.') {
        assert!(!word.is_empty() && word.chars().next().unwrap().is_ascii_alphabetic());
        for (index, character) in word.chars().enumerate() {
            assert!(character.is_ascii_alphanumeric());
            variant.push(if index == 0 {
                character.to_ascii_uppercase()
            } else {
                character
            });
            if character.is_ascii_uppercase() && !field.is_empty() {
                field.push('_');
            }
            field.push(character.to_ascii_lowercase());
        }
    }
    if [
        "type", "self", "move", "ref", "match", "loop", "where", "in", "as", "use", "mod", "pub",
        "crate", "super", "enum", "struct", "fn", "impl", "const", "static", "let", "mut", "async",
        "await", "dyn", "trait", "return", "break", "continue", "for", "if", "else", "while",
        "true", "false",
    ]
    .contains(&field.as_str())
    {
        field.insert_str(0, "value_");
    }
    (variant, field)
}

fn parameters(template: &str) -> BTreeSet<String> {
    message_template::placeholders(template)
        .map(|(_, name)| name.to_owned())
        .collect()
}

fn factory_name(variant: &str) -> String {
    let mut name = String::new();
    for character in variant.chars() {
        if character.is_ascii_uppercase() && !name.is_empty() {
            name.push('_');
        }
        name.push(character.to_ascii_lowercase());
    }
    name
}

fn main() {
    let source = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../assets/ui-messages.json");
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed=src/message_template.rs");
    let catalog: Catalog = serde_json::from_slice(&fs::read(source).unwrap()).unwrap();
    assert_eq!(catalog.version, 1);
    assert_eq!(catalog.default_locale, "zh-CN");
    assert_eq!(catalog.locales, ["zh-CN", "en-US"]);
    let mut variants = BTreeSet::new();
    let mut output = format!(
        "static MESSAGE_DEFINITIONS: [MessageDefinition; {}] = [\n",
        catalog.messages.len()
    );
    let mut factories = String::from("impl UiMessage {\n");
    let mut arguments = String::from("pub mod params {\n");
    let mut unique_factories = BTreeSet::new();
    let mut maximum_params = 0;
    for (index, (key, message)) in catalog.messages.into_iter().enumerate() {
        assert!(key.contains('.'));
        let (variant, _) = identifiers(&key);
        assert!(
            variants.insert(variant.clone()),
            "Duplicate UI variant {variant}"
        );
        let expected: BTreeSet<_> = message.params.keys().cloned().collect();
        assert_eq!(parameters(&message.zh), expected, "Chinese params: {key}");
        assert_eq!(parameters(&message.en), expected, "English params: {key}");
        assert!(!message.zh.is_empty() && !message.en.is_empty());
        let factory = factory_name(&variant);
        assert!(
            unique_factories.insert(factory.clone()),
            "Duplicate factory {factory}"
        );
        maximum_params = maximum_params.max(message.params.len());
        output.push_str(&format!(
            "MessageDefinition {{ key: {key:?}, zh: {:?}, en: {:?}, params: &[",
            message.zh, message.en
        ));
        let has_params = !message.params.is_empty();
        if has_params {
            arguments.push_str(&format!("pub struct {variant} {{\n"));
        }
        let mut values = Vec::new();
        let mut unique_fields = BTreeSet::new();
        for (name, param) in message.params {
            let (_, field) = identifiers(&name);
            assert!(
                unique_fields.insert(field.clone()),
                "Duplicate UI field {key}.{field}"
            );
            let kind = match param {
                Param::Text => "String",
                Param::Integer => "super::UiInteger",
            };
            let value_kind = match param {
                Param::Text => "Text",
                Param::Integer => "Integer",
            };
            arguments.push_str(&format!("pub {field}: {kind},\n"));
            output.push_str(&format!(
                "MessageParameter {{ name: {name:?}, kind: ParameterKind::{value_kind} }},"
            ));
            values.push(format!("UiParameterValue::{value_kind}(arguments.{field})"));
        }
        output.push_str("] },\n");
        if has_params {
            arguments.push_str("}\n");
        }
        let signature = if has_params {
            format!("arguments: params::{variant}")
        } else {
            String::new()
        };
        factories.push_str(&format!("pub fn {factory}({signature}) -> Self {{ Self {{ index: {index}, values: vec![{}] }} }}\n", values.join(", ")));
    }
    output.push_str("];\n");
    output.push_str(&format!(
        "const MESSAGE_MAX_PARAMS: usize = {maximum_params};\n"
    ));
    arguments.push_str("}\n");
    factories.push_str("}\n");
    output.push_str(&arguments);
    output.push_str(&factories);
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("messages.rs"),
        output,
    )
    .unwrap();
}
