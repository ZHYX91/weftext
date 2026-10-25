//! Console hosted owner shares the protected lifecycle broker, but never the
//! local author bridge, desktop scheduler, tray, or a second Core.
use crate::{
    broker::{Command, windows::OwnerPipe},
    hosted::HostedService,
    launch::ServeError,
    local_client::OwnerRole,
};
use std::{
    io,
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
};
use tokio::sync::{mpsc, watch};

fn failed(code: &'static str, diagnostic: &'static str) -> ServeError {
    ServeError { code, diagnostic }
}

pub(crate) async fn run(data_dir: PathBuf, config: PathBuf) -> Result<(), ServeError> {
    crate::artifact_catalog::Catalog::get().map_err(|_| {
        failed(
            "artifactQualification",
            "This executable is not a complete qualified Weftext build",
        )
    })?;
    let profile = crate::work::spawn_blocking(move || crate::host_paths::qualify_core(&data_dir))
        .await
        .map_err(|_| {
            failed(
                "profileQualification",
                "The exact persistent profile could not be qualified",
            )
        })?
        .map_err(|_| {
            failed(
                "profileQualification",
                "The exact persistent profile could not be qualified",
            )
        })?;
    let data = profile.path().to_path_buf();
    let owner=OwnerPipe::acquire(profile,OwnerRole::Hosted).map_err(|_|failed("ownerUnavailable","This physical profile already has an owner or owner qualification failed; no second Core was opened"))?;
    let identity = owner.identity();
    let _owner_name = owner.retain_name().map_err(|_| {
        failed(
            "ownerLease",
            "The original owner name could not be retained through Core release",
        )
    })?;
    // Move the original leases into the actual worker: cancelling this wait
    // cannot release the elected name while Core is still being opened. The
    // result drops Core before these leases if no caller receives it.
    let (core, data, owner, _owner_name) = crate::work::spawn_blocking(move || {
        crate::host_environment::open_core(data)
            .map(|(core, data)| (core, data, owner, _owner_name))
    })
    .await
    .map_err(|_| failed("coreUnavailable", "The persistent Core could not be opened"))?
    .map_err(|_| failed("coreUnavailable", "The persistent Core could not be opened"))?;
    let service=HostedService::bind(core,config,data.join("legacy-office"),crate::compiler_worker::Transport::default()).await.map_err(|error|{
        if error.is::<crate::hosted::PairingMismatch>() {
            failed("syncPairingMismatch","Persisted sync pairing differs from configuration; existing grants cannot silently move to another family")
        }else {failed("hostedConfiguration","The configured hosted service could not be bound; check the operator configuration and selected profile")}
    })?;
    let url = service.origin().to_owned();
    let (stop_broker, broker_stopping) = watch::channel(false);
    let (stop_http, mut http_stopping) = watch::channel(false);
    let (requests, mut requested) = mpsc::channel(8);
    let mut broker = tokio::spawn(owner.serve(
        None,
        requests,
        Arc::new(AtomicUsize::new(0)),
        Arc::new(crate::broker::lifetime::ServiceContinuation::new(false)),
        broker_stopping,
    ));
    let mut serving = tokio::spawn(service.serve_until(async move {
        if !*http_stopping.borrow() {
            let _ = http_stopping.changed().await;
        }
    }));
    let mut broker_result = None;
    let mut http_result = None;
    let mut failure = crate::launch::write_ready(&url, &identity).err().map(|_| {
        failed(
            "readyOutput",
            "The service bound but its Ready output was not confirmed; shutdown was requested",
        )
    });
    if failure.is_none() {
        loop {
            tokio::select! {
                biased;
                result=&mut broker=>{broker_result=Some(result);failure=Some(failed("ownerStoppedUnexpectedly","The protected owner listener stopped unexpectedly"));break;}
                result=&mut serving=>{http_result=Some(result);failure=Some(failed("httpStoppedUnexpectedly","The hosted listener stopped unexpectedly"));break;}
                request=requested.recv()=>{
                    let Some(request)=request else {failure=Some(failed("ownerControlUnavailable","The protected lifecycle channel became unavailable"));break;};
                    match request.command {
                        Command::Shutdown=>{
                            // Acknowledge closing before joining the broker task
                            // that is currently delivering this very reply.
                            let _=request.reply.send(Ok(br#"{"accepted":true}"#.to_vec()));
                            break;
                        }
                        _=>{let _=request.reply.send(Err(io::Error::new(io::ErrorKind::PermissionDenied,"Hosted owner has no desktop author surface")));}
                    }
                }
                signal=tokio::signal::ctrl_c()=>{if signal.is_err(){failure=Some(failed("shutdownSignalUnavailable","The console shutdown signal could not be monitored"));}break;}
            }
        }
    }
    // No accepted HostRequest may remain waiting on an undropped receiver while
    // the owner joins its connections. Future sends fail, not hang or restart.
    requested.close();
    while let Ok(request) = requested.try_recv() {
        let _ = request
            .reply
            .send(Err(io::Error::other("Owner is already closing")));
    }
    drop(requested);
    let _ = stop_broker.send(true);
    let _ = stop_http.send(true);
    let broker_result = match broker_result {
        Some(result) => result,
        None => broker.await,
    };
    let http_result = match http_result {
        Some(result) => result,
        None => serving.await,
    };
    if !matches!(broker_result, Ok(Ok(()))) || !matches!(http_result, Ok(Ok(()))) {
        failure.get_or_insert(failed("shutdownUnknown","Accepted work or final Core release was not confirmed; the original operations remain authoritative"));
    }
    failure.map_or(Ok(()), Err)
}
