use std::fs::{self, File};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use sqlite_replication_ltx::DatabaseState;
use sqlite_replication_sqlite::create_online_backup;
use sqlite_replication_wal::translate::{Position, State, translate_wal};

#[test]
fn translates_wal_produced_by_bundled_sqlite_runtime() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    let database = Connection::open_with_flags(
        &database_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .expect("open SQLite database");
    database
        .busy_timeout(Duration::ZERO)
        .expect("disable busy timeout");
    database
        .execute_batch(
            "PRAGMA page_size=512;
             PRAGMA journal_mode=WAL;
             PRAGMA wal_autocheckpoint=0;
             CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT);
             INSERT INTO records VALUES (1, 'base');",
        )
        .expect("initialize source database");
    let journal_mode: String = database
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("read journal mode");
    assert_eq!(journal_mode, "wal");

    let (busy, log_frames, checkpointed): (i64, i64, i64) = database
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("checkpoint initial database");
    assert_eq!(busy, 0);
    assert!(log_frames < 0 || checkpointed == log_frames);

    let base_path = directory.path().join("base.sqlite");
    create_online_backup(&database_path, &base_path).expect("create initial image");
    let mut base_file = File::open(&base_path).expect("open initial image");
    let base_state = DatabaseState::load(&mut base_file, 1).expect("load initial state");

    for statement in [
        "UPDATE records SET value='first' WHERE id=1",
        "INSERT INTO records VALUES (2, 'second')",
        "UPDATE records SET value='third' WHERE id=2",
    ] {
        database
            .execute(statement, [])
            .unwrap_or_else(|error| panic!("execute SQLite transaction {statement:?}: {error}"));
    }

    let wal_path = format!("{}-wal", database_path.display());
    let wal_bytes = fs::read(wal_path).expect("read SQLite WAL");
    let (transactions, state) = translate_wal(
        &wal_bytes,
        &State {
            database: base_state,
            wal: Position::default(),
        },
        200,
    )
    .expect("translate SQLite WAL");
    assert_eq!(transactions.len(), 3);

    let final_path = directory.path().join("final.sqlite");
    create_online_backup(&database_path, &final_path).expect("create final image");
    let mut final_file = File::open(final_path).expect("open final image");
    let final_state =
        DatabaseState::load(&mut final_file, state.database.txid).expect("load final state");
    assert_eq!(state.database.checksum, final_state.checksum);
    assert_eq!(state.database.commit, final_state.commit);
}
