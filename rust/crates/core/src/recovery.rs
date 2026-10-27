//! Private restore, compaction, and retention workflows for immutable LTX data.
//! S3 operations stage objects in a temporary filesystem repository and reuse
//! the same validated planner; this module is not wired into the public API.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;

use sha2::{Digest, Sha256};
use sqlite_replication_ltx::{DatabaseState, decode_delta, decode_snapshot, encode_snapshot_at};
use sqlite_replication_sqlite::verify_database;
use sqlite_replication_storage_fs::{promote_if_missing, read_repository, sync_directory};
use sqlite_replication_storage_s3::S3Store;
use tokio::io::AsyncReadExt;
const CHECKSUM_FLAG: u64 = 1 << 63;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Segment {
    pub key: String,
    pub level: u32,
    pub min_txid: u64,
    pub max_txid: u64,
    pub timestamp_millis: i64,
    pub page_size: u32,
    pub commit: u32,
    pub pre_checksum: u64,
    pub post_checksum: u64,
    pub snapshot: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RestoreTarget {
    Txid(u64),
    AtOrBeforeTimestampMillis(i64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestorePlan {
    pub segments: Vec<Segment>,
    pub target_txid: u64,
    pub timestamp_millis: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GCObject {
    pub key: String,
    pub size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GCPlan {
    pub required_targets: Vec<RestoreTarget>,
    pub retained_keys: Vec<String>,
    pub candidates: Vec<GCObject>,
    pub inventory_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotPolicy {
    pub every_txids: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct S3GCPlan {
    pub prefix: String,
    pub plan: GCPlan,
    pub retained_object_keys: Vec<String>,
    pub candidate_object_keys: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlanError {
    InvalidInventory,
    NoRestorePoint,
    AmbiguousChain,
}

struct Candidate {
    plan: RestorePlan,
    base_level: u32,
}

pub(crate) fn build_restore_plan(
    inventory: &[Segment],
    target: RestoreTarget,
) -> Result<RestorePlan, PlanError> {
    if matches!(target, RestoreTarget::Txid(0)) {
        return Err(PlanError::InvalidInventory);
    }
    if inventory.is_empty() {
        return Err(PlanError::NoRestorePoint);
    }

    let mut keys = HashSet::with_capacity(inventory.len());
    let mut snapshots = Vec::new();
    let mut deltas = Vec::new();
    for segment in inventory {
        validate_segment(segment)?;
        if !keys.insert(segment.key.as_str()) {
            return Err(PlanError::InvalidInventory);
        }
        if segment.snapshot {
            snapshots.push(segment.clone());
        } else {
            deltas.push(segment.clone());
        }
    }
    if snapshots.is_empty() {
        return Err(PlanError::NoRestorePoint);
    }
    snapshots.sort_by(|left, right| {
        right
            .max_txid
            .cmp(&left.max_txid)
            .then_with(|| right.level.cmp(&left.level))
    });
    deltas.sort_by(|left, right| {
        left.min_txid
            .cmp(&right.min_txid)
            .then_with(|| left.max_txid.cmp(&right.max_txid))
            .then_with(|| left.key.cmp(&right.key))
    });

    let mut candidates = Vec::new();
    for snapshot in snapshots {
        let mut segments = vec![snapshot.clone()];
        let mut points = vec![snapshot.clone()];
        let mut current_txid = snapshot.max_txid;
        let mut current_checksum = snapshot.post_checksum;
        while current_txid < u64::MAX {
            let mut next = None;
            for delta in &deltas {
                if delta.min_txid == current_txid + 1 && delta.pre_checksum == current_checksum {
                    if next.is_some() {
                        return Err(PlanError::AmbiguousChain);
                    }
                    next = Some(delta);
                }
            }
            let Some(next) = next else {
                break;
            };
            segments.push(next.clone());
            points.push(next.clone());
            current_txid = next.max_txid;
            current_checksum = next.post_checksum;
        }

        match target {
            RestoreTarget::Txid(txid) => {
                if let Some((index, point)) = points
                    .iter()
                    .enumerate()
                    .find(|(_, point)| point.max_txid == txid)
                {
                    candidates.push(Candidate {
                        plan: RestorePlan {
                            segments: segments[..=index].to_vec(),
                            target_txid: point.max_txid,
                            timestamp_millis: point.timestamp_millis,
                        },
                        base_level: snapshot.level,
                    });
                }
            }
            RestoreTarget::AtOrBeforeTimestampMillis(timestamp) => {
                if let Some((index, point)) = points
                    .iter()
                    .enumerate()
                    .filter(|(_, point)| point.timestamp_millis <= timestamp)
                    .max_by_key(|(_, point)| point.max_txid)
                {
                    candidates.push(Candidate {
                        plan: RestorePlan {
                            segments: segments[..=index].to_vec(),
                            target_txid: point.max_txid,
                            timestamp_millis: point.timestamp_millis,
                        },
                        base_level: snapshot.level,
                    });
                }
            }
        }
    }

    if candidates.is_empty() {
        return Err(PlanError::NoRestorePoint);
    }
    candidates.sort_by(|left, right| {
        right
            .plan
            .target_txid
            .cmp(&left.plan.target_txid)
            .then_with(|| right.base_level.cmp(&left.base_level))
    });
    if candidates.get(1).is_some_and(|second| {
        candidates[0].plan.target_txid == second.plan.target_txid
            && candidates[0].base_level == second.base_level
    }) {
        return Err(PlanError::AmbiguousChain);
    }
    Ok(candidates.remove(0).plan)
}

pub(crate) fn build_restore_plan_from_filesystem(
    root: &Path,
    target: RestoreTarget,
) -> io::Result<RestorePlan> {
    read_repository(root)?;
    let ltx_root = root.join("ltx");
    let mut levels = fs::read_dir(&ltx_root)?.collect::<Result<Vec<_>, _>>()?;
    levels.sort_by_key(|entry| entry.file_name());
    let mut inventory = Vec::new();
    for level_entry in levels {
        let file_type = level_entry.file_type()?;
        if !file_type.is_dir() {
            continue;
        }
        let level_name = level_entry.file_name();
        let level = level_name
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
            .ok_or_else(|| invalid_data("invalid LTX level directory"))?;
        let mut files = fs::read_dir(level_entry.path())?.collect::<Result<Vec<_>, _>>()?;
        files.sort_by_key(|entry| entry.file_name());
        for entry in files {
            let file_type = entry.file_type()?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if file_type.is_dir() || !name.ends_with(".ltx") {
                continue;
            }
            if !file_type.is_file() {
                return Err(invalid_data("LTX object is not a regular file"));
            }
            inventory.push(inspect_file(&entry.path(), level, name)?);
        }
    }
    build_restore_plan(&inventory, target).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("restore plan: {error:?}"),
        )
    })
}

pub(crate) fn restore_to_target(
    root: &Path,
    destination: &Path,
    target: RestoreTarget,
) -> io::Result<()> {
    let plan = build_restore_plan_from_filesystem(root, target)?;
    let base = plan
        .segments
        .first()
        .filter(|segment| segment.snapshot)
        .ok_or_else(|| invalid_data("restore plan has no full snapshot"))?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    for suffix in ["-wal", "-shm"] {
        if destination
            .with_file_name(format!(
                "{}{}",
                destination
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                suffix
            ))
            .exists()
        {
            return Err(invalid_data(
                "restore destination has a stale SQLite sidecar",
            ));
        }
    }

    let mut restored = tempfile::Builder::new()
        .prefix(".restore-")
        .suffix(".sqlite")
        .tempfile_in(parent)?;
    let snapshot_path = root.join(&base.key);
    let mut snapshot = File::open(snapshot_path)?;
    let header = decode_snapshot(&mut snapshot, restored.as_file_mut())?;
    if header.max_txid != base.max_txid
        || header.page_size != base.page_size
        || header.commit != base.commit
    {
        return Err(invalid_data("restore base changed after planning"));
    }
    let mut state = DatabaseState::load(restored.as_file_mut(), header.max_txid)?;

    for segment in plan.segments.iter().skip(1) {
        let delta_path = root.join(&segment.key);
        let delta = File::open(delta_path)?;
        let (header, pages, post_checksum) = decode_delta(delta)?;
        if header.min_txid != segment.min_txid
            || header.max_txid != segment.max_txid
            || post_checksum != segment.post_checksum
        {
            return Err(invalid_data("restore delta changed after planning"));
        }
        let updated = state.apply_delta(header, &pages, post_checksum)?;
        for page in &pages {
            let offset = u64::from(page.page_no - 1) * u64::from(header.page_size);
            restored.as_file_mut().seek(SeekFrom::Start(offset))?;
            restored.as_file_mut().write_all(&page.data)?;
        }
        let expected_size = u64::from(header.commit) * u64::from(header.page_size);
        restored.as_file_mut().set_len(expected_size)?;
        let actual = DatabaseState::load(restored.as_file_mut(), updated.txid)?;
        if actual.page_size != updated.page_size
            || actual.commit != updated.commit
            || actual.txid != updated.txid
            || actual.checksum != updated.checksum
        {
            return Err(invalid_data(
                "applied delta differs from validated LTX state",
            ));
        }
        state = updated;
    }
    if state.txid != plan.target_txid {
        return Err(invalid_data(
            "restore plan target differs from reconstructed state",
        ));
    }
    restored.as_file().sync_all()?;
    verify_database(restored.path())?;
    super::remove_sqlite_sidecars(restored.path())?;
    promote_if_missing(restored, destination)?;
    sync_directory(parent)
}

pub(crate) fn compact_to_snapshot(
    root: &Path,
    target: RestoreTarget,
) -> io::Result<std::path::PathBuf> {
    let plan = build_restore_plan_from_filesystem(root, target)?;
    let level_root = root.join("ltx");
    let mut maximum_level = 0_u32;
    for entry in fs::read_dir(&level_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let level = entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| invalid_data("invalid LTX level directory"))?;
        maximum_level = maximum_level.max(level);
    }
    let target_level = maximum_level
        .checked_add(1)
        .ok_or_else(|| invalid_data("LTX level exhausted"))?;
    let final_path = root
        .join("ltx")
        .join(target_level.to_string())
        .join(format!("{:016x}-{:016x}.ltx", 1, plan.target_txid));
    let parent = final_path
        .parent()
        .ok_or_else(|| invalid_data("compacted snapshot has no parent"))?;
    fs::create_dir_all(parent)?;

    let restore_directory = tempfile::tempdir()?;
    let restored_path = restore_directory.path().join("compacted.sqlite");
    restore_to_target(root, &restored_path, target)?;
    let mut database = File::open(&restored_path)?;
    let database_size = database.metadata()?.len();
    let mut staged = tempfile::Builder::new()
        .prefix(".compact-")
        .suffix(".ltx")
        .tempfile_in(parent)?;
    encode_snapshot_at(
        &mut database,
        staged.as_file_mut(),
        plan.target_txid,
        plan.timestamp_millis,
    )?;
    staged.as_file_mut().sync_all()?;
    let mut staged_reader = File::open(staged.path())?;
    let mut decoded = tempfile::tempfile()?;
    let compacted_header = decode_snapshot(&mut staged_reader, &mut decoded)?;
    let compacted_state = DatabaseState::load(&mut decoded, plan.target_txid)?;
    if compacted_header.max_txid != plan.target_txid
        || compacted_header.timestamp_millis != plan.timestamp_millis
        || compacted_state.commit != plan.segments.last().map_or(0, |segment| segment.commit)
        || compacted_state.page_size != plan.segments.last().map_or(0, |segment| segment.page_size)
        || compacted_state.checksum
            != plan
                .segments
                .last()
                .map_or(0, |segment| segment.post_checksum)
        || database.metadata()?.len() != database_size
    {
        return Err(invalid_data("compacted snapshot failed verification"));
    }
    let (_, staged_path) = staged.keep().map_err(|error| error.error)?;
    match sqlite_replication_storage_fs::publish(&staged_path, &final_path) {
        Ok(()) => {}
        Err(error) => {
            let _ = fs::remove_file(&staged_path);
            return Err(error);
        }
    }
    Ok(final_path)
}

pub(crate) fn should_compact(
    last_snapshot_txid: u64,
    current_txid: u64,
    policy: SnapshotPolicy,
) -> io::Result<bool> {
    if policy.every_txids == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "snapshot cadence must be positive",
        ));
    }
    let distance = current_txid
        .checked_sub(last_snapshot_txid)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "current TXID precedes last snapshot TXID",
            )
        })?;
    Ok(distance >= policy.every_txids)
}

pub(crate) fn compact_if_due(
    root: &Path,
    last_snapshot_txid: u64,
    current_txid: u64,
    policy: SnapshotPolicy,
) -> io::Result<Option<std::path::PathBuf>> {
    if !should_compact(last_snapshot_txid, current_txid, policy)? {
        return Ok(None);
    }
    compact_to_snapshot(root, RestoreTarget::Txid(current_txid)).map(Some)
}

pub(crate) async fn restore_s3_to_target(
    store: &S3Store,
    prefix: &str,
    destination: &Path,
    target: RestoreTarget,
) -> io::Result<()> {
    let staging = stage_s3_repository(store, prefix).await?;
    restore_to_target(staging.path(), destination, target)
}

pub(crate) async fn compact_s3_to_snapshot(
    store: &S3Store,
    prefix: &str,
    target: RestoreTarget,
) -> io::Result<String> {
    validate_s3_prefix(prefix)?;
    let staging = stage_s3_repository(store, prefix).await?;
    let local_path = compact_to_snapshot(staging.path(), target)?;
    let local_key = local_path
        .strip_prefix(staging.path())
        .map_err(io::Error::other)?
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    let key = local_key_to_s3_key(prefix, &local_key)?;
    let local_file = tokio::fs::File::open(&local_path).await?;
    let receipt = store
        .put_immutable(&key, local_file)
        .await
        .map_err(|_| io::Error::other("publish compacted S3 snapshot failed"))?;
    let output = store
        .get(&key)
        .await
        .map_err(|_| io::Error::other("read back compacted S3 snapshot failed"))?;
    let mut body = output.body.into_async_read();
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = body.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        size = size
            .checked_add(count as u64)
            .ok_or_else(|| io::Error::other("S3 snapshot size overflow"))?;
    }
    let checksum = format!("{:x}", hasher.finalize());
    if size != receipt.size || checksum != receipt.sha256 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "compacted S3 snapshot differs after publication",
        ));
    }
    Ok(key)
}

pub(crate) async fn build_s3_gc_plan(
    store: &S3Store,
    prefix: &str,
    required_targets: &[RestoreTarget],
) -> io::Result<S3GCPlan> {
    validate_s3_prefix(prefix)?;
    let staging = stage_s3_repository(store, prefix).await?;
    let plan = build_gc_plan(staging.path(), required_targets)?;
    let retained_object_keys = plan
        .retained_keys
        .iter()
        .map(|key| local_key_to_s3_key(prefix, key))
        .collect::<io::Result<Vec<_>>>()?;
    let candidate_object_keys = plan
        .candidates
        .iter()
        .map(|object| local_key_to_s3_key(prefix, &object.key))
        .collect::<io::Result<Vec<_>>>()?;
    Ok(S3GCPlan {
        prefix: prefix.to_owned(),
        plan,
        retained_object_keys,
        candidate_object_keys,
    })
}

pub(crate) async fn apply_s3_gc_plan(
    store: &S3Store,
    prefix: &str,
    reviewed: &S3GCPlan,
) -> io::Result<()> {
    if prefix != reviewed.prefix {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "S3 garbage-collection plan belongs to another prefix",
        ));
    }
    let current = build_s3_gc_plan(store, prefix, &reviewed.plan.required_targets).await?;
    if current != *reviewed {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "S3 garbage-collection plan is stale",
        ));
    }
    for keys in current.candidate_object_keys.chunks(1000) {
        store
            .delete_batch(keys)
            .await
            .map_err(|_| io::Error::other("collect unreferenced S3 LTX objects failed"))?;
    }
    Ok(())
}

async fn stage_s3_repository(store: &S3Store, prefix: &str) -> io::Result<tempfile::TempDir> {
    validate_s3_prefix(prefix)?;
    let staging = tempfile::tempdir()?;
    let manifest_key = join_s3_key(prefix, "_meta/repository.json");
    download_s3_object(
        store,
        &manifest_key,
        &staging.path().join("_meta/repository.json"),
    )
    .await?;
    read_repository(staging.path()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "S3 repository metadata is invalid",
        )
    })?;

    let list_prefix = if prefix.is_empty() {
        String::new()
    } else {
        format!("{prefix}/")
    };
    let mut cursor: Option<String> = None;
    let mut ltx_count = 0_usize;
    loop {
        let page = store
            .list_page(&list_prefix, cursor.as_deref(), 1000)
            .await
            .map_err(|_| io::Error::other("list S3 repository objects failed"))?;
        for object in page.contents() {
            let key = object.key().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "S3 object has no key")
            })?;
            if key == manifest_key || !key.ends_with(".ltx") {
                continue;
            }
            let local_key = s3_key_to_local_key(prefix, key)?;
            download_s3_object(store, key, &staging.path().join(&local_key)).await?;
            ltx_count += 1;
        }
        if !page.is_truncated().unwrap_or(false) {
            break;
        }
        let next = page.next_continuation_token().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "S3 listing is truncated without a continuation token",
            )
        })?;
        if cursor.as_deref() == Some(next) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "S3 listing returned a non-advancing cursor",
            ));
        }
        cursor = Some(next.to_owned());
    }
    if ltx_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "S3 repository contains no LTX restore points",
        ));
    }
    Ok(staging)
}

async fn download_s3_object(store: &S3Store, key: &str, destination: &Path) -> io::Result<()> {
    let output = store
        .get(key)
        .await
        .map_err(|_| io::Error::other("download S3 repository object failed"))?;
    let mut body = output.body.into_async_read();
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "S3 object has no parent"))?;
    tokio::fs::create_dir_all(parent).await?;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .await?;
    let result = tokio::io::copy(&mut body, &mut file).await;
    if let Err(error) = result {
        let _ = tokio::fs::remove_file(destination).await;
        return Err(error);
    }
    if let Err(error) = file.sync_all().await {
        let _ = tokio::fs::remove_file(destination).await;
        return Err(error);
    }
    Ok(())
}

fn local_key_to_s3_key(prefix: &str, key: &str) -> io::Result<String> {
    let parts = key.split('/').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "ltx" || !parts[2].ends_with(".ltx") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid local LTX key",
        ));
    }
    let level = parts[1]
        .parse::<u16>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid local LTX level"))?;
    parse_filename(parts[2])?;
    Ok(join_s3_key(prefix, &format!("{level:04x}/{}", parts[2])))
}

fn s3_key_to_local_key(prefix: &str, key: &str) -> io::Result<String> {
    let relative = if prefix.is_empty() {
        key
    } else {
        key.strip_prefix(&format!("{prefix}/")).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "S3 LTX object is outside repository prefix",
            )
        })?
    };
    let parts = relative.split('/').collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].len() != 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid S3 LTX key",
        ));
    }
    let level = u16::from_str_radix(parts[0], 16)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid S3 LTX level"))?;
    parse_filename(parts[1])?;
    Ok(format!("ltx/{level}/{}", parts[1]))
}

fn join_s3_key(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() {
        suffix.to_owned()
    } else {
        format!("{prefix}/{suffix}")
    }
}

fn validate_s3_prefix(prefix: &str) -> io::Result<()> {
    if prefix.starts_with('/')
        || prefix.ends_with('/')
        || prefix.contains('\0')
        || prefix
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid S3 repository prefix",
        ));
    }
    Ok(())
}

pub(crate) fn build_gc_plan(root: &Path, required_targets: &[RestoreTarget]) -> io::Result<GCPlan> {
    if required_targets.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "at least one explicit retained restore point is required",
        ));
    }
    let mut retained = HashSet::new();
    for target in required_targets {
        let plan = build_restore_plan_from_filesystem(root, *target)?;
        for segment in plan.segments {
            retained.insert(segment.key);
        }
    }
    let repository = read_repository(root)?;
    let (objects, objects_sha256) = list_gc_objects(root)?;
    let mut inventory_hash = Sha256::new();
    inventory_hash.update(repository.repository_id.to_string().as_bytes());
    inventory_hash.update([0]);
    inventory_hash.update(repository.database_id.to_string().as_bytes());
    inventory_hash.update([0]);
    inventory_hash.update(objects_sha256);
    let inventory_sha256 = inventory_hash.finalize().into();
    let mut retained_keys = retained.iter().cloned().collect::<Vec<_>>();
    retained_keys.sort();
    let candidates = objects
        .into_iter()
        .filter(|object| !retained.contains(&object.key))
        .collect();
    Ok(GCPlan {
        required_targets: required_targets.to_vec(),
        retained_keys,
        candidates,
        inventory_sha256,
    })
}

pub(crate) fn apply_gc_plan(root: &Path, reviewed: &GCPlan) -> io::Result<()> {
    if reviewed.required_targets.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GC plan has no explicit retained restore points",
        ));
    }
    let current = build_gc_plan(root, &reviewed.required_targets)?;
    if current.required_targets != reviewed.required_targets
        || current.inventory_sha256 != reviewed.inventory_sha256
        || current.retained_keys != reviewed.retained_keys
        || current.candidates != reviewed.candidates
    {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "garbage-collection plan is stale",
        ));
    }
    for candidate in current.candidates {
        let path = root.join(&candidate.key);
        fs::remove_file(&path)?;
        let parent = path
            .parent()
            .ok_or_else(|| invalid_data("collected LTX object has no parent"))?;
        sync_directory(parent)?;
    }
    Ok(())
}

fn list_gc_objects(root: &Path) -> io::Result<(Vec<GCObject>, [u8; 32])> {
    let mut levels = fs::read_dir(root.join("ltx"))?.collect::<Result<Vec<_>, _>>()?;
    levels.sort_by_key(|entry| entry.file_name());
    let mut objects = Vec::new();
    let mut inventory = Sha256::new();
    for level in levels {
        if !level.file_type()?.is_dir() {
            continue;
        }
        let mut entries = fs::read_dir(level.path())?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.ends_with(".ltx") {
                continue;
            }
            if !entry.file_type()?.is_file() {
                return Err(invalid_data("LTX object is not a regular file"));
            }
            let key = format!("ltx/{}/{name}", level.file_name().to_string_lossy());
            let mut file = File::open(entry.path())?;
            let mut content_hash = Sha256::new();
            let size = io::copy(&mut file, &mut HashWriter(&mut content_hash))?;
            inventory.update(key.as_bytes());
            inventory.update([0]);
            inventory.update(size.to_be_bytes());
            inventory.update(content_hash.finalize());
            objects.push(GCObject { key, size });
        }
    }
    objects.sort_by(|left, right| left.key.cmp(&right.key));
    Ok((objects, inventory.finalize().into()))
}

struct HashWriter<'a>(&'a mut Sha256);

impl Write for HashWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.update(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn inspect_file(path: &Path, level: u32, name: &str) -> io::Result<Segment> {
    let (minimum, maximum) = parse_filename(name)?;
    let mut source = File::open(path)?;
    let (header, post_checksum, snapshot) = if minimum == 1 {
        let mut restored = tempfile::tempfile()?;
        let header = decode_snapshot(&mut source, &mut restored)?;
        let state = DatabaseState::load(&mut restored, header.max_txid)?;
        if state.page_size != header.page_size || state.commit != header.commit {
            return Err(invalid_data("snapshot header differs from decoded image"));
        }
        (header, state.checksum, true)
    } else {
        let (header, _pages, post_checksum) = decode_delta(source)?;
        (header, post_checksum, false)
    };
    if header.min_txid != minimum || header.max_txid != maximum {
        return Err(invalid_data("filename TXID range differs from LTX header"));
    }
    Ok(Segment {
        key: format!("ltx/{level}/{name}"),
        level,
        min_txid: minimum,
        max_txid: maximum,
        timestamp_millis: header.timestamp_millis,
        page_size: header.page_size,
        commit: header.commit,
        pre_checksum: header.pre_apply_checksum,
        post_checksum,
        snapshot,
    })
}

fn parse_filename(name: &str) -> io::Result<(u64, u64)> {
    let Some(stem) = name.strip_suffix(".ltx") else {
        return Err(invalid_data("invalid LTX filename"));
    };
    let Some((minimum, maximum)) = stem.split_once('-') else {
        return Err(invalid_data("invalid LTX filename"));
    };
    if minimum.len() != 16 || maximum.len() != 16 || maximum.contains('-') {
        return Err(invalid_data("invalid LTX filename"));
    }
    let minimum =
        u64::from_str_radix(minimum, 16).map_err(|_| invalid_data("invalid LTX filename TXID"))?;
    let maximum =
        u64::from_str_radix(maximum, 16).map_err(|_| invalid_data("invalid LTX filename TXID"))?;
    Ok((minimum, maximum))
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn validate_segment(segment: &Segment) -> Result<(), PlanError> {
    if segment.key.is_empty()
        || segment.max_txid < segment.min_txid
        || !valid_page_size(segment.page_size)
        || segment.commit == 0
        || segment.post_checksum & CHECKSUM_FLAG == 0
    {
        return Err(PlanError::InvalidInventory);
    }
    if segment.snapshot {
        if segment.min_txid != 1 || segment.pre_checksum != 0 {
            return Err(PlanError::InvalidInventory);
        }
    } else if segment.min_txid <= 1 || segment.pre_checksum & CHECKSUM_FLAG == 0 {
        return Err(PlanError::InvalidInventory);
    }
    Ok(())
}

fn valid_page_size(size: u32) -> bool {
    (512..=65_536).contains(&size) && size.is_power_of_two()
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::{Cursor, Seek, SeekFrom};
    use std::path::Path;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    use aws_credential_types::Credentials;
    use rusqlite::Connection;
    use sqlite_replication_ltx::{
        DatabaseState, encode_delta, encode_snapshot, encode_snapshot_at,
    };
    use sqlite_replication_storage_fs::{ensure_repository, ltx_path, publish};
    use sqlite_replication_storage_s3::S3Store;

    use super::{
        PlanError, RestoreTarget, Segment, SnapshotPolicy, apply_gc_plan, apply_s3_gc_plan,
        build_gc_plan, build_restore_plan, build_restore_plan_from_filesystem, build_s3_gc_plan,
        compact_if_due, compact_s3_to_snapshot, local_key_to_s3_key, restore_s3_to_target,
        restore_to_target, s3_key_to_local_key, should_compact,
    };

    const FLAG: u64 = 1 << 63;

    fn snapshot(key: &str, max_txid: u64, level: u32, checksum: u64) -> Segment {
        Segment {
            key: key.into(),
            level,
            min_txid: 1,
            max_txid,
            timestamp_millis: (max_txid * 100) as i64,
            page_size: 4096,
            commit: 20,
            pre_checksum: 0,
            post_checksum: FLAG | checksum,
            snapshot: true,
        }
    }

    fn delta(
        key: &str,
        min_txid: u64,
        max_txid: u64,
        pre_checksum: u64,
        post_checksum: u64,
        timestamp_millis: i64,
    ) -> Segment {
        Segment {
            key: key.into(),
            level: 0,
            min_txid,
            max_txid,
            timestamp_millis,
            page_size: 4096,
            commit: 20,
            pre_checksum: FLAG | pre_checksum,
            post_checksum: FLAG | post_checksum,
            snapshot: false,
        }
    }

    #[test]
    fn plans_inclusive_txid_from_a_snapshot_and_delta_chain() {
        let plan = build_restore_plan(
            &[
                delta("delta-2", 2, 3, 10, 12, 200),
                snapshot("base", 1, 0, 10),
                delta("delta-4", 4, 4, 12, 13, 400),
            ],
            RestoreTarget::Txid(3),
        )
        .expect("build restore plan");
        assert_eq!(plan.target_txid, 3);
        assert_eq!(
            plan.segments
                .iter()
                .map(|segment| segment.key.as_str())
                .collect::<Vec<_>>(),
            ["base", "delta-2"]
        );
    }

    #[test]
    fn timestamp_selects_latest_txid_at_or_before_inclusive_instant() {
        let plan = build_restore_plan(
            &[
                snapshot("base", 1, 0, 10),
                delta("delta-2", 2, 2, 10, 11, 200),
                delta("delta-3", 3, 4, 11, 12, 300),
            ],
            RestoreTarget::AtOrBeforeTimestampMillis(300),
        )
        .expect("build restore plan");
        assert_eq!(plan.target_txid, 4);
        assert_eq!(plan.timestamp_millis, 300);
    }

    #[test]
    fn rejects_interior_txid_gaps_and_forks() {
        assert_eq!(
            build_restore_plan(
                &[
                    snapshot("base", 1, 0, 10),
                    delta("delta", 2, 4, 10, 11, 400)
                ],
                RestoreTarget::Txid(3)
            ),
            Err(PlanError::NoRestorePoint)
        );
        assert_eq!(
            build_restore_plan(
                &[
                    snapshot("base", 1, 0, 10),
                    delta("delta", 3, 3, 10, 11, 300)
                ],
                RestoreTarget::Txid(3)
            ),
            Err(PlanError::NoRestorePoint)
        );
        assert_eq!(
            build_restore_plan(
                &[
                    snapshot("base", 1, 0, 10),
                    delta("fork-a", 2, 2, 10, 11, 200),
                    delta("fork-b", 2, 2, 10, 12, 200),
                ],
                RestoreTarget::Txid(2)
            ),
            Err(PlanError::AmbiguousChain)
        );
    }

    #[test]
    fn filesystem_inventory_validates_snapshot_without_mutating_it() {
        let directory = tempfile::tempdir().expect("create repository");
        ensure_repository(directory.path()).expect("initialize repository");
        let mut database = vec![0_u8; 4096];
        database[..16].copy_from_slice(b"SQLite format 3\0");
        database[16..18].copy_from_slice(&4096_u16.to_be_bytes());
        database[28..32].copy_from_slice(&1_u32.to_be_bytes());
        let snapshot_path = ltx_path(directory.path(), 1, 7);
        fs::create_dir_all(snapshot_path.parent().expect("snapshot parent"))
            .expect("create LTX level");
        let mut snapshot = File::create(&snapshot_path).expect("create snapshot");
        encode_snapshot_at(&mut Cursor::new(database), &mut snapshot, 7, 1234)
            .expect("encode snapshot at TXID 7");
        drop(snapshot);
        let original = fs::read(&snapshot_path).expect("read original snapshot");
        let plan = build_restore_plan_from_filesystem(directory.path(), RestoreTarget::Txid(7))
            .expect("build restore plan");
        assert_eq!(plan.target_txid, 7);
        assert_eq!(plan.segments.len(), 1);
        assert!(plan.segments[0].snapshot);
        assert_eq!(
            fs::read(&snapshot_path).expect("read snapshot after planning"),
            original
        );
    }

    #[test]
    fn restore_compaction_and_gc_preserve_promised_point() {
        let directory = tempfile::tempdir().expect("create repository");
        ensure_repository(directory.path()).expect("initialize repository");
        let source_directory = tempfile::tempdir().expect("create source directory");
        let source_path = source_directory.path().join("source.sqlite");
        let database = Connection::open(&source_path).expect("open source SQLite database");
        database
            .execute_batch(
                "CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT NOT NULL); INSERT INTO records VALUES (1, 'base');",
            )
            .expect("create base database contents");
        drop(database);

        let mut source = File::open(&source_path).expect("open base image");
        let base_state = DatabaseState::load(&mut source, 1).expect("load base state");
        let base_path = ltx_path(directory.path(), 1, 1);
        fs::create_dir_all(base_path.parent().expect("base parent")).expect("create base level");
        let base_stage = base_path.with_extension("stage");
        let mut base_output = File::create(&base_stage).expect("create base staging file");
        encode_snapshot(&mut source, &mut base_output, 100).expect("encode base snapshot");
        base_output.sync_all().expect("sync base snapshot");
        drop(base_output);
        publish(&base_stage, &base_path).expect("publish base snapshot");

        let database = Connection::open(&source_path).expect("reopen source database");
        database
            .execute("INSERT INTO records VALUES (2, 'delta')", [])
            .expect("write delta contents");
        drop(database);
        let mut target_image = File::open(&source_path).expect("open target image");
        target_image
            .seek(SeekFrom::Start(0))
            .expect("rewind target image");
        let (delta_header, pages, target_state) = base_state
            .delta_to_database_image(&mut target_image, 200)
            .expect("build delta")
            .expect("delta exists");
        let delta_path = ltx_path(
            directory.path(),
            delta_header.min_txid,
            delta_header.max_txid,
        );
        let delta_stage = delta_path.with_extension("stage");
        let mut delta_output = File::create(&delta_stage).expect("create delta staging file");
        encode_delta(
            delta_header,
            &pages,
            target_state.checksum,
            &mut delta_output,
        )
        .expect("encode delta");
        delta_output.sync_all().expect("sync delta");
        drop(delta_output);
        publish(&delta_stage, &delta_path).expect("publish delta");

        let restored_directory = tempfile::tempdir().expect("create restore directory");
        let restored_path = restored_directory.path().join("restored.sqlite");
        restore_to_target(directory.path(), &restored_path, RestoreTarget::Txid(2))
            .expect("restore TXID 2");
        let restored = Connection::open(&restored_path).expect("open restored database");
        let row_count: i64 = restored
            .query_row("SELECT count(*) FROM records", [], |row| row.get(0))
            .expect("count restored records");
        assert_eq!(row_count, 2);
        drop(restored);

        assert_eq!(
            compact_if_due(directory.path(), 1, 2, SnapshotPolicy { every_txids: 2 })
                .expect("no-op before snapshot cadence"),
            None
        );
        let compacted_path =
            compact_if_due(directory.path(), 1, 2, SnapshotPolicy { every_txids: 1 })
                .expect("compact when due")
                .expect("compaction published");
        assert!(compacted_path.exists());
        assert!(base_path.exists(), "compaction removed the source snapshot");
        assert!(delta_path.exists(), "compaction removed the source delta");
        let compacted_plan =
            build_restore_plan_from_filesystem(directory.path(), RestoreTarget::Txid(2))
                .expect("plan compacted restore point");
        assert_eq!(compacted_plan.segments.len(), 1);
        assert_eq!(compacted_plan.segments[0].level, 1);

        let gc_plan =
            build_gc_plan(directory.path(), &[RestoreTarget::Txid(2)]).expect("dry-run GC plan");
        assert_eq!(gc_plan.candidates.len(), 2);
        assert!(base_path.exists());
        apply_gc_plan(directory.path(), &gc_plan).expect("apply revalidated GC plan");
        assert!(!base_path.exists());
        assert!(!delta_path.exists());
        assert!(compacted_path.exists());
        build_restore_plan_from_filesystem(directory.path(), RestoreTarget::Txid(2))
            .expect("promised restore remains available after GC");
    }

    #[test]
    fn gc_requires_explicit_policy_and_rejects_stale_inventory() {
        let directory = tempfile::tempdir().expect("create repository");
        ensure_repository(directory.path()).expect("initialize repository");
        assert!(build_gc_plan(directory.path(), &[]).is_err());

        let mut database = vec![0_u8; 4096];
        database[..16].copy_from_slice(b"SQLite format 3\0");
        database[16..18].copy_from_slice(&4096_u16.to_be_bytes());
        database[28..32].copy_from_slice(&1_u32.to_be_bytes());
        for (level, txid) in [(0, 1), (1, 2)] {
            let path = directory
                .path()
                .join("ltx")
                .join(level.to_string())
                .join(format!("{:016x}-{:016x}.ltx", 1, txid));
            fs::create_dir_all(path.parent().expect("snapshot parent"))
                .expect("create snapshot level");
            let mut file = File::create(&path).expect("create snapshot");
            encode_snapshot_at(
                &mut Cursor::new(database.clone()),
                &mut file,
                txid,
                (txid * 100) as i64,
            )
            .expect("encode snapshot");
        }
        let reviewed = build_gc_plan(directory.path(), &[RestoreTarget::Txid(2)])
            .expect("build dry-run GC plan");
        assert_eq!(reviewed.candidates.len(), 1);
        let new_path = directory
            .path()
            .join("ltx/2/0000000000000001-0000000000000003.ltx");
        fs::create_dir_all(new_path.parent().expect("new snapshot parent"))
            .expect("create new snapshot level");
        let mut new_file = File::create(&new_path).expect("create added snapshot");
        encode_snapshot_at(
            &mut Cursor::new(std::mem::take(&mut database)),
            &mut new_file,
            3,
            300,
        )
        .expect("encode added snapshot");
        assert!(apply_gc_plan(directory.path(), &reviewed).is_err());
    }

    #[test]
    fn snapshot_cadence_requires_explicit_positive_interval() {
        let policy = SnapshotPolicy { every_txids: 5 };
        assert!(should_compact(10, 15, policy).expect("cadence boundary"));
        assert!(!should_compact(10, 14, policy).expect("before cadence"));
        assert!(should_compact(10, 15, SnapshotPolicy { every_txids: 0 }).is_err());
        assert!(should_compact(15, 10, policy).is_err());
    }

    #[test]
    fn s3_key_mapping_round_trips_and_rejects_traversal() {
        let local = s3_key_to_local_key(
            "tenant/db",
            "tenant/db/000a/0000000000000001-0000000000000007.ltx",
        )
        .expect("map S3 key");
        assert_eq!(local, "ltx/10/0000000000000001-0000000000000007.ltx");
        assert_eq!(
            local_key_to_s3_key("tenant/db", &local).expect("map local key"),
            "tenant/db/000a/0000000000000001-0000000000000007.ltx"
        );
        for prefix in ["/absolute", "trailing/", "a//b", "a/../b", "a/./b"] {
            assert!(super::validate_s3_prefix(prefix).is_err(), "{prefix}");
        }
        assert!(s3_key_to_local_key("tenant/db", "tenant/db/../000a/file.ltx").is_err());
    }

    #[tokio::test]
    #[ignore = "requires the isolated Floci service from make test-floci"]
    async fn floci_s3_recovery_restore_compaction_and_gc() {
        let endpoint = std::env::var("SQLITE_REPLICATION_FLOCI_ENDPOINT")
            .expect("Floci endpoint configured by smoke script");
        let run_id = std::env::var("SQLITE_REPLICATION_FLOCI_RUN_ID")
            .expect("Floci run ID configured by smoke script");
        let shared_floci =
            std::env::var("SQLITE_REPLICATION_FLOCI_CROSS_ENGINE").as_deref() == Ok("1");
        let bucket = format!("sqlite-replication-m4-{run_id}");
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(Credentials::new("test", "test", None, None, "m4-test"))
            .retry_config(aws_sdk_s3::config::retry::RetryConfig::standard())
            .build();
        let client = aws_sdk_s3::Client::from_conf(config);
        if !shared_floci {
            client
                .create_bucket()
                .bucket(&bucket)
                .send()
                .await
                .expect("create isolated M4 bucket");
        }
        let store = S3Store::new(client.clone(), &bucket).expect("construct S3 store");
        if shared_floci {
            let go_restored =
                tempfile::tempdir().expect("create Go cross-engine restore directory");
            let go_restored_path = go_restored.path().join("go-restored.sqlite");
            restore_s3_to_target(
                &store,
                "recovery-test",
                &go_restored_path,
                RestoreTarget::Txid(8),
            )
            .await
            .expect("restore Go-published S3 repository with Rust");
            assert_s3_record_value(&go_restored_path, "updated");
        }
        let prefix = if shared_floci {
            "rust-recovery-test"
        } else {
            "recovery-test"
        };

        let source_directory = tempfile::tempdir().expect("create source directory");
        let database_path = source_directory.path().join("source.sqlite");
        let database = Connection::open(&database_path).expect("create source database");
        database
            .execute_batch(
                "CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT NOT NULL); INSERT INTO records VALUES (1, 'source');",
            )
            .expect("write source record");
        drop(database);
        let database_bytes = fs::read(&database_path).expect("read source database");
        let base_state = DatabaseState::load(&mut Cursor::new(database_bytes.as_slice()), 7)
            .expect("load source LTX state");
        let source_repository = tempfile::tempdir().expect("create source repository");
        ensure_repository(source_repository.path()).expect("initialize source repository");
        let source_snapshot = source_repository
            .path()
            .join("ltx/0/0000000000000001-0000000000000007.ltx");
        fs::create_dir_all(source_snapshot.parent().expect("snapshot parent"))
            .expect("create snapshot level");
        let mut snapshot_file = File::create(&source_snapshot).expect("create source snapshot");
        encode_snapshot_at(
            &mut Cursor::new(database_bytes.as_slice()),
            &mut snapshot_file,
            7,
            1_000,
        )
        .expect("encode source snapshot");
        snapshot_file.sync_all().expect("sync source snapshot");

        let manifest_key = super::join_s3_key(prefix, "_meta/repository.json");
        let source_key = super::join_s3_key(prefix, "0000/0000000000000001-0000000000000007.ltx");
        let manifest =
            tokio::fs::File::open(source_repository.path().join("_meta/repository.json"))
                .await
                .expect("open repository metadata");
        store
            .put_immutable(&manifest_key, manifest)
            .await
            .expect("upload repository metadata");
        let snapshot_input = tokio::fs::File::open(&source_snapshot)
            .await
            .expect("open source snapshot");
        store
            .put_immutable(&source_key, snapshot_input)
            .await
            .expect("upload source snapshot");

        let database = Connection::open(&database_path).expect("reopen source database");
        database
            .execute("UPDATE records SET value = 'updated' WHERE id = 1", [])
            .expect("update source record");
        drop(database);
        let updated_bytes = fs::read(&database_path).expect("read updated database");
        let (delta_header, delta_pages, updated_state) = base_state
            .delta_to_database_image(&mut Cursor::new(updated_bytes.as_slice()), 2_000)
            .expect("build LTX delta")
            .expect("database change requires a delta");
        let delta_path = source_repository
            .path()
            .join("ltx/0/0000000000000008-0000000000000008.ltx");
        let mut delta_file = File::create(&delta_path).expect("create source delta");
        sqlite_replication_ltx::encode_delta(
            delta_header,
            &delta_pages,
            updated_state.checksum,
            &mut delta_file,
        )
        .expect("encode source delta");
        delta_file.sync_all().expect("sync source delta");
        let delta_key = super::join_s3_key(prefix, "0000/0000000000000008-0000000000000008.ltx");
        store
            .put_immutable(
                &delta_key,
                tokio::fs::File::open(&delta_path)
                    .await
                    .expect("open source delta"),
            )
            .await
            .expect("upload source delta");

        let restore_directory = tempfile::tempdir().expect("create restore directory");
        let restored_path = restore_directory.path().join("restored.sqlite");
        restore_s3_to_target(&store, prefix, &restored_path, RestoreTarget::Txid(7))
            .await
            .expect("restore S3 TXID");
        assert_s3_record(&restored_path);
        let before_delta = restore_directory.path().join("before-delta.sqlite");
        restore_s3_to_target(
            &store,
            prefix,
            &before_delta,
            RestoreTarget::AtOrBeforeTimestampMillis(1_500),
        )
        .await
        .expect("restore S3 timestamp before delta");
        assert_s3_record(&before_delta);
        let after_delta = restore_directory.path().join("after-delta.sqlite");
        restore_s3_to_target(
            &store,
            prefix,
            &after_delta,
            RestoreTarget::AtOrBeforeTimestampMillis(2_500),
        )
        .await
        .expect("restore S3 timestamp after delta");
        assert_s3_record_value(&after_delta, "updated");

        let (first_compaction, second_compaction) = tokio::join!(
            compact_s3_to_snapshot(&store, prefix, RestoreTarget::Txid(8)),
            compact_s3_to_snapshot(&store, prefix, RestoreTarget::Txid(8)),
        );
        let compacted_keys = [
            first_compaction.expect("first concurrent S3 compaction"),
            second_compaction.expect("second concurrent S3 compaction"),
        ];
        for key in &compacted_keys {
            assert!(key.starts_with(&format!("{prefix}/")));
            assert!(key.ends_with(".ltx"));
        }
        let plan = build_s3_gc_plan(&store, prefix, &[RestoreTarget::Txid(8)])
            .await
            .expect("build S3 dry-run GC plan");
        let delta_key = super::join_s3_key(prefix, "0000/0000000000000008-0000000000000008.ltx");
        assert!(plan.candidate_object_keys.contains(&source_key));
        assert!(plan.candidate_object_keys.contains(&delta_key));
        let original_candidates = plan.candidate_object_keys.clone();

        let extra_key = super::join_s3_key(prefix, "0002/0000000000000001-0000000000000007.ltx");
        store
            .put_immutable(
                &extra_key,
                Cursor::new(fs::read(&source_snapshot).expect("read valid extra snapshot")),
            )
            .await
            .expect("add object to stale the plan");
        assert!(apply_s3_gc_plan(&store, prefix, &plan).await.is_err());
        store
            .delete_batch(std::slice::from_ref(&extra_key))
            .await
            .expect("remove stale-plan fixture");
        crash_after_s3_gc_delete(
            plan.candidate_object_keys
                .first()
                .expect("GC candidate exists"),
        );
        assert!(apply_s3_gc_plan(&store, prefix, &plan).await.is_err());
        let plan = build_s3_gc_plan(&store, prefix, &[RestoreTarget::Txid(8)])
            .await
            .expect("rebuild S3 GC plan after partial collection");
        apply_s3_gc_plan(&store, prefix, &plan)
            .await
            .expect("apply revalidated S3 GC plan");
        for key in original_candidates {
            assert!(
                store.head(&key).await.is_err(),
                "GC candidate remains: {key}"
            );
        }

        let restored_after_gc = restore_directory.path().join("restored-after-gc.sqlite");
        restore_s3_to_target(&store, prefix, &restored_after_gc, RestoreTarget::Txid(8))
            .await
            .expect("restore retained point after S3 GC");
        assert_s3_record_value(&restored_after_gc, "updated");
        let restored_time_after_gc = restore_directory
            .path()
            .join("restored-time-after-gc.sqlite");
        restore_s3_to_target(
            &store,
            prefix,
            &restored_time_after_gc,
            RestoreTarget::AtOrBeforeTimestampMillis(2_500),
        )
        .await
        .expect("restore retained timestamp after S3 GC");
        assert_s3_record_value(&restored_time_after_gc, "updated");

        if !shared_floci {
            let mut cursor = None;
            let mut remaining_keys = Vec::new();
            loop {
                let page = store
                    .list_page(&format!("{prefix}/"), cursor.as_deref(), 1000)
                    .await
                    .expect("list remaining isolated S3 objects");
                remaining_keys.extend(
                    page.contents()
                        .iter()
                        .filter_map(|object| object.key().map(ToOwned::to_owned)),
                );
                if !page.is_truncated().unwrap_or(false) {
                    break;
                }
                cursor = page.next_continuation_token().map(ToOwned::to_owned);
            }
            for keys in remaining_keys.chunks(1000) {
                store
                    .delete_batch(keys)
                    .await
                    .expect("remove isolated recovery objects");
            }
            client
                .delete_bucket()
                .bucket(bucket)
                .send()
                .await
                .expect("delete isolated M4 bucket");
        }
    }

    #[tokio::test]
    #[ignore = "child process for the isolated Floci GC crash test"]
    async fn floci_s3_gc_delete_crash_child() {
        let endpoint = std::env::var("SQLITE_REPLICATION_FLOCI_ENDPOINT")
            .expect("Floci endpoint configured by smoke script");
        let run_id = std::env::var("SQLITE_REPLICATION_FLOCI_RUN_ID")
            .expect("Floci run ID configured by smoke script");
        let key = std::env::var("SQLITE_REPLICATION_FLOCI_DELETE_KEY")
            .expect("GC object key configured for crash child");
        let marker = std::env::var("SQLITE_REPLICATION_FLOCI_DELETE_MARKER")
            .expect("GC completion marker configured for crash child");
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(Credentials::new("test", "test", None, None, "m4-test"))
            .retry_config(aws_sdk_s3::config::retry::RetryConfig::standard())
            .build();
        let client = aws_sdk_s3::Client::from_conf(config);
        let bucket = format!("sqlite-replication-m4-{run_id}");
        let store = S3Store::new(client, &bucket).expect("construct crash-child S3 store");
        store
            .delete_batch(std::slice::from_ref(&key))
            .await
            .expect("delete one GC candidate in crash child");
        fs::write(marker, b"deleted").expect("signal completed delete to parent");
        std::future::pending::<()>().await;
    }

    fn crash_after_s3_gc_delete(key: &str) {
        let marker_directory = tempfile::tempdir().expect("create GC crash marker directory");
        let marker = marker_directory.path().join("delete-complete");
        let mut child = Command::new(std::env::current_exe().expect("current test executable"))
            .args([
                "--ignored",
                "--exact",
                "recovery::tests::floci_s3_gc_delete_crash_child",
            ])
            .env("SQLITE_REPLICATION_FLOCI_DELETE_KEY", key)
            .env("SQLITE_REPLICATION_FLOCI_DELETE_MARKER", &marker)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start S3 GC delete crash child");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if marker.exists() {
                break;
            }
            if let Some(status) = child.try_wait().expect("poll S3 GC crash child") {
                panic!("S3 GC crash child exited before deletion marker: {status}");
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("timed out waiting for S3 GC crash child deletion");
            }
            thread::sleep(Duration::from_millis(10));
        }
        child.kill().expect("kill S3 GC child after partial delete");
        let status = child.wait().expect("wait for killed S3 GC crash child");
        assert!(
            !status.success(),
            "S3 GC crash child exited successfully instead of being killed"
        );
    }

    fn assert_s3_record(path: &Path) {
        assert_s3_record_value(path, "source");
    }

    fn assert_s3_record_value(path: &Path, expected: &str) {
        let database = Connection::open(path).expect("open restored S3 database");
        let value: String = database
            .query_row("SELECT value FROM records WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("read restored S3 record");
        assert_eq!(value, expected);
    }
}
