use crate::*;
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::{size_of, zeroed};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

mod supervisor;
static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop_signal(_: i32) {
    STOP.store(true, Ordering::Relaxed);
}

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Rejected, message)
}
fn cvt(value: i32) -> Result<i32> {
    if value == -1 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(value)
    }
}
fn cstr(value: &std::ffi::OsStr) -> Result<CString> {
    CString::new(value.as_bytes()).map_err(|_| invalid("NUL in path"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Policy {
    version: u32,
    enrollment: PathBuf,
    enrollment_sha256: String,
    enrollment_id: String,
    #[serde(skip)]
    qualified: Option<weftext_enrollment::ValidatedEnrollment>,
    socket: PathBuf,
    state_root: PathBuf,
    cgroup_root: PathBuf,
    image: PathBuf,
    image_sha256: String,
}

/// Shared enrollment authority validates every administrator-owned ancestor.
struct TrustedPath {
    file: File,
}
fn trusted(path: &Path, directory: bool) -> Result<TrustedPath> {
    let file = if directory {
        weftext_enrollment::open_administrator_directory(path)
    } else {
        weftext_enrollment::open_administrator_file(path, 1024 * 1024 * 1024)
    }
    .map_err(|error| invalid(format!("administrator object/ancestor rejected: {error:?}")))?;
    reject_acl(&file)?;
    Ok(TrustedPath { file })
}
fn reject_acl(file: &File) -> Result<()> {
    for name in [c"system.posix_acl_access", c"system.posix_acl_default"] {
        let size =
            unsafe { libc::fgetxattr(file.as_raw_fd(), name.as_ptr(), std::ptr::null_mut(), 0) };
        if size >= 0 {
            return Err(invalid("POSIX ACL is not qualified"));
        }
        let code = std::io::Error::last_os_error().raw_os_error();
        if code != Some(libc::ENODATA) && code != Some(libc::ENOTSUP) {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(())
}
fn bounded_bytes(file: &File, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid("administrator file exceeds bound"));
    }
    Ok(bytes)
}
fn snapshot_image(file: &File, expected: &str) -> Result<File> {
    // MFD_EXEC is an explicit kernel requirement; no mutable-path exec fallback.
    let fd = cvt(unsafe {
        libc::memfd_create(
            c"weftext-worker".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING | 0x0010,
        )
    })
    .map_err(|error| {
        Error::new(
            ErrorKind::Unsupported,
            format!("executable sealed memfd: {error}"),
        )
    })?;
    let mut snapshot = unsafe { File::from_raw_fd(fd) };
    let mut hasher = Sha256::new();
    let mut reader = file;
    let mut chunk = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > 1024 * 1024 * 1024 {
            return Err(invalid("image exceeds 1 GiB qualification bound"));
        }
        hasher.update(&chunk[..n]);
        snapshot.write_all(&chunk[..n])?;
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest != expected {
        return Err(invalid("product image type/hash mismatch"));
    }
    cvt(unsafe { libc::fchmod(snapshot.as_raw_fd(), 0o555) })?;
    cvt(unsafe {
        libc::fcntl(
            snapshot.as_raw_fd(),
            libc::F_ADD_SEALS,
            libc::F_SEAL_SEAL | libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK,
        )
    })?;
    Ok(snapshot)
}
impl Policy {
    fn load(path: &Path) -> Result<Self> {
        let guard = trusted(path, false)?;
        let mut p: Self = serde_json::from_slice(&bounded_bytes(&guard.file, 16384)?)?;
        if p.version != 1 {
            return Err(invalid("invalid broker policy version"));
        }
        let cgroup_mount = Path::new("/sys/fs/cgroup");
        if !p.cgroup_root.starts_with(cgroup_mount) || p.cgroup_root == cgroup_mount {
            return Err(invalid(
                "broker requires an explicit owned cgroup v2 subtree",
            ));
        }
        for hash in [&p.image_sha256, &p.enrollment_sha256] {
            if hash.len() != 64
                || !hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid("expected lowercase SHA256"));
            }
        }
        // The same sealed validator is consumed by Core and this broker.
        // It holds no Core key bytes and exposes no enrollment constructor.
        let enrollment = weftext_enrollment::inspect_enrollment(&p.enrollment)
            .map_err(|error| invalid(format!("Core enrollment rejected: {error:?}")))?;
        if enrollment.enrollment_sha256() != p.enrollment_sha256
            || enrollment.enrollment_id() != p.enrollment_id
        {
            return Err(invalid("policy binding differs from the Core enrollment"));
        }
        p.qualified = Some(enrollment);
        Ok(p)
    }
    fn qualified(&self) -> &weftext_enrollment::ValidatedEnrollment {
        self.qualified.as_ref().expect("Policy::load qualification")
    }
    fn service_uid(&self) -> u32 {
        self.qualified().service_uid()
    }
    fn service_gid(&self) -> u32 {
        self.qualified().service_gid()
    }
    fn worker_uid(&self) -> u32 {
        self.qualified().worker_uid()
    }
    fn worker_gid(&self) -> u32 {
        self.qualified().worker_gid()
    }
}

struct Socket(OwnedFd);
struct BoundSocketPath {
    parent: File,
    name: CString,
    device: u64,
    inode: u64,
}
impl Drop for BoundSocketPath {
    fn drop(&mut self) {
        let mut stat: libc::stat = unsafe { zeroed() };
        if unsafe {
            libc::fstatat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                &mut stat,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } == 0
            && stat.st_dev == self.device
            && stat.st_ino == self.inode
            && stat.st_uid == 0
            && stat.st_mode & libc::S_IFMT == libc::S_IFSOCK
        {
            // Only this daemon's own bound runtime socket, never an old entry.
            unsafe {
                libc::unlinkat(self.parent.as_raw_fd(), self.name.as_ptr(), 0);
            }
        }
    }
}
struct Packet {
    bytes: Vec<u8>,
    fds: Vec<OwnedFd>,
}
impl Socket {
    fn new() -> Result<Self> {
        let fd = cvt(unsafe {
            libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0)
        })?;
        Ok(Self(unsafe { OwnedFd::from_raw_fd(fd) }))
    }
    fn address(path: &Path) -> Result<(libc::sockaddr_un, libc::socklen_t)> {
        let bytes = path.as_os_str().as_bytes();
        let mut address: libc::sockaddr_un = unsafe { zeroed() };
        if !path.is_absolute() || bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
            return Err(invalid("invalid broker socket path"));
        }
        address.sun_family = libc::AF_UNIX as _;
        for (to, from) in address.sun_path.iter_mut().zip(bytes) {
            *to = *from as _;
        }
        Ok((address, size_of::<libc::sockaddr_un>() as _))
    }
    fn peer(&self) -> Result<libc::ucred> {
        let mut cred: libc::ucred = unsafe { zeroed() };
        let mut len = size_of::<libc::ucred>() as libc::socklen_t;
        cvt(unsafe {
            libc::getsockopt(
                self.0.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut cred as *mut libc::ucred).cast(),
                &mut len,
            )
        })?;
        if len as usize != size_of::<libc::ucred>() || cred.pid <= 1 {
            return Err(invalid("invalid peer credentials"));
        }
        Ok(cred)
    }
    fn send<T: Serialize>(&self, frame: &T, fds: &[RawFd]) -> Result<()> {
        self.send_flags(frame, fds, 0)
    }
    fn send_flags<T: Serialize>(&self, frame: &T, fds: &[RawFd], flags: i32) -> Result<()> {
        let bytes = serde_json::to_vec(frame)?;
        if bytes.len() > MAX_FRAME || fds.len() > 3 {
            return Err(invalid("invalid outgoing frame bound"));
        }
        let mut iov = libc::iovec {
            iov_base: bytes.as_ptr().cast_mut().cast(),
            iov_len: bytes.len(),
        };
        let mut control = [0usize; 8];
        let mut message: libc::msghdr = unsafe { zeroed() };
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        if !fds.is_empty() {
            message.msg_control = control.as_mut_ptr().cast();
            message.msg_controllen =
                unsafe { libc::CMSG_SPACE(std::mem::size_of_val(fds) as _) } as _;
            unsafe {
                let c = libc::CMSG_FIRSTHDR(&message);
                (*c).cmsg_level = libc::SOL_SOCKET;
                (*c).cmsg_type = libc::SCM_RIGHTS;
                (*c).cmsg_len = libc::CMSG_LEN(std::mem::size_of_val(fds) as _) as _;
                std::ptr::copy_nonoverlapping(fds.as_ptr(), libc::CMSG_DATA(c).cast(), fds.len());
            }
        }
        let n = unsafe { libc::sendmsg(self.0.as_raw_fd(), &message, libc::MSG_NOSIGNAL | flags) };
        if n < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if n as usize != bytes.len() {
            return Err(invalid("short frame write"));
        }
        Ok(())
    }
    fn receive(&self, nonblocking: bool) -> Result<Option<Packet>> {
        let mut bytes = vec![0u8; MAX_FRAME + 1];
        let mut control = [0usize; 8];
        let mut iov = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: bytes.len(),
        };
        let mut message: libc::msghdr = unsafe { zeroed() };
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = std::mem::size_of_val(&control);
        let n = unsafe {
            libc::recvmsg(
                self.0.as_raw_fd(),
                &mut message,
                libc::MSG_CMSG_CLOEXEC | if nonblocking { libc::MSG_DONTWAIT } else { 0 },
            )
        };
        if n < 0 {
            let e = std::io::Error::last_os_error();
            if nonblocking && e.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(e.into());
        }
        let mut fds = Vec::new();
        let mut bad_control = false;
        unsafe {
            let mut c = libc::CMSG_FIRSTHDR(&message);
            while !c.is_null() {
                if (*c).cmsg_level != libc::SOL_SOCKET
                    || (*c).cmsg_type != libc::SCM_RIGHTS
                    || (*c).cmsg_len < libc::CMSG_LEN(0) as usize
                {
                    bad_control = true;
                } else {
                    let count = ((*c).cmsg_len - libc::CMSG_LEN(0) as usize) / size_of::<RawFd>();
                    for i in 0..count {
                        fds.push(OwnedFd::from_raw_fd(
                            *libc::CMSG_DATA(c).cast::<RawFd>().add(i),
                        ));
                    }
                }
                c = libc::CMSG_NXTHDR(&message, c);
            }
        }
        if bad_control
            || message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0
            || n as usize > MAX_FRAME
        {
            return Err(invalid("truncated or unexpected ancillary frame"));
        }
        // A zero-length SOCK_SEQPACKET message can still install SCM_RIGHTS.
        // Adopt them first so every rejection/EOF path closes received handles.
        if n == 0 {
            return Err(Error::unknown(
                "broker connection closed without a completion receipt",
            ));
        }
        bytes.truncate(n as usize);
        Ok(Some(Packet { bytes, fds }))
    }
}

fn pipe() -> Result<(File, File)> {
    let mut pair = [-1; 2];
    cvt(unsafe { libc::pipe2(pair.as_mut_ptr(), libc::O_CLOEXEC) })?;
    Ok(unsafe { (File::from_raw_fd(pair[0]), File::from_raw_fd(pair[1])) })
}
fn verify_stdio(fds: &[OwnedFd]) -> Result<()> {
    if fds.len() != 3 {
        return Err(invalid("exactly three anonymous pipe descriptors required"));
    }
    let mut identities = Vec::new();
    for (index, fd) in fds.iter().enumerate() {
        let mut stat: libc::stat = unsafe { zeroed() };
        cvt(unsafe { libc::fstat(fd.as_raw_fd(), &mut stat) })?;
        let flags = cvt(unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) })?;
        let path = std::fs::read_link(format!("/proc/self/fd/{}", fd.as_raw_fd()))?;
        if stat.st_mode & libc::S_IFMT != libc::S_IFIFO
            || !path.as_os_str().as_bytes().starts_with(b"pipe:[")
            || flags & libc::O_ACCMODE
                != if index == 0 {
                    libc::O_RDONLY
                } else {
                    libc::O_WRONLY
                }
            || flags & (libc::O_PATH | libc::O_NONBLOCK) != 0
            || identities.contains(&(stat.st_dev, stat.st_ino))
        {
            return Err(invalid(
                "stdio must be distinct blocking anonymous pipes in the owned direction",
            ));
        }
        identities.push((stat.st_dev, stat.st_ino));
    }
    Ok(())
}

/// Caller supplies the already-selected Core enrollment, never an executable.
pub struct BrokerClient {
    policy: PathBuf,
    binding: EnrollmentBinding,
}
impl BrokerClient {
    pub fn connect(
        enrollment: &weftext_enrollment::ValidatedEnrollment,
        policy: &Path,
    ) -> Result<Self> {
        enrollment
            .verify_service_process()
            .map_err(|error| invalid(format!("original service qualification: {error:?}")))?;
        enrollment
            .check_current()
            .map_err(|error| invalid(format!("original enrollment continuity: {error:?}")))?;
        let binding = EnrollmentBinding {
            path: enrollment.enrollment_path().into(),
            id: enrollment.enrollment_id().into(),
            sha256: enrollment.enrollment_sha256().into(),
        };
        let p = Policy::load(policy)?;
        if !binding.matches(&p.enrollment, &p.enrollment_id, &p.enrollment_sha256) {
            return Err(invalid("broker client is not the enrolled Core service"));
        }
        p.qualified()
            .verify_service_process()
            .map_err(|error| invalid(format!("service qualification: {error:?}")))?;
        Ok(Self {
            policy: policy.into(),
            binding,
        })
    }
    pub fn launch(&self, kind: WorkerKind, timeout: Duration) -> Result<OwnedWorker> {
        let timeout_ms =
            u64::try_from(timeout.as_millis()).map_err(|_| invalid("timeout overflow"))?;
        let request = Launch {
            version: 1,
            kind,
            timeout_ms,
        };
        request.validate()?;
        let policy = Policy::load(&self.policy)?;
        if !self.binding.matches(
            &policy.enrollment,
            &policy.enrollment_id,
            &policy.enrollment_sha256,
        ) {
            return Err(invalid("enrollment changed"));
        }
        policy
            .qualified()
            .verify_service_process()
            .map_err(|error| invalid(format!("service qualification: {error:?}")))?;
        trusted(
            policy
                .socket
                .parent()
                .ok_or_else(|| invalid("socket parent absent"))?,
            true,
        )?;
        let socket = Socket::new()?;
        // A stalled administrator broker must not retain a service thread
        // forever. These bounds cover connect, send and the Started reply.
        let timeout = libc::timeval {
            tv_sec: 5,
            tv_usec: 0,
        };
        for option in [libc::SO_RCVTIMEO, libc::SO_SNDTIMEO] {
            cvt(unsafe {
                libc::setsockopt(
                    socket.0.as_raw_fd(),
                    libc::SOL_SOCKET,
                    option,
                    (&timeout as *const libc::timeval).cast(),
                    size_of::<libc::timeval>() as _,
                )
            })?;
        }
        let (address, len) = Socket::address(&policy.socket)?;
        cvt(unsafe {
            libc::connect(
                socket.0.as_raw_fd(),
                (&address as *const libc::sockaddr_un).cast(),
                len,
            )
        })?;
        if socket.peer()?.uid != 0 {
            return Err(invalid("broker peer is not administrator-owned"));
        }
        let (child_in, input) = pipe()?;
        let (output, child_out) = pipe()?;
        let (error, child_err) = pipe()?;
        socket
            .send(
                &request,
                &[
                    child_in.as_raw_fd(),
                    child_out.as_raw_fd(),
                    child_err.as_raw_fd(),
                ],
            )
            .map_err(|error| Error::unknown(format!("worker launch send: {error}")))?;
        drop((child_in, child_out, child_err));
        let reply = socket
            .receive(false)
            .map_err(|error| Error::unknown(format!("worker launch reply: {error}")))?
            .ok_or_else(|| Error::unknown("missing launch reply"))?;
        if !reply.fds.is_empty() {
            return Err(Error::unknown("unexpected broker descriptors"));
        }
        let started: Started = decode(&reply.bytes)
            .map_err(|error| Error::unknown(format!("worker launch frame: {error}")))?;
        if started.version != 1
            || started.job_id.len() != 32
            || started.pid <= 1
            || started.image_sha256 != policy.image_sha256
        {
            return Err(Error::unknown("invalid launch identity"));
        }
        // Synchronous broker consumers may wait for the full original worker
        // deadline plus cleanup; the async App consumer uses try_wait instead.
        let completion_timeout = libc::timeval {
            tv_sec: (MAX_TIMEOUT_MS / 1000 + 5) as _,
            tv_usec: 0,
        };
        cvt(unsafe {
            libc::setsockopt(
                socket.0.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                (&completion_timeout as *const libc::timeval).cast(),
                size_of::<libc::timeval>() as _,
            )
        })
        .map_err(|error| Error::unknown(format!("worker receipt deadline setup: {error}")))?;
        Ok(OwnedWorker {
            socket,
            started,
            stdin: Some(input),
            stdout: Some(output),
            stderr: Some(error),
            receipt: None,
        })
    }
}
pub struct OwnedWorker {
    socket: Socket,
    pub started: Started,
    stdin: Option<File>,
    stdout: Option<File>,
    stderr: Option<File>,
    receipt: Option<Receipt>,
}
impl OwnedWorker {
    pub fn take_stdin(&mut self) -> Option<File> {
        self.stdin.take()
    }
    pub fn take_stdout(&mut self) -> Option<File> {
        self.stdout.take()
    }
    pub fn take_stderr(&mut self) -> Option<File> {
        self.stderr.take()
    }
    fn receive_receipt(&mut self, nonblocking: bool) -> Result<Option<Receipt>> {
        if let Some(receipt) = &self.receipt {
            return Ok(Some(receipt.clone()));
        }
        let Some(reply) = self
            .socket
            .receive(nonblocking)
            .map_err(|error| Error::unknown(format!("worker completion transport: {error}")))?
        else {
            return Ok(None);
        };
        if !reply.fds.is_empty() {
            return Err(Error::unknown("unexpected completion descriptors"));
        }
        let receipt: Receipt = decode(&reply.bytes)
            .map_err(|error| Error::unknown(format!("worker completion frame: {error}")))?;
        receipt.validate(&self.started)?;
        self.receipt = Some(receipt.clone());
        Ok(Some(receipt))
    }
    pub fn try_wait(&mut self) -> Result<Option<Receipt>> {
        self.receive_receipt(true)
    }
    pub fn wait(&mut self) -> Result<Receipt> {
        self.receive_receipt(false)?
            .ok_or_else(|| Error::unknown("no worker receipt"))
    }
    /// Nonblocking exact-job cancellation; no cleanup claim until its receipt.
    pub fn request_cancel(&self) -> Result<()> {
        self.socket
            .send_flags(
                &Cancel {
                    version: 1,
                    cancel: true,
                },
                &[],
                libc::MSG_DONTWAIT,
            )
            .map_err(|error| Error::unknown(format!("worker cancellation transport: {error}")))
    }
    pub fn cancel_and_wait(&mut self) -> Result<Receipt> {
        if let Some(receipt) = self.try_wait()? {
            return Ok(receipt);
        }
        self.request_cancel()?;
        self.wait()
    }
}
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        if self.receipt.is_none() {
            let _ = self.request_cancel();
        }
        // Disconnect transfers no completion claim: the daemon still owns reaping.
    }
}

pub fn run_daemon(policy_path: &Path) -> Result<()> {
    if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
        return Err(invalid("administrator launch required"));
    }
    // A dedicated new process, never an elevation of an existing Core process.
    cvt(unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) } as i32)?;
    cvt(unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) })?;
    cvt(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) })?;
    STOP.store(false, Ordering::Relaxed);
    let mut action: libc::sigaction = unsafe { zeroed() };
    action.sa_sigaction = stop_signal as *const () as usize;
    unsafe {
        libc::sigemptyset(&mut action.sa_mask);
    }
    for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGQUIT] {
        cvt(unsafe { libc::sigaction(signal, &action, std::ptr::null_mut()) })?;
    }
    let policy = Policy::load(policy_path)?;
    supervisor::qualify_environment(&policy)?;
    let parent = trusted(
        policy
            .socket
            .parent()
            .ok_or_else(|| invalid("missing socket parent"))?,
        true,
    )?;
    if parent.file.metadata()?.mode() & 0o777 != 0o755 {
        return Err(invalid("socket parent must be administrator 0755"));
    }
    let socket = Socket::new()?;
    let (address, len) = Socket::address(&policy.socket)?;
    // No unlink/replacement of an unknown existing socket or daemon.
    cvt(unsafe {
        libc::bind(
            socket.0.as_raw_fd(),
            (&address as *const libc::sockaddr_un).cast(),
            len,
        )
    })?;
    let name = cstr(
        policy
            .socket
            .file_name()
            .ok_or_else(|| invalid("socket name absent"))?,
    )?;
    let mut socket_stat: libc::stat = unsafe { zeroed() };
    cvt(unsafe {
        libc::fstatat(
            parent.file.as_raw_fd(),
            name.as_ptr(),
            &mut socket_stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    })?;
    let _bound = BoundSocketPath {
        parent: parent.file.try_clone()?,
        name: name.clone(),
        device: socket_stat.st_dev,
        inode: socket_stat.st_ino,
    };
    cvt(unsafe {
        libc::fchownat(
            parent.file.as_raw_fd(),
            name.as_ptr(),
            0,
            policy.service_gid(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    })?;
    cvt(unsafe { libc::fchmodat(parent.file.as_raw_fd(), name.as_ptr(), 0o660, 0) })?;
    cvt(unsafe { libc::listen(socket.0.as_raw_fd(), 8) })?;
    loop {
        if STOP.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut ready = libc::pollfd {
            fd: socket.0.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let n = unsafe { libc::poll(&mut ready, 1, 100) };
        if n < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
            continue;
        }
        cvt(n)?;
        if n == 0 {
            continue;
        }
        let fd = cvt(unsafe {
            libc::accept4(
                socket.0.as_raw_fd(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                libc::SOCK_CLOEXEC,
            )
        })?;
        let session = Socket(unsafe { OwnedFd::from_raw_fd(fd) });
        let result = (|| {
            let peer = session.peer()?;
            if peer.uid != policy.service_uid() || peer.gid != policy.service_gid() {
                return Err(invalid("peer is not enrolled Core service"));
            }
            policy
                .qualified()
                .check_current()
                .map_err(|error| invalid(format!("enrollment changed: {error:?}")))?;
            let mut poll = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            if cvt(unsafe { libc::poll(&mut poll, 1, 2000) })? != 1 {
                return Err(invalid("launch frame timeout"));
            }
            let packet = session
                .receive(false)?
                .ok_or_else(|| invalid("missing request"))?;
            let launch: Launch = decode(&packet.bytes)?;
            launch.validate()?;
            verify_stdio(&packet.fds)?;
            // Never reuse a service-selected executable, cwd or environment.
            supervisor::run(&policy, &launch, &packet.fds, &session)
        })();
        if let Err(error) = result {
            eprintln!("broker rejected/failed session: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn socket_pair() -> (Socket, Socket) {
        let mut pair = [-1; 2];
        cvt(unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                pair.as_mut_ptr(),
            )
        })
        .unwrap();
        unsafe {
            (
                Socket(OwnedFd::from_raw_fd(pair[0])),
                Socket(OwnedFd::from_raw_fd(pair[1])),
            )
        }
    }
    fn receipt_client(socket: Socket) -> OwnedWorker {
        OwnedWorker {
            socket,
            started: Started {
                version: 1,
                job_id: "0".repeat(32),
                pid: 42,
                image_sha256: "0".repeat(64),
            },
            stdin: None,
            stdout: None,
            stderr: None,
            receipt: None,
        }
    }
    #[test]
    fn cancellation_is_nonblocking_and_does_not_invent_a_receipt() {
        let (client, server) = socket_pair();
        let mut worker = receipt_client(client);
        worker.request_cancel().unwrap();
        let packet = server.receive(false).unwrap().unwrap();
        let cancel: Cancel = decode(&packet.bytes).unwrap();
        assert_eq!(cancel.version, 1);
        assert!(cancel.cancel);
        assert!(packet.fds.is_empty());
        assert!(worker.try_wait().unwrap().is_none());
        for _ in 0..100_000 {
            if worker.request_cancel().is_err() {
                let start = Instant::now();
                assert_eq!(
                    worker.request_cancel().unwrap_err().kind,
                    ErrorKind::Unknown
                );
                assert!(start.elapsed() < Duration::from_secs(1));
                return;
            }
        }
        panic!("bounded socket queue did not fill");
    }
    #[test]
    fn zero_payload_rights_are_closed_before_eof_rejection() {
        let (a, b) = socket_pair();
        let (read, _write) = pipe().unwrap();
        let pipe_identity =
            std::fs::read_link(format!("/proc/self/fd/{}", read.as_raw_fd())).unwrap();
        let matching_fds = || {
            std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| std::fs::read_link(entry.path()).ok())
                .filter(|target| *target == pipe_identity)
                .count()
        };
        let baseline = matching_fds();
        for _ in 0..64 {
            let mut control = [0usize; 8];
            let mut message: libc::msghdr = unsafe { zeroed() };
            message.msg_control = control.as_mut_ptr().cast();
            message.msg_controllen = unsafe { libc::CMSG_SPACE(size_of::<RawFd>() as _) } as _;
            unsafe {
                let c = libc::CMSG_FIRSTHDR(&message);
                (*c).cmsg_level = libc::SOL_SOCKET;
                (*c).cmsg_type = libc::SCM_RIGHTS;
                (*c).cmsg_len = libc::CMSG_LEN(size_of::<RawFd>() as _) as _;
                *libc::CMSG_DATA(c).cast::<RawFd>() = read.as_raw_fd();
                assert_eq!(
                    libc::sendmsg(a.0.as_raw_fd(), &message, libc::MSG_NOSIGNAL),
                    0
                );
            }
            assert!(b.receive(false).is_err());
            assert_eq!(matching_fds(), baseline);
        }
    }
    #[test]
    fn incomplete_confused_and_disconnected_receipts_stay_unknown() {
        let good = Receipt {
            version: 1,
            job_id: "0".repeat(32),
            completion: Completion::Exited,
            exit_code: Some(23),
            signal: None,
            reaped: 2,
            cgroup_empty: true,
            pidfd_exited: true,
        };
        for bad in [
            Receipt {
                job_id: "1".repeat(32),
                ..good.clone()
            },
            Receipt {
                cgroup_empty: false,
                ..good.clone()
            },
            Receipt {
                pidfd_exited: false,
                ..good.clone()
            },
            Receipt {
                reaped: 0,
                ..good.clone()
            },
        ] {
            let (client, server) = socket_pair();
            let mut worker = receipt_client(client);
            assert!(worker.try_wait().unwrap().is_none());
            server.send(&bad, &[]).unwrap();
            assert_eq!(worker.wait().unwrap_err().kind, ErrorKind::Unknown);
        }
        let (client, server) = socket_pair();
        let mut worker = receipt_client(client);
        drop(server);
        assert_eq!(
            worker.cancel_and_wait().unwrap_err().kind,
            ErrorKind::Unknown
        );
        let (client, server) = socket_pair();
        let mut worker = receipt_client(client);
        server.send(&good, &[]).unwrap();
        assert_eq!(worker.wait().unwrap(), good);
        assert_eq!(worker.try_wait().unwrap(), Some(good));
    }
    #[test]
    fn approved_snapshot_is_immutable_after_source_changes() {
        use std::io::{Seek, SeekFrom};
        let fd =
            cvt(unsafe { libc::memfd_create(c"source-test".as_ptr(), libc::MFD_CLOEXEC) }).unwrap();
        let mut source = unsafe { File::from_raw_fd(fd) };
        let bytes = b"approved immutable image bytes";
        source.write_all(bytes).unwrap();
        source.seek(SeekFrom::Start(0)).unwrap();
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let mut snapshot = match snapshot_image(&source, &digest) {
            Ok(snapshot) => snapshot,
            Err(error) if error.kind == ErrorKind::Unsupported => {
                eprintln!("unsupported executable memfd correctly fails closed: {error}");
                return;
            }
            Err(error) => panic!("snapshot: {error}"),
        };
        source.seek(SeekFrom::Start(0)).unwrap();
        source.write_all(b"changed").unwrap();
        snapshot.seek(SeekFrom::Start(0)).unwrap();
        let mut actual = Vec::new();
        snapshot.read_to_end(&mut actual).unwrap();
        assert_eq!(actual, bytes);
        assert!(snapshot.write_all(b"injection").is_err());
        source.seek(SeekFrom::Start(0)).unwrap();
        assert!(snapshot_image(&source, &digest).is_err());
    }
    #[test]
    fn regular_file_and_wrong_pipe_direction_rejected() {
        let files: Vec<OwnedFd> = (0..3)
            .map(|_| File::open("/etc/passwd").unwrap().into())
            .collect();
        assert!(verify_stdio(&files).is_err());
        let (input, input_write) = pipe().unwrap();
        let (output, output_write) = pipe().unwrap();
        let (error, error_write) = pipe().unwrap();
        let wrong: Vec<OwnedFd> = vec![input_write.into(), output_write.into(), error_write.into()];
        assert!(verify_stdio(&wrong).is_err());
        drop((output, error));
        let (_, out) = pipe().unwrap();
        let (_, err) = pipe().unwrap();
        assert!(verify_stdio(&[input.into(), out.into(), err.into()]).is_ok());
    }
    #[test]
    fn seqpacket_fd_count_truncation_and_strict_payload() {
        let (a, b) = socket_pair();
        a.send(
            &Launch {
                version: 1,
                kind: WorkerKind::Render,
                timeout_ms: 1,
            },
            &[],
        )
        .unwrap();
        let packet = b.receive(false).unwrap().unwrap();
        assert!(decode::<Launch>(&packet.bytes).is_ok());
        assert!(verify_stdio(&packet.fds).is_err());
        let bytes = [b'x'; MAX_FRAME + 1];
        assert_eq!(
            unsafe {
                libc::send(
                    a.0.as_raw_fd(),
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    libc::MSG_NOSIGNAL,
                )
            },
            bytes.len() as isize
        );
        assert!(b.receive(false).is_err());
    }
}
