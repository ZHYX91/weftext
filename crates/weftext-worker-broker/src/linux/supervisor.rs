use super::*;
const CGROUP2_SUPER_MAGIC: libc::c_long = 0x63677270;
const CLONE_INTO_CGROUP: u64 = 0x200000000;
const CLONE_PIDFD: u64 = 0x00001000;
const MAX_ADMISSION_RESIDUE: usize = 256;

#[repr(C)]
#[derive(Default)]
struct CloneArgs {
    flags: u64,
    pidfd: u64,
    child_tid: u64,
    parent_tid: u64,
    exit_signal: u64,
    stack: u64,
    stack_size: u64,
    tls: u64,
    set_tid: u64,
    set_tid_size: u64,
    cgroup: u64,
}
#[repr(C)]
struct MountAttr {
    attr_set: u64,
    attr_clr: u64,
    propagation: u64,
    userns_fd: u64,
}
struct ChildSetup<'a> {
    policy: &'a Policy,
    argv: &'a [*const libc::c_char],
    env: &'a [*const libc::c_char],
    image: RawFd,
    stdio: [RawFd; 3],
    report: RawFd,
    cwd_path: &'a std::ffi::CStr,
    tmpfs_options: &'a std::ffi::CStr,
    expected_parent: libc::pid_t,
}

pub(super) fn qualify_environment(policy: &Policy) -> Result<()> {
    if !cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
        return Err(invalid("unsupported: seccomp architecture"));
    }
    // The v3 capability ABI used below has exactly 64 capability bits.
    let future_cap = unsafe { libc::prctl(libc::PR_CAPBSET_READ, 64, 0, 0, 0) };
    if future_cap >= 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINVAL) {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "unqualified capability ABI",
        ));
    }
    let state = trusted(&policy.state_root, true)?;
    if state.file.metadata()?.mode() & 0o777 != 0o700 {
        return Err(invalid("worker state root must be root 0700"));
    }
    let cgroup = trusted(&policy.cgroup_root, true)?;
    let mut fs: libc::statfs = unsafe { zeroed() };
    cvt(unsafe { libc::fstatfs(cgroup.file.as_raw_fd(), &mut fs) })?;
    if fs.f_type != CGROUP2_SUPER_MAGIC {
        return Err(invalid("unsupported: cgroup v2 required"));
    }
    let available = read_at(&cgroup.file, c"cgroup.subtree_control")?;
    for required in ["cpu", "memory", "pids"] {
        if !available.split_whitespace().any(|word| word == required) {
            return Err(invalid(format!(
                "unsupported: cgroup controller {required} not enabled"
            )));
        }
    }
    qualify_image(policy)?;
    Ok(())
}
fn qualify_image(policy: &Policy) -> Result<File> {
    let image = trusted(&policy.image, false)?;
    let metadata = image.file.metadata()?;
    if metadata.mode() & 0o6111 != 0o111 {
        return Err(invalid(
            "product image must be executable without set-id bits",
        ));
    }
    let mut magic = [0u8; 4];
    use std::os::unix::fs::FileExt;
    image.file.read_exact_at(&mut magic, 0)?;
    if magic != *b"\x7fELF" {
        return Err(invalid("product image type/hash mismatch"));
    }
    // set-id/capability elevation is independently blocked by no_new_privs.
    let size = unsafe {
        libc::fgetxattr(
            image.file.as_raw_fd(),
            c"security.capability".as_ptr(),
            std::ptr::null_mut(),
            0,
        )
    };
    if size >= 0 {
        return Err(invalid("product image has file capabilities"));
    }
    if !matches!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ENODATA | libc::ENOTSUP)
    ) {
        return Err(std::io::Error::last_os_error().into());
    }
    snapshot_image(&image.file, &policy.image_sha256)
}
fn open_at(parent: &File, name: &std::ffi::CStr, flags: i32) -> Result<File> {
    let fd = cvt(unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    })?;
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn read_at(parent: &File, name: &std::ffi::CStr) -> Result<String> {
    let file = open_at(parent, name, libc::O_RDONLY)?;
    String::from_utf8(bounded_bytes(&file, 4096)?)
        .map_err(|_| invalid("invalid cgroup control bytes"))
}
fn write_at(parent: &File, name: &std::ffi::CStr, value: &[u8]) -> Result<()> {
    open_at(parent, name, libc::O_WRONLY)?.write_all(value)?;
    Ok(())
}
fn random_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    let mut offset = 0;
    while offset < bytes.len() {
        let n = unsafe {
            libc::getrandom(bytes[offset..].as_mut_ptr().cast(), bytes.len() - offset, 0)
        };
        if n <= 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        offset += n as usize;
    }
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn unique_directory(parent: &File, name: &CString) -> Result<File> {
    cvt(unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) })?;
    let dir = open_at(parent, name, libc::O_RDONLY | libc::O_DIRECTORY)?;
    cvt(unsafe { libc::fchmod(dir.as_raw_fd(), 0o700) })?;
    Ok(dir)
}
fn duplicate(fd: RawFd) -> Result<OwnedFd> {
    let fd = cvt(unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10) })?;
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}
fn check_admission_budget(directory: &File, cgroup: bool) -> Result<()> {
    let entries = std::fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))?;
    let mut count = 0;
    for entry in entries {
        let entry = entry?;
        let kind = entry.file_type()?;
        if !cgroup || kind.is_dir() || kind.is_symlink() {
            count += 1;
            if count >= MAX_ADMISSION_RESIDUE {
                return Err(Error::new(
                    ErrorKind::Unavailable,
                    "owned worker subtree residue budget reached; administrator inspection required",
                ));
            }
        }
    }
    Ok(())
}
fn remove_empty_owned(parent: &File, name: &CString, owned: &File) -> Result<()> {
    let held = owned.metadata()?;
    let mut current: libc::stat = unsafe { zeroed() };
    cvt(unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            &mut current,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    })?;
    if current.st_dev != held.dev()
        || current.st_ino != held.ino()
        || current.st_uid != held.uid()
        || current.st_mode & libc::S_IFMT != libc::S_IFDIR
    {
        return Err(invalid("owned empty-directory identity changed; retained"));
    }
    cvt(unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) })?;
    Ok(())
}

struct Job {
    id: String,
    pid: i32,
    pidfd: OwnedFd,
    cgroup: File,
    cgroup_parent: TrustedPath,
    cwd: File,
    state_parent: File,
}
impl Job {
    fn kill(&self) -> Result<()> {
        write_at(&self.cgroup, c"cgroup.kill", b"1")
    }
    fn pidfd_exited(&self) -> Result<bool> {
        let mut poll = libc::pollfd {
            fd: self.pidfd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        cvt(unsafe { libc::poll(&mut poll, 1, 0) })?;
        Ok(poll.revents & libc::POLLIN != 0)
    }
    fn populated(&self) -> Result<bool> {
        match read_at(&self.cgroup, c"cgroup.events")?
            .lines()
            .find(|line| line.starts_with("populated "))
        {
            Some("populated 0") => Ok(false),
            Some("populated 1") => Ok(true),
            _ => Err(invalid("invalid cgroup.events population")),
        }
    }
    fn finish(self, completion: Completion) -> Result<Receipt> {
        // cgroup.kill includes orphaned/setsid descendants. A root exit is not a receipt.
        // Never release ownership or admit another job when a kernel refuses
        // cleanup. Keep the exact cgroup and pidfd pinned and retry only this
        // owned kill, logging the real error; do not issue a completion receipt.
        loop {
            match self.kill() {
                Ok(()) => break,
                Err(error) => {
                    eprintln!("owned cgroup kill unavailable ({}): {error}", self.id);
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
        let mut reaped = 0u32;
        let mut root_status = None;
        loop {
            let mut no_children = false;
            loop {
                let mut status = 0;
                let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG | libc::__WALL) };
                if pid > 0 {
                    reaped = reaped
                        .checked_add(1)
                        .ok_or_else(|| invalid("reap count overflow"))?;
                    if pid == self.pid {
                        root_status = Some(status);
                    }
                } else if pid == 0 {
                    break;
                } else {
                    let code = std::io::Error::last_os_error().raw_os_error();
                    if code == Some(libc::ECHILD) {
                        no_children = true;
                        break;
                    }
                    if code != Some(libc::EINTR) {
                        eprintln!(
                            "owned child reap unavailable ({}): {}",
                            self.id,
                            std::io::Error::last_os_error()
                        );
                        break;
                    }
                }
            }
            let empty = match self.populated() {
                Ok(populated) => !populated,
                Err(error) => {
                    eprintln!("owned cgroup population unavailable ({}): {error}", self.id);
                    false
                }
            };
            let exited = match self.pidfd_exited() {
                Ok(exited) => exited,
                Err(error) => {
                    eprintln!("owned pidfd unavailable ({}): {error}", self.id);
                    false
                }
            };
            if empty && exited && root_status.is_some() && no_children {
                break;
            }
            // Ownership remains here for kernel-stuck tasks; never acknowledge false completion.
            std::thread::sleep(Duration::from_millis(10));
        }
        let status = root_status.unwrap();
        let name = CString::new(self.id.clone()).unwrap();
        cvt(unsafe {
            libc::unlinkat(
                self.cgroup_parent.file.as_raw_fd(),
                name.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        })?;
        if let Err(error) = remove_empty_owned(&self.state_parent, &name, &self.cwd) {
            // Process completion is already proven. Never recursively remove
            // residue, nor turn a cwd cleanup failure into an unreaped claim.
            eprintln!("owned empty cwd retained ({}): {error}", self.id);
        }
        Ok(Receipt {
            version: 1,
            job_id: self.id,
            completion,
            exit_code: libc::WIFEXITED(status).then(|| libc::WEXITSTATUS(status)),
            signal: libc::WIFSIGNALED(status).then(|| libc::WTERMSIG(status)),
            reaped,
            cgroup_empty: true,
            pidfd_exited: true,
        })
    }
}

pub(super) fn run(
    policy: &Policy,
    launch: &Launch,
    stdio: &[OwnedFd],
    session: &Socket,
) -> Result<()> {
    let image = qualify_image(policy)?;
    let cgroup_parent = trusted(&policy.cgroup_root, true)?;
    let state_parent = trusted(&policy.state_root, true)?;
    // Check both owned subtrees before the first mkdir. Failure leftovers cannot
    // consume unbounded host inodes, including clone/setup failures.
    check_admission_budget(&state_parent.file, false)?;
    check_admission_budget(&cgroup_parent.file, true)?;
    let id = random_id()?;
    let name = CString::new(id.clone()).unwrap();
    let cgroup = unique_directory(&cgroup_parent.file, &name)?;
    let mut created_cwd = None;
    let setup = (|| {
        write_at(&cgroup, c"memory.max", b"536870912")?;
        write_at(&cgroup, c"memory.swap.max", b"0")?;
        write_at(&cgroup, c"memory.oom.group", b"1")?;
        write_at(&cgroup, c"pids.max", b"32")?;
        write_at(&cgroup, c"cpu.max", b"100000 100000")?;
        // Presence is mandatory before there can be an untrusted child.
        let _kill = open_at(&cgroup, c"cgroup.kill", libc::O_WRONLY)?;
        let cwd = unique_directory(&state_parent.file, &name)?;
        created_cwd = Some(cwd.try_clone()?);
        // This inode was just created. Never chown a pre-existing user tree.
        cvt(unsafe { libc::fchown(cwd.as_raw_fd(), policy.worker_uid(), policy.worker_gid()) })?;
        let image_fd = duplicate(image.as_raw_fd())?;
        let stdin = duplicate(stdio[0].as_raw_fd())?;
        let stdout = duplicate(stdio[1].as_raw_fd())?;
        let stderr = duplicate(stdio[2].as_raw_fd())?;
        let (status_read, status_write) = pipe()?;
        let report = duplicate(status_write.as_raw_fd())?;
        drop(status_write);
        let arg0 = cstr(policy.image.as_os_str())?;
        let arg1 = CString::new(launch.kind.fixed_argument()?).unwrap();
        let argv = [arg0.as_ptr(), arg1.as_ptr(), std::ptr::null()];
        let env = [std::ptr::null::<libc::c_char>()];
        let cwd_path = cstr(policy.state_root.join(&id).as_os_str())?;
        let tmpfs_options = CString::new(format!(
            "size=67108864,nr_inodes=4096,mode=0700,uid={},gid={}",
            policy.worker_uid(),
            policy.worker_gid()
        ))
        .unwrap();
        // Prepare all fallible resources before a runnable child can exist.
        let held_cgroup = cgroup.try_clone()?;
        let held_parent = trusted(&policy.cgroup_root, true)?;
        let held_state_parent = state_parent.file.try_clone()?;
        let owned_id = id.clone();
        let expected_parent = unsafe { libc::getpid() };
        let mut pidfd = -1i32;
        let args = CloneArgs {
            flags: CLONE_INTO_CGROUP
                | CLONE_PIDFD
                | libc::CLONE_NEWNET as u64
                | libc::CLONE_NEWNS as u64
                | libc::CLONE_NEWIPC as u64,
            pidfd: (&mut pidfd as *mut i32) as u64,
            exit_signal: libc::SIGCHLD as u64,
            cgroup: cgroup.as_raw_fd() as u64,
            ..Default::default()
        };
        // CLONE_INTO_CGROUP is atomic with birth; there is no runnable outside interval.
        let pid = unsafe { libc::syscall(libc::SYS_clone3, &args, size_of::<CloneArgs>()) };
        if pid == -1 {
            return Err(Error::new(
                ErrorKind::Unsupported,
                format!(
                    "unsupported/unavailable: atomic cgroup/pidfd/network clone: {}",
                    std::io::Error::last_os_error()
                ),
            ));
        }
        if pid == 0 {
            unsafe {
                child(ChildSetup {
                    policy,
                    argv: &argv,
                    env: &env,
                    image: image_fd.as_raw_fd(),
                    stdio: [stdin.as_raw_fd(), stdout.as_raw_fd(), stderr.as_raw_fd()],
                    report: report.as_raw_fd(),
                    cwd_path: &cwd_path,
                    tmpfs_options: &tmpfs_options,
                    expected_parent,
                })
            }
        }
        // CLONE_PIDFD's successful syscall supplies this owned FD atomically.
        let job = Job {
            id: owned_id,
            pid: pid as i32,
            pidfd: unsafe { OwnedFd::from_raw_fd(pidfd) },
            cgroup: held_cgroup,
            cgroup_parent: held_parent,
            cwd,
            state_parent: held_state_parent,
        };
        drop((report, stdin, stdout, stderr, image_fd));
        Ok((job, status_read))
    })();
    let (job, mut status_read) = match setup {
        Ok(result) => result,
        Err(error) => {
            // No child exists on any ordinary failure before clone3 returns success.
            if let Err(cleanup) = remove_empty_owned(&cgroup_parent.file, &name, &cgroup) {
                eprintln!("owned empty cgroup retained ({id}): {cleanup}");
            }
            if let Some(cwd) = &created_cwd
                && let Err(cleanup) = remove_empty_owned(&state_parent.file, &name, cwd)
            {
                eprintln!("owned pre-clone cwd retained ({id}): {cleanup}");
            }
            return Err(error);
        }
    };
    let lifecycle = (|| {
        // Started identifies the atomically born owned job, before exec qualification.
        session.send(
            &Started {
                version: 1,
                job_id: job.id.clone(),
                pid: job.pid,
                image_sha256: policy.image_sha256.clone(),
            },
            &[],
        )?;
        let mut setup_status = [0u8; 4];
        let n = status_read.read(&mut setup_status)?;
        if n != 0 {
            return Err(invalid(format!(
                "trusted child setup failed: errno {}",
                i32::from_ne_bytes(setup_status)
            )));
        }
        let deadline = Instant::now() + Duration::from_millis(launch.timeout_ms);
        loop {
            if STOP.load(Ordering::Relaxed) {
                return Ok(Completion::Disconnected);
            }
            if job.pidfd_exited()? {
                return Ok(Completion::Exited);
            }
            if Instant::now() >= deadline {
                return Ok(Completion::Timeout);
            }
            let mut polls = [
                libc::pollfd {
                    fd: session.0.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: job.pidfd.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            let n = unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as _, 10) };
            if n < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            cvt(n)?;
            if polls[0].revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
                return Ok(Completion::Disconnected);
            }
            if polls[0].revents & libc::POLLIN != 0 {
                let packet = session
                    .receive(false)?
                    .ok_or_else(|| invalid("missing control frame"))?;
                let cancel: Cancel = decode(&packet.bytes)?;
                if !packet.fds.is_empty() || cancel.version != 1 || !cancel.cancel {
                    return Err(invalid("invalid cancel frame"));
                }
                return Ok(Completion::Cancelled);
            }
        }
    })();
    let completion = lifecycle
        .as_ref()
        .cloned()
        .unwrap_or(Completion::SetupFailed);
    // This happens for malformed controls and transport errors too.
    let receipt = job.finish(completion)?;
    session.send(&receipt, &[])?;
    lifecycle.map(|_| ())
}

/// Only trusted syscall setup runs between clone and exec. It does not call product code.
unsafe fn child(setup: ChildSetup<'_>) -> ! {
    let ChildSetup {
        policy,
        argv,
        env,
        image,
        stdio,
        report,
        cwd_path,
        tmpfs_options,
        expected_parent,
    } = setup;
    unsafe {
        // Report FD is established first, and closes on a successful execveat.
        if libc::dup3(report, 4, libc::O_CLOEXEC) < 0 {
            libc::_exit(126);
        }
        macro_rules! must {
            ($value:expr) => {
                if $value < 0 {
                    fail_child();
                }
            };
        }
        must!(libc::dup3(image, 3, libc::O_CLOEXEC));
        for (number, fd) in stdio.iter().enumerate() {
            must!(libc::dup2(*fd, number as i32));
        }
        // All mount changes are confined to CLONE_NEWNS. Root is read-only,
        // including /tmp and /dev/shm; only this bounded scratch mount is writable.
        must!(libc::mount(
            std::ptr::null(),
            c"/".as_ptr(),
            std::ptr::null(),
            libc::MS_REC | libc::MS_PRIVATE,
            std::ptr::null()
        ));
        let readonly = MountAttr {
            // MOUNT_ATTR_RDONLY | MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV.
            attr_set: 1 | 2 | 4,
            attr_clr: 0,
            propagation: 0,
            userns_fd: 0,
        };
        must!(libc::syscall(
            libc::SYS_mount_setattr,
            libc::AT_FDCWD,
            c"/".as_ptr(),
            0x8000u32,
            &readonly,
            size_of::<MountAttr>()
        ));
        must!(libc::mount(
            c"tmpfs".as_ptr(),
            cwd_path.as_ptr(),
            c"tmpfs".as_ptr(),
            libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
            tmpfs_options.as_ptr().cast()
        ));
        let scratch = libc::open(
            cwd_path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        );
        must!(scratch);
        must!(libc::fchdir(scratch));
        must!(libc::syscall(libc::SYS_close_range, 5u32, u32::MAX, 0u32));
        must!(libc::setgroups(0, std::ptr::null()));
        // Clear ambient and bounding capabilities before relinquishing root.
        must!(libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0
        ));
        for cap in 0..64 {
            if libc::prctl(libc::PR_CAPBSET_DROP, cap, 0, 0, 0) < 0
                && std::io::Error::last_os_error().raw_os_error() != Some(libc::EINVAL)
            {
                fail_child();
            }
        }
        must!(libc::setresgid(
            policy.worker_gid(),
            policy.worker_gid(),
            policy.worker_gid()
        ));
        must!(libc::setresuid(
            policy.worker_uid(),
            policy.worker_uid(),
            policy.worker_uid()
        ));
        // setresuid clears PDEATHSIG, so establish it only after credential drop.
        must!(libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0));
        if libc::getppid() != expected_parent {
            fail_child();
        }
        #[repr(C)]
        struct CapHeader {
            version: u32,
            pid: i32,
        }
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct CapData {
            effective: u32,
            permitted: u32,
            inheritable: u32,
        }
        let header = CapHeader {
            version: 0x20080522,
            pid: 0,
        };
        let caps = [CapData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }; 2];
        must!(libc::syscall(libc::SYS_capset, &header, caps.as_ptr()));
        must!(libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0));
        must!(libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0));
        libc::umask(0o077);
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        must!(libc::setrlimit(libc::RLIMIT_CORE, &limit));
        let fsize = libc::rlimit {
            rlim_cur: 104857600,
            rlim_max: 104857600,
        };
        must!(libc::setrlimit(libc::RLIMIT_FSIZE, &fsize));
        let files = libc::rlimit {
            rlim_cur: 128,
            rlim_max: 128,
        };
        must!(libc::setrlimit(libc::RLIMIT_NOFILE, &files));
        if install_seccomp().is_err() {
            fail_child();
        }
        must!(libc::syscall(
            libc::SYS_execveat,
            3,
            c"".as_ptr(),
            argv.as_ptr(),
            env.as_ptr(),
            libc::AT_EMPTY_PATH
        ));
        fail_child()
    }
}
unsafe fn fail_child() -> ! {
    let error = std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EIO)
        .to_ne_bytes();
    unsafe {
        libc::write(4, error.as_ptr().cast(), error.len());
        libc::_exit(126)
    }
}

/// Disallow all new sockets (including filesystem Unix sockets), io_uring bypass,
/// cross-process FD acquisition, ptrace and namespace/mount changes.
unsafe fn install_seccomp() -> Result<()> {
    #[cfg(target_arch = "x86_64")]
    const ARCH: u32 = 0xc000003e;
    #[cfg(target_arch = "aarch64")]
    const ARCH: u32 = 0xc00000b7;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    const ARCH: u32 = 0;
    let stmt = |code, k| libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    };
    let jump = |k, jt, jf| libc::sock_filter {
        code: 0x15,
        jt,
        jf,
        k,
    };
    let mut filters = vec![
        stmt(0x20, 4),
        jump(ARCH, 1, 0),
        stmt(0x06, 0x80000000),
        stmt(0x20, 0),
    ];
    #[cfg(target_arch = "x86_64")]
    {
        filters.push(libc::sock_filter {
            code: 0x45,
            jt: 0,
            jf: 1,
            k: 0x40000000,
        });
        filters.push(stmt(0x06, 0x80000000));
    }
    for syscall in [
        libc::SYS_socket,
        libc::SYS_socketpair,
        libc::SYS_io_uring_setup,
        libc::SYS_ptrace,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_pidfd_getfd,
        libc::SYS_open_by_handle_at,
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_mount_setattr,
        libc::SYS_open_tree,
        libc::SYS_move_mount,
        libc::SYS_fsopen,
        libc::SYS_fsconfig,
        libc::SYS_fsmount,
        libc::SYS_fspick,
        libc::SYS_unshare,
        libc::SYS_setns,
        libc::SYS_bpf,
        libc::SYS_perf_event_open,
    ] {
        filters.push(jump(syscall as u32, 0, 1));
        filters.push(stmt(0x06, 0x00050000 | libc::EPERM as u32));
    }
    filters.push(stmt(0x06, 0x7fff0000));
    let program = libc::sock_fprog {
        len: filters.len() as _,
        filter: filters.as_mut_ptr(),
    };
    cvt(unsafe { libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &program) })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn residue_budget_counts_state_entries_and_cgroup_child_directories() {
        use std::os::unix::fs::DirBuilderExt;
        let path =
            std::env::temp_dir().join(format!("weftext-broker-budget-{}", random_id().unwrap()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        let directory = File::open(&path).unwrap();
        assert!(check_admission_budget(&directory, false).is_ok());
        assert!(check_admission_budget(&directory, true).is_ok());
        std::fs::write(path.join("cgroup.events"), b"control-file").unwrap();
        for index in 0..MAX_ADMISSION_RESIDUE - 1 {
            std::fs::create_dir(path.join(index.to_string())).unwrap();
        }
        assert!(check_admission_budget(&directory, false).is_err());
        assert!(check_admission_budget(&directory, true).is_ok());
        std::fs::create_dir(path.join((MAX_ADMISSION_RESIDUE - 1).to_string())).unwrap();
        assert!(check_admission_budget(&directory, true).is_err());
        for index in 0..MAX_ADMISSION_RESIDUE {
            std::fs::remove_dir(path.join(index.to_string())).unwrap();
        }
        std::fs::remove_file(path.join("cgroup.events")).unwrap();
        std::fs::remove_dir(path).unwrap();
    }
}
