//! Parser for the SQLite WAL file format.
//!
//! Parsing bytes does not coordinate SQLite readers, writers, or checkpoints
//! and therefore does not establish CP-01.

use std::io;

pub mod translate;

const HEADER_SIZE: usize = 32;
const FRAME_HEADER_SIZE: usize = 24;
const WAL_FORMAT_VERSION: u32 = 3_007_000;
const MAGIC_CHECKSUM_LITTLE: u32 = 0x377f_0682;
const MAGIC_CHECKSUM_BIG: u32 = 0x377f_0683;
const MAX_PAGE_NUMBER: u32 = 0xffff_fffe;

/// Validated fields from a SQLite WAL header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    pub page_size: u32,
    pub checkpoint_sequence: u32,
    pub salt_1: u32,
    pub salt_2: u32,
    pub checksum_1: u32,
    pub checksum_2: u32,
}

/// Metadata for one validated WAL frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frame {
    pub page_number: u32,
    pub database_size: u32,
    /// Offset of the frame's page bytes in the input passed to [`scan`].
    /// This is not an independently durable WAL position.
    pub page_offset: u64,
    pub checksum_1: u32,
    pub checksum_2: u32,
}

/// Result for the longest valid frame prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanResult {
    pub header: Header,
    pub frames: Vec<Frame>,
    /// Number of prefix frames through the last commit frame, or zero if none.
    pub committed_frame_count: usize,
    /// Logical page count from the last commit frame, or zero if none.
    pub database_size: u32,
    /// Bytes after the last complete frame that do not form a full frame.
    pub incomplete_tail_bytes: usize,
}

/// Scans and validates one immutable byte slice containing a WAL header and
/// frames. A partial trailing frame is reported, not treated as committed.
/// A fully present invalid frame fails the entire scan closed.
pub fn scan(data: &[u8]) -> io::Result<ScanResult> {
    if data.len() < HEADER_SIZE {
        return Err(invalid_data("incomplete SQLite WAL header"));
    }

    let magic = read_u32_be(&data[..4]);
    let checksum_little_endian = match magic {
        MAGIC_CHECKSUM_LITTLE => true,
        MAGIC_CHECKSUM_BIG => false,
        _ => return Err(invalid_data("unsupported SQLite WAL magic")),
    };
    let version = read_u32_be(&data[4..8]);
    if version != WAL_FORMAT_VERSION {
        return Err(invalid_data("unsupported SQLite WAL format version"));
    }

    let page_size = read_u32_be(&data[8..12]);
    if !valid_page_size(page_size) {
        return Err(invalid_data("invalid SQLite WAL page size"));
    }

    let (checksum_1, checksum_2) = wal_checksum(&data[..24], 0, 0, checksum_little_endian);
    let header = Header {
        page_size,
        checkpoint_sequence: read_u32_be(&data[12..16]),
        salt_1: read_u32_be(&data[16..20]),
        salt_2: read_u32_be(&data[20..24]),
        checksum_1: read_u32_be(&data[24..28]),
        checksum_2: read_u32_be(&data[28..32]),
    };
    if checksum_1 != header.checksum_1 || checksum_2 != header.checksum_2 {
        return Err(invalid_data("SQLite WAL header checksum mismatch"));
    }

    let frame_size = FRAME_HEADER_SIZE + page_size as usize;
    let mut result = ScanResult {
        header,
        frames: Vec::new(),
        committed_frame_count: 0,
        database_size: 0,
        incomplete_tail_bytes: 0,
    };
    let mut checksum_state = (checksum_1, checksum_2);
    let mut offset = HEADER_SIZE;

    while offset < data.len() {
        let remaining = data.len() - offset;
        if remaining < frame_size {
            result.incomplete_tail_bytes = remaining;
            break;
        }

        let frame_header = &data[offset..offset + FRAME_HEADER_SIZE];
        let page = &data[offset + FRAME_HEADER_SIZE..offset + frame_size];
        let page_number = read_u32_be(&frame_header[..4]);
        let database_size = read_u32_be(&frame_header[4..8]);
        let salt_1 = read_u32_be(&frame_header[8..12]);
        let salt_2 = read_u32_be(&frame_header[12..16]);
        if page_number == 0
            || page_number > MAX_PAGE_NUMBER
            || (database_size != 0 && database_size > MAX_PAGE_NUMBER)
            || salt_1 != header.salt_1
            || salt_2 != header.salt_2
        {
            return Err(invalid_data("invalid SQLite WAL frame page number or salt"));
        }

        checksum_state = wal_checksum(
            &frame_header[..8],
            checksum_state.0,
            checksum_state.1,
            checksum_little_endian,
        );
        checksum_state = wal_checksum(
            page,
            checksum_state.0,
            checksum_state.1,
            checksum_little_endian,
        );
        if checksum_state.0 != read_u32_be(&frame_header[16..20])
            || checksum_state.1 != read_u32_be(&frame_header[20..24])
        {
            return Err(invalid_data("SQLite WAL frame checksum mismatch"));
        }

        result.frames.push(Frame {
            page_number,
            database_size,
            page_offset: (offset + FRAME_HEADER_SIZE) as u64,
            checksum_1: checksum_state.0,
            checksum_2: checksum_state.1,
        });
        if database_size != 0 {
            result.committed_frame_count = result.frames.len();
            result.database_size = database_size;
        }

        offset += frame_size;
    }

    Ok(result)
}

fn valid_page_size(page_size: u32) -> bool {
    (512..=65_536).contains(&page_size) && page_size.is_power_of_two()
}

fn read_u32_be(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes.try_into().expect("fixed-width WAL field"))
}

fn wal_checksum(
    data: &[u8],
    mut checksum_1: u32,
    mut checksum_2: u32,
    little_endian: bool,
) -> (u32, u32) {
    for words in data.chunks_exact(8) {
        let first = if little_endian {
            u32::from_le_bytes(words[..4].try_into().expect("fixed-width checksum word"))
        } else {
            u32::from_be_bytes(words[..4].try_into().expect("fixed-width checksum word"))
        };
        let second = if little_endian {
            u32::from_le_bytes(words[4..].try_into().expect("fixed-width checksum word"))
        } else {
            u32::from_be_bytes(words[4..].try_into().expect("fixed-width checksum word"))
        };
        checksum_1 = checksum_1.wrapping_add(first).wrapping_add(checksum_2);
        checksum_2 = checksum_2.wrapping_add(second).wrapping_add(checksum_1);
    }
    (checksum_1, checksum_2)
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::{FRAME_HEADER_SIZE, HEADER_SIZE, scan};
    use std::fs;
    use std::path::Path;

    fn fixture(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../conformance/fixtures/wal")
            .join(name);
        fs::read(path).expect("read shared WAL fixture")
    }

    #[test]
    fn scans_shared_little_and_big_endian_fixtures() {
        for (name, expected_page_size) in [
            ("sqlite-le.wal", 512),
            ("sqlite-be.wal", 512),
            ("sqlite-le-65536.wal", 65_536),
        ] {
            let content = fixture(name);
            let result = scan(&content).expect("scan shared WAL fixture");
            assert_eq!(result.header.page_size, expected_page_size);
            assert_eq!(result.header.checkpoint_sequence, 7);
            assert_eq!(result.header.salt_1, 0x1122_3344);
            assert_eq!(result.header.salt_2, 0xaabb_ccdd);
            assert_eq!(result.frames.len(), 3);
            assert_eq!(result.committed_frame_count, 2);
            assert_eq!(result.database_size, 2);
            assert_eq!(result.incomplete_tail_bytes, 0);
            assert_eq!(result.frames[0].page_number, 1);
            assert_eq!(result.frames[0].database_size, 0);
            assert_eq!(
                result.frames[0].page_offset,
                (HEADER_SIZE + FRAME_HEADER_SIZE) as u64
            );
            assert_ne!(result.frames[0].checksum_1, 0);
            assert_ne!(result.frames[0].checksum_2, 0);
        }
    }

    #[test]
    fn reports_partial_tail_without_advancing_commit() {
        let mut content = fixture("sqlite-le.wal");
        content.extend_from_slice(&[0_u8; 17]);
        let result = scan(&content).expect("scan WAL with incomplete tail");
        assert_eq!(result.frames.len(), 3);
        assert_eq!(result.committed_frame_count, 2);
        assert_eq!(result.incomplete_tail_bytes, 17);
    }

    #[test]
    fn rejects_corrupt_header_and_frames() {
        let original = fixture("sqlite-le.wal");
        let cases: [(&str, usize); 3] = [
            ("header checksum", 24),
            ("frame salt", HEADER_SIZE + 8),
            ("frame page checksum", HEADER_SIZE + FRAME_HEADER_SIZE + 20),
        ];
        for (name, offset) in cases {
            let mut corrupted = original.clone();
            corrupted[offset] ^= 1;
            assert!(scan(&corrupted).is_err(), "accepted corruption: {name}");
        }

        let mut reserved_page_number = original.clone();
        reserved_page_number[HEADER_SIZE..HEADER_SIZE + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(scan(&reserved_page_number).is_err());

        let mut reserved_database_size = original;
        reserved_database_size[HEADER_SIZE + 4..HEADER_SIZE + 8]
            .copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(scan(&reserved_database_size).is_err());
    }

    #[test]
    fn independent_wal_mutations_and_truncations_preserve_scan_invariants() {
        for name in ["sqlite-le.wal", "sqlite-be.wal"] {
            let original = fixture(name);
            let result = scan(&original).expect("scan independent WAL fixture");
            assert_scan_invariants(&original, &result);

            for end in 0..original.len() {
                if let Ok(result) = scan(&original[..end]) {
                    assert_scan_invariants(&original[..end], &result);
                }
            }
            for offset in 0..original.len() {
                for bit in 0..8 {
                    let mut mutated = original.clone();
                    mutated[offset] ^= 1 << bit;
                    if let Ok(result) = scan(&mutated) {
                        assert_scan_invariants(&mutated, &result);
                    }
                }
            }
        }

        let maximum_page_fixture = fixture("sqlite-le-65536.wal");
        let offsets = [
            0,
            7,
            8,
            16,
            24,
            31,
            32,
            55,
            56,
            maximum_page_fixture.len() / 2,
            maximum_page_fixture.len() - 9,
            maximum_page_fixture.len() - 1,
        ];
        for end in offsets {
            if let Ok(result) = scan(&maximum_page_fixture[..end]) {
                assert_scan_invariants(&maximum_page_fixture[..end], &result);
            }
            for bit in 0..8 {
                let mut mutated = maximum_page_fixture.clone();
                mutated[end] ^= 1 << bit;
                if let Ok(result) = scan(&mutated) {
                    assert_scan_invariants(&mutated, &result);
                }
            }
        }
    }

    fn assert_scan_invariants(data: &[u8], result: &super::ScanResult) {
        assert!(matches!(
            result.header.page_size,
            512 | 1024 | 2048 | 4096 | 8192 | 16384 | 32768 | 65536
        ));
        assert!(result.committed_frame_count <= result.frames.len());
        let frame_size = FRAME_HEADER_SIZE + result.header.page_size as usize;
        assert!(result.incomplete_tail_bytes < frame_size);
        let mut previous_page_offset = None;
        for frame in &result.frames {
            let page_offset = frame.page_offset as usize;
            assert!(page_offset >= HEADER_SIZE + FRAME_HEADER_SIZE);
            assert!(page_offset + result.header.page_size as usize <= data.len());
            if let Some(previous) = previous_page_offset {
                assert_eq!(page_offset - previous, frame_size);
            }
            previous_page_offset = Some(page_offset);
        }
        if result.committed_frame_count == 0 {
            assert_eq!(result.database_size, 0);
        } else {
            let last_committed = &result.frames[result.committed_frame_count - 1];
            assert_ne!(last_committed.database_size, 0);
            assert_eq!(result.database_size, last_committed.database_size);
        }
    }

    #[test]
    fn rejects_unsupported_format_version() {
        let mut content = fixture("sqlite-le.wal");
        content[4] ^= 1;
        assert!(scan(&content).is_err());
    }

    #[test]
    fn rejects_database_header_page_size_sentinel_in_wal_header() {
        let mut content = fixture("sqlite-le.wal");
        content[8..12].copy_from_slice(&1_u32.to_be_bytes());
        assert!(scan(&content).is_err());
    }
}
