use crate::{recover_candidate_from_image, RecoveryExport, WorkflowError};
use ef_core::{hash_file, ImageSource, RecoveryCandidate, RecoverySession, SessionStatus};
use ef_report::RecoveryReceipt;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

pub const SESSION_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SourceIntegrity {
    Verified,
    Changed { observed_source: ImageSource },
    Unavailable { detail: String },
}

impl SourceIntegrity {
    pub fn allows_recovery(&self) -> bool {
        matches!(self, Self::Verified)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedExport {
    pub candidate_id: String,
    pub exported_at_unix_ms: u128,
    pub output_path: PathBuf,
    pub receipt_path: PathBuf,
    pub receipt: RecoveryReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordedExportVerification {
    pub candidate_id: String,
    pub output_path: PathBuf,
    pub receipt_path: PathBuf,
    pub integrity: RecordedExportIntegrity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RecordedExportIntegrity {
    Verified,
    ReceiptUnavailable {
        detail: String,
    },
    ReceiptChanged,
    ReceiptInconsistent {
        detail: String,
    },
    ArtifactUnavailable {
        detail: String,
    },
    ArtifactChanged {
        expected_sha256: String,
        observed_sha256: String,
        expected_blake3: String,
        observed_blake3: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionManifest {
    pub schema_version: u32,
    pub session: RecoverySession,
    pub candidates: Vec<RecoveryCandidate>,
    pub exports: Vec<RecordedExport>,
}

#[derive(Debug, Error)]
pub enum SessionManifestError {
    #[error("recovery session {0} was not completed and cannot be saved")]
    SessionNotCompleted(Uuid),
    #[error("session manifest schema version {0} is not supported")]
    UnsupportedSchemaVersion(u32),
    #[error("session manifest contains duplicate candidate id '{0}'")]
    DuplicateCandidateId(String),
    #[error("candidate id '{0}' is not present in the saved session")]
    CandidateNotInManifest(String),
    #[error("candidate id '{0}' no longer matches the saved session record")]
    CandidateMismatch(String),
    #[error("the recorded source image changed after this session was saved")]
    SourceChanged,
    #[error("the recorded source image is unavailable: {0}")]
    SourceUnavailable(String),
    #[error("system clock is before the Unix epoch")]
    ClockBeforeEpoch,
    #[error("unable to read session manifest '{path}': {source}")]
    ReadManifest {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unable to parse session manifest '{path}': {source}")]
    ParseManifest {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unable to serialize session manifest: {0}")]
    SerializeManifest(#[from] serde_json::Error),
    #[error("unable to write session manifest '{path}': {source}")]
    WriteManifest {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unable to write local case brief '{path}': {source}")]
    WriteCaseBrief {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Workflow(#[from] WorkflowError),
}

impl SessionManifest {
    pub fn new(
        session: RecoverySession,
        candidates: Vec<RecoveryCandidate>,
    ) -> Result<Self, SessionManifestError> {
        if session.status != SessionStatus::ScanCompleted {
            return Err(SessionManifestError::SessionNotCompleted(session.id));
        }
        ensure_unique_candidate_ids(&candidates)?;

        Ok(Self {
            schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
            session,
            candidates,
            exports: Vec::new(),
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, SessionManifestError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| SessionManifestError::ReadManifest {
            path: path.to_path_buf(),
            source,
        })?;
        let manifest = serde_json::from_slice::<Self>(&bytes).map_err(|source| {
            SessionManifestError::ParseManifest {
                path: path.to_path_buf(),
                source,
            }
        })?;

        if manifest.schema_version != SESSION_MANIFEST_SCHEMA_VERSION {
            return Err(SessionManifestError::UnsupportedSchemaVersion(
                manifest.schema_version,
            ));
        }
        if manifest.session.status != SessionStatus::ScanCompleted {
            return Err(SessionManifestError::SessionNotCompleted(
                manifest.session.id,
            ));
        }
        ensure_unique_candidate_ids(&manifest.candidates)?;
        Ok(manifest)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SessionManifestError> {
        let path = path.as_ref();
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("evidenceforge-session.json");
        let temporary_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));

        let write_result = write_manifest_atomically(self, &temporary_path, path);
        if write_result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        write_result
    }

    pub fn verify_source(&self) -> SourceIntegrity {
        match ImageSource::inspect(&self.session.source.identity.canonical_path) {
            Ok(source) if self.session.matches_source(&source) => SourceIntegrity::Verified,
            Ok(observed_source) => SourceIntegrity::Changed { observed_source },
            Err(error) => SourceIntegrity::Unavailable {
                detail: error.to_string(),
            },
        }
    }

    pub fn verify_recorded_exports(&self) -> Vec<RecordedExportVerification> {
        self.exports.iter().map(verify_recorded_export).collect()
    }

    pub fn render_case_brief(&self) -> Result<String, SessionManifestError> {
        let source_integrity = self.verify_source();
        let export_audit = self.verify_recorded_exports();
        let generated_at_unix_ms = unix_time_millis()?;
        Ok(render_case_brief(
            self,
            &source_integrity,
            &export_audit,
            generated_at_unix_ms,
        ))
    }

    pub fn save_case_brief(&self, path: impl AsRef<Path>) -> Result<(), SessionManifestError> {
        let path = path.as_ref();
        let case_brief = self.render_case_brief()?;
        write_case_brief_atomically(&case_brief, path)
    }

    pub fn recover_to_destination(
        &mut self,
        candidate_id: &str,
        destination_path: impl AsRef<Path>,
    ) -> Result<RecoveryExport, SessionManifestError> {
        match self.verify_source() {
            SourceIntegrity::Verified => {}
            SourceIntegrity::Changed { .. } => return Err(SessionManifestError::SourceChanged),
            SourceIntegrity::Unavailable { detail } => {
                return Err(SessionManifestError::SourceUnavailable(detail));
            }
        }

        let saved_candidate = self
            .candidates
            .iter()
            .find(|candidate| candidate.id == candidate_id)
            .ok_or_else(|| SessionManifestError::CandidateNotInManifest(candidate_id.to_owned()))?;
        let recovered = recover_candidate_from_image(
            &self.session.source.identity.canonical_path,
            candidate_id,
        )?;
        if recovered.candidate != *saved_candidate {
            return Err(SessionManifestError::CandidateMismatch(
                candidate_id.to_owned(),
            ));
        }

        let export = crate::recover_session_candidate_to_destination(
            &self.session,
            &self.session.source.identity.canonical_path,
            candidate_id,
            destination_path,
        )?;
        self.exports.push(RecordedExport {
            candidate_id: candidate_id.to_owned(),
            exported_at_unix_ms: unix_time_millis()?,
            output_path: export.output_path.clone(),
            receipt_path: export.receipt_path.clone(),
            receipt: export.receipt.clone(),
        });
        Ok(export)
    }
}

fn render_case_brief(
    manifest: &SessionManifest,
    source_integrity: &SourceIntegrity,
    export_audit: &[RecordedExportVerification],
    generated_at_unix_ms: u128,
) -> String {
    let source = &manifest.session.source;
    let mut method_counts = std::collections::BTreeMap::<&str, usize>::new();
    let mut validation_counts = std::collections::BTreeMap::<&str, usize>::new();
    for candidate in &manifest.candidates {
        *method_counts
            .entry(recovery_method_label(candidate.method))
            .or_default() += 1;
        *validation_counts
            .entry(validation_label(candidate.validation))
            .or_default() += 1;
    }

    let mut report = String::new();
    report.push_str("# DiskTrace case brief\n\n");
    report.push_str(&format!(
        "Generated locally (Unix ms): {generated_at_unix_ms}\n\n"
    ));
    report.push_str("> This is a local read-only summary of a completed recovery session. It does not contain source-image bytes or recovered-file payload bytes, and it is not a legal chain-of-custody or authenticity claim.\n\n",
    );
    report.push_str("## Session and source\n\n");
    report.push_str("| Field | Value |\n| --- | --- |\n");
    report.push_str(&format!("| Session ID | `{}` |\n", manifest.session.id));
    report.push_str(&format!(
        "| Source display name | {} |\n",
        markdown_cell(&source.display_name)
    ));
    report.push_str(&format!(
        "| Source path | `{}` |\n",
        source.identity.canonical_path.display()
    ));
    report.push_str(&format!(
        "| Source byte length | {} |\n",
        source.identity.byte_length
    ));
    report.push_str(&format!(
        "| Source SHA-256 | `{}` |\n",
        source.identity.sha256
    ));
    report.push_str(&format!(
        "| Source BLAKE3 | `{}` |\n",
        source.identity.blake3
    ));
    report.push_str(&format!(
        "| Current source status | {} |\n\n",
        markdown_cell(&source_integrity_summary(source_integrity))
    ));

    report.push_str("## Candidate summary\n\n");
    report.push_str(&format!(
        "- **Total candidates:** {}\n",
        manifest.candidates.len()
    ));
    report.push_str("- **By recovery method:**\n");
    if method_counts.is_empty() {
        report.push_str("  - No candidates recorded.\n");
    } else {
        for (method, count) in method_counts {
            report.push_str(&format!("  - {method}: {count}\n"));
        }
    }
    report.push_str("- **By validation state:**\n");
    if validation_counts.is_empty() {
        report.push_str("  - No candidates recorded.\n\n");
    } else {
        for (validation, count) in validation_counts {
            report.push_str(&format!("  - {validation}: {count}\n"));
        }
        report.push('\n');
    }

    report.push_str("## Candidate inventory\n\n");
    report
        .push_str("| Candidate ID | Name | Type | Source offset | Bytes | Method | Validation |\n");
    report.push_str("| --- | --- | --- | ---: | ---: | --- | --- |\n");
    for candidate in &manifest.candidates {
        report.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} |\n",
            markdown_cell(&candidate.id),
            markdown_cell(&candidate.evidence_name),
            markdown_cell(&candidate.file_type),
            candidate.source_offset,
            candidate.byte_length,
            recovery_method_label(candidate.method),
            validation_label(candidate.validation),
        ));
    }
    report.push('\n');

    report.push_str("## Recorded export audit\n\n");
    if export_audit.is_empty() {
        report.push_str("No receipt-backed recovery exports are recorded in this session.\n\n");
    } else {
        report.push_str("| Candidate ID | Output path | Receipt path | Current audit status |\n");
        report.push_str("| --- | --- | --- | --- |\n");
        for verification in export_audit {
            report.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                markdown_cell(&verification.candidate_id),
                verification.output_path.display(),
                verification.receipt_path.display(),
                markdown_cell(&recorded_export_integrity_summary(&verification.integrity)),
            ));
        }
        report.push('\n');
    }

    report.push_str("## Limitations\n\n");
    report.push_str("- The brief records the current local source and export-audit observations only. A verified source or export audit does not establish ownership, original-file authenticity, malware safety, legal admissibility, or recovery completeness.\n");
    report.push_str("- Candidate methods and validation states describe the bounded recovery observation. They do not guarantee that every deleted file was found or that a recovered file is complete.\n");
    report.push_str("- Keep this brief with its local session manifest, receipt files, and source image when you need to repeat these checks later.\n");
    report
}

fn source_integrity_summary(integrity: &SourceIntegrity) -> String {
    match integrity {
        SourceIntegrity::Verified => "Verified: byte length, SHA-256, and BLAKE3 match the saved session.".to_owned(),
        SourceIntegrity::Changed { observed_source } => format!(
            "Changed: current source is {} bytes with SHA-256 {} and does not match the saved session.",
            observed_source.identity.byte_length, observed_source.identity.sha256
        ),
        SourceIntegrity::Unavailable { detail } => format!("Unavailable: {detail}"),
    }
}

fn recorded_export_integrity_summary(integrity: &RecordedExportIntegrity) -> String {
    match integrity {
        RecordedExportIntegrity::Verified => {
            "Verified: persisted receipt and current SHA-256/BLAKE3 match.".to_owned()
        }
        RecordedExportIntegrity::ReceiptUnavailable { detail } => {
            format!("Receipt unavailable: {detail}")
        }
        RecordedExportIntegrity::ReceiptChanged => {
            "Receipt changed: persisted receipt differs from the session record.".to_owned()
        }
        RecordedExportIntegrity::ReceiptInconsistent { detail } => {
            format!("Receipt inconsistent: {detail}")
        }
        RecordedExportIntegrity::ArtifactUnavailable { detail } => {
            format!("Artifact unavailable: {detail}")
        }
        RecordedExportIntegrity::ArtifactChanged { .. } => {
            "Artifact changed: current SHA-256/BLAKE3 differ from the receipt.".to_owned()
        }
    }
}

fn recovery_method_label(method: ef_core::RecoveryMethod) -> &'static str {
    match method {
        ef_core::RecoveryMethod::Fat12DeletedRootMetadata => "Deleted FAT12 metadata",
        ef_core::RecoveryMethod::Fat16DeletedRootMetadata => "Deleted FAT16 metadata",
        ef_core::RecoveryMethod::ExfatDeletedContiguousRootMetadata => {
            "Deleted exFAT contiguous metadata"
        }
        ef_core::RecoveryMethod::NtfsDeletedResidentRecord => "Deleted NTFS resident metadata",
        ef_core::RecoveryMethod::NtfsDeletedContiguousNonresident => {
            "Deleted NTFS contiguous metadata"
        }
        ef_core::RecoveryMethod::SignatureCarvingPng => "PNG signature carving",
        ef_core::RecoveryMethod::SignatureCarvingJpeg => "JPEG signature carving",
        ef_core::RecoveryMethod::SignatureCarvingGif => "GIF structural carving",
        ef_core::RecoveryMethod::SignatureCarvingAvi => "AVI structural carving",
        ef_core::RecoveryMethod::SignatureCarvingMp4 => "MP4/MOV structural carving",
        ef_core::RecoveryMethod::SignatureCarvingPdf => "PDF structural carving",
        ef_core::RecoveryMethod::SignatureCarvingZipOffice => "ZIP and Open XML carving",
    }
}

fn validation_label(validation: ef_core::CandidateValidation) -> &'static str {
    match validation {
        ef_core::CandidateValidation::MetadataVerified => "Metadata verified",
        ef_core::CandidateValidation::ContentValidated => "Recovered and checked",
        ef_core::CandidateValidation::RecoveredUnvalidated => "Recovered — review recommended",
        ef_core::CandidateValidation::PartialOrErrorAffected => "Partial or error affected",
        ef_core::CandidateValidation::Unavailable => "Unavailable",
    }
}

fn markdown_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "<br>")
}

fn write_case_brief_atomically(case_brief: &str, path: &Path) -> Result<(), SessionManifestError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("evidenceforge-case-brief.md");
    let temporary_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));
    let write_result = (|| {
        let mut temporary = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|source| SessionManifestError::WriteCaseBrief {
                path: temporary_path.clone(),
                source,
            })?;
        temporary
            .write_all(case_brief.as_bytes())
            .map_err(|source| SessionManifestError::WriteCaseBrief {
                path: temporary_path.clone(),
                source,
            })?;
        temporary
            .write_all(b"\n")
            .map_err(|source| SessionManifestError::WriteCaseBrief {
                path: temporary_path.clone(),
                source,
            })?;
        temporary
            .sync_all()
            .map_err(|source| SessionManifestError::WriteCaseBrief {
                path: temporary_path.clone(),
                source,
            })?;
        drop(temporary);
        fs::rename(&temporary_path, path).map_err(|source| SessionManifestError::WriteCaseBrief {
            path: path.to_path_buf(),
            source,
        })
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}

fn verify_recorded_export(recorded: &RecordedExport) -> RecordedExportVerification {
    let mut verification = RecordedExportVerification {
        candidate_id: recorded.candidate_id.clone(),
        output_path: recorded.output_path.clone(),
        receipt_path: recorded.receipt_path.clone(),
        integrity: RecordedExportIntegrity::Verified,
    };

    let persisted_receipt = match fs::read(&recorded.receipt_path) {
        Ok(bytes) => match serde_json::from_slice::<RecoveryReceipt>(&bytes) {
            Ok(receipt) => receipt,
            Err(error) => {
                verification.integrity = RecordedExportIntegrity::ReceiptUnavailable {
                    detail: format!("unable to parse receipt: {error}"),
                };
                return verification;
            }
        },
        Err(error) => {
            verification.integrity = RecordedExportIntegrity::ReceiptUnavailable {
                detail: error.to_string(),
            };
            return verification;
        }
    };

    if persisted_receipt != recorded.receipt {
        verification.integrity = RecordedExportIntegrity::ReceiptChanged;
        return verification;
    }

    let [artifact] = recorded.receipt.artifacts.as_slice() else {
        verification.integrity = RecordedExportIntegrity::ReceiptInconsistent {
            detail: "the recorded export receipt must contain exactly one artifact".to_owned(),
        };
        return verification;
    };
    let expected_output_path = recorded.receipt.destination.join(&artifact.relative_path);
    if expected_output_path != recorded.output_path {
        verification.integrity = RecordedExportIntegrity::ReceiptInconsistent {
            detail: "the receipt artifact path does not match the recorded export path".to_owned(),
        };
        return verification;
    }

    match hash_file(&recorded.output_path) {
        Ok((observed_sha256, observed_blake3))
            if observed_sha256 == artifact.sha256 && observed_blake3 == artifact.blake3 => {}
        Ok((observed_sha256, observed_blake3)) => {
            verification.integrity = RecordedExportIntegrity::ArtifactChanged {
                expected_sha256: artifact.sha256.clone(),
                observed_sha256,
                expected_blake3: artifact.blake3.clone(),
                observed_blake3,
            };
        }
        Err(error) => {
            verification.integrity = RecordedExportIntegrity::ArtifactUnavailable {
                detail: error.to_string(),
            };
        }
    }

    verification
}

fn ensure_unique_candidate_ids(
    candidates: &[RecoveryCandidate],
) -> Result<(), SessionManifestError> {
    let mut ids = std::collections::BTreeSet::new();
    for candidate in candidates {
        if !ids.insert(&candidate.id) {
            return Err(SessionManifestError::DuplicateCandidateId(
                candidate.id.clone(),
            ));
        }
    }
    Ok(())
}

fn unix_time_millis() -> Result<u128, SessionManifestError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SessionManifestError::ClockBeforeEpoch)
        .map(|duration| duration.as_millis())
}

fn write_manifest_atomically(
    manifest: &SessionManifest,
    temporary_path: &Path,
    path: &Path,
) -> Result<(), SessionManifestError> {
    let mut temporary = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary_path)
        .map_err(|source| SessionManifestError::WriteManifest {
            path: temporary_path.to_path_buf(),
            source,
        })?;
    serde_json::to_writer_pretty(&mut temporary, manifest)?;
    temporary
        .write_all(b"\n")
        .map_err(|source| SessionManifestError::WriteManifest {
            path: temporary_path.to_path_buf(),
            source,
        })?;
    temporary
        .sync_all()
        .map_err(|source| SessionManifestError::WriteManifest {
            path: temporary_path.to_path_buf(),
            source,
        })?;
    drop(temporary);
    fs::rename(temporary_path, path).map_err(|source| SessionManifestError::WriteManifest {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::{RecordedExportIntegrity, SessionManifest, SessionManifestError, SourceIntegrity};
    use ef_core::{ImageSource, RecoveryCandidate, RecoveryMethod, RecoverySession, SessionStatus};
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("evidenceforge-session-{name}-{}", Uuid::new_v4()))
    }

    fn completed_session(image_path: &PathBuf) -> RecoverySession {
        let source = ImageSource::inspect(image_path).expect("inspect test source");
        let mut session = RecoverySession::create(source).expect("create test session");
        session.status = SessionStatus::ScanCompleted;
        session
    }

    fn candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "png-carve-0000".to_owned(),
            evidence_name: "carved-0000.png".to_owned(),
            file_type: "png".to_owned(),
            source_offset: 0,
            byte_length: 70,
            method: RecoveryMethod::SignatureCarvingPng,
            validation: ef_core::CandidateValidation::ContentValidated,
            original_path: None,
        }
    }

    #[test]
    fn persists_and_reloads_a_completed_session_manifest() {
        let source_path = test_path("persist-source.img");
        let manifest_path = test_path("persist-session.json");
        fs::write(&source_path, b"session source").expect("write source image");
        let manifest = SessionManifest::new(completed_session(&source_path), vec![candidate()])
            .expect("manifest");

        manifest.save(&manifest_path).expect("save manifest");
        let loaded = SessionManifest::load(&manifest_path).expect("load manifest");

        assert_eq!(loaded, manifest);
        assert_eq!(loaded.verify_source(), SourceIntegrity::Verified);
        fs::remove_file(source_path).expect("remove source");
        fs::remove_file(manifest_path).expect("remove manifest");
    }

    #[test]
    fn rejects_unknown_top_level_manifest_fields() {
        let source_path = test_path("unknown-manifest-field.img");
        let manifest_path = test_path("unknown-manifest-field.json");
        fs::write(&source_path, b"session source").expect("write source image");
        let manifest = SessionManifest::new(completed_session(&source_path), vec![candidate()])
            .expect("manifest");
        manifest.save(&manifest_path).expect("save manifest");

        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).expect("read manifest"))
                .expect("parse saved manifest");
        document.as_object_mut().expect("manifest object").insert(
            "unrecognized_forensic_field".to_owned(),
            serde_json::json!(true),
        );
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&document).expect("serialize changed manifest"),
        )
        .expect("write changed manifest");

        assert!(matches!(
            SessionManifest::load(&manifest_path),
            Err(SessionManifestError::ParseManifest { .. })
        ));
        fs::remove_file(source_path).expect("remove source");
        fs::remove_file(manifest_path).expect("remove manifest");
    }

    #[test]
    fn renders_and_saves_a_local_case_brief_without_source_payload_bytes() {
        let root = test_path("case-brief");
        let source_root = root.join("source");
        let brief_path = root.join("case-brief.md");
        fs::create_dir_all(&source_root).expect("create source root");
        let source_path = source_root.join("fixture.img");
        fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/fat12-deleted-file-v1/source.img"),
            &source_path,
        )
        .expect("copy fixture");
        let scan = crate::scan_image(&source_path).expect("scan fixture");
        let manifest =
            SessionManifest::new(scan.session, scan.candidates).expect("create manifest");

        let fat_candidate_id = manifest.candidates[0].id.clone();
        let brief = manifest.render_case_brief().expect("render case brief");
        assert!(brief.contains("# DiskTrace case brief"));
        assert!(brief.contains("Source SHA-256"));
        assert!(brief.contains(&fat_candidate_id));
        assert!(brief.contains("Verified: byte length, SHA-256, and BLAKE3 match"));
        assert!(!brief.contains("recover me"));

        manifest
            .save_case_brief(&brief_path)
            .expect("save case brief");
        let saved_brief = fs::read_to_string(&brief_path).expect("read case brief");
        assert!(saved_brief.ends_with('\n'));
        assert!(saved_brief.contains("# DiskTrace case brief"));
        assert!(saved_brief.contains(&fat_candidate_id));
        assert!(saved_brief.contains("No receipt-backed recovery exports are recorded"));

        fs::write(&source_path, b"changed source").expect("change source");
        let changed_brief = manifest
            .render_case_brief()
            .expect("render changed case brief");
        assert!(changed_brief.contains("Current source status | Changed:"));
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn detects_a_changed_source_without_mutating_the_original_identity() {
        let source_path = test_path("changed-source.img");
        fs::write(&source_path, b"original source").expect("write original source");
        let manifest = SessionManifest::new(completed_session(&source_path), vec![candidate()])
            .expect("manifest");
        let original_identity = manifest.session.source.identity.clone();

        fs::write(&source_path, b"substituted source").expect("replace source image");

        assert!(matches!(
            manifest.verify_source(),
            SourceIntegrity::Changed { .. }
        ));
        assert_eq!(manifest.session.source.identity, original_identity);
        fs::remove_file(source_path).expect("remove source");
    }

    #[test]
    fn rejects_duplicate_candidate_ids_and_incomplete_sessions() {
        let source_path = test_path("invalid-session.img");
        fs::write(&source_path, b"session source").expect("write source");
        let source = ImageSource::inspect(&source_path).expect("inspect source");
        let incomplete = RecoverySession::create(source).expect("create session");

        assert!(matches!(
            SessionManifest::new(incomplete, vec![candidate()]),
            Err(SessionManifestError::SessionNotCompleted(_))
        ));
        assert!(matches!(
            SessionManifest::new(
                completed_session(&source_path),
                vec![candidate(), candidate()]
            ),
            Err(SessionManifestError::DuplicateCandidateId(_))
        ));
        fs::remove_file(source_path).expect("remove source");
    }

    #[test]
    fn recovery_export_uses_the_saved_session_and_is_retained_in_history() {
        let root = test_path("export-history");
        let source_root = root.join("source");
        let destination_root = root.join("destination");
        let manifest_path = root.join("session.json");
        fs::create_dir_all(&source_root).expect("create source root");
        fs::create_dir_all(&destination_root).expect("create destination root");
        let source_path = source_root.join("fixture.img");
        fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/fat12-deleted-file-v1/source.img"),
            &source_path,
        )
        .expect("copy fixture");
        let scan = crate::scan_image(&source_path).expect("scan fixture");
        let mut manifest =
            SessionManifest::new(scan.session, scan.candidates).expect("create manifest");

        let candidate_id = manifest.candidates[0].id.clone();
        let export = manifest
            .recover_to_destination(&candidate_id, &destination_root)
            .expect("recover from saved session");
        manifest.save(&manifest_path).expect("save manifest");
        let loaded = SessionManifest::load(&manifest_path).expect("reload manifest");

        assert_eq!(export.receipt.session_id, manifest.session.id);
        assert_eq!(loaded.exports.len(), 1);
        assert_eq!(loaded.exports[0].candidate_id, candidate_id);
        assert_eq!(loaded.exports[0].receipt, export.receipt);
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn audits_receipt_backed_exports_and_detects_artifact_or_receipt_changes() {
        let root = test_path("export-audit");
        let source_root = root.join("source");
        let destination_root = root.join("destination");
        fs::create_dir_all(&source_root).expect("create source root");
        fs::create_dir_all(&destination_root).expect("create destination root");
        let source_path = source_root.join("fixture.img");
        fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/fat12-deleted-file-v1/source.img"),
            &source_path,
        )
        .expect("copy fixture");
        let scan = crate::scan_image(&source_path).expect("scan fixture");
        let mut manifest =
            SessionManifest::new(scan.session, scan.candidates).expect("create manifest");
        let candidate_id = manifest.candidates[0].id.clone();
        manifest
            .recover_to_destination(&candidate_id, &destination_root)
            .expect("recover from saved session");

        let verified = manifest.verify_recorded_exports();
        assert_eq!(verified.len(), 1);
        assert_eq!(verified[0].integrity, RecordedExportIntegrity::Verified);

        let output_path = manifest.exports[0].output_path.clone();
        fs::write(&output_path, b"changed recovered bytes").expect("change recovered artifact");
        let changed_artifact = manifest.verify_recorded_exports();
        assert!(matches!(
            changed_artifact[0].integrity,
            RecordedExportIntegrity::ArtifactChanged { .. }
        ));

        let receipt_path = manifest.exports[0].receipt_path.clone();
        let mut changed_receipt = manifest.exports[0].receipt.clone();
        changed_receipt.source_byte_length += 1;
        changed_receipt
            .write_json(&receipt_path)
            .expect("write changed receipt");
        let changed_receipt_audit = manifest.verify_recorded_exports();
        assert_eq!(
            changed_receipt_audit[0].integrity,
            RecordedExportIntegrity::ReceiptChanged
        );

        fs::remove_file(&receipt_path).expect("remove receipt");
        let unavailable_receipt = manifest.verify_recorded_exports();
        assert!(matches!(
            unavailable_receipt[0].integrity,
            RecordedExportIntegrity::ReceiptUnavailable { .. }
        ));
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn recovery_from_a_saved_session_is_blocked_after_source_changes() {
        let source_path = test_path("blocked-export.img");
        let destination_root = test_path("blocked-export-destination");
        fs::write(&source_path, b"session source").expect("write source");
        fs::create_dir_all(&destination_root).expect("create destination");
        let mut manifest = SessionManifest::new(completed_session(&source_path), vec![candidate()])
            .expect("manifest");
        fs::write(&source_path, b"changed source").expect("change source");

        assert!(matches!(
            manifest.recover_to_destination("png-carve-0000", &destination_root),
            Err(SessionManifestError::SourceChanged)
        ));
        assert!(manifest.exports.is_empty());
        fs::remove_file(source_path).expect("remove source");
        fs::remove_dir_all(destination_root).expect("remove destination");
    }
}
