//! Experimental local advisory lock. Remote storage is not coordinated.

use std::fs::File;
use std::io;
use std::path::Path;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn acquire(root: &Path) -> io::Result<File> {
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
    Ok(file)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(crate) fn acquire(_root: &Path) -> io::Result<File> {
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
}
