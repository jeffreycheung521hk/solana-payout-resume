//! Payout records on disk: one JSON file per payout, replaced atomically.

use crate::record::{Attempt, PayoutRecord, Signature};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Longest payout id the store accepts, in bytes.
pub const MAX_PAYOUT_ID_LEN: usize = 128;

/// Why the store could not read or write a record.
#[derive(Debug)]
pub enum StoreError {
    /// The payout id cannot be used as a file name. It must be 1 to [`MAX_PAYOUT_ID_LEN`] bytes
    /// of ASCII letters, digits, `.`, `_` or `-`, and must not start with `.`.
    InvalidPayoutId(String),
    /// No record exists for this payout id.
    NotFound(String),
    /// The attempt's signature is already in the record.
    DuplicateSignature(Signature),
    /// Reading or writing a file failed.
    Io(io::Error),
    /// The file is not a valid payout record.
    Json(serde_json::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPayoutId(id) => {
                write!(f, "payout id {id:?} cannot be used as a file name")
            }
            Self::NotFound(id) => write!(f, "no record for payout {id:?}"),
            Self::DuplicateSignature(signature) => {
                write!(f, "signature {signature} is already in the record")
            }
            Self::Io(error) => write!(f, "record file I/O failed: {error}"),
            Self::Json(error) => write!(f, "record file is not a valid payout record: {error}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// Keeps payout records as `<payout_id>.json` files in one directory.
#[derive(Clone, Debug)]
pub struct PayoutRecordStore {
    dir: PathBuf,
}

impl PayoutRecordStore {
    /// Opens the directory, creating it if needed.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// The directory that holds the records.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Writes the record so that a reader sees either the previous version or this one, and so
    /// that this version survives a power loss once the call returns.
    ///
    /// Steps: write a temporary file and fsync it, rename it over the record, then fsync the
    /// directory so the rename itself is on disk. The directory fsync runs on Unix only.
    pub fn save(&self, record: &PayoutRecord) -> Result<(), StoreError> {
        validate_payout_id(&record.payout_id)?;
        let path = self.record_path(&record.payout_id);
        let temp = self.dir.join(format!(".{}.json.tmp", record.payout_id));
        let mut bytes = serde_json::to_vec_pretty(record)?;
        bytes.push(b'\n');
        let mut file = File::create(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, &path)?;
        sync_directory(&self.dir)?;
        Ok(())
    }

    /// Reads the record for this payout id.
    pub fn load(&self, payout_id: &str) -> Result<PayoutRecord, StoreError> {
        validate_payout_id(payout_id)?;
        let bytes = match fs::read(self.record_path(payout_id)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(StoreError::NotFound(payout_id.to_owned()));
            }
            Err(error) => return Err(error.into()),
        };
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Adds an attempt to the record and saves the record.
    ///
    /// Broadcast the attempt only after this returns `Ok`. On error, the record in memory is
    /// left as it was.
    pub fn append_attempt(
        &self,
        record: &mut PayoutRecord,
        attempt: Attempt,
    ) -> Result<(), StoreError> {
        if record.attempt(&attempt.signature).is_some() {
            return Err(StoreError::DuplicateSignature(attempt.signature));
        }
        record.attempts.push(attempt);
        if let Err(error) = self.save(record) {
            record.attempts.pop();
            return Err(error);
        }
        Ok(())
    }

    fn record_path(&self, payout_id: &str) -> PathBuf {
        self.dir.join(format!("{payout_id}.json"))
    }
}

fn validate_payout_id(payout_id: &str) -> Result<(), StoreError> {
    let valid = !payout_id.is_empty()
        && payout_id.len() <= MAX_PAYOUT_ID_LEN
        && !payout_id.starts_with('.')
        && payout_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidPayoutId(payout_id.to_owned()))
    }
}

#[cfg(unix)]
fn sync_directory(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_dir: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Address;

    fn temp_store(name: &str) -> PayoutRecordStore {
        let dir = std::env::temp_dir().join(format!(
            "solana-payout-resume-store-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        PayoutRecordStore::open(dir).unwrap()
    }

    fn record() -> PayoutRecord {
        PayoutRecord::new(
            "INV-001",
            Address([1; 32]),
            Address([2; 32]),
            Address([3; 32]),
            120_000_000,
        )
    }

    fn attempt(n: u8) -> Attempt {
        Attempt {
            signature: Signature([n; 64]),
            signed_tx: vec![n; 4],
            last_valid_block_height: 1_150,
            blockhash_fetch_slot: 1_000,
        }
    }

    #[test]
    fn saved_record_loads_back_and_leaves_no_temp_file() {
        let store = temp_store("round-trip");
        let mut record = record();
        store.append_attempt(&mut record, attempt(1)).unwrap();
        assert_eq!(store.load("INV-001").unwrap(), record);
        let names: Vec<_> = fs::read_dir(store.dir())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["INV-001.json"]);
        fs::remove_dir_all(store.dir()).unwrap();
    }

    #[test]
    fn a_temp_file_left_by_a_crash_does_not_replace_the_record() {
        let store = temp_store("crash");
        let saved = record();
        store.save(&saved).unwrap();
        fs::write(store.dir().join(".INV-001.json.tmp"), b"{\"half\": ").unwrap();
        assert_eq!(store.load("INV-001").unwrap(), saved);
        fs::remove_dir_all(store.dir()).unwrap();
    }

    #[test]
    fn duplicate_signature_is_rejected_and_memory_is_unchanged() {
        let store = temp_store("duplicate");
        let mut record = record();
        store.append_attempt(&mut record, attempt(1)).unwrap();
        let before = record.clone();
        assert!(matches!(
            store.append_attempt(&mut record, attempt(1)),
            Err(StoreError::DuplicateSignature(_))
        ));
        assert_eq!(record, before);
        fs::remove_dir_all(store.dir()).unwrap();
    }

    #[test]
    fn payout_ids_that_are_unsafe_file_names_are_rejected() {
        let store = temp_store("ids");
        let long = "a".repeat(MAX_PAYOUT_ID_LEN + 1);
        for id in ["", ".hidden", "a/b", "../x", "x|y", "空白", long.as_str()] {
            assert!(
                matches!(store.load(id), Err(StoreError::InvalidPayoutId(_))),
                "{id:?} should be rejected"
            );
        }
        assert!(matches!(
            store.load("INV-2026_11.a"),
            Err(StoreError::NotFound(_))
        ));
        fs::remove_dir_all(store.dir()).unwrap();
    }
}
