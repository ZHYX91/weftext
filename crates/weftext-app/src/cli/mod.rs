//! Formal business console client. Storage and author effects remain in Core.
mod args;
mod input;
mod owner;
mod prepare;
mod remote;

use crate::client_protocol::{BusinessCall, CallKind};
use crate::local_client::{
    self, LocalCall, LocalConnection, ProfileLocator, RemoteSyncSecret, SubmissionEffect,
};
use args::{Args, OutputFormat};
use clap::{CommandFactory, FromArgMatches};
use input::{FrozenRequest, Target};
use prepare::{Action, Prepared};
use remote::{Remote, Reply};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::Path,
    time::Duration,
};
use weftext_messages::{UiLocale, UiMessage};

const INPUT: u8 = 2;
const UNAVAILABLE: u8 = 3;
const AUTHORIZATION: u8 = 4;
const CONFLICT: u8 = 5;
const UNKNOWN: u8 = 6;
const DELIVERY: u8 = 7;
const PARTIAL: u8 = 8;

enum Transport {
    Local(Box<LocalConnection>),
    Remote(Remote),
}
struct Context {
    transport: Transport,
    target: Target,
    owner_incarnation: Option<String>,
    owner_identity: Option<local_client::OwnerIdentity>,
    private_root: Option<std::path::PathBuf>,
}
struct Failure {
    code: String,
    exit: u8,
    original_id: Option<String>,
    request_path: Option<String>,
}
impl Failure {
    fn new(code: impl Into<String>, exit: u8) -> Self {
        let code = code.into();
        let code = if safe_code(&code) {
            code
        } else {
            "commandFailed".into()
        };
        Self {
            code,
            exit,
            original_id: None,
            request_path: None,
        }
    }
    fn original(mut self, request: &FrozenRequest, path: Option<&Path>) -> Self {
        self.original_id = request.operation_id.clone();
        self.request_path = path.map(|p| p.display().to_string());
        self
    }
}

impl Context {
    async fn connect(args: &Args) -> Result<Self, Failure> {
        if let Some(endpoint) = &args.endpoint {
            let source = args
                .credential_env
                .as_deref()
                .ok_or_else(|| Failure::new("credentialsRequired", AUTHORIZATION))?;
            let client = Remote::from_environment(endpoint, source, args.allow_loopback_http)
                .map_err(|e| Failure::new(e.code, remote_setup_exit(e.code)))?;
            let target = client.target().await.map_err(|e| {
                Failure::new(
                    e.code,
                    if e.code == "authentication" {
                        AUTHORIZATION
                    } else {
                        UNAVAILABLE
                    },
                )
            })?;
            Ok(Self {
                transport: Transport::Remote(client),
                target,
                owner_incarnation: None,
                owner_identity: None,
                private_root: None,
            })
        } else {
            let profile = match &args.data_dir {
                Some(data_dir) => ProfileLocator {
                    data_dir: data_dir.clone(),
                },
                None => ProfileLocator::host_default()
                    .map_err(|_| Failure::new("profileUnavailable", UNAVAILABLE))?,
            };
            let connection = local_client::connect_or_start(&profile, Duration::from_secs(30))
                .await
                .map_err(|e| {
                    Failure::new(
                        e.code,
                        if e.effect == SubmissionEffect::MayHaveBeenAccepted {
                            UNKNOWN
                        } else {
                            UNAVAILABLE
                        },
                    )
                })?;
            let owner_identity = connection.owner_identity();
            let target = Target::Local {
                profile_identity: connection.profile_identity().to_owned(),
            };
            Ok(Self {
                transport: Transport::Local(Box::new(connection)),
                target,
                owner_incarnation: None,
                owner_identity: Some(owner_identity),
                private_root: Some(profile.data_dir),
            })
        }
    }
    fn remote(&self) -> bool {
        matches!(self.target, Target::Remote { .. })
    }
    async fn call(
        &mut self,
        kind: CallKind,
        wire: &[u8],
        request_id: &str,
    ) -> Result<Reply, Failure> {
        self.call_with_secret(kind, wire, request_id, None).await
    }
    async fn call_with_secret(
        &mut self,
        kind: CallKind,
        wire: &[u8],
        request_id: &str,
        secret: Option<RemoteSyncSecret>,
    ) -> Result<Reply, Failure> {
        if self.remote() && secret.is_some() {
            return Err(Failure::new("unsupportedSecret", UNAVAILABLE));
        }
        match &mut self.transport {
            Transport::Remote(client) => client.call(kind, wire).await.map_err(|error| {
                Failure::new(
                    error.code,
                    if kind.is_effect() && error.may_have_been_accepted {
                        UNKNOWN
                    } else {
                        UNAVAILABLE
                    },
                )
            }),
            Transport::Local(connection) => {
                let reply = connection
                    .call(LocalCall {
                        request_id: request_id.to_owned(),
                        kind,
                        body: wire.to_vec(),
                        secret,
                    })
                    .await
                    .map_err(|error| {
                        Failure::new(
                            error.code,
                            if kind.is_effect()
                                && error.effect == SubmissionEffect::MayHaveBeenAccepted
                            {
                                UNKNOWN
                            } else {
                                UNAVAILABLE
                            },
                        )
                    })?;
                if reply.request_id != request_id
                    || reply.body.is_empty() && kind != CallKind::ExportDownload
                {
                    return Err(Failure::new(
                        "replyIdentity",
                        if kind.is_effect() {
                            UNKNOWN
                        } else {
                            UNAVAILABLE
                        },
                    ));
                }
                self.owner_incarnation = Some(reply.owner_incarnation);
                Ok(Reply {
                    status: reply.http_status,
                    content_type: reply.content_type,
                    body: reply.body,
                })
            }
        }
    }
    async fn json(&mut self, call: BusinessCall) -> Result<(Reply, Value), Failure> {
        let wire = call.encode().map_err(|e| Failure::new(e, INPUT))?;
        let reply = self
            .call(call.kind, &wire, &uuid::Uuid::new_v4().to_string())
            .await?;
        if !reply
            .content_type
            .split(';')
            .next()
            .is_some_and(|s| s.trim() == "application/json")
        {
            return Err(Failure::new(
                "replyType",
                if call.kind.is_effect() {
                    UNKNOWN
                } else {
                    UNAVAILABLE
                },
            ));
        }
        let body = input::parse_json(&reply.body).map_err(|_| {
            Failure::new(
                "replyJson",
                if call.kind.is_effect() {
                    UNKNOWN
                } else {
                    UNAVAILABLE
                },
            )
        })?;
        Ok((reply, body))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a> {
    schema_version: u32,
    target: &'a Target,
    call: CallKind,
    http_status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_incarnation: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_identity: Option<&'a local_client::OwnerIdentity>,
    result: &'a Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_request: Option<&'a Value>,
}

/// Called only after the executable's shared internal-worker dispatcher.
pub async fn run() -> u8 {
    run_from(std::env::args_os()).await
}

pub async fn run_from(arguments: impl IntoIterator<Item = OsString>) -> u8 {
    let arguments: Vec<OsString> = arguments.into_iter().collect();
    let diagnostic_format = early_output_format(&arguments);
    let locale = arguments
        .windows(2)
        .find(|pair| pair[0] == "--locale")
        .and_then(|pair| pair[1].to_str())
        .or_else(|| {
            arguments
                .iter()
                .filter_map(|a| a.to_str())
                .find_map(|v| v.strip_prefix("--locale="))
        })
        .and_then(|v| v.parse::<UiLocale>().ok())
        .unwrap_or_default();
    let mut command = localized_command(Args::command(), locale)
        .about(UiMessage::cli_description().render(locale))
        .after_help(UiMessage::cli_help_commands().render(locale));
    let matches = match command.try_get_matches_from_mut(arguments) {
        Ok(matches) => matches,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                return if error.print().is_ok() { 0 } else { DELIVERY };
            }
            report_failure(
                &Failure::new("invalidArguments", INPUT),
                locale,
                diagnostic_format,
            );
            return INPUT;
        }
    };
    let args = match Args::from_arg_matches(&matches) {
        Ok(args) => args,
        Err(_) => {
            report_failure(
                &Failure::new("invalidArguments", INPUT),
                locale,
                diagnostic_format,
            );
            return INPUT;
        }
    };
    let locale = args.locale.parse::<UiLocale>().unwrap_or_default();
    let result = match &args.command {
        args::Command::Serve(serve) => run_server(&args, serve).await,
        args::Command::OwnerStop { owner_identity } => {
            owner::stop(&args, owner_identity, locale).await
        }
        args::Command::OwnerStatus => owner::status(&args, locale).await,
        _ => execute(&args, locale).await,
    };
    match result {
        Ok(exit) => exit,
        Err(failure) => {
            report_failure(&failure, locale, args.format);
            failure.exit
        }
    }
}
async fn run_server(args: &Args, serve: &crate::launch::ServeArgs) -> Result<u8, Failure> {
    if args.endpoint.is_some() || args.credential_env.is_some() || args.allow_loopback_http {
        return Err(Failure::new("serveClientOptions", INPUT));
    }
    if args.output.is_some() || args.format != OutputFormat::Json || args.allow_partial {
        return Err(Failure::new("serveOutputOptions", INPUT));
    }
    let mut serve = serve.clone();
    serve.data_dir = args.data_dir.clone();
    // Launch owns the sole Ready line and the actual owner lifecycle. No CLI
    // business connection, replacement Core or second readiness record exists.
    crate::launch::serve(serve)
        .await
        .map(|()| 0)
        .map_err(|error| {
            // ServeError exposes approved static diagnostics only. Arbitrary
            // config, URL, credential and Debug text never cross this boundary.
            json_diagnostic(json!({"schemaVersion":1,"diagnostic":{
                "code":error.code,"message":error.diagnostic
            }}));
            Failure::new(error.code, UNAVAILABLE)
        })
}
fn early_output_format(arguments: &[OsString]) -> OutputFormat {
    let value = arguments
        .windows(2)
        .find(|pair| pair[0] == "--format")
        .and_then(|pair| pair[1].to_str())
        .or_else(|| {
            arguments
                .iter()
                .filter_map(|a| a.to_str())
                .find_map(|v| v.strip_prefix("--format="))
        });
    if value == Some("human") {
        OutputFormat::Human
    } else {
        OutputFormat::Json
    }
}
fn remote_setup_exit(code: &str) -> u8 {
    match code {
        "invalidEndpoint" | "credentialSource" => INPUT,
        "credentialsRequired" | "credentialFormat" => AUTHORIZATION,
        _ => UNAVAILABLE,
    }
}
fn localized_command(command: clap::Command, locale: UiLocale) -> clap::Command {
    let mut template = format!(
        "{{before-help}}{{name}} {{version}}\n{{about-with-newline}}\n{}: {{usage}}\n\n",
        UiMessage::cli_usage_heading().render(locale)
    );
    if command.has_subcommands() {
        template.push_str(&format!(
            "{}:\n{{subcommands}}\n",
            UiMessage::cli_commands_heading().render(locale)
        ));
    }
    if command.get_arguments().any(|arg| arg.is_positional()) {
        template.push_str(&format!(
            "{}:\n{{positionals}}\n",
            UiMessage::cli_arguments_heading().render(locale)
        ));
    }
    template.push_str(&format!(
        "{}:\n{{options}}{{after-help}}\n",
        UiMessage::cli_options_heading().render(locale)
    ));
    let has_version = command.get_version().is_some();
    let command = command
        .help_template(template)
        .disable_help_subcommand(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            clap::Arg::new("help")
                .long("help")
                .short('h')
                .action(clap::ArgAction::Help)
                .help(UiMessage::cli_help_flag().render(locale)),
        )
        .mut_subcommands(|sub| localized_command(sub, locale));
    if has_version {
        command.arg(
            clap::Arg::new("version")
                .long("version")
                .short('V')
                .action(clap::ArgAction::Version)
                .help(UiMessage::cli_version_flag().render(locale)),
        )
    } else {
        command
    }
}

async fn execute(args: &Args, locale: UiLocale) -> Result<u8, Failure> {
    // Parse again from the original command only once through a non-owning
    // boundary: prepare consumes its DTO, not a body that can later change.
    let command = args.command.clone();
    let prepared = prepare::prepare(command, args.endpoint.is_some())
        .map_err(|e| Failure::new(io_code(&e), INPUT))?;
    if let Some(endpoint) = &args.endpoint {
        remote::validate_endpoint(endpoint, args.allow_loopback_http)
            .map_err(|e| Failure::new(e.code, INPUT))?;
        if let Action::Call(call) = &prepared.action {
            remote::validate_remote_value(call.kind, &call.body)
                .map_err(|e| Failure::new(e.code, INPUT))?;
        }
    }
    let mut outputs = Vec::new();
    for path in [&args.output, &prepared.record, &prepared.source_output]
        .into_iter()
        .flatten()
    {
        outputs.push(path.as_path());
    }
    if let Action::Download { destination, .. } = &prepared.action {
        outputs.push(destination);
    }
    input::distinct_outputs(&outputs).map_err(|e| Failure::new(io_code(&e), INPUT))?;
    let pending_secret = prepared
        .secret_source
        .as_deref()
        .map(remote::read_credential)
        .transpose()
        .map_err(|e| Failure::new(e.code, AUTHORIZATION))?
        .map(RemoteSyncSecret::new)
        .transpose()
        .map_err(|e| Failure::new(e.code, AUTHORIZATION))?;
    let mut context = Context::connect(args).await?;
    if prepared
        .expected_target
        .as_ref()
        .is_some_and(|target| target != &context.target)
    {
        return Err(Failure::new("requestTarget", CONFLICT));
    }
    for path in outputs {
        input::artifact_path(
            path,
            &[
                prepared.author_root.as_deref(),
                context.private_root.as_deref(),
            ],
        )
        .map_err(|e| Failure::new(io_code(&e), INPUT))?;
    }
    for prerequisite in &prepared.prerequisites {
        let (reply, value) = context.json(prerequisite.clone()).await?;
        if !(200..300).contains(&reply.status) {
            return Err(Failure::new(
                error_code(&value),
                outcome(prerequisite.kind, reply.status, &value, false),
            ));
        }
        if prerequisite.kind == CallKind::EditorSnapshot
            && (value["serial"] != prerequisite.body["serial"]
                || value["sourceHash"] != prerequisite.body["sourceHash"])
        {
            return Err(Failure::new("snapshotChanged", CONFLICT));
        }
    }
    let Prepared {
        action,
        record,
        author_root,
        source_output,
        resource,
        expected_artifact_digest,
        original_path,
        wait_seconds,
        ..
    } = prepared;
    let mut original_reference = None;
    let (kind, reply, value) = match action {
        Action::Call(call) => {
            let kind = call.kind;
            let secret = pending_secret;
            if kind == CallKind::ExportConfirm && !context.remote() {
                let (_, fact) = context
                    .json(BusinessCall {
                        kind: CallKind::ExportStatus,
                        body: json!({"requestId":call.body["id"]}),
                    })
                    .await?;
                if fact["digest"].as_str() != expected_artifact_digest.as_deref() {
                    return Err(Failure::new("artifactChanged", CONFLICT));
                }
            }
            let mut original = FrozenRequest::new(context.target.clone(), call)
                .map_err(|e| Failure::new(e, INPUT))?;
            original.resource = resource;
            original.author_root = author_root.clone();
            let record_path = if kind.is_effect() {
                let path = record
                    .as_deref()
                    .ok_or_else(|| Failure::new("requestFileRequired", INPUT))?;
                let saved = original
                    .persist(
                        path,
                        author_root.as_deref(),
                        context.private_root.as_deref(),
                    )
                    .map_err(|e| Failure::new(io_code(&e), INPUT))?;
                retained_diagnostic(args.format, locale, &original, &saved);
                Some(saved)
            } else {
                None
            };
            let wire = original.wire().map_err(|e| Failure::new(e, INPUT))?;
            if kind.is_effect() {
                original_reference = Some(
                    json!({"operationId":original.operation_id,"requestFile":record_path,"wireSha256":original.wire_sha256}),
                );
            }
            let attempted = context
                .call_with_secret(kind, &wire, &original.request_id, secret)
                .await;
            let (mut reply, mut value) =
                resolve_attempt(&mut context, &original, attempted, record_path.as_deref()).await?;
            if kind == CallKind::RemoteSyncStart && wait_seconds > 0 {
                let deadline = tokio::time::Instant::now() + Duration::from_secs(wait_seconds);
                while value["status"] == "running" && tokio::time::Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    (reply, value) =
                        context
                            .json(status_call(&original, false)?)
                            .await
                            .map_err(|e| {
                                Failure::new(e.code, UNKNOWN)
                                    .original(&original, record_path.as_deref())
                            })?;
                    if !valid_status_fact(&original, &value) {
                        return Err(Failure::new("replyIdentity", UNKNOWN)
                            .original(&original, record_path.as_deref()));
                    }
                }
            }
            (kind, reply, value)
        }
        Action::OriginalStatus(original) => {
            original_reference = Some(
                json!({"operationId":original.operation_id,"requestFile":original_path,"wireSha256":original.wire_sha256}),
            );
            let call = status_call(&original, context.remote())?;
            let kind = call.kind;
            let (reply, value) = context.json(call).await?;
            if (200..300).contains(&reply.status) && !valid_status_fact(&original, &value) {
                return Err(Failure::new("replyIdentity", UNKNOWN)
                    .original(&original, original_path.as_deref()));
            }
            (kind, reply, value)
        }
        Action::Settle(original) => {
            // First query. A known committed effect never becomes cancellation.
            let (observed, fact) = context
                .json(status_call(&original, context.remote())?)
                .await?;
            if !(200..300).contains(&observed.status) || !valid_status_fact(&original, &fact) {
                return Err(Failure::new(
                    "originalStatusUnavailable",
                    if observed.status == 401 || observed.status == 403 {
                        AUTHORIZATION
                    } else {
                        UNKNOWN
                    },
                )
                .original(&original, original_path.as_deref()));
            }
            if (200..300).contains(&observed.status) && is_committed(&fact) {
                original_reference = Some(
                    json!({"operationId":original.operation_id,"requestFile":original_path,"wireSha256":original.wire_sha256}),
                );
                (original.kind, observed, fact)
            } else {
                let call = settle_call(&original, context.remote())?;
                let kind = call.kind;
                let mut fence = FrozenRequest::new(context.target.clone(), call)
                    .map_err(|e| Failure::new(e, INPUT))?;
                fence.resource = original.resource.clone();
                fence.author_root = author_root.clone();
                let saved = fence
                    .persist(
                        record
                            .as_deref()
                            .ok_or_else(|| Failure::new("requestFileRequired", INPUT))?,
                        author_root.as_deref(),
                        context.private_root.as_deref(),
                    )
                    .map_err(|e| Failure::new(io_code(&e), INPUT))?;
                retained_diagnostic(args.format, locale, &fence, &saved);
                original_reference = Some(
                    json!({"operationId":fence.operation_id,"requestFile":saved,"wireSha256":fence.wire_sha256}),
                );
                let wire = fence.wire().map_err(|e| Failure::new(e, INPUT))?;
                let attempted = context.call(kind, &wire, &fence.request_id).await;
                let (reply, value) =
                    resolve_attempt(&mut context, &fence, attempted, Some(&saved)).await?;
                (kind, reply, value)
            }
        }
        Action::Download {
            request,
            destination,
        } => {
            let receipt = download(&mut context, &request, &destination)
                .await
                .map_err(|e| e.original(&request, original_path.as_deref()))?;
            original_reference = Some(
                json!({"operationId":request.operation_id,"requestFile":original_path,"wireSha256":request.wire_sha256}),
            );
            let bytes =
                serde_json::to_vec(&receipt).map_err(|_| Failure::new("replyJson", DELIVERY))?;
            (
                CallKind::ExportDownload,
                Reply {
                    status: 200,
                    content_type: "application/json".into(),
                    body: bytes,
                },
                receipt,
            )
        }
    };
    let exit = outcome(kind, reply.status, &value, args.allow_partial);
    if let Some(path) = source_output
        && exit == 0
    {
        let source = value["source"]
            .as_str()
            .ok_or_else(|| Failure::new("sourceUnavailable", DELIVERY))?;
        let delivered_root = if context.remote() {
            None
        } else {
            prepare::author_root(&value)
        };
        let actual_root = delivered_root.as_deref().or(author_root.as_deref());
        let path = input::artifact_path(&path, &[actual_root, context.private_root.as_deref()])
            .map_err(|e| Failure::new(io_code(&e), DELIVERY))?;
        input::write_new(&path, source.as_bytes())
            .map_err(|e| Failure::new(io_code(&e), DELIVERY))?;
    }
    let envelope = Envelope {
        schema_version: 1,
        target: &context.target,
        call: kind,
        http_status: reply.status,
        owner_incarnation: context.owner_incarnation.as_deref(),
        owner_identity: context.owner_identity.as_ref(),
        result: &value,
        original_request: original_reference.as_ref(),
    };
    let bytes = if args.format == OutputFormat::Human {
        let message = match outcome(kind, reply.status, &value, false) {
            0 => UiMessage::cli_complete().render(locale),
            PARTIAL => UiMessage::cli_partial().render(locale),
            DELIVERY => {
                UiMessage::cli_delivery_failed(weftext_messages::params::CliDeliveryFailed {
                    operation_id: value["operationId"]
                        .as_str()
                        .or_else(|| value["requestId"].as_str())
                        .unwrap_or("")
                        .into(),
                    path: record
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                })
                .render(locale)
            }
            UNKNOWN => UiMessage::cli_unknown(weftext_messages::params::CliUnknown {
                operation_id: value["operationId"].as_str().unwrap_or("").into(),
                path: record
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            })
            .render(locale),
            _ => UiMessage::cli_error(weftext_messages::params::CliError {
                code: error_code(&value).into(),
            })
            .render(locale),
        };
        let mut bytes = message.into_bytes();
        bytes.push(b'\n');
        bytes.extend(
            serde_json::to_vec_pretty(&envelope)
                .map_err(|_| Failure::new("replyJson", DELIVERY))?,
        );
        bytes.push(b'\n');
        bytes
    } else {
        let mut bytes =
            serde_json::to_vec(&envelope).map_err(|_| Failure::new("replyJson", DELIVERY))?;
        bytes.push(b'\n');
        bytes
    };
    let delivered_root = if context.remote() {
        None
    } else {
        prepare::author_root(&value)
    };
    match &args.output {
        Some(path) => input::artifact_path(
            path,
            &[
                author_root.as_deref(),
                delivered_root.as_deref(),
                context.private_root.as_deref(),
            ],
        )
        .and_then(|path| input::write_new(&path, &bytes)),
        None => io::stdout().lock().write_all(&bytes),
    }
    .map_err(|e| {
        let mut failure = Failure::new(
            if e.kind() == io::ErrorKind::BrokenPipe {
                "brokenPipe"
            } else {
                "outputDelivery"
            },
            DELIVERY,
        );
        if let Some(reference) = &original_reference {
            failure.original_id = reference["operationId"].as_str().map(str::to_owned);
            failure.request_path = reference["requestFile"].as_str().map(str::to_owned);
        }
        failure
    })?;
    Ok(exit)
}

async fn resolve_attempt(
    context: &mut Context,
    original: &FrozenRequest,
    attempted: Result<Reply, Failure>,
    path: Option<&Path>,
) -> Result<(Reply, Value), Failure> {
    let parsed = attempted.and_then(|reply| {
        let exit = if original.kind.is_effect() {
            UNKNOWN
        } else {
            UNAVAILABLE
        };
        if !reply
            .content_type
            .split(';')
            .next()
            .is_some_and(|s| s.trim() == "application/json")
        {
            return Err(Failure::new("replyType", exit));
        }
        let body = input::parse_json(&reply.body).map_err(|_| Failure::new("replyJson", exit))?;
        if (200..300).contains(&reply.status) && !valid_result_identity(original, &body) {
            return Err(Failure::new("replyIdentity", exit));
        }
        Ok((reply, body))
    });
    match &parsed {
        Ok((reply, body))
            if (200..300).contains(&reply.status)
                || !original.kind.is_effect()
                || reply.status < 500 && !uncertain_error(body) =>
        {
            return parsed;
        }
        Err(error) if error.exit != UNKNOWN => {
            return parsed.map_err(|e| e.original(original, path));
        }
        _ => {}
    }
    if original.kind.is_effect()
        && let Ok(call) = status_call(original, context.remote())
        && let Ok((reply, fact)) = context.json(call).await
        && (200..300).contains(&reply.status)
        && valid_result_identity(original, &fact)
        && known_original_result(original.kind, &fact)
    {
        return Ok((reply, fact));
    }
    match parsed {
        Ok((reply, body)) if reply.status == 401 || reply.status == 403 => Ok((reply, body)),
        Ok((_, body)) => Err(Failure::new(error_code(&body), UNKNOWN).original(original, path)),
        Err(error) => Err(error.original(original, path)),
    }
}

fn valid_result_identity(original: &FrozenRequest, value: &Value) -> bool {
    if (!original.kind.is_effect()
        && !matches!(
            original.kind,
            CallKind::OperationStatus
                | CallKind::StructureStatus
                | CallKind::HistoryStatus
                | CallKind::ExportStatus
                | CallKind::ImportStatus
                | CallKind::OfficeImportStatus
                | CallKind::RemoteSyncJob
        ))
        || matches!(
            original.kind,
            CallKind::SyncEnable | CallKind::SyncRevalidationReview
        )
    {
        return true;
    }
    match original.operation_id.as_deref() {
        Some(expected) => {
            let matches_id = value["operationId"]
                .as_str()
                .or_else(|| value["requestId"].as_str())
                .or_else(|| value["id"].as_str())
                == Some(expected);
            matches_id
                && (original.kind != CallKind::SyncCancel
                    || original
                        .body()
                        .is_ok_and(|body| value["planId"] == body["planId"]))
        }
        None => true,
    }
}
fn valid_status_fact(original: &FrozenRequest, value: &Value) -> bool {
    valid_result_identity(original, value)
        && matches!(
            value["status"].as_str(),
            Some(
                "prepared"
                    | "installing"
                    | "committed"
                    | "unknown"
                    | "resolved"
                    | "pending"
                    | "failed"
                    | "expired"
                    | "notRegistered"
                    | "notCommitted"
                    | "cancelled"
                    | "saved"
                    | "running"
                    | "complete"
                    | "stopped"
                    | "confirmed"
                    | "recoveryPending"
                    | "unavailable"
                    | "blocked"
            )
        )
}

fn status_call(original: &FrozenRequest, remote: bool) -> Result<BusinessCall, Failure> {
    let kind = original
        .kind
        .original_status(remote)
        .ok_or_else(|| Failure::new("originalStatusUnavailable", UNKNOWN))?;
    let source = original.body().map_err(|e| Failure::new(e, INPUT))?;
    let id = original
        .operation_id
        .as_deref()
        .or_else(|| source["requestId"].as_str())
        .or_else(|| source["id"].as_str())
        .ok_or_else(|| Failure::new("originalIdUnavailable", UNKNOWN))?;
    let field = if matches!(kind, CallKind::ExportStatus | CallKind::RemoteSyncJob) {
        "requestId"
    } else {
        "operationId"
    };
    let mut body = json!({field:id});
    if remote
        && matches!(
            kind,
            CallKind::OperationStatus
                | CallKind::ImportStatus
                | CallKind::OfficeImportStatus
                | CallKind::StructureStatus
                | CallKind::HistoryStatus
        )
    {
        selector(original, &source, &mut body)?;
    }
    Ok(BusinessCall { kind, body })
}

fn settle_call(original: &FrozenRequest, remote: bool) -> Result<BusinessCall, Failure> {
    let id = original
        .operation_id
        .as_deref()
        .ok_or_else(|| Failure::new("originalIdUnavailable", UNKNOWN))?;
    let source = original.body().map_err(|e| Failure::new(e, INPUT))?;
    if !remote {
        let request_kind = original
            .kind
            .fence_kind()
            .ok_or_else(|| Failure::new("originalFenceUnavailable", UNKNOWN))?;
        return Ok(BusinessCall {
            kind: CallKind::OperationSettle,
            body: json!({"operationId":id,"requestKind":request_kind,"wireDigest":original.wire_sha256}),
        });
    }
    let kind = match original.kind {
        CallKind::Save => CallKind::OperationSettle,
        CallKind::OperationPrepare | CallKind::OperationCommit => CallKind::StructureSettle,
        CallKind::ImportPrepare | CallKind::ImportCommit => CallKind::ImportSettle,
        CallKind::OfficeImportPrepare | CallKind::OfficeImportCommit => {
            CallKind::OfficeImportSettle
        }
        CallKind::HistoryRestore => CallKind::HistorySettle,
        _ => return Err(Failure::new("originalFenceUnavailable", UNKNOWN)),
    };
    let mut body = json!({"operationId":id});
    selector(original, &source, &mut body)?;
    if kind == CallKind::OfficeImportSettle && source["format"].is_string() {
        body["format"] = source["format"].clone();
    }
    Ok(BusinessCall { kind, body })
}

fn selector(original: &FrozenRequest, source: &Value, target: &mut Value) -> Result<(), Failure> {
    if let Some(resource) = &original.resource {
        target["resource"] = json!(resource);
        Ok(())
    } else {
        prepare::copy_selector(source, target)
            .map_err(|_| Failure::new("originalSelector", UNKNOWN))
    }
}

fn is_committed(value: &Value) -> bool {
    matches!(
        value["status"].as_str(),
        Some("saved" | "committed" | "complete" | "success")
    )
}
fn error_code(value: &Value) -> &str {
    value["error"]["code"]
        .as_str()
        .or_else(|| value["documentError"]["code"].as_str())
        .or_else(|| value["resultError"]["code"].as_str())
        .or_else(|| value["recoveryIssue"]["code"].as_str())
        .or_else(|| value["code"].as_str())
        .filter(|code| safe_code(code))
        .unwrap_or("commandFailed")
}
fn safe_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 80
        && code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn uncertain_error(value: &Value) -> bool {
    matches!(
        error_code(value),
        "unknown" | "recoveryPending" | "resultUnknown" | "committedResultUnavailable"
    )
}
fn known_original_result(kind: CallKind, value: &Value) -> bool {
    use CallKind::*;
    let status = value["status"].as_str();
    match kind {
        OperationPrepare | ImportPrepare | OfficeImportPrepare => {
            matches!(status, Some("prepared" | "committed"))
        }
        ExportPrepare => matches!(status, Some("prepared" | "confirmed")),
        ExportConfirm => status == Some("confirmed"),
        RemoteSyncStart => matches!(status, Some("running" | "complete")),
        RemoteSyncStop => matches!(status, Some("running" | "stopped" | "complete")),
        OperationSettle | StructureSettle | ImportSettle | OfficeImportSettle | HistorySettle => {
            matches!(
                status,
                Some("committed" | "saved" | "resolved" | "cancelled" | "notCommitted")
            )
        }
        SyncCancel => matches!(status, Some("committed" | "resolved" | "cancelled")),
        _ => is_committed(value),
    }
}
fn outcome(kind: CallKind, status: u16, value: &Value, allow_partial: bool) -> u8 {
    if status == 401 || status == 403 {
        return AUTHORIZATION;
    }
    if uncertain_error(value) {
        return UNKNOWN;
    }
    if !(200..300).contains(&status) {
        return if status == 409 { CONFLICT } else { UNAVAILABLE };
    }
    if matches!(
        value["status"].as_str(),
        Some(
            "unknown" | "notRegistered" | "recoveryPending" | "pending" | "running" | "installing"
        )
    ) {
        return UNKNOWN;
    }
    if kind == CallKind::RemoteSyncStart && value["status"] == "stopped" {
        return CONFLICT;
    }
    if kind == CallKind::SyncCancel && value["status"] == "committed" {
        return CONFLICT;
    }
    if matches!(
        value["status"].as_str(),
        Some("failed" | "expired" | "unavailable" | "blocked")
    ) || value.get("error").is_some_and(|value| !value.is_null())
    {
        return CONFLICT;
    }
    if kind.is_effect()
        && matches!(
            value["status"].as_str(),
            Some("cancelled" | "notCommitted" | "resolved")
        )
        && !matches!(
            kind,
            CallKind::OperationSettle
                | CallKind::StructureSettle
                | CallKind::ImportSettle
                | CallKind::OfficeImportSettle
                | CallKind::HistorySettle
                | CallKind::SyncCancel
                | CallKind::RecoveryResolve
        )
    {
        return CONFLICT;
    }
    if matches!(
        kind,
        CallKind::Save
            | CallKind::SaveAs
            | CallKind::NoteCreate
            | CallKind::OperationCommit
            | CallKind::HistoryRestore
            | CallKind::ImportCommit
            | CallKind::OfficeImportCommit
            | CallKind::OperationPrepare
            | CallKind::ImportPrepare
            | CallKind::OfficeImportPrepare
            | CallKind::ExportPrepare
            | CallKind::ExportConfirm
            | CallKind::OperationSettle
            | CallKind::StructureSettle
            | CallKind::ImportSettle
            | CallKind::OfficeImportSettle
            | CallKind::HistorySettle
            | CallKind::SyncCancel
    ) && !known_original_result(kind, value)
    {
        return UNKNOWN;
    }
    if value
        .get("documentError")
        .is_some_and(|value| !value.is_null())
        || value
            .get("resultError")
            .is_some_and(|value| !value.is_null())
        || value["resultStatus"] == "unavailable"
    {
        return DELIVERY;
    }
    if value
        .get("recoveryIssue")
        .is_some_and(|value| !value.is_null())
        && !allow_partial
    {
        return PARTIAL;
    }
    if !allow_partial
        && (value["partial"] == true
            || value["coverage"] == "partial"
            || value["status"] == "partial"
            || value["result"]["coverage"] == "partial"
            || value["complete"] == false
            || value["limited"] == true
            || value["result"]["complete"] == false
            || value["result"]["limited"] == true
            || value["result"]["partial"] == true)
    {
        return PARTIAL;
    }
    0
}
fn io_code(error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::AlreadyExists {
        return "outputExists".into();
    }
    let text = error.to_string();
    if text.len() <= 80 && text.bytes().all(|b| b.is_ascii_alphanumeric()) {
        text
    } else {
        "inputIo".into()
    }
}
fn diagnostic(text: String) {
    let _ = writeln!(io::stderr().lock(), "{text}");
}
fn json_diagnostic(value: Value) {
    if let Ok(mut bytes) = serde_json::to_vec(&value) {
        bytes.push(b'\n');
        let _ = io::stderr().lock().write_all(&bytes);
    }
}
fn retained_diagnostic(format: OutputFormat, locale: UiLocale, input: &FrozenRequest, path: &Path) {
    let operation_id = input.operation_id.as_deref().unwrap_or(&input.request_id);
    if format == OutputFormat::Json {
        json_diagnostic(json!({
            "schemaVersion":1,"event":"requestRetained",
            "operationId":operation_id,"requestFile":path,
        }));
    } else {
        diagnostic(
            UiMessage::cli_request_retained(weftext_messages::params::CliRequestRetained {
                operation_id: operation_id.into(),
                path: path.display().to_string(),
            })
            .render(locale),
        );
    }
}
fn failure_document(failure: &Failure) -> Value {
    let mut error = json!({"code":failure.code,"exitCode":failure.exit});
    if let Some(id) = &failure.original_id {
        error["operationId"] = json!(id);
    }
    if let Some(path) = &failure.request_path {
        error["requestFile"] = json!(path);
    }
    json!({"schemaVersion":1,"error":error})
}
fn report_failure(failure: &Failure, locale: UiLocale, format: OutputFormat) {
    if format == OutputFormat::Json {
        json_diagnostic(failure_document(failure));
        return;
    }
    let message = if failure.exit == UNKNOWN {
        UiMessage::cli_unknown(weftext_messages::params::CliUnknown {
            operation_id: failure.original_id.clone().unwrap_or_default(),
            path: failure.request_path.clone().unwrap_or_default(),
        })
    } else if failure.exit == DELIVERY {
        UiMessage::cli_delivery_failed(weftext_messages::params::CliDeliveryFailed {
            operation_id: failure.original_id.clone().unwrap_or_default(),
            path: failure.request_path.clone().unwrap_or_default(),
        })
    } else if failure.code == "invalidArguments" {
        UiMessage::cli_invalid_arguments()
    } else {
        UiMessage::cli_error(weftext_messages::params::CliError {
            code: failure.code.clone(),
        })
    };
    diagnostic(message.render(locale));
}

async fn download(
    context: &mut Context,
    original: &FrozenRequest,
    destination: &Path,
) -> Result<Value, Failure> {
    let destination = input::artifact_path(
        destination,
        &[
            original.author_root.as_deref(),
            context.private_root.as_deref(),
        ],
    )
    .map_err(|e| Failure::new(io_code(&e), DELIVERY))?;
    let source = original.body().map_err(|e| Failure::new(e, INPUT))?;
    let id = original
        .operation_id
        .as_deref()
        .or_else(|| source["requestId"].as_str())
        .or_else(|| source["id"].as_str())
        .ok_or_else(|| Failure::new("originalIdUnavailable", UNKNOWN))?;
    let (status, fact) = context
        .json(BusinessCall {
            kind: CallKind::ExportStatus,
            body: json!({"requestId":id}),
        })
        .await?;
    if !(200..300).contains(&status.status) {
        return Err(Failure::new(
            error_code(&fact),
            outcome(CallKind::ExportStatus, status.status, &fact, false),
        ));
    }
    if fact["requestId"].as_str().or_else(|| fact["id"].as_str()) != Some(id) {
        return Err(Failure::new("replyIdentity", DELIVERY));
    }
    let expected = fact["digest"]
        .as_str()
        .filter(|v| v.len() == 64)
        .ok_or_else(|| Failure::new("artifactDigest", DELIVERY))?;
    let length = fact["length"]
        .as_u64()
        .filter(|v| *v <= 64 * 1024 * 1024)
        .ok_or_else(|| Failure::new("artifactBudget", DELIVERY))? as usize;
    let mut bytes = Vec::with_capacity(length);
    if context.remote() {
        use base64::Engine as _;
        loop {
            let offset = bytes.len();
            let (reply, chunk) = context
                .json(BusinessCall {
                    kind: CallKind::ExportDownload,
                    body: json!({"requestId":id,"offset":offset}),
                })
                .await?;
            if !(200..300).contains(&reply.status) {
                return Err(Failure::new(
                    error_code(&chunk),
                    outcome(CallKind::ExportDownload, reply.status, &chunk, false),
                ));
            }
            if chunk["requestId"].as_str() != Some(id) {
                return Err(Failure::new("replyIdentity", DELIVERY));
            }
            let encoded = chunk["bytesBase64"]
                .as_str()
                .filter(|v| v.len() <= 65536 * 4 / 3 + 4)
                .ok_or_else(|| Failure::new("artifactChunk", DELIVERY))?;
            let part = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|_| Failure::new("artifactChunk", DELIVERY))?;
            if chunk["offset"].as_u64() != Some(offset as u64)
                || chunk["total"].as_u64() != Some(length as u64)
                || chunk["length"].as_u64() != Some(part.len() as u64)
                || chunk["sha256"].as_str() != Some(expected)
                || part.is_empty() && length != 0
                || part.len() > 65536
                || part.len() > length - offset
                || chunk["done"].as_bool() != Some(offset + part.len() == length)
            {
                return Err(Failure::new("artifactChunk", DELIVERY));
            }
            bytes.extend_from_slice(&part);
            if bytes.len() == length {
                break;
            }
        }
    } else {
        let wire =
            serde_json::to_vec(&json!({"id":id})).map_err(|_| Failure::new("inputJson", INPUT))?;
        let reply = context
            .call(
                CallKind::ExportDownload,
                &wire,
                &uuid::Uuid::new_v4().to_string(),
            )
            .await?;
        if reply.status != 200 {
            return Err(Failure::new("artifactDelivery", DELIVERY));
        }
        bytes = reply.body;
    }
    if bytes.len() != length || input::digest(&bytes) != expected {
        return Err(Failure::new("artifactDigest", DELIVERY));
    }
    input::write_new(&destination, &bytes).map_err(|error| {
        Failure::new(io_code(&error), DELIVERY).original(original, Some(&destination))
    })?;
    Ok(
        json!({"requestId":id,"status":"delivered","destination":destination,"length":length,"sha256":expected}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn machine_diagnostics_keep_stable_codes_and_original_input_locators() {
        let mut failure = Failure::new("timeout", UNKNOWN);
        failure.original_id = Some("original-id".into());
        failure.request_path = Some("C:\\working\\original.weftext-request.json".into());
        assert_eq!(
            failure_document(&failure),
            json!({"schemaVersion":1,"error":{
                "code":"timeout","exitCode":6,"operationId":"original-id",
                "requestFile":"C:\\working\\original.weftext-request.json"
            }})
        );
        assert_eq!(
            failure_document(&Failure::new("unsafe diagnostic with spaces", INPUT)),
            json!({"schemaVersion":1,"error":{"code":"commandFailed","exitCode":2}})
        );
        for (arguments, expected) in [
            (
                vec!["weftext-cli", "--format", "human"],
                OutputFormat::Human,
            ),
            (vec!["weftext-cli", "--format=human"], OutputFormat::Human),
            (vec!["weftext-cli", "--format=invalid"], OutputFormat::Json),
            (vec!["weftext-cli"], OutputFormat::Json),
        ] {
            assert_eq!(
                early_output_format(
                    &arguments
                        .into_iter()
                        .map(OsString::from)
                        .collect::<Vec<_>>()
                ),
                expected
            );
        }
    }
    fn original(kind: CallKind, remote: bool) -> FrozenRequest {
        FrozenRequest::new(
            if remote {
                Target::Remote {
                    origin: "https://notes.test/".into(),
                    actor: "alice".into(),
                    deployment: "server".into(),
                }
            } else {
                Target::Local {
                    profile_identity: "profile".into(),
                }
            },
            BusinessCall {
                kind,
                body: json!({"operationId":uuid::Uuid::new_v4().to_string(),"resource":"root"}),
            },
        )
        .unwrap()
    }
    #[test]
    fn actual_core_unknown_error_is_not_an_ordinary_conflict() {
        assert_eq!(
            outcome(
                CallKind::OperationCommit,
                409,
                &json!({"error":{"code":"unknown"}}),
                false
            ),
            UNKNOWN
        );
        assert_eq!(
            outcome(
                CallKind::OperationCommit,
                409,
                &json!({"error":{"code":"operationConflict"}}),
                false
            ),
            CONFLICT
        );
        assert!(!uncertain_error(
            &json!({"error":{"code":"operationConflict"}})
        ));
    }
    #[test]
    fn preparation_confirmation_and_committing_have_distinct_success_facts() {
        assert!(known_original_result(
            CallKind::ExportPrepare,
            &json!({"status":"prepared"})
        ));
        assert!(known_original_result(
            CallKind::ExportConfirm,
            &json!({"status":"confirmed"})
        ));
        assert!(!known_original_result(
            CallKind::OperationCommit,
            &json!({"status":"prepared"})
        ));
        assert_eq!(
            outcome(
                CallKind::ExportConfirm,
                200,
                &json!({"status":"expired"}),
                false
            ),
            CONFLICT
        );
        assert_eq!(
            outcome(CallKind::Save, 200, &json!({"status":"cancelled"}), false),
            CONFLICT
        );
        assert_eq!(
            outcome(
                CallKind::OperationSettle,
                200,
                &json!({"status":"resolved"}),
                false
            ),
            0
        );
        assert_eq!(
            outcome(
                CallKind::OperationSettle,
                200,
                &json!({"status":"prepared"}),
                false
            ),
            UNKNOWN
        );
    }
    #[test]
    fn original_fact_requires_both_exact_identity_and_a_known_state() {
        let request = original(CallKind::Save, false);
        assert!(!valid_status_fact(&request, &json!({})));
        assert!(!valid_status_fact(
            &request,
            &json!({"operationId":uuid::Uuid::new_v4().to_string(),"status":"committed"})
        ));
        assert!(!valid_status_fact(
            &request,
            &json!({"operationId":request.operation_id})
        ));
        assert!(valid_status_fact(
            &request,
            &json!({"operationId":request.operation_id,"status":"committed"})
        ));
    }
    #[test]
    fn sync_cancel_reports_an_already_committed_sync_without_claiming_cancellation() {
        assert_eq!(
            outcome(
                CallKind::SyncCancel,
                200,
                &json!({"status":"committed"}),
                false
            ),
            CONFLICT
        );
        assert_eq!(
            outcome(
                CallKind::SyncCancel,
                200,
                &json!({"status":"resolved"}),
                false
            ),
            0
        );
        let id = uuid::Uuid::new_v4().to_string();
        let request = FrozenRequest::new(
            Target::Local {
                profile_identity: "profile".into(),
            },
            BusinessCall {
                kind: CallKind::SyncCancel,
                body: json!({"operationId":id,"planId":"original-plan"}),
            },
        )
        .unwrap();
        assert!(valid_result_identity(
            &request,
            &json!({"operationId":id,"planId":"original-plan","status":"resolved"})
        ));
        assert!(!valid_result_identity(
            &request,
            &json!({"operationId":id,"planId":"different-plan","status":"resolved"})
        ));
    }
    #[test]
    fn export_status_cannot_return_a_different_artifact_request() {
        let expected = uuid::Uuid::new_v4().to_string();
        let request = FrozenRequest::new(
            Target::Local {
                profile_identity: "profile".into(),
            },
            BusinessCall {
                kind: CallKind::ExportStatus,
                body: json!({"requestId":expected}),
            },
        )
        .unwrap();
        assert!(!valid_result_identity(
            &request,
            &json!({"requestId":uuid::Uuid::new_v4().to_string(),"status":"prepared"})
        ));
        assert!(valid_result_identity(
            &request,
            &json!({"requestId":expected,"status":"prepared"})
        ));
    }
    #[test]
    fn a_committed_effect_can_have_independent_delivery_or_recovery_failures() {
        let fact = json!({"status":"committed","documentError":{"code":"unavailable"}});
        assert_eq!(
            outcome(CallKind::OperationCommit, 200, &fact, false),
            DELIVERY
        );
        assert_eq!(fact["status"], "committed");
        assert_eq!(
            outcome(
                CallKind::Save,
                200,
                &json!({"status":"committed","recoveryIssue":{"code":"recoveryReadFailed"}}),
                false
            ),
            PARTIAL
        );
        assert_eq!(
            outcome(
                CallKind::Save,
                200,
                &json!({"status":"saved","recoveryIssue":null}),
                false
            ),
            0
        );
    }
    #[test]
    fn partial_query_keeps_coverage_and_requires_an_explicit_exit_override() {
        let fact = json!({"complete":false,"rows":[],"coverage":"partial"});
        assert_eq!(outcome(CallKind::QueryRun, 200, &fact, false), PARTIAL);
        assert_eq!(outcome(CallKind::QueryRun, 200, &fact, true), 0);
        assert_eq!(fact["coverage"], "partial");
    }
    #[test]
    fn local_fence_uses_original_exact_wire_and_does_not_make_a_new_operation() {
        let request = original(CallKind::Save, false);
        let call = settle_call(&request, false).ok().unwrap();
        assert_eq!(call.kind, CallKind::OperationSettle);
        assert_eq!(
            call.body["operationId"].as_str(),
            request.operation_id.as_deref()
        );
        assert_eq!(call.body["requestKind"], "save");
        assert_eq!(call.body["wireDigest"], request.wire_sha256);
    }
    #[test]
    fn remote_status_and_fence_remain_in_the_original_business_family() {
        let request = original(CallKind::OperationPrepare, true);
        let status = status_call(&request, true).ok().unwrap();
        let fence = settle_call(&request, true).ok().unwrap();
        assert_eq!(status.kind, CallKind::StructureStatus);
        assert_eq!(status.body["resource"], "root");
        assert_eq!(fence.kind, CallKind::StructureSettle);
        assert_eq!(fence.body["resource"], "root");
    }
    #[test]
    fn remote_save_status_and_settlement_keep_the_original_out_of_wire_selector() {
        let mut request = FrozenRequest::new(
            Target::Remote {
                origin: "https://notes.test/".into(),
                actor: "alice".into(),
                deployment: "server".into(),
            },
            BusinessCall {
                kind: CallKind::Save,
                body: json!({"operationId":uuid::Uuid::new_v4().to_string(),
                    "sessionId":"original-session","baseId":"original-base",
                    "source":"original authored text","definitionReservations":[]}),
            },
        )
        .unwrap();
        request.resource = Some("Shared".into());
        let wire = request.wire().unwrap();
        let record = serde_json::to_value(&request).unwrap();
        assert!(request.body().unwrap().get("resource").is_none());
        let status = status_call(&request, true).ok().unwrap();
        let fence = settle_call(&request, true).ok().unwrap();
        assert_eq!(status.kind, CallKind::OperationStatus);
        assert_eq!(fence.kind, CallKind::OperationSettle);
        let selector = json!({"resource":"Shared","operationId":request.operation_id});
        assert_eq!(status.body, selector);
        assert_eq!(fence.body, selector);
        assert_eq!(request.wire().unwrap(), wire);
        assert_eq!(serde_json::to_value(&request).unwrap(), record);
        request.resource = None;
        for failure in [
            status_call(&request, true).err().unwrap(),
            settle_call(&request, true).err().unwrap(),
        ] {
            assert_eq!(failure.code, "originalSelector");
            assert_eq!(failure.exit, UNKNOWN);
        }
        assert_eq!(request.wire().unwrap(), wire);
    }
    #[test]
    fn localized_help_keeps_machine_names_and_uses_the_canonical_catalog() {
        for locale in [UiLocale::ZhCn, UiLocale::EnUs] {
            let mut command = localized_command(Args::command(), locale);
            command.clone().debug_assert();
            let help = command.render_long_help().to_string();
            assert!(help.contains(&UiMessage::cli_usage_heading().render(locale)));
            assert!(help.contains("--endpoint"));
            assert!(help.contains("operation"));
        }
    }
}
