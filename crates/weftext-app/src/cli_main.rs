fn main() -> std::process::ExitCode {
    if let Some(result) = weftext_app::launch::dispatch_worker() {
        return if result.is_ok() {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => return std::process::ExitCode::FAILURE,
    };
    std::process::ExitCode::from(runtime.block_on(weftext_app::cli::run()))
}
