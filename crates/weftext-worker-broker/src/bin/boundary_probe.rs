//! Opt-in hostile test image, never a shipped worker/runtime.
#[cfg(target_os = "linux")]
fn main() {
    use serde_json::json;
    use std::io::{Read, Write};
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 || !["--render-worker", "--notes-edit-worker"].contains(&args[1].as_str()) {
        std::process::exit(90);
    }
    let mut bytes = Vec::new();
    std::io::stdin().take(4097).read_to_end(&mut bytes).unwrap();
    let input: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let cgroup = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let caps: Vec<_> = status
        .lines()
        .filter(|s| {
            s.starts_with("Cap")
                || s.starts_with("NoNewPrivs:")
                || s.starts_with("Groups:")
                || s.starts_with("Uid:")
                || s.starts_with("Gid:")
        })
        .collect();
    let network = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if network >= 0 {
        unsafe {
            libc::close(network);
        }
    }
    let core_denied = input["core"]
        .as_str()
        .is_some_and(|p| std::fs::read_dir(p).is_err());
    let key_denied = input["key"]
        .as_str()
        .is_some_and(|p| std::fs::read(p).is_err());
    let mut inherited = Vec::new();
    for fd in 3..128 {
        if unsafe { libc::fcntl(fd, libc::F_GETFD) } >= 0 {
            inherited.push(fd);
        }
    }
    let host_tmp = unsafe {
        libc::open(
            c"/tmp".as_ptr(),
            libc::O_TMPFILE | libc::O_WRONLY | libc::O_CLOEXEC,
            0o600,
        )
    };
    let readonly_root =
        host_tmp < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EROFS);
    if host_tmp >= 0 {
        unsafe {
            libc::close(host_tmp);
        }
    }
    let scratch_writable = std::fs::write("scratch-check", b"private scratch").is_ok();
    let mut scratch: libc::statfs = unsafe { std::mem::zeroed() };
    let scratch_bounded = unsafe { libc::statfs(c".".as_ptr(), &mut scratch) } == 0
        && scratch.f_type == 0x01021994
        && scratch.f_blocks as u64 * scratch.f_bsize as u64 <= 67108864
        && scratch.f_files <= 4096;
    let mut pid = 0;
    if ["orphan", "clone-orphan", "timeout", "cancel"]
        .contains(&input["action"].as_str().unwrap_or(""))
    {
        let mut ready = [-1; 2];
        if unsafe { libc::pipe2(ready.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            std::process::exit(92);
        }
        pid = if input["action"] == "clone-orphan" {
            unsafe {
                libc::syscall(
                    libc::SYS_clone,
                    0u64,
                    std::ptr::null::<u8>(),
                    std::ptr::null::<u8>(),
                    std::ptr::null::<u8>(),
                    0u64,
                ) as i32
            }
        } else {
            unsafe { libc::fork() }
        };
        if pid < 0 {
            std::process::exit(93);
        }
        if pid == 0 {
            unsafe {
                libc::close(ready[0]);
            }
            if unsafe { libc::setsid() } < 0 {
                std::process::exit(91);
            }
            unsafe {
                libc::write(ready[1], b"1".as_ptr().cast(), 1);
                libc::close(ready[1]);
            }
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        unsafe {
            libc::close(ready[1]);
            let mut marker = [0u8; 1];
            if libc::read(ready[0], marker.as_mut_ptr().cast(), 1) != 1 {
                std::process::exit(94);
            }
            libc::close(ready[0]);
        }
    }
    println!(
        "{}",
        json!({"pid":std::process::id(),"child":pid,"cgroup":cgroup,"status":caps,"networkDenied":network<0,"coreDenied":core_denied,"keyDenied":key_denied,"inherited":inherited,"environmentCount":std::env::vars_os().count(),"readonlyRoot":readonly_root,"scratchWritable":scratch_writable,"scratchBounded":scratch_bounded})
    );
    std::io::stdout().flush().unwrap();
    if ["timeout", "cancel"].contains(&input["action"].as_str().unwrap_or("")) {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    std::process::exit(23);
}
#[cfg(not(target_os = "linux"))]
fn main() {
    panic!("Linux-only boundary test image")
}
