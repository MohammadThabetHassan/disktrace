use anyhow::{bail, Context, Result};
use ef_catalogue::{
    build_catalogue, present_candidate, CandidateCatalogue, CandidatePresentation, CatalogueQuery,
};
use ef_core::{
    CandidateValidation, ImageSource, RecoveryCandidate, RecoveryMethod, RecoverySession,
};
use ef_policy::approve_destination;
use ef_report::RecoveryReceipt;
use ef_workflow::{
    recover_candidate_from_image, recover_to_destination, scan_image, RecordedExportVerification,
    RecoveryExport, SessionManifest, SourceIntegrity,
};
use serde::Serialize;
use std::env;

#[derive(Debug, Serialize)]
struct RecoveryScanReport {
    session: RecoverySession,
    candidates: Vec<RecoveryCandidate>,
}

#[derive(Debug, Serialize)]
struct CatalogueReport {
    session: RecoverySession,
    catalogue: CandidateCatalogue,
    presentations: Vec<CandidatePresentation>,
}

#[derive(Debug, Serialize)]
struct RecoveryExportReport {
    output_path: String,
    receipt_path: String,
    receipt: RecoveryReceipt,
}

#[derive(Debug, Serialize)]
struct SessionStatusReport {
    manifest: SessionManifest,
    source_integrity: SourceIntegrity,
}

#[derive(Debug, Serialize)]
struct SessionAuditReport {
    source_integrity: SourceIntegrity,
    exports: Vec<RecordedExportVerification>,
}

fn main() -> Result<()> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "help".to_owned());

    match command.as_str() {
        "inspect" => inspect(arguments.collect()),
        "check-destination" => check_destination(arguments.collect()),
        "scan" => scan(arguments.collect()),
        "catalogue" => catalogue(arguments.collect()),
        "recover" => recover(arguments.collect()),
        "save-session" => save_session(arguments.collect()),
        "session-status" => session_status(arguments.collect()),
        "audit-session" => audit_session(arguments.collect()),
        "case-brief" => case_brief(arguments.collect()),
        "recover-session" => recover_session(arguments.collect()),
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn inspect(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 1 {
        bail!("inspect requires exactly one image path");
    }

    let scan = scan_image(&arguments[0]).context("inspect recovery image")?;
    print_json(&scan.session)
}

fn check_destination(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 2 {
        bail!("check-destination requires an image path and destination directory");
    }

    let source = ImageSource::inspect(&arguments[0]).context("inspect recovery image")?;
    let destination =
        approve_destination(&source, &arguments[1]).context("validate recovery destination")?;
    println!("{}", destination.canonical_path.display());
    Ok(())
}

fn scan(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 1 {
        bail!("scan requires exactly one image path");
    }

    let scan = scan_image(&arguments[0]).context("scan recovery image")?;
    let report = RecoveryScanReport {
        session: scan.session,
        candidates: scan.candidates,
    };
    print_json(&report)
}

fn catalogue(arguments: Vec<String>) -> Result<()> {
    let (image_path, query) = parse_catalogue_arguments(arguments)?;
    let scan = scan_image(&image_path).context("scan recovery image")?;
    let candidate_catalogue = build_catalogue(scan.candidates, &query);
    let presentations = candidate_catalogue
        .candidates
        .iter()
        .cloned()
        .map(|candidate| {
            let recovered_bytes = recover_candidate_from_image(&image_path, &candidate.id)
                .ok()
                .map(|recovered| recovered.bytes);
            present_candidate(candidate, recovered_bytes.as_deref())
        })
        .collect();
    let report = CatalogueReport {
        session: scan.session,
        catalogue: candidate_catalogue,
        presentations,
    };
    print_json(&report)
}

fn recover(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 3 {
        bail!("recover requires an image path, candidate id, and destination directory");
    }

    let export = recover_to_destination(&arguments[0], &arguments[1], &arguments[2])
        .context("recover selected candidate")?;
    print_json(&export_report(export))
}

fn save_session(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 2 {
        bail!("save-session requires an image path and manifest path");
    }

    let scan = scan_image(&arguments[0]).context("scan recovery image")?;
    let manifest = SessionManifest::new(scan.session, scan.candidates)
        .context("create recovery session manifest")?;
    manifest
        .save(&arguments[1])
        .context("save recovery session manifest")?;
    print_json(&manifest)
}

fn session_status(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 1 {
        bail!("session-status requires exactly one manifest path");
    }

    let manifest =
        SessionManifest::load(&arguments[0]).context("load recovery session manifest")?;
    let report = SessionStatusReport {
        source_integrity: manifest.verify_source(),
        manifest,
    };
    print_json(&report)
}

fn audit_session(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 1 {
        bail!("audit-session requires exactly one manifest path");
    }

    let manifest =
        SessionManifest::load(&arguments[0]).context("load recovery session manifest")?;
    let report = SessionAuditReport {
        source_integrity: manifest.verify_source(),
        exports: manifest.verify_recorded_exports(),
    };
    print_json(&report)
}

fn case_brief(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 2 {
        bail!("case-brief requires a manifest path and an output Markdown path");
    }

    let manifest =
        SessionManifest::load(&arguments[0]).context("load recovery session manifest")?;
    manifest
        .save_case_brief(&arguments[1])
        .context("save local case brief")?;
    println!("{}", arguments[1]);
    Ok(())
}

fn recover_session(arguments: Vec<String>) -> Result<()> {
    if arguments.len() != 3 {
        bail!("recover-session requires a manifest path, candidate id, and destination directory");
    }

    let mut manifest =
        SessionManifest::load(&arguments[0]).context("load recovery session manifest")?;
    let export = manifest
        .recover_to_destination(&arguments[1], &arguments[2])
        .context("recover selected candidate from saved session")?;
    manifest
        .save(&arguments[0])
        .context("record recovery export in session manifest")?;
    print_json(&export_report(export))
}

fn export_report(export: RecoveryExport) -> RecoveryExportReport {
    RecoveryExportReport {
        output_path: export.output_path.display().to_string(),
        receipt_path: export.receipt_path.display().to_string(),
        receipt: export.receipt,
    }
}

fn parse_catalogue_arguments(arguments: Vec<String>) -> Result<(String, CatalogueQuery)> {
    let image_path = arguments
        .first()
        .cloned()
        .context("catalogue requires an image path")?;
    let mut query = CatalogueQuery::default();
    let mut index = 1;

    while index < arguments.len() {
        let flag = &arguments[index];
        let value = arguments
            .get(index + 1)
            .with_context(|| format!("{flag} requires a value"))?;
        match flag.as_str() {
            "--search" => query.text = Some(value.clone()),
            "--method" => query.methods.push(parse_method_filter(value)?),
            "--validation" => query.validations.push(parse_validation_filter(value)?),
            _ => bail!("unsupported catalogue filter '{flag}'"),
        }
        index += 2;
    }

    Ok((image_path, query))
}

fn parse_method_filter(value: &str) -> Result<RecoveryMethod> {
    match value {
        "fat12" | "fat12_deleted_root_metadata" => Ok(RecoveryMethod::Fat12DeletedRootMetadata),
        "fat16" | "fat16_deleted_root_metadata" => Ok(RecoveryMethod::Fat16DeletedRootMetadata),
        "exfat" | "exfat_deleted_contiguous_root_metadata" => {
            Ok(RecoveryMethod::ExfatDeletedContiguousRootMetadata)
        }
        "ntfs" | "ntfs-resident" | "ntfs_deleted_resident_record" => {
            Ok(RecoveryMethod::NtfsDeletedResidentRecord)
        }
        "ntfs-contiguous" | "ntfs-nonresident" | "ntfs_deleted_contiguous_nonresident" => {
            Ok(RecoveryMethod::NtfsDeletedContiguousNonresident)
        }
        "png" | "signature_carving_png" => Ok(RecoveryMethod::SignatureCarvingPng),
        "jpeg" | "jpg" | "signature_carving_jpeg" => Ok(RecoveryMethod::SignatureCarvingJpeg),
        "gif" | "signature_carving_gif" => Ok(RecoveryMethod::SignatureCarvingGif),
        "avi" | "signature_carving_avi" => Ok(RecoveryMethod::SignatureCarvingAvi),
        "mp4" | "mov" | "m4v" | "signature_carving_mp4" => Ok(RecoveryMethod::SignatureCarvingMp4),
        "pdf" | "signature_carving_pdf" => Ok(RecoveryMethod::SignatureCarvingPdf),
        "zip" | "office" | "docx" | "xlsx" | "pptx" | "signature_carving_zip_office" => {
            Ok(RecoveryMethod::SignatureCarvingZipOffice)
        }
        _ => bail!("unsupported recovery method filter '{value}'"),
    }
}

fn parse_validation_filter(value: &str) -> Result<CandidateValidation> {
    match value {
        "metadata_verified" => Ok(CandidateValidation::MetadataVerified),
        "content_validated" => Ok(CandidateValidation::ContentValidated),
        "recovered_unvalidated" => Ok(CandidateValidation::RecoveredUnvalidated),
        "partial_or_error_affected" => Ok(CandidateValidation::PartialOrErrorAffected),
        "unavailable" => Ok(CandidateValidation::Unavailable),
        _ => bail!("unsupported validation filter '{value}'"),
    }
}

fn print_json(value: &impl Serialize) -> Result<()> {
    let json = serde_json::to_string_pretty(value).context("serialize JSON output")?;
    println!("{json}");
    Ok(())
}

fn print_usage() {
    eprintln!(
        "Usage:\n  evidenceforge inspect <image-path>\n  evidenceforge check-destination <image-path> <destination-directory>\n  evidenceforge scan <image-path>\n  evidenceforge catalogue <image-path> [--search <text>] [--method <fat12|fat16|exfat|ntfs|ntfs-contiguous|png|jpeg|gif|avi|mp4|mov|pdf|zip|office>] [--validation <state>]\n  evidenceforge recover <image-path> <candidate-id> <destination-directory>\n  evidenceforge save-session <image-path> <manifest-path>\n  evidenceforge session-status <manifest-path>\n  evidenceforge audit-session <manifest-path>\n  evidenceforge case-brief <manifest-path> <output-markdown-path>\n  evidenceforge recover-session <manifest-path> <candidate-id> <destination-directory>"
    );
}
