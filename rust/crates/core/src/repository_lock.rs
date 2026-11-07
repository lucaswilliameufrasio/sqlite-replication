//! Experimental local advisory lock. Remote storage is not coordinated.

use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct RepositoryLock {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    file: File,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl Drop for RepositoryLock {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;

        // SAFETY: the guard owns a live descriptor. Explicit unlock also releases
        // the lock if a concurrent fork temporarily inherited that descriptor.
        unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn acquire(root: &Path) -> io::Result<RepositoryLock> {
    use std::fs::OpenOptions;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root.join(".repository-coordination.lock"))?;
    // SAFETY: the file descriptor remains owned by `file` throughout the call.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(RepositoryLock { file })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(crate) fn acquire(_root: &Path) -> io::Result<RepositoryLock> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "repository coordination lock is unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn exclusive_until_file_is_closed() {
        let root = tempfile::tempdir().expect("repository");
        let first = acquire(root.path()).expect("first lock");
        assert_eq!(
            acquire(root.path()).expect_err("lock must contend").kind(),
            io::ErrorKind::WouldBlock
        );
        drop(first);
        acquire(root.path()).expect("lock after release");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn dropping_guard_releases_lock_with_duplicated_descriptor() {
        use std::os::fd::{AsRawFd, FromRawFd};

        let root = tempfile::tempdir().expect("repository");
        let first = acquire(root.path()).expect("first lock");
        // SAFETY: dup returns a new owned descriptor or a negative error.
        let fd = unsafe { libc::dup(first.file.as_raw_fd()) };
        assert!(fd >= 0, "dup: {}", io::Error::last_os_error());
        // SAFETY: fd is a new descriptor owned by this test.
        let duplicate = unsafe { File::from_raw_fd(fd) };
        drop(first);
        acquire(root.path()).expect("lock released despite duplicate descriptor");
        drop(duplicate);
        acquire(root.path()).expect("lock after duplicate closed");
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn capture_rejects_held_repository_lock() {
        let root = tempfile::tempdir().expect("repository");
        let _lock = acquire(root.path()).expect("lock");
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let error = crate::capture::capture_once_cancellable(
            &root.path().join("source.sqlite"),
            root.path(),
            1,
            &cancelled,
            |_, _| Ok(()),
        )
        .err()
        .expect("capture must fail on contention");
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn initial_snapshot_and_restore_reject_held_lock() {
        let root = tempfile::tempdir().expect("repository");
        sqlite_replication_storage_fs::ensure_repository(root.path()).expect("initialize");
        let _lock = acquire(root.path()).expect("lock");
        assert_eq!(
            crate::snapshot(root.path().join("source.sqlite"), root.path())
                .expect_err("snapshot must reject contention")
                .kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(
            crate::restore_if_missing(root.path(), root.path().join("restored.sqlite"))
                .expect_err("restore must reject contention")
                .kind(),
            io::ErrorKind::WouldBlock
        );
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn initial_snapshot_initializes_metadata_before_lock_contention() {
        let root = tempfile::tempdir().expect("repository");
        let _lock = acquire(root.path()).expect("lock");
        let error = crate::snapshot(root.path().join("missing.sqlite"), root.path())
            .expect_err("snapshot must reject contention");
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        sqlite_replication_storage_fs::read_repository(root.path())
            .expect("metadata initialized before lock attempt");
        assert!(!sqlite_replication_storage_fs::ltx_path(root.path(), 1, 1).exists());
    }

    #[test]
    #[ignore = "requires a separately built Go repositorylock test binary"]
    #[cfg(target_os = "linux")]
    fn go_process_rejects_rust_held_lock() {
        let binary = std::env::var_os("SQLITE_REPLICATION_GO_LOCK_TEST_BINARY")
            .expect("set Go lock test binary path");
        let root = tempfile::tempdir().expect("repository");
        let _lock = acquire(root.path()).expect("Rust repository lock");
        let output = std::process::Command::new(binary)
            .arg("-test.run=^TestExclusiveLockAcrossProcesses$")
            .env("SQLITE_REPLICATION_LOCK_TEST_ROOT", root.path())
            .output()
            .expect("run Go lock test process");
        assert!(
            output.status.success(),
            "Go process did not observe contention: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    #[ignore = "requires a separately built Go repositorylock test binary"]
    #[cfg(target_os = "linux")]
    fn rust_rejects_go_process_held_lock() {
        use std::io::{BufRead, BufReader, Write};
        use std::process::Stdio;

        let binary = std::env::var_os("SQLITE_REPLICATION_GO_LOCK_TEST_BINARY")
            .expect("set Go lock test binary path");
        let root = tempfile::tempdir().expect("repository");
        let mut child = std::process::Command::new(binary)
            .arg("-test.run=^TestHoldRepositoryLockForInterop$")
            .env("SQLITE_REPLICATION_INTEROP_HOLD_ROOT", root.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("run Go holder");
        let mut line = String::new();
        let mut stdout = BufReader::new(child.stdout.take().expect("child stdout"));
        stdout.read_line(&mut line).expect("read readiness");
        assert_eq!(line.trim(), "LOCKED");
        assert_eq!(
            acquire(root.path())
                .expect_err("Rust must see contention")
                .kind(),
            io::ErrorKind::WouldBlock
        );
        writeln!(child.stdin.take().expect("child stdin"), "release").expect("release child");
        assert!(child.wait().expect("wait for Go holder").success());
        acquire(root.path()).expect("Rust lock after release");
    }
}
