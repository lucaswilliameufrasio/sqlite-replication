use std::collections::BTreeMap;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};

use lz4_flex::frame::{FrameDecoder, FrameEncoder};

const MAGIC: &[u8; 4] = b"LTX1";
const HEADER_SIZE: usize = 100;
const PAGE_HEADER_SIZE: usize = 4;
const TRAILER_SIZE: usize = 16;
const CHECKSUM_FLAG: u64 = 1 << 63;
const LOCK_BYTE: u64 = 0x4000_0000;
const CRC64_ISO_POLY_REFLECTED: u64 = 0xD800_0000_0000_0000;

const fn crc64_iso_table() -> [u64; 256] {
    let mut table = [0_u64; 256];
    let mut index = 0;
    while index < table.len() {
        let mut value = index as u64;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 1 == 1 {
                (value >> 1) ^ CRC64_ISO_POLY_REFLECTED
            } else {
                value >> 1
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}

const CRC64_ISO_TABLE: [u64; 256] = crc64_iso_table();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    pub flags: u32,
    pub page_size: u32,
    pub commit: u32,
    pub min_txid: u64,
    pub max_txid: u64,
    pub timestamp_millis: i64,
    pub pre_apply_checksum: u64,
    pub wal_offset: i64,
    pub wal_size: i64,
    pub wal_salt_1: u32,
    pub wal_salt_2: u32,
    pub node_id: u64,
}

/// One changed SQLite page in a non-snapshot LTX transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Page {
    pub page_no: u32,
    pub data: Vec<u8>,
}

/// Database checksum state for validating and applying an LTX chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseState {
    pub page_size: u32,
    pub commit: u32,
    pub txid: u64,
    pub checksum: u64,
    page_checksums: BTreeMap<u32, u64>,
}

impl DatabaseState {
    /// Loads page checksums from a consistent SQLite database image.
    pub fn load<R: Read + Seek>(source: &mut R, txid: u64) -> io::Result<Self> {
        let file_size = source.seek(SeekFrom::End(0))?;
        if file_size < 100 || txid == 0 {
            return Err(invalid_data("invalid database image size or TXID"));
        }
        source.seek(SeekFrom::Start(0))?;
        let mut database_header = [0_u8; 100];
        source.read_exact(&mut database_header)?;
        if &database_header[..16] != b"SQLite format 3\0" {
            return Err(invalid_data("invalid SQLite database header"));
        }
        let mut page_size = u16::from_be_bytes([database_header[16], database_header[17]]) as u32;
        if page_size == 1 {
            page_size = 65_536;
        }
        if !valid_page_size(page_size) || file_size % u64::from(page_size) != 0 {
            return Err(invalid_data("invalid SQLite page size or image length"));
        }
        let page_count = file_size / u64::from(page_size);
        if page_count == 0 || page_count > u64::from(u32::MAX) {
            return Err(invalid_data("SQLite page count out of range"));
        }
        let header_page_count =
            u32::from_be_bytes(database_header[28..32].try_into().expect("fixed slice"));
        if header_page_count != 0 && u64::from(header_page_count) != page_count {
            return Err(invalid_data("SQLite header and image page counts differ"));
        }

        let lock_page = lock_page_no(page_size);
        let mut page_checksums = BTreeMap::new();
        let mut rolling_checksum = 0_u64;
        let mut page = vec![0_u8; page_size as usize];
        for page_index in 0..page_count {
            let page_no = (page_index + 1) as u32;
            if page_no == lock_page {
                continue;
            }
            source.seek(SeekFrom::Start(page_index * u64::from(page_size)))?;
            source.read_exact(&mut page)?;
            let checksum = page_checksum(page_no, &page);
            page_checksums.insert(page_no, checksum);
            rolling_checksum ^= checksum;
        }
        Ok(Self {
            page_size,
            commit: page_count as u32,
            txid,
            checksum: CHECKSUM_FLAG | (rolling_checksum & !CHECKSUM_FLAG),
            page_checksums,
        })
    }

    /// Returns the state after applying the listed pages without mutating self.
    pub fn apply_pages(&self, header: Header, pages: &[Page]) -> io::Result<Self> {
        header.validate()?;
        if self.txid == u64::MAX
            || header.min_txid == 1
            || header.min_txid != self.txid + 1
            || header.pre_apply_checksum != self.checksum
        {
            return Err(invalid_data(
                "LTX delta does not continue the current database position",
            ));
        }
        if header.page_size != self.page_size {
            return Err(invalid_data(
                "LTX delta page size differs from database state",
            ));
        }
        validate_delta_pages(header, pages)?;

        let mut page_checksums = self.page_checksums.clone();
        let mut rolling_checksum = self.checksum & !CHECKSUM_FLAG;
        page_checksums.retain(|page_no, checksum| {
            if *page_no > header.commit {
                rolling_checksum ^= *checksum;
                false
            } else {
                true
            }
        });
        let lock_page = lock_page_no(header.page_size);
        for page in pages {
            let previous_checksum = page_checksums.get(&page.page_no).copied();
            if previous_checksum.is_none()
                && page.page_no <= self.commit
                && page.page_no != lock_page
            {
                return Err(invalid_data("database state is missing an existing page"));
            }
            if let Some(checksum) = previous_checksum {
                rolling_checksum ^= checksum;
            }
            let checksum = page_checksum(page.page_no, &page.data);
            rolling_checksum ^= checksum;
            page_checksums.insert(page.page_no, checksum);
        }

        let zero_page = vec![0_u8; header.page_size as usize];
        for page_index in u64::from(self.commit) + 1..=u64::from(header.commit) {
            let page_no = page_index as u32;
            if page_no == lock_page || page_checksums.contains_key(&page_no) {
                continue;
            }
            let checksum = page_checksum(page_no, &zero_page);
            rolling_checksum ^= checksum;
            page_checksums.insert(page_no, checksum);
        }

        Ok(Self {
            page_size: self.page_size,
            commit: header.commit,
            txid: header.max_txid,
            checksum: CHECKSUM_FLAG | (rolling_checksum & !CHECKSUM_FLAG),
            page_checksums,
        })
    }

    /// Validates the pre/post checksums and returns the resulting state.
    pub fn apply_delta(
        &self,
        header: Header,
        pages: &[Page],
        post_apply_checksum: u64,
    ) -> io::Result<Self> {
        let updated = self.apply_pages(header, pages)?;
        if updated.checksum != post_apply_checksum {
            return Err(invalid_data("LTX post-apply checksum mismatch"));
        }
        Ok(updated)
    }

    /// Builds one delta reconciling this chain with a consistent SQLite image.
    pub fn delta_to_database_image<R: Read + Seek>(
        &self,
        source: &mut R,
        timestamp_millis: i64,
    ) -> io::Result<Option<(Header, Vec<Page>, Self)>> {
        let next_txid = self
            .txid
            .checked_add(1)
            .ok_or_else(|| invalid_data("LTX transaction ID exhausted"))?;
        let target = Self::load(source, next_txid)?;
        if target.page_size != self.page_size {
            return Err(invalid_data(
                "SQLite image page size differs from staged database",
            ));
        }
        if target.commit == self.commit && target.checksum == self.checksum {
            return Ok(None);
        }

        let lock_page = lock_page_no(self.page_size);
        let mut pages = Vec::new();
        let mut page_data = vec![0_u8; self.page_size as usize];
        for page_index in 1..=u64::from(target.commit) {
            let page_no = page_index as u32;
            if page_no == lock_page {
                continue;
            }
            let target_checksum = target
                .page_checksums
                .get(&page_no)
                .ok_or_else(|| invalid_data("target image is missing a page checksum"))?;
            if self.page_checksums.get(&page_no) == Some(target_checksum) {
                continue;
            }
            source.seek(SeekFrom::Start(
                (page_index - 1) * u64::from(self.page_size),
            ))?;
            source.read_exact(&mut page_data)?;
            pages.push(Page {
                page_no,
                data: page_data.clone(),
            });
        }

        let header = Header {
            flags: 0,
            page_size: self.page_size,
            commit: target.commit,
            min_txid: next_txid,
            max_txid: next_txid,
            timestamp_millis,
            pre_apply_checksum: self.checksum,
            wal_offset: 0,
            wal_size: 0,
            wal_salt_1: 0,
            wal_salt_2: 0,
            node_id: 0,
        };
        let updated = self.apply_delta(header, &pages, target.checksum)?;
        Ok(Some((header, pages, updated)))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct IndexEntry {
    page_no: u32,
    offset: u64,
    size: u64,
}

struct CountingReader<R> {
    inner: R,
    bytes_read: u64,
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.bytes_read += count as u64;
        Ok(count)
    }
}

impl Header {
    fn validate(self) -> io::Result<()> {
        if self.flags != 0
            || !valid_page_size(self.page_size)
            || self.min_txid == 0
            || self.max_txid < self.min_txid
            || self.wal_offset < 0
            || self.wal_size < 0
            || (self.wal_offset == 0 && self.wal_size != 0)
            || ((self.wal_salt_1 != 0 || self.wal_salt_2 != 0) && self.wal_offset == 0)
        {
            return Err(invalid_data("invalid LTX v2 header"));
        }
        if self.min_txid == 1 {
            if self.commit == 0 || self.pre_apply_checksum != 0 {
                return Err(invalid_data("invalid LTX v2 snapshot header"));
            }
        } else if self.pre_apply_checksum == 0 || self.pre_apply_checksum & CHECKSUM_FLAG == 0 {
            return Err(invalid_data("invalid LTX v2 delta checksum header"));
        }
        Ok(())
    }

    fn encode(self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0_u8; HEADER_SIZE];
        bytes[..4].copy_from_slice(MAGIC);
        bytes[4..8].copy_from_slice(&self.flags.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.page_size.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.commit.to_be_bytes());
        bytes[16..24].copy_from_slice(&self.min_txid.to_be_bytes());
        bytes[24..32].copy_from_slice(&self.max_txid.to_be_bytes());
        bytes[32..40].copy_from_slice(&self.timestamp_millis.to_be_bytes());
        bytes[40..48].copy_from_slice(&self.pre_apply_checksum.to_be_bytes());
        bytes[48..56].copy_from_slice(&self.wal_offset.to_be_bytes());
        bytes[56..64].copy_from_slice(&self.wal_size.to_be_bytes());
        bytes[64..68].copy_from_slice(&self.wal_salt_1.to_be_bytes());
        bytes[68..72].copy_from_slice(&self.wal_salt_2.to_be_bytes());
        bytes[72..80].copy_from_slice(&self.node_id.to_be_bytes());
        bytes
    }

    fn decode(bytes: &[u8; HEADER_SIZE]) -> io::Result<Self> {
        if &bytes[..4] != MAGIC || bytes[80..].iter().any(|byte| *byte != 0) {
            return Err(invalid_data(
                "invalid LTX v2 magic or reserved header bytes",
            ));
        }
        let header = Self {
            flags: u32::from_be_bytes(bytes[4..8].try_into().expect("fixed slice")),
            page_size: u32::from_be_bytes(bytes[8..12].try_into().expect("fixed slice")),
            commit: u32::from_be_bytes(bytes[12..16].try_into().expect("fixed slice")),
            min_txid: u64::from_be_bytes(bytes[16..24].try_into().expect("fixed slice")),
            max_txid: u64::from_be_bytes(bytes[24..32].try_into().expect("fixed slice")),
            timestamp_millis: i64::from_be_bytes(bytes[32..40].try_into().expect("fixed slice")),
            pre_apply_checksum: u64::from_be_bytes(bytes[40..48].try_into().expect("fixed slice")),
            wal_offset: i64::from_be_bytes(bytes[48..56].try_into().expect("fixed slice")),
            wal_size: i64::from_be_bytes(bytes[56..64].try_into().expect("fixed slice")),
            wal_salt_1: u32::from_be_bytes(bytes[64..68].try_into().expect("fixed slice")),
            wal_salt_2: u32::from_be_bytes(bytes[68..72].try_into().expect("fixed slice")),
            node_id: u64::from_be_bytes(bytes[72..80].try_into().expect("fixed slice")),
        };
        header.validate()?;
        Ok(header)
    }
}

/// Encodes a seekable SQLite database image as an LTX v2 full snapshot.
pub fn encode_snapshot<R: Read + Seek, W: Write>(
    source: &mut R,
    destination: &mut W,
    timestamp_millis: i64,
) -> io::Result<Header> {
    encode_snapshot_at(source, destination, 1, timestamp_millis)
}

/// Encodes a full snapshot representing the database at the inclusive TXID.
/// Its LTX range begins at TXID 1 because the snapshot contains a complete image.
pub fn encode_snapshot_at<R: Read + Seek, W: Write>(
    source: &mut R,
    destination: &mut W,
    txid: u64,
    timestamp_millis: i64,
) -> io::Result<Header> {
    if txid == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "snapshot TXID must be positive",
        ));
    }
    let file_size = source.seek(SeekFrom::End(0))?;
    if file_size < 100 {
        return Err(invalid_data("SQLite image is too short"));
    }
    source.seek(SeekFrom::Start(0))?;
    let mut database_header = [0_u8; 100];
    source.read_exact(&mut database_header)?;
    if &database_header[..16] != b"SQLite format 3\0" {
        return Err(invalid_data("invalid SQLite database header"));
    }
    let mut page_size = u16::from_be_bytes([database_header[16], database_header[17]]) as u32;
    if page_size == 1 {
        page_size = 65_536;
    }
    if !valid_page_size(page_size) || file_size % u64::from(page_size) != 0 {
        return Err(invalid_data("invalid SQLite page size or image length"));
    }
    let page_count = file_size / u64::from(page_size);
    if page_count == 0 || page_count > u64::from(u32::MAX) {
        return Err(invalid_data("SQLite page count out of range"));
    }
    let header_page_count =
        u32::from_be_bytes(database_header[28..32].try_into().expect("fixed slice"));
    if header_page_count != 0 && u64::from(header_page_count) != page_count {
        return Err(invalid_data("SQLite header and image page counts differ"));
    }

    let header = Header {
        flags: 0,
        page_size,
        commit: page_count as u32,
        min_txid: 1,
        max_txid: txid,
        timestamp_millis,
        pre_apply_checksum: 0,
        wal_offset: 0,
        wal_size: 0,
        wal_salt_1: 0,
        wal_salt_2: 0,
        node_id: 0,
    };
    header.validate()?;

    let mut file_checksum = 0;
    write_hashed(destination, &mut file_checksum, &header.encode())?;
    source.seek(SeekFrom::Start(0))?;
    let mut page = vec![0_u8; page_size as usize];
    let mut index = Vec::new();
    let mut rolling_checksum = 0_u64;
    let mut offset = HEADER_SIZE as u64;
    let lock_pgno = lock_page_no(page_size);
    for page_no in 1..=header.commit {
        source.seek(SeekFrom::Start(
            u64::from(page_no - 1) * u64::from(page_size),
        ))?;
        source.read_exact(&mut page)?;
        if page_no == lock_pgno {
            continue;
        }
        let page_header = page_no.to_be_bytes();
        write_hashed(destination, &mut file_checksum, &page_header)?;
        let mut encoder = FrameEncoder::new(Vec::new());
        encoder.write_all(&page)?;
        let compressed = encoder.finish()?;
        destination.write_all(&compressed)?;
        file_checksum = crc64_iso_update(file_checksum, &page);
        index.push(IndexEntry {
            page_no,
            offset,
            size: PAGE_HEADER_SIZE as u64 + compressed.len() as u64,
        });
        offset += PAGE_HEADER_SIZE as u64 + compressed.len() as u64;
        rolling_checksum ^= page_checksum(page_no, &page);
    }

    write_hashed(destination, &mut file_checksum, &[0_u8; PAGE_HEADER_SIZE])?;
    let mut index_bytes = Vec::new();
    for entry in &index {
        append_uvarint(&mut index_bytes, u64::from(entry.page_no));
        append_uvarint(&mut index_bytes, entry.offset);
        append_uvarint(&mut index_bytes, entry.size);
    }
    append_uvarint(&mut index_bytes, 0);
    let index_size = index_bytes.len() as u64;
    index_bytes.extend_from_slice(&index_size.to_be_bytes());
    write_hashed(destination, &mut file_checksum, &index_bytes)?;

    let post_apply_checksum = CHECKSUM_FLAG | (rolling_checksum & !CHECKSUM_FLAG);
    let mut trailer = [0_u8; TRAILER_SIZE];
    trailer[..8].copy_from_slice(&post_apply_checksum.to_be_bytes());
    file_checksum = crc64_iso_update(file_checksum, &trailer[..8]);
    trailer[8..].copy_from_slice(&(CHECKSUM_FLAG | file_checksum).to_be_bytes());
    destination.write_all(&trailer)?;
    Ok(header)
}

/// Encodes sorted changed pages as one non-snapshot LTX v2 file.
/// The caller supplies the database checksum after applying these pages.
pub fn encode_delta<W: Write>(
    header: Header,
    pages: &[Page],
    post_apply_checksum: u64,
    destination: &mut W,
) -> io::Result<()> {
    header.validate()?;
    if header.min_txid == 1 {
        return Err(invalid_data("delta LTX must not be a snapshot"));
    }
    if post_apply_checksum & CHECKSUM_FLAG == 0 {
        return Err(invalid_data(
            "post-apply checksum must include the checksum flag",
        ));
    }

    validate_delta_pages(header, pages)?;

    let mut file_checksum = 0_u64;
    write_hashed(destination, &mut file_checksum, &header.encode())?;
    let mut index = Vec::with_capacity(pages.len());
    let mut offset = HEADER_SIZE as u64;
    for page in pages {
        write_hashed(destination, &mut file_checksum, &page.page_no.to_be_bytes())?;
        let mut encoder = FrameEncoder::new(Vec::new());
        encoder.write_all(&page.data)?;
        let compressed = encoder.finish()?;
        destination.write_all(&compressed)?;
        file_checksum = crc64_iso_update(file_checksum, &page.data);
        index.push(IndexEntry {
            page_no: page.page_no,
            offset,
            size: PAGE_HEADER_SIZE as u64 + compressed.len() as u64,
        });
        offset += PAGE_HEADER_SIZE as u64 + compressed.len() as u64;
    }

    write_hashed(destination, &mut file_checksum, &[0_u8; PAGE_HEADER_SIZE])?;
    let mut index_bytes = Vec::new();
    for entry in &index {
        append_uvarint(&mut index_bytes, u64::from(entry.page_no));
        append_uvarint(&mut index_bytes, entry.offset);
        append_uvarint(&mut index_bytes, entry.size);
    }
    append_uvarint(&mut index_bytes, 0);
    let index_size = index_bytes.len() as u64;
    index_bytes.extend_from_slice(&index_size.to_be_bytes());
    write_hashed(destination, &mut file_checksum, &index_bytes)?;

    let mut trailer = [0_u8; TRAILER_SIZE];
    trailer[..8].copy_from_slice(&post_apply_checksum.to_be_bytes());
    file_checksum = crc64_iso_update(file_checksum, &trailer[..8]);
    trailer[8..].copy_from_slice(&(CHECKSUM_FLAG | file_checksum).to_be_bytes());
    destination.write_all(&trailer)?;
    Ok(())
}

fn validate_delta_pages(header: Header, pages: &[Page]) -> io::Result<()> {
    let lock_page = lock_page_no(header.page_size);
    let mut previous_page = 0_u32;
    for page in pages {
        if page.page_no == 0 || page.page_no > header.commit || page.page_no == lock_page {
            return Err(invalid_data("invalid delta page number"));
        }
        if page.page_no <= previous_page {
            return Err(invalid_data("delta pages are not strictly ordered"));
        }
        if page.data.len() != header.page_size as usize {
            return Err(invalid_data("invalid delta page size"));
        }
        previous_page = page.page_no;
    }
    Ok(())
}

/// Validates and decodes an LTX v2 full snapshot into a database image.
pub fn decode_snapshot<R: Read, W: Write>(source: R, destination: &mut W) -> io::Result<Header> {
    let counted = CountingReader {
        inner: source,
        bytes_read: 0,
    };
    let mut reader = BufReader::with_capacity(64 * 1024, counted);
    let mut file_checksum = 0_u64;
    let mut header_bytes = [0_u8; HEADER_SIZE];
    read_hashed(&mut reader, &mut file_checksum, &mut header_bytes)?;
    let header = Header::decode(&header_bytes)?;
    if header.min_txid != 1 {
        return Err(invalid_data(
            "cannot decode non-snapshot LTX as a database image",
        ));
    }
    let lock_pgno = lock_page_no(header.page_size);
    let mut expected_page = 1_u32;
    let mut page = vec![0_u8; header.page_size as usize];
    let mut actual_index = Vec::new();
    let mut rolling_checksum = 0_u64;

    loop {
        let frame_offset = logical_offset(&reader);
        let mut page_header = [0_u8; PAGE_HEADER_SIZE];
        read_hashed(&mut reader, &mut file_checksum, &mut page_header)?;
        let page_no = u32::from_be_bytes(page_header);
        if page_no == 0 {
            break;
        }
        if expected_page == lock_pgno && lock_pgno <= header.commit {
            destination.write_all(&vec![0_u8; header.page_size as usize])?;
            expected_page += 1;
        }
        if page_no > header.commit || page_no == lock_pgno || page_no != expected_page {
            return Err(invalid_data("snapshot page sequence is invalid"));
        }
        expected_page += 1;

        let mut decoder = FrameDecoder::new(&mut reader);
        decoder.read_exact(&mut page)?;
        let mut extra = [0_u8; 1];
        if decoder.read(&mut extra)? != 0 {
            return Err(invalid_data("LZ4 page frame expands past SQLite page size"));
        }
        drop(decoder);

        file_checksum = crc64_iso_update(file_checksum, &page);
        rolling_checksum ^= page_checksum(page_no, &page);
        destination.write_all(&page)?;
        let frame_end = logical_offset(&reader);
        actual_index.push(IndexEntry {
            page_no,
            offset: frame_offset,
            size: frame_end - frame_offset,
        });
    }

    if expected_page == lock_pgno && lock_pgno <= header.commit {
        destination.write_all(&vec![0_u8; header.page_size as usize])?;
        expected_page += 1;
    }
    if expected_page != header.commit + 1 {
        return Err(invalid_data(
            "snapshot does not cover its commit page count",
        ));
    }
    validate_index(&mut reader, &mut file_checksum, &actual_index)?;

    let mut trailer = [0_u8; TRAILER_SIZE];
    reader.read_exact(&mut trailer)?;
    let post_apply_checksum = u64::from_be_bytes(trailer[..8].try_into().expect("fixed slice"));
    let stored_file_checksum = u64::from_be_bytes(trailer[8..].try_into().expect("fixed slice"));
    let expected_post = CHECKSUM_FLAG | (rolling_checksum & !CHECKSUM_FLAG);
    if post_apply_checksum != expected_post || post_apply_checksum & CHECKSUM_FLAG == 0 {
        return Err(invalid_data("LTX post-apply checksum mismatch"));
    }
    file_checksum = crc64_iso_update(file_checksum, &trailer[..8]);
    if stored_file_checksum != CHECKSUM_FLAG | file_checksum
        || stored_file_checksum & CHECKSUM_FLAG == 0
    {
        return Err(invalid_data("LTX file checksum mismatch"));
    }
    let mut trailing = [0_u8; 1];
    if reader.read(&mut trailing)? != 0 {
        return Err(invalid_data("LTX file contains trailing bytes"));
    }
    Ok(header)
}

/// Validates and decodes one non-snapshot LTX v2 file.
pub fn decode_delta<R: Read>(source: R) -> io::Result<(Header, Vec<Page>, u64)> {
    let counted = CountingReader {
        inner: source,
        bytes_read: 0,
    };
    let mut reader = BufReader::with_capacity(64 * 1024, counted);
    let mut file_checksum = 0_u64;
    let mut header_bytes = [0_u8; HEADER_SIZE];
    read_hashed(&mut reader, &mut file_checksum, &mut header_bytes)?;
    let header = Header::decode(&header_bytes)?;
    if header.min_txid == 1 {
        return Err(invalid_data("LTX file is a snapshot, not a delta"));
    }

    let lock_page = lock_page_no(header.page_size);
    let mut previous_page = 0_u32;
    let mut page_data = vec![0_u8; header.page_size as usize];
    let mut pages = Vec::new();
    let mut actual_index = Vec::new();
    loop {
        let frame_offset = logical_offset(&reader);
        let mut page_header = [0_u8; PAGE_HEADER_SIZE];
        read_hashed(&mut reader, &mut file_checksum, &mut page_header)?;
        let page_no = u32::from_be_bytes(page_header);
        if page_no == 0 {
            break;
        }
        if page_no > header.commit || page_no == lock_page || page_no <= previous_page {
            return Err(invalid_data("invalid or out-of-order delta page"));
        }
        previous_page = page_no;

        let mut decoder = FrameDecoder::new(&mut reader);
        decoder.read_exact(&mut page_data)?;
        let mut extra = [0_u8; 1];
        if decoder.read(&mut extra)? != 0 {
            return Err(invalid_data("LTX delta page exceeds SQLite page size"));
        }
        drop(decoder);
        file_checksum = crc64_iso_update(file_checksum, &page_data);
        pages.push(Page {
            page_no,
            data: page_data.clone(),
        });
        let frame_end = logical_offset(&reader);
        actual_index.push(IndexEntry {
            page_no,
            offset: frame_offset,
            size: frame_end - frame_offset,
        });
    }

    validate_index(&mut reader, &mut file_checksum, &actual_index)?;
    let mut trailer = [0_u8; TRAILER_SIZE];
    reader.read_exact(&mut trailer)?;
    let post_apply_checksum = u64::from_be_bytes(trailer[..8].try_into().expect("fixed slice"));
    let stored_file_checksum = u64::from_be_bytes(trailer[8..].try_into().expect("fixed slice"));
    if post_apply_checksum & CHECKSUM_FLAG == 0 || stored_file_checksum & CHECKSUM_FLAG == 0 {
        return Err(invalid_data("invalid LTX delta checksum flags"));
    }
    file_checksum = crc64_iso_update(file_checksum, &trailer[..8]);
    if stored_file_checksum != CHECKSUM_FLAG | file_checksum {
        return Err(invalid_data("LTX delta file checksum mismatch"));
    }
    let mut trailing = [0_u8; 1];
    if reader.read(&mut trailing)? != 0 {
        return Err(invalid_data("LTX delta contains trailing bytes"));
    }
    Ok((header, pages, post_apply_checksum))
}

fn validate_index<R: Read>(
    reader: &mut BufReader<CountingReader<R>>,
    checksum: &mut u64,
    actual: &[IndexEntry],
) -> io::Result<()> {
    let mut encoded_size = 0_u64;
    let mut index_count = 0_usize;
    loop {
        let (page_no, length) = read_uvarint_hashed(reader, checksum)?;
        encoded_size += length as u64;
        if page_no == 0 {
            if index_count != actual.len() {
                return Err(invalid_data("LTX page index count mismatch"));
            }
            break;
        }
        let (offset, offset_size) = read_uvarint_hashed(reader, checksum)?;
        let (size, frame_size) = read_uvarint_hashed(reader, checksum)?;
        encoded_size += (offset_size + frame_size) as u64;
        let entry = actual
            .get(index_count)
            .ok_or_else(|| invalid_data("LTX page index has extra entries"))?;
        if u64::from(entry.page_no) != page_no || entry.offset != offset || entry.size != size {
            return Err(invalid_data("LTX page index entry mismatch"));
        }
        index_count += 1;
    }
    let mut size_bytes = [0_u8; 8];
    reader.read_exact(&mut size_bytes)?;
    *checksum = crc64_iso_update(*checksum, &size_bytes);
    if u64::from_be_bytes(size_bytes) != encoded_size {
        return Err(invalid_data("LTX page index length mismatch"));
    }
    Ok(())
}

fn read_uvarint_hashed<R: Read>(
    reader: &mut BufReader<CountingReader<R>>,
    checksum: &mut u64,
) -> io::Result<(u64, usize)> {
    let mut value = 0_u64;
    for index in 0..10 {
        let mut byte_buffer = [0_u8; 1];
        reader.read_exact(&mut byte_buffer)?;
        let byte = byte_buffer[0];
        *checksum = crc64_iso_update(*checksum, &[byte]);
        if index == 9 && byte > 1 {
            return Err(invalid_data("LTX unsigned varint overflow"));
        }
        value |= u64::from(byte & 0x7f) << (7 * index);
        if byte < 0x80 {
            return Ok((value, index + 1));
        }
    }
    Err(invalid_data("LTX unsigned varint is too long"))
}

fn read_hashed<R: Read>(reader: &mut R, checksum: &mut u64, bytes: &mut [u8]) -> io::Result<()> {
    reader.read_exact(bytes)?;
    *checksum = crc64_iso_update(*checksum, bytes);
    Ok(())
}

fn write_hashed<W: Write>(writer: &mut W, checksum: &mut u64, bytes: &[u8]) -> io::Result<()> {
    writer.write_all(bytes)?;
    *checksum = crc64_iso_update(*checksum, bytes);
    Ok(())
}

fn append_uvarint(bytes: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        bytes.push((value as u8) | 0x80);
        value >>= 7;
    }
    bytes.push(value as u8);
}

fn valid_page_size(page_size: u32) -> bool {
    (512..=65_536).contains(&page_size) && page_size.is_power_of_two()
}

fn lock_page_no(page_size: u32) -> u32 {
    (LOCK_BYTE / u64::from(page_size) + 1) as u32
}

fn page_checksum(page_no: u32, page: &[u8]) -> u64 {
    let mut checksum = crc64_iso_update(0, &page_no.to_be_bytes());
    checksum = crc64_iso_update(checksum, page);
    checksum & !CHECKSUM_FLAG
}

fn crc64_iso_update(mut checksum: u64, bytes: &[u8]) -> u64 {
    checksum = !checksum;
    for byte in bytes {
        let index = ((checksum ^ u64::from(*byte)) & 0xff) as usize;
        checksum = (checksum >> 8) ^ CRC64_ISO_TABLE[index];
    }
    !checksum
}

fn logical_offset<R>(reader: &BufReader<CountingReader<R>>) -> u64 {
    reader.get_ref().bytes_read - reader.buffer().len() as u64
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        DatabaseState, Header, Page, decode_delta, decode_snapshot, encode_delta, encode_snapshot,
        encode_snapshot_at,
    };

    #[test]
    fn crc64_iso_matches_standard_check_value() {
        assert_eq!(
            super::crc64_iso_update(0, b"123456789"),
            0xB909_56C7_75A4_1001
        );
    }

    #[test]
    fn snapshot_round_trip() {
        let mut database = vec![0_u8; 512];
        database[..16].copy_from_slice(b"SQLite format 3\0");
        database[16..18].copy_from_slice(&512_u16.to_be_bytes());
        database[28..32].copy_from_slice(&1_u32.to_be_bytes());
        for (index, byte) in database[100..].iter_mut().enumerate() {
            *byte = (index * 13) as u8;
        }
        let mut encoded = Vec::new();
        encode_snapshot(&mut Cursor::new(database.clone()), &mut encoded, 1234)
            .expect("encode snapshot");
        let mut restored = Vec::new();
        decode_snapshot(Cursor::new(encoded), &mut restored).expect("decode snapshot");
        assert_eq!(restored, database);
    }

    #[test]
    fn compacted_snapshot_preserves_inclusive_txid() {
        let mut database = vec![0_u8; 512];
        database[..16].copy_from_slice(b"SQLite format 3\0");
        database[16..18].copy_from_slice(&512_u16.to_be_bytes());
        database[28..32].copy_from_slice(&1_u32.to_be_bytes());
        let mut encoded = Vec::new();
        let header = encode_snapshot_at(&mut Cursor::new(database.clone()), &mut encoded, 42, 1234)
            .expect("encode compacted snapshot");
        assert_eq!(header.min_txid, 1);
        assert_eq!(header.max_txid, 42);
        assert_eq!(header.timestamp_millis, 1234);
        let mut restored = Vec::new();
        let decoded = decode_snapshot(Cursor::new(encoded), &mut restored)
            .expect("decode compacted snapshot");
        assert_eq!(decoded, header);
        assert_eq!(restored, database);
        assert!(encode_snapshot_at(&mut Cursor::new(database), &mut Vec::new(), 0, 1234).is_err());
    }

    #[test]
    fn delta_round_trip() {
        let header = Header {
            flags: 0,
            page_size: 512,
            commit: 3,
            min_txid: 2,
            max_txid: 2,
            timestamp_millis: 1234,
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
                data: vec![0x11; 512],
            },
            Page {
                page_no: 3,
                data: vec![0x33; 512],
            },
        ];
        let post_apply_checksum = (1_u64 << 63) | 0x9876;
        let mut encoded = Vec::new();
        encode_delta(header, &pages, post_apply_checksum, &mut encoded).expect("encode delta");

        let (decoded_header, decoded_pages, decoded_checksum) =
            decode_delta(Cursor::new(encoded)).expect("decode delta");
        assert_eq!(decoded_header, header);
        assert_eq!(decoded_pages, pages);
        assert_eq!(decoded_checksum, post_apply_checksum);
    }

    #[test]
    fn database_state_validates_and_applies_delta_checksums() {
        let mut image = vec![0_u8; 3 * 512];
        image[..16].copy_from_slice(b"SQLite format 3\0");
        image[16..18].copy_from_slice(&512_u16.to_be_bytes());
        image[28..32].copy_from_slice(&3_u32.to_be_bytes());
        for (index, byte) in image[512..].iter_mut().enumerate() {
            *byte = (index * 13) as u8;
        }
        let mut image_reader = Cursor::new(image);
        let state = DatabaseState::load(&mut image_reader, 1).expect("load initial state");

        let first_header = Header {
            flags: 0,
            page_size: 512,
            commit: 3,
            min_txid: 2,
            max_txid: 2,
            timestamp_millis: 1,
            pre_apply_checksum: state.checksum,
            wal_offset: 32,
            wal_size: 32 + 24 + 512,
            wal_salt_1: 1,
            wal_salt_2: 2,
            node_id: 0,
        };
        let first_pages = vec![Page {
            page_no: 2,
            data: vec![0x72; 512],
        }];
        let first_state = state
            .apply_pages(first_header, &first_pages)
            .expect("apply first delta pages");
        let mut encoded = Vec::new();
        encode_delta(
            first_header,
            &first_pages,
            first_state.checksum,
            &mut encoded,
        )
        .expect("encode first delta");
        let (decoded_header, decoded_pages, decoded_checksum) =
            decode_delta(Cursor::new(encoded)).expect("decode first delta");
        let verified_state = state
            .apply_delta(decoded_header, &decoded_pages, decoded_checksum)
            .expect("verify first delta state");
        assert_eq!(verified_state, first_state);
        assert_eq!(verified_state.txid, 2);

        let grow_header = Header {
            flags: 0,
            page_size: 512,
            commit: 5,
            min_txid: 3,
            max_txid: 3,
            timestamp_millis: 2,
            pre_apply_checksum: verified_state.checksum,
            wal_offset: 32 + 24 + 512,
            wal_size: 32 + 2 * (24 + 512),
            wal_salt_1: 1,
            wal_salt_2: 2,
            node_id: 0,
        };
        let grown_state = verified_state
            .apply_pages(
                grow_header,
                &[Page {
                    page_no: 4,
                    data: vec![0x44; 512],
                }],
            )
            .expect("apply growing delta");
        assert_eq!(grown_state.page_checksums.len(), 5);

        let shrink_header = Header {
            flags: 0,
            page_size: 512,
            commit: 2,
            min_txid: 4,
            max_txid: 4,
            timestamp_millis: 3,
            pre_apply_checksum: grown_state.checksum,
            wal_offset: 32 + 2 * (24 + 512),
            wal_size: 32 + 3 * (24 + 512),
            wal_salt_1: 1,
            wal_salt_2: 2,
            node_id: 0,
        };
        let shrunk_state = grown_state
            .apply_pages(shrink_header, &[])
            .expect("apply shrinking delta");
        assert_eq!(shrunk_state.commit, 2);
        assert_eq!(shrunk_state.page_checksums.len(), 2);
    }

    #[test]
    fn database_state_builds_reconciliation_deltas() {
        let mut base = vec![0_u8; 2 * 512];
        base[..16].copy_from_slice(b"SQLite format 3\0");
        base[16..18].copy_from_slice(&512_u16.to_be_bytes());
        base[28..32].copy_from_slice(&2_u32.to_be_bytes());
        for (index, byte) in base[512..].iter_mut().enumerate() {
            *byte = (index * 9) as u8;
        }
        let state = DatabaseState::load(&mut Cursor::new(&base), 1).expect("load base state");

        let mut target = base.clone();
        target.extend_from_slice(&vec![0x73; 512]);
        target[28..32].copy_from_slice(&3_u32.to_be_bytes());
        target[512..1024].fill(0x42);
        let (header, pages, updated) = state
            .delta_to_database_image(&mut Cursor::new(&target), 200)
            .expect("build reconciliation delta")
            .expect("target differs from base");
        assert_eq!(header.commit, 3);
        assert_eq!((header.min_txid, header.max_txid), (2, 2));
        assert_eq!(header.wal_offset, 0);
        assert_eq!(
            pages.iter().map(|page| page.page_no).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let expected =
            DatabaseState::load(&mut Cursor::new(&target), 2).expect("load target state");
        assert_eq!(updated.checksum, expected.checksum);
        assert!(
            updated
                .delta_to_database_image(&mut Cursor::new(&target), 201)
                .expect("check identical target")
                .is_none()
        );

        let mut shrunk = target[..512].to_vec();
        shrunk[28..32].copy_from_slice(&1_u32.to_be_bytes());
        let (shrink_header, shrink_pages, shrunk_state) = updated
            .delta_to_database_image(&mut Cursor::new(&shrunk), 202)
            .expect("build shrinking delta")
            .expect("database shrunk");
        assert_eq!(shrink_header.commit, 1);
        assert_eq!(shrink_pages.len(), 1);
        assert_eq!(shrink_pages[0].page_no, 1);
        let expected_shrunk =
            DatabaseState::load(&mut Cursor::new(&shrunk), 3).expect("load shrunk state");
        assert_eq!(shrunk_state.checksum, expected_shrunk.checksum);
    }

    #[test]
    fn delta_encoder_rejects_invalid_pages() {
        let header = Header {
            flags: 0,
            page_size: 512,
            commit: 3,
            min_txid: 2,
            max_txid: 2,
            timestamp_millis: 1,
            pre_apply_checksum: (1_u64 << 63) | 1,
            wal_offset: 32,
            wal_size: 1024,
            wal_salt_1: 1,
            wal_salt_2: 2,
            node_id: 0,
        };
        let valid_page = vec![0x22; 512];
        let cases = [
            vec![Page {
                page_no: 0,
                data: valid_page.clone(),
            }],
            vec![Page {
                page_no: 4,
                data: valid_page.clone(),
            }],
            vec![
                Page {
                    page_no: 2,
                    data: valid_page.clone(),
                },
                Page {
                    page_no: 1,
                    data: valid_page.clone(),
                },
            ],
            vec![
                Page {
                    page_no: 1,
                    data: valid_page.clone(),
                },
                Page {
                    page_no: 1,
                    data: valid_page.clone(),
                },
            ],
            vec![Page {
                page_no: 1,
                data: valid_page[..256].to_vec(),
            }],
        ];
        for pages in cases {
            let mut encoded = Vec::new();
            assert!(encode_delta(header, &pages, (1_u64 << 63) | 3, &mut encoded).is_err());
        }
    }

    #[test]
    fn checksum_detects_corruption() {
        let mut database = vec![0_u8; 512];
        database[..16].copy_from_slice(b"SQLite format 3\0");
        database[16..18].copy_from_slice(&512_u16.to_be_bytes());
        database[28..32].copy_from_slice(&1_u32.to_be_bytes());
        let mut encoded = Vec::new();
        encode_snapshot(&mut Cursor::new(database), &mut encoded, 1).expect("encode snapshot");
        *encoded.last_mut().expect("trailer exists") ^= 1;
        assert!(decode_snapshot(Cursor::new(encoded), &mut Vec::new()).is_err());
    }

    #[test]
    fn reference_snapshot_fixture_decodes() {
        let fixture = include_bytes!("../../../../conformance/fixtures/ltx-v2/snapshot-4096.ltx");
        let mut restored = Vec::new();
        let header = decode_snapshot(Cursor::new(fixture), &mut restored)
            .expect("decode independent reference fixture");
        assert_eq!(header.page_size, 4096);
        assert!(header.commit > 1);
        assert!(restored.starts_with(b"SQLite format 3\0"));
        assert_eq!(
            restored.len(),
            header.page_size as usize * header.commit as usize
        );
    }
}
