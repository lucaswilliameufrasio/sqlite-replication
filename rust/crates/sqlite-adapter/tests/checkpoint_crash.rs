#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

// The parent traces a real FULL checkpoint and kills the child after a
// successful write or sync of the main database file. These tests cover
// process crashes, not power loss or storage-device durability.

use std::ffi::{c_int, c_long, c_void};
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use rusqlite::Connection;

const CHILD_READY: &str = "SQLITE_REPLICATION_CHECKPOINT_CHILD_READY";
const CHILD_ENV: &str = "SQLITE_REPLICATION_CHECKPOINT_CRASH_CHILD";
const DATABASE_ENV: &str = "SQLITE_REPLICATION_CHECKPOINT_CRASH_DATABASE";
const BOUNDARY_ENV: &str = "SQLITE_REPLICATION_CHECKPOINT_CRASH_BOUNDARY";

const PTRACE_ATTACH: c_int = 16;
const PTRACE_GET_SYSCALL_INFO: c_int = 0x420e;
const PTRACE_O_TRACESYSGOOD: usize = 0x0000_0001;
const PTRACE_SETOPTIONS: c_int = 0x4200;
const PTRACE_SYSCALL: c_int = 24;
const PTRACE_SYSCALL_INFO_ENTRY: u8 = 1;
const PTRACE_SYSCALL_INFO_EXIT: u8 = 2;
const SIGKILL: c_int = 9;
const SIGSTOP: c_int = 19;
const SIGTRAP: c_int = 5;
const WAIT_ALL_THREADS: c_int = 0x4000_0000;

#[repr(C)]
struct PtraceSyscallInfo {
    operation: u8,
    padding: [u8; 3],
    architecture: u32,
    instruction_pointer: u64,
    stack_pointer: u64,
    data: [u64; 8],
}

unsafe extern "C" {
    fn gettid() -> c_int;
    fn ptrace(request: c_int, ...) -> c_long;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
}

struct ChildGuard {
    child: Child,
    reaped: bool,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn kills_real_checkpoint_after_database_write_and_recovers() {
    run_checkpoint_crash_recovery("write");
}

#[test]
fn kills_real_checkpoint_after_database_sync_and_recovers() {
    run_checkpoint_crash_recovery("sync");
}

fn run_checkpoint_crash_recovery(boundary: &str) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = fs::canonicalize(directory.path())
        .expect("canonicalize temporary directory")
        .join("checkpoint-crash.sqlite");
    let child = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("checkpoint_crash_child")
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .env(DATABASE_ENV, &database_path)
        .env(BOUNDARY_ENV, boundary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start checkpoint child");
    let mut child_guard = ChildGuard {
        child,
        reaped: false,
    };
    let trace_tid = wait_for_child_ready(
        child_guard
            .child
            .stdout
            .take()
            .expect("capture checkpoint child stdout"),
    )
    .expect("wait for checkpoint child readiness");
    let process_id = child_guard.child.id() as c_int;
    let mut stdin = child_guard
        .child
        .stdin
        .take()
        .expect("capture checkpoint child stdin");

    let status = kill_after_checkpoint_database_io(
        trace_tid,
        process_id,
        &database_path,
        boundary,
        &mut stdin,
    )
    .unwrap_or_else(|error| panic!("trace and kill after checkpoint {boundary}: {error}"));
    child_guard.reaped = true;
    assert!(
        is_signaled(status),
        "child status {status:#x} was not signaled"
    );
    assert_eq!(
        status & 0x7f,
        SIGKILL,
        "child was not killed with SIGKILL after checkpoint {boundary}"
    );

    verify_checkpoint_recovery(&database_path);
}

#[test]
fn checkpoint_crash_child() {
    if std::env::var(CHILD_ENV).as_deref() != Ok("1") {
        return;
    }
    let database_path =
        PathBuf::from(std::env::var_os(DATABASE_ENV).expect("checkpoint child database path"));
    let boundary = std::env::var(BOUNDARY_ENV).expect("checkpoint child I/O boundary");
    assert!(boundary == "write" || boundary == "sync");
    let mut database = Connection::open(&database_path).expect("open checkpoint child database");
    database
        .execute_batch(
            "PRAGMA page_size=4096;
             PRAGMA journal_mode=WAL;
             PRAGMA wal_autocheckpoint=0;
             PRAGMA synchronous=FULL;
             CREATE TABLE records (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
        )
        .expect("initialize checkpoint child database");
    let transaction = database
        .transaction()
        .expect("begin checkpoint child data transaction");
    let payload = vec![0_u8; 3000];
    for id in 1..=512 {
        transaction
            .execute(
                "INSERT INTO records(id, payload) VALUES (?1, ?2)",
                rusqlite::params![id, payload],
            )
            .unwrap_or_else(|error| panic!("insert checkpoint child row {id}: {error}"));
    }
    transaction.commit().expect("commit checkpoint child data");
    let mut wal_path = database_path.as_os_str().to_os_string();
    wal_path.push("-wal");
    let wal_path = PathBuf::from(wal_path);
    let wal_size = fs::metadata(&wal_path)
        .expect("stat checkpoint child WAL")
        .len();
    assert!(
        wal_size > 32,
        "checkpoint child WAL size {wal_size} has no frames"
    );

    let trace_tid = unsafe { gettid() };
    writeln!(io::stdout().lock(), "{CHILD_READY} {trace_tid}")
        .expect("signal checkpoint child readiness");
    let mut release = [0_u8; 1];
    io::stdin()
        .read_exact(&mut release)
        .expect("wait for parent to attach syscall tracer");

    let checkpoint: (i64, i64, i64) = database
        .query_row("PRAGMA wal_checkpoint(FULL)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("run real FULL checkpoint in child");
    panic!("checkpoint completed before {boundary} interruption: {checkpoint:?}");
}

fn wait_for_child_ready(stdout: impl Read + Send + 'static) -> io::Result<c_int> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let fields: Vec<&str> = line.split_whitespace().collect();
                    if fields.len() == 2 && fields[0] == CHILD_READY {
                        let result = fields[1].parse::<c_int>().map_err(io::Error::other);
                        let _ = sender.send(result);
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(error));
                    return;
                }
            }
        }
        let _ = sender.send(Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "checkpoint child exited before readiness",
        )));
    });
    receiver
        .recv_timeout(Duration::from_secs(15))
        .map_err(io::Error::other)?
}

fn kill_after_checkpoint_database_io(
    trace_tid: c_int,
    process_id: c_int,
    database_path: &Path,
    boundary: &str,
    stdin: &mut impl Write,
) -> io::Result<c_int> {
    ptrace_call(
        PTRACE_ATTACH,
        trace_tid,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    )?;
    let initial_status = wait_for_trace_status(trace_tid)?;
    if !is_stopped(initial_status) || stop_signal(initial_status) != SIGSTOP {
        return Err(io::Error::other(format!(
            "checkpoint child initial status {initial_status:#x} was not SIGSTOP"
        )));
    }
    ptrace_call(
        PTRACE_SETOPTIONS,
        trace_tid,
        std::ptr::null_mut(),
        PTRACE_O_TRACESYSGOOD as *mut c_void,
    )?;
    stdin.write_all(&[1])?;

    let mut database_write_pending = false;
    let mut signal_to_deliver = 0;
    loop {
        ptrace_call(
            PTRACE_SYSCALL,
            trace_tid,
            std::ptr::null_mut(),
            signal_to_deliver as usize as *mut c_void,
        )?;
        signal_to_deliver = 0;
        let status = wait_for_trace_status(trace_tid)?;
        if !is_stopped(status) {
            return Err(io::Error::other(format!(
                "checkpoint child exited before database write: status {status:#x}"
            )));
        }
        let signal = stop_signal(status);
        if signal == (SIGTRAP | 0x80) {
            let info = read_syscall_info(trace_tid)?;
            match info.operation {
                PTRACE_SYSCALL_INFO_ENTRY => {
                    database_write_pending =
                        is_database_io(process_id, database_path, boundary, &info);
                }
                PTRACE_SYSCALL_INFO_EXIT => {
                    let completed = if boundary == "write" {
                        info.data[0] as i64 > 0
                    } else {
                        info.data[0] as i64 == 0
                    };
                    if database_write_pending && completed {
                        ptrace_call(
                            PTRACE_SYSCALL,
                            trace_tid,
                            std::ptr::null_mut(),
                            SIGKILL as usize as *mut c_void,
                        )?;
                        return wait_for_trace_status(trace_tid);
                    }
                    database_write_pending = false;
                }
                _ => {}
            }
        } else {
            signal_to_deliver = signal;
        }
    }
}

fn wait_for_trace_status(trace_tid: c_int) -> io::Result<c_int> {
    loop {
        let mut status = 0;
        let result = unsafe { waitpid(trace_tid, &mut status, WAIT_ALL_THREADS) };
        if result == trace_tid {
            return Ok(status);
        }
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        return Err(io::Error::other(format!(
            "waitpid returned {result}, expected trace thread {trace_tid}"
        )));
    }
}

fn read_syscall_info(trace_tid: c_int) -> io::Result<PtraceSyscallInfo> {
    let mut info = PtraceSyscallInfo {
        operation: 0,
        padding: [0; 3],
        architecture: 0,
        instruction_pointer: 0,
        stack_pointer: 0,
        data: [0; 8],
    };
    ptrace_call(
        PTRACE_GET_SYSCALL_INFO,
        trace_tid,
        std::mem::size_of::<PtraceSyscallInfo>() as usize as *mut c_void,
        (&mut info as *mut PtraceSyscallInfo).cast(),
    )?;
    Ok(info)
}

fn is_database_io(
    process_id: c_int,
    database_path: &Path,
    boundary: &str,
    info: &PtraceSyscallInfo,
) -> bool {
    let syscall_numbers = database_io_syscall_numbers(boundary);
    if !syscall_numbers.contains(&info.data[0]) {
        return false;
    }
    let file_descriptor = info.data[1] as c_int;
    let descriptor_path = format!("/proc/{process_id}/fd/{file_descriptor}");
    match fs::read_link(descriptor_path) {
        Ok(opened_path) => opened_path == database_path,
        Err(_) => false,
    }
}

#[cfg(target_arch = "x86_64")]
fn database_io_syscall_numbers(boundary: &str) -> [u64; 2] {
    match boundary {
        "write" => [1, 18],
        "sync" => [74, 75],
        _ => [0, 0],
    }
}

#[cfg(target_arch = "aarch64")]
fn database_io_syscall_numbers(boundary: &str) -> [u64; 2] {
    match boundary {
        "write" => [64, 68],
        "sync" => [82, 83],
        _ => [0, 0],
    }
}

fn ptrace_call(
    request: c_int,
    trace_tid: c_int,
    address: *mut c_void,
    data: *mut c_void,
) -> io::Result<c_long> {
    let result = unsafe { ptrace(request, trace_tid, address, data) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(result)
}

fn is_stopped(status: c_int) -> bool {
    status & 0xff == 0x7f
}

fn stop_signal(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

fn is_signaled(status: c_int) -> bool {
    let signal = status & 0x7f;
    signal != 0 && signal != 0x7f
}

fn verify_checkpoint_recovery(database_path: &Path) {
    let database = Connection::open(database_path).expect("reopen database after crash");
    let integrity: String = database
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("run integrity_check after crash");
    assert_eq!(integrity, "ok");
    let (rows, invalid_payloads): (i64, i64) = database
        .query_row(
            "SELECT count(*), count(*) FILTER (WHERE length(payload) != 3000) FROM records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read recovered checkpoint rows");
    assert_eq!(rows, 512);
    assert_eq!(invalid_payloads, 0);
    let (busy, log_frames, checkpointed): (i64, i64, i64) = database
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("finish checkpoint after crash recovery");
    assert_eq!(busy, 0);
    assert!(log_frames < 0 || checkpointed == log_frames);
}
