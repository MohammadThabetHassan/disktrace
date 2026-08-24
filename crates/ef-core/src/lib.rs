use blake3::Hasher as Blake3Hasher;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

const HASH_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unable to inspect image '{path}': {source}")]
    ImageInspection {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("source inspection was cancelled")]
    Cancelled,
    #[error("system clock is before the Unix epoch")]
    ClockBeforeEpoch,
    #[error("source image '{path}' no longer matches the expected identity")]
    SourceIdentityMismatch { path: PathBuf },
    #[error(
        "source image '{path}' changed length while reading: expected {expected_byte_length} bytes, observed {observed_byte_length} bytes"
    )]
    SourceLengthChanged {
        path: PathBuf,
        expected_byte_length: u64,
        observed_byte_length: u64,
    },
    #[error("requested source range offset {offset} plus length {length} overflowed")]
    RangeOverflow { offset: u64, length: u64 },
    #[error(
        "requested source range offset {offset} with length {length} exceeds source length {source_length}"
    )]
    RangeOutOfBounds {
        offset: u64,
        length: u64,
        source_length: u64,
    },
    #[error("unable to reserve memory for source range length {length}")]
    RangeAllocation { length: u64 },
    #[error(
        "source image '{path}' ended after {bytes_read} bytes while reading range offset {offset} with length {length}"
    )]
    RangeShortRead {
        path: PathBuf,
        offset: u64,
        length: u64,
        bytes_read: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub canonical_path: PathBuf,
    pub byte_length: u64,
    pub sha256: String,
    pub blake3: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSource {
    pub display_name: String,
    pub identity: SourceIdentity,
    pub read_only: bool,
}

impl ImageSource {
    pub fn inspect(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        Self::inspect_with_cancellation(path, &AtomicBool::new(false))
    }

    pub fn inspect_with_cancellation(
        path: impl AsRef<Path>,
        cancellation: &AtomicBool,
    ) -> Result<Self, CoreError> {
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled);
        }
        let path = path.as_ref();
        let canonical_path = path
            .canonicalize()
            .map_err(|source| CoreError::ImageInspection {
                path: path.to_path_buf(),
                source,
            })?;
        let metadata = canonical_path
            .metadata()
            .map_err(|source| CoreError::ImageInspection {
                path: canonical_path.clone(),
                source,
            })?;
        let (sha256, blake3) = hash_file_with_cancellation(&canonical_path, cancellation)?;
        let display_name = canonical_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("recovery-image")
            .to_owned();

        Ok(Self {
            display_name,
            identity: SourceIdentity {
                canonical_path,
                byte_length: metadata.len(),
                sha256,
                blake3,
            },
            read_only: true,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    ReadyForScan,
    ScanInProgress,
    ScanCompleted,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySession {
    pub id: Uuid,
    pub created_at_unix_ms: u128,
    pub policy_version: String,
    pub source: ImageSource,
    pub status: SessionStatus,
}

impl RecoverySession {
    pub fn create(source: ImageSource) -> Result<Self, CoreError> {
        let created_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CoreError::ClockBeforeEpoch)?
            .as_millis();

        Ok(Self {
            id: Uuid::new_v4(),
            created_at_unix_ms,
            policy_version: "1.0.0".to_owned(),
            source,
            status: SessionStatus::ReadyForScan,
        })
    }

    pub fn matches_source(&self, source: &ImageSource) -> bool {
        self.source.identity == source.identity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryMethod {
    Fat12DeletedRootMetadata,
    Fat16DeletedRootMetadata,
    ExfatDeletedContiguousRootMetadata,
    NtfsDeletedResidentRecord,
    NtfsDeletedContiguousNonresident,
    SignatureCarvingPng,
    SignatureCarvingJpeg,
    SignatureCarvingGif,
    SignatureCarvingAvi,
    SignatureCarvingMp4,
    SignatureCarvingPdf,
    SignatureCarvingZipOffice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateValidation {
    MetadataVerified,
    ContentValidated,
    RecoveredUnvalidated,
    PartialOrErrorAffected,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryCandidate {
    pub id: String,
    pub evidence_name: String,
    pub file_type: String,
    pub source_offset: u64,
    pub byte_length: u64,
    pub method: RecoveryMethod,
    pub validation: CandidateValidation,
    pub original_path: Option<String>,
}

pub fn hash_file(path: impl AsRef<Path>) -> Result<(String, String), CoreError> {
    hash_file_with_cancellation(path, &AtomicBool::new(false))
}

pub fn hash_file_with_cancellation(
    path: impl AsRef<Path>,
    cancellation: &AtomicBool,
) -> Result<(String, String), CoreError> {
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled);
    }
    let path = path.as_ref();
    let mut file = File::open(path).map_err(|source| CoreError::ImageInspection {
        path: path.to_path_buf(),
        source,
    })?;
    hash_open_file_with_cancellation(&mut file, path, cancellation)
}

pub fn read_verified_range_with_cancellation(
    expected_identity: &SourceIdentity,
    range: SourceRange,
    cancellation: &AtomicBool,
) -> Result<Vec<u8>, CoreError> {
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled);
    }

    let path = &expected_identity.canonical_path;
    let mut file = File::open(path).map_err(|source| CoreError::ImageInspection {
        path: path.clone(),
        source,
    })?;
    let observed_byte_length = file
        .metadata()
        .map_err(|source| CoreError::ImageInspection {
            path: path.clone(),
            source,
        })?
        .len();
    if observed_byte_length != expected_identity.byte_length {
        return Err(CoreError::SourceLengthChanged {
            path: path.clone(),
            expected_byte_length: expected_identity.byte_length,
            observed_byte_length,
        });
    }

    let (sha256, blake3) = hash_open_file_with_cancellation(&mut file, path, cancellation)?;
    if sha256 != expected_identity.sha256 || blake3 != expected_identity.blake3 {
        return Err(CoreError::SourceIdentityMismatch { path: path.clone() });
    }

    read_range_from_file_with_cancellation(
        &mut file,
        path,
        expected_identity.byte_length,
        range,
        cancellation,
    )
}

pub fn read_range_from_file_with_cancellation(
    file: &mut File,
    path: &Path,
    expected_byte_length: u64,
    range: SourceRange,
    cancellation: &AtomicBool,
) -> Result<Vec<u8>, CoreError> {
    read_range_from_file_with_cancellation_after_chunk(
        file,
        path,
        expected_byte_length,
        range,
        cancellation,
        |_| {},
    )
}

fn read_range_from_file_with_cancellation_after_chunk<F>(
    file: &mut File,
    path: &Path,
    expected_byte_length: u64,
    range: SourceRange,
    cancellation: &AtomicBool,
    mut after_chunk: F,
) -> Result<Vec<u8>, CoreError>
where
    F: FnMut(usize),
{
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled);
    }

    let range_end = range
        .offset
        .checked_add(range.length)
        .ok_or(CoreError::RangeOverflow {
            offset: range.offset,
            length: range.length,
        })?;
    if range_end > expected_byte_length {
        return Err(CoreError::RangeOutOfBounds {
            offset: range.offset,
            length: range.length,
            source_length: expected_byte_length,
        });
    }

    let observed_byte_length = file
        .metadata()
        .map_err(|source| CoreError::ImageInspection {
            path: path.to_path_buf(),
            source,
        })?
        .len();
    if observed_byte_length != expected_byte_length {
        return Err(CoreError::SourceLengthChanged {
            path: path.to_path_buf(),
            expected_byte_length,
            observed_byte_length,
        });
    }

    let allocation_length =
        usize::try_from(range.length).map_err(|_| CoreError::RangeAllocation {
            length: range.length,
        })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation_length)
        .map_err(|_| CoreError::RangeAllocation {
            length: range.length,
        })?;
    bytes.resize(allocation_length, 0);
    file.seek(SeekFrom::Start(range.offset))
        .map_err(|source| CoreError::ImageInspection {
            path: path.to_path_buf(),
            source,
        })?;

    let mut bytes_read_total = 0_usize;
    while bytes_read_total < bytes.len() {
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled);
        }
        let chunk_end = (bytes_read_total + HASH_BUFFER_SIZE).min(bytes.len());
        let bytes_read = file
            .read(&mut bytes[bytes_read_total..chunk_end])
            .map_err(|source| CoreError::ImageInspection {
                path: path.to_path_buf(),
                source,
            })?;
        if bytes_read == 0 {
            return Err(CoreError::RangeShortRead {
                path: path.to_path_buf(),
                offset: range.offset,
                length: range.length,
                bytes_read: bytes_read_total as u64,
            });
        }
        bytes_read_total += bytes_read;
        after_chunk(bytes_read_total);
    }

    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled);
    }
    Ok(bytes)
}

fn hash_open_file_with_cancellation(
    file: &mut File,
    path: &Path,
    cancellation: &AtomicBool,
) -> Result<(String, String), CoreError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|source| CoreError::ImageInspection {
            path: path.to_path_buf(),
            source,
        })?;
    let mut sha256 = Sha256::new();
    let mut blake3 = Blake3Hasher::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_SIZE];

    loop {
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled);
        }
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|source| CoreError::ImageInspection {
                path: path.to_path_buf(),
                source,
            })?;
        if bytes_read == 0 {
            break;
        }
        sha256.update(&buffer[..bytes_read]);
        blake3.update(&buffer[..bytes_read]);
    }

    Ok((
        hex::encode(sha256.finalize()),
        blake3.finalize().to_hex().to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        hash_file, hash_file_with_cancellation, read_range_from_file_with_cancellation,
        read_range_from_file_with_cancellation_after_chunk, read_verified_range_with_cancellation,
        CoreError, ImageSource, RecoverySession, SessionStatus, SourceRange, HASH_BUFFER_SIZE,
    };
    use std::fs::{self, File};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use uuid::Uuid;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("evidenceforge-{name}-{}", Uuid::new_v4()))
    }

    #[test]
    fn hashes_known_content_with_two_algorithms() {
        let path = test_path("known-content.img");
        fs::write(&path, b"evidenceforge").expect("write test image");

        let (sha256, blake3) = hash_file(&path).expect("hash test image");

        assert_eq!(
            sha256,
            "ed2994bfb5ab79d1d933413772a2bb8f4ac4e38d5288db507524cd83888dc8cf"
        );
        assert_eq!(
            blake3,
            "3a4b3957c6bb711543c0fb01f3ae5353c726ea614b81f02d7afe028c225a0f72"
        );
        fs::remove_file(path).expect("remove test image");
    }

    #[test]
    fn verified_range_reads_exact_bytes_and_allows_an_empty_range_at_source_end() {
        let path = test_path("verified-range.img");
        fs::write(&path, b"0123456789").expect("write test image");
        let source = ImageSource::inspect(&path).expect("inspect image");
        let cancellation = AtomicBool::new(false);

        let bytes = read_verified_range_with_cancellation(
            &source.identity,
            SourceRange {
                offset: 3,
                length: 4,
            },
            &cancellation,
        )
        .expect("read verified range");
        let empty = read_verified_range_with_cancellation(
            &source.identity,
            SourceRange {
                offset: source.identity.byte_length,
                length: 0,
            },
            &cancellation,
        )
        .expect("read empty range at source end");

        assert_eq!(bytes, b"3456");
        assert!(empty.is_empty());
        fs::remove_file(path).expect("remove test image");
    }

    #[test]
    fn verified_range_refuses_substituted_or_changed_sources() {
        let path = test_path("changed-range.img");
        fs::write(&path, b"original-evidence").expect("write original image");
        let source = ImageSource::inspect(&path).expect("inspect original image");
        let cancellation = AtomicBool::new(false);

        fs::write(&path, b"substituted-data!").expect("write same-length substitute");
        assert!(matches!(
            read_verified_range_with_cancellation(
                &source.identity,
                SourceRange {
                    offset: 0,
                    length: 4,
                },
                &cancellation,
            ),
            Err(CoreError::SourceIdentityMismatch { .. })
        ));

        fs::write(&path, b"short").expect("write changed-length source");
        assert!(matches!(
            read_verified_range_with_cancellation(
                &source.identity,
                SourceRange {
                    offset: 0,
                    length: 4,
                },
                &cancellation,
            ),
            Err(CoreError::SourceLengthChanged { .. })
        ));
        fs::remove_file(path).expect("remove test image");
    }

    #[test]
    fn range_reader_refuses_overflow_and_source_end_violations() {
        let path = test_path("range-bounds.img");
        fs::write(&path, b"0123456789").expect("write test image");
        let cancellation = AtomicBool::new(false);
        let mut file = File::open(&path).expect("open test image");

        assert!(matches!(
            read_range_from_file_with_cancellation(
                &mut file,
                &path,
                10,
                SourceRange {
                    offset: 9,
                    length: 2,
                },
                &cancellation,
            ),
            Err(CoreError::RangeOutOfBounds { .. })
        ));
        assert!(matches!(
            read_range_from_file_with_cancellation(
                &mut file,
                &path,
                10,
                SourceRange {
                    offset: u64::MAX,
                    length: 1,
                },
                &cancellation,
            ),
            Err(CoreError::RangeOverflow { .. })
        ));
        fs::remove_file(path).expect("remove test image");
    }

    #[test]
    fn cancelled_verified_range_refuses_source_access_and_discards_partial_bytes() {
        let cancellation = AtomicBool::new(true);
        let missing_path = test_path("missing-range.img");
        let expected_identity = super::SourceIdentity {
            canonical_path: missing_path,
            byte_length: 0,
            sha256: String::new(),
            blake3: String::new(),
        };
        assert!(matches!(
            read_verified_range_with_cancellation(
                &expected_identity,
                SourceRange {
                    offset: 0,
                    length: 0,
                },
                &cancellation,
            ),
            Err(CoreError::Cancelled)
        ));

        let path = test_path("mid-read-cancellation.img");
        fs::write(&path, vec![0xA5_u8; HASH_BUFFER_SIZE * 2]).expect("write test image");
        let mut file = File::open(&path).expect("open test image");
        let cancellation = AtomicBool::new(false);
        assert!(matches!(
            read_range_from_file_with_cancellation_after_chunk(
                &mut file,
                &path,
                (HASH_BUFFER_SIZE * 2) as u64,
                SourceRange {
                    offset: 0,
                    length: (HASH_BUFFER_SIZE * 2) as u64,
                },
                &cancellation,
                |_| cancellation.store(true, Ordering::Relaxed),
            ),
            Err(CoreError::Cancelled)
        ));
        assert!(cancellation.load(Ordering::Relaxed));
        fs::remove_file(path).expect("remove test image");
    }

    #[test]
    fn cancelled_hash_and_inspection_refuse_to_create_source_identity() {
        let path = test_path("cancelled-source.img");
        fs::write(&path, b"evidenceforge").expect("write test image");
        let cancellation = AtomicBool::new(true);

        assert!(matches!(
            hash_file_with_cancellation(&path, &cancellation),
            Err(CoreError::Cancelled)
        ));
        assert!(matches!(
            ImageSource::inspect_with_cancellation(&path, &cancellation),
            Err(CoreError::Cancelled)
        ));

        fs::remove_file(path).expect("remove test image");
    }

    #[test]
    fn inspected_images_are_marked_read_only_and_bound_to_session() {
        let path = test_path("session-source.img");
        fs::write(&path, b"recovery-session").expect("write test image");

        let source = ImageSource::inspect(&path).expect("inspect image");
        let session = RecoverySession::create(source.clone()).expect("create session");

        assert!(source.read_only);
        assert_eq!(session.status, SessionStatus::ReadyForScan);
        assert!(session.matches_source(&source));
        assert_eq!(session.source.identity.byte_length, 16);
        fs::remove_file(path).expect("remove test image");
    }

    #[test]
    fn rejects_a_substituted_source_image() {
        let original_path = test_path("original-source.img");
        let substituted_path = test_path("substituted-source.img");
        fs::write(&original_path, b"original evidence").expect("write original image");
        fs::write(&substituted_path, b"substituted image").expect("write substituted image");

        let original_source = ImageSource::inspect(&original_path).expect("inspect original image");
        let substituted_source =
            ImageSource::inspect(&substituted_path).expect("inspect substituted image");
        let session = RecoverySession::create(original_source).expect("create session");

        assert!(!session.matches_source(&substituted_source));
        fs::remove_file(original_path).expect("remove original image");
        fs::remove_file(substituted_path).expect("remove substituted image");
    }
}
