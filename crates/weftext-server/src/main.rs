use std::env;
use std::future::IntoFuture;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use weftext_server::{HttpSecurityConfig, ServerConfig, ServerState, app, validate_bind_address};

const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("weftext-server: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let arguments = parse_arguments()?;
    let bind = validate_bind_address(arguments.bind).map_err(|error| error.to_string())?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|error| format!("cannot bind loopback listener: {error}"))?;
    let local = listener
        .local_addr()
        .map_err(|error| format!("cannot inspect listener: {error}"))?;
    let http = arguments
        .public_origin
        .as_deref()
        .map_or_else(
            || Ok(HttpSecurityConfig::loopback(local)),
            |origin| HttpSecurityConfig::same_host_reverse_proxy(local, origin),
        )
        .map_err(|error| error.to_string())?;
    let mut config = ServerConfig::new(arguments.control_plane, http)
        .with_admin_permanent_delete(arguments.allow_admin_permanent_delete);
    if let Some(snapshot_parent) = arguments.trash_migration_snapshot_parent {
        config = config.with_trash_migration_snapshot_parent(snapshot_parent);
    }
    let state =
        ServerState::open(arguments.workspace, config).map_err(|error| error.to_string())?;
    println!("WEFTEXT_SERVER_LISTENING=http://{local}");
    println!(
        "WEFTEXT_SERVER_BOUNDARY={}",
        if state.reverse_proxy_secret_path().is_some() {
            "same-host-tls-reverse-proxy"
        } else {
            "direct-loopback"
        }
    );

    let (shutdown_sender, shutdown_receiver) = tokio::sync::watch::channel(false);
    let shutdown_state = state.clone();
    let _signal_task = tokio::spawn(async move {
        shutdown_signal().await;
        shutdown_state.begin_shutdown();
        shutdown_sender.send_replace(true);
    });
    let server_shutdown = shutdown_receiver.clone();
    let server = axum::serve(listener, app(state))
        .with_graceful_shutdown(wait_for_shutdown(server_shutdown))
        .into_future();
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => result.map_err(|error| format!("server failed: {error}")),
        () = wait_for_shutdown(shutdown_receiver) => {
            tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, &mut server)
                .await
                .map_err(|_| "graceful shutdown timed out after 30 seconds".to_owned())?
                .map_err(|error| format!("server failed while draining: {error}"))
        }
    }
}

struct Arguments {
    workspace: PathBuf,
    control_plane: PathBuf,
    bind: SocketAddr,
    public_origin: Option<String>,
    allow_admin_permanent_delete: bool,
    trash_migration_snapshot_parent: Option<PathBuf>,
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut arguments = env::args_os().skip(1);
    let workspace = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let mut bind = "127.0.0.1:8787"
        .parse::<SocketAddr>()
        .expect("static loopback address");
    let mut control_plane = None;
    let mut public_origin = None;
    let mut allow_admin_permanent_delete = false;
    let mut trash_migration_snapshot_parent = None;
    let mut bind_seen = false;
    while let Some(argument) = arguments.next() {
        if argument == "--bind" {
            if bind_seen {
                return Err(usage());
            }
            bind_seen = true;
            let value = arguments.next().ok_or_else(usage)?;
            bind = value.to_string_lossy().parse().map_err(|_| {
                "--bind must be an IP socket address such as 127.0.0.1:8787".to_owned()
            })?;
        } else if argument == "--control-plane" {
            if control_plane.is_some() {
                return Err(usage());
            }
            control_plane = Some(PathBuf::from(arguments.next().ok_or_else(usage)?));
        } else if argument == "--same-host-proxy-origin" {
            if public_origin.is_some() {
                return Err(usage());
            }
            public_origin = Some(
                arguments
                    .next()
                    .ok_or_else(usage)?
                    .into_string()
                    .map_err(|_| "--same-host-proxy-origin must be Unicode".to_owned())?,
            );
        } else if argument == "--allow-admin-permanent-delete" {
            if allow_admin_permanent_delete {
                return Err(usage());
            }
            allow_admin_permanent_delete = true;
        } else if argument == "--trash-migration-snapshot-parent" {
            if trash_migration_snapshot_parent.is_some() {
                return Err(usage());
            }
            trash_migration_snapshot_parent =
                Some(PathBuf::from(arguments.next().ok_or_else(usage)?));
        } else {
            return Err(usage());
        }
    }
    Ok(Arguments {
        workspace,
        control_plane: control_plane.ok_or_else(usage)?,
        bind,
        public_origin,
        allow_admin_permanent_delete,
        trash_migration_snapshot_parent,
    })
}

fn usage() -> String {
    "usage: weftext-server <workspace> --control-plane <directory> [--bind 127.0.0.1:8787] [--same-host-proxy-origin https://authority] [--allow-admin-permanent-delete] [--trash-migration-snapshot-parent <directory>]".to_owned()
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        if let Ok(mut terminate) = signal(SignalKind::terminate()) {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = terminate.recv() => {}
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

async fn wait_for_shutdown(mut receiver: tokio::sync::watch::Receiver<bool>) {
    if *receiver.borrow() {
        return;
    }
    while receiver.changed().await.is_ok() {
        if *receiver.borrow() {
            return;
        }
    }
}
