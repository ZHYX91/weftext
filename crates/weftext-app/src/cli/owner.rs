//! Explicit control of an already qualified owner, without starting a Core.
use super::{
    DELIVERY, Failure, INPUT, UNAVAILABLE, UNKNOWN,
    args::{Args, OutputFormat},
    input, io_code,
};
use crate::local_client::{self, OwnerIdentity, ProfileLocator, SubmissionEffect};
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    path::Path,
    time::Duration,
};
use weftext_messages::{UiLocale, UiMessage};

fn profile(args: &Args) -> Result<ProfileLocator, Failure> {
    if args.endpoint.is_some() {
        return Err(Failure::new("internalControlTarget", INPUT));
    }
    let profile = ProfileLocator {
        data_dir: args
            .data_dir
            .clone()
            .ok_or_else(|| Failure::new("profileRequired", INPUT))?,
    };
    if let Some(path) = &args.output {
        input::distinct_outputs(&[path]).map_err(|e| Failure::new(io_code(&e), INPUT))?;
        input::artifact_path(path, &[Some(&profile.data_dir)])
            .map_err(|e| Failure::new(io_code(&e), INPUT))?;
    }
    Ok(profile)
}
fn deliver(args: &Args, locale: UiLocale, value: Value, exit: u8) -> Result<u8, Failure> {
    let mut bytes = serde_json::to_vec(&value).map_err(|_| Failure::new("replyJson", DELIVERY))?;
    bytes.push(b'\n');
    if args.format == OutputFormat::Human {
        let message = if exit == 0 {
            UiMessage::cli_complete()
        } else {
            UiMessage::cli_error(weftext_messages::params::CliError {
                code: "ownerExit".into(),
            })
        };
        let mut text = message.render(locale).into_bytes();
        text.push(b'\n');
        text.extend(bytes);
        bytes = text;
    }
    match &args.output {
        Some(path) => input::write_new(path, &bytes),
        None => io::stdout().lock().write_all(&bytes),
    }
    .map_err(|_| Failure::new("outputDelivery", DELIVERY))?;
    Ok(exit)
}
pub(super) async fn status(args: &Args, locale: UiLocale) -> Result<u8, Failure> {
    let profile = profile(args)?;
    let identity = local_client::inspect_existing(&profile, Duration::from_secs(10))
        .await
        .map_err(|e| Failure::new(e.code, UNAVAILABLE))?;
    deliver(
        args,
        locale,
        json!({"schemaVersion":1,"call":"ownerStatus","ownerIdentity":identity}),
        0,
    )
}
pub(super) async fn stop(args: &Args, path: &Path, locale: UiLocale) -> Result<u8, Failure> {
    let profile = profile(args)?;
    let bytes = input::read_input(path, 16 * 1024).map_err(|e| Failure::new(io_code(&e), INPUT))?;
    let value = input::parse_json(&bytes).map_err(|e| Failure::new(io_code(&e), INPUT))?;
    let identity: OwnerIdentity = serde_json::from_value(if value["schemaVersion"] == 1 {
        value["ownerIdentity"].clone()
    } else {
        value
    })
    .map_err(|_| Failure::new("ownerIdentity", INPUT))?;
    let receipt = local_client::shutdown_existing(&profile, &identity, Duration::from_secs(30))
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
    let exit = if receipt.exit_code == 0 {
        0
    } else {
        UNAVAILABLE
    };
    deliver(
        args,
        locale,
        json!({"schemaVersion":1,"call":"ownerStop","result":receipt}),
        exit,
    )
}
