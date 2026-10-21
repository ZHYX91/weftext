fn main() {
    #[cfg(target_os = "linux")]
    let result = (|| {
        let mut args = std::env::args_os();
        args.next();
        let policy = args.next().ok_or_else(|| {
            weftext_worker_broker::Error::new(
                weftext_worker_broker::ErrorKind::Rejected,
                "usage: weftext-worker-broker /absolute/admin-policy.json",
            )
        })?;
        if args.next().is_some() {
            return Err(weftext_worker_broker::Error::new(
                weftext_worker_broker::ErrorKind::Rejected,
                "unexpected argument",
            ));
        }
        weftext_worker_broker::run_daemon(std::path::Path::new(&policy))
    })();
    #[cfg(not(target_os = "linux"))]
    let result: weftext_worker_broker::Result<()> = Err(weftext_worker_broker::Error::new(
        weftext_worker_broker::ErrorKind::Unsupported,
        "Linux kernel isolation required",
    ));
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
