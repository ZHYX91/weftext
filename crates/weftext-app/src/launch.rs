//! Console argument dispatch. Native hosts can bind `LocalService` directly.
use crate::{
    host_environment,
    local_service::{InitialDocument, LocalService, LocalServiceOptions},
};
use clap::Parser;
use std::{future::Future, io, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "weftext",
    version,
    about = "Weftext local Typst writing and knowledge tools"
)]
struct Args {
    /// Run the bounded hosted service with an operator-owned configuration.
    #[arg(long, conflicts_with_all = ["open", "root", "port"])]
    server_config: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    port: u16,
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long)]
    open: Option<PathBuf>,
    #[arg(long, requires = "open")]
    root: Option<PathBuf>,
    #[arg(long)]
    no_browser: bool,
    #[arg(long, hide = true, conflicts_with_all = ["notes_edit_worker", "server_config", "port", "data_dir", "open", "root", "no_browser"])]
    render_worker: bool,
    #[arg(long, hide = true, conflicts_with_all = ["render_worker", "server_config", "port", "data_dir", "open", "root", "no_browser"])]
    notes_edit_worker: bool,
}

enum Action {
    RenderWorker,
    NotesWorker,
    Hosted {
        data_dir: PathBuf,
        config: PathBuf,
    },
    Local {
        options: LocalServiceOptions,
        browser: bool,
    },
}

impl Args {
    fn action(self) -> io::Result<Action> {
        // Internal workers never resolve a user directory or acquire Core.
        if self.render_worker {
            return Ok(Action::RenderWorker);
        }
        if self.notes_edit_worker {
            return Ok(Action::NotesWorker);
        }
        let data_dir = match self.data_dir {
            Some(path) => path,
            None => default_data_dir()?,
        };
        if let Some(config) = self.server_config {
            return Ok(Action::Hosted { data_dir, config });
        }
        Ok(Action::Local {
            options: LocalServiceOptions {
                data_dir,
                port: self.port,
                initial_document: self.open.map(|path| InitialDocument {
                    path,
                    root: self.root,
                }),
            },
            browser: !self.no_browser,
        })
    }
}

/// Resolve the persistent host default without creating it. This does not grant
/// the current OS a supported Core backend; startup still checks that backend.
pub fn default_data_dir() -> io::Result<PathBuf> {
    host_environment::default_data_dir()
}

/// Run the installed console entry using the process arguments.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    match Args::parse().action()? {
        Action::RenderWorker => crate::worker_main().or_else(|error| {
            crate::write_render_worker_failure(error, &mut std::io::stdout().lock())
        }),
        Action::NotesWorker => crate::notes_worker_main().or_else(|error| {
            crate::write_render_worker_failure(error, &mut std::io::stdout().lock())
        }),
        Action::Hosted { data_dir, config } => {
            let (core, data_dir) = host_environment::open_core(data_dir)?;
            crate::hosted::serve(core, config, data_dir.join("legacy-office")).await
        }
        Action::Local { options, browser } => {
            let service = LocalService::bind(options).await?;
            println!(
                "Weftext {}\n{}\n关闭此进程停止本地服务。",
                env!("CARGO_PKG_VERSION"),
                service.browser_url()
            );
            let address = service.browser_url().to_owned();
            let handoff = async move {
                if browser && let Err(error) = host_environment::open_browser(&address).await {
                    eprintln!(
                        "Default browser handoff failed or is unconfirmed: {error}. The local service remains running at the address above; --no-browser skips this handoff."
                    );
                }
            };
            // Serve while the OS handles the UI request: a launcher must not
            // stall the very HTTP page the selected browser is trying to open.
            serve_with_handoff(
                service,
                async {
                    if let Err(error) = tokio::signal::ctrl_c().await {
                        eprintln!(
                            "Cannot monitor shutdown signal; stopping local service: {error}"
                        );
                    }
                },
                handoff,
            )
            .await?;
            Ok(())
        }
    }
}

async fn serve_with_handoff(
    service: LocalService,
    shutdown: impl Future<Output = ()> + Send + 'static,
    handoff: impl Future<Output = ()>,
) -> io::Result<()> {
    let serving = service.serve_until(shutdown);
    tokio::pin!(serving, handoff);
    tokio::select! {
        biased;
        () = &mut handoff => serving.await,
        result = &mut serving => {
            eprintln!("Local service stopped before browser handoff completed; its outcome remains unconfirmed and no retry was sent.");
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workers_cannot_be_combined_with_user_or_other_worker_modes() {
        for worker in ["--render-worker", "--notes-edit-worker"] {
            for other in [
                vec!["--data-dir", "state"],
                vec!["--port", "1234"],
                vec!["--open", "note.typ"],
                vec!["--server-config", "server.json"],
                vec!["--no-browser"],
            ] {
                let mut args = vec!["weftext", worker];
                args.extend(other);
                assert!(Args::try_parse_from(args).is_err());
            }
        }
        assert!(
            Args::try_parse_from(["weftext", "--render-worker", "--notes-edit-worker"]).is_err()
        );
        assert!(matches!(
            Args::try_parse_from(["weftext", "--render-worker"])
                .unwrap()
                .action()
                .unwrap(),
            Action::RenderWorker
        ));
        assert!(matches!(
            Args::try_parse_from(["weftext", "--notes-edit-worker"])
                .unwrap()
                .action()
                .unwrap(),
            Action::NotesWorker
        ));
    }

    #[test]
    fn root_is_never_ignored_and_hosted_mode_cannot_open_a_local_document() {
        assert!(Args::try_parse_from(["weftext", "--root", "library"]).is_err());
        assert!(
            Args::try_parse_from([
                "weftext",
                "--server-config",
                "server.json",
                "--open",
                "note.typ"
            ])
            .is_err()
        );
        assert!(
            Args::try_parse_from([
                "weftext",
                "--server-config",
                "server.json",
                "--port",
                "1234"
            ])
            .is_err()
        );
        let action = Args::try_parse_from([
            "weftext",
            "--no-browser",
            "--data-dir",
            "owned-state",
            "--open",
            "note.typ",
            "--root",
            "library",
        ])
        .unwrap()
        .action()
        .unwrap();
        let Action::Local { options, browser } = action else {
            panic!("wrong launch mode")
        };
        assert!(!browser);
        assert_eq!(options.data_dir, PathBuf::from("owned-state"));
        let initial = options.initial_document.unwrap();
        assert_eq!(initial.path, PathBuf::from("note.typ"));
        assert_eq!(initial.root, Some(PathBuf::from("library")));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn host_shutdown_does_not_wait_for_an_unfinished_browser_handoff() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("state");
        let service = LocalService::bind(LocalServiceOptions::new(data.clone()))
            .await
            .unwrap();
        let address = url::Url::parse(service.browser_url()).unwrap();
        let token = address
            .fragment()
            .unwrap()
            .strip_prefix("token=")
            .unwrap()
            .to_owned();
        let endpoint = format!("http://127.0.0.1:{}/api/bootstrap", address.port().unwrap());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let host = tokio::spawn(serve_with_handoff(
            service,
            async {
                let _ = stopped.await;
            },
            std::future::pending(),
        ));
        let client = reqwest::Client::new();
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            client.get(endpoint).header("x-weftext-token", token).send(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(response.status().is_success());
        drop(response);
        drop(client);
        stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), host)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let reopened = LocalService::bind(LocalServiceOptions::new(data))
            .await
            .unwrap();
        drop(reopened);
    }
}
