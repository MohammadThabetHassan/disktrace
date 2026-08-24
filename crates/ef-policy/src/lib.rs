use ef_core::ImageSource;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DestinationPolicyError {
    #[error("destination does not exist: {0}")]
    Missing(PathBuf),
    #[error("destination is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("destination resolves inside the source image storage location: {0}")]
    SourceStorage(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedDestination {
    pub canonical_path: PathBuf,
}

pub fn approve_destination(
    source: &ImageSource,
    destination: impl AsRef<Path>,
) -> Result<ApprovedDestination, DestinationPolicyError> {
    let destination = destination.as_ref();
    if !destination.exists() {
        return Err(DestinationPolicyError::Missing(destination.to_path_buf()));
    }
    if !destination.is_dir() {
        return Err(DestinationPolicyError::NotDirectory(
            destination.to_path_buf(),
        ));
    }

    let canonical_path = destination
        .canonicalize()
        .map_err(|_| DestinationPolicyError::Missing(destination.to_path_buf()))?;
    let source_storage_root = source
        .identity
        .canonical_path
        .parent()
        .expect("canonical image path has a parent");

    if canonical_path.starts_with(source_storage_root) {
        return Err(DestinationPolicyError::SourceStorage(canonical_path));
    }

    Ok(ApprovedDestination { canonical_path })
}

#[cfg(test)]
mod tests {
    use super::{approve_destination, DestinationPolicyError};
    use ef_core::ImageSource;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("evidenceforge-policy-{name}-{}", Uuid::new_v4()))
    }

    #[test]
    fn rejects_source_storage_and_nested_paths() {
        let source_root = test_path("source-root");
        let nested_destination = source_root.join("exports");
        fs::create_dir_all(&nested_destination).expect("create source storage");
        let source_path = source_root.join("source.img");
        fs::write(&source_path, b"source").expect("write source image");
        let source = ImageSource::inspect(&source_path).expect("inspect source");

        let error =
            approve_destination(&source, &nested_destination).expect_err("reject source storage");

        assert!(matches!(error, DestinationPolicyError::SourceStorage(_)));
        fs::remove_dir_all(source_root).expect("remove source storage");
    }

    #[test]
    fn approves_separate_destination_directory() {
        let source_root = test_path("source-root");
        let destination_root = test_path("destination-root");
        fs::create_dir_all(&source_root).expect("create source root");
        fs::create_dir_all(&destination_root).expect("create destination root");
        let source_path = source_root.join("source.img");
        fs::write(&source_path, b"source").expect("write source image");
        let source = ImageSource::inspect(&source_path).expect("inspect source");

        let approved =
            approve_destination(&source, &destination_root).expect("approve destination");

        assert_eq!(
            approved.canonical_path,
            destination_root
                .canonicalize()
                .expect("canonical destination")
        );
        fs::remove_dir_all(source_root).expect("remove source root");
        fs::remove_dir_all(destination_root).expect("remove destination root");
    }

    #[test]
    fn rejects_missing_destination_directory() {
        let source_root = test_path("source-root");
        fs::create_dir_all(&source_root).expect("create source root");
        let source_path = source_root.join("source.img");
        fs::write(&source_path, b"source").expect("write source image");
        let source = ImageSource::inspect(&source_path).expect("inspect source");
        let missing_destination = test_path("missing-destination");

        let error = approve_destination(&source, &missing_destination)
            .expect_err("reject missing destination");

        assert_eq!(error, DestinationPolicyError::Missing(missing_destination));
        fs::remove_dir_all(source_root).expect("remove source root");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_that_resolves_into_source_storage() {
        use std::os::unix::fs::symlink;

        let source_root = test_path("source-root");
        let external_root = test_path("external-root");
        fs::create_dir_all(&source_root).expect("create source root");
        fs::create_dir_all(&external_root).expect("create external root");
        let source_path = source_root.join("source.img");
        let symlink_path = external_root.join("source-link");
        fs::write(&source_path, b"source").expect("write source image");
        symlink(&source_root, &symlink_path).expect("create source storage symlink");
        let source = ImageSource::inspect(&source_path).expect("inspect source");

        let error =
            approve_destination(&source, &symlink_path).expect_err("reject linked source storage");

        assert!(matches!(error, DestinationPolicyError::SourceStorage(_)));
        fs::remove_dir_all(source_root).expect("remove source root");
        fs::remove_dir_all(external_root).expect("remove external root");
    }
}
