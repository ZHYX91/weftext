//! Opt-in real-pipe fixture, never a shipped worker/runtime.
#[cfg(target_os = "linux")]
fn main() {
    use std::io::Write;
    match std::env::args().nth(1).as_deref() {
        Some("--render-worker") => std::thread::sleep(std::time::Duration::from_secs(60)),
        Some("--notes-edit-worker") => {
            let block = [b'x'; 65536];
            let mut output = std::io::stdout().lock();
            loop {
                if output.write_all(&block).is_err() {
                    break;
                }
            }
        }
        _ => std::process::exit(90),
    }
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Linux controlled-worker test fixture only");
    std::process::exit(2);
}
