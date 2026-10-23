//! Read and validate all caller input before any effect-bearing request.
use super::{
    args::*,
    input::{self, FrozenRequest, Target},
};
use crate::client_protocol::{BusinessCall, CallKind};
use base64::Engine as _;
use serde_json::{Value, json};
use std::{
    io,
    path::{Path, PathBuf},
};

pub(crate) enum Action {
    Call(BusinessCall),
    OriginalStatus(FrozenRequest),
    Settle(FrozenRequest),
    Download {
        request: FrozenRequest,
        destination: PathBuf,
    },
}
pub(crate) struct Prepared {
    pub action: Action,
    pub record: Option<PathBuf>,
    pub expected_target: Option<Target>,
    pub author_root: Option<PathBuf>,
    pub source_output: Option<PathBuf>,
    pub resource: Option<String>,
    pub expected_artifact_digest: Option<String>,
    pub prerequisites: Vec<BusinessCall>,
    pub original_path: Option<PathBuf>,
    pub secret_source: Option<String>,
    pub wait_seconds: u64,
}

fn invalid(code: &'static str) -> io::Error {
    io::Error::other(code)
}
fn text(value: &Value, field: &str) -> io::Result<String> {
    value[field]
        .as_str()
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid("basisField"))
}
fn object(value: Value) -> io::Result<Value> {
    if value.is_object() {
        Ok(value)
    } else {
        Err(invalid("inputShape"))
    }
}

fn with_id(mut body: Value, field: &str, generate: bool) -> io::Result<Value> {
    let fields = body.as_object_mut().ok_or_else(|| invalid("inputShape"))?;
    if !fields.contains_key(field) && generate {
        fields.insert(field.into(), json!(uuid::Uuid::new_v4().to_string()));
    }
    let id = fields
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("operationIdRequired"))?;
    input::valid_id(id).map_err(invalid)?;
    Ok(body)
}

fn request(path: &Path) -> io::Result<FrozenRequest> {
    input::read_request(path)
}
fn original_id(request: &FrozenRequest) -> io::Result<String> {
    let body = request.body().map_err(invalid)?;
    request
        .operation_id
        .clone()
        .or_else(|| {
            body["requestId"]
                .as_str()
                .or_else(|| body["id"].as_str())
                .map(str::to_owned)
        })
        .ok_or_else(|| invalid("originalIdUnavailable"))
}

fn basis(path: &Path) -> io::Result<(Target, Value)> {
    let value = input::read_json(path)?;
    if value["schemaVersion"] != 1 {
        return Err(invalid("basisVersion"));
    }
    let target =
        serde_json::from_value(value["target"].clone()).map_err(|_| invalid("basisTarget"))?;
    let body = value
        .get("result")
        .cloned()
        .ok_or_else(|| invalid("basisResult"))?;
    text(&body, "sessionId")?;
    Ok((target, body))
}
pub(crate) fn author_root(document: &Value) -> Option<PathBuf> {
    document["workspaceId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .and_then(|_| document["root"].as_str())
        .map(PathBuf::from)
}

pub(crate) fn prepare(command: Command, remote: bool) -> io::Result<Prepared> {
    validate_stdin(&command)?;
    let mut out = Prepared {
        action: Action::Call(BusinessCall {
            kind: CallKind::Capabilities,
            body: json!({}),
        }),
        record: None,
        expected_target: None,
        author_root: None,
        source_output: None,
        resource: None,
        expected_artifact_digest: None,
        prerequisites: Vec::new(),
        original_path: None,
        secret_source: None,
        wait_seconds: 0,
    };
    let (kind, body) = match command {
        Command::Serve(_) | Command::OwnerStop { .. } | Command::OwnerStatus => {
            return Err(invalid("internalControl"));
        }
        Command::Capabilities => (CallKind::Capabilities, json!({})),
        Command::Library { command } => match command {
            LibraryCommand::List => (CallKind::Documents, json!({})),
            LibraryCommand::Inspect { path } => {
                local_only(remote)?;
                let path = absolute(path)?;
                out.author_root = Some(PathBuf::from(&path));
                (CallKind::LibraryInspect, json!({"path":path}))
            }
            LibraryCommand::Create {
                path,
                options,
                record,
            } => {
                local_only(remote)?;
                out.record = Some(record.request_file);
                let path = absolute(path)?;
                out.author_root = Some(PathBuf::from(&path));
                let options = options
                    .map(|p| input::read_json(&p))
                    .transpose()?
                    .unwrap_or_else(|| json!({}));
                (
                    CallKind::LibraryCreate,
                    with_id(json!({"path":path,"options":options}), "operationId", true)?,
                )
            }
        },
        Command::Note { command } => match command {
            NoteCommand::Read {
                document,
                root,
                source_output,
            } => {
                out.source_output = source_output;
                let root = root.map(absolute).transpose()?;
                if !remote {
                    out.author_root = root.as_ref().map(PathBuf::from);
                }
                match (remote, document.path, document.resource) {
                    (false, Some(path), None) => {
                        (CallKind::Open, json!({"path":absolute(path)?,"root":root}))
                    }
                    (true, None, Some(resource)) if root.is_none() => {
                        (CallKind::Open, json!({"resource":resource}))
                    }
                    _ => return Err(invalid("documentMode")),
                }
            }
            NoteCommand::Create {
                root,
                parent,
                name,
                record,
            } => {
                local_only(remote)?;
                let root = absolute(root)?;
                out.author_root = Some(PathBuf::from(&root));
                out.record = Some(record.request_file);
                (
                    CallKind::NoteCreate,
                    with_id(
                        json!({"root":root,"parent":absolute(parent)?,"name":name}),
                        "operationId",
                        true,
                    )?,
                )
            }
            NoteCommand::Write {
                basis: path,
                source,
                record,
            } => {
                let (target, document) = basis(&path)?;
                require_mode(&target, remote)?;
                out.expected_target = Some(target);
                out.resource = document["resource"].as_str().map(str::to_owned);
                out.author_root = author_root(&document);
                out.record = Some(record.request_file);
                let mut body = json!({"sessionId":text(&document,"sessionId")?,"baseId":text(&document,"baseId")?,"source":input::read_source(&source)?});
                if remote {
                    body["definitionReservations"] = json!([]);
                }
                (CallKind::Save, with_id(body, "operationId", true)?)
            }
            NoteCommand::SaveAs {
                basis: path,
                source,
                path: destination,
                record,
            } => {
                local_only(remote)?;
                let (target, document) = basis(&path)?;
                require_mode(&target, false)?;
                out.expected_target = Some(target);
                out.author_root = author_root(&document);
                out.record = Some(record.request_file);
                (
                    CallKind::SaveAs,
                    with_id(
                        json!({"sessionId":text(&document,"sessionId")?,"path":absolute(destination)?,"source":input::read_source(&source)?}),
                        "operationId",
                        true,
                    )?,
                )
            }
        },
        Command::Query { command } => match command {
            QueryCommand::Catalog { root } => {
                if remote {
                    if root.is_some() {
                        return Err(invalid("remoteLocalPath"));
                    }
                    (CallKind::QueryCatalog, json!({}))
                } else {
                    (
                        CallKind::QueryCatalog,
                        json!({"root":absolute(root.ok_or_else(||invalid("rootRequired"))?)?}),
                    )
                }
            }
            QueryCommand::Run { input: path } => {
                (CallKind::QueryRun, object(input::read_json(&path)?)?)
            }
        },
        Command::Operation { command } => match command {
            OperationCommand::Inspect { request: path } => {
                let original = request(&path)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path.clone());
                out.author_root = original.author_root.clone();
                out.action = Action::OriginalStatus(original);
                return Ok(out);
            }
            OperationCommand::Settle {
                request: path,
                record,
            } => {
                let original = request(&path)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path.clone());
                out.author_root = original.author_root.clone();
                out.action = Action::Settle(original);
                out.record = Some(record.request_file);
                return Ok(out);
            }
            OperationCommand::Prepare {
                input: path,
                record,
            } => {
                out.record = Some(record.request_file);
                (
                    CallKind::OperationPrepare,
                    with_id(input::read_json(&path)?, "operationId", true)?,
                )
            }
            OperationCommand::Commit {
                input: path,
                record,
            } => {
                out.record = Some(record.request_file);
                (
                    CallKind::OperationCommit,
                    with_id(input::read_json(&path)?, "operationId", false)?,
                )
            }
            OperationCommand::Recoveries => {
                local_only(remote)?;
                (CallKind::RecoveryList, json!({}))
            }
            OperationCommand::Recovery { operation_id } => {
                local_only(remote)?;
                input::valid_id(&operation_id).map_err(invalid)?;
                (
                    CallKind::RecoveryInspect,
                    json!({"operationId":operation_id}),
                )
            }
            OperationCommand::Resolve {
                operation_id,
                action,
                record,
            } => {
                local_only(remote)?;
                input::valid_id(&operation_id).map_err(invalid)?;
                out.record = Some(record.request_file);
                (
                    CallKind::RecoveryResolve,
                    json!({"operationId":operation_id,"action":action}),
                )
            }
        },
        Command::History { command } => match command {
            HistoryCommand::List { basis: path } => {
                let (target, document) = basis(&path)?;
                require_mode(&target, remote)?;
                out.expected_target = Some(target);
                (
                    CallKind::HistoryList,
                    json!({"sessionId":text(&document,"sessionId")?}),
                )
            }
            HistoryCommand::Read {
                basis: path,
                version,
            } => {
                let (target, document) = basis(&path)?;
                require_mode(&target, remote)?;
                out.expected_target = Some(target);
                (
                    CallKind::HistoryRead,
                    json!({"sessionId":text(&document,"sessionId")?,"versionId":version}),
                )
            }
            HistoryCommand::Compare {
                basis: path,
                version,
            } => {
                let (target, document) = basis(&path)?;
                require_mode(&target, remote)?;
                out.expected_target = Some(target);
                (
                    CallKind::HistoryCompare,
                    json!({"sessionId":text(&document,"sessionId")?,"versionId":version}),
                )
            }
            HistoryCommand::Restore {
                input: path,
                record,
            } => {
                out.record = Some(record.request_file);
                (
                    CallKind::HistoryRestore,
                    with_id(input::read_json(&path)?, "operationId", true)?,
                )
            }
        },
        Command::Export { command } => match command {
            ExportCommand::Prepare {
                basis: path,
                input: payload,
                source,
                kind,
                output_locale,
                review_mode,
                include_resolved,
                style_policy,
                record,
            } => {
                if let Some(payload) = payload {
                    if source.is_some()
                        || kind.is_some()
                        || output_locale.is_some()
                        || review_mode != "none"
                        || include_resolved
                        || style_policy != "source"
                    {
                        return Err(invalid("exportInputOptions"));
                    }
                    let body = with_id(input::read_json(&payload)?, "requestId", true)?;
                    if let Some(path) = path {
                        let (target, document) = basis(&path)?;
                        require_mode(&target, remote)?;
                        if !remote
                            && (body["sessionId"] != document["sessionId"]
                                || body["baseId"] != document["baseId"])
                        {
                            return Err(invalid("basisConflict"));
                        }
                        out.expected_target = Some(target);
                        if !remote {
                            out.author_root = author_root(&document);
                        }
                    } else if !remote {
                        return Err(invalid("basisRequired"));
                    }
                    export_snapshot(&mut out, &body, remote)?;
                    out.record = Some(record.request_file);
                    out.resource = body["resource"].as_str().map(str::to_owned);
                    out.action = Action::Call(BusinessCall {
                        kind: CallKind::ExportPrepare,
                        body,
                    });
                    return Ok(out);
                }
                let path = path.ok_or_else(|| invalid("basisRequired"))?;
                let (target, document) = basis(&path)?;
                require_mode(&target, remote)?;
                out.expected_target = Some(target);
                out.author_root = author_root(&document);
                out.resource = document["resource"].as_str().map(str::to_owned);
                out.record = Some(record.request_file);
                let kind = kind.ok_or_else(|| invalid("formatRequired"))?;
                if !matches!(kind.as_str(), "docx" | "doc" | "wps" | "pdf" | "source")
                    || remote && kind == "source"
                {
                    return Err(invalid("exportPayloadRequired"));
                }
                let mut body = json!({"source":match source {Some(path)=>input::read_source(&path)?,None=>document["source"].as_str().ok_or_else(||invalid("basisField"))?.to_owned()},"format":kind,"outputLocale":output_locale.as_deref().unwrap_or("zh-CN"),"reviewMode":review_mode,"includeResolved":include_resolved,"stylePolicy":style_policy,"bindings":{}});
                if remote {
                    body["resource"] = json!(text(&document, "resource")?);
                    if kind == "pdf" {
                        body["sessionId"] = json!(text(&document, "sessionId")?);
                        body["baseId"] = json!(text(&document, "baseId")?);
                        body["serial"] = json!(0);
                        body["sourceHash"] = json!(weftext_engine::source_hash(
                            body["source"]
                                .as_str()
                                .ok_or_else(|| invalid("sourceRequired"))?
                        ));
                    }
                } else {
                    body["sessionId"] = json!(text(&document, "sessionId")?);
                    body["baseId"] = json!(text(&document, "baseId")?);
                    body["serial"] = json!(0);
                }
                export_snapshot(&mut out, &body, remote)?;
                (CallKind::ExportPrepare, with_id(body, "requestId", true)?)
            }
            ExportCommand::Status { request: path } => {
                let original = request(&path)?;
                require_export(&original)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path.clone());
                out.author_root = original.author_root.clone();
                (
                    CallKind::ExportStatus,
                    json!({"requestId":original_id(&original)?}),
                )
            }
            ExportCommand::Confirm {
                request: path,
                digest,
                accept_losses,
                record,
            } => {
                let original = request(&path)?;
                if !matches!(
                    original.kind,
                    CallKind::ExportPrepare | CallKind::ExportConfirm
                ) {
                    return Err(invalid("exportRequestKind"));
                }
                if digest.len() != 64
                    || !digest
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                {
                    return Err(invalid("artifactDigest"));
                }
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path.clone());
                out.author_root = original.author_root.clone();
                out.record = Some(record.request_file);
                out.expected_artifact_digest = Some(digest.clone());
                let id = original_id(&original)?;
                (
                    CallKind::ExportConfirm,
                    if remote {
                        json!({"requestId":id,"digest":digest,"acceptLosses":accept_losses})
                    } else {
                        json!({"id":id,"acceptLosses":accept_losses})
                    },
                )
            }
            ExportCommand::Download {
                request: path,
                destination,
            } => {
                let original = request(&path)?;
                require_export(&original)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path.clone());
                out.author_root = original.author_root.clone();
                out.action = Action::Download {
                    request: original,
                    destination,
                };
                return Ok(out);
            }
        },
        Command::Import { command } => match command {
            ImportCommand::Inspect {
                file,
                kind,
                destination,
            } => import_payload(&mut out, remote, file, kind, destination, None, false)?,
            ImportCommand::Prepare {
                file,
                kind,
                destination,
                record,
            } => import_payload(
                &mut out,
                remote,
                file,
                kind,
                destination,
                Some(record.request_file),
                true,
            )?,
            ImportCommand::Status { request: path } => {
                let original = request(&path)?;
                require_import(&original)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path.clone());
                out.author_root = original.author_root.clone();
                return status_prepared(out, original, remote);
            }
            ImportCommand::Settle {
                request: path,
                record,
            } => {
                let original = request(&path)?;
                require_import(&original)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path.clone());
                out.author_root = original.author_root.clone();
                out.record = Some(record.request_file);
                out.action = Action::Settle(original);
                return Ok(out);
            }
            ImportCommand::Commit {
                request: path,
                plan_id,
                accept_losses,
                record,
            } => {
                let original = request(&path)?;
                if !matches!(
                    original.kind,
                    CallKind::ImportPrepare | CallKind::OfficeImportPrepare
                ) {
                    return Err(invalid("importRequestKind"));
                }
                let original_body = original.body().map_err(invalid)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path.clone());
                out.author_root = original.author_root.clone();
                out.record = Some(record.request_file);
                out.resource = original.resource.clone();
                let kind = if original.kind == CallKind::OfficeImportPrepare {
                    CallKind::OfficeImportCommit
                } else {
                    CallKind::ImportCommit
                };
                let mut body = json!({"operationId":original_id(&original)?,"planId":plan_id,"acceptLosses":accept_losses});
                if remote {
                    copy_selector(&original_body, &mut body)?;
                }
                (kind, body)
            }
        },
        Command::Sync { command } => match command {
            SyncCommand::Check { connection } => {
                local_only(remote)?;
                let body = sync_connection(&mut out, connection)?;
                (CallKind::RemoteSyncInspect, body["connection"].clone())
            }
            SyncCommand::Push {
                connection,
                root,
                wait_seconds,
                record,
            } => {
                local_only(remote)?;
                let root = absolute(root)?;
                out.author_root = Some(PathBuf::from(&root));
                out.record = Some(record.request_file);
                out.wait_seconds = wait_seconds;
                let mut body = sync_connection(&mut out, connection)?;
                body["requestId"] = json!(uuid::Uuid::new_v4().to_string());
                body["direction"] = json!("push");
                body["root"] = json!(root);
                (CallKind::RemoteSyncStart, body)
            }
            SyncCommand::Pull {
                connection,
                objects,
                wait_seconds,
                record,
            } => {
                local_only(remote)?;
                out.record = Some(record.request_file);
                out.wait_seconds = wait_seconds;
                let mut body = sync_connection(&mut out, connection)?;
                body["requestId"] = json!(uuid::Uuid::new_v4().to_string());
                body["direction"] = json!("pull");
                if let Some(path) = objects {
                    body["objects"] = input::read_json(&path)?;
                }
                (CallKind::RemoteSyncStart, body)
            }
            SyncCommand::Job { request: path } => {
                local_only(remote)?;
                let original = request(&path)?;
                require_sync_job(&original)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path);
                out.author_root = original.author_root.clone();
                out.action = Action::OriginalStatus(original);
                return Ok(out);
            }
            SyncCommand::Stop {
                request: path,
                record,
            } => {
                local_only(remote)?;
                let original = request(&path)?;
                require_sync_job(&original)?;
                out.expected_target = Some(original.target.clone());
                out.original_path = Some(path);
                out.author_root = original.author_root.clone();
                out.record = Some(record.request_file);
                (
                    CallKind::RemoteSyncStop,
                    json!({"requestId":original_id(&original)?}),
                )
            }
            SyncCommand::Binding { root } => {
                local_only(remote)?;
                (CallKind::SyncBinding, json!({"root":absolute(root)?}))
            }
            SyncCommand::Enable { root, record } => {
                local_only(remote)?;
                out.record = Some(record.request_file);
                (
                    CallKind::SyncEnable,
                    with_id(json!({"root":absolute(root)?}), "operationId", true)?,
                )
            }
            SyncCommand::Status { input: path } => {
                (CallKind::SyncStatus, object(input::read_json(&path)?)?)
            }
            SyncCommand::Publish { root, record } => {
                local_only(remote)?;
                out.record = Some(record.request_file);
                (CallKind::SyncPublish, json!({"root":absolute(root)?}))
            }
            SyncCommand::Receive {
                input: path,
                record,
            } => {
                local_only(remote)?;
                out.record = Some(record.request_file);
                (CallKind::SyncReceive, object(input::read_json(&path)?)?)
            }
            SyncCommand::Export {
                input: path,
                record,
            } => {
                local_only(remote)?;
                out.record = Some(record.request_file);
                (CallKind::SyncExport, object(input::read_json(&path)?)?)
            }
            SyncCommand::History { input: path } => {
                local_only(remote)?;
                (CallKind::SyncHistory, object(input::read_json(&path)?)?)
            }
            SyncCommand::Operation { operation_id } => {
                local_only(remote)?;
                input::valid_id(&operation_id).map_err(invalid)?;
                (CallKind::SyncOperation, json!({"operationId":operation_id}))
            }
            SyncCommand::Cancel {
                input: path,
                record,
            } => {
                local_only(remote)?;
                out.record = Some(record.request_file);
                (
                    CallKind::SyncCancel,
                    with_id(input::read_json(&path)?, "operationId", false)?,
                )
            }
            SyncCommand::AdoptionCut { input: path } => {
                local_only(remote)?;
                (CallKind::SyncAdoptionCut, object(input::read_json(&path)?)?)
            }
            SyncCommand::RevalidationCut { input: path } => {
                local_only(remote)?;
                (
                    CallKind::SyncRevalidationCut,
                    object(input::read_json(&path)?)?,
                )
            }
            SyncCommand::RevalidationReview { input: path } => {
                local_only(remote)?;
                (
                    CallKind::SyncRevalidationReview,
                    object(input::read_json(&path)?)?,
                )
            }
        },
    };
    let body = object(body)?;
    if out.author_root.is_none() && !remote {
        out.author_root = body["root"].as_str().map(PathBuf::from);
    }
    if out.resource.is_none() {
        out.resource = body["resource"].as_str().map(str::to_owned);
    }
    let supported = if remote {
        kind.remote_route().is_some()
    } else {
        kind.local_route().is_some()
    };
    if !supported {
        return Err(invalid("unsupported"));
    }
    if kind.is_effect() && out.record.is_none() {
        return Err(invalid("requestFileRequired"));
    }
    out.action = Action::Call(BusinessCall { kind, body });
    Ok(out)
}

fn require_export(request: &FrozenRequest) -> io::Result<()> {
    if matches!(
        request.kind,
        CallKind::ExportPrepare | CallKind::ExportConfirm
    ) {
        Ok(())
    } else {
        Err(invalid("exportRequestKind"))
    }
}
fn require_sync_job(request: &FrozenRequest) -> io::Result<()> {
    if matches!(
        request.kind,
        CallKind::RemoteSyncStart | CallKind::RemoteSyncStop
    ) {
        Ok(())
    } else {
        Err(invalid("syncRequestKind"))
    }
}
fn sync_connection(out: &mut Prepared, connection: SyncConnection) -> io::Result<Value> {
    let endpoint =
        super::remote::validate_endpoint(&connection.server, connection.allow_server_loopback_http)
            .map_err(|error| invalid(error.code))?;
    let value = input::read_json(&connection.identity)?;
    let identity = if value["schemaVersion"] == 1 {
        let target: Target =
            serde_json::from_value(value["target"].clone()).map_err(|_| invalid("basisTarget"))?;
        require_mode(&target, false)?;
        out.expected_target = Some(target);
        value["result"]["identity"].clone()
    } else {
        value
    };
    let _: weftext_core::sync::SyncIdentity =
        serde_json::from_value(identity.clone()).map_err(|_| invalid("syncIdentity"))?;
    out.secret_source = Some(connection.server_credential_env);
    Ok(json!({"connection":{"endpoint":endpoint.as_str(),"identity":identity}}))
}
fn export_snapshot(out: &mut Prepared, body: &Value, remote: bool) -> io::Result<()> {
    if remote
        && body["format"] != "pdf"
        && !body["embeddedQueries"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    {
        return Ok(());
    }
    let source = body["source"]
        .as_str()
        .ok_or_else(|| invalid("sourceRequired"))?;
    let hash = weftext_engine::source_hash(source);
    if body
        .get("sourceHash")
        .is_some_and(|value| value.as_str() != Some(hash.as_str()))
    {
        return Err(invalid("sourceHash"));
    }
    let serial = body["serial"]
        .as_u64()
        .ok_or_else(|| invalid("serialRequired"))?;
    out.prerequisites.push(BusinessCall{kind:CallKind::EditorSnapshot,body:json!({"sessionId":text(body,"sessionId")?,"source":source,"serial":serial,"sourceHash":hash})});
    Ok(())
}
fn require_import(request: &FrozenRequest) -> io::Result<()> {
    if matches!(
        request.kind,
        CallKind::ImportPrepare
            | CallKind::ImportCommit
            | CallKind::ImportSettle
            | CallKind::OfficeImportPrepare
            | CallKind::OfficeImportCommit
            | CallKind::OfficeImportSettle
    ) {
        Ok(())
    } else {
        Err(invalid("importRequestKind"))
    }
}

/// Each process receives one stream. Reject ambiguous consumers before reading
/// any file or stdin, rather than freezing a silently empty second input.
fn validate_stdin(command: &Command) -> io::Result<()> {
    let mut paths: Vec<&Path> = Vec::new();
    match command {
        Command::Sync {
            command: SyncCommand::Check { connection } | SyncCommand::Push { connection, .. },
        } => paths.push(&connection.identity),
        Command::Sync {
            command:
                SyncCommand::Pull {
                    connection,
                    objects,
                    ..
                },
        } => {
            paths.push(&connection.identity);
            if let Some(objects) = objects {
                paths.push(objects);
            }
        }
        Command::Sync {
            command: SyncCommand::Job { request } | SyncCommand::Stop { request, .. },
        } => paths.push(request),
        Command::Library {
            command:
                LibraryCommand::Create {
                    options: Some(path),
                    ..
                },
        } => paths.push(path),
        Command::Note {
            command:
                NoteCommand::Write { basis, source, .. } | NoteCommand::SaveAs { basis, source, .. },
        } => paths.extend([basis.as_path(), source.as_path()]),
        Command::Query {
            command: QueryCommand::Run { input },
        }
        | Command::Operation {
            command:
                OperationCommand::Prepare { input, .. } | OperationCommand::Commit { input, .. },
        }
        | Command::History {
            command: HistoryCommand::Restore { input, .. },
        }
        | Command::Sync {
            command:
                SyncCommand::Status { input }
                | SyncCommand::Receive { input, .. }
                | SyncCommand::Export { input, .. }
                | SyncCommand::History { input }
                | SyncCommand::Cancel { input, .. }
                | SyncCommand::AdoptionCut { input }
                | SyncCommand::RevalidationCut { input }
                | SyncCommand::RevalidationReview { input },
        } => paths.push(input),
        Command::Operation {
            command:
                OperationCommand::Inspect { request } | OperationCommand::Settle { request, .. },
        }
        | Command::Export {
            command:
                ExportCommand::Status { request }
                | ExportCommand::Confirm { request, .. }
                | ExportCommand::Download { request, .. },
        }
        | Command::Import {
            command:
                ImportCommand::Status { request }
                | ImportCommand::Commit { request, .. }
                | ImportCommand::Settle { request, .. },
        } => paths.push(request),
        Command::History {
            command:
                HistoryCommand::List { basis }
                | HistoryCommand::Read { basis, .. }
                | HistoryCommand::Compare { basis, .. },
        } => paths.push(basis),
        Command::Export {
            command:
                ExportCommand::Prepare {
                    basis,
                    input,
                    source,
                    ..
                },
        } => {
            if let Some(basis) = basis {
                paths.push(basis);
            }
            if let Some(input) = input {
                paths.push(input);
            }
            if let Some(source) = source {
                paths.push(source);
            }
        }
        Command::Import {
            command:
                ImportCommand::Inspect {
                    file, destination, ..
                }
                | ImportCommand::Prepare {
                    file, destination, ..
                },
        } => {
            paths.push(file);
            if let Some(basis) = &destination.basis {
                paths.push(basis);
            }
        }
        _ => {}
    }
    if paths
        .into_iter()
        .filter(|path| *path == Path::new("-"))
        .count()
        > 1
    {
        return Err(invalid("multipleStdinInputs"));
    }
    Ok(())
}

fn status_prepared(
    mut out: Prepared,
    original: FrozenRequest,
    _remote: bool,
) -> io::Result<Prepared> {
    out.action = Action::OriginalStatus(original);
    Ok(out)
}
fn import_payload(
    out: &mut Prepared,
    remote: bool,
    file: PathBuf,
    format: String,
    destination: ImportDestination,
    record: Option<PathBuf>,
    effect: bool,
) -> io::Result<(CallKind, Value)> {
    let office = remote && matches!(format.as_str(), "docx" | "doc" | "wps");
    let mut body = if remote {
        if destination.basis.is_some() || destination.parent.is_some() || destination.name.is_some()
        {
            return Err(invalid("remoteLocalPath"));
        }
        if office {
            if destination.slot.is_some() {
                return Err(invalid("importSelector"));
            }
            json!({"resource":destination.resource.ok_or_else(||invalid("resourceRequired"))?})
        } else {
            if destination.resource.is_some() {
                return Err(invalid("importSelector"));
            }
            json!({"slot":destination.slot.ok_or_else(||invalid("slotRequired"))?})
        }
    } else {
        if destination.resource.is_some() || destination.slot.is_some() {
            return Err(invalid("documentMode"));
        }
        if effect {
            let (target, document) =
                basis(&destination.basis.ok_or_else(|| invalid("basisRequired"))?)?;
            require_mode(&target, false)?;
            out.expected_target = Some(target);
            let root = text(&document, "root")?;
            json!({"sessionId":text(&document,"sessionId")?,"baseId":text(&document,"baseId")?,"root":root,
                "parent":match destination.parent{Some(path)=>absolute(path)?,None=>root},
                "name":destination.name.ok_or_else(||invalid("nameRequired"))?})
        } else {
            json!({})
        }
    };
    let bytes = input::read_input(
        &file,
        if remote {
            input::MAX_UPLOAD_BYTES
        } else {
            64 * 1024 * 1024
        },
    )?;
    body["format"] = json!(format);
    body["bytesBase64"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
    out.record = record;
    if !effect {
        let original = body;
        body = json!({"format":original["format"],"bytesBase64":original["bytesBase64"]});
        if remote {
            copy_selector(&original, &mut body)?;
        }
    }
    if !remote {
        out.author_root = body["root"].as_str().map(PathBuf::from);
    }
    out.resource = body["resource"].as_str().map(str::to_owned);
    if effect {
        body = with_id(body, "operationId", true)?;
    }
    Ok((
        match (effect, office) {
            (false, false) => CallKind::ImportInspect,
            (true, false) => CallKind::ImportPrepare,
            (false, true) => CallKind::OfficeImportInspect,
            (true, true) => CallKind::OfficeImportPrepare,
        },
        body,
    ))
}
pub(crate) fn copy_selector(source: &Value, target: &mut Value) -> io::Result<()> {
    let field = if source["slot"].is_string() {
        "slot"
    } else if source["resource"].is_string() {
        "resource"
    } else {
        return Err(invalid("originalSelector"));
    };
    target[field] = source[field].clone();
    Ok(())
}
fn require_mode(target: &Target, remote: bool) -> io::Result<()> {
    if matches!(target, Target::Remote { .. }) == remote {
        Ok(())
    } else {
        Err(invalid("basisTarget"))
    }
}
fn local_only(remote: bool) -> io::Result<()> {
    if remote {
        Err(invalid("unsupportedRemote"))
    } else {
        Ok(())
    }
}
fn absolute(path: PathBuf) -> io::Result<String> {
    std::path::absolute(path)?
        .into_os_string()
        .into_string()
        .map_err(|_| invalid("pathUnicode"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    fn command(args: &[&str]) -> Command {
        Args::parse_from(std::iter::once("weftext-cli").chain(args.iter().copied())).command
    }
    fn record() -> Record {
        Record {
            request_file: PathBuf::from("result.weftext-request.json"),
        }
    }
    fn local_basis(path: &Path, source: &str) -> PathBuf {
        let file = path.join("basis.json");
        std::fs::write(&file,serde_json::to_vec(&json!({"schemaVersion":1,"target":{"mode":"local","profileIdentity":"physical-profile"},"result":{"sessionId":"session","baseId":"base","root":path.join("library"),"source":source}})).unwrap()).unwrap();
        file
    }
    fn call(prepared: Prepared) -> BusinessCall {
        match prepared.action {
            Action::Call(call) => call,
            _ => panic!("expected business call"),
        }
    }
    #[test]
    fn ambiguous_stdin_is_rejected_before_any_read() {
        let command = command(&[
            "note",
            "write",
            "--basis",
            "-",
            "--source",
            "-",
            "--request-file",
            "result.weftext-request.json",
        ]);
        assert_eq!(
            prepare(command, false).err().unwrap().to_string(),
            "multipleStdinInputs"
        );
    }
    #[test]
    fn write_uses_frozen_basis_and_literal_source() {
        let temp = tempfile::tempdir().unwrap();
        let basis = local_basis(temp.path(), "original");
        let source = temp.path().join("source.typ");
        std::fs::write(&source, "中文🙂\r\n").unwrap();
        let prepared = prepare(
            Command::Note {
                command: NoteCommand::Write {
                    basis,
                    source,
                    record: record(),
                },
            },
            false,
        )
        .unwrap();
        assert_eq!(
            prepared.expected_target,
            Some(Target::Local {
                profile_identity: "physical-profile".into()
            })
        );
        let call = call(prepared);
        assert_eq!(call.kind, CallKind::Save);
        assert_eq!(call.body["sessionId"], "session");
        assert_eq!(call.body["baseId"], "base");
        assert_eq!(call.body["source"], "中文🙂\r\n");
        input::valid_id(call.body["operationId"].as_str().unwrap()).unwrap();
    }
    #[test]
    fn empty_authored_source_is_valid_export_input() {
        let temp = tempfile::tempdir().unwrap();
        let basis = local_basis(temp.path(), "");
        let call = call(
            prepare(
                Command::Export {
                    command: ExportCommand::Prepare {
                        basis: Some(basis),
                        input: None,
                        source: None,
                        kind: Some("docx".into()),
                        output_locale: None,
                        review_mode: "none".into(),
                        include_resolved: false,
                        style_policy: "source".into(),
                        record: record(),
                    },
                },
                false,
            )
            .unwrap(),
        );
        assert_eq!(call.body["source"], "");
        assert_eq!(call.body["sessionId"], "session");
        assert_eq!(call.body["serial"], 0);
        assert_eq!(call.body["outputLocale"], "zh-CN");
    }
    #[test]
    fn export_content_locale_is_independent_of_presentation_and_full_input() {
        let temp = tempfile::tempdir().unwrap();
        let basis = local_basis(temp.path(), "作者原文😀");
        for (presentation, content) in [("en-US", "zh-CN"), ("zh-CN", "en-US")] {
            let command = command(&[
                "--locale",
                presentation,
                "export",
                "prepare",
                "--basis",
                basis.to_str().unwrap(),
                "--kind",
                "source",
                "--output-locale",
                content,
                "--request-file",
                "result.weftext-request.json",
            ]);
            let call = call(prepare(command, false).unwrap());
            assert_eq!(call.body["outputLocale"], content);
            assert_eq!(call.body["source"], "作者原文😀");
        }
        for content in ["en", ""] {
            assert!(
                Args::try_parse_from([
                    "weftext-cli",
                    "export",
                    "prepare",
                    "--basis",
                    "basis.json",
                    "--kind",
                    "source",
                    "--output-locale",
                    content,
                    "--request-file",
                    "result.weftext-request.json"
                ])
                .is_err()
            );
        }
        assert!(
            Args::try_parse_from([
                "weftext-cli",
                "export",
                "prepare",
                "--input",
                "input.json",
                "--output-locale",
                "zh-CN",
                "--request-file",
                "result.weftext-request.json"
            ])
            .is_err()
        );
    }
    #[test]
    fn remote_import_never_promotes_a_local_path_into_a_server_capability() {
        let command = command(&[
            "import",
            "prepare",
            "--file",
            "does-not-exist.docx",
            "--kind",
            "docx",
            "--basis",
            "local.json",
            "--request-file",
            "result.weftext-request.json",
        ]);
        assert_eq!(
            prepare(command, true).err().unwrap().to_string(),
            "remoteLocalPath"
        );
    }
    #[test]
    fn commit_requires_the_original_id_and_cannot_generate_a_new_attempt() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("input.json");
        std::fs::write(&input, br#"{"planId":"plan"}"#).unwrap();
        assert_eq!(
            prepare(
                Command::Operation {
                    command: OperationCommand::Commit {
                        input,
                        record: record()
                    }
                },
                false
            )
            .err()
            .unwrap()
            .to_string(),
            "operationIdRequired"
        );
    }
    #[test]
    fn business_family_is_not_selected_only_by_reused_uuid() {
        let request = FrozenRequest::new(
            Target::Local {
                profile_identity: "physical-profile".into(),
            },
            BusinessCall {
                kind: CallKind::Save,
                body: json!({"operationId":uuid::Uuid::new_v4().to_string()}),
            },
        )
        .unwrap();
        assert!(require_export(&request).is_err());
        assert!(require_import(&request).is_err());
    }
    #[test]
    fn remote_native_export_needs_no_additional_readable_document() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("export.json");
        std::fs::write(&path,br#"{"format":"native","outputLocale":"en-US","reviewMode":"none","includeResolved":false,"stylePolicy":"source","bindings":{}}"#).unwrap();
        let prepared = prepare(
            Command::Export {
                command: ExportCommand::Prepare {
                    basis: None,
                    input: Some(path),
                    source: None,
                    kind: None,
                    output_locale: None,
                    review_mode: "none".into(),
                    include_resolved: false,
                    style_policy: "source".into(),
                    record: record(),
                },
            },
            true,
        )
        .unwrap();
        assert!(prepared.prerequisites.is_empty());
        let call = call(prepared);
        assert_eq!(call.body["format"], "native");
        assert_eq!(call.body["outputLocale"], "en-US");
        assert!(call.body.get("resource").is_none());
        assert!(call.body.get("source").is_none());
    }
    #[test]
    fn native_source_budgets_are_checked_before_any_frozen_effect() {
        let call = BusinessCall {
            kind: CallKind::OperationPrepare,
            body: json!({"context":{"source":"x".repeat(input::MAX_SOURCE_BYTES+1)}}),
        };
        assert_eq!(call.encode().unwrap_err(), "inputBudget");
    }
    #[test]
    fn unmanaged_file_parent_is_not_invented_as_an_admitted_author_library() {
        let value = json!({"root":"C:/Documents","source":"plain source"});
        assert!(author_root(&value).is_none());
        let value = json!({"root":"C:/Notes","workspaceId":"workspace"});
        assert_eq!(author_root(&value), Some(PathBuf::from("C:/Notes")));
    }
}
