//! Durable publication and local watermarking for captured LTX transactions.

use std::fs::{self, File};
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlite_replication_ltx::{DatabaseState, Header, decode_delta, decode_snapshot};
use sqlite_replication_storage_fs::{ensure_repository, ltx_path, publish, sync_directory};
use sqlite_replication_wal::scan;
use sqlite_replication_wal::translate::{Position, State, Transaction};
use tempfile::Builder;

const CAPTURE_WATERMARK_VERSION: u32 = 1;
const CHECKSUM_FLAG: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultPoint {
    BeforeStageWrite,
    AfterStageWrite,
    BeforeStageSync,
    AfterStageSync,
    BeforeLtxPublish,
    AfterLtxPublish,
    BeforeWatermark,
    BeforeWatermarkWrite,
    BeforeWatermarkSync,
    AfterWatermarkSync,
    AfterWatermark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Watermark {
    pub version: u32,
    pub page_size: u32,
    pub commit: u32,
    pub txid: u64,
    pub database_checksum: u64,
    pub wal: PersistedPosition,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedPosition {
    pub initialized: bool,
    pub checkpoint_sequence: u32,
    pub salt_1: u32,
    pub salt_2: u32,
    pub frame_count: u32,
    pub checksum_1: u32,
    pub checksum_2: u32,
}

impl From<Position> for PersistedPosition {
    fn from(position: Position) -> Self {
        Self {
            initialized: position.initialized,
            checkpoint_sequence: position.checkpoint_sequence,
            salt_1: position.salt_1,
            salt_2: position.salt_2,
            frame_count: position.frame_count,
            checksum_1: position.checksum_1,
            checksum_2: position.checksum_2,
        }
    }
}

impl From<PersistedPosition> for Position {
    fn from(position: PersistedPosition) -> Self {
        Self {
            initialized: position.initialized,
            checkpoint_sequence: position.checkpoint_sequence,
            salt_1: position.salt_1,
            salt_2: position.salt_2,
            frame_count: position.frame_count,
            checksum_1: position.checksum_1,
            checksum_2: position.checksum_2,
        }
    }
}

/// Syncs and immutably publishes every LTX before atomically advancing the
/// capture watermark. The caller must keep its SQLite locks until this returns.
pub fn publish_transactions<F>(
    root: &Path,
    transactions: &[Transaction],
    state: &State,
    mut hook: F,
) -> io::Result<()>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    ensure_repository(root)?;
    validate_batch(transactions, state)?;

    for (index, transaction) in transactions.iter().enumerate() {
        let destination = ltx_path(
            root,
            transaction.header.min_txid,
            transaction.header.max_txid,
        );
        let parent = destination
            .parent()
            .ok_or_else(|| invalid_input("LTX destination has no parent"))?;
        fs::create_dir_all(parent)?;
        let mut staged = Builder::new()
            .prefix(".capture-")
            .suffix(".stage")
            .tempfile_in(parent)?;
        hook(FaultPoint::BeforeStageWrite, index)?;
        staged.write_all(&transaction.encoded)?;
        hook(FaultPoint::AfterStageWrite, index)?;
        hook(FaultPoint::BeforeStageSync, index)?;
        staged.as_file().sync_all()?;
        hook(FaultPoint::AfterStageSync, index)?;
        hook(FaultPoint::BeforeLtxPublish, index)?;

        let (staged_file, staged_path) = staged.keep().map_err(|error| error.error)?;
        drop(staged_file);
        if let Err(error) = publish(&staged_path, &destination) {
            let _ = fs::remove_file(&staged_path);
            return Err(error);
        }
        hook(FaultPoint::AfterLtxPublish, index)?;
    }

    hook(FaultPoint::BeforeWatermark, transactions.len())?;
    let watermark = Watermark {
        version: CAPTURE_WATERMARK_VERSION,
        page_size: state.database.page_size,
        commit: state.database.commit,
        txid: state.database.txid,
        database_checksum: state.database.checksum,
        wal: state.wal.into(),
    };
    write_watermark(root, watermark, &mut hook, transactions.len())?;
    hook(FaultPoint::AfterWatermark, transactions.len())
}

pub fn read_watermark(root: &Path) -> io::Result<Watermark> {
    let path = root.join("_meta").join("capture-watermark.json");
    let file = File::open(path)?;
    let mut deserializer = serde_json::Deserializer::from_reader(file);
    let watermark = Watermark::deserialize(&mut deserializer).map_err(io::Error::other)?;
    deserializer.end().map_err(io::Error::other)?;
    if watermark.version != CAPTURE_WATERMARK_VERSION
        || watermark.page_size == 0
        || watermark.txid == 0
        || watermark.database_checksum & CHECKSUM_FLAG == 0
        || (watermark.wal.frame_count > 0 && !watermark.wal.initialized)
    {
        return Err(invalid_data(
            "capture watermark is incompatible or incomplete",
        ));
    }
    Ok(watermark)
}

/// Rebuilds the checksum chain and repairs a watermark that lagged an immutable
/// LTX publication. The caller must reconcile this state with the live database
/// image before capture when the WAL generation is no longer present.
pub fn recover(root: &Path, wal_bytes: &[u8]) -> io::Result<State> {
    let snapshot_path = ltx_path(root, 1, 1);
    let snapshot = File::open(&snapshot_path)?;
    let mut restored = Builder::new()
        .prefix(".recover-snapshot-")
        .suffix(".sqlite")
        .tempfile_in(root.join("_meta"))?;
    let snapshot_header = decode_snapshot(snapshot, restored.as_file_mut())?;
    restored.as_file().sync_all()?;
    let mut database = DatabaseState::load(restored.as_file_mut(), 1)?;
    if database.page_size != snapshot_header.page_size || database.commit != snapshot_header.commit
    {
        return Err(invalid_data(
            "base LTX header differs from restored SQLite image",
        ));
    }

    let mut known_positions = std::collections::BTreeMap::new();
    known_positions.insert(
        database.txid,
        (database.page_size, database.commit, database.checksum),
    );
    let deltas = list_delta_files(root)?;
    let mut last_header = None;
    for delta in deltas {
        let file = File::open(&delta.path)?;
        let (header, pages, checksum) = decode_delta(file)?;
        if header.min_txid != delta.min_txid || header.max_txid != delta.max_txid {
            return Err(invalid_data("LTX filename and header positions differ"));
        }
        if database.txid == u64::MAX || header.min_txid != database.txid + 1 {
            return Err(invalid_data("LTX chain has a gap or overlap"));
        }
        database = database.apply_delta(header, &pages, checksum)?;
        known_positions.insert(
            database.txid,
            (database.page_size, database.commit, database.checksum),
        );
        last_header = Some(header);
    }

    let watermark = match read_watermark(root) {
        Ok(watermark) => Some(watermark),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let watermark_txid = watermark.map_or(1, |value| value.txid);
    let mut position = Position::default();
    if let Some(watermark) = watermark {
        let known = known_positions.get(&watermark.txid).ok_or_else(|| {
            invalid_data("capture watermark does not match a verified LTX chain position")
        })?;
        if *known
            != (
                watermark.page_size,
                watermark.commit,
                watermark.database_checksum,
            )
        {
            return Err(invalid_data(
                "capture watermark differs from verified LTX chain state",
            ));
        }
        if watermark.txid == database.txid {
            position = watermark.wal.into();
        }
    }
    if watermark_txid < database.txid {
        position = position_after_published_delta(last_header, wal_bytes)?;
    }
    position = validate_current_wal_position(position, wal_bytes, database.page_size)?;

    let state = State {
        database,
        wal: position,
    };
    let watermark_needs_repair = match watermark {
        Some(watermark) => {
            watermark.txid != state.database.txid
                || watermark.wal != PersistedPosition::from(position)
        }
        None => true,
    };
    if watermark_needs_repair {
        publish_transactions(root, &[], &state, |_, _| Ok(()))?;
    }
    Ok(state)
}

#[derive(Clone, Debug)]
struct DeltaFile {
    min_txid: u64,
    max_txid: u64,
    path: PathBuf,
}

fn list_delta_files(root: &Path) -> io::Result<Vec<DeltaFile>> {
    let directory = root.join("ltx").join("0");
    let mut deltas = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".ltx") {
            continue;
        }
        let (min_txid, max_txid) = parse_ltx_filename(&name)?;
        if min_txid == 1 && max_txid == 1 {
            continue;
        }
        if min_txid == 1 || max_txid < min_txid {
            return Err(invalid_data("invalid LTX chain filename"));
        }
        deltas.push(DeltaFile {
            min_txid,
            max_txid,
            path: entry.path(),
        });
    }
    deltas.sort_by_key(|delta| (delta.min_txid, delta.max_txid));
    Ok(deltas)
}

fn parse_ltx_filename(name: &str) -> io::Result<(u64, u64)> {
    let stem = name
        .strip_suffix(".ltx")
        .ok_or_else(|| invalid_data("invalid LTX chain filename"))?;
    let (minimum, maximum) = stem
        .split_once('-')
        .ok_or_else(|| invalid_data("invalid LTX chain filename"))?;
    if minimum.len() != 16 || maximum.len() != 16 || maximum.contains('-') {
        return Err(invalid_data("invalid LTX chain filename"));
    }
    let min_txid = u64::from_str_radix(minimum, 16)
        .map_err(|_| invalid_data("invalid LTX minimum TXID filename"))?;
    let max_txid = u64::from_str_radix(maximum, 16)
        .map_err(|_| invalid_data("invalid LTX maximum TXID filename"))?;
    Ok((min_txid, max_txid))
}

fn position_after_published_delta(
    header: Option<Header>,
    wal_bytes: &[u8],
) -> io::Result<Position> {
    let Some(header) = header else {
        return Ok(Position::default());
    };
    if header.wal_size == 0
        || header.wal_size < 32
        || usize::try_from(header.wal_size).map_or(true, |size| size > wal_bytes.len())
    {
        return Ok(Position::default());
    }
    let current_scan = scan(wal_bytes)?;
    if current_scan.header.salt_1 != header.wal_salt_1
        || current_scan.header.salt_2 != header.wal_salt_2
    {
        return Ok(Position::default());
    }
    sqlite_replication_wal::translate::position_from_wal_prefix(
        wal_bytes,
        usize::try_from(header.wal_size).map_err(|_| invalid_data("WAL size exceeds memory"))?,
    )
}

fn validate_current_wal_position(
    position: Position,
    wal_bytes: &[u8],
    page_size: u32,
) -> io::Result<Position> {
    if wal_bytes.is_empty() {
        return Ok(Position::default());
    }
    let scanned = scan(wal_bytes)?;
    if !position.initialized
        || position.salt_1 != scanned.header.salt_1
        || position.salt_2 != scanned.header.salt_2
        || position.checkpoint_sequence != scanned.header.checkpoint_sequence
    {
        return Ok(Position::default());
    }
    let frame_size = 24_u64 + u64::from(page_size);
    let prefix_size = 32_u64 + u64::from(position.frame_count) * frame_size;
    let prefix_size = usize::try_from(prefix_size)
        .map_err(|_| invalid_data("persisted WAL boundary exceeds memory"))?;
    if prefix_size > wal_bytes.len() {
        return Err(invalid_data(
            "current WAL is shorter than persisted capture boundary",
        ));
    }
    let observed =
        sqlite_replication_wal::translate::position_from_wal_prefix(wal_bytes, prefix_size)?;
    if observed.frame_count != position.frame_count
        || observed.checksum_1 != position.checksum_1
        || observed.checksum_2 != position.checksum_2
    {
        return Err(invalid_data(
            "current WAL prefix differs from persisted capture boundary",
        ));
    }
    Ok(position)
}

fn write_watermark<F>(
    root: &Path,
    watermark: Watermark,
    hook: &mut F,
    transaction_index: usize,
) -> io::Result<()>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    let metadata_directory = root.join("_meta");
    fs::create_dir_all(&metadata_directory)?;
    let mut staged = Builder::new()
        .prefix(".capture-watermark-")
        .suffix(".tmp")
        .tempfile_in(&metadata_directory)?;
    hook(FaultPoint::BeforeWatermarkWrite, transaction_index)?;
    serde_json::to_writer_pretty(staged.as_file_mut(), &watermark).map_err(io::Error::other)?;
    staged.write_all(b"\n")?;
    hook(FaultPoint::BeforeWatermarkSync, transaction_index)?;
    staged.as_file().sync_all()?;
    hook(FaultPoint::AfterWatermarkSync, transaction_index)?;

    let destination = metadata_directory.join("capture-watermark.json");
    let staged_path = staged.into_temp_path();
    fs::rename(&staged_path, &destination)?;
    sync_directory(&metadata_directory)?;
    Ok(())
}

fn validate_batch(transactions: &[Transaction], state: &State) -> io::Result<()> {
    if state.database.page_size == 0
        || state.database.txid == 0
        || state.database.checksum & CHECKSUM_FLAG == 0
    {
        return Err(invalid_data("capture state is incomplete"));
    }
    let mut previous_txid: Option<u64> = None;
    for (index, transaction) in transactions.iter().enumerate() {
        if transaction.header.min_txid == 0
            || transaction.header.max_txid < transaction.header.min_txid
            || previous_txid
                .is_some_and(|txid| txid.checked_add(1) != Some(transaction.header.min_txid))
        {
            return Err(invalid_data("capture batch has invalid transaction IDs"));
        }
        let (header, pages, checksum) = decode_delta(Cursor::new(&transaction.encoded))?;
        if header != transaction.header
            || pages != transaction.pages
            || checksum != transaction.post_checksum
        {
            return Err(invalid_data("encoded LTX differs from capture transaction"));
        }
        previous_txid = Some(transaction.header.max_txid);
        if index + 1 == transactions.len()
            && (transaction.header.max_txid != state.database.txid
                || transaction.header.commit != state.database.commit
                || transaction.post_checksum != state.database.checksum)
        {
            return Err(invalid_data("watermark differs from final LTX transaction"));
        }
    }
    Ok(())
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn invalid_input(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;

    use sqlite_replication_ltx::{DatabaseState, Header, Page};
    use sqlite_replication_wal::translate::{Position, State, Transaction};

    use super::{Builder, FaultPoint, publish_transactions, read_watermark, recover};

    #[test]
    fn publication_orders_ltx_before_watermark_and_survives_injected_failures() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let (transaction, state) = sample_capture();
        publish_transactions(
            directory.path(),
            std::slice::from_ref(&transaction),
            &state,
            |_, _| Ok(()),
        )
        .expect("publish capture transaction");
        let watermark = read_watermark(directory.path()).expect("read watermark");
        assert_eq!(watermark.txid, state.database.txid);
        assert_eq!(watermark.database_checksum, state.database.checksum);
        assert_eq!(Position::from(watermark.wal), state.wal);

        for point in [
            FaultPoint::AfterStageSync,
            FaultPoint::BeforeLtxPublish,
            FaultPoint::AfterLtxPublish,
            FaultPoint::BeforeWatermark,
            FaultPoint::AfterWatermarkSync,
        ] {
            let fault_directory = tempfile::tempdir().expect("fault directory");
            let result = publish_transactions(
                fault_directory.path(),
                std::slice::from_ref(&transaction),
                &state,
                |actual, _| {
                    if actual == point {
                        return Err(std::io::Error::other("injected fault"));
                    }
                    Ok(())
                },
            );
            assert!(result.is_err(), "fault was not injected at {point:?}");
            let path = sqlite_replication_storage_fs::ltx_path(
                fault_directory.path(),
                transaction.header.min_txid,
                transaction.header.max_txid,
            );
            match point {
                FaultPoint::AfterStageSync | FaultPoint::BeforeLtxPublish => {
                    assert!(!path.exists(), "LTX published before its sync boundary");
                }
                _ => assert!(path.exists(), "durable LTX was lost after publication"),
            }
            assert!(read_watermark(fault_directory.path()).is_err());
        }

        for (point, published_ltx) in [
            (FaultPoint::BeforeStageWrite, false),
            (FaultPoint::AfterStageWrite, false),
            (FaultPoint::BeforeStageSync, false),
            (FaultPoint::BeforeWatermarkWrite, true),
            (FaultPoint::BeforeWatermarkSync, true),
        ] {
            let fault_directory = tempfile::tempdir().expect("I/O fault directory");
            let result = publish_transactions(
                fault_directory.path(),
                std::slice::from_ref(&transaction),
                &state,
                |actual, _| {
                    if actual == point {
                        let message = match point {
                            FaultPoint::BeforeStageWrite | FaultPoint::BeforeWatermarkWrite => {
                                "write: no space left on device"
                            }
                            _ => "fsync: input/output error",
                        };
                        return Err(std::io::Error::other(message));
                    }
                    Ok(())
                },
            );
            assert!(result.is_err(), "I/O fault was not injected at {point:?}");
            let path = sqlite_replication_storage_fs::ltx_path(
                fault_directory.path(),
                transaction.header.min_txid,
                transaction.header.max_txid,
            );
            assert_eq!(path.exists(), published_ltx, "wrong LTX state at {point:?}");
            assert!(read_watermark(fault_directory.path()).is_err());
        }

        let fault_directory = tempfile::tempdir().expect("post-watermark fault directory");
        let result = publish_transactions(
            fault_directory.path(),
            &[transaction],
            &state,
            |point, _| {
                if point == FaultPoint::AfterWatermark {
                    return Err(std::io::Error::other("lost acknowledgement"));
                }
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(
            read_watermark(fault_directory.path()).unwrap().txid,
            state.database.txid
        );
    }

    #[test]
    fn recovery_repairs_published_ltx_ahead_of_watermark() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path();
        let mut image = vec![0_u8; 1024];
        image[..16].copy_from_slice(b"SQLite format 3\0");
        image[16..18].copy_from_slice(&512_u16.to_be_bytes());
        image[28..32].copy_from_slice(&2_u32.to_be_bytes());
        let base = DatabaseState::load(&mut Cursor::new(&image), 1).expect("load base state");

        sqlite_replication_storage_fs::ensure_repository(root).expect("initialize repository");
        let snapshot_path = sqlite_replication_storage_fs::ltx_path(root, 1, 1);
        let parent = snapshot_path.parent().expect("snapshot parent");
        fs::create_dir_all(parent).expect("create LTX directory");
        let mut staged_snapshot = Builder::new()
            .prefix(".snapshot-")
            .suffix(".stage")
            .tempfile_in(parent)
            .expect("create snapshot staging file");
        sqlite_replication_ltx::encode_snapshot(
            &mut Cursor::new(&image),
            staged_snapshot.as_file_mut(),
            1,
        )
        .expect("encode base snapshot");
        staged_snapshot
            .as_file()
            .sync_all()
            .expect("sync base snapshot");
        let (snapshot_file, snapshot_stage_path) =
            staged_snapshot.keep().expect("keep snapshot stage");
        drop(snapshot_file);
        sqlite_replication_storage_fs::publish(&snapshot_stage_path, &snapshot_path)
            .expect("publish base snapshot");
        publish_transactions(
            root,
            &[],
            &State {
                database: base,
                wal: Position::default(),
            },
            |_, _| Ok(()),
        )
        .expect("publish base watermark");

        let (transaction, state) = sample_capture();
        let result = publish_transactions(
            root,
            std::slice::from_ref(&transaction),
            &state,
            |point, _| {
                if point == FaultPoint::AfterLtxPublish {
                    return Err(std::io::Error::other("crash before watermark"));
                }
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(read_watermark(root).unwrap().txid, 1);

        let recovered = recover(root, &[]).expect("recover published delta");
        assert_eq!(recovered.database.txid, 2);
        assert_eq!(recovered.database.checksum, state.database.checksum);
        assert_eq!(read_watermark(root).unwrap().txid, 2);
        let repeated = recover(root, &[]).expect("repeat recovery");
        assert_eq!(repeated.database, recovered.database);
    }

    fn sample_capture() -> (Transaction, State) {
        let mut image = vec![0_u8; 1024];
        image[..16].copy_from_slice(b"SQLite format 3\0");
        image[16..18].copy_from_slice(&512_u16.to_be_bytes());
        image[28..32].copy_from_slice(&2_u32.to_be_bytes());
        let base = DatabaseState::load(&mut Cursor::new(image), 1).expect("load base state");
        let header = Header {
            flags: 0,
            page_size: 512,
            commit: 2,
            min_txid: 2,
            max_txid: 2,
            timestamp_millis: 1,
            pre_apply_checksum: base.checksum,
            wal_offset: 32,
            wal_size: 568,
            wal_salt_1: 1,
            wal_salt_2: 2,
            node_id: 0,
        };
        let pages = vec![Page {
            page_no: 2,
            data: vec![0x56; 512],
        }];
        let updated = base.apply_pages(header, &pages).expect("apply delta");
        let mut encoded = Vec::new();
        sqlite_replication_ltx::encode_delta(header, &pages, updated.checksum, &mut encoded)
            .expect("encode delta");
        (
            Transaction {
                start_frame: 1,
                end_frame: 1,
                header,
                pages,
                post_checksum: updated.checksum,
                encoded,
            },
            State {
                database: updated,
                wal: Position {
                    initialized: true,
                    checkpoint_sequence: 1,
                    salt_1: 1,
                    salt_2: 2,
                    frame_count: 1,
                    checksum_1: 3,
                    checksum_2: 4,
                },
            },
        )
    }
}
