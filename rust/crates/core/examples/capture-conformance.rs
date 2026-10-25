#![cfg(not(test))]

// Test-only conformance helper. This example includes the private capture
// implementation without exposing it through the library API.
#[allow(dead_code)]
#[path = "../src/capture.rs"]
mod capture;

use std::env;
use std::error::Error;
use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;
use sqlite_replication_core::snapshot;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    let source_path = arguments.next().ok_or_else(usage_error)?;
    let repository_path = arguments.next().ok_or_else(usage_error)?;
    if arguments.next().is_some() {
        return Err(usage_error().into());
    }

    create_base_database(Path::new(&source_path))?;
    snapshot(&source_path, &repository_path)?;

    let mut database = Connection::open(&source_path)?;
    database.busy_timeout(Duration::ZERO)?;
    database.execute_batch("PRAGMA wal_autocheckpoint=0; PRAGMA synchronous=FULL;")?;
    for (index, id) in [1_i64, 2_i64].into_iter().enumerate() {
        let transaction = database.transaction()?;
        let updated_rows = transaction.execute(
            "UPDATE records SET payload = substr(payload, 1, length(payload) - 1) || 'y' WHERE id = ?1",
            [id],
        )?;
        if updated_rows != 1 {
            return Err(format!("capture-chain update changed {updated_rows} rows, want 1").into());
        }
        transaction.commit()?;

        let captured = capture::capture_once(
            Path::new(&source_path),
            Path::new(&repository_path),
            1_700_000_000_001 + index as i64,
        )?;
        let expected_txid = index as u64 + 2;
        if captured.wal_frames == 0
            || captured.transactions.len() != 1
            || captured.state.database.txid != expected_txid
        {
            return Err(format!(
                "capture {} observed {} WAL frames, produced {} LTX files at TXID {}",
                index + 1,
                captured.wal_frames,
                captured.transactions.len(),
                captured.state.database.txid
            )
            .into());
        }
    }

    checkpoint_source(&database)?;
    verify_capture_chain(&database)?;
    Ok(())
}

fn create_base_database(path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut database = Connection::open(path)?;
    database.busy_timeout(Duration::ZERO)?;
    database.execute_batch(
        "PRAGMA page_size=4096;
         PRAGMA journal_mode=WAL;
         PRAGMA wal_autocheckpoint=0;
         PRAGMA synchronous=FULL;
         CREATE TABLE records (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
    )?;
    let transaction = database.transaction()?;
    for id in 1..=128 {
        let payload = format!("value-{id:04}-{}", "x".repeat(1024));
        transaction.execute(
            "INSERT INTO records(id, payload) VALUES (?1, ?2)",
            (id, payload),
        )?;
    }
    transaction.commit()?;
    checkpoint_source(&database)?;
    Ok(())
}

fn checkpoint_source(database: &Connection) -> Result<(), Box<dyn Error>> {
    let (busy, log_frames, checkpointed): (i64, i64, i64) =
        database.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
    if busy != 0 || (log_frames >= 0 && checkpointed != log_frames) {
        return Err(format!(
            "capture-chain checkpoint incomplete: busy={busy} log={log_frames} checkpointed={checkpointed}"
        )
        .into());
    }
    Ok(())
}

fn verify_capture_chain(database: &Connection) -> Result<(), Box<dyn Error>> {
    let (count, id_sum, payload_bytes, updated_rows, unchanged_rows): (i64, i64, i64, i64, i64) =
        database.query_row(
            "SELECT count(*), sum(id), sum(length(payload)),
                    sum(CASE WHEN id IN (1, 2) AND substr(payload, -1, 1) = 'y' THEN 1 ELSE 0 END),
                    sum(CASE WHEN id NOT IN (1, 2) AND substr(payload, -1, 1) = 'x' THEN 1 ELSE 0 END)
             FROM records",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
    if count != 128
        || id_sum != 8256
        || payload_bytes != 132480
        || updated_rows != 2
        || unchanged_rows != 126
    {
        return Err(format!(
            "unexpected capture-chain contents: count={count} id_sum={id_sum} payload_bytes={payload_bytes} updated_rows={updated_rows} unchanged_rows={unchanged_rows}"
        )
        .into());
    }
    Ok(())
}

fn usage_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "usage: capture-conformance SOURCE_DATABASE REPOSITORY",
    )
}
