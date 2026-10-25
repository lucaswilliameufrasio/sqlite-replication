//! Versioned filesystem repository with no-overwrite immutable publication.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};
use uuid::{Uuid, Variant};

pub const FORMAT_VERSION: u32 = 2;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repository {
    pub schema_version: u32,
    pub format_version: u32,
    #[serde(deserialize_with = "deserialize_identity")]
    pub repository_id: Uuid,
    #[serde(deserialize_with = "deserialize_identity")]
    pub database_id: Uuid,
}

fn deserialize_identity<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let identity = Uuid::parse_str(&value).map_err(serde::de::Error::custom)?;
    if value.len() != 36
        || ![8, 13, 18, 23]
            .iter()
            .all(|&index| value.as_bytes()[index] == b'-')
        || identity.get_version_num() != 4
        || identity.get_variant() != Variant::RFC4122
    {
        return Err(serde::de::Error::custom(
            "repository identity must be a hyphenated RFC 4122 UUID v4",
        ));
    }
    Ok(identity)
}

pub fn ltx_path(root: &Path, min_txid: u64, max_txid: u64) -> PathBuf {
    root.join("ltx")
        .join("0")
        .join(format!("{min_txid:016x}-{max_txid:016x}.ltx"))
}

pub fn ensure_repository(root: &Path) -> io::Result<()> {
    let metadata_directory = root.join("_meta");
    fs::create_dir_all(&metadata_directory)?;
    let manifest_path = metadata_directory.join("repository.json");
    if manifest_path.exists() {
        read_repository(root)?;
        return sync_directory(&metadata_directory);
    }
    let repository = Repository {
        schema_version: 1,
        format_version: FORMAT_VERSION,
        repository_id: Uuid::new_v4(),
        database_id: Uuid::new_v4(),
    };
    let mut staged = tempfile::Builder::new()
        .prefix(".repository-")
        .suffix(".tmp")
        .tempfile_in(&metadata_directory)?;
    serde_json::to_writer_pretty(&mut staged, &repository).map_err(io::Error::other)?;
    staged.write_all(b"\n")?;
    staged.as_file().sync_all()?;
    let (_, stage_path) = staged.keep().map_err(|error| error.error)?;
    match publish(&stage_path, &manifest_path) {
        Ok(()) => sync_directory(&metadata_directory),
        Err(_error) if manifest_path.exists() => {
            read_repository(root)?;
            let _ = fs::remove_file(stage_path);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

pub fn read_repository(root: &Path) -> io::Result<Repository> {
    let manifest_path = root.join("_meta").join("repository.json");
    let file = File::open(manifest_path)?;
    let repository: Repository = serde_json::from_reader(file).map_err(io::Error::other)?;
    if repository.schema_version != 1 || repository.format_version != FORMAT_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "repository metadata is incompatible with LTX v2",
        ));
    }
    Ok(repository)
}

/// Installs staged content atomically and rejects divergent retries.
pub fn publish(staged_path: &Path, final_path: &Path) -> io::Result<()> {
    let parent = final_path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "immutable key has no parent")
    })?;
    fs::create_dir_all(parent)?;
    match fs::hard_link(staged_path, final_path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if !files_equal(staged_path, final_path)? {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "immutable key exists with different content",
                ));
            }
        }
        Err(error) => return Err(error),
    }
    sync_directory(parent)?;
    fs::remove_file(staged_path)?;
    sync_directory(parent)
}

/// Promotes a verified tempfile only when the destination does not exist.
pub fn promote_if_missing(staged: tempfile::NamedTempFile, destination: &Path) -> io::Result<()> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let stage_path = staged.into_temp_path();
    fs::hard_link(&stage_path, destination)?;
    sync_directory(parent)?;
    stage_path.close()?;
    sync_directory(parent)
}

pub fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

fn files_equal(left_path: &Path, right_path: &Path) -> io::Result<bool> {
    let left_metadata = fs::metadata(left_path)?;
    let right_metadata = fs::metadata(right_path)?;
    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }
    let mut left = File::open(left_path)?;
    let mut right = File::open(right_path)?;
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_size = read_chunk(&mut left, &mut left_buffer)?;
        let right_size = read_chunk(&mut right, &mut right_buffer)?;
        if left_size != right_size || left_buffer[..left_size] != right_buffer[..right_size] {
            return Ok(false);
        }
        if left_size == 0 {
            return Ok(true);
        }
    }
}

fn read_chunk(reader: &mut File, buffer: &mut [u8]) -> io::Result<usize> {
    loop {
        match reader.read(buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{ensure_repository, publish, read_repository};

    #[test]
    fn repository_identity_is_stable() {
        let directory = tempfile::tempdir().expect("temporary directory");
        ensure_repository(directory.path()).expect("initialize repository");
        let first = read_repository(directory.path()).expect("read repository");
        ensure_repository(directory.path()).expect("initialize again");
        let second = read_repository(directory.path()).expect("read again");
        assert_eq!(first, second);
    }

    #[test]
    fn repository_rejects_invalid_identity_without_reinitializing() {
        for identity in [
            "00000000-0000-0000-0000-000000000000",
            "550e8400-e29b-11d4-a716-446655440000",
            "550e8400-e29b-41d4-0716-446655440000",
            "550e8400e29b41d4a716446655440000",
            "urn:uuid:550e8400-e29b-41d4-a716-446655440000",
            "{550e8400-e29b-41d4-a716-446655440000}",
        ] {
            for field in ["repository_id", "database_id"] {
                let directory = tempfile::tempdir().expect("temporary directory");
                ensure_repository(directory.path()).expect("initialize repository");
                let manifest_path = directory.path().join("_meta/repository.json");
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(&fs::read(&manifest_path).expect("read metadata"))
                        .expect("parse metadata");
                manifest[field] = identity.into();
                let content = serde_json::to_vec(&manifest).expect("serialize invalid metadata");
                fs::write(&manifest_path, &content).expect("write invalid metadata");
                assert!(
                    read_repository(directory.path()).is_err(),
                    "{field}: {identity}"
                );
                assert!(ensure_repository(directory.path()).is_err());
                assert_eq!(
                    fs::read(&manifest_path).expect("read preserved metadata"),
                    content
                );
            }
        }
    }

    #[test]
    fn repository_accepts_uppercase_identity() {
        let directory = tempfile::tempdir().expect("temporary directory");
        ensure_repository(directory.path()).expect("initialize repository");
        let manifest = br#"{"schema_version":1,"format_version":2,"repository_id":"550E8400-E29B-41D4-A716-446655440000","database_id":"550E8400-E29B-41D4-A716-446655440001"}"#;
        fs::write(directory.path().join("_meta/repository.json"), manifest)
            .expect("write metadata");
        read_repository(directory.path()).expect("valid uppercase identities");
    }

    #[test]
    fn immutable_publish_accepts_identical_retry_only() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let destination = directory.path().join("snapshot.ltx");
        let first = directory.path().join("first.tmp");
        fs::write(&first, b"first payload").expect("write first stage");
        publish(&first, &destination).expect("publish first object");
        assert!(!first.exists());

        let matching = directory.path().join("matching.tmp");
        fs::write(&matching, b"first payload").expect("write matching stage");
        publish(&matching, &destination).expect("publish identical retry");

        let different = directory.path().join("different.tmp");
        fs::write(&different, b"other payload").expect("write different stage");
        assert!(publish(&different, &destination).is_err());
        assert_eq!(
            fs::read(destination).expect("read published object"),
            b"first payload"
        );
    }
}
