pub use ef_core::CandidateValidation as ValidationState;
use ef_core::{hash_file, RecoverySession};
use ef_policy::ApprovedDestination;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("artifact is outside the approved destination: {0}")]
    ArtifactOutsideDestination(PathBuf),
    #[error("unable to write receipt '{path}': {source}")]
    WriteReceipt {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unable to serialize receipt: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error(transparent)]
    Core(#[from] ef_core::CoreError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedArtifact {
    pub relative_path: PathBuf,
    pub source_range_start: u64,
    pub source_range_length: u64,
    pub recovery_method: String,
    pub validation: ValidationState,
    pub sha256: String,
    pub blake3: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryReceipt {
    pub receipt_id: Uuid,
    pub session_id: Uuid,
    pub source_sha256: String,
    pub source_blake3: String,
    pub source_byte_length: u64,
    pub destination: PathBuf,
    pub artifacts: Vec<ExportedArtifact>,
}

impl RecoveryReceipt {
    pub fn create(
        session: &RecoverySession,
        destination: &ApprovedDestination,
        artifact_paths: impl IntoIterator<Item = (PathBuf, u64, u64, String, ValidationState)>,
    ) -> Result<Self, ReportError> {
        let mut artifacts = Vec::new();

        for (relative_path, source_range_start, source_range_length, recovery_method, validation) in
            artifact_paths
        {
            let absolute_path = destination.canonical_path.join(&relative_path);
            let canonical_artifact = absolute_path
                .canonicalize()
                .map_err(|_| ReportError::ArtifactOutsideDestination(absolute_path.clone()))?;
            if !canonical_artifact.starts_with(&destination.canonical_path) {
                return Err(ReportError::ArtifactOutsideDestination(canonical_artifact));
            }
            let (sha256, blake3) = hash_file(&canonical_artifact)?;
            artifacts.push(ExportedArtifact {
                relative_path,
                source_range_start,
                source_range_length,
                recovery_method,
                validation,
                sha256,
                blake3,
            });
        }

        Ok(Self {
            receipt_id: Uuid::new_v4(),
            session_id: session.id,
            source_sha256: session.source.identity.sha256.clone(),
            source_blake3: session.source.identity.blake3.clone(),
            source_byte_length: session.source.identity.byte_length,
            destination: destination.canonical_path.clone(),
            artifacts,
        })
    }

    pub fn write_json(&self, output_path: impl AsRef<Path>) -> Result<(), ReportError> {
        let output_path = output_path.as_ref();
        let serialized = serde_json::to_vec_pretty(self)?;
        fs::write(output_path, serialized).map_err(|source| ReportError::WriteReceipt {
            path: output_path.to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{RecoveryReceipt, ValidationState};
    use ef_core::{ImageSource, RecoverySession};
    use ef_policy::approve_destination;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("evidenceforge-report-{name}-{}", Uuid::new_v4()))
    }

    #[test]
    fn receipt_records_artifact_hashes_and_source_identity() {
        let source_root = test_path("source");
        let destination_root = test_path("destination");
        fs::create_dir_all(&source_root).expect("create source root");
        fs::create_dir_all(&destination_root).expect("create destination root");
        let source_path = source_root.join("source.img");
        let artifact_path = destination_root.join("recovered.txt");
        fs::write(&source_path, b"source image").expect("write source image");
        fs::write(&artifact_path, b"restored content").expect("write recovered artifact");
        let source = ImageSource::inspect(&source_path).expect("inspect source");
        let session = RecoverySession::create(source).expect("create session");
        let destination =
            approve_destination(&session.source, &destination_root).expect("approve destination");

        let receipt = RecoveryReceipt::create(
            &session,
            &destination,
            [(
                PathBuf::from("recovered.txt"),
                4096,
                16,
                "metadata".to_owned(),
                ValidationState::MetadataVerified,
            )],
        )
        .expect("create receipt");

        assert_eq!(receipt.artifacts.len(), 1);
        assert_eq!(
            receipt.artifacts[0].validation,
            ValidationState::MetadataVerified
        );
        assert_eq!(receipt.source_sha256, session.source.identity.sha256);
        fs::remove_dir_all(source_root).expect("remove source root");
        fs::remove_dir_all(destination_root).expect("remove destination root");
    }
}
