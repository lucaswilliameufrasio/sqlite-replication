//! Conversion of complete commits from a stable SQLite WAL read into LTX v2 deltas.
//!
//! This module consumes immutable bytes only. It does not acquire SQLite locks,
//! pin a reader, publish files, or establish the CP-01 capture guarantee.

use std::collections::BTreeMap;
use std::io::{self, Cursor};

use sqlite_replication_ltx::{
    DatabaseState, Header as LtxHeader, Page, decode_delta, encode_delta,
};

use crate::{ScanResult, scan};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Position {
    pub initialized: bool,
    pub checkpoint_sequence: u32,
    pub salt_1: u32,
    pub salt_2: u32,
    pub frame_count: u32,
    pub checksum_1: u32,
    pub checksum_2: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub database: DatabaseState,
    pub wal: Position,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    pub start_frame: u32,
    pub end_frame: u32,
    pub header: LtxHeader,
    pub pages: Vec<Page>,
    pub post_checksum: u64,
    pub encoded: Vec<u8>,
}

/// Converts one stable, commit-bounded WAL prefix into verified LTX deltas.
/// The caller must prove the writer guard and previous reader pin separately.
pub fn translate_wal(
    data: &[u8],
    state: &State,
    timestamp_millis: i64,
) -> io::Result<(Vec<Transaction>, State)> {
    if data.is_empty() {
        if state.wal.initialized && state.wal.frame_count > 0 {
            return Err(invalid_data(
                "WAL disappeared while its captured position was non-zero",
            ));
        }
        return Ok((Vec::new(), state.clone()));
    }
    if data.len() < 32 {
        return Err(invalid_data("WAL prefix is shorter than its header"));
    }

    let scanned = scan(data)?;
    let frame_size = 24_usize
        .checked_add(scanned.header.page_size as usize)
        .ok_or_else(|| invalid_data("WAL frame size overflow"))?;
    if scanned.header.page_size != state.database.page_size {
        return Err(invalid_data(
            "WAL page size differs from the staged database",
        ));
    }
    let frame_count = u32::try_from(scanned.committed_frame_count)
        .map_err(|_| invalid_data("WAL frame count exceeds supported range"))?;
    let position = position_for_scan(&scanned, frame_count);

    let mut start_frame = 0_usize;
    if same_generation(state.wal, position) {
        if state.wal.frame_count > frame_count {
            return Err(invalid_data(
                "WAL frame count regressed without a generation change",
            ));
        }
        start_frame = state.wal.frame_count as usize;
        if start_frame > 0 && scanned.frames[start_frame - 1].database_size == 0 {
            return Err(invalid_data(
                "staged WAL position does not end at a commit frame",
            ));
        }
        let (checksum_1, checksum_2) = if start_frame == 0 {
            (scanned.header.checksum_1, scanned.header.checksum_2)
        } else {
            let frame = scanned.frames[start_frame - 1];
            (frame.checksum_1, frame.checksum_2)
        };
        if checksum_1 != state.wal.checksum_1 || checksum_2 != state.wal.checksum_2 {
            return Err(invalid_data(
                "previously staged WAL prefix changed without a generation change",
            ));
        }
    }

    let mut transactions = Vec::new();
    let mut current_state = state.clone();
    let mut changed_pages = BTreeMap::<u32, Vec<u8>>::new();
    let mut group_start = start_frame;
    for frame_index in start_frame..scanned.committed_frame_count {
        let frame = scanned.frames[frame_index];
        let page_start = usize::try_from(frame.page_offset)
            .map_err(|_| invalid_data("WAL page offset exceeds addressable memory"))?;
        let page_end = page_start
            .checked_add(scanned.header.page_size as usize)
            .ok_or_else(|| invalid_data("WAL page range overflow"))?;
        let page = data
            .get(page_start..page_end)
            .ok_or_else(|| invalid_data("WAL page falls outside validated prefix"))?;
        changed_pages.insert(frame.page_number, page.to_vec());
        if frame.database_size == 0 {
            continue;
        }

        let lock_page = 0x4000_0000 / scanned.header.page_size + 1;
        let pages: Vec<Page> = changed_pages
            .iter()
            .filter(|(page_no, _)| **page_no <= frame.database_size && **page_no != lock_page)
            .map(|(page_no, data)| Page {
                page_no: *page_no,
                data: data.clone(),
            })
            .collect();

        let txid = current_state
            .database
            .txid
            .checked_add(1)
            .ok_or_else(|| invalid_data("LTX transaction ID exhausted"))?;
        let wal_offset = 32_usize
            .checked_add(
                group_start
                    .checked_mul(frame_size)
                    .ok_or_else(|| invalid_data("LTX WAL offset overflow"))?,
            )
            .ok_or_else(|| invalid_data("LTX WAL offset overflow"))?;
        let wal_size = 32_usize
            .checked_add(
                (frame_index + 1)
                    .checked_mul(frame_size)
                    .ok_or_else(|| invalid_data("LTX WAL size overflow"))?,
            )
            .ok_or_else(|| invalid_data("LTX WAL size overflow"))?;
        let header = LtxHeader {
            flags: 0,
            page_size: scanned.header.page_size,
            commit: frame.database_size,
            min_txid: txid,
            max_txid: txid,
            timestamp_millis,
            pre_apply_checksum: current_state.database.checksum,
            wal_offset: i64::try_from(wal_offset)
                .map_err(|_| invalid_data("LTX WAL offset exceeds signed range"))?,
            wal_size: i64::try_from(wal_size)
                .map_err(|_| invalid_data("LTX WAL size exceeds signed range"))?,
            wal_salt_1: scanned.header.salt_1,
            wal_salt_2: scanned.header.salt_2,
            node_id: 0,
        };
        let next_database = current_state.database.apply_pages(header, &pages)?;
        let mut encoded = Vec::new();
        encode_delta(header, &pages, next_database.checksum, &mut encoded)?;
        let (decoded_header, decoded_pages, post_checksum) = decode_delta(Cursor::new(&encoded))?;
        let verified_database =
            current_state
                .database
                .apply_delta(decoded_header, &decoded_pages, post_checksum)?;
        if verified_database.checksum != next_database.checksum {
            return Err(invalid_data(
                "database checksum differs after LTX delta validation",
            ));
        }

        transactions.push(Transaction {
            start_frame: u32::try_from(group_start + 1)
                .map_err(|_| invalid_data("LTX start frame exceeds supported range"))?,
            end_frame: u32::try_from(frame_index + 1)
                .map_err(|_| invalid_data("LTX end frame exceeds supported range"))?,
            header,
            pages,
            post_checksum,
            encoded,
        });
        current_state.database = verified_database;
        group_start = frame_index + 1;
        changed_pages.clear();
    }
    if !changed_pages.is_empty() {
        return Err(invalid_data(
            "WAL prefix ends with an uncommitted transaction",
        ));
    }
    current_state.wal = position;
    Ok((transactions, current_state))
}

/// Records the last committed boundary in a stable WAL read.
pub fn position_from_wal(data: &[u8]) -> io::Result<Position> {
    if data.is_empty() {
        return Ok(Position::default());
    }
    let scanned = scan(data)?;
    let frame_count = u32::try_from(scanned.committed_frame_count)
        .map_err(|_| invalid_data("WAL frame count exceeds supported range"))?;
    Ok(position_for_scan(&scanned, frame_count))
}

/// Requires the WAL bytes up to the supplied size to end exactly at a commit frame.
pub fn position_from_wal_prefix(data: &[u8], prefix_size: usize) -> io::Result<Position> {
    if prefix_size > data.len() {
        return Err(invalid_data("WAL prefix length exceeds supplied bytes"));
    }
    if prefix_size == 0 {
        return Ok(Position::default());
    }
    let data = &data[..prefix_size];
    let scanned = scan(data)?;
    if scanned.committed_frame_count != scanned.frames.len() || scanned.incomplete_tail_bytes != 0 {
        return Err(invalid_data(
            "WAL prefix does not end at a complete commit boundary",
        ));
    }
    let frame_count = u32::try_from(scanned.committed_frame_count)
        .map_err(|_| invalid_data("WAL frame count exceeds supported range"))?;
    Ok(position_for_scan(&scanned, frame_count))
}

fn position_for_scan(scanned: &ScanResult, frame_count: u32) -> Position {
    let (checksum_1, checksum_2) = if scanned.committed_frame_count == 0 {
        (scanned.header.checksum_1, scanned.header.checksum_2)
    } else {
        let frame = scanned.frames[scanned.committed_frame_count - 1];
        (frame.checksum_1, frame.checksum_2)
    };
    Position {
        initialized: true,
        checkpoint_sequence: scanned.header.checkpoint_sequence,
        salt_1: scanned.header.salt_1,
        salt_2: scanned.header.salt_2,
        frame_count,
        checksum_1,
        checksum_2,
    }
}

fn same_generation(previous: Position, current: Position) -> bool {
    previous.initialized
        && current.initialized
        && previous.checkpoint_sequence == current.checkpoint_sequence
        && previous.salt_1 == current.salt_1
        && previous.salt_2 == current.salt_2
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use sqlite_replication_ltx::DatabaseState;

    use super::{Position, State, position_from_wal, translate_wal};
    use crate::scan;

    #[derive(Clone)]
    struct WalFrame {
        page_number: u32,
        database_size: u32,
        page: Vec<u8>,
    }

    #[test]
    fn translates_commit_boundaries_and_resumes_idempotently() {
        let base = sqlite_image(2);
        let base_state = DatabaseState::load(&mut Cursor::new(&base), 1).expect("load base state");

        let mut page_one_grown = base[..512].to_vec();
        page_one_grown[28..32].copy_from_slice(&3_u32.to_be_bytes());
        let mut page_one_shrunk = page_one_grown.clone();
        page_one_shrunk[28..32].copy_from_slice(&1_u32.to_be_bytes());
        let frames = vec![
            WalFrame {
                page_number: 2,
                database_size: 0,
                page: vec![0xa1; 512],
            },
            WalFrame {
                page_number: 2,
                database_size: 2,
                page: vec![0xb2; 512],
            },
            WalFrame {
                page_number: 1,
                database_size: 0,
                page: page_one_grown,
            },
            WalFrame {
                page_number: 3,
                database_size: 3,
                page: vec![0x3a; 512],
            },
            WalFrame {
                page_number: 2,
                database_size: 0,
                page: vec![0xc3; 512],
            },
            WalFrame {
                page_number: 3,
                database_size: 0,
                page: vec![0x3b; 512],
            },
            WalFrame {
                page_number: 1,
                database_size: 1,
                page: page_one_shrunk.clone(),
            },
        ];
        let wal_bytes = encode_wal(&frames, 7, 0x1122_3344, 0xaabb_ccdd);
        let initial_state = State {
            database: base_state.clone(),
            wal: Position::default(),
        };
        let (transactions, current) =
            translate_wal(&wal_bytes, &initial_state, 100).expect("translate WAL");
        assert_eq!(transactions.len(), 3);
        assert_eq!(
            (transactions[0].start_frame, transactions[0].end_frame),
            (1, 2)
        );
        assert_eq!(transactions[0].pages.len(), 1);
        assert_eq!(transactions[0].pages[0].page_no, 2);
        assert_eq!(transactions[0].pages[0].data, vec![0xb2; 512]);
        assert_eq!(transactions[0].header.wal_size, 32 + 2 * (24 + 512));
        assert_eq!(
            (transactions[1].start_frame, transactions[1].end_frame),
            (3, 4)
        );
        assert_eq!(
            transactions[1]
                .pages
                .iter()
                .map(|page| page.page_no)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(transactions[1].header.wal_size, 32 + 4 * (24 + 512));
        assert_eq!(
            (transactions[2].start_frame, transactions[2].end_frame),
            (5, 7)
        );
        assert_eq!(transactions[2].pages.len(), 1);
        assert_eq!(transactions[2].pages[0].page_no, 1);
        assert_eq!(current.database.txid, 4);
        assert_eq!(current.database.commit, 1);
        assert_eq!(current.wal.frame_count, 7);

        let expected_state =
            DatabaseState::load(&mut Cursor::new(page_one_shrunk.clone()), 4).expect("load final");
        assert_eq!(current.database.checksum, expected_state.checksum);
        let mut verified_state = base_state;
        for transaction in &transactions {
            let (header, pages, post_checksum) =
                sqlite_replication_ltx::decode_delta(Cursor::new(&transaction.encoded))
                    .expect("decode translated delta");
            verified_state = verified_state
                .apply_delta(header, &pages, post_checksum)
                .expect("verify translated delta chain");
        }
        assert_eq!(verified_state.checksum, current.database.checksum);

        let (repeated, repeated_state) =
            translate_wal(&wal_bytes, &current, 101).expect("resume unchanged WAL");
        assert!(repeated.is_empty());
        assert_eq!(repeated_state, current);

        let mut next_page = page_one_shrunk;
        next_page[100] ^= 0x7f;
        let mut appended_frames = frames.clone();
        appended_frames.push(WalFrame {
            page_number: 1,
            database_size: 1,
            page: next_page,
        });
        let appended_wal = encode_wal(&appended_frames, 7, 0x1122_3344, 0xaabb_ccdd);
        let (appended, appended_state) =
            translate_wal(&appended_wal, &current, 102).expect("translate appended commit");
        assert_eq!(appended.len(), 1);
        assert_eq!((appended[0].start_frame, appended[0].end_frame), (8, 8));
        assert_eq!(appended_state.database.txid, 5);

        let mut changed_prefix = appended_frames;
        changed_prefix[0].page.fill(0xee);
        let changed_wal = encode_wal(&changed_prefix, 7, 0x1122_3344, 0xaabb_ccdd);
        assert!(translate_wal(&changed_wal, &current, 103).is_err());
        let truncated_wal = &wal_bytes[..wal_bytes.len() - (24 + 512)];
        assert!(translate_wal(truncated_wal, &current, 103).is_err());
        let mut generation_page = transactions[2].pages[0].data.clone();
        generation_page[101] ^= 0x5c;
        let new_generation_wal = encode_wal(
            &[WalFrame {
                page_number: 1,
                database_size: 1,
                page: generation_page,
            }],
            8,
            0x5566_7788,
            0x99aa_bbcc,
        );
        let (new_generation_transactions, new_generation_state) =
            translate_wal(&new_generation_wal, &current, 103)
                .expect("translate changed WAL salt/checkpoint generation");
        assert_eq!(new_generation_transactions.len(), 1);
        assert_eq!(new_generation_state.wal.salt_1, 0x5566_7788);
        assert_eq!(new_generation_state.wal.checkpoint_sequence, 8);
        let mut partial_wal = appended_wal.clone();
        partial_wal.push(0);
        let (partial_transactions, partial_state) =
            translate_wal(&partial_wal, &current, 104).expect("ignore incomplete WAL tail");
        assert_eq!(partial_transactions.len(), 1);
        assert_eq!(partial_state.wal.frame_count, 8);
        assert_eq!(partial_state.database.txid, 5);
        assert!(position_from_wal(&appended_wal).is_ok());
        assert_eq!(position_from_wal(&partial_wal).unwrap().frame_count, 8);
        assert!(super::position_from_wal_prefix(&partial_wal, partial_wal.len()).is_err());
        assert!(scan(&wal_bytes).is_ok());
    }

    fn sqlite_image(page_count: u32) -> Vec<u8> {
        let mut image = vec![0; page_count as usize * 512];
        image[..16].copy_from_slice(b"SQLite format 3\0");
        image[16..18].copy_from_slice(&512_u16.to_be_bytes());
        image[28..32].copy_from_slice(&page_count.to_be_bytes());
        for (index, byte) in image[512..].iter_mut().enumerate() {
            *byte = (index * 17) as u8;
        }
        image
    }

    fn encode_wal(
        frames: &[WalFrame],
        checkpoint_sequence: u32,
        salt_1: u32,
        salt_2: u32,
    ) -> Vec<u8> {
        let page_size = frames[0].page.len() as u32;
        let mut result = vec![0_u8; 32];
        result[..4].copy_from_slice(&0x377f_0682_u32.to_be_bytes());
        result[4..8].copy_from_slice(&3_007_000_u32.to_be_bytes());
        result[8..12].copy_from_slice(&page_size.to_be_bytes());
        result[12..16].copy_from_slice(&checkpoint_sequence.to_be_bytes());
        result[16..20].copy_from_slice(&salt_1.to_be_bytes());
        result[20..24].copy_from_slice(&salt_2.to_be_bytes());
        let (mut checksum_1, mut checksum_2) = wal_checksum(&result[..24], 0, 0);
        result[24..28].copy_from_slice(&checksum_1.to_be_bytes());
        result[28..32].copy_from_slice(&checksum_2.to_be_bytes());

        for frame in frames {
            let mut frame_header = [0_u8; 24];
            frame_header[..4].copy_from_slice(&frame.page_number.to_be_bytes());
            frame_header[4..8].copy_from_slice(&frame.database_size.to_be_bytes());
            frame_header[8..12].copy_from_slice(&salt_1.to_be_bytes());
            frame_header[12..16].copy_from_slice(&salt_2.to_be_bytes());
            (checksum_1, checksum_2) = wal_checksum(&frame_header[..8], checksum_1, checksum_2);
            (checksum_1, checksum_2) = wal_checksum(&frame.page, checksum_1, checksum_2);
            frame_header[16..20].copy_from_slice(&checksum_1.to_be_bytes());
            frame_header[20..24].copy_from_slice(&checksum_2.to_be_bytes());
            result.extend_from_slice(&frame_header);
            result.extend_from_slice(&frame.page);
        }
        result
    }

    fn wal_checksum(data: &[u8], mut checksum_1: u32, mut checksum_2: u32) -> (u32, u32) {
        for words in data.chunks_exact(8) {
            let first = u32::from_le_bytes(words[..4].try_into().expect("checksum word"));
            let second = u32::from_le_bytes(words[4..].try_into().expect("checksum word"));
            checksum_1 = checksum_1.wrapping_add(first).wrapping_add(checksum_2);
            checksum_2 = checksum_2.wrapping_add(second).wrapping_add(checksum_1);
        }
        (checksum_1, checksum_2)
    }
}
