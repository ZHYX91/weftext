//! CLI-owned pure input and create-only output. Never writes author source files.
use crate::client_protocol::{BusinessCall, CallKind, MAX_WIRE_BYTES, WIRE_VERSION};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

pub(crate) use crate::client_protocol::MAX_SOURCE_BYTES;
pub(crate) const MAX_UPLOAD_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Target {
    Local {
        profile_identity: String,
    },
    Remote {
        origin: String,
        actor: String,
        deployment: String,
    },
}

/// An immutable exact input record, not a parallel operation/receipt ledger.
/// It contains no credential and is never updated from a response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FrozenRequest {
    pub version: u32,
    pub target: Target,
    pub kind: CallKind,
    pub request_id: String,
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// A denial boundary for delivery files, never an authorization capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_root: Option<PathBuf>,
    pub wire_sha256: String,
    pub wire_base64: String,
}

impl FrozenRequest {
    pub fn new(target: Target, call: BusinessCall) -> Result<Self, &'static str> {
        let wire = call.encode()?;
        let operation_id = identifier(call.kind, &call.body).map(str::to_owned);
        if let Some(id) = &operation_id {
            valid_id(id)?;
        }
        Ok(Self {
            version: WIRE_VERSION,
            target,
            kind: call.kind,
            request_id: uuid::Uuid::new_v4().to_string(),
            operation_id,
            resource: None,
            author_root: None,
            wire_sha256: digest(&wire),
            wire_base64: base64::engine::general_purpose::STANDARD.encode(wire),
        })
    }

    pub fn wire(&self) -> Result<Vec<u8>, &'static str> {
        if self.version != WIRE_VERSION {
            return Err("requestVersion");
        }
        valid_id(&self.request_id)?;
        if self.wire_base64.len() > MAX_WIRE_BYTES * 4 / 3 + 4 {
            return Err("inputBudget");
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.wire_base64)
            .map_err(|_| "requestEncoding")?;
        if bytes.len() > MAX_WIRE_BYTES || digest(&bytes) != self.wire_sha256 {
            return Err("requestDigest");
        }
        let body = parse_json(&bytes).map_err(|_| "requestJson")?;
        if !body.is_object() {
            return Err("requestShape");
        }
        let actual = identifier(self.kind, &body);
        if actual != self.operation_id.as_deref() {
            return Err("requestIdentity");
        }
        if let Some(id) = &self.operation_id {
            valid_id(id)?;
        }
        reject_secrets(&body)?;
        Ok(bytes)
    }

    pub fn body(&self) -> Result<Value, &'static str> {
        parse_json(&self.wire()?).map_err(|_| "requestJson")
    }

    pub fn persist(
        &self,
        path: &Path,
        author_root: Option<&Path>,
        private_root: Option<&Path>,
    ) -> io::Result<PathBuf> {
        self.wire().map_err(io::Error::other)?;
        let path = artifact_path(path, &[author_root, private_root])?;
        if !path
            .file_name()
            .and_then(|v| v.to_str())
            .is_some_and(|v| v.ends_with(".weftext-request.json"))
        {
            return Err(io::Error::other("requestFileSuffix"));
        }
        let mut file = private_create_new(&path)?;
        // Keep any partial file as evidence if write/flush fails. Never erase it.
        serde_json::to_writer_pretty(&mut file, self).map_err(io::Error::other)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(path)
    }
}

/// Every CLI delivery stays outside author and Core trees. Resolve the actual
/// parent (including a junction) before create_new; do not overwrite originals.
pub(crate) fn artifact_path(path: &Path, roots: &[Option<&Path>]) -> io::Result<PathBuf> {
    if path == Path::new("-") {
        return Err(io::Error::other("outputPath"));
    }
    let path = std::path::absolute(path)?;
    if path.to_str().is_none() {
        return Err(io::Error::other("pathUnicode"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("outputParent"))?
        .canonicalize()?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("outputPath"))?;
    #[cfg(windows)]
    validate_windows_leaf(
        name.to_str()
            .ok_or_else(|| io::Error::other("pathUnicode"))?,
    )?;
    let resolved = parent.join(name);
    for root in roots.iter().flatten() {
        let root = std::path::absolute(root)?;
        let root = if root.exists() {
            root.canonicalize()?
        } else {
            root
        };
        if parent.starts_with(&root) {
            return Err(io::Error::other("outputInsideManagedTree"));
        }
    }
    Ok(resolved)
}
#[cfg(windows)]
fn validate_windows_leaf(name: &str) -> io::Result<()> {
    // Ordinary output files only, never DOS devices, console handles, streams
    // or Win32 trailing-dot/space aliases. Extensions do not unreserve devices.
    // https://learn.microsoft.com/windows/win32/fileio/naming-a-file
    // https://learn.microsoft.com/windows/console/console-handles
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    let numbered_device = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        });
    if name.is_empty()
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" | "CLOCK$"
        )
        || numbered_device
    {
        return Err(io::Error::other("outputPath"));
    }
    Ok(())
}
pub(crate) fn distinct_outputs(paths: &[&Path]) -> io::Result<()> {
    let mut names = std::collections::BTreeSet::new();
    for path in paths {
        let path = artifact_path(path, &[])?;
        #[cfg(windows)]
        let key = path.as_os_str().to_string_lossy().to_lowercase();
        #[cfg(not(windows))]
        let key = path.as_os_str().to_owned();
        if !names.insert(key) {
            return Err(io::Error::other("outputAlias"));
        }
        if path.try_exists()? {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "outputExists"));
        }
    }
    Ok(())
}

fn identifier(kind: CallKind, body: &Value) -> Option<&str> {
    kind.identifier_field()
        .and_then(|field| body[field].as_str())
        .or_else(|| {
            matches!(kind, CallKind::ExportConfirm | CallKind::ExportDownload)
                .then(|| body["id"].as_str())
                .flatten()
        })
}

pub(crate) fn valid_id(value: &str) -> Result<(), &'static str> {
    let id = uuid::Uuid::parse_str(value).map_err(|_| "invalidOperationId")?;
    if id.get_version_num() != 4 || id.to_string() != value {
        return Err("invalidOperationId");
    }
    Ok(())
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn read_bounded(mut input: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    input
        .by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::other("inputBudget"));
    }
    Ok(bytes)
}

pub(crate) fn read_input(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    if path == Path::new("-") {
        read_bounded(io::stdin().lock(), limit)
    } else {
        read_bounded(File::open(path)?, limit)
    }
}

pub(crate) fn read_source(path: &Path) -> io::Result<String> {
    String::from_utf8(read_input(path, MAX_SOURCE_BYTES)?)
        .map_err(|_| io::Error::other("inputUtf8"))
}

pub(crate) fn read_json(path: &Path) -> io::Result<Value> {
    let bytes = read_input(path, MAX_WIRE_BYTES)?;
    let value = parse_json(&bytes)?;
    reject_secrets(&value).map_err(io::Error::other)?;
    Ok(value)
}

pub(crate) fn read_request(path: &Path) -> io::Result<FrozenRequest> {
    let bytes = read_input(path, MAX_WIRE_BYTES * 4 / 3 + 4096)?;
    let request: FrozenRequest =
        serde_json::from_slice(&bytes).map_err(|_| io::Error::other("requestJson"))?;
    request.wire().map_err(io::Error::other)?;
    Ok(request)
}

fn private_create_new(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = private_create_new(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Reject duplicate keys before constructing Value. Re-encoding cannot erase a
/// duplicate that the production handler would otherwise have rejected.
pub(crate) fn parse_json(bytes: &[u8]) -> io::Result<Value> {
    struct Unique(Value);
    impl<'de> Deserialize<'de> for Unique {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Unique;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("JSON with unique object keys")
                }
                fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                    Ok(Unique(Value::Bool(v)))
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                    Ok(Unique(Value::Number(v.into())))
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                    Ok(Unique(Value::Number(v.into())))
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                    serde_json::Number::from_f64(v)
                        .map(|v| Unique(Value::Number(v)))
                        .ok_or_else(|| E::custom("non-finite number"))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                    Ok(Unique(Value::String(v.into())))
                }
                fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Unique, E> {
                    Ok(Unique(Value::String(v)))
                }
                fn visit_none<E: serde::de::Error>(self) -> Result<Unique, E> {
                    Ok(Unique(Value::Null))
                }
                fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                    Ok(Unique(Value::Null))
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut seq: A,
                ) -> Result<Unique, A::Error> {
                    let mut values = Vec::new();
                    while let Some(Unique(v)) = seq.next_element()? {
                        values.push(v);
                    }
                    Ok(Unique(Value::Array(values)))
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> Result<Unique, A::Error> {
                    let mut values = serde_json::Map::new();
                    while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                        if values.insert(key, value).is_some() {
                            return Err(serde::de::Error::custom("duplicate object key"));
                        }
                    }
                    Ok(Unique(Value::Object(values)))
                }
            }
            deserializer.deserialize_any(Visitor)
        }
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let Unique(value) =
        Unique::deserialize(&mut deserializer).map_err(|_| io::Error::other("inputJson"))?;
    deserializer
        .end()
        .map_err(|_| io::Error::other("inputJson"))?;
    Ok(value)
}

fn reject_secrets(value: &Value) -> Result<(), &'static str> {
    // These are envelope credentials. Author source and nested native values
    // remain opaque: a legitimate authored field named "token" is not a token.
    if let Value::Object(fields) = value {
        for key in fields.keys() {
            if matches!(
                key.to_ascii_lowercase().as_str(),
                "token" | "password" | "authorization" | "credential" | "credentials"
            ) {
                return Err("credentialInInput");
            }
        }
        if fields
            .get("connection")
            .and_then(Value::as_object)
            .is_some_and(|c| c.contains_key("token"))
        {
            return Err("credentialInInput");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn windows_outputs_are_plain_files_not_devices_streams_or_aliased_names() {
        for name in [
            "NUL.weftext-request.json",
            "con.json",
            "COM1.log",
            "COM¹.log",
            "LPT³.txt",
            "CONIN$",
            "CONOUT$",
            "file:stream",
            "result.json.",
            "result.json ",
            "NUL .json",
        ] {
            assert!(validate_windows_leaf(name).is_err(), "{name}");
        }
        for name in [
            "result.weftext-request.json",
            "中文🙂.json",
            "my file.json",
            ".working.json",
        ] {
            assert!(validate_windows_leaf(name).is_ok(), "{name}");
        }
    }
    #[test]
    fn source_bytes_keep_crlf_and_non_bmp_and_reject_truncation() {
        let bytes = "中文🙂\r\n".as_bytes();
        assert_eq!(read_bounded(bytes, bytes.len()).unwrap(), bytes);
        assert!(read_bounded(bytes, bytes.len() - 1).is_err());
        assert!(String::from_utf8(read_bounded(&b"\xff"[..], 10).unwrap()).is_err());
    }
    #[test]
    fn request_is_exact_input_not_a_mutable_receipt() {
        let id = uuid::Uuid::new_v4().to_string();
        let mut request = FrozenRequest::new(
            Target::Local {
                profile_identity: "owned-physical-profile".into(),
            },
            BusinessCall {
                kind: CallKind::Save,
                body: serde_json::json!({"operationId":id,"source":"作者🙂\r\n"}),
            },
        )
        .unwrap();
        let before = request.wire().unwrap();
        request.wire_base64 = base64::engine::general_purpose::STANDARD.encode(b"{}");
        assert_eq!(request.wire().unwrap_err(), "requestDigest");
        assert!(std::str::from_utf8(&before).unwrap().contains("作者🙂"));
    }
    #[test]
    fn input_records_cannot_include_credentials() {
        let body = serde_json::json!({"connection":{"token":"secret"}});
        assert!(reject_secrets(&body).is_err());
        assert!(reject_secrets(&serde_json::json!({"source":"token: author words"})).is_ok());
        assert!(
            reject_secrets(
                &serde_json::json!({"fields":{"token":{"kind":"text","value":"author value"}}})
            )
            .is_ok()
        );
    }
    #[test]
    fn create_only_output_never_overwrites_an_original() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("original.txt");
        write_new(&path, b"original").unwrap();
        assert!(write_new(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"original");
    }
    #[test]
    fn duplicate_json_and_invalid_unicode_are_not_normalized_into_valid_requests() {
        for bytes in [
            &br#"{"fields":{"x":1,"x":2}}"#[..],
            &br#"{"source":"\ud800"}"#[..],
            &br#"{} {}"#[..],
        ] {
            assert!(parse_json(bytes).is_err());
        }
        assert_eq!(
            parse_json(br#"{"exact":"9007199254740993"}"#).unwrap()["exact"],
            "9007199254740993"
        );
    }
    #[test]
    fn every_delivery_rejects_known_managed_roots_and_aliases_before_effects() {
        let temp = tempfile::tempdir().unwrap();
        let author = temp.path().join("author");
        let private = temp.path().join("private");
        let output = temp.path().join("out");
        for path in [&author, &private, &output] {
            std::fs::create_dir(path).unwrap();
        }
        assert!(artifact_path(&author.join("new.typ"), &[Some(&author), Some(&private)]).is_err());
        assert!(
            artifact_path(&private.join("new.json"), &[Some(&author), Some(&private)]).is_err()
        );
        let a = output.join("a.json");
        let b = output.join("..").join("out").join("a.json");
        assert!(distinct_outputs(&[&a, &b]).is_err());
        assert!(!a.exists());
        #[cfg(windows)]
        {
            let upper = output.join("A.JSON");
            assert!(distinct_outputs(&[&a, &upper]).is_err());
        }
        assert!(artifact_path(&output.join("new.typ"), &[Some(&author), Some(&private)]).is_ok());
    }
    #[test]
    fn retained_input_is_create_new_flushed_and_does_not_become_a_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let request=FrozenRequest::new(Target::Local{profile_identity:"profile".into()},BusinessCall{kind:CallKind::Save,body:serde_json::json!({"operationId":uuid::Uuid::new_v4().to_string(),"source":"作者\r\n"})}).unwrap();
        let path = temp.path().join("save.weftext-request.json");
        let saved = request.persist(&path, None, None).unwrap();
        let bytes = std::fs::read(&saved).unwrap();
        assert!(request.persist(&path, None, None).is_err());
        assert_eq!(
            read_request(&saved).unwrap().wire().unwrap(),
            request.wire().unwrap()
        );
        assert_eq!(std::fs::read(&saved).unwrap(), bytes);
        let value = parse_json(&bytes).unwrap();
        assert!(value.get("receipt").is_none());
        assert!(value.get("status").is_none());
        assert!(value.get("credentials").is_none());
    }
}
