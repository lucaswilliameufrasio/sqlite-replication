use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::Seek;
use std::path::Path;

use rusqlite::Connection;
use sqlite_replication_core::{restore_if_missing, snapshot};
use sqlite_replication_ltx::{Header, Page, decode_snapshot, encode_delta, encode_snapshot_at};
use sqlite_replication_sqlite::verify_database;
use sqlite_replication_storage_fs::{promote_if_missing, sync_directory};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().ok_or_else(usage_error)?;
    let first = arguments.next().ok_or_else(usage_error)?;
    match command.as_str() {
        "create" => {
            if arguments.next().is_some() {
                return Err(usage_error().into());
            }
            create_database(Path::new(&first))
        }
        "snapshot" => {
            let repository = arguments.next().ok_or_else(usage_error)?;
            if arguments.next().is_some() {
                return Err(usage_error().into());
            }
            snapshot(first, repository)?;
            Ok(())
        }
        "encode-snapshot-at" => {
            let output = arguments.next().ok_or_else(usage_error)?;
            let txid = arguments.next().ok_or_else(usage_error)?.parse::<u64>()?;
            let timestamp = arguments.next().ok_or_else(usage_error)?.parse::<i64>()?;
            if arguments.next().is_some() {
                return Err(usage_error().into());
            }
            encode_snapshot_at_file(Path::new(&first), Path::new(&output), txid, timestamp)
        }
        "create-delta" => {
            if arguments.next().is_some() {
                return Err(usage_error().into());
            }
            create_delta(Path::new(&first))
        }
        "restore" => {
            let destination = arguments.next().ok_or_else(usage_error)?;
            if arguments.next().is_some() {
                return Err(usage_error().into());
            }
            restore_if_missing(first, destination)?;
            Ok(())
        }
        "decode-ltx" => {
            let destination = arguments.next().ok_or_else(usage_error)?;
            if arguments.next().is_some() {
                return Err(usage_error().into());
            }
            decode_ltx(Path::new(&first), Path::new(&destination))
        }
        "verify" => {
            if arguments.next().is_some() {
                return Err(usage_error().into());
            }
            verify_contents(Path::new(&first))
        }
        _ => Err(usage_error().into()),
    }
}

fn encode_snapshot_at_file(
    database_path: &Path,
    output_path: &Path,
    txid: u64,
    timestamp_millis: i64,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut database = File::open(database_path)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)?;
    encode_snapshot_at(&mut database, &mut output, txid, timestamp_millis)?;
    output.sync_all()?;
    Ok(())
}

fn create_delta(path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut page_one = vec![0x11; 512];
    page_one[..16].copy_from_slice(b"SQLite format 3\0");
    page_one[16] = 2;
    let header = Header {
        flags: 0,
        page_size: 512,
        commit: 3,
        min_txid: 2,
        max_txid: 2,
        timestamp_millis: 1_700_000_000_000,
        pre_apply_checksum: (1_u64 << 63) | 0x1234,
        wal_offset: 32,
        wal_size: 32 + 2 * (24 + 512),
        wal_salt_1: 0x1122_3344,
        wal_salt_2: 0xaabb_ccdd,
        node_id: 0,
    };
    let pages = vec![
        Page {
            page_no: 1,
            data: page_one,
        },
        Page {
            page_no: 3,
            data: vec![0x33; 512],
        },
    ];
    encode_delta(header, &pages, (1_u64 << 63) | 0x9876, &mut file)?;
    file.sync_all()?;
    Ok(())
}

fn create_database(path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut connection = Connection::open(path)?;
    connection.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE records (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
    )?;
    let transaction = connection.transaction()?;
    for id in 1..=128 {
        let payload = format!("value-{id:04}-{}", "x".repeat(1024));
        transaction.execute(
            "INSERT INTO records(id, payload) VALUES (?1, ?2)",
            (id, payload),
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn verify_contents(path: &Path) -> Result<(), Box<dyn Error>> {
    verify_database(path)?;
    let connection = Connection::open(path)?;
    let (count, id_sum, payload_bytes): (i64, i64, i64) = connection.query_row(
        "SELECT count(*), sum(id), sum(length(payload)) FROM records",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if count != 128 || id_sum != 8256 || payload_bytes != 132480 {
        return Err(format!(
            "unexpected restored logical contents: count={count} id_sum={id_sum} payload_bytes={payload_bytes}"
        )
        .into());
    }
    Ok(())
}

fn decode_ltx(source_path: &Path, destination_path: &Path) -> Result<(), Box<dyn Error>> {
    let source = File::open(source_path)?;
    let parent = destination_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut restored = tempfile::Builder::new()
        .prefix(".conformance-restore-")
        .suffix(".sqlite")
        .tempfile_in(parent)?;
    let header = decode_snapshot(source, &mut restored)?;
    restored.as_file().sync_all()?;
    let expected_size = u64::from(header.page_size) * u64::from(header.commit);
    if restored.as_file().metadata()?.len() != expected_size {
        return Err("restored SQLite image has an invalid size".into());
    }
    verify_database(restored.path())?;
    restored.as_file_mut().rewind()?;
    promote_if_missing(restored, destination_path)?;
    sync_directory(parent)?;
    Ok(())
}

fn usage_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "usage: replication-conformance <create DATABASE | snapshot DATABASE REPOSITORY | encode-snapshot-at DATABASE LTX TXID TIMESTAMP_MILLIS | create-delta LTX | restore REPOSITORY DESTINATION | decode-ltx LTX DESTINATION | verify DATABASE>",
    )
}
