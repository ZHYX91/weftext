#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() -> std::process::ExitCode {
    if let Some(result) = weftext_app::launch::dispatch_worker() {
        return if result.is_ok() {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }
    #[cfg(windows)]
    let _winrt = match weftext_app::launch::ProcessWinRt::enter() {
        Ok(lifetime) => lifetime,
        Err(error) => {
            weftext_app::launch::report_desktop_error(&error);
            return std::process::ExitCode::FAILURE;
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => {
            weftext_app::launch::report_desktop_error(&weftext_app::launch::ServeError {
                code: "runtimeStartFailed",
                diagnostic: "The application runtime could not start",
                local_origin: None,
            });
            return std::process::ExitCode::FAILURE;
        }
    };
    match runtime.block_on(weftext_app::launch::desktop()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            weftext_app::launch::report_desktop_error(&error);
            eprintln!("{}: {}", error.code, error.diagnostic);
            std::process::ExitCode::FAILURE
        }
    }
}
