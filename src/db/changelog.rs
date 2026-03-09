use crate::types::id::Id;
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// A single change record in the append-only log file.
/// Fixed-width: 24 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub user: [u8; 12],
    pub table: u8,
    pub op: u8,
    pub columns: u16,
    pub created: i64,
}

const RECORD_SIZE: usize = 24;
const _: () = assert!(RECORD_SIZE == 24);

impl Record {
    pub fn new(user: Id, table: u8, op: u8, columns: u16) -> Self {
        Self {
            user: user.bytes(),
            table,
            op,
            columns,
            created: chrono::Utc::now().timestamp(),
        }
    }

    pub fn to_bytes(&self) -> [u8; RECORD_SIZE] {
        let mut buf = [0u8; RECORD_SIZE];
        buf[0..12].copy_from_slice(&self.user);
        buf[12] = self.table;
        buf[13] = self.op;
        buf[14..16].copy_from_slice(&self.columns.to_le_bytes());
        buf[16..24].copy_from_slice(&self.created.to_le_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; RECORD_SIZE]) -> Self {
        let mut user = [0u8; 12];
        user.copy_from_slice(&buf[0..12]);
        Self {
            user,
            table: buf[12],
            op: buf[13],
            columns: u16::from_le_bytes([buf[14], buf[15]]),
            created: i64::from_le_bytes([
                buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], buf[22], buf[23],
            ]),
        }
    }
}

/// A variable-width record stored in the deletes sidecar file.
///
/// Layout:
///   - `table`:   1 byte   — `LogTable` discriminant
///   - `key_len`: 1 byte   — length of the UTF-8 row_key (max 255)
///   - `key`:     key_len bytes — the row_key string
///   - `created`: 8 bytes  — LE i64 unix timestamp (seconds)
///
/// Minimum size: 10 bytes (empty key). Maximum: 265 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteRecord {
    pub table: u8,
    pub key: String,
    pub created: i64,
}

impl DeleteRecord {
    pub fn to_bytes(&self) -> Vec<u8> {
        let key_bytes = self.key.as_bytes();
        // Truncate to 255 if somehow longer (shouldn't happen in practice)
        let key_len = key_bytes.len().min(255) as u8;
        let mut buf = Vec::with_capacity(2 + key_len as usize + 8);
        buf.push(self.table);
        buf.push(key_len);
        buf.extend_from_slice(&key_bytes[..key_len as usize]);
        buf.extend_from_slice(&self.created.to_le_bytes());
        buf
    }

    /// Read a single `DeleteRecord` from the reader.
    /// Returns `Ok(None)` at EOF, `Ok(Some(record))` on success.
    pub fn from_reader(reader: &mut impl Read) -> std::io::Result<Option<Self>> {
        let mut header = [0u8; 2];
        match reader.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }

        let table = header[0];
        let key_len = header[1] as usize;

        let mut key_buf = vec![0u8; key_len];
        reader.read_exact(&mut key_buf)?;

        let mut ts_buf = [0u8; 8];
        reader.read_exact(&mut ts_buf)?;
        let created = i64::from_le_bytes(ts_buf);

        let key = String::from_utf8(key_buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(Some(Self {
            table,
            key,
            created,
        }))
    }
}

pub struct ChangeLog {
    file: File,
    path: PathBuf,
    delete_file: File,
    delete_path: PathBuf,
}

const PATH: &str = "changelog.bin";

impl ChangeLog {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;

        // Derive the deletes path by appending ".deletes" to the changelog
        // path.  E.g. "changelog.bin" → "changelog.bin.deletes".  This
        // keeps each ChangeLog instance's delete file unique when tests
        // open multiple instances with different paths.
        let mut delete_path = path.as_os_str().to_owned();
        delete_path.push(".deletes");
        let delete_path = PathBuf::from(delete_path);

        let delete_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(&delete_path)?;

        Ok(Self {
            file,
            path: path.to_path_buf(),
            delete_file,
            delete_path,
        })
    }

    /// Appends a record and returns the byte offset *after* the write
    /// (i.e. the new file length). This is the cursor value clients
    /// should store.
    pub fn append(&mut self, record: &Record) -> std::io::Result<u64> {
        self.file.write_all(&record.to_bytes())?;
        self.file.flush()?;
        self.len()
    }

    /// Reads all records starting at `offset`. Returns an error if
    /// `offset` is not aligned to the record size.
    pub fn read_from(&self, offset: u64) -> std::io::Result<Vec<Record>> {
        if offset % RECORD_SIZE as u64 != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "offset is not aligned to record size",
            ));
        }

        let len = self.len()?;
        if offset >= len {
            return Ok(Vec::new());
        }

        let remaining = (len - offset) as usize;
        let count = remaining / RECORD_SIZE;

        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;

        let mut records = Vec::with_capacity(count);
        let mut buf = [0u8; RECORD_SIZE];
        for _ in 0..count {
            file.read_exact(&mut buf)?;
            records.push(Record::from_bytes(&buf));
        }

        Ok(records)
    }

    /// Returns the current file size (the cursor for "everything is synced").
    pub fn len(&self) -> std::io::Result<u64> {
        self.file.metadata().map(|m| m.len())
    }

    /// Append a delete record to the deletes sidecar file.
    pub fn append_delete(&mut self, table: u8, row_key: &str) -> std::io::Result<()> {
        let record = DeleteRecord {
            table,
            key: row_key.to_owned(),
            created: chrono::Utc::now().timestamp(),
        };
        self.delete_file.write_all(&record.to_bytes())?;
        self.delete_file.flush()?;
        Ok(())
    }

    /// Read all delete records starting at `offset`.
    /// Returns `(records, new_offset)` where `new_offset` is the byte
    /// position after the last record read — callers should store this
    /// for the next call.
    pub fn read_deletes_from(&self, offset: u64) -> std::io::Result<(Vec<DeleteRecord>, u64)> {
        let file_len = self.delete_file.metadata()?.len();
        if offset >= file_len {
            return Ok((Vec::new(), offset));
        }

        let mut file = File::open(&self.delete_path)?;
        file.seek(SeekFrom::Start(offset))?;

        let mut records = Vec::new();
        loop {
            match DeleteRecord::from_reader(&mut file)? {
                Some(r) => records.push(r),
                None => break,
            }
        }

        let new_offset = file.stream_position()?;
        Ok((records, new_offset))
    }

    /// Returns the current size of the deletes sidecar file.
    pub fn delete_cursor(&self) -> std::io::Result<u64> {
        self.delete_file.metadata().map(|m| m.len())
    }
}

thread_local! {
    pub static LOG: RefCell<ChangeLog> = RefCell::new(
        ChangeLog::open(Path::new(PATH)).expect("failed to open changelog")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_roundtrip() {
        let id: Id = "683d5a1b4f2e7c0019abcdef".parse().unwrap();
        let record = Record::new(id, 1, 0, 0);
        let bytes = record.to_bytes();
        let decoded = Record::from_bytes(&bytes);
        assert_eq!(record, decoded);
    }

    #[test]
    fn record_size_is_24() {
        assert_eq!(std::mem::size_of::<[u8; RECORD_SIZE]>(), 24);
    }

    #[test]
    fn record_columns_endianness() {
        let id: Id = "683d5a1b4f2e7c0019abcdef".parse().unwrap();
        let record = Record::new(id, 2, 1, 0b1010_0000_0000_0101);
        let bytes = record.to_bytes();
        let decoded = Record::from_bytes(&bytes);
        assert_eq!(decoded.columns, 0b1010_0000_0000_0101);
    }

    #[test]
    fn changelog_append_and_read() {
        let dir = std::env::temp_dir().join("ledger_test_changelog");
        let delete_dir = PathBuf::from(format!("{}.deletes", dir.display()));
        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_file(&delete_dir);

        let mut log = ChangeLog::open(&dir).unwrap();
        let id: Id = "683d5a1b4f2e7c0019abcdef".parse().unwrap();

        let r1 = Record::new(id, 1, 0, 0);
        let r2 = Record::new(id, 2, 1, 7);

        let cursor1 = log.append(&r1).unwrap();
        assert_eq!(cursor1, 24);

        let cursor2 = log.append(&r2).unwrap();
        assert_eq!(cursor2, 48);

        let all = log.read_from(0).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].table, 1);
        assert_eq!(all[1].table, 2);

        let from_mid = log.read_from(24).unwrap();
        assert_eq!(from_mid.len(), 1);
        assert_eq!(from_mid[0].columns, 7);

        let from_end = log.read_from(48).unwrap();
        assert!(from_end.is_empty());

        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_file(&delete_dir);
    }

    #[test]
    fn changelog_unaligned_offset_errors() {
        let dir = std::env::temp_dir().join("ledger_test_changelog_unaligned");
        let delete_dir = PathBuf::from(format!("{}.deletes", dir.display()));
        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_file(&delete_dir);

        let log = ChangeLog::open(&dir).unwrap();
        let result = log.read_from(5);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_file(&delete_dir);
    }

    #[test]
    fn delete_record_roundtrip() {
        let record = DeleteRecord {
            table: 5,
            key: "school123|user456".to_owned(),
            created: 1700000000,
        };
        let bytes = record.to_bytes();
        let mut reader = std::io::Cursor::new(&bytes);
        let decoded = DeleteRecord::from_reader(&mut reader).unwrap().unwrap();
        assert_eq!(decoded, record);
    }

    #[test]
    fn delete_record_empty_key() {
        let record = DeleteRecord {
            table: 1,
            key: String::new(),
            created: 1700000000,
        };
        let bytes = record.to_bytes();
        assert_eq!(bytes.len(), 10); // 1 + 1 + 0 + 8
        let mut reader = std::io::Cursor::new(&bytes);
        let decoded = DeleteRecord::from_reader(&mut reader).unwrap().unwrap();
        assert_eq!(decoded, record);
    }

    #[test]
    fn delete_record_eof_returns_none() {
        let buf: &[u8] = &[];
        let mut reader = std::io::Cursor::new(buf);
        let result = DeleteRecord::from_reader(&mut reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_record_multiple_sequential() {
        let r1 = DeleteRecord {
            table: 1,
            key: "abc".to_owned(),
            created: 100,
        };
        let r2 = DeleteRecord {
            table: 2,
            key: "defgh".to_owned(),
            created: 200,
        };
        let r3 = DeleteRecord {
            table: 3,
            key: "x".to_owned(),
            created: 300,
        };

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&r1.to_bytes());
        bytes.extend_from_slice(&r2.to_bytes());
        bytes.extend_from_slice(&r3.to_bytes());

        let mut reader = std::io::Cursor::new(&bytes);
        let d1 = DeleteRecord::from_reader(&mut reader).unwrap().unwrap();
        let d2 = DeleteRecord::from_reader(&mut reader).unwrap().unwrap();
        let d3 = DeleteRecord::from_reader(&mut reader).unwrap().unwrap();
        let d4 = DeleteRecord::from_reader(&mut reader).unwrap();

        assert_eq!(d1, r1);
        assert_eq!(d2, r2);
        assert_eq!(d3, r3);
        assert!(d4.is_none());
    }

    #[test]
    fn changelog_append_delete_and_read() {
        let dir = std::env::temp_dir().join("ledger_test_changelog_deletes");
        let delete_dir = PathBuf::from(format!("{}.deletes", dir.display()));
        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_file(&delete_dir);

        let mut log = ChangeLog::open(&dir).unwrap();

        // Initially empty
        let (records, cursor) = log.read_deletes_from(0).unwrap();
        assert!(records.is_empty());
        assert_eq!(cursor, 0);

        // Append two deletes
        log.append_delete(1, "user123").unwrap();
        log.append_delete(3, "school|owner").unwrap();

        // Read all from 0
        let (records, cursor) = log.read_deletes_from(0).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].table, 1);
        assert_eq!(records[0].key, "user123");
        assert_eq!(records[1].table, 3);
        assert_eq!(records[1].key, "school|owner");

        // Read from the cursor (should be empty)
        let (records2, cursor2) = log.read_deletes_from(cursor).unwrap();
        assert!(records2.is_empty());
        assert_eq!(cursor2, cursor);

        // Append one more
        log.append_delete(2, "new_school").unwrap();

        // Read from the previous cursor
        let (records3, _cursor3) = log.read_deletes_from(cursor).unwrap();
        assert_eq!(records3.len(), 1);
        assert_eq!(records3[0].table, 2);
        assert_eq!(records3[0].key, "new_school");

        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_file(&delete_dir);
    }

    #[test]
    fn delete_cursor_reflects_file_size() {
        let dir = std::env::temp_dir().join("ledger_test_delete_cursor");
        let delete_dir = PathBuf::from(format!("{}.deletes", dir.display()));
        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_file(&delete_dir);

        let mut log = ChangeLog::open(&dir).unwrap();
        assert_eq!(log.delete_cursor().unwrap(), 0);

        // "ab" = 2 bytes key => 1 + 1 + 2 + 8 = 12 bytes
        log.append_delete(1, "ab").unwrap();
        assert_eq!(log.delete_cursor().unwrap(), 12);

        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_file(&delete_dir);
    }
}
