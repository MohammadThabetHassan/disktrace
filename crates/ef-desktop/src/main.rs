use ef_catalogue::{
    build_catalogue, present_candidate, CandidateCatalogue, CandidatePresentation, CatalogueQuery,
    PreviewKind,
};
use ef_core::{CandidateValidation, CoreError, RecoveryCandidate, RecoveryMethod};
use ef_workflow::{
    read_session_candidate_range, scan_image_with_cancellation, RecordedExportIntegrity,
    RecordedExportVerification, ScanResult, SessionManifest, SourceIntegrity, WorkflowError,
};
use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};
use std::thread;
use std::time::Duration;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([1120.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DiskTrace",
        options,
        Box::new(|creation_context| {
            configure_style(&creation_context.egui_ctx);
            Ok(Box::<EvidenceForgeApp>::default())
        }),
    )
}

#[derive(Default, PartialEq, Eq)]
enum MethodFilter {
    #[default]
    All,
    Fat12,
    Fat16,
    Exfat,
    Ntfs,
    NtfsContiguous,
    Png,
    Jpeg,
    Gif,
    Avi,
    Mp4,
    Pdf,
    ZipOffice,
}

#[derive(Default, PartialEq, Eq)]
enum ValidationFilter {
    #[default]
    All,
    Checked,
    Review,
}

enum NoticeTone {
    Information,
    Success,
    Warning,
    Error,
}

struct Notice {
    tone: NoticeTone,
    title: String,
    detail: String,
}

enum ScanWorkerEvent {
    Completed(Box<Result<ScanResult, String>>),
    Cancelled,
}

struct ScanWorker {
    generation: u64,
    cancellation: Arc<AtomicBool>,
    receiver: Receiver<ScanWorkerEvent>,
}

struct PreviewWorker {
    generation: u64,
    candidate_id: String,
    cancellation: Arc<AtomicBool>,
    receiver: Receiver<Result<Vec<u8>, String>>,
}

struct EvidenceForgeApp {
    image_path: String,
    destination_path: String,
    search: String,
    method_filter: MethodFilter,
    validation_filter: ValidationFilter,
    candidates: Vec<RecoveryCandidate>,
    catalogue: Option<CandidateCatalogue>,
    presentations: Vec<CandidatePresentation>,
    selected_id: Option<String>,
    source_detail: Option<String>,
    session_manifest: Option<SessionManifest>,
    session_manifest_path: Option<PathBuf>,
    source_integrity: Option<SourceIntegrity>,
    export_audit: Option<Vec<RecordedExportVerification>>,
    notice: Option<Notice>,
    scan_generation: u64,
    scan_worker: Option<ScanWorker>,
    preview_generation: u64,
    preview_worker: Option<PreviewWorker>,
    preview_error: Option<(String, String)>,
    show_shortcuts: bool,
    show_recovery_review: bool,
}

impl Default for EvidenceForgeApp {
    fn default() -> Self {
        Self {
            image_path: String::new(),
            destination_path: String::new(),
            search: String::new(),
            method_filter: MethodFilter::All,
            validation_filter: ValidationFilter::All,
            candidates: Vec::new(),
            catalogue: None,
            presentations: Vec::new(),
            selected_id: None,
            source_detail: None,
            session_manifest: None,
            session_manifest_path: None,
            source_integrity: None,
            export_audit: None,
            scan_generation: 0,
            scan_worker: None,
            preview_generation: 0,
            preview_worker: None,
            preview_error: None,
            show_shortcuts: false,
            show_recovery_review: false,
            notice: Some(Notice {
                tone: NoticeTone::Information,
                title: "Start with a copy when possible".to_owned(),
                detail: "DiskTrace reads the selected image and never writes to it. Choose a separate destination before saving recovered files.".to_owned(),
            }),
        }
    }
}

impl EvidenceForgeApp {
    fn load_demo_fixture(&mut self) {
        self.reset_workspace();
        self.image_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/fat12-deleted-file-v1/source.img")
            .display()
            .to_string();
        self.notice = Some(Notice {
            tone: NoticeTone::Information,
            title: "Demonstration fixture selected".to_owned(),
            detail: "This local synthetic image contains one FAT12 metadata candidate and one PNG carving candidate.".to_owned(),
        });
    }

    fn load_document_fixture(&mut self) {
        self.reset_workspace();
        self.image_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/document-carving-multimethod-v1/source.img")
            .display()
            .to_string();
        self.notice = Some(Notice {
            tone: NoticeTone::Information,
            title: "Document-carving fixture selected".to_owned(),
            detail: "This local synthetic image contains one structurally validated PDF and one DOCX-style Open XML package candidate."
                .to_owned(),
        });
    }

    fn load_media_fixture(&mut self) {
        self.reset_workspace();
        self.image_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/media-carving-multimethod-v1/source.img")
            .display()
            .to_string();
        self.notice = Some(Notice {
            tone: NoticeTone::Information,
            title: "Media-carving fixture selected".to_owned(),
            detail: "This local synthetic image contains one structurally bounded GIF, one standard AVI container, and one self-contained MP4 candidate. Fragmented media is intentionally excluded."
                .to_owned(),
        });
    }

    fn load_ntfs_contiguous_fixture(&mut self) {
        self.reset_workspace();
        self.image_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ntfs-deleted-contiguous-v1/source.img")
            .display()
            .to_string();
        self.notice = Some(Notice {
            tone: NoticeTone::Information,
            title: "NTFS contiguous recovery fixture selected".to_owned(),
            detail: "This local synthetic NTFS image contains one deleted non-resident record whose single former extent is currently marked free."
                .to_owned(),
        });
    }

    fn load_ntfs_fixture(&mut self) {
        self.reset_workspace();
        self.image_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ntfs-deleted-resident-v1/source.img")
            .display()
            .to_string();
        self.notice = Some(Notice {
            tone: NoticeTone::Information,
            title: "NTFS resident recovery fixture selected".to_owned(),
            detail: "This local synthetic NTFS image contains one deleted Master File Table record with valid sector fixups and resident data."
                .to_owned(),
        });
    }

    fn load_exfat_fixture(&mut self) {
        self.reset_workspace();
        self.image_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/exfat-contiguous-deleted-v1/source.img")
            .display()
            .to_string();
        self.notice = Some(Notice {
            tone: NoticeTone::Information,
            title: "exFAT recovery fixture selected".to_owned(),
            detail: "This local synthetic exFAT image contains one deleted root file whose entry set validates and whose contiguous cluster extent is currently free."
                .to_owned(),
        });
    }

    fn reset_workspace(&mut self) {
        self.candidates.clear();
        self.catalogue = None;
        self.presentations.clear();
        self.selected_id = None;
        self.source_detail = None;
        self.session_manifest = None;
        self.session_manifest_path = None;
        self.source_integrity = None;
        self.export_audit = None;
        self.cancel_preview_worker();
        self.show_recovery_review = false;
    }

    fn apply_manifest(
        &mut self,
        manifest: SessionManifest,
        manifest_path: Option<PathBuf>,
        source_integrity: SourceIntegrity,
    ) {
        self.image_path = manifest
            .session
            .source
            .identity
            .canonical_path
            .display()
            .to_string();
        self.candidates = manifest.candidates.clone();
        self.source_detail = Some(format!(
            "{} • {} bytes • SHA-256 {}…",
            manifest.session.source.display_name,
            manifest.session.source.identity.byte_length,
            &manifest.session.source.identity.sha256[..12]
        ));
        self.source_integrity = Some(source_integrity);
        self.export_audit = None;
        self.session_manifest_path = manifest_path;
        self.session_manifest = Some(manifest);
        self.selected_id = None;
        self.show_recovery_review = false;
        self.refresh_catalogue();
    }

    fn recheck_source_integrity(&mut self) {
        let Some(manifest) = self.session_manifest.as_ref() else {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Scan or open a session before checking the source".to_owned(),
                detail: "Source identity can be checked only against a completed local recovery session."
                    .to_owned(),
            });
            return;
        };

        let integrity = manifest.verify_source();
        self.notice = Some(match &integrity {
            SourceIntegrity::Verified => Notice {
                tone: NoticeTone::Success,
                title: "Source remains verified".to_owned(),
                detail: "The current byte length, SHA-256, and BLAKE3 still match this recovery session."
                    .to_owned(),
            },
            SourceIntegrity::Changed { .. } => Notice {
                tone: NoticeTone::Warning,
                title: "Source changed — recovery remains blocked".to_owned(),
                detail: "The historical catalogue remains available, but scan the changed image as a new session before exporting."
                    .to_owned(),
            },
            SourceIntegrity::Unavailable { detail } => Notice {
                tone: NoticeTone::Warning,
                title: "Source unavailable — recovery remains blocked".to_owned(),
                detail: format!(
                    "The historical catalogue remains available, but the current source could not be verified: {detail}"
                ),
            },
        });
        let preview_allowed = matches!(integrity, SourceIntegrity::Verified);
        self.source_integrity = Some(integrity);
        if !preview_allowed {
            self.refresh_catalogue();
        }
    }

    fn audit_recorded_exports(&mut self) {
        let Some(manifest) = self.session_manifest.as_ref() else {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Open a session before auditing exports".to_owned(),
                detail:
                    "Export evidence can be audited only from a completed local recovery session."
                        .to_owned(),
            });
            return;
        };

        let audit = manifest.verify_recorded_exports();
        let verified = audit
            .iter()
            .filter(|result| matches!(result.integrity, RecordedExportIntegrity::Verified))
            .count();
        let total = audit.len();
        let has_problem = verified != total;
        self.notice = Some(Notice {
            tone: if has_problem {
                NoticeTone::Warning
            } else {
                NoticeTone::Success
            },
            title: if has_problem {
                "Export audit needs review".to_owned()
            } else {
                "Recorded exports verified".to_owned()
            },
            detail: if total == 0 {
                "This session has no recorded exports to audit.".to_owned()
            } else if has_problem {
                format!(
                    "{verified} of {total} recorded exports match their persisted receipt and current artifact hashes. No files were changed."
                )
            } else {
                format!(
                    "All {total} recorded exports match their persisted receipt and current artifact hashes. No files were changed."
                )
            },
        });
        self.export_audit = Some(audit);
    }

    fn save_session(&mut self, manifest_path: PathBuf) -> Result<(), String> {
        let manifest = self
            .session_manifest
            .as_ref()
            .ok_or_else(|| "Scan or open an image session before saving it.".to_owned())?;
        manifest
            .save(&manifest_path)
            .map_err(|error| error.to_string())?;
        self.session_manifest_path = Some(manifest_path);
        Ok(())
    }

    fn choose_session_to_save(&mut self) {
        if self.session_manifest.is_none() {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Scan an image before saving a session".to_owned(),
                detail: "A saved workspace records a completed read-only scan, not an unscanned image path."
                    .to_owned(),
            });
            return;
        }

        if let Some(path) = FileDialog::new()
            .set_title("Save local recovery session")
            .add_filter("DiskTrace session", &["json"])
            .set_file_name("disktrace-session.json")
            .save_file()
        {
            match self.save_session(path.clone()) {
                Ok(()) => {
                    self.notice = Some(Notice {
                        tone: NoticeTone::Success,
                        title: "Session saved locally".to_owned(),
                        detail: format!(
                            "Saved the scan catalogue and export history to {}. The source image was not copied or modified.",
                            path.display()
                        ),
                    });
                }
                Err(error) => {
                    self.notice = Some(Notice {
                        tone: NoticeTone::Error,
                        title: "Session was not saved".to_owned(),
                        detail: error,
                    });
                }
            }
        }
    }

    fn choose_session_to_open(&mut self) {
        if let Some(path) = FileDialog::new()
            .set_title("Open local recovery session")
            .add_filter("EvidenceForge session", &["json"])
            .pick_file()
        {
            match SessionManifest::load(&path) {
                Ok(manifest) => {
                    let integrity = manifest.verify_source();
                    self.apply_manifest(manifest, Some(path.clone()), integrity.clone());
                    self.notice = Some(match integrity {
                        SourceIntegrity::Verified => Notice {
                            tone: NoticeTone::Success,
                            title: "Saved session opened and source verified".to_owned(),
                            detail: "The current source image matches the saved byte length, SHA-256, and BLAKE3 identity. Recovery is available."
                                .to_owned(),
                        },
                        SourceIntegrity::Changed { .. } => Notice {
                            tone: NoticeTone::Warning,
                            title: "Saved session opened, but the source changed".to_owned(),
                            detail: "The historical catalogue remains available, but recovery is blocked until you scan the changed image as a new session."
                                .to_owned(),
                        },
                        SourceIntegrity::Unavailable { detail } => Notice {
                            tone: NoticeTone::Warning,
                            title: "Saved session opened, but the source is unavailable".to_owned(),
                            detail: format!(
                                "The historical catalogue and export history remain available. Recovery is blocked: {detail}"
                            ),
                        },
                    });
                }
                Err(error) => {
                    self.notice = Some(Notice {
                        tone: NoticeTone::Error,
                        title: "Saved session could not be opened".to_owned(),
                        detail: error.to_string(),
                    });
                }
            }
        }
    }

    fn save_case_brief(&self, case_brief_path: PathBuf) -> Result<(), String> {
        let manifest = self.session_manifest.as_ref().ok_or_else(|| {
            "Scan or open an image session before saving a case brief.".to_owned()
        })?;
        manifest
            .save_case_brief(&case_brief_path)
            .map_err(|error| error.to_string())
    }

    fn choose_case_brief_to_save(&mut self) {
        if self.session_manifest.is_none() {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Scan or open a session before saving a case brief".to_owned(),
                detail: "A case brief summarizes a completed local session and its current verification state."
                    .to_owned(),
            });
            return;
        }

        if let Some(path) = FileDialog::new()
            .set_title("Save local case brief")
            .add_filter("Markdown", &["md"])
            .set_file_name("evidenceforge-case-brief.md")
            .save_file()
        {
            match self.save_case_brief(path.clone()) {
                Ok(()) => {
                    self.notice = Some(Notice {
                        tone: NoticeTone::Success,
                        title: "Case brief saved locally".to_owned(),
                        detail: format!(
                            "Saved the current source, candidate, and export-audit summary to {}. It contains no source-image or recovered-file payload bytes.",
                            path.display()
                        ),
                    });
                }
                Err(error) => {
                    self.notice = Some(Notice {
                        tone: NoticeTone::Error,
                        title: "Case brief was not saved".to_owned(),
                        detail: error,
                    });
                }
            }
        }
    }

    fn choose_image(&mut self) {
        if let Some(path) = FileDialog::new()
            .set_title("Choose a recovery image")
            .add_filter("Disk images", &["img", "dd", "raw", "iso", "bin"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.reset_workspace();
            self.image_path = path.display().to_string();
            self.notice = Some(Notice {
                tone: NoticeTone::Information,
                title: "Image selected".to_owned(),
                detail: "The selected path has not been scanned or modified. Start a read-only scan when ready.".to_owned(),
            });
        }
    }

    fn choose_destination(&mut self) {
        if let Some(path) = FileDialog::new()
            .set_title("Choose a separate recovery destination")
            .pick_folder()
        {
            self.destination_path = path.display().to_string();
            self.notice = Some(Notice {
                tone: NoticeTone::Information,
                title: "Destination selected".to_owned(),
                detail: "EvidenceForge will validate this destination against the source image before any export.".to_owned(),
            });
        }
    }

    fn start_scan(&mut self) {
        if self.scan_worker.is_some() {
            return;
        }
        let path = self.image_path.trim();
        if path.is_empty() {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Choose an image first".to_owned(),
                detail: "Enter the path to a local disk image, load the demonstration fixture, or use the native chooser.".to_owned(),
            });
            return;
        }

        self.scan_generation += 1;
        let generation = self.scan_generation;
        let image_path = PathBuf::from(path);
        let cancellation = Arc::new(AtomicBool::new(false));
        let worker_cancellation = Arc::clone(&cancellation);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let event = match scan_image_with_cancellation(&image_path, &worker_cancellation) {
                Err(WorkflowError::Core(CoreError::Cancelled)) => ScanWorkerEvent::Cancelled,
                result => {
                    ScanWorkerEvent::Completed(Box::new(result.map_err(|error| error.to_string())))
                }
            };
            let _ = sender.send(event);
        });
        self.scan_worker = Some(ScanWorker {
            generation,
            cancellation,
            receiver,
        });
        self.notice = Some(Notice {
            tone: NoticeTone::Information,
            title: "Scanning image".to_owned(),
            detail: "EvidenceForge is reading the selected image in the background. The source remains read-only.".to_owned(),
        });
    }

    fn cancel_scan(&mut self) {
        if let Some(worker) = &self.scan_worker {
            worker.cancellation.store(true, Ordering::Relaxed);
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Stopping scan".to_owned(),
                detail: "EvidenceForge is stopping the current local read. The pending result will be discarded and any previous catalogue remains available.".to_owned(),
            });
        }
    }

    fn scan_is_stopping(&self) -> bool {
        self.scan_worker
            .as_ref()
            .is_some_and(|worker| worker.cancellation.load(Ordering::Relaxed))
    }

    fn poll_scan_worker(&mut self) {
        let event = self.scan_worker.as_ref().map(|worker| {
            (
                worker.generation,
                worker.cancellation.load(Ordering::Relaxed),
                worker.receiver.try_recv(),
            )
        });
        let Some((generation, cancellation_requested, event)) = event else {
            return;
        };

        match event {
            Ok(ScanWorkerEvent::Cancelled) => {
                self.scan_worker = None;
                self.notice = Some(Notice {
                    tone: NoticeTone::Information,
                    title: "Scan stopped".to_owned(),
                    detail: "The local read acknowledged cancellation. No new result was applied and any previous catalogue remains available."
                        .to_owned(),
                });
            }
            Ok(ScanWorkerEvent::Completed(result)) => {
                self.scan_worker = None;
                let result = *result;
                if cancellation_requested {
                    self.notice = Some(Notice {
                        tone: NoticeTone::Information,
                        title: "Scan result discarded".to_owned(),
                        detail: "A stop was requested before the pending scan result was applied. Any previous catalogue remains available."
                            .to_owned(),
                    });
                    return;
                }
                if generation != self.scan_generation {
                    return;
                }
                match result {
                    Ok(scan) => self.apply_scan(scan),
                    Err(error) => {
                        self.notice = Some(Notice {
                            tone: NoticeTone::Error,
                            title: "The image could not be scanned".to_owned(),
                            detail: error,
                        });
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.scan_worker = None;
                self.notice = Some(if cancellation_requested {
                    Notice {
                        tone: NoticeTone::Information,
                        title: "Scan stopped".to_owned(),
                        detail: "The local read stopped before producing a result. Any previous catalogue remains available."
                            .to_owned(),
                    }
                } else {
                    Notice {
                        tone: NoticeTone::Error,
                        title: "The scan worker stopped unexpectedly".to_owned(),
                        detail:
                            "No new scan result was applied. You can try scanning the image again."
                                .to_owned(),
                    }
                });
            }
        }
    }

    fn apply_scan(&mut self, scan: ScanResult) {
        match SessionManifest::new(scan.session, scan.candidates) {
            Ok(manifest) => {
                self.apply_manifest(manifest, None, SourceIntegrity::Verified);
                self.notice = Some(Notice {
                    tone: NoticeTone::Success,
                    title: "Read-only scan complete".to_owned(),
                    detail: "Review results, inspect the explanation for each item, then save only to a separate destination. Save the session locally when you want to return later."
                        .to_owned(),
                });
            }
            Err(error) => {
                self.notice = Some(Notice {
                    tone: NoticeTone::Error,
                    title: "The scan results could not become a recovery session".to_owned(),
                    detail: error.to_string(),
                });
            }
        }
    }

    fn refresh_catalogue(&mut self) {
        let query = CatalogueQuery {
            text: (!self.search.trim().is_empty()).then(|| self.search.clone()),
            methods: match self.method_filter {
                MethodFilter::All => Vec::new(),
                MethodFilter::Fat12 => vec![RecoveryMethod::Fat12DeletedRootMetadata],
                MethodFilter::Fat16 => vec![RecoveryMethod::Fat16DeletedRootMetadata],
                MethodFilter::Exfat => vec![RecoveryMethod::ExfatDeletedContiguousRootMetadata],
                MethodFilter::Ntfs => vec![RecoveryMethod::NtfsDeletedResidentRecord],
                MethodFilter::NtfsContiguous => {
                    vec![RecoveryMethod::NtfsDeletedContiguousNonresident]
                }
                MethodFilter::Png => vec![RecoveryMethod::SignatureCarvingPng],
                MethodFilter::Jpeg => vec![RecoveryMethod::SignatureCarvingJpeg],
                MethodFilter::Gif => vec![RecoveryMethod::SignatureCarvingGif],
                MethodFilter::Avi => vec![RecoveryMethod::SignatureCarvingAvi],
                MethodFilter::Mp4 => vec![RecoveryMethod::SignatureCarvingMp4],
                MethodFilter::Pdf => vec![RecoveryMethod::SignatureCarvingPdf],
                MethodFilter::ZipOffice => vec![RecoveryMethod::SignatureCarvingZipOffice],
            },
            validations: match self.validation_filter {
                ValidationFilter::All => Vec::new(),
                ValidationFilter::Checked => vec![CandidateValidation::ContentValidated],
                ValidationFilter::Review => vec![CandidateValidation::RecoveredUnvalidated],
            },
        };
        let catalogue = build_catalogue(self.candidates.clone(), &query);
        self.presentations = catalogue
            .candidates
            .iter()
            .cloned()
            .map(|candidate| present_candidate(candidate, None))
            .collect();
        if let Some(selected_id) = &self.selected_id {
            if !self
                .presentations
                .iter()
                .any(|presentation| presentation.candidate.id == *selected_id)
            {
                self.selected_id = None;
            }
        }
        self.catalogue = Some(catalogue);
        self.start_selected_preview();
    }

    fn cancel_preview_worker(&mut self) {
        self.preview_generation += 1;
        if let Some(worker) = &self.preview_worker {
            worker.cancellation.store(true, Ordering::Relaxed);
        }
        self.preview_worker = None;
        self.preview_error = None;
    }

    fn start_selected_preview(&mut self) {
        self.cancel_preview_worker();
        let Some(candidate_id) = self.selected_id.clone() else {
            return;
        };
        if !self
            .presentations
            .iter()
            .any(|presentation| presentation.candidate.id == candidate_id)
        {
            return;
        }
        if !matches!(
            self.source_integrity.as_ref(),
            Some(SourceIntegrity::Verified)
        ) {
            self.preview_error = Some((
                candidate_id,
                "The source is not verified for this recovery session, so no new preview bytes were read. Recheck the source or scan the current image as a new session."
                    .to_owned(),
            ));
            return;
        }

        let Some(manifest) = self.session_manifest.clone() else {
            self.preview_error = Some((
                candidate_id,
                "The selected preview requires a completed local recovery session. Scan the image again before preparing preview bytes."
                    .to_owned(),
            ));
            return;
        };

        let generation = self.preview_generation;
        let preview_candidate_id = candidate_id.clone();
        let cancellation = Arc::new(AtomicBool::new(false));
        let worker_cancellation = Arc::clone(&cancellation);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = read_session_candidate_range(
                &manifest,
                &preview_candidate_id,
                worker_cancellation.as_ref(),
            )
            .map(|recovered| recovered.bytes)
            .map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
        self.preview_worker = Some(PreviewWorker {
            generation,
            candidate_id,
            cancellation,
            receiver,
        });
    }

    fn poll_preview_worker(&mut self) {
        let event = self.preview_worker.as_ref().map(|worker| {
            (
                worker.generation,
                worker.candidate_id.clone(),
                worker.receiver.try_recv(),
            )
        });
        let Some((generation, candidate_id, event)) = event else {
            return;
        };

        match event {
            Ok(result) => {
                self.preview_worker = None;
                if generation != self.preview_generation
                    || self.selected_id.as_deref() != Some(candidate_id.as_str())
                {
                    return;
                }
                match result {
                    Ok(bytes) => {
                        if let Some(presentation) = self
                            .presentations
                            .iter_mut()
                            .find(|presentation| presentation.candidate.id == candidate_id)
                        {
                            *presentation = present_candidate(
                                presentation.candidate.clone(),
                                Some(bytes.as_slice()),
                            );
                        }
                    }
                    Err(error) => {
                        self.preview_error = Some((candidate_id, error));
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.preview_worker = None;
                if generation == self.preview_generation
                    && self.selected_id.as_deref() == Some(candidate_id.as_str())
                {
                    self.preview_error = Some((
                        candidate_id,
                        "The bounded local preview worker stopped before it returned a result. Select the result again to retry."
                            .to_owned(),
                    ));
                }
            }
        }
    }

    fn selected_preview_is_loading(&self) -> bool {
        self.preview_worker
            .as_ref()
            .is_some_and(|worker| self.selected_id.as_deref() == Some(worker.candidate_id.as_str()))
    }

    fn selected_preview_error(&self) -> Option<&str> {
        self.preview_error
            .as_ref()
            .and_then(|(candidate_id, error)| {
                (self.selected_id.as_deref() == Some(candidate_id.as_str()))
                    .then_some(error.as_str())
            })
    }

    fn select_result(&mut self, candidate_id: String) {
        if self.selected_id.as_deref() == Some(candidate_id.as_str()) {
            return;
        }
        self.selected_id = Some(candidate_id);
        self.start_selected_preview();
    }

    fn selected_presentation(&self) -> Option<&CandidatePresentation> {
        let selected_id = self.selected_id.as_deref()?;
        self.presentations
            .iter()
            .find(|presentation| presentation.candidate.id == selected_id)
    }

    fn begin_recovery_review(&mut self) {
        if self.selected_id.is_none() {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Select a result to recover".to_owned(),
                detail: "Choose one candidate from the results list before saving it.".to_owned(),
            });
            return;
        }
        if self.destination_path.trim().is_empty() {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Choose a separate destination".to_owned(),
                detail: "EvidenceForge refuses to write recovered files into the source image storage location.".to_owned(),
            });
            return;
        }
        if !matches!(
            self.source_integrity.as_ref(),
            Some(SourceIntegrity::Verified)
        ) {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Recovery remains blocked".to_owned(),
                detail: "The source must match the saved identity before an export can be created."
                    .to_owned(),
            });
            return;
        }
        self.show_recovery_review = true;
    }

    fn select_result_by_offset(&mut self, offset: isize) {
        if self.presentations.is_empty() {
            return;
        }
        let last_index = self.presentations.len().saturating_sub(1);
        let selected_index = self.selected_id.as_ref().and_then(|selected_id| {
            self.presentations
                .iter()
                .position(|presentation| presentation.candidate.id == *selected_id)
        });
        let next_index = match selected_index {
            Some(index) => (index as isize + offset).clamp(0, last_index as isize) as usize,
            None if offset < 0 => last_index,
            None => 0,
        };
        self.select_result(self.presentations[next_index].candidate.id.clone());
    }

    fn recover_selected(&mut self) {
        self.show_recovery_review = false;
        let Some(candidate_id) = self.selected_id.clone() else {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Select a result to recover".to_owned(),
                detail: "Choose one candidate from the results list before saving it.".to_owned(),
            });
            return;
        };
        if self.destination_path.trim().is_empty() {
            self.notice = Some(Notice {
                tone: NoticeTone::Warning,
                title: "Choose a separate destination".to_owned(),
                detail: "EvidenceForge refuses to write recovered files into the source image storage location.".to_owned(),
            });
            return;
        }

        let export = match self.session_manifest.as_mut() {
            Some(manifest) => {
                manifest.recover_to_destination(&candidate_id, &self.destination_path)
            }
            None => {
                self.notice = Some(Notice {
                    tone: NoticeTone::Error,
                    title: "Recovery session is unavailable".to_owned(),
                    detail: "Scan the image again before recovering a result.".to_owned(),
                });
                return;
            }
        };

        match export {
            Ok(export) => {
                self.export_audit = None;
                self.source_integrity = self
                    .session_manifest
                    .as_ref()
                    .map(SessionManifest::verify_source);
                let history_detail = match self.session_manifest_path.clone() {
                    Some(manifest_path) => match self.save_session(manifest_path.clone()) {
                        Ok(()) => format!(
                            "Saved {} and recorded {} in the receipt and local session history.",
                            export.output_path.display(),
                            export.receipt_path.display()
                        ),
                        Err(error) => format!(
                            "Saved {} and recorded {}. The export history is currently in memory but could not be saved to {}: {error}",
                            export.output_path.display(),
                            export.receipt_path.display(),
                            manifest_path.display()
                        ),
                    },
                    None => format!(
                        "Saved {} and recorded {}. Save the local session to retain this export history after closing the app.",
                        export.output_path.display(),
                        export.receipt_path.display()
                    ),
                };
                self.notice = Some(Notice {
                    tone: NoticeTone::Success,
                    title: "Recovery export completed".to_owned(),
                    detail: history_detail,
                });
            }
            Err(error) => {
                self.source_integrity = self
                    .session_manifest
                    .as_ref()
                    .map(SessionManifest::verify_source);
                let (title, detail) = match self.source_integrity.as_ref() {
                    Some(SourceIntegrity::Changed { .. }) => (
                        "Recovery was blocked because the source changed".to_owned(),
                        "The historical session remains available, but scan the changed image as a new session before exporting."
                            .to_owned(),
                    ),
                    Some(SourceIntegrity::Unavailable { detail }) => (
                        "Recovery was blocked because the source is unavailable".to_owned(),
                        detail.clone(),
                    ),
                    _ => ("Recovery export was not completed".to_owned(), error.to_string()),
                };
                self.notice = Some(Notice {
                    tone: NoticeTone::Error,
                    title,
                    detail,
                });
            }
        }
    }
}

impl eframe::App for EvidenceForgeApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_scan_worker();
        self.poll_preview_worker();
        let keyboard_available = !context.wants_keyboard_input();
        if keyboard_available
            && context.input(|input| input.modifiers.command && input.key_pressed(egui::Key::O))
        {
            self.choose_image();
        }
        if keyboard_available
            && context.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter))
        {
            self.start_scan();
        }
        if keyboard_available
            && context.input(|input| input.modifiers.command && input.key_pressed(egui::Key::S))
        {
            self.choose_session_to_save();
        }
        if context.input(|input| input.key_pressed(egui::Key::F1)) {
            self.show_shortcuts = true;
        }
        if self.show_shortcuts && context.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.show_shortcuts = false;
        }
        if self.show_recovery_review && context.input(|input| input.key_pressed(egui::Key::Escape))
        {
            self.show_recovery_review = false;
        }
        if keyboard_available && !self.show_shortcuts && !self.show_recovery_review {
            let navigation = context.input(|input| {
                if input.key_pressed(egui::Key::ArrowUp) {
                    Some(-1)
                } else if input.key_pressed(egui::Key::ArrowDown) {
                    Some(1)
                } else {
                    None
                }
            });
            if let Some(offset) = navigation {
                self.select_result_by_offset(offset);
            }
        }
        if self.scan_worker.is_some() || self.preview_worker.is_some() {
            context.request_repaint_after(Duration::from_millis(80));
        }
        let (workflow_label, workflow_color) = workflow_state_label(self);
        egui::TopBottomPanel::top("top_bar").show(context, |ui| {
            egui::Frame::NONE
                .fill(Palette::CHROME)
                .inner_margin(egui::Margin::symmetric(18, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("DiskTrace")
                                .size(21.0)
                                .strong()
                                .color(Palette::TEXT),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Recovery workspace")
                                .small()
                                .color(Palette::TEXT_MUTED),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Help · F1").clicked() {
                                self.show_shortcuts = true;
                            }
                            ui.add_space(10.0);
                            ui.colored_label(workflow_color, format!("• {workflow_label}"));
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new("Read-only source")
                                    .small()
                                    .color(Palette::TEXT_MUTED),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Local only")
                                    .small()
                                    .color(Palette::TEXT_MUTED),
                            );
                        });
                    });
                });
        });

        egui::SidePanel::left("workflow_panel")
            .resizable(false)
            .default_width(320.0)
            .show(context, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                ui.add_space(12.0);
                workflow_steps_panel(ui, self);
                ui.small("The rail continues with filters and destination controls below.");
                ui.add_space(14.0);
                ui.collapsing("Open or save an evidence session", |ui| {
                    ui.label("Keep a local, resumable record of a completed scan and its exports.");
                    ui.horizontal(|ui| {
                        if ui.button("Open saved session…").clicked() {
                            self.choose_session_to_open();
                        }
                        if ui.button("Save current session…").clicked() {
                            self.choose_session_to_save();
                        }
                    });
                    ui.small("Session files contain local paths, source hashes, candidate details, and export receipts. They never include recovered file bytes.");
                });
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);
                ui.heading("1. Select a recovery image");
                ui.label("Use an image copy whenever possible. Your selected image stays read-only.");
                ui.add_space(8.0);
                ui.label("Local image path");
                ui.add(
                    egui::TextEdit::multiline(&mut self.image_path)
                        .desired_rows(3)
                        .hint_text("/path/to/recovery-image.img"),
                );
                ui.add_space(6.0);
                if self.image_path.trim().is_empty() {
                    ui.small("Use the primary action in the workspace to choose an image or try the guided demo.");
                } else {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Replace image…").clicked() {
                            self.choose_image();
                        }
                        if ui.button("Try another guided demo").clicked() {
                            self.load_demo_fixture();
                            self.start_scan();
                        }
                    });
                }
                ui.collapsing("Advanced synthetic fixtures", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Documents").clicked() {
                            self.load_document_fixture();
                        }
                        if ui.button("Media").clicked() {
                            self.load_media_fixture();
                        }
                        if ui.button("exFAT").clicked() {
                            self.load_exfat_fixture();
                        }
                        if ui.button("NTFS resident").clicked() {
                            self.load_ntfs_fixture();
                        }
                        if ui.button("NTFS contiguous").clicked() {
                            self.load_ntfs_contiguous_fixture();
                        }
                    });
                    ui.small("These harmless local fixtures demonstrate supported recovery boundaries. They are not real evidence images.");
                });
                let scanning = self.scan_worker.is_some();
                let stopping_scan = self.scan_is_stopping();
                ui.add_space(4.0);
                if scanning {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            egui::RichText::new(if stopping_scan {
                                "Stopping local scan — discarding the pending result"
                            } else {
                                "Scanning locally — the source stays read-only"
                            })
                            .color(if stopping_scan {
                                Palette::WARNING_STRONG
                            } else {
                                Palette::FOCUS_STRONG
                            }),
                        );
                    });
                    ui.add_space(6.0);
                    let (title, detail, color) = active_scan_presentation(stopping_scan);
                    action_guidance_panel(ui, title, detail, color);
                } else if self.image_path.trim().is_empty() {
                    ui.small("Choose an image or start the synthetic guided demo.");
                } else {
                    ui.small("Ready for a read-only scan. No source bytes will be changed.");
                }
                ui.add_space(6.0);
                let has_completed_scan = self.session_manifest.is_some();
                if ui
                    .add_enabled_ui(!scanning && !self.image_path.trim().is_empty(), |ui| {
                        ui.add_sized(
                            [ui.available_width(), 38.0],
                            egui::Button::new(
                                egui::RichText::new(if has_completed_scan {
                                    "Scan image again"
                                } else {
                                    "Scan selected image"
                                })
                                .strong()
                                .color(if has_completed_scan {
                                    Palette::TEXT_SOFT
                                } else {
                                    Palette::INK
                                }),
                            )
                            .fill(if has_completed_scan {
                                Palette::SURFACE_RAISED
                            } else {
                                Palette::SUCCESS_STRONG
                            }),
                        )
                        .clicked()
                    })
                    .inner
                {
                    self.start_scan();
                }
                if scanning
                    && ui
                        .add_enabled_ui(!stopping_scan, |ui| {
                            ui.add_sized(
                                [ui.available_width(), 30.0],
                                egui::Button::new(if stopping_scan {
                                    "Stopping local scan…"
                                } else {
                                    "Stop scan"
                                }),
                            )
                        })
                        .inner
                        .clicked()
                {
                    self.cancel_scan();
                }
                ui.add_space(18.0);
                ui.separator();
                ui.add_space(10.0);
                ui.heading("2. Filter results");
                ui.label("Search");
                ui.text_edit_singleline(&mut self.search);
                ui.add_space(6.0);
                egui::ComboBox::from_label("Recovery method")
                    .selected_text(method_filter_label(&self.method_filter))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::All,
                            "All methods",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Fat12,
                            "Deleted FAT12 metadata",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Fat16,
                            "Deleted FAT16 metadata",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Exfat,
                            "Deleted exFAT contiguous metadata",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Ntfs,
                            "Deleted NTFS resident records",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::NtfsContiguous,
                            "Deleted NTFS contiguous metadata",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Png,
                            "PNG signature carving",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Jpeg,
                            "JPEG signature carving",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Gif,
                            "GIF structural carving",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Avi,
                            "AVI structural carving",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Mp4,
                            "MP4 and MOV structural carving",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::Pdf,
                            "PDF structural carving",
                        );
                        ui.selectable_value(
                            &mut self.method_filter,
                            MethodFilter::ZipOffice,
                            "ZIP and Open XML carving",
                        );
                    });
                egui::ComboBox::from_label("Result status")
                    .selected_text(validation_filter_label(&self.validation_filter))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.validation_filter,
                            ValidationFilter::All,
                            "All results",
                        );
                        ui.selectable_value(
                            &mut self.validation_filter,
                            ValidationFilter::Checked,
                            "Recovered and checked",
                        );
                        ui.selectable_value(
                            &mut self.validation_filter,
                            ValidationFilter::Review,
                            "Review recommended",
                        );
                    });
                ui.horizontal(|ui| {
                    if ui.button("Apply filters").clicked() {
                        self.refresh_catalogue();
                    }
                    if ui.small_button("Reset").clicked() {
                        self.search.clear();
                        self.method_filter = MethodFilter::All;
                        self.validation_filter = ValidationFilter::All;
                        self.refresh_catalogue();
                    }
                });
                if let Some(catalogue) = &self.catalogue {
                    ui.small(format!(
                        "Showing {} of {} recovered candidates",
                        catalogue.candidates.len(),
                        self.candidates.len()
                    ));
                }
                ui.add_space(18.0);
                ui.separator();
                ui.add_space(10.0);
                ui.heading("3. Save safely");
                ui.label("Separate destination folder");
                ui.add(
                    egui::TextEdit::multiline(&mut self.destination_path)
                        .desired_rows(3)
                        .hint_text("/path/to/separate/recovery-output"),
                );
                if ui.button("Browse destination…").clicked() {
                    self.choose_destination();
                }
                ui.label(
                    "The destination must already exist and cannot be inside source image storage.",
                );
                    });
            });

        egui::CentralPanel::default().show(context, |ui| {
            if let Some(notice) = &self.notice {
                notice_panel(ui, notice);
                ui.add_space(12.0);
            }
            if let Some(manifest) = &self.session_manifest {
                let (recheck_requested, audit_requested, case_brief_requested) =
                    session_workspace_panel(
                        ui,
                        manifest,
                        self.session_manifest_path.as_deref(),
                        self.source_integrity.as_ref(),
                        self.export_audit.as_deref(),
                    );
                if recheck_requested {
                    self.recheck_source_integrity();
                }
                if audit_requested {
                    self.audit_recorded_exports();
                }
                if case_brief_requested {
                    self.choose_case_brief_to_save();
                }
                ui.add_space(12.0);
            }
            if self.catalogue.is_none() && self.session_manifest.is_none() && self.scan_worker.is_none() {
                start_workspace_panel(ui, self);
                ui.add_space(16.0);
            }

            let available = ui.available_size();
            let list_width = (available.x * 0.48)
                .clamp(360.0, 620.0)
                .min((available.x - 300.0).max(300.0));
            let detail_width = (available.x - list_width - 10.0).max(300.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(list_width, available.y),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.label(
                            egui::RichText::new("Recovered evidence")
                                .size(19.0)
                                .strong(),
                        );
                        let results_prompt = if let Some(source_detail) = &self.source_detail {
                            source_detail.as_str()
                        } else if self.scan_worker.is_some() {
                            "Reading the image locally. Results will appear here when the scan completes."
                        } else if self.image_path.trim().is_empty() {
                            "Choose a recovery image in the workspace above to begin."
                        } else {
                            "The image is ready. Start a read-only scan from the workflow rail or press Cmd/Ctrl + Enter."
                        };
                        ui.label(results_prompt);
                        if let Some(catalogue) = &self.catalogue {
                            ui.add_space(4.0);
                            ui.label(format!(
                                "{} results • {} metadata • {} carved • {} checked",
                                catalogue.summary.total_candidates,
                                catalogue.summary.metadata_candidates,
                                catalogue.summary.carved_candidates,
                                catalogue.summary.content_validated_candidates
                            ));
                        }
                        if !self.presentations.is_empty() {
                            ui.small("Use Up/Down to review the filtered results without leaving the evidence detail.");
                        }
                        ui.add_space(8.0);
                        if self.catalogue.is_some() && self.presentations.is_empty() {
                            workspace_empty_panel(
                                ui,
                                "No results match the current filters",
                                "Reset the filters in the workflow rail to show all candidates from this completed scan.",
                                Palette::WARNING,
                            );
                        } else {
                            let mut clicked_candidate_id = None;
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                for presentation in &self.presentations {
                                    let selected = self.selected_id.as_deref()
                                        == Some(presentation.candidate.id.as_str());
                                    let method_tone = method_color(presentation.candidate.method);
                                    let validation_tone = validation_color(presentation.candidate.validation);
                                    let response = egui::Frame::NONE
                                        .fill(if selected {
                                            Palette::SURFACE_RAISED
                                        } else {
                                            Palette::CANVAS
                                        })
                                        .stroke(egui::Stroke::new(
                                            if selected { 1.2_f32 } else { 0.0_f32 },
                                            if selected { Palette::FOCUS } else { Palette::CANVAS },
                                        ))
                                        .corner_radius(egui::CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(10, 9))
                                        .show(ui, |ui| {
                                            ui.horizontal_top(|ui| {
                                                if selected {
                                                    let (rule, _) = ui.allocate_exact_size(
                                                        egui::vec2(3.0, 38.0),
                                                        egui::Sense::hover(),
                                                    );
                                                    ui.painter().rect_filled(rule, 1.0, method_tone);
                                                    ui.add_space(3.0);
                                                }
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(&presentation.candidate.evidence_name)
                                                            .strong()
                                                            .color(if selected { Palette::TEXT } else { Palette::TEXT }),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(&presentation.method_label)
                                                            .small()
                                                            .color(if selected { method_tone } else { Palette::TEXT_SOFT }),
                                                    );
                                                });
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::TOP),
                                                    |ui| {
                                                        status_badge(
                                                            ui,
                                                            &presentation.validation_label,
                                                            validation_tone,
                                                        );
                                                    },
                                                );
                                            });
                                            ui.add_space(3.0);
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} bytes  •  source offset {}",
                                                    presentation.candidate.byte_length,
                                                    presentation.candidate.source_offset
                                                ))
                                                .small()
                                                .color(Palette::TEXT_FAINT),
                                            );
                                        })
                                        .response
                                        .interact(egui::Sense::click())
                                        .on_hover_text(format!(
                                            "Candidate {}",
                                            presentation.candidate.id
                                        ));
                                    if response.clicked() {
                                        clicked_candidate_id = Some(presentation.candidate.id.clone());
                                    }
                                    ui.add_space(6.0);
                                    }
                                });
                            if let Some(candidate_id) = clicked_candidate_id {
                                self.select_result(candidate_id);
                            }
                        }
                    },
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(detail_width, available.y),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("evidence_detail_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Evidence detail")
                                .size(11.0)
                                .strong()
                                .color(Palette::TEXT_SOFT),
                        );
                        let preview_loading = self.selected_preview_is_loading();
                        let preview_error = self.selected_preview_error().map(str::to_owned);
                        if let Some(presentation) = self.selected_presentation().cloned() {
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(&presentation.candidate.evidence_name)
                                    .size(21.0)
                                    .strong(),
                            );
                            ui.horizontal_wrapped(|ui| {
                                status_badge(
                                    ui,
                                    &presentation.method_label,
                                    method_color(presentation.candidate.method),
                                );
                                status_badge(
                                    ui,
                                    &presentation.validation_label,
                                    validation_color(presentation.candidate.validation),
                                );
                            });
                            ui.add_space(10.0);
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("At a glance").strong());
                                egui::Grid::new("candidate_at_a_glance")
                                    .num_columns(2)
                                    .spacing([18.0, 7.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Recovery method").color(Palette::TEXT_MUTED));
                                        ui.label(egui::RichText::new(&presentation.method_label).strong().color(Palette::TEXT));
                                        ui.end_row();
                                        ui.label(egui::RichText::new("Validation").color(Palette::TEXT_MUTED));
                                        ui.label(egui::RichText::new(&presentation.validation_label).strong().color(Palette::TEXT));
                                        ui.end_row();
                                        ui.label(egui::RichText::new("Recovered bytes").color(Palette::TEXT_MUTED));
                                        ui.label(egui::RichText::new(presentation.candidate.byte_length.to_string()).monospace().color(Palette::TEXT));
                                        ui.end_row();
                                        ui.label(egui::RichText::new("Source offset").color(Palette::TEXT_MUTED));
                                        ui.label(egui::RichText::new(presentation.candidate.source_offset.to_string()).monospace().color(Palette::TEXT));
                                        ui.end_row();
                                    });
                            });
                            ui.add_space(12.0);
                            ui.group(|ui| {
                                ui.strong("Preview");
                                match presentation.preview.kind {
                                    PreviewKind::TextExcerpt => {
                                        if let Some(mut text) = presentation.preview.text_excerpt {
                                            ui.add(
                                                egui::TextEdit::multiline(&mut text)
                                                    .desired_rows(6)
                                                    .interactive(false),
                                            );
                                        }
                                    }
                                    PreviewKind::StructureSummary => {
                                        ui.label(
                                            egui::RichText::new("Bounded structure summary")
                                                .small()
                                                .color(Palette::TEXT_SOFT),
                                        );
                                        egui::Grid::new("candidate_preview_structure")
                                            .num_columns(2)
                                            .spacing([14.0, 6.0])
                                            .show(ui, |ui| {
                                                for fact in &presentation.preview.facts {
                                                    ui.label(
                                                        egui::RichText::new(&fact.label)
                                                            .small()
                                                            .color(Palette::TEXT_MUTED),
                                                    );
                                                    ui.label(egui::RichText::new(&fact.value).color(Palette::TEXT));
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                    PreviewKind::MetadataOnly if preview_loading => {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.label("Preparing bounded local preview…");
                                        });
                                        ui.small(
                                            "EvidenceForge is rechecking the saved source identity, then reading only this selected candidate range. The source remains read-only and no file is opened or executed.",
                                        );
                                    }
                                    PreviewKind::MetadataOnly if preview_error.is_some() => {
                                        ui.label(
                                            egui::RichText::new("Bounded local preview unavailable")
                                                .color(Palette::ERROR_STRONG),
                                        );
                                        ui.small(preview_error.as_deref().unwrap_or_default());
                                    }
                                    PreviewKind::MetadataOnly => {
                                        ui.label("No bounded structure summary is available for this recovered binary.");
                                    }
                                }
                                if !preview_loading && preview_error.is_none() {
                                    ui.add_space(4.0);
                                    ui.small(&presentation.preview.note);
                                }
                            });
                            ui.add_space(12.0);
                            let (basis, validation) = candidate_evidence_presentation(&presentation.candidate);
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("What this evidence establishes").strong().color(Palette::TEXT));
                                ui.label(egui::RichText::new(basis).color(Palette::TEXT_SOFT));
                                ui.small(egui::RichText::new(validation).color(Palette::TEXT_SOFT));
                                ui.add_space(4.0);
                                ui.small(egui::RichText::new(
                                    "It does not establish the original path, completeness, authenticity, safety, legal admissibility, or recovery of every deleted file."
                                ).color(Palette::TEXT_MUTED));
                            });
                            ui.add_space(8.0);
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Method notes").strong().color(Palette::TEXT));
                                ui.label(egui::RichText::new(&presentation.explanation).color(Palette::TEXT_SOFT));
                            });
                            ui.add_space(8.0);
                            ui.collapsing("Candidate record", |ui| {
                                egui::Grid::new("candidate_metadata")
                                    .num_columns(2)
                                    .spacing([18.0, 8.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Candidate ID").color(Palette::TEXT_MUTED));
                                        ui.label(egui::RichText::new(&presentation.candidate.id).monospace().color(Palette::TEXT));
                                        ui.end_row();
                                        ui.label(egui::RichText::new("Original path").color(Palette::TEXT_MUTED));
                                        ui.label(egui::RichText::new("Unavailable for this result").color(Palette::TEXT_SOFT));
                                        ui.end_row();
                                    });
                            });
                            ui.add_space(14.0);
                            let source_verified = matches!(
                                self.source_integrity.as_ref(),
                                Some(SourceIntegrity::Verified)
                            );
                            let destination_ready = !self.destination_path.trim().is_empty();
                            if !source_verified {
                                action_guidance_panel(
                                    ui,
                                    "Recovery remains blocked",
                                    "The source must match the saved identity before an export can be created. Review the source status above and scan the changed image as a new session if needed.",
                                    Palette::WARNING,
                                );
                            } else if !destination_ready {
                                action_guidance_panel(
                                    ui,
                                    "Choose a separate destination",
                                    "Recovered files are never written beside the source image. Select an existing folder on separate storage before exporting.",
                                    Palette::FOCUS_STRONG,
                                );
                                ui.add_space(8.0);
                                if ui
                                    .add_sized(
                                        [240.0, 40.0],
                                        egui::Button::new(
                                            egui::RichText::new("Choose destination…")
                                                .strong()
                                                .color(Palette::INK),
                                        )
                                        .fill(Palette::FOCUS_STRONG),
                                    )
                                    .clicked()
                                {
                                    self.choose_destination();
                                }
                            } else if ui
                                .add_sized(
                                    [240.0, 40.0],
                                    egui::Button::new(
                                        egui::RichText::new("Recover selected file safely")
                                            .strong()
                                            .color(Palette::INK),
                                    )
.fill(Palette::SUCCESS_STRONG),
                                )
                                .clicked()
                            {
                                self.begin_recovery_review();
                            }
                        } else {
                            ui.add_space(20.0);
                            egui::Frame::NONE
                                .fill(Palette::SURFACE)
                                .stroke(egui::Stroke::new(
                                    1.0_f32,
                                    Palette::LINE,
                                ))
                                .corner_radius(egui::CornerRadius::same(10))
                                .inner_margin(egui::Margin::same(14))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Select a recovery result")
                                            .strong()
                                            .color(Palette::TEXT_SOFT),
                                    );
                                    ui.small("Its evidence method, validation state, source location, limitations, and safe preview will appear here. Use Up/Down to compare filtered results from the keyboard.");
                                });
                        }
                            });
                    },
                );
            });
        });
        shortcut_reference_window(context, &mut self.show_shortcuts);
        recovery_review_window(context, self);
    }
}

fn recovery_review_window(context: &egui::Context, app: &mut EvidenceForgeApp) {
    if !app.show_recovery_review {
        return;
    }
    let Some(presentation) = app.selected_presentation().cloned() else {
        app.show_recovery_review = false;
        return;
    };

    let mut cancel = false;
    let mut confirm = false;
    let response = egui::Modal::new(egui::Id::new("recovery_review_modal"))
        .frame(
            egui::Frame::NONE
                .fill(Palette::CANVAS)
                .stroke(egui::Stroke::new(1.0_f32, Palette::LINE_STRONG))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(16)),
        )
        .show(context, |ui| {
            ui.set_min_width(480.0);
            ui.label(
                egui::RichText::new("Review recovery export")
                    .size(20.0)
                    .strong(),
            );
            ui.label(
                egui::RichText::new("Confirm the evidence and destination")
                    .strong()
                    .color(Palette::FOCUS),
            );
            ui.small("This creates a recovered copy and a companion receipt. It never modifies the source image.");
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new(&presentation.candidate.evidence_name).strong());
                ui.horizontal_wrapped(|ui| {
                    status_badge(
                        ui,
                        &presentation.method_label,
                        method_color(presentation.candidate.method),
                    );
                    status_badge(
                        ui,
                        &presentation.validation_label,
                        validation_color(presentation.candidate.validation),
                    );
                });
                ui.add_space(6.0);
                egui::Grid::new("recovery_review_metadata")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Candidate ID");
                        ui.monospace(&presentation.candidate.id);
                        ui.end_row();
                        ui.label("Recovered bytes");
                        ui.monospace(presentation.candidate.byte_length.to_string());
                        ui.end_row();
                        ui.label("Source status");
                        ui.colored_label(Palette::SUCCESS, "Verified for this session");
                        ui.end_row();
                        ui.label("Destination");
                        ui.label(&app.destination_path);
                        ui.end_row();
                    });
            });
            ui.add_space(10.0);
            action_guidance_panel(
                ui,
                "One final safety check will run",
                "EvidenceForge validates the destination against the source before writing. If the destination is unsafe or unavailable, no recovery file is created.",
                Palette::WARNING,
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                if ui
                    .add_sized(
                        [260.0, 38.0],
                        egui::Button::new(
                            egui::RichText::new("Confirm recovery and create receipt")
                                .strong()
                                .color(Palette::INK),
                        )
                        .fill(Palette::SUCCESS_STRONG),
                    )
                    .clicked()
                {
                    confirm = true;
                }
            });
        });

    if confirm {
        app.recover_selected();
    } else if cancel || response.should_close() {
        app.show_recovery_review = false;
    }
}

fn shortcut_reference_window(context: &egui::Context, open: &mut bool) {
    egui::Window::new("Keyboard shortcuts")
        .open(open)
        .resizable(false)
        .collapsible(false)
        .default_width(340.0)
        .show(context, |ui| {
            ui.label(
                egui::RichText::new("Use the platform command key")
                    .strong()
                    .color(Palette::FOCUS),
            );
            ui.small("Command on macOS; Control on Windows and Linux. Shortcuts do not run while you are typing into a text field.");
            ui.add_space(8.0);
            egui::Grid::new("shortcut_reference")
                .num_columns(2)
                .spacing([20.0, 8.0])
                .show(ui, |ui| {
                    ui.monospace("Cmd/Ctrl + O (letter)");
                    ui.label("Choose a recovery image");
                    ui.end_row();
                    ui.monospace("Cmd/Ctrl + Enter");
                    ui.label("Start a read-only scan");
                    ui.end_row();
                    ui.monospace("Cmd/Ctrl + S");
                    ui.label("Save the current session");
                    ui.end_row();
                    ui.monospace("Up / Down");
                    ui.label("Review filtered recovery results");
                    ui.end_row();
                    ui.monospace("F1");
                    ui.label("Open this reference");
                    ui.end_row();
                    ui.monospace("Escape");
                    ui.label("Close this reference");
                    ui.end_row();
                });
        });
}

struct Palette;

impl Palette {
    const INK: egui::Color32 = egui::Color32::from_rgb(0x04, 0x0A, 0x12);
    const CANVAS: egui::Color32 = egui::Color32::from_rgb(0x08, 0x10, 0x1A);
    const CHROME: egui::Color32 = egui::Color32::from_rgb(0x0C, 0x16, 0x22);
    const SURFACE: egui::Color32 = egui::Color32::from_rgb(0x10, 0x20, 0x2E);
    const SURFACE_RAISED: egui::Color32 = egui::Color32::from_rgb(0x16, 0x28, 0x38);
    const SURFACE_MUTED: egui::Color32 = egui::Color32::from_rgb(0x0E, 0x1A, 0x26);
    #[allow(dead_code)]
    const SURFACE_SUBTLE: egui::Color32 = egui::Color32::from_rgb(0x14, 0x24, 0x32);
    const LINE: egui::Color32 = egui::Color32::from_rgb(0x2A, 0x42, 0x56);
    const LINE_STRONG: egui::Color32 = egui::Color32::from_rgb(0x3E, 0x5A, 0x70);
    #[allow(dead_code)]
    const LINE_FOCUS: egui::Color32 = egui::Color32::from_rgb(0x58, 0xC0, 0xD6);
    const TEXT: egui::Color32 = egui::Color32::from_rgb(0xF4, 0xF8, 0xFC);
    const TEXT_SOFT: egui::Color32 = egui::Color32::from_rgb(0xDC, 0xE8, 0xF0);
    const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0xB0, 0xC4, 0xD4);
    #[allow(dead_code)]
    const TEXT_FAINT: egui::Color32 = egui::Color32::from_rgb(0x88, 0x9E, 0xB4);
    const FOCUS: egui::Color32 = egui::Color32::from_rgb(0x00, 0xC4, 0xE0);
    const FOCUS_STRONG: egui::Color32 = egui::Color32::from_rgb(0x30, 0xE0, 0xF8);
    #[allow(dead_code)]
    const FOCUS_SOFT: egui::Color32 = egui::Color32::from_rgb(0x1C, 0x4C, 0x58);
    const INFO: egui::Color32 = egui::Color32::from_rgb(0x50, 0xA8, 0xE0);
    const INFO_STRONG: egui::Color32 = egui::Color32::from_rgb(0x70, 0xC0, 0xF0);
    const SUCCESS: egui::Color32 = egui::Color32::from_rgb(0x38, 0xD0, 0x94);
    const SUCCESS_STRONG: egui::Color32 = egui::Color32::from_rgb(0x5C, 0xE0, 0xAC);
    #[allow(dead_code)]
    const SUCCESS_SOFT: egui::Color32 = egui::Color32::from_rgb(0x12, 0x4C, 0x34);
    const WARNING: egui::Color32 = egui::Color32::from_rgb(0xF8, 0xC0, 0x30);
    const WARNING_STRONG: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xD0, 0x50);
    #[allow(dead_code)]
    const WARNING_SOFT: egui::Color32 = egui::Color32::from_rgb(0x4C, 0x3C, 0x0E);
    const ERROR: egui::Color32 = egui::Color32::from_rgb(0xF8, 0x60, 0x60);
    const ERROR_STRONG: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x80, 0x80);
    #[allow(dead_code)]
    const ERROR_SOFT: egui::Color32 = egui::Color32::from_rgb(0x4C, 0x1C, 0x1C);
    const METHOD_FAT12: egui::Color32 = egui::Color32::from_rgb(0x50, 0xD0, 0xF0);
    const METHOD_FAT16: egui::Color32 = egui::Color32::from_rgb(0x38, 0xE0, 0xBC);
    const METHOD_EXFAT: egui::Color32 = egui::Color32::from_rgb(0x60, 0xE0, 0x94);
    const METHOD_NTFS: egui::Color32 = egui::Color32::from_rgb(0x70, 0xC0, 0xF0);
    const METHOD_NTFS_CONTIGUOUS: egui::Color32 = egui::Color32::from_rgb(0x50, 0xA8, 0xD0);
    const METHOD_PNG: egui::Color32 = egui::Color32::from_rgb(0xC0, 0x94, 0xF0);
    const METHOD_JPEG: egui::Color32 = egui::Color32::from_rgb(0xF0, 0xB0, 0x60);
    const METHOD_GIF: egui::Color32 = egui::Color32::from_rgb(0xF0, 0xD0, 0x60);
    const METHOD_AVI: egui::Color32 = egui::Color32::from_rgb(0x70, 0xE0, 0xCC);
    const METHOD_MP4: egui::Color32 = egui::Color32::from_rgb(0x80, 0xB0, 0xF0);
    const METHOD_PDF: egui::Color32 = egui::Color32::from_rgb(0xF0, 0x80, 0x80);
    const METHOD_ZIP_OFFICE: egui::Color32 = egui::Color32::from_rgb(0x60, 0xD0, 0xE0);
}

fn configure_style(context: &egui::Context) {
    let mut style = (*context.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(9.0, 5.0);
    style.spacing.interact_size = egui::vec2(40.0, 30.0);
    style.spacing.text_edit_width = 220.0;
    context.set_style(style);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Palette::CHROME;
    visuals.window_fill = Palette::CANVAS;
    visuals.extreme_bg_color = Palette::INK;
    visuals.faint_bg_color = Palette::SURFACE_RAISED;
    visuals.selection.bg_fill = Palette::FOCUS.gamma_multiply(0.46);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, Palette::FOCUS_STRONG);
    visuals.widgets.inactive.bg_fill = Palette::SURFACE_RAISED;
    visuals.widgets.inactive.weak_bg_fill = Palette::SURFACE_MUTED;
    visuals.widgets.hovered.bg_fill = Palette::FOCUS.gamma_multiply(0.42);
    visuals.widgets.hovered.weak_bg_fill = Palette::FOCUS.gamma_multiply(0.32);
    visuals.widgets.active.bg_fill = Palette::FOCUS.gamma_multiply(0.56);
    visuals.override_text_color = Some(Palette::TEXT);
    context.set_visuals(visuals);
}

fn start_workspace_panel(ui: &mut egui::Ui, app: &mut EvidenceForgeApp) {
    egui::Frame::NONE
        .fill(Palette::SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, Palette::LINE))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(24, 22))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Start a local recovery session")
                    .size(24.0)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Choose an image copy, review what the scan found, then write only to a separate destination.",
                )
                .color(Palette::TEXT_SOFT),
            );
            ui.add_space(16.0);
            ui.horizontal_wrapped(|ui| {
                casework_step(ui, "01", "Select", "Choose a local image copy.");
                ui.separator();
                casework_step(ui, "02", "Review", "Read method and validation details.");
                ui.separator();
                casework_step(ui, "03", "Export", "Create a recovered copy with a receipt.");
            });
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        [196.0, 38.0],
                        egui::Button::new(
                            egui::RichText::new("Choose recovery image…")
                                .strong()
                                .color(Palette::INK),
                        )
                        .fill(Palette::FOCUS_STRONG),
                    )
                    .clicked()
                {
                    app.choose_image();
                }
                if ui.button("Open guided demo").clicked() {
                    app.load_demo_fixture();
                    app.start_scan();
                }
            });
            ui.add_space(6.0);
            ui.small("The guided demo contains harmless synthetic data and never touches a real device.");
        });
}

fn casework_step(ui: &mut egui::Ui, number: &str, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(number)
                .monospace()
                .strong()
                .color(Palette::FOCUS),
        );
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).strong());
            ui.label(
                egui::RichText::new(detail)
                    .small()
                    .color(Palette::TEXT_MUTED),
            );
        });
    });
}

fn workflow_state_label(app: &EvidenceForgeApp) -> (&'static str, egui::Color32) {
    if app.scan_is_stopping() {
        return ("STOPPING SCAN", Palette::WARNING);
    }
    if app.scan_worker.is_some() {
        return ("SCAN IN PROGRESS", Palette::FOCUS_STRONG);
    }
    match app.source_integrity.as_ref() {
        Some(SourceIntegrity::Verified) => ("SOURCE VERIFIED", Palette::SUCCESS_STRONG),
        Some(SourceIntegrity::Changed { .. }) => ("SOURCE CHANGED", Palette::WARNING),
        Some(SourceIntegrity::Unavailable { .. }) => ("SOURCE UNAVAILABLE", Palette::ERROR),
        None if !app.image_path.trim().is_empty() => ("IMAGE READY", Palette::INFO_STRONG),
        None => ("START HERE", Palette::INFO_STRONG),
    }
}

fn active_scan_presentation(stopping: bool) -> (&'static str, &'static str, egui::Color32) {
    if stopping {
        (
            "Stop requested",
            "DiskTrace will acknowledge the request at an implemented cooperative checkpoint. The pending scan result will not be applied, and any completed catalogue remains available.",
            Palette::WARNING,
        )
    } else {
        (
            "Read-only scan active",
            "DiskTrace is performing its implemented local scan stages. A byte-percentage is intentionally not shown because it is not yet a tested scan-progress contract.",
            Palette::FOCUS_STRONG,
        )
    }
}

fn candidate_evidence_presentation(candidate: &RecoveryCandidate) -> (&'static str, &'static str) {
    let basis = match candidate.method {
        RecoveryMethod::Fat12DeletedRootMetadata
        | RecoveryMethod::Fat16DeletedRootMetadata
        | RecoveryMethod::ExfatDeletedContiguousRootMetadata
        | RecoveryMethod::NtfsDeletedResidentRecord
        | RecoveryMethod::NtfsDeletedContiguousNonresident => {
            "The listed source range was identified through the supported filesystem metadata and allocation checks for this method."
        }
        RecoveryMethod::SignatureCarvingPng
        | RecoveryMethod::SignatureCarvingJpeg
        | RecoveryMethod::SignatureCarvingGif
        | RecoveryMethod::SignatureCarvingAvi
        | RecoveryMethod::SignatureCarvingMp4
        | RecoveryMethod::SignatureCarvingPdf
        | RecoveryMethod::SignatureCarvingZipOffice => {
            "The listed source range was identified through the supported structural checks for this file format."
        }
    };
    let validation = match candidate.validation {
        CandidateValidation::MetadataVerified => {
            "The available metadata checks passed within this method's documented scope."
        }
        CandidateValidation::ContentValidated => {
            "The supported content or structure checks accepted this bounded byte range."
        }
        CandidateValidation::RecoveredUnvalidated => {
            "Recovery produced a bounded byte range, but its content needs independent review."
        }
        CandidateValidation::PartialOrErrorAffected => {
            "The result may be incomplete or affected by a recorded recovery condition."
        }
        CandidateValidation::Unavailable => {
            "This source cannot safely provide a recoverable result through the selected method."
        }
    };
    (basis, validation)
}

fn status_badge(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    let bg = color.gamma_multiply(0.16);
    let text_color = if color == Palette::WARNING || color == Palette::WARNING_STRONG {
        Palette::INK
    } else {
        Palette::TEXT
    };
    egui::Frame::NONE
        .fill(bg)
        .stroke(egui::Stroke::new(1.0_f32, color.gamma_multiply(0.7)))
        .corner_radius(egui::CornerRadius::same(5))
        .inner_margin(egui::Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).size(10.0).strong().color(text_color));
        });
}

fn action_guidance_panel(ui: &mut egui::Ui, title: &str, detail: &str, color: egui::Color32) {
    let bg = color.gamma_multiply(0.14);
    let text_color = if color == Palette::WARNING || color == Palette::WARNING_STRONG {
        Palette::INK
    } else {
        Palette::TEXT
    };
    egui::Frame::NONE
        .fill(bg)
        .stroke(egui::Stroke::new(1.0_f32, color.gamma_multiply(0.75)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(11))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).strong().color(text_color));
            ui.small(egui::RichText::new(detail).color(Palette::TEXT_SOFT));
        });
}

fn workspace_empty_panel(ui: &mut egui::Ui, title: &str, detail: &str, color: egui::Color32) {
    let text_color = if color == Palette::WARNING || color == Palette::WARNING_STRONG {
        Palette::INK
    } else {
        Palette::TEXT
    };
    egui::Frame::NONE
        .fill(Palette::SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, color.gamma_multiply(0.7)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).strong().color(text_color));
            ui.add_space(4.0);
            ui.small(egui::RichText::new(detail).color(Palette::TEXT_SOFT));
        });
}

fn workflow_steps_panel(ui: &mut egui::Ui, app: &EvidenceForgeApp) {
    let completed_scan = app.session_manifest.is_some();
    let active_step = if app.scan_worker.is_some() {
        2
    } else if completed_scan {
        3
    } else if app.image_path.trim().is_empty() {
        1
    } else {
        2
    };

    ui.label(
        egui::RichText::new("Recovery workflow")
            .size(11.0)
            .strong()
            .color(Palette::TEXT_SOFT),
    );
    ui.add_space(5.0);
    for (step, title, detail) in [
        (1, "Select image", "Choose a local image copy"),
        (2, "Scan and review", "Read methods and limitations"),
        (3, "Export safely", "Write only to a separate folder"),
    ] {
        let complete =
            (step == 1 && !app.image_path.trim().is_empty()) || (step == 2 && completed_scan);
        let active = active_step == step;
        let _step_color = if complete {
            Palette::SUCCESS_STRONG
        } else if active {
            Palette::FOCUS_STRONG
        } else {
            Palette::TEXT_MUTED
        };
        let text_color = if complete {
            Palette::INK
        } else {
            Palette::TEXT
        };
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(if complete {
                    "OK".to_owned()
                } else {
                    format!("0{step}")
                })
                .monospace()
                .strong()
                .color(text_color),
            );
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(title).strong().color(if active {
                    Palette::TEXT
                } else if complete {
                    Palette::SUCCESS_STRONG
                } else {
                    Palette::TEXT_SOFT
                }));
                ui.label(
                    egui::RichText::new(detail)
                        .small()
                        .color(Palette::TEXT_MUTED),
                );
            });
        });
        if step != 3 {
            ui.add_space(5.0);
        }
    }
}

fn session_workspace_panel(
    ui: &mut egui::Ui,
    manifest: &SessionManifest,
    manifest_path: Option<&std::path::Path>,
    source_integrity: Option<&SourceIntegrity>,
    export_audit: Option<&[RecordedExportVerification]>,
) -> (bool, bool, bool) {
    let (integrity_label, integrity_color, integrity_detail) = match source_integrity {
        Some(SourceIntegrity::Verified) => (
            "Source verified",
            Palette::SUCCESS,
            "Current byte length, SHA-256, and BLAKE3 match this session.",
        ),
        Some(SourceIntegrity::Changed { .. }) => (
            "Source changed — recovery blocked",
            Palette::WARNING,
            "This catalogue is historical. Scan the changed image as a new session before exporting.",
        ),
        Some(SourceIntegrity::Unavailable { .. }) => (
            "Source unavailable — recovery blocked",
            Palette::ERROR,
            "The catalogue and export history remain readable, but a source must be verified before exporting.",
        ),
        None => (
            "Source status not checked",
            Palette::WARNING,
            "Verify the source before attempting a recovery export.",
        ),
    };

    let mut recheck_requested = false;
    let mut audit_requested = false;
    let mut case_brief_requested = false;
    egui::Frame::NONE
        .fill(Palette::SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, Palette::LINE))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Evidence session").strong());
                ui.colored_label(integrity_color, integrity_label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Save case brief").clicked() {
                        case_brief_requested = true;
                    }
                    ui.add_space(8.0);
                    if !manifest.exports.is_empty() && ui.small_button("Audit exports").clicked() {
                        audit_requested = true;
                    }
                    if !manifest.exports.is_empty() {
                        ui.add_space(8.0);
                    }
                    if ui.small_button("Recheck source").clicked() {
                        recheck_requested = true;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("{} exported", manifest.exports.len()))
                            .small()
                            .color(Palette::TEXT_MUTED),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("{} candidates", manifest.candidates.len()))
                            .small()
                            .color(Palette::TEXT_MUTED),
                    );
                });
            });
            ui.small(integrity_detail);
            ui.add_space(6.0);
            ui.collapsing("Case record", |ui| {
                egui::Grid::new("session_workspace_metadata")
                    .num_columns(2)
                    .spacing([18.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Session ID");
                        ui.monospace(manifest.session.id.to_string());
                        ui.end_row();
                        ui.label("Source image");
                        ui.monospace(
                            manifest
                                .session
                                .source
                                .identity
                                .canonical_path
                                .display()
                                .to_string(),
                        );
                        ui.end_row();
                        ui.label("Local manifest");
                        match manifest_path {
                            Some(path) => ui.monospace(path.display().to_string()),
                            None => ui.label("Not saved yet"),
                        };
                        ui.end_row();
                    });
            });
            if !manifest.exports.is_empty() {
                ui.collapsing("Export record", |ui| {
                    for recorded in &manifest.exports {
                        ui.group(|ui| {
                            ui.monospace(&recorded.candidate_id);
                            ui.label(format!("Saved {}", recorded.output_path.display()));
                            let artifact_hash = recorded
                                .receipt
                                .artifacts
                                .first()
                                .map(|artifact| format!("{}…", &artifact.sha256[..12]))
                                .unwrap_or_else(|| "No artifact hash recorded".to_owned());
                            ui.small(format!(
                                "Receipt {} • SHA-256 {artifact_hash}",
                                recorded.receipt_path.display()
                            ));
                        });
                    }
                });
            }
            if let Some(audit) = export_audit {
                ui.collapsing("Export audit", |ui| {
                    for result in audit {
                        let (label, color, detail) = export_audit_presentation(&result.integrity);
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.monospace(&result.candidate_id);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| ui.colored_label(color, label),
                                );
                            });
                            ui.small(format!("{} • {detail}", result.output_path.display()));
                        });
                    }
                });
            }
        });
    (recheck_requested, audit_requested, case_brief_requested)
}

fn export_audit_presentation(
    integrity: &RecordedExportIntegrity,
) -> (&'static str, egui::Color32, &'static str) {
    match integrity {
        RecordedExportIntegrity::Verified => (
            "Verified",
            Palette::SUCCESS,
            "Receipt and current SHA-256/BLAKE3 match",
        ),
        RecordedExportIntegrity::ReceiptUnavailable { .. } => (
            "Receipt unavailable",
            Palette::ERROR,
            "The persisted receipt could not be read",
        ),
        RecordedExportIntegrity::ReceiptChanged => (
            "Receipt changed",
            Palette::WARNING,
            "The persisted receipt differs from the session record",
        ),
        RecordedExportIntegrity::ReceiptInconsistent { .. } => (
            "Receipt inconsistent",
            Palette::WARNING,
            "The receipt does not safely describe this export",
        ),
        RecordedExportIntegrity::ArtifactUnavailable { .. } => (
            "Artifact unavailable",
            Palette::ERROR,
            "The recovered output could not be read",
        ),
        RecordedExportIntegrity::ArtifactChanged { .. } => (
            "Artifact changed",
            Palette::WARNING,
            "The current output hashes differ from its receipt",
        ),
    }
}

fn notice_panel(ui: &mut egui::Ui, notice: &Notice) {
    let color = match notice.tone {
        NoticeTone::Information => Palette::INFO,
        NoticeTone::Success => Palette::SUCCESS,
        NoticeTone::Warning => Palette::WARNING,
        NoticeTone::Error => Palette::ERROR,
    };
    egui::Frame::NONE
        .fill(color.gamma_multiply(0.18))
        .stroke(egui::Stroke::new(1.0_f32, color))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.colored_label(color, &notice.title);
            ui.label(&notice.detail);
        });
}

fn method_filter_label(filter: &MethodFilter) -> &'static str {
    match filter {
        MethodFilter::All => "All methods",
        MethodFilter::Fat12 => "Deleted FAT12 metadata",
        MethodFilter::Fat16 => "Deleted FAT16 metadata",
        MethodFilter::Exfat => "Deleted exFAT contiguous metadata",
        MethodFilter::Ntfs => "Deleted NTFS resident records",
        MethodFilter::NtfsContiguous => "Deleted NTFS contiguous metadata",
        MethodFilter::Png => "PNG signature carving",
        MethodFilter::Jpeg => "JPEG signature carving",
        MethodFilter::Gif => "GIF structural carving",
        MethodFilter::Avi => "AVI structural carving",
        MethodFilter::Mp4 => "MP4 and MOV structural carving",
        MethodFilter::Pdf => "PDF structural carving",
        MethodFilter::ZipOffice => "ZIP and Open XML carving",
    }
}

fn validation_filter_label(filter: &ValidationFilter) -> &'static str {
    match filter {
        ValidationFilter::All => "All results",
        ValidationFilter::Checked => "Recovered and checked",
        ValidationFilter::Review => "Review recommended",
    }
}

fn method_color(method: RecoveryMethod) -> egui::Color32 {
    match method {
        RecoveryMethod::Fat12DeletedRootMetadata => Palette::METHOD_FAT12,
        RecoveryMethod::Fat16DeletedRootMetadata => Palette::METHOD_FAT16,
        RecoveryMethod::ExfatDeletedContiguousRootMetadata => Palette::METHOD_EXFAT,
        RecoveryMethod::NtfsDeletedResidentRecord => Palette::METHOD_NTFS,
        RecoveryMethod::NtfsDeletedContiguousNonresident => Palette::METHOD_NTFS_CONTIGUOUS,
        RecoveryMethod::SignatureCarvingPng => Palette::METHOD_PNG,
        RecoveryMethod::SignatureCarvingJpeg => Palette::METHOD_JPEG,
        RecoveryMethod::SignatureCarvingGif => Palette::METHOD_GIF,
        RecoveryMethod::SignatureCarvingAvi => Palette::METHOD_AVI,
        RecoveryMethod::SignatureCarvingMp4 => Palette::METHOD_MP4,
        RecoveryMethod::SignatureCarvingPdf => Palette::METHOD_PDF,
        RecoveryMethod::SignatureCarvingZipOffice => Palette::METHOD_ZIP_OFFICE,
    }
}

fn validation_color(validation: CandidateValidation) -> egui::Color32 {
    match validation {
        CandidateValidation::ContentValidated | CandidateValidation::MetadataVerified => {
            Palette::SUCCESS
        }
        CandidateValidation::RecoveredUnvalidated | CandidateValidation::PartialOrErrorAffected => {
            Palette::WARNING
        }
        CandidateValidation::Unavailable => Palette::ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_scan_presentation, candidate_evidence_presentation, workflow_state_label,
        EvidenceForgeApp, Palette, PreviewWorker, RecordedExportIntegrity,
        RecordedExportVerification, SourceIntegrity,
    };
    use ef_catalogue::PreviewKind;
    use ef_core::{CandidateValidation, RecoveryMethod};
    use ef_workflow::SessionManifest;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    };
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "evidenceforge-desktop-{name}-{}-{timestamp}",
            std::process::id()
        ))
    }

    fn wait_for_preview(application: &mut EvidenceForgeApp) {
        for _ in 0..200 {
            application.poll_preview_worker();
            if application.preview_worker.is_none() {
                return;
            }
            thread::sleep(Duration::from_millis(2));
        }
        panic!("preview worker did not complete");
    }

    fn wait_for_scan(application: &mut EvidenceForgeApp) {
        for _ in 0..2_500 {
            application.poll_scan_worker();
            if application.scan_worker.is_none() {
                return;
            }
            thread::sleep(Duration::from_millis(2));
        }
        panic!("scan worker did not complete within the five-second test timeout");
    }

    #[test]
    fn background_scan_applies_the_fixture_result() {
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();

        wait_for_scan(&mut application);

        assert!(application.scan_worker.is_none());
        assert_eq!(application.candidates.len(), 2);
        assert_eq!(
            application
                .notice
                .as_ref()
                .map(|notice| notice.title.as_str()),
            Some("Read-only scan complete")
        );
    }

    #[test]
    fn cancelling_selected_preview_signals_the_worker_and_discards_its_result() {
        let mut application = EvidenceForgeApp::default();
        let candidate_id = "efc1-preview-test".to_owned();
        let cancellation = Arc::new(AtomicBool::new(false));
        let (_sender, receiver) = mpsc::channel::<Result<Vec<u8>, String>>();
        application.selected_id = Some(candidate_id.clone());
        application.preview_worker = Some(PreviewWorker {
            generation: application.preview_generation,
            candidate_id,
            cancellation: Arc::clone(&cancellation),
            receiver,
        });

        application.cancel_preview_worker();

        assert!(cancellation.load(Ordering::Relaxed));
        assert!(application.preview_worker.is_none());
        assert!(application.selected_preview_error().is_none());
    }

    #[test]
    fn background_scan_applies_document_candidates() {
        let mut application = EvidenceForgeApp::default();
        application.load_document_fixture();
        application.start_scan();

        wait_for_scan(&mut application);

        assert!(application.scan_worker.is_none());
        assert_eq!(application.candidates.len(), 2);
        assert!(application.candidates[0]
            .id
            .starts_with("efc1-signature_carving_pdf-"));
        assert_eq!(application.candidates[1].file_type, "docx");
        assert!(application
            .presentations
            .iter()
            .all(|presentation| presentation.preview.kind == PreviewKind::MetadataOnly));
        let pdf_id = application.candidates[0].id.clone();
        application.select_result(pdf_id);
        assert!(application.selected_preview_is_loading());
        wait_for_preview(&mut application);
        let pdf = application
            .selected_presentation()
            .expect("selected pdf presentation");
        assert_eq!(pdf.preview.kind, PreviewKind::StructureSummary);
        assert!(pdf.preview.facts.iter().any(|fact| fact.label == "Format"));
        let docx_id = application
            .presentations
            .iter()
            .find(|presentation| presentation.candidate.file_type == "docx")
            .expect("docx presentation")
            .candidate
            .id
            .clone();
        application.select_result(docx_id);
        wait_for_preview(&mut application);
        let docx = application
            .selected_presentation()
            .expect("selected docx presentation");
        assert_eq!(docx.preview.kind, PreviewKind::StructureSummary);
        assert!(docx
            .preview
            .facts
            .iter()
            .any(|fact| fact.label == "Central-directory entries"));
    }

    #[test]
    fn background_scan_applies_media_candidates_with_structure_summaries() {
        let mut application = EvidenceForgeApp::default();
        application.load_media_fixture();
        application.start_scan();

        wait_for_scan(&mut application);

        assert!(application.scan_worker.is_none());
        assert_eq!(application.candidates.len(), 3);
        assert!(application.candidates[0]
            .id
            .starts_with("efc1-signature_carving_gif-"));
        assert!(application.candidates[1]
            .id
            .starts_with("efc1-signature_carving_avi-"));
        assert!(application.candidates[2]
            .id
            .starts_with("efc1-signature_carving_mp4-"));
        assert!(application
            .presentations
            .iter()
            .all(|presentation| presentation.preview.kind == PreviewKind::MetadataOnly));
        let media_ids: Vec<_> = application
            .candidates
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect();
        for (candidate_id, fact_label) in
            media_ids
                .iter()
                .zip(["Logical screen", "Required lists", "Top-level boxes"])
        {
            application.select_result(candidate_id.clone());
            wait_for_preview(&mut application);
            let presentation = application
                .selected_presentation()
                .expect("selected media presentation");
            assert_eq!(presentation.preview.kind, PreviewKind::StructureSummary);
            assert!(presentation
                .preview
                .facts
                .iter()
                .any(|fact| fact.label == fact_label));
        }
    }

    #[test]
    fn background_scan_applies_exfat_candidate() {
        let mut application = EvidenceForgeApp::default();
        application.load_exfat_fixture();
        application.start_scan();

        wait_for_scan(&mut application);

        assert!(application.scan_worker.is_none());
        assert_eq!(application.candidates.len(), 1);
        assert!(application.candidates[0]
            .id
            .starts_with("efc1-exfat_deleted_contiguous_root_metadata-"));
        assert_eq!(application.candidates[0].evidence_name, "recover.txt");
    }

    #[test]
    fn background_scan_applies_ntfs_contiguous_candidate() {
        let mut application = EvidenceForgeApp::default();
        application.load_ntfs_contiguous_fixture();
        application.start_scan();

        wait_for_scan(&mut application);

        assert!(application.scan_worker.is_none());
        assert_eq!(application.candidates.len(), 1);
        assert!(application.candidates[0]
            .id
            .starts_with("efc1-ntfs_deleted_contiguous_nonresident-"));
        assert_eq!(application.candidates[0].evidence_name, "extent.txt");
    }

    #[test]
    fn background_scan_applies_ntfs_resident_candidate() {
        let mut application = EvidenceForgeApp::default();
        application.load_ntfs_fixture();
        application.start_scan();

        wait_for_scan(&mut application);

        assert!(application.scan_worker.is_none());
        assert_eq!(application.candidates.len(), 1);
        assert!(application.candidates[0]
            .id
            .starts_with("efc1-ntfs_deleted_resident_record-"));
        assert_eq!(application.candidates[0].evidence_name, "gone.txt");
    }

    #[test]
    fn saves_exports_and_reopens_a_verified_local_session() {
        let root = test_path("saved-session");
        let destination = root.join("destination");
        let manifest_path = root.join("session.json");
        fs::create_dir_all(&destination).expect("create destination");
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();

        for _ in 0..200 {
            application.poll_scan_worker();
            if application.scan_worker.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        application.destination_path = destination.display().to_string();
        application.selected_id = Some(application.candidates[0].id.clone());
        application.recover_selected();
        application
            .save_session(manifest_path.clone())
            .expect("save session");
        let saved = SessionManifest::load(&manifest_path).expect("load saved session");
        let integrity = saved.verify_source();
        let mut reopened = EvidenceForgeApp::default();
        reopened.apply_manifest(saved, Some(manifest_path), integrity);

        assert_eq!(reopened.candidates.len(), 2);
        assert_eq!(
            reopened
                .session_manifest
                .as_ref()
                .map(|manifest| manifest.exports.len()),
            Some(1)
        );
        assert_eq!(reopened.source_integrity, Some(SourceIntegrity::Verified));
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn export_audit_reports_verified_and_changed_recorded_outputs() {
        let root = test_path("export-audit");
        let destination = root.join("destination");
        fs::create_dir_all(&destination).expect("create destination");
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();

        for _ in 0..200 {
            application.poll_scan_worker();
            if application.scan_worker.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        application.destination_path = destination.display().to_string();
        application.selected_id = Some(application.candidates[0].id.clone());
        application.recover_selected();
        application.audit_recorded_exports();

        assert!(matches!(
            application.export_audit.as_deref(),
            Some([RecordedExportVerification {
                integrity: RecordedExportIntegrity::Verified,
                ..
            }])
        ));
        assert_eq!(
            application
                .notice
                .as_ref()
                .map(|notice| notice.title.as_str()),
            Some("Recorded exports verified")
        );

        let output_path = application
            .session_manifest
            .as_ref()
            .and_then(|manifest| manifest.exports.first())
            .expect("recorded export")
            .output_path
            .clone();
        fs::write(output_path, b"changed recovered output").expect("change recovered output");
        application.audit_recorded_exports();

        assert!(matches!(
            application.export_audit.as_deref(),
            Some([RecordedExportVerification {
                integrity: RecordedExportIntegrity::ArtifactChanged { .. },
                ..
            }])
        ));
        assert_eq!(
            application
                .notice
                .as_ref()
                .map(|notice| notice.title.as_str()),
            Some("Export audit needs review")
        );
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn saves_a_case_brief_from_the_current_local_session() {
        let root = test_path("case-brief");
        let brief_path = root.join("case-brief.md");
        fs::create_dir_all(&root).expect("create test root");
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();

        for _ in 0..200 {
            application.poll_scan_worker();
            if application.scan_worker.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        application
            .save_case_brief(brief_path.clone())
            .expect("save case brief");
        let fat_candidate_id = application.candidates[0].id.clone();
        let brief = fs::read_to_string(&brief_path).expect("read case brief");
        assert!(brief.contains("# DiskTrace case brief"));
        assert!(brief.contains(&fat_candidate_id));
        assert!(brief.contains("No receipt-backed recovery exports are recorded"));
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn recovery_review_requires_verified_source_and_does_not_export_until_confirmed() {
        let root = test_path("recovery-review");
        let destination = root.join("destination");
        fs::create_dir_all(&destination).expect("create destination");
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();

        for _ in 0..200 {
            application.poll_scan_worker();
            if application.scan_worker.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        application.selected_id = Some(application.candidates[0].id.clone());
        application.destination_path = destination.display().to_string();
        application.begin_recovery_review();

        assert!(application.show_recovery_review);
        assert_eq!(
            application
                .session_manifest
                .as_ref()
                .map(|manifest| manifest.exports.len()),
            Some(0)
        );

        application.show_recovery_review = false;
        assert_eq!(
            application
                .session_manifest
                .as_ref()
                .map(|manifest| manifest.exports.len()),
            Some(0)
        );

        application.source_integrity = Some(SourceIntegrity::Unavailable {
            detail: "fixture source is unavailable".to_owned(),
        });
        application.begin_recovery_review();
        assert!(!application.show_recovery_review);
        assert_eq!(
            application
                .notice
                .as_ref()
                .map(|notice| notice.title.as_str()),
            Some("Recovery remains blocked")
        );
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn result_navigation_stays_within_filtered_presentations() {
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();

        for _ in 0..200 {
            application.poll_scan_worker();
            if application.scan_worker.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        let fat_candidate_id = application.candidates[0].id.clone();
        let png_candidate_id = application.candidates[1].id.clone();
        application.select_result_by_offset(1);
        assert_eq!(
            application.selected_id.as_deref(),
            Some(fat_candidate_id.as_str())
        );
        application.select_result_by_offset(1);
        assert_eq!(
            application.selected_id.as_deref(),
            Some(png_candidate_id.as_str())
        );
        application.select_result_by_offset(1);
        assert_eq!(
            application.selected_id.as_deref(),
            Some(png_candidate_id.as_str())
        );

        application.method_filter = super::MethodFilter::Png;
        application.refresh_catalogue();
        application.select_result_by_offset(-1);
        assert_eq!(
            application.selected_id.as_deref(),
            Some(png_candidate_id.as_str())
        );
    }

    #[test]
    fn source_recheck_refreshes_integrity_without_discarding_candidates() {
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();

        for _ in 0..200 {
            application.poll_scan_worker();
            if application.scan_worker.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        let candidate_count = application.candidates.len();
        let candidate_id = application.candidates[0].id.clone();
        application.select_result(candidate_id);
        wait_for_preview(&mut application);
        assert_eq!(
            application
                .selected_presentation()
                .expect("selected candidate")
                .preview
                .kind,
            PreviewKind::TextExcerpt
        );
        application.recheck_source_integrity();
        assert_eq!(
            application.source_integrity,
            Some(SourceIntegrity::Verified)
        );
        assert_eq!(
            application
                .notice
                .as_ref()
                .map(|notice| notice.title.as_str()),
            Some("Source remains verified")
        );

        let unavailable_path = test_path("unavailable-source").join("missing.img");
        application
            .session_manifest
            .as_mut()
            .expect("completed session")
            .session
            .source
            .identity
            .canonical_path = unavailable_path;
        application.recheck_source_integrity();

        assert!(matches!(
            application.source_integrity,
            Some(SourceIntegrity::Unavailable { .. })
        ));
        assert_eq!(application.candidates.len(), candidate_count);
        assert!(application.preview_worker.is_none());
        assert!(application
            .selected_preview_error()
            .is_some_and(|error| error.contains("source is not verified")));
        assert_eq!(
            application
                .selected_presentation()
                .expect("selected candidate")
                .preview
                .kind,
            PreviewKind::MetadataOnly
        );
        assert_eq!(
            application
                .notice
                .as_ref()
                .map(|notice| notice.title.as_str()),
            Some("Source unavailable — recovery remains blocked")
        );
    }

    #[test]
    fn unverified_source_withholds_selected_preview_bytes() {
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();
        wait_for_scan(&mut application);
        let candidate_id = application.candidates[0].id.clone();
        application.source_integrity = Some(SourceIntegrity::Unavailable {
            detail: "fixture source unavailable".to_owned(),
        });

        application.select_result(candidate_id);

        assert!(application.preview_worker.is_none());
        assert!(application
            .selected_preview_error()
            .is_some_and(|error| error.contains("source is not verified")));
        assert_eq!(
            application
                .selected_presentation()
                .expect("selected candidate")
                .preview
                .kind,
            PreviewKind::MetadataOnly
        );
    }

    #[test]
    fn selected_preview_rechecks_source_identity_before_reading_a_range() {
        let root = test_path("preview-source-substitution");
        fs::create_dir_all(&root).expect("create test root");
        let replacement_source = root.join("source.img");
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();
        wait_for_scan(&mut application);
        let original_source = PathBuf::from(application.image_path.trim());
        fs::copy(&original_source, &replacement_source).expect("copy fixture source");
        application
            .session_manifest
            .as_mut()
            .expect("completed session")
            .session
            .source
            .identity
            .canonical_path = replacement_source
            .canonicalize()
            .expect("canonicalize copied source");
        let source_length = fs::metadata(&replacement_source)
            .expect("inspect copied source")
            .len() as usize;
        fs::write(&replacement_source, vec![0_u8; source_length])
            .expect("substitute same-length source contents");
        let candidate_id = application.candidates[0].id.clone();

        application.select_result(candidate_id);
        wait_for_preview(&mut application);

        assert!(application.preview_worker.is_none());
        assert!(application
            .selected_preview_error()
            .is_some_and(|error| error.contains("no longer matches the expected identity")));
        assert_eq!(
            application
                .selected_presentation()
                .expect("selected candidate")
                .preview
                .kind,
            PreviewKind::MetadataOnly
        );
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn current_preview_worker_failure_remains_visible() {
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();
        wait_for_scan(&mut application);
        let candidate_id = application.candidates[0].id.clone();
        application.selected_id = Some(candidate_id.clone());
        let cancellation = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel();
        application.preview_worker = Some(PreviewWorker {
            generation: application.preview_generation,
            candidate_id,
            cancellation,
            receiver,
        });
        sender
            .send(Err("fixture preview read failed".to_owned()))
            .expect("worker receiver remains available");

        application.poll_preview_worker();

        assert!(application.preview_worker.is_none());
        assert_eq!(
            application.selected_preview_error(),
            Some("fixture preview read failed")
        );
        assert_eq!(
            application
                .selected_presentation()
                .expect("selected candidate")
                .preview
                .kind,
            PreviewKind::MetadataOnly
        );
    }

    #[test]
    fn active_scan_presentation_explains_truthful_scan_and_stop_states() {
        let (active_title, active_detail, active_color) = active_scan_presentation(false);
        assert_eq!(active_title, "Read-only scan active");
        assert!(active_detail.contains("local scan stages"));
        assert!(active_detail.contains("not yet a tested scan-progress contract"));
        assert_eq!(active_color, Palette::FOCUS_STRONG);

        let (stopping_title, stopping_detail, stopping_color) = active_scan_presentation(true);
        assert_eq!(stopping_title, "Stop requested");
        assert!(stopping_detail.contains("cooperative checkpoint"));
        assert!(stopping_detail.contains("will not be applied"));
        assert_eq!(stopping_color, Palette::WARNING);
    }

    #[test]
    fn candidate_evidence_presentation_distinguishes_metadata_and_carving_scope() {
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();
        wait_for_scan(&mut application);

        let metadata_candidate = application
            .candidates
            .iter()
            .find(|candidate| candidate.method == RecoveryMethod::Fat12DeletedRootMetadata)
            .expect("FAT12 fixture candidate");
        let (metadata_basis, metadata_validation) =
            candidate_evidence_presentation(metadata_candidate);
        assert!(metadata_basis.contains("filesystem metadata and allocation checks"));
        assert!(metadata_validation.contains("bounded byte range"));

        let carved_candidate = application
            .candidates
            .iter()
            .find(|candidate| candidate.method == RecoveryMethod::SignatureCarvingPng)
            .expect("PNG fixture candidate");
        let (carved_basis, carved_validation) = candidate_evidence_presentation(carved_candidate);
        assert!(carved_basis.contains("structural checks"));
        assert!(carved_validation.contains("bounded byte range"));

        let mut review_candidate = carved_candidate.clone();
        review_candidate.validation = CandidateValidation::RecoveredUnvalidated;
        let (_, review_validation) = candidate_evidence_presentation(&review_candidate);
        assert!(review_validation.contains("independent review"));
    }

    #[test]
    fn workflow_state_tracks_image_and_scan_progress() {
        let mut application = EvidenceForgeApp::default();
        assert_eq!(workflow_state_label(&application).0, "START HERE");

        application.load_demo_fixture();
        assert_eq!(workflow_state_label(&application).0, "IMAGE READY");

        application.start_scan();
        assert_eq!(workflow_state_label(&application).0, "SCAN IN PROGRESS");

        application.cancel_scan();
        assert_eq!(workflow_state_label(&application).0, "STOPPING SCAN");
    }

    #[test]
    fn cancelling_a_pending_scan_preserves_the_previous_catalogue() {
        let mut application = EvidenceForgeApp::default();
        application.load_demo_fixture();
        application.start_scan();
        application.cancel_scan();

        assert!(application.scan_worker.is_some());
        assert!(application.scan_is_stopping());
        assert!(application.candidates.is_empty());
        wait_for_scan(&mut application);

        assert!(application.scan_worker.is_none());
        assert!(application.candidates.is_empty());
        assert!(matches!(
            application
                .notice
                .as_ref()
                .map(|notice| notice.title.as_str()),
            Some("Scan stopped") | Some("Scan result discarded")
        ));
    }
}
