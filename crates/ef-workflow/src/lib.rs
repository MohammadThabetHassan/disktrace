use blake3::Hasher;
use ef_carve::{
    carve_avis, carve_gifs, carve_jpegs, carve_mp4s, carve_pdfs, carve_pngs, carve_zip_archives,
    extract_avi, extract_gif, extract_jpeg, extract_mp4, extract_pdf, extract_png, extract_zip,
    AviCarvedCandidate, CarveError, GifCarvedCandidate, JpegCarvedCandidate, Mp4CarvedCandidate,
    PdfCarvedCandidate, PngCarvedCandidate, ZipCarvedCandidate,
};
use ef_core::{
    read_range_from_file_with_cancellation, read_verified_range_with_cancellation,
    CandidateValidation, CoreError, ImageSource, RecoveryCandidate, RecoveryMethod,
    RecoverySession, SessionStatus, SourceIdentity, SourceRange,
};
use ef_fat::{
    DeletedExfatRootFile, DeletedNtfsContiguousFile, DeletedNtfsResidentFile, DeletedRootFile,
    ExfatError, ExfatVolume, Fat12Error, Fat12Volume, Fat16Error, Fat16Volume, NtfsError,
    NtfsVolume,
};
use ef_policy::{approve_destination, DestinationPolicyError};
use ef_report::{RecoveryReceipt, ReportError, ValidationState};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

pub mod session;
pub use session::{
    RecordedExport, RecordedExportIntegrity, RecordedExportVerification, SessionManifest,
    SessionManifestError, SourceIntegrity, SESSION_MANIFEST_SCHEMA_VERSION,
};

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Fat12(#[from] Fat12Error),
    #[error(transparent)]
    Fat16(#[from] Fat16Error),
    #[error(transparent)]
    Exfat(#[from] ExfatError),
    #[error(transparent)]
    Ntfs(#[from] NtfsError),
    #[error(transparent)]
    Carve(#[from] CarveError),
    #[error(transparent)]
    Policy(#[from] DestinationPolicyError),
    #[error(transparent)]
    Report(#[from] ReportError),
    #[error("unable to read recovery image '{path}': {source}")]
    ReadImage {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unable to create recovery output '{path}': {source}")]
    CreateOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unable to write recovery output '{path}': {source}")]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("candidate id '{0}' has an unsupported recovery method")]
    UnsupportedCandidate(String),
    #[error("candidate id '{0}' is not available from this image")]
    CandidateUnavailable(String),
    #[error("candidate id '{0}' is not present in the local recovery session")]
    CandidateNotInSession(String),
    #[error("the source image no longer matches the recovery session")]
    SourceIdentityMismatch,
    #[error("windowed PNG discovery diverged from compatibility discovery")]
    WindowedPngParity,
    #[error("windowed JPEG discovery diverged from compatibility discovery")]
    WindowedJpegParity,
    #[error("windowed GIF discovery diverged from compatibility discovery")]
    WindowedGifParity,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub session: RecoverySession,
    pub candidates: Vec<RecoveryCandidate>,
}

#[derive(Debug, Clone)]
pub struct RecoveredCandidate {
    pub candidate: RecoveryCandidate,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RecoveryExport {
    pub output_path: PathBuf,
    pub receipt_path: PathBuf,
    pub receipt: RecoveryReceipt,
}

pub fn scan_image(path: impl AsRef<Path>) -> Result<ScanResult, WorkflowError> {
    scan_image_with_cancellation(path, &AtomicBool::new(false))
}

pub fn scan_image_with_cancellation(
    path: impl AsRef<Path>,
    cancellation: &AtomicBool,
) -> Result<ScanResult, WorkflowError> {
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }
    let path = path.as_ref();
    let source = ImageSource::inspect_with_cancellation(path, cancellation)?;
    let mut session = RecoverySession::create(source)?;
    let image = read_image_with_cancellation(path, cancellation)?;
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }
    let candidates = discover_candidates_for_scan(&image, &session.source.identity, cancellation)?;
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }
    session.status = SessionStatus::ScanCompleted;
    Ok(ScanResult {
        session,
        candidates,
    })
}

pub fn read_session_candidate_range(
    manifest: &SessionManifest,
    candidate_id: &str,
    cancellation: &AtomicBool,
) -> Result<RecoveredCandidate, WorkflowError> {
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }
    let candidate = manifest
        .candidates
        .iter()
        .find(|candidate| candidate.id == candidate_id)
        .cloned()
        .ok_or_else(|| WorkflowError::CandidateNotInSession(candidate_id.to_owned()))?;
    let bytes = read_verified_range_with_cancellation(
        &manifest.session.source.identity,
        SourceRange {
            offset: candidate.source_offset,
            length: candidate.byte_length,
        },
        cancellation,
    )?;
    Ok(RecoveredCandidate { candidate, bytes })
}

pub fn recover_candidate_from_image(
    path: impl AsRef<Path>,
    candidate_id: &str,
) -> Result<RecoveredCandidate, WorkflowError> {
    recover_candidate_from_image_with_cancellation(path, candidate_id, &AtomicBool::new(false))
}

pub fn recover_candidate_from_image_with_cancellation(
    path: impl AsRef<Path>,
    candidate_id: &str,
    cancellation: &AtomicBool,
) -> Result<RecoveredCandidate, WorkflowError> {
    let path = path.as_ref();
    let image = read_image_with_cancellation(path, cancellation)?;
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }
    recover_candidate(&image, candidate_id)
}

pub fn recover_to_destination(
    image_path: impl AsRef<Path>,
    candidate_id: &str,
    destination_path: impl AsRef<Path>,
) -> Result<RecoveryExport, WorkflowError> {
    let image_path = image_path.as_ref();
    let source = ImageSource::inspect(image_path)?;
    let mut session = RecoverySession::create(source)?;
    session.status = SessionStatus::ScanCompleted;
    recover_session_candidate_to_destination(&session, image_path, candidate_id, destination_path)
}

pub fn recover_session_candidate_to_destination(
    session: &RecoverySession,
    image_path: impl AsRef<Path>,
    candidate_id: &str,
    destination_path: impl AsRef<Path>,
) -> Result<RecoveryExport, WorkflowError> {
    let image_path = image_path.as_ref();
    let current_source = ImageSource::inspect(image_path)?;
    if !session.matches_source(&current_source) {
        return Err(WorkflowError::SourceIdentityMismatch);
    }

    let destination = approve_destination(&session.source, destination_path)?;
    let image = read_image(image_path)?;
    let recovered = recover_candidate(&image, candidate_id)?;
    let relative_path = safe_export_name(&recovered.candidate);
    let output_path = destination.canonical_path.join(&relative_path);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_path)
        .map_err(|source| WorkflowError::CreateOutput {
            path: output_path.clone(),
            source,
        })?;
    output
        .write_all(&recovered.bytes)
        .map_err(|source| WorkflowError::WriteOutput {
            path: output_path.clone(),
            source,
        })?;
    output
        .sync_all()
        .map_err(|source| WorkflowError::WriteOutput {
            path: output_path.clone(),
            source,
        })?;

    let receipt = RecoveryReceipt::create(
        session,
        &destination,
        [(
            relative_path.clone(),
            recovered.candidate.source_offset,
            recovered.candidate.byte_length,
            recovery_method_name(recovered.candidate.method).to_owned(),
            validation_state(recovered.candidate.validation),
        )],
    )?;
    let receipt_path = destination
        .canonical_path
        .join(format!("{}.receipt.json", relative_path.display()));
    receipt.write_json(&receipt_path)?;

    Ok(RecoveryExport {
        output_path,
        receipt_path,
        receipt,
    })
}

fn discover_candidates_legacy(image: &[u8]) -> Vec<RecoveryCandidate> {
    discover_candidates_legacy_with_cancellation(image, &AtomicBool::new(false), || {})
        .expect("a disabled cancellation flag must not interrupt legacy discovery")
}

fn discover_candidates_legacy_with_cancellation<F>(
    image: &[u8],
    cancellation: &AtomicBool,
    mut after_method_stage: F,
) -> Result<Vec<RecoveryCandidate>, WorkflowError>
where
    F: FnMut(),
{
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }

    let mut candidates = Vec::new();

    if let Ok(volume) = Fat12Volume::parse(image) {
        for (index, file) in volume.find_deleted_root_files().iter().enumerate() {
            if let Ok(source_offset) = volume.source_offset_for_candidate(file) {
                candidates.push(fat_candidate(
                    format!("fat12-root-{index:04}"),
                    file,
                    source_offset,
                    RecoveryMethod::Fat12DeletedRootMetadata,
                ));
            }
        }
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    if let Ok(volume) = Fat16Volume::parse(image) {
        for (index, file) in volume.find_deleted_root_files().iter().enumerate() {
            if let Ok(source_offset) = volume.source_offset_for_candidate(file) {
                candidates.push(fat_candidate(
                    format!("fat16-root-{index:04}"),
                    file,
                    source_offset,
                    RecoveryMethod::Fat16DeletedRootMetadata,
                ));
            }
        }
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    if let Ok(volume) = ExfatVolume::parse(image) {
        for (index, file) in volume.find_deleted_root_files().iter().enumerate() {
            if let Ok(source_offset) = volume.source_offset_for_candidate(file) {
                candidates.push(exfat_candidate(
                    format!("exfat-root-{index:04}"),
                    file,
                    source_offset,
                ));
            }
        }
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    if let Ok(volume) = NtfsVolume::parse(image) {
        for (index, file) in volume.find_deleted_resident_files().iter().enumerate() {
            if let Ok(source_offset) = volume.source_offset_for_candidate(file) {
                candidates.push(ntfs_candidate(
                    format!("ntfs-resident-{index:04}"),
                    file,
                    source_offset,
                ));
            }
        }
        for (index, file) in volume.find_deleted_contiguous_files().iter().enumerate() {
            if let Ok(source_offset) = volume.source_offset_for_contiguous_candidate(file) {
                candidates.push(ntfs_contiguous_candidate(
                    format!("ntfs-contiguous-{index:04}"),
                    file,
                    source_offset,
                ));
            }
        }
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    for (index, png) in carve_pngs(image).into_iter().enumerate() {
        candidates.push(png_candidate(format!("png-carve-{index:04}"), png));
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    for (index, jpeg) in carve_jpegs(image).into_iter().enumerate() {
        candidates.push(jpeg_candidate(format!("jpeg-carve-{index:04}"), jpeg));
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    for (index, gif) in carve_gifs(image).into_iter().enumerate() {
        candidates.push(gif_candidate(format!("gif-carve-{index:04}"), gif));
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    for (index, avi) in carve_avis(image).into_iter().enumerate() {
        candidates.push(avi_candidate(format!("avi-carve-{index:04}"), avi));
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    for (index, mp4) in carve_mp4s(image).into_iter().enumerate() {
        candidates.push(mp4_candidate(format!("mp4-carve-{index:04}"), mp4));
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    for (index, pdf) in carve_pdfs(image).into_iter().enumerate() {
        candidates.push(pdf_candidate(format!("pdf-carve-{index:04}"), pdf));
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    for (index, zip) in carve_zip_archives(image).into_iter().enumerate() {
        candidates.push(zip_candidate(format!("zip-carve-{index:04}"), zip));
    }
    observe_legacy_discovery_stage(cancellation, &mut after_method_stage)?;

    Ok(candidates)
}

fn observe_legacy_discovery_stage<F>(
    cancellation: &AtomicBool,
    after_method_stage: &mut F,
) -> Result<(), WorkflowError>
where
    F: FnMut(),
{
    after_method_stage();
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }
    Ok(())
}

pub fn discover_candidates(image: &[u8]) -> Vec<RecoveryCandidate> {
    discover_candidates_legacy(image)
        .into_iter()
        .map(with_stable_candidate_id)
        .collect()
}

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const PNG_IHDR: [u8; 4] = *b"IHDR";
const PNG_IEND: [u8; 4] = *b"IEND";
const PNG_WINDOW_PRIMARY_LENGTH: u64 = 1024 * 1024;
const PNG_SIGNATURE_OVERLAP: u64 = PNG_SIGNATURE.len() as u64 - 1;
const PNG_CHUNK_HEADER_LENGTH: u64 = 12;

const JPEG_SOI: [u8; 2] = [0xff, 0xd8];
const JPEG_WINDOW_PRIMARY_LENGTH: u64 = PNG_WINDOW_PRIMARY_LENGTH;
const JPEG_SIGNATURE_OVERLAP: u64 = JPEG_SOI.len() as u64 - 1;
const JPEG_MAX_CARVE_LENGTH: u64 = 128 * 1024 * 1024;

const GIF87A_HEADER: [u8; 6] = *b"GIF87a";
const GIF89A_HEADER: [u8; 6] = *b"GIF89a";
const GIF_TRAILER: u8 = 0x3b;
const GIF_WINDOW_PRIMARY_LENGTH: u64 = PNG_WINDOW_PRIMARY_LENGTH;
const GIF_SIGNATURE_OVERLAP: u64 = GIF87A_HEADER.len() as u64 - 1;
const GIF_MAX_CARVE_LENGTH: u64 = 64 * 1024 * 1024;

fn discover_candidates_for_scan(
    image: &[u8],
    source_identity: &SourceIdentity,
    cancellation: &AtomicBool,
) -> Result<Vec<RecoveryCandidate>, WorkflowError> {
    let mut candidates: Vec<RecoveryCandidate> =
        discover_candidates_legacy_with_cancellation(image, cancellation, || {})?
            .into_iter()
            .map(with_stable_candidate_id)
            .collect();
    let windowed_png_candidates = discover_windowed_png_candidates(source_identity, cancellation)?;
    let legacy_png_count = candidates
        .iter()
        .filter(|candidate| candidate.method == RecoveryMethod::SignatureCarvingPng)
        .count();

    if legacy_png_count != windowed_png_candidates.len() {
        return Err(WorkflowError::WindowedPngParity);
    }

    let mut windowed_png_candidates = windowed_png_candidates.into_iter();
    for candidate in &mut candidates {
        if candidate.method == RecoveryMethod::SignatureCarvingPng {
            let replacement = windowed_png_candidates
                .next()
                .ok_or(WorkflowError::WindowedPngParity)?;
            if *candidate != replacement {
                return Err(WorkflowError::WindowedPngParity);
            }
            *candidate = replacement;
        }
    }

    if windowed_png_candidates.next().is_some() {
        return Err(WorkflowError::WindowedPngParity);
    }

    let windowed_jpeg_candidates =
        discover_windowed_jpeg_candidates(source_identity, cancellation)?;
    let legacy_jpeg_count = candidates
        .iter()
        .filter(|candidate| candidate.method == RecoveryMethod::SignatureCarvingJpeg)
        .count();

    if legacy_jpeg_count != windowed_jpeg_candidates.len() {
        return Err(WorkflowError::WindowedJpegParity);
    }

    let mut windowed_jpeg_candidates = windowed_jpeg_candidates.into_iter();
    for candidate in &mut candidates {
        if candidate.method == RecoveryMethod::SignatureCarvingJpeg {
            let replacement = windowed_jpeg_candidates
                .next()
                .ok_or(WorkflowError::WindowedJpegParity)?;
            if *candidate != replacement {
                return Err(WorkflowError::WindowedJpegParity);
            }
            *candidate = replacement;
        }
    }

    if windowed_jpeg_candidates.next().is_some() {
        return Err(WorkflowError::WindowedJpegParity);
    }

    let windowed_gif_candidates = discover_windowed_gif_candidates(source_identity, cancellation)?;
    let legacy_gif_count = candidates
        .iter()
        .filter(|candidate| candidate.method == RecoveryMethod::SignatureCarvingGif)
        .count();

    if legacy_gif_count != windowed_gif_candidates.len() {
        return Err(WorkflowError::WindowedGifParity);
    }

    let mut windowed_gif_candidates = windowed_gif_candidates.into_iter();
    for candidate in &mut candidates {
        if candidate.method == RecoveryMethod::SignatureCarvingGif {
            let replacement = windowed_gif_candidates
                .next()
                .ok_or(WorkflowError::WindowedGifParity)?;
            if *candidate != replacement {
                return Err(WorkflowError::WindowedGifParity);
            }
            *candidate = replacement;
        }
    }

    if windowed_gif_candidates.next().is_some() {
        return Err(WorkflowError::WindowedGifParity);
    }

    Ok(candidates)
}

fn discover_windowed_png_candidates(
    source_identity: &SourceIdentity,
    cancellation: &AtomicBool,
) -> Result<Vec<RecoveryCandidate>, WorkflowError> {
    discover_windowed_png_candidates_after_window(source_identity, cancellation, || {})
}

fn discover_windowed_png_candidates_after_window<F>(
    source_identity: &SourceIdentity,
    cancellation: &AtomicBool,
    mut after_primary_window: F,
) -> Result<Vec<RecoveryCandidate>, WorkflowError>
where
    F: FnMut(),
{
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }

    let path = &source_identity.canonical_path;
    let mut file = File::open(path).map_err(|source| WorkflowError::ReadImage {
        path: path.clone(),
        source,
    })?;
    let mut candidates = Vec::new();
    let mut primary_start = 0_u64;
    let mut suppression_end = 0_u64;

    while primary_start < source_identity.byte_length {
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled.into());
        }
        let remaining = source_identity.byte_length - primary_start;
        let primary_length = remaining.min(PNG_WINDOW_PRIMARY_LENGTH);
        let readable_length = primary_length
            .checked_add(PNG_SIGNATURE_OVERLAP)
            .map(|length| length.min(remaining))
            .ok_or(CoreError::RangeOverflow {
                offset: primary_start,
                length: primary_length,
            })?;
        let window = read_range_from_file_with_cancellation(
            &mut file,
            path,
            source_identity.byte_length,
            SourceRange {
                offset: primary_start,
                length: readable_length,
            },
            cancellation,
        )?;
        let primary_length =
            usize::try_from(primary_length).map_err(|_| CoreError::RangeAllocation {
                length: primary_length,
            })?;

        for relative_start in 0..primary_length {
            let signature_end = relative_start.checked_add(PNG_SIGNATURE.len()).ok_or(
                CoreError::RangeOverflow {
                    offset: primary_start,
                    length: PNG_SIGNATURE.len() as u64,
                },
            )?;
            if window.get(relative_start..signature_end) != Some(&PNG_SIGNATURE) {
                continue;
            }
            let source_offset = primary_start.checked_add(relative_start as u64).ok_or(
                CoreError::RangeOverflow {
                    offset: primary_start,
                    length: relative_start as u64,
                },
            )?;
            if source_offset < suppression_end {
                continue;
            }
            let Some(byte_length) =
                parse_windowed_png_length(&mut file, source_identity, source_offset, cancellation)?
            else {
                continue;
            };
            suppression_end =
                source_offset
                    .checked_add(byte_length)
                    .ok_or(CoreError::RangeOverflow {
                        offset: source_offset,
                        length: byte_length,
                    })?;
            let index = candidates.len();
            candidates.push(with_stable_candidate_id(png_candidate(
                format!("png-carve-{index:04}"),
                PngCarvedCandidate {
                    evidence_name: format!("carved-png-{index:04}.png"),
                    source_offset,
                    byte_length,
                },
            )));
        }

        primary_start =
            primary_start
                .checked_add(primary_length as u64)
                .ok_or(CoreError::RangeOverflow {
                    offset: primary_start,
                    length: primary_length as u64,
                })?;
        after_primary_window();
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled.into());
        }
    }

    Ok(candidates)
}

fn parse_windowed_png_length(
    file: &mut File,
    source_identity: &SourceIdentity,
    source_offset: u64,
    cancellation: &AtomicBool,
) -> Result<Option<u64>, WorkflowError> {
    let Some(signature) = read_windowed_png_range(
        file,
        source_identity,
        source_offset,
        PNG_SIGNATURE.len() as u64,
        cancellation,
    )?
    else {
        return Ok(None);
    };
    if signature.as_slice() != PNG_SIGNATURE {
        return Ok(None);
    }

    let mut chunk_offset = source_offset
        .checked_add(PNG_SIGNATURE.len() as u64)
        .ok_or(CoreError::RangeOverflow {
            offset: source_offset,
            length: PNG_SIGNATURE.len() as u64,
        })?;
    let mut chunk_index = 0_u64;

    loop {
        let Some(chunk_header) =
            read_windowed_png_range(file, source_identity, chunk_offset, 8, cancellation)?
        else {
            return Ok(None);
        };
        let chunk_length = u64::from(u32::from_be_bytes(
            chunk_header[..4]
                .try_into()
                .expect("fixed PNG chunk header length"),
        ));
        let chunk_type: [u8; 4] = chunk_header[4..8]
            .try_into()
            .expect("fixed PNG chunk type length");
        let chunk_end = chunk_offset
            .checked_add(PNG_CHUNK_HEADER_LENGTH)
            .and_then(|offset| offset.checked_add(chunk_length))
            .ok_or(CoreError::RangeOverflow {
                offset: chunk_offset,
                length: chunk_length,
            })?;
        if chunk_end > source_identity.byte_length {
            return Ok(None);
        }

        if chunk_index == 0 && (chunk_type != PNG_IHDR || chunk_length != 13) {
            return Ok(None);
        }
        if chunk_type == PNG_IEND {
            return Ok((chunk_length == 0).then_some(chunk_end - source_offset));
        }

        chunk_offset = chunk_end;
        chunk_index = chunk_index.checked_add(1).ok_or(CoreError::RangeOverflow {
            offset: chunk_index,
            length: 1,
        })?;
    }
}

fn read_windowed_png_range(
    file: &mut File,
    source_identity: &SourceIdentity,
    offset: u64,
    length: u64,
    cancellation: &AtomicBool,
) -> Result<Option<Vec<u8>>, WorkflowError> {
    let end = offset
        .checked_add(length)
        .ok_or(CoreError::RangeOverflow { offset, length })?;
    if end > source_identity.byte_length {
        return Ok(None);
    }
    Ok(Some(read_range_from_file_with_cancellation(
        file,
        &source_identity.canonical_path,
        source_identity.byte_length,
        SourceRange { offset, length },
        cancellation,
    )?))
}

fn discover_windowed_jpeg_candidates(
    source_identity: &SourceIdentity,
    cancellation: &AtomicBool,
) -> Result<Vec<RecoveryCandidate>, WorkflowError> {
    discover_windowed_jpeg_candidates_after_window(source_identity, cancellation, || {})
}

fn discover_windowed_jpeg_candidates_after_window<F>(
    source_identity: &SourceIdentity,
    cancellation: &AtomicBool,
    mut after_primary_window: F,
) -> Result<Vec<RecoveryCandidate>, WorkflowError>
where
    F: FnMut(),
{
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }

    let path = &source_identity.canonical_path;
    let mut file = File::open(path).map_err(|source| WorkflowError::ReadImage {
        path: path.clone(),
        source,
    })?;
    let mut candidates = Vec::new();
    let mut primary_start = 0_u64;
    let mut suppression_end = 0_u64;

    while primary_start < source_identity.byte_length {
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled.into());
        }
        let remaining = source_identity.byte_length - primary_start;
        let primary_length = remaining.min(JPEG_WINDOW_PRIMARY_LENGTH);
        let readable_length = primary_length
            .checked_add(JPEG_SIGNATURE_OVERLAP)
            .map(|length| length.min(remaining))
            .ok_or(CoreError::RangeOverflow {
                offset: primary_start,
                length: primary_length,
            })?;
        let window = read_range_from_file_with_cancellation(
            &mut file,
            path,
            source_identity.byte_length,
            SourceRange {
                offset: primary_start,
                length: readable_length,
            },
            cancellation,
        )?;
        let primary_length =
            usize::try_from(primary_length).map_err(|_| CoreError::RangeAllocation {
                length: primary_length,
            })?;

        for relative_start in 0..primary_length {
            let signature_end =
                relative_start
                    .checked_add(JPEG_SOI.len())
                    .ok_or(CoreError::RangeOverflow {
                        offset: primary_start,
                        length: JPEG_SOI.len() as u64,
                    })?;
            if window.get(relative_start..signature_end) != Some(&JPEG_SOI) {
                continue;
            }
            let source_offset = primary_start.checked_add(relative_start as u64).ok_or(
                CoreError::RangeOverflow {
                    offset: primary_start,
                    length: relative_start as u64,
                },
            )?;
            if source_offset < suppression_end {
                continue;
            }
            let Some(byte_length) = parse_windowed_jpeg_length(
                &mut file,
                source_identity,
                source_offset,
                cancellation,
            )?
            else {
                continue;
            };
            suppression_end =
                source_offset
                    .checked_add(byte_length)
                    .ok_or(CoreError::RangeOverflow {
                        offset: source_offset,
                        length: byte_length,
                    })?;
            let index = candidates.len();
            candidates.push(with_stable_candidate_id(jpeg_candidate(
                format!("jpeg-carve-{index:04}"),
                JpegCarvedCandidate {
                    evidence_name: format!("carved-jpeg-{index:04}.jpg"),
                    source_offset,
                    byte_length,
                },
            )));
        }

        primary_start =
            primary_start
                .checked_add(primary_length as u64)
                .ok_or(CoreError::RangeOverflow {
                    offset: primary_start,
                    length: primary_length as u64,
                })?;
        after_primary_window();
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled.into());
        }
    }

    Ok(candidates)
}

fn discover_windowed_gif_candidates(
    source_identity: &SourceIdentity,
    cancellation: &AtomicBool,
) -> Result<Vec<RecoveryCandidate>, WorkflowError> {
    discover_windowed_gif_candidates_after_window(source_identity, cancellation, || {})
}

fn discover_windowed_gif_candidates_after_window<F>(
    source_identity: &SourceIdentity,
    cancellation: &AtomicBool,
    mut after_primary_window: F,
) -> Result<Vec<RecoveryCandidate>, WorkflowError>
where
    F: FnMut(),
{
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }

    let path = &source_identity.canonical_path;
    let mut file = File::open(path).map_err(|source| WorkflowError::ReadImage {
        path: path.clone(),
        source,
    })?;
    let mut candidates = Vec::new();
    let mut primary_start = 0_u64;
    let mut suppression_end = 0_u64;

    while primary_start < source_identity.byte_length {
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled.into());
        }
        let remaining = source_identity.byte_length - primary_start;
        let primary_length = remaining.min(GIF_WINDOW_PRIMARY_LENGTH);
        let readable_length = primary_length
            .checked_add(GIF_SIGNATURE_OVERLAP)
            .map(|length| length.min(remaining))
            .ok_or(CoreError::RangeOverflow {
                offset: primary_start,
                length: primary_length,
            })?;
        let window = read_range_from_file_with_cancellation(
            &mut file,
            path,
            source_identity.byte_length,
            SourceRange {
                offset: primary_start,
                length: readable_length,
            },
            cancellation,
        )?;
        let primary_length =
            usize::try_from(primary_length).map_err(|_| CoreError::RangeAllocation {
                length: primary_length,
            })?;

        for relative_start in 0..primary_length {
            let signature_end = relative_start.checked_add(GIF87A_HEADER.len()).ok_or(
                CoreError::RangeOverflow {
                    offset: primary_start,
                    length: GIF87A_HEADER.len() as u64,
                },
            )?;
            let is_gif_header = window.get(relative_start..signature_end) == Some(&GIF87A_HEADER)
                || window.get(relative_start..signature_end) == Some(&GIF89A_HEADER);
            if !is_gif_header {
                continue;
            }
            let source_offset = primary_start.checked_add(relative_start as u64).ok_or(
                CoreError::RangeOverflow {
                    offset: primary_start,
                    length: relative_start as u64,
                },
            )?;
            if source_offset < suppression_end {
                continue;
            }
            let Some(byte_length) =
                parse_windowed_gif_length(&mut file, source_identity, source_offset, cancellation)?
            else {
                continue;
            };
            suppression_end =
                source_offset
                    .checked_add(byte_length)
                    .ok_or(CoreError::RangeOverflow {
                        offset: source_offset,
                        length: byte_length,
                    })?;
            let index = candidates.len();
            candidates.push(with_stable_candidate_id(gif_candidate(
                format!("gif-carve-{index:04}"),
                GifCarvedCandidate {
                    evidence_name: format!("carved-gif-{index:04}.gif"),
                    source_offset,
                    byte_length,
                },
            )));
        }

        primary_start =
            primary_start
                .checked_add(primary_length as u64)
                .ok_or(CoreError::RangeOverflow {
                    offset: primary_start,
                    length: primary_length as u64,
                })?;
        after_primary_window();
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled.into());
        }
    }

    Ok(candidates)
}

fn windowed_gif_candidate_limit(source_offset: u64, source_length: u64) -> u64 {
    source_offset
        .saturating_add(GIF_MAX_CARVE_LENGTH)
        .min(source_length)
}

fn parse_windowed_gif_length(
    file: &mut File,
    source_identity: &SourceIdentity,
    source_offset: u64,
    cancellation: &AtomicBool,
) -> Result<Option<u64>, WorkflowError> {
    let candidate_limit = windowed_gif_candidate_limit(source_offset, source_identity.byte_length);
    let mut reader = WindowedGifReader::new(
        file,
        source_identity,
        source_offset,
        candidate_limit,
        cancellation,
    );

    let mut header = [0_u8; 6];
    for byte in &mut header {
        let Some(value) = reader.read_byte()? else {
            return Ok(None);
        };
        *byte = value;
    }
    if header != GIF87A_HEADER && header != GIF89A_HEADER {
        return Ok(None);
    }

    for _ in 0..4 {
        if reader.read_byte()?.is_none() {
            return Ok(None);
        }
    }
    let Some(packed_fields) = reader.read_byte()? else {
        return Ok(None);
    };
    for _ in 0..2 {
        if reader.read_byte()?.is_none() {
            return Ok(None);
        }
    }
    if packed_fields & 0x80 != 0
        && !reader.skip_bytes(windowed_gif_color_table_length(packed_fields)?)?
    {
        return Ok(None);
    }

    let mut saw_image = false;
    while reader.offset() < candidate_limit {
        let Some(block_introducer) = reader.read_byte()? else {
            return Ok(None);
        };
        match block_introducer {
            GIF_TRAILER if saw_image => return Ok(Some(reader.offset() - source_offset)),
            0x21 => {
                if !parse_windowed_gif_extension(&mut reader)? {
                    return Ok(None);
                }
            }
            0x2c => {
                saw_image = true;
                if !parse_windowed_gif_image(&mut reader)? {
                    return Ok(None);
                }
            }
            _ => return Ok(None),
        }
    }

    Ok(None)
}

fn windowed_gif_color_table_length(packed_fields: u8) -> Result<u64, WorkflowError> {
    let entries = 1_u64
        .checked_shl(u32::from((packed_fields & 0x07) + 1))
        .ok_or(CoreError::RangeOverflow {
            offset: u64::from(packed_fields),
            length: 1,
        })?;
    entries.checked_mul(3).ok_or(
        CoreError::RangeOverflow {
            offset: entries,
            length: 3,
        }
        .into(),
    )
}

fn parse_windowed_gif_extension(reader: &mut WindowedGifReader<'_>) -> Result<bool, WorkflowError> {
    if reader.read_byte()?.is_none() {
        return Ok(false);
    }
    let Some(block_length) = reader.read_byte()? else {
        return Ok(false);
    };
    if !reader.skip_bytes(u64::from(block_length))? {
        return Ok(false);
    }
    parse_windowed_gif_sub_blocks(reader)
}

fn parse_windowed_gif_image(reader: &mut WindowedGifReader<'_>) -> Result<bool, WorkflowError> {
    for _ in 0..8 {
        if reader.read_byte()?.is_none() {
            return Ok(false);
        }
    }
    let Some(packed_fields) = reader.read_byte()? else {
        return Ok(false);
    };
    if packed_fields & 0x80 != 0
        && !reader.skip_bytes(windowed_gif_color_table_length(packed_fields)?)?
    {
        return Ok(false);
    }
    let Some(lzw_minimum_code_size) = reader.read_byte()? else {
        return Ok(false);
    };
    if !(2..=8).contains(&lzw_minimum_code_size) {
        return Ok(false);
    }
    parse_windowed_gif_sub_blocks(reader)
}

fn parse_windowed_gif_sub_blocks(
    reader: &mut WindowedGifReader<'_>,
) -> Result<bool, WorkflowError> {
    loop {
        let Some(length) = reader.read_byte()? else {
            return Ok(false);
        };
        if length == 0 {
            return Ok(true);
        }
        if !reader.skip_bytes(u64::from(length))? {
            return Ok(false);
        }
    }
}

struct WindowedGifReader<'a> {
    file: &'a mut File,
    source_identity: &'a SourceIdentity,
    cancellation: &'a AtomicBool,
    offset: u64,
    limit: u64,
    buffer_start: u64,
    buffer: Vec<u8>,
}

impl<'a> WindowedGifReader<'a> {
    fn new(
        file: &'a mut File,
        source_identity: &'a SourceIdentity,
        offset: u64,
        limit: u64,
        cancellation: &'a AtomicBool,
    ) -> Self {
        Self {
            file,
            source_identity,
            cancellation,
            offset,
            limit,
            buffer_start: limit,
            buffer: Vec::new(),
        }
    }

    fn offset(&self) -> u64 {
        self.offset
    }

    fn read_byte(&mut self) -> Result<Option<u8>, WorkflowError> {
        if self.offset >= self.limit {
            return Ok(None);
        }
        self.ensure_buffer_contains_offset()?;
        let relative_offset = usize::try_from(self.offset - self.buffer_start).map_err(|_| {
            CoreError::RangeAllocation {
                length: self.offset - self.buffer_start,
            }
        })?;
        let byte = *self
            .buffer
            .get(relative_offset)
            .ok_or(CoreError::RangeOutOfBounds {
                offset: self.offset,
                length: 1,
                source_length: self.limit,
            })?;
        self.offset = self.offset.checked_add(1).ok_or(CoreError::RangeOverflow {
            offset: self.offset,
            length: 1,
        })?;
        Ok(Some(byte))
    }

    fn skip_bytes(&mut self, length: u64) -> Result<bool, WorkflowError> {
        let Some(next_offset) = self.offset.checked_add(length) else {
            return Ok(false);
        };
        if next_offset > self.limit {
            return Ok(false);
        }
        self.offset = next_offset;
        Ok(true)
    }

    fn ensure_buffer_contains_offset(&mut self) -> Result<(), WorkflowError> {
        let buffer_end = self
            .buffer_start
            .checked_add(self.buffer.len() as u64)
            .ok_or(CoreError::RangeOverflow {
                offset: self.buffer_start,
                length: self.buffer.len() as u64,
            })?;
        if self.offset >= self.buffer_start && self.offset < buffer_end {
            return Ok(());
        }

        let length = (self.limit - self.offset).min(GIF_WINDOW_PRIMARY_LENGTH);
        self.buffer = read_range_from_file_with_cancellation(
            self.file,
            &self.source_identity.canonical_path,
            self.source_identity.byte_length,
            SourceRange {
                offset: self.offset,
                length,
            },
            self.cancellation,
        )?;
        self.buffer_start = self.offset;
        Ok(())
    }
}

fn windowed_jpeg_candidate_limit(source_offset: u64, source_length: u64) -> u64 {
    source_offset
        .saturating_add(JPEG_MAX_CARVE_LENGTH)
        .min(source_length)
}

fn parse_windowed_jpeg_length(
    file: &mut File,
    source_identity: &SourceIdentity,
    source_offset: u64,
    cancellation: &AtomicBool,
) -> Result<Option<u64>, WorkflowError> {
    let candidate_limit = windowed_jpeg_candidate_limit(source_offset, source_identity.byte_length);
    let mut reader = WindowedJpegReader::new(
        file,
        source_identity,
        source_offset,
        candidate_limit,
        cancellation,
    );

    if reader.read_byte()? != Some(JPEG_SOI[0]) || reader.read_byte()? != Some(JPEG_SOI[1]) {
        return Ok(None);
    }

    let mut saw_frame = false;
    while reader.offset() < candidate_limit {
        if reader.read_byte()? != Some(0xff) {
            return Ok(None);
        }
        let marker = loop {
            let Some(byte) = reader.read_byte()? else {
                return Ok(None);
            };
            if byte != 0xff {
                break byte;
            }
        };

        if marker == 0xd9 {
            return Ok(None);
        }
        if marker == 0xd8 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }

        let segment_length_offset = reader.offset();
        let Some(segment_length) = reader.read_be_u16()? else {
            return Ok(None);
        };
        let segment_length = u64::from(segment_length);
        if segment_length < 2 {
            return Ok(None);
        }
        let segment_end =
            segment_length_offset
                .checked_add(segment_length)
                .ok_or(CoreError::RangeOverflow {
                    offset: segment_length_offset,
                    length: segment_length,
                })?;
        if segment_end > candidate_limit {
            return Ok(None);
        }

        if is_windowed_jpeg_frame_marker(marker) {
            saw_frame = true;
        }
        reader.advance_to(segment_end)?;
        if marker == 0xda {
            if !saw_frame {
                return Ok(None);
            }
            return find_windowed_jpeg_end_of_image(&mut reader, source_offset);
        }
    }

    Ok(None)
}

fn is_windowed_jpeg_frame_marker(marker: u8) -> bool {
    matches!(
        marker,
        0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf
    )
}

fn find_windowed_jpeg_end_of_image(
    reader: &mut WindowedJpegReader<'_>,
    source_offset: u64,
) -> Result<Option<u64>, WorkflowError> {
    while reader.offset() < reader.limit().saturating_sub(1) {
        let Some(first) = reader.read_byte()? else {
            return Ok(None);
        };
        if first != 0xff {
            continue;
        }
        let Some(second) = reader.read_byte()? else {
            return Ok(None);
        };
        match second {
            0x00 | 0xd0..=0xd7 => {}
            0xd9 => return Ok(Some(reader.offset() - source_offset)),
            _ => return Ok(None),
        }
    }

    Ok(None)
}

struct WindowedJpegReader<'a> {
    file: &'a mut File,
    source_identity: &'a SourceIdentity,
    cancellation: &'a AtomicBool,
    offset: u64,
    limit: u64,
    buffer_start: u64,
    buffer: Vec<u8>,
}

impl<'a> WindowedJpegReader<'a> {
    fn new(
        file: &'a mut File,
        source_identity: &'a SourceIdentity,
        offset: u64,
        limit: u64,
        cancellation: &'a AtomicBool,
    ) -> Self {
        Self {
            file,
            source_identity,
            cancellation,
            offset,
            limit,
            buffer_start: limit,
            buffer: Vec::new(),
        }
    }

    fn offset(&self) -> u64 {
        self.offset
    }

    fn limit(&self) -> u64 {
        self.limit
    }

    fn advance_to(&mut self, offset: u64) -> Result<(), WorkflowError> {
        if offset > self.limit {
            return Err(CoreError::RangeOutOfBounds {
                offset,
                length: 0,
                source_length: self.limit,
            }
            .into());
        }
        self.offset = offset;
        Ok(())
    }

    fn read_be_u16(&mut self) -> Result<Option<u16>, WorkflowError> {
        let Some(high) = self.read_byte()? else {
            return Ok(None);
        };
        let Some(low) = self.read_byte()? else {
            return Ok(None);
        };
        Ok(Some(u16::from_be_bytes([high, low])))
    }

    fn read_byte(&mut self) -> Result<Option<u8>, WorkflowError> {
        if self.offset >= self.limit {
            return Ok(None);
        }
        self.ensure_buffer_contains_offset()?;
        let relative_offset = usize::try_from(self.offset - self.buffer_start).map_err(|_| {
            CoreError::RangeAllocation {
                length: self.offset - self.buffer_start,
            }
        })?;
        let byte = *self
            .buffer
            .get(relative_offset)
            .ok_or(CoreError::RangeOutOfBounds {
                offset: self.offset,
                length: 1,
                source_length: self.limit,
            })?;
        self.offset = self.offset.checked_add(1).ok_or(CoreError::RangeOverflow {
            offset: self.offset,
            length: 1,
        })?;
        Ok(Some(byte))
    }

    fn ensure_buffer_contains_offset(&mut self) -> Result<(), WorkflowError> {
        let buffer_end = self
            .buffer_start
            .checked_add(self.buffer.len() as u64)
            .ok_or(CoreError::RangeOverflow {
                offset: self.buffer_start,
                length: self.buffer.len() as u64,
            })?;
        if self.offset >= self.buffer_start && self.offset < buffer_end {
            return Ok(());
        }

        let length = (self.limit - self.offset).min(JPEG_WINDOW_PRIMARY_LENGTH);
        self.buffer = read_range_from_file_with_cancellation(
            self.file,
            &self.source_identity.canonical_path,
            self.source_identity.byte_length,
            SourceRange {
                offset: self.offset,
                length,
            },
            self.cancellation,
        )?;
        self.buffer_start = self.offset;
        Ok(())
    }
}

pub fn recover_candidate(
    image: &[u8],
    candidate_id: &str,
) -> Result<RecoveredCandidate, WorkflowError> {
    if candidate_id.starts_with(STABLE_CANDIDATE_ID_PREFIX) {
        let legacy_candidate = discover_candidates_legacy(image)
            .into_iter()
            .find(|candidate| stable_candidate_id(candidate) == candidate_id)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let legacy_id = legacy_candidate.id.clone();
        let expected_candidate = with_stable_candidate_id(legacy_candidate);
        let mut recovered = recover_legacy_candidate(image, &legacy_id)?;
        recovered.candidate.id = expected_candidate.id.clone();
        if recovered.candidate != expected_candidate {
            return Err(WorkflowError::CandidateUnavailable(candidate_id.to_owned()));
        }
        return Ok(recovered);
    }

    recover_legacy_candidate(image, candidate_id)
}

fn recover_legacy_candidate(
    image: &[u8],
    candidate_id: &str,
) -> Result<RecoveredCandidate, WorkflowError> {
    if let Some(index) = parse_candidate_index(candidate_id, "fat12-root-") {
        let volume = Fat12Volume::parse(image)?;
        let file = volume
            .find_deleted_root_files()
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let source_offset = volume.source_offset_for_candidate(&file)?;
        let bytes = volume.read_deleted_file(&file)?;
        return Ok(RecoveredCandidate {
            candidate: fat_candidate(
                candidate_id.to_owned(),
                &file,
                source_offset,
                RecoveryMethod::Fat12DeletedRootMetadata,
            ),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "fat16-root-") {
        let volume = Fat16Volume::parse(image)?;
        let file = volume
            .find_deleted_root_files()
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let source_offset = volume.source_offset_for_candidate(&file)?;
        let bytes = volume.read_deleted_file(&file)?;
        return Ok(RecoveredCandidate {
            candidate: fat_candidate(
                candidate_id.to_owned(),
                &file,
                source_offset,
                RecoveryMethod::Fat16DeletedRootMetadata,
            ),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "exfat-root-") {
        let volume = ExfatVolume::parse(image)?;
        let file = volume
            .find_deleted_root_files()
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let source_offset = volume.source_offset_for_candidate(&file)?;
        let bytes = volume.read_deleted_file(&file)?;
        return Ok(RecoveredCandidate {
            candidate: exfat_candidate(candidate_id.to_owned(), &file, source_offset),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "ntfs-resident-") {
        let volume = NtfsVolume::parse(image)?;
        let file = volume
            .find_deleted_resident_files()
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let source_offset = volume.source_offset_for_candidate(&file)?;
        let bytes = volume.read_deleted_file(&file)?;
        return Ok(RecoveredCandidate {
            candidate: ntfs_candidate(candidate_id.to_owned(), &file, source_offset),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "ntfs-contiguous-") {
        let volume = NtfsVolume::parse(image)?;
        let file = volume
            .find_deleted_contiguous_files()
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let source_offset = volume.source_offset_for_contiguous_candidate(&file)?;
        let bytes = volume.read_deleted_contiguous_file(&file)?;
        return Ok(RecoveredCandidate {
            candidate: ntfs_contiguous_candidate(candidate_id.to_owned(), &file, source_offset),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "png-carve-") {
        let file = carve_pngs(image)
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let bytes = extract_png(image, &file)?;
        return Ok(RecoveredCandidate {
            candidate: png_candidate(candidate_id.to_owned(), file),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "jpeg-carve-") {
        let file = carve_jpegs(image)
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let bytes = extract_jpeg(image, &file)?;
        return Ok(RecoveredCandidate {
            candidate: jpeg_candidate(candidate_id.to_owned(), file),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "gif-carve-") {
        let file = carve_gifs(image)
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let bytes = extract_gif(image, &file)?;
        return Ok(RecoveredCandidate {
            candidate: gif_candidate(candidate_id.to_owned(), file),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "avi-carve-") {
        let file = carve_avis(image)
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let bytes = extract_avi(image, &file)?;
        return Ok(RecoveredCandidate {
            candidate: avi_candidate(candidate_id.to_owned(), file),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "mp4-carve-") {
        let file = carve_mp4s(image)
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let bytes = extract_mp4(image, &file)?;
        return Ok(RecoveredCandidate {
            candidate: mp4_candidate(candidate_id.to_owned(), file),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "pdf-carve-") {
        let file = carve_pdfs(image)
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let bytes = extract_pdf(image, &file)?;
        return Ok(RecoveredCandidate {
            candidate: pdf_candidate(candidate_id.to_owned(), file),
            bytes,
        });
    }

    if let Some(index) = parse_candidate_index(candidate_id, "zip-carve-") {
        let file = carve_zip_archives(image)
            .into_iter()
            .nth(index)
            .ok_or_else(|| WorkflowError::CandidateUnavailable(candidate_id.to_owned()))?;
        let bytes = extract_zip(image, &file)?;
        return Ok(RecoveredCandidate {
            candidate: zip_candidate(candidate_id.to_owned(), file),
            bytes,
        });
    }

    Err(WorkflowError::UnsupportedCandidate(candidate_id.to_owned()))
}

const STABLE_CANDIDATE_ID_PREFIX: &str = "efc1-";

fn with_stable_candidate_id(mut candidate: RecoveryCandidate) -> RecoveryCandidate {
    candidate.id = stable_candidate_id(&candidate);
    candidate
}

fn stable_candidate_id(candidate: &RecoveryCandidate) -> String {
    let mut hasher = Hasher::new();
    hasher.update(b"evidenceforge-candidate-identity-v1\0");
    update_candidate_identity_component(
        &mut hasher,
        recovery_method_name(candidate.method).as_bytes(),
    );
    update_candidate_identity_component(
        &mut hasher,
        validation_state_name(candidate.validation).as_bytes(),
    );
    update_candidate_identity_component(&mut hasher, candidate.file_type.as_bytes());
    update_candidate_identity_component(&mut hasher, candidate.evidence_name.as_bytes());
    hasher.update(&candidate.source_offset.to_le_bytes());
    hasher.update(&candidate.byte_length.to_le_bytes());
    match candidate.original_path.as_deref() {
        Some(path) => {
            hasher.update(&[1]);
            update_candidate_identity_component(&mut hasher, path.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    format!(
        "{STABLE_CANDIDATE_ID_PREFIX}{}-{:016x}-{:016x}-{}",
        recovery_method_name(candidate.method),
        candidate.source_offset,
        candidate.byte_length,
        hasher.finalize().to_hex(),
    )
}

fn update_candidate_identity_component(hasher: &mut Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn validation_state_name(validation: CandidateValidation) -> &'static str {
    match validation {
        CandidateValidation::MetadataVerified => "metadata_verified",
        CandidateValidation::ContentValidated => "content_validated",
        CandidateValidation::RecoveredUnvalidated => "recovered_unvalidated",
        CandidateValidation::PartialOrErrorAffected => "partial_or_error_affected",
        CandidateValidation::Unavailable => "unavailable",
    }
}

pub fn safe_export_name(candidate: &RecoveryCandidate) -> PathBuf {
    let extension =
        file_extension(&candidate.evidence_name).unwrap_or_else(|| candidate.file_type.clone());
    PathBuf::from(format!("{}.{extension}", candidate.id))
}

pub fn recovery_method_name(method: RecoveryMethod) -> &'static str {
    match method {
        RecoveryMethod::Fat12DeletedRootMetadata => "fat12_deleted_root_metadata",
        RecoveryMethod::Fat16DeletedRootMetadata => "fat16_deleted_root_metadata",
        RecoveryMethod::ExfatDeletedContiguousRootMetadata => {
            "exfat_deleted_contiguous_root_metadata"
        }
        RecoveryMethod::NtfsDeletedResidentRecord => "ntfs_deleted_resident_record",
        RecoveryMethod::NtfsDeletedContiguousNonresident => "ntfs_deleted_contiguous_nonresident",
        RecoveryMethod::SignatureCarvingPng => "signature_carving_png",
        RecoveryMethod::SignatureCarvingJpeg => "signature_carving_jpeg",
        RecoveryMethod::SignatureCarvingGif => "signature_carving_gif",
        RecoveryMethod::SignatureCarvingAvi => "signature_carving_avi",
        RecoveryMethod::SignatureCarvingMp4 => "signature_carving_mp4",
        RecoveryMethod::SignatureCarvingPdf => "signature_carving_pdf",
        RecoveryMethod::SignatureCarvingZipOffice => "signature_carving_zip_office",
    }
}

fn read_image(path: &Path) -> Result<Vec<u8>, WorkflowError> {
    read_image_with_cancellation(path, &AtomicBool::new(false))
}

fn read_image_with_cancellation(
    path: &Path,
    cancellation: &AtomicBool,
) -> Result<Vec<u8>, WorkflowError> {
    if cancellation.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled.into());
    }
    let file = File::open(path).map_err(|source| WorkflowError::ReadImage {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut bytes = Vec::new();
    let mut buffer = vec![0_u8; 1024 * 1024];

    loop {
        if cancellation.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled.into());
        }
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|source| WorkflowError::ReadImage {
                path: path.to_path_buf(),
                source,
            })?;
        if bytes_read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&buffer[..bytes_read]);
    }
}

fn fat_candidate(
    id: String,
    file: &DeletedRootFile,
    source_offset: u64,
    method: RecoveryMethod,
) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name.clone(),
        file_type: file_extension(&file.evidence_name).unwrap_or_else(|| "binary".to_owned()),
        source_offset,
        byte_length: u64::from(file.byte_length),
        method,
        validation: CandidateValidation::RecoveredUnvalidated,
        original_path: None,
    }
}

fn exfat_candidate(
    id: String,
    file: &DeletedExfatRootFile,
    source_offset: u64,
) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name.clone(),
        file_type: file_extension(&file.evidence_name).unwrap_or_else(|| "binary".to_owned()),
        source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::ExfatDeletedContiguousRootMetadata,
        validation: CandidateValidation::RecoveredUnvalidated,
        original_path: None,
    }
}

fn ntfs_candidate(
    id: String,
    file: &DeletedNtfsResidentFile,
    source_offset: u64,
) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name.clone(),
        file_type: file_extension(&file.evidence_name).unwrap_or_else(|| "binary".to_owned()),
        source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::NtfsDeletedResidentRecord,
        validation: CandidateValidation::RecoveredUnvalidated,
        original_path: None,
    }
}

fn ntfs_contiguous_candidate(
    id: String,
    file: &DeletedNtfsContiguousFile,
    source_offset: u64,
) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name.clone(),
        file_type: file_extension(&file.evidence_name).unwrap_or_else(|| "binary".to_owned()),
        source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::NtfsDeletedContiguousNonresident,
        validation: CandidateValidation::RecoveredUnvalidated,
        original_path: None,
    }
}

fn png_candidate(id: String, file: PngCarvedCandidate) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name,
        file_type: "png".to_owned(),
        source_offset: file.source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::SignatureCarvingPng,
        validation: CandidateValidation::ContentValidated,
        original_path: None,
    }
}

fn jpeg_candidate(id: String, file: JpegCarvedCandidate) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name,
        file_type: "jpg".to_owned(),
        source_offset: file.source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::SignatureCarvingJpeg,
        validation: CandidateValidation::ContentValidated,
        original_path: None,
    }
}

fn gif_candidate(id: String, file: GifCarvedCandidate) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name,
        file_type: "gif".to_owned(),
        source_offset: file.source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::SignatureCarvingGif,
        validation: CandidateValidation::ContentValidated,
        original_path: None,
    }
}

fn avi_candidate(id: String, file: AviCarvedCandidate) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name,
        file_type: "avi".to_owned(),
        source_offset: file.source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::SignatureCarvingAvi,
        validation: CandidateValidation::ContentValidated,
        original_path: None,
    }
}

fn mp4_candidate(id: String, file: Mp4CarvedCandidate) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name,
        file_type: file.file_type,
        source_offset: file.source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::SignatureCarvingMp4,
        validation: CandidateValidation::ContentValidated,
        original_path: None,
    }
}

fn pdf_candidate(id: String, file: PdfCarvedCandidate) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name,
        file_type: "pdf".to_owned(),
        source_offset: file.source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::SignatureCarvingPdf,
        validation: CandidateValidation::ContentValidated,
        original_path: None,
    }
}

fn zip_candidate(id: String, file: ZipCarvedCandidate) -> RecoveryCandidate {
    RecoveryCandidate {
        id,
        evidence_name: file.evidence_name,
        file_type: file.file_type,
        source_offset: file.source_offset,
        byte_length: file.byte_length,
        method: RecoveryMethod::SignatureCarvingZipOffice,
        validation: CandidateValidation::ContentValidated,
        original_path: None,
    }
}

fn parse_candidate_index(id: &str, prefix: &str) -> Option<usize> {
    id.strip_prefix(prefix)?.parse().ok()
}

fn file_extension(name: &str) -> Option<String> {
    let extension = name.rsplit_once('.')?.1;
    if extension.is_empty() || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return None;
    }
    Some(extension.to_ascii_lowercase())
}

fn validation_state(validation: CandidateValidation) -> ValidationState {
    validation
}

#[cfg(test)]
mod tests {
    use super::{
        discover_candidates, discover_candidates_legacy,
        discover_candidates_legacy_with_cancellation, discover_windowed_gif_candidates,
        discover_windowed_gif_candidates_after_window, discover_windowed_jpeg_candidates,
        discover_windowed_jpeg_candidates_after_window, discover_windowed_png_candidates,
        discover_windowed_png_candidates_after_window, read_session_candidate_range,
        recover_candidate, recover_candidate_from_image,
        recover_candidate_from_image_with_cancellation, scan_image, scan_image_with_cancellation,
        stable_candidate_id, windowed_gif_candidate_limit, windowed_jpeg_candidate_limit,
        SessionManifest, WorkflowError, GIF_MAX_CARVE_LENGTH, GIF_SIGNATURE_OVERLAP,
        GIF_WINDOW_PRIMARY_LENGTH, JPEG_MAX_CARVE_LENGTH, JPEG_SIGNATURE_OVERLAP,
        JPEG_WINDOW_PRIMARY_LENGTH, PNG_SIGNATURE_OVERLAP, PNG_WINDOW_PRIMARY_LENGTH,
    };
    use ef_core::{CoreError, ImageSource, RecoveryCandidate, RecoveryMethod};
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use uuid::Uuid;

    const FIXTURE: &[u8] = include_bytes!("../../../fixtures/fat12-deleted-file-v1/source.img");
    const EXPECTED_PNG: &[u8] =
        include_bytes!("../../../fixtures/fat12-deleted-file-v1/expected-carved.png");
    const JPEG_FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/fat16-jpeg-multimethod-v1/source.img");
    const EXPECTED_JPEG: &[u8] =
        include_bytes!("../../../fixtures/fat16-jpeg-multimethod-v1/expected-carved.jpg");

    fn temporary_source_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("disktrace-windowed-{name}-{}", Uuid::new_v4()))
    }

    fn write_windowed_source(name: &str, bytes: &[u8]) -> (PathBuf, ImageSource) {
        let path = temporary_source_path(name);
        fs::write(&path, bytes).expect("write deterministic windowed PNG source");
        let source =
            ImageSource::inspect(&path).expect("inspect deterministic windowed PNG source");
        (path, source)
    }

    fn png_candidates(candidates: Vec<RecoveryCandidate>) -> Vec<RecoveryCandidate> {
        candidates
            .into_iter()
            .filter(|candidate| candidate.method == RecoveryMethod::SignatureCarvingPng)
            .collect()
    }

    fn jpeg_candidates(candidates: Vec<RecoveryCandidate>) -> Vec<RecoveryCandidate> {
        candidates
            .into_iter()
            .filter(|candidate| candidate.method == RecoveryMethod::SignatureCarvingJpeg)
            .collect()
    }

    fn gif_candidates(candidates: Vec<RecoveryCandidate>) -> Vec<RecoveryCandidate> {
        candidates
            .into_iter()
            .filter(|candidate| candidate.method == RecoveryMethod::SignatureCarvingGif)
            .collect()
    }

    fn push_png_chunk(output: &mut Vec<u8>, chunk_type: &[u8; 4], payload: &[u8]) {
        output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        output.extend_from_slice(chunk_type);
        output.extend_from_slice(payload);
        output.extend_from_slice(&[0_u8; 4]);
    }

    fn valid_png_with_idat(payload: &[u8]) -> Vec<u8> {
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        push_png_chunk(&mut png, b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
        push_png_chunk(&mut png, b"IDAT", payload);
        push_png_chunk(&mut png, b"IEND", &[]);
        png
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("fixtures")
            .join(name)
            .join("source.img")
    }

    fn assert_stable_id(candidate: &RecoveryCandidate, prefix: &str) {
        assert!(
            candidate.id.starts_with(prefix),
            "unexpected candidate id: {}",
            candidate.id
        );
        assert_eq!(candidate.id.len(), prefix.len() + 64);
        assert!(candidate.id[prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn legacy_discovery_cancellation_keeps_candidate_parity_when_not_signalled() {
        let cancellation = AtomicBool::new(false);
        let mut completed_method_stages = 0;

        let cancellable =
            discover_candidates_legacy_with_cancellation(FIXTURE, &cancellation, || {
                completed_method_stages += 1;
            })
            .expect("a disabled cancellation flag must preserve legacy discovery");

        assert_eq!(cancellable, discover_candidates_legacy(FIXTURE));
        assert_eq!(completed_method_stages, 11);
    }

    #[test]
    fn legacy_discovery_cancels_after_a_completed_method_stage() {
        let cancellation = AtomicBool::new(false);
        let mut completed_method_stages = 0;

        let error = discover_candidates_legacy_with_cancellation(FIXTURE, &cancellation, || {
            completed_method_stages += 1;
            if completed_method_stages == 1 {
                cancellation.store(true, Ordering::Relaxed);
            }
        })
        .expect_err("cancellation after the first completed method stage must stop discovery");

        assert!(matches!(error, WorkflowError::Core(CoreError::Cancelled)));
        assert_eq!(completed_method_stages, 1);
    }

    #[test]
    fn windowed_jpeg_candidate_limit_matches_the_legacy_cap_semantics() {
        assert_eq!(
            windowed_jpeg_candidate_limit(16, JPEG_MAX_CARVE_LENGTH + 64),
            JPEG_MAX_CARVE_LENGTH + 16
        );
        assert_eq!(windowed_jpeg_candidate_limit(16, 32), 32);
        assert_eq!(
            windowed_jpeg_candidate_limit(u64::MAX - 8, u64::MAX),
            u64::MAX
        );
    }

    #[test]
    fn windowed_jpeg_discovery_matches_the_legacy_fixture_candidates() {
        let path = fixture_path("fat16-jpeg-multimethod-v1");
        let source = ImageSource::inspect(&path).expect("inspect JPEG fixture");
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_jpeg_candidates(&source.identity, &cancellation)
            .expect("discover JPEG candidates through windows");
        let legacy = jpeg_candidates(discover_candidates(JPEG_FIXTURE));

        assert_eq!(windowed, legacy);
    }

    #[test]
    fn windowed_jpeg_discovery_owns_a_signature_straddling_the_primary_boundary() {
        let start = usize::try_from(JPEG_WINDOW_PRIMARY_LENGTH - JPEG_SIGNATURE_OVERLAP)
            .expect("window boundary fits usize");
        let mut image = vec![0_u8; start + EXPECTED_JPEG.len() + 32];
        image[start..start + EXPECTED_JPEG.len()].copy_from_slice(EXPECTED_JPEG);
        let (path, source) = write_windowed_source("jpeg-signature-boundary", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_jpeg_candidates(&source.identity, &cancellation)
            .expect("discover boundary JPEG through windows");
        let legacy = jpeg_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 1);
        assert_eq!(windowed[0].source_offset, start as u64);
        assert_eq!(windowed[0].byte_length, EXPECTED_JPEG.len() as u64);
        fs::remove_file(path).expect("remove deterministic windowed JPEG source");
    }

    #[test]
    fn invalid_boundary_jpeg_does_not_hide_a_later_valid_candidate() {
        let malformed_start = usize::try_from(JPEG_WINDOW_PRIMARY_LENGTH - JPEG_SIGNATURE_OVERLAP)
            .expect("window boundary fits usize");
        let valid_start =
            usize::try_from(JPEG_WINDOW_PRIMARY_LENGTH + 96).expect("valid offset fits usize");
        let mut image = vec![0_u8; valid_start + EXPECTED_JPEG.len() + 32];
        image[malformed_start..malformed_start + 8]
            .copy_from_slice(&[0xff, 0xd8, 0xff, 0xda, 0, 2, 0xff, 0xd9]);
        image[valid_start..valid_start + EXPECTED_JPEG.len()].copy_from_slice(EXPECTED_JPEG);
        let (path, source) = write_windowed_source("invalid-jpeg-boundary", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_jpeg_candidates(&source.identity, &cancellation)
            .expect("discover later JPEG through windows");
        let legacy = jpeg_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 1);
        assert_eq!(windowed[0].source_offset, valid_start as u64);
        fs::remove_file(path).expect("remove deterministic windowed JPEG source");
    }

    #[test]
    fn windowed_jpeg_discovery_preserves_adjacent_candidate_ordering() {
        let first_start =
            usize::try_from(JPEG_WINDOW_PRIMARY_LENGTH - 16).expect("window boundary fits usize");
        let second_start = first_start + EXPECTED_JPEG.len() + 24;
        let mut image = vec![0_u8; second_start + EXPECTED_JPEG.len() + 32];
        image[first_start..first_start + EXPECTED_JPEG.len()].copy_from_slice(EXPECTED_JPEG);
        image[second_start..second_start + EXPECTED_JPEG.len()].copy_from_slice(EXPECTED_JPEG);
        let (path, source) = write_windowed_source("adjacent-windowed-jpegs", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_jpeg_candidates(&source.identity, &cancellation)
            .expect("discover adjacent JPEGs through windows");
        let legacy = jpeg_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 2);
        assert_eq!(windowed[0].source_offset, first_start as u64);
        assert_eq!(windowed[1].source_offset, second_start as u64);
        fs::remove_file(path).expect("remove deterministic windowed JPEG source");
    }

    #[test]
    fn windowed_jpeg_discovery_refuses_a_truncated_candidate_at_source_end() {
        let image = [0xff, 0xd8, 0xff, 0xda, 0, 2];
        let (path, source) = write_windowed_source("truncated-windowed-jpeg", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_jpeg_candidates(&source.identity, &cancellation)
            .expect("refuse truncated JPEG through windows");
        let legacy = jpeg_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert!(windowed.is_empty());
        fs::remove_file(path).expect("remove deterministic windowed JPEG source");
    }

    #[test]
    fn windowed_jpeg_discovery_cancels_after_a_completed_primary_window() {
        let image = vec![
            0_u8;
            usize::try_from(JPEG_WINDOW_PRIMARY_LENGTH + 16)
                .expect("window fits usize")
        ];
        let (path, source) = write_windowed_source("cancel-after-jpeg-window", &image);
        let cancellation = AtomicBool::new(false);

        let error =
            discover_windowed_jpeg_candidates_after_window(&source.identity, &cancellation, || {
                cancellation.store(true, Ordering::Relaxed)
            })
            .expect_err("cancellation after a completed primary window must stop JPEG discovery");

        assert!(matches!(error, WorkflowError::Core(CoreError::Cancelled)));
        fs::remove_file(path).expect("remove deterministic windowed JPEG source");
    }

    #[test]
    fn windowed_gif_candidate_limit_matches_the_legacy_cap_semantics() {
        assert_eq!(
            windowed_gif_candidate_limit(16, GIF_MAX_CARVE_LENGTH + 64),
            GIF_MAX_CARVE_LENGTH + 16
        );
        assert_eq!(windowed_gif_candidate_limit(16, 32), 32);
        assert_eq!(
            windowed_gif_candidate_limit(u64::MAX - 8, u64::MAX),
            u64::MAX
        );
    }

    #[test]
    fn windowed_gif_discovery_matches_the_legacy_fixture_candidates() {
        let path = fixture_path("media-carving-multimethod-v1");
        let source = ImageSource::inspect(&path).expect("inspect GIF fixture");
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_gif_candidates(&source.identity, &cancellation)
            .expect("discover GIF candidates through windows");
        let legacy = gif_candidates(discover_candidates(MEDIA_FIXTURE));

        assert_eq!(windowed, legacy);
    }

    #[test]
    fn windowed_gif_discovery_owns_a_signature_straddling_the_primary_boundary() {
        let start = usize::try_from(GIF_WINDOW_PRIMARY_LENGTH - GIF_SIGNATURE_OVERLAP)
            .expect("window boundary fits usize");
        let mut image = vec![0_u8; start + EXPECTED_GIF.len() + 32];
        image[start..start + EXPECTED_GIF.len()].copy_from_slice(EXPECTED_GIF);
        let (path, source) = write_windowed_source("gif-signature-boundary", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_gif_candidates(&source.identity, &cancellation)
            .expect("discover boundary GIF through windows");
        let legacy = gif_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 1);
        assert_eq!(windowed[0].source_offset, start as u64);
        assert_eq!(windowed[0].byte_length, EXPECTED_GIF.len() as u64);
        fs::remove_file(path).expect("remove deterministic windowed GIF source");
    }

    #[test]
    fn invalid_boundary_gif_does_not_hide_a_later_valid_candidate() {
        let malformed_start = usize::try_from(GIF_WINDOW_PRIMARY_LENGTH - GIF_SIGNATURE_OVERLAP)
            .expect("window boundary fits usize");
        let valid_start =
            usize::try_from(GIF_WINDOW_PRIMARY_LENGTH + 96).expect("valid offset fits usize");
        let mut image = vec![0_u8; valid_start + EXPECTED_GIF.len() + 32];
        image[malformed_start..malformed_start + 13]
            .copy_from_slice(&[b'G', b'I', b'F', b'8', b'9', b'a', 1, 0, 1, 0, 0, 0, 0]);
        image[valid_start..valid_start + EXPECTED_GIF.len()].copy_from_slice(EXPECTED_GIF);
        let (path, source) = write_windowed_source("invalid-gif-boundary", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_gif_candidates(&source.identity, &cancellation)
            .expect("discover later GIF through windows");
        let legacy = gif_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 1);
        assert_eq!(windowed[0].source_offset, valid_start as u64);
        fs::remove_file(path).expect("remove deterministic windowed GIF source");
    }

    #[test]
    fn windowed_gif_discovery_preserves_adjacent_candidate_ordering() {
        let first_start =
            usize::try_from(GIF_WINDOW_PRIMARY_LENGTH - 16).expect("window boundary fits usize");
        let second_start = first_start + EXPECTED_GIF.len() + 24;
        let mut image = vec![0_u8; second_start + EXPECTED_GIF.len() + 32];
        image[first_start..first_start + EXPECTED_GIF.len()].copy_from_slice(EXPECTED_GIF);
        image[second_start..second_start + EXPECTED_GIF.len()].copy_from_slice(EXPECTED_GIF);
        let (path, source) = write_windowed_source("adjacent-windowed-gifs", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_gif_candidates(&source.identity, &cancellation)
            .expect("discover adjacent GIFs through windows");
        let legacy = gif_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 2);
        assert_eq!(windowed[0].source_offset, first_start as u64);
        assert_eq!(windowed[1].source_offset, second_start as u64);
        fs::remove_file(path).expect("remove deterministic windowed GIF source");
    }

    #[test]
    fn windowed_gif_discovery_refuses_a_truncated_candidate_at_source_end() {
        let image = [b'G', b'I', b'F', b'8', b'9', b'a', 1, 0, 1, 0, 0, 0, 0];
        let (path, source) = write_windowed_source("truncated-windowed-gif", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_gif_candidates(&source.identity, &cancellation)
            .expect("refuse truncated GIF through windows");
        let legacy = gif_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert!(windowed.is_empty());
        fs::remove_file(path).expect("remove deterministic windowed GIF source");
    }

    #[test]
    fn windowed_gif_discovery_cancels_after_a_completed_primary_window() {
        let image =
            vec![0_u8; usize::try_from(GIF_WINDOW_PRIMARY_LENGTH + 16).expect("window fits usize")];
        let (path, source) = write_windowed_source("cancel-after-gif-window", &image);
        let cancellation = AtomicBool::new(false);

        let error =
            discover_windowed_gif_candidates_after_window(&source.identity, &cancellation, || {
                cancellation.store(true, Ordering::Relaxed)
            })
            .expect_err("cancellation after a completed primary window must stop GIF discovery");

        assert!(matches!(error, WorkflowError::Core(CoreError::Cancelled)));
        fs::remove_file(path).expect("remove deterministic windowed GIF source");
    }

    #[test]
    fn windowed_png_discovery_matches_the_legacy_fixture_candidates() {
        let path = fixture_path("fat12-deleted-file-v1");
        let source = ImageSource::inspect(&path).expect("inspect PNG fixture");
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_png_candidates(&source.identity, &cancellation)
            .expect("discover PNG candidates through windows");
        let legacy = png_candidates(discover_candidates(FIXTURE));

        assert_eq!(windowed, legacy);
    }

    #[test]
    fn windowed_png_discovery_owns_a_signature_straddling_the_primary_boundary() {
        let start = usize::try_from(PNG_WINDOW_PRIMARY_LENGTH - PNG_SIGNATURE_OVERLAP)
            .expect("window boundary fits usize");
        let mut image = vec![0_u8; start + EXPECTED_PNG.len() + 32];
        image[start..start + EXPECTED_PNG.len()].copy_from_slice(EXPECTED_PNG);
        let (path, source) = write_windowed_source("signature-boundary", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_png_candidates(&source.identity, &cancellation)
            .expect("discover boundary PNG through windows");
        let legacy = png_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 1);
        assert_eq!(windowed[0].source_offset, start as u64);
        assert_eq!(windowed[0].byte_length, EXPECTED_PNG.len() as u64);
        fs::remove_file(path).expect("remove deterministic windowed source");
    }

    #[test]
    fn invalid_boundary_png_does_not_hide_a_later_valid_candidate() {
        let malformed_start = usize::try_from(PNG_WINDOW_PRIMARY_LENGTH - PNG_SIGNATURE_OVERLAP)
            .expect("window boundary fits usize");
        let valid_start =
            usize::try_from(PNG_WINDOW_PRIMARY_LENGTH + 96).expect("valid offset fits usize");
        let mut image = vec![0_u8; valid_start + EXPECTED_PNG.len() + 32];
        image[malformed_start..malformed_start + 8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        image[valid_start..valid_start + EXPECTED_PNG.len()].copy_from_slice(EXPECTED_PNG);
        let (path, source) = write_windowed_source("invalid-boundary", &image);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_png_candidates(&source.identity, &cancellation)
            .expect("discover later valid PNG through windows");
        let legacy = png_candidates(discover_candidates(&image));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 1);
        assert_eq!(windowed[0].source_offset, valid_start as u64);
        fs::remove_file(path).expect("remove deterministic windowed source");
    }

    #[test]
    fn windowed_png_discovery_suppresses_nested_signatures_like_the_legacy_carver() {
        let outer = valid_png_with_idat(EXPECTED_PNG);
        let (path, source) = write_windowed_source("nested-signature", &outer);
        let cancellation = AtomicBool::new(false);

        let windowed = discover_windowed_png_candidates(&source.identity, &cancellation)
            .expect("discover nested PNG control through windows");
        let legacy = png_candidates(discover_candidates(&outer));

        assert_eq!(windowed, legacy);
        assert_eq!(windowed.len(), 1);
        assert_eq!(windowed[0].byte_length, outer.len() as u64);
        fs::remove_file(path).expect("remove deterministic windowed source");
    }

    #[test]
    fn windowed_png_discovery_cancels_after_a_completed_primary_window() {
        let image =
            vec![0_u8; usize::try_from(PNG_WINDOW_PRIMARY_LENGTH + 16).expect("window fits usize")];
        let (path, source) = write_windowed_source("cancel-after-window", &image);
        let cancellation = AtomicBool::new(false);

        let error =
            discover_windowed_png_candidates_after_window(&source.identity, &cancellation, || {
                cancellation.store(true, Ordering::Relaxed)
            })
            .expect_err("cancellation after a completed primary window must stop discovery");

        assert!(matches!(error, WorkflowError::Core(CoreError::Cancelled)));
        fs::remove_file(path).expect("remove deterministic windowed source");
    }

    #[test]
    fn cancelled_scan_refuses_to_access_or_identify_a_source() {
        let cancellation = AtomicBool::new(true);
        let missing_source = PathBuf::from("/definitely-not-an-evidenceforge-source.img");

        let error = scan_image_with_cancellation(&missing_source, &cancellation)
            .expect_err("pre-signalled cancellation must stop before source access");

        assert!(matches!(error, WorkflowError::Core(CoreError::Cancelled)));
        assert!(cancellation.load(Ordering::Relaxed));
    }

    #[test]
    fn cancelled_preview_recovery_refuses_to_access_a_source() {
        let cancellation = AtomicBool::new(true);
        let missing_source = PathBuf::from("/definitely-not-an-evidenceforge-source.img");

        let error = recover_candidate_from_image_with_cancellation(
            &missing_source,
            "efc1-preview-candidate",
            &cancellation,
        )
        .expect_err("pre-signalled cancellation must stop before preview source access");

        assert!(matches!(error, WorkflowError::Core(CoreError::Cancelled)));
        assert!(cancellation.load(Ordering::Relaxed));
    }

    #[test]
    fn verified_session_ranges_match_full_recovery_for_every_fixture_candidate() {
        let fixtures = [
            "fat12-deleted-file-v1",
            "fat16-jpeg-multimethod-v1",
            "document-carving-multimethod-v1",
            "media-carving-multimethod-v1",
            "exfat-contiguous-deleted-v1",
            "ntfs-deleted-resident-v1",
            "ntfs-deleted-contiguous-v1",
        ];
        let cancellation = AtomicBool::new(false);

        for fixture in fixtures {
            let path = fixture_path(fixture);
            let scan = scan_image(&path).expect("scan fixture");
            let manifest = SessionManifest::new(scan.session, scan.candidates)
                .expect("create completed session manifest");
            for candidate in &manifest.candidates {
                let ranged = read_session_candidate_range(&manifest, &candidate.id, &cancellation)
                    .expect("read verified session candidate range");
                let full = recover_candidate_from_image(&path, &candidate.id)
                    .expect("recover candidate through compatibility path");

                assert_eq!(ranged.candidate, full.candidate, "fixture {fixture}");
                assert_eq!(ranged.bytes, full.bytes, "fixture {fixture}");
            }
        }
    }

    #[test]
    fn session_range_refuses_unknown_candidate_ids() {
        let path = fixture_path("fat12-deleted-file-v1");
        let scan = scan_image(&path).expect("scan fixture");
        let manifest =
            SessionManifest::new(scan.session, scan.candidates).expect("create session manifest");
        let cancellation = AtomicBool::new(false);

        assert!(matches!(
            read_session_candidate_range(&manifest, "efc1-missing", &cancellation),
            Err(WorkflowError::CandidateNotInSession(_))
        ));
    }

    #[test]
    fn discovers_metadata_and_carving_candidates_from_the_shared_fixture() {
        let candidates = discover_candidates(FIXTURE);

        assert_eq!(candidates.len(), 2);
        assert_stable_id(
            &candidates[0],
            "efc1-fat12_deleted_root_metadata-0000000000000600-000000000000000b-",
        );
        assert_stable_id(
            &candidates[1],
            "efc1-signature_carving_png-0000000000001000-0000000000000046-",
        );
    }

    #[test]
    fn stable_candidate_ids_are_order_independent_unique_and_fact_bound() {
        let candidates = discover_candidates(FIXTURE);
        let original_ids: Vec<_> = candidates.iter().map(stable_candidate_id).collect();
        let mut reordered = candidates.clone();
        reordered.reverse();
        let reordered_ids: Vec<_> = reordered.iter().map(stable_candidate_id).collect();
        let original_id_set: HashSet<_> = original_ids.iter().collect();
        let reordered_id_set: HashSet<_> = reordered_ids.iter().collect();

        assert_eq!(original_id_set, reordered_id_set);
        assert_eq!(original_id_set.len(), candidates.len());
        let mut changed = candidates[0].clone();
        changed.byte_length += 1;
        assert_ne!(stable_candidate_id(&changed), candidates[0].id);
    }

    #[test]
    fn legacy_index_addressed_ids_remain_recoverable() {
        let recovered = recover_candidate(FIXTURE, "fat12-root-0000")
            .expect("extract a legacy FAT12 candidate");

        assert_eq!(recovered.candidate.id, "fat12-root-0000");
        assert_eq!(recovered.bytes, b"recover me\n");
    }

    #[test]
    fn extracts_both_candidate_types_through_the_shared_workflow() {
        let candidates = discover_candidates(FIXTURE);
        let text = recover_candidate(FIXTURE, &candidates[0].id).expect("extract FAT candidate");
        let png = recover_candidate(FIXTURE, &candidates[1].id).expect("extract PNG candidate");

        assert_eq!(text.bytes, b"recover me\n");
        assert_eq!(png.bytes.len(), 70);
        assert_eq!(&png.bytes[..8], b"\x89PNG\r\n\x1a\n");
    }

    const FAT16_FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/fat16-jpeg-multimethod-v1/source.img");

    #[test]
    fn discovers_fat16_metadata_and_jpeg_carving_candidates() {
        let candidates = discover_candidates(FAT16_FIXTURE);

        assert_eq!(candidates.len(), 2);
        assert_stable_id(
            &candidates[0],
            "efc1-fat16_deleted_root_metadata-0000000000002800-0000000000000015-",
        );
        assert_stable_id(
            &candidates[1],
            "efc1-signature_carving_jpeg-0000000000002c00-0000000000000023-",
        );
    }

    #[test]
    fn extracts_fat16_and_jpeg_candidates_through_the_shared_workflow() {
        let candidates = discover_candidates(FAT16_FIXTURE);
        let text =
            recover_candidate(FAT16_FIXTURE, &candidates[0].id).expect("extract FAT16 candidate");
        let jpeg =
            recover_candidate(FAT16_FIXTURE, &candidates[1].id).expect("extract JPEG candidate");

        assert_eq!(text.bytes, b"fat16 recovered text\n");
        assert_eq!(&jpeg.bytes[..2], b"\xff\xd8");
        assert_eq!(&jpeg.bytes[jpeg.bytes.len() - 2..], b"\xff\xd9");
    }

    const DOCUMENT_FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/document-carving-multimethod-v1/source.img");
    const EXPECTED_PDF: &[u8] =
        include_bytes!("../../../fixtures/document-carving-multimethod-v1/expected-carved.pdf");
    const EXPECTED_DOCX: &[u8] =
        include_bytes!("../../../fixtures/document-carving-multimethod-v1/expected-carved.docx");

    #[test]
    fn discovers_pdf_and_docx_candidates_from_the_document_fixture() {
        let candidates = discover_candidates(DOCUMENT_FIXTURE);

        assert_eq!(candidates.len(), 2);
        assert_stable_id(
            &candidates[0],
            "efc1-signature_carving_pdf-0000000000000400-00000000000000df-",
        );
        assert_eq!(candidates[0].source_offset, 1024);
        assert_stable_id(
            &candidates[1],
            "efc1-signature_carving_zip_office-0000000000004000-00000000000002c0-",
        );
        assert_eq!(candidates[1].file_type, "docx");
        assert_eq!(candidates[1].source_offset, 16384);
    }

    #[test]
    fn extracts_pdf_and_docx_candidates_through_the_shared_workflow() {
        let candidates = discover_candidates(DOCUMENT_FIXTURE);
        let pdf = recover_candidate(DOCUMENT_FIXTURE, &candidates[0].id).expect("extract PDF");
        let docx = recover_candidate(DOCUMENT_FIXTURE, &candidates[1].id).expect("extract DOCX");

        assert_eq!(pdf.bytes, EXPECTED_PDF);
        assert_eq!(docx.bytes, EXPECTED_DOCX);
    }

    const MEDIA_FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/media-carving-multimethod-v1/source.img");
    const EXPECTED_GIF: &[u8] =
        include_bytes!("../../../fixtures/media-carving-multimethod-v1/expected-carved.gif");
    const EXPECTED_AVI: &[u8] =
        include_bytes!("../../../fixtures/media-carving-multimethod-v1/expected-carved.avi");
    const EXPECTED_MP4: &[u8] =
        include_bytes!("../../../fixtures/media-carving-multimethod-v1/expected-carved.mp4");

    #[test]
    fn discovers_gif_and_video_candidates_from_the_media_fixture() {
        let candidates = discover_candidates(MEDIA_FIXTURE);

        assert_eq!(candidates.len(), 3);
        assert_stable_id(
            &candidates[0],
            "efc1-signature_carving_gif-0000000000000400-0000000000000023-",
        );
        assert_eq!(candidates[0].source_offset, 1024);
        assert_eq!(candidates[0].method, RecoveryMethod::SignatureCarvingGif);
        assert_stable_id(
            &candidates[1],
            "efc1-signature_carving_avi-0000000000002000-0000000000000028-",
        );
        assert_eq!(candidates[1].source_offset, 8192);
        assert_eq!(candidates[1].method, RecoveryMethod::SignatureCarvingAvi);
        assert_stable_id(
            &candidates[2],
            "efc1-signature_carving_mp4-0000000000004000-0000000000000048-",
        );
        assert_eq!(candidates[2].source_offset, 16384);
        assert_eq!(candidates[2].method, RecoveryMethod::SignatureCarvingMp4);
    }

    #[test]
    fn extracts_gif_and_video_candidates_through_the_shared_workflow() {
        let candidates = discover_candidates(MEDIA_FIXTURE);
        let gif = recover_candidate(MEDIA_FIXTURE, &candidates[0].id).expect("extract GIF");
        let avi = recover_candidate(MEDIA_FIXTURE, &candidates[1].id).expect("extract AVI");
        let mp4 = recover_candidate(MEDIA_FIXTURE, &candidates[2].id).expect("extract MP4");

        assert_eq!(gif.bytes, EXPECTED_GIF);
        assert_eq!(avi.bytes, EXPECTED_AVI);
        assert_eq!(mp4.bytes, EXPECTED_MP4);
    }

    const EXFAT_FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/exfat-contiguous-deleted-v1/source.img");
    const EXFAT_EXPECTED: &[u8] =
        include_bytes!("../../../fixtures/exfat-contiguous-deleted-v1/expected-recovered.txt");

    #[test]
    fn discovers_a_validated_contiguous_exfat_deleted_candidate() {
        let candidates = discover_candidates(EXFAT_FIXTURE);

        assert_eq!(candidates.len(), 1);
        assert_stable_id(
            &candidates[0],
            "efc1-exfat_deleted_contiguous_root_metadata-0000000000005400-0000000000000010-",
        );
        assert_eq!(candidates[0].evidence_name, "recover.txt");
        assert_eq!(candidates[0].source_offset, 21504);
        assert_eq!(candidates[0].byte_length, 16);
    }

    #[test]
    fn extracts_a_contiguous_exfat_candidate_through_the_shared_workflow() {
        let candidates = discover_candidates(EXFAT_FIXTURE);
        let recovered =
            recover_candidate(EXFAT_FIXTURE, &candidates[0].id).expect("extract exFAT candidate");

        assert_eq!(recovered.bytes, EXFAT_EXPECTED);
    }

    const NTFS_FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/ntfs-deleted-resident-v1/source.img");
    const NTFS_EXPECTED: &[u8] =
        include_bytes!("../../../fixtures/ntfs-deleted-resident-v1/expected-recovered.txt");

    #[test]
    fn discovers_a_fixed_up_deleted_ntfs_resident_candidate() {
        let candidates = discover_candidates(NTFS_FIXTURE);

        assert_eq!(candidates.len(), 1);
        assert_stable_id(
            &candidates[0],
            "efc1-ntfs_deleted_resident_record-0000000000000cc0-000000000000000f-",
        );
        assert_eq!(candidates[0].evidence_name, "gone.txt");
        assert_eq!(candidates[0].source_offset, 3264);
        assert_eq!(candidates[0].byte_length, 15);
    }

    #[test]
    fn extracts_a_deleted_ntfs_resident_candidate_through_the_shared_workflow() {
        let candidates = discover_candidates(NTFS_FIXTURE);
        let recovered = recover_candidate(NTFS_FIXTURE, &candidates[0].id)
            .expect("extract NTFS resident candidate");

        assert_eq!(recovered.bytes, NTFS_EXPECTED);
    }

    const NTFS_CONTIGUOUS_FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/ntfs-deleted-contiguous-v1/source.img");
    const NTFS_CONTIGUOUS_EXPECTED: &[u8] =
        include_bytes!("../../../fixtures/ntfs-deleted-contiguous-v1/expected-recovered.txt");

    #[test]
    fn discovers_a_deleted_ntfs_contiguous_candidate_with_free_extent() {
        let candidates = discover_candidates(NTFS_CONTIGUOUS_FIXTURE);

        assert_eq!(candidates.len(), 1);
        assert_stable_id(
            &candidates[0],
            "efc1-ntfs_deleted_contiguous_nonresident-0000000000008000-0000000000000010-",
        );
        assert_eq!(candidates[0].evidence_name, "extent.txt");
        assert_eq!(candidates[0].source_offset, 32768);
        assert_eq!(candidates[0].byte_length, 16);
        assert_eq!(
            candidates[0].method,
            RecoveryMethod::NtfsDeletedContiguousNonresident
        );
    }

    #[test]
    fn extracts_a_deleted_ntfs_contiguous_candidate_through_the_shared_workflow() {
        let candidates = discover_candidates(NTFS_CONTIGUOUS_FIXTURE);
        let recovered = recover_candidate(NTFS_CONTIGUOUS_FIXTURE, &candidates[0].id)
            .expect("extract NTFS contiguous candidate");

        assert_eq!(recovered.bytes, NTFS_CONTIGUOUS_EXPECTED);
    }
}
