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
    messages: BTreeMap<String, Message>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Message {
    #[serde(rename = "zh-CN")]
    zh: String,
    #[serde(rename = "en-US")]
    en: String,
    params: BTreeMap<String, Param>,
}

#[derive(Deserialize)]
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
    let mut output = "#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]\n#[serde(tag = \"key\", content = \"params\", deny_unknown_fields)]\npub enum UiMessage {\n".to_owned();
    let mut arms = String::new();
    let mut key_arms = String::new();
    for (key, message) in catalog.messages {
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
        output.push_str(&format!("#[serde(rename = {key:?})] {variant} {{\n"));
        let mut fields = Vec::new();
        let mut rendered = Vec::new();
        let mut unique_fields = BTreeSet::new();
        for (name, param) in message.params {
            let (_, field) = identifiers(&name);
            assert!(
                unique_fields.insert(field.clone()),
                "Duplicate UI field {key}.{field}"
            );
            let kind = match param {
                Param::Text => "String",
                Param::Integer => "UiInteger",
            };
            output.push_str(&format!("#[serde(rename = {name:?})] {field}: {kind},\n"));
            rendered.push(format!("({name:?}, {field}.to_string())"));
            fields.push(field);
        }
        output.push_str("},\n");
        let pattern = fields.join(", ");
        arms.push_str(&format!("Self::{variant} {{ {pattern} }} => render_template(match locale {{ UiLocale::ZhCn => {:?}, UiLocale::EnUs => {:?} }}, &[{}]),\n", message.zh, message.en, rendered.join(", ")));
        key_arms.push_str(&format!("Self::{variant} {{ .. }} => {key:?},\n"));
    }
    output.push_str("}\nimpl UiMessage { pub fn key(&self) -> &'static str { match self {\n");
    output.push_str(&key_arms);
    output.push_str("}} pub fn render(&self, locale: UiLocale) -> String { match self {\n");
    output.push_str(&arms);
    output.push_str("}}}\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("messages.rs"),
        output,
    )
    .unwrap();
}
