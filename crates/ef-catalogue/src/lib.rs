use ef_core::{CandidateValidation, RecoveryCandidate, RecoveryMethod};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueQuery {
    pub text: Option<String>,
    pub methods: Vec<RecoveryMethod>,
    pub validations: Vec<CandidateValidation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueSummary {
    pub total_candidates: usize,
    pub metadata_candidates: usize,
    pub carved_candidates: usize,
    pub content_validated_candidates: usize,
    pub review_recommended_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateCatalogue {
    pub summary: CatalogueSummary,
    pub candidates: Vec<RecoveryCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewKind {
    MetadataOnly,
    TextExcerpt,
    StructureSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewFact {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePreview {
    pub kind: PreviewKind,
    pub byte_length: u64,
    pub source_offset: u64,
    pub text_excerpt: Option<String>,
    pub facts: Vec<PreviewFact>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePresentation {
    pub candidate: RecoveryCandidate,
    pub method_label: String,
    pub validation_label: String,
    pub explanation: String,
    pub preview: CandidatePreview,
}

pub fn present_candidate(
    candidate: RecoveryCandidate,
    recovered_bytes: Option<&[u8]>,
) -> CandidatePresentation {
    let preview = preview_for(&candidate, recovered_bytes);
    CandidatePresentation {
        method_label: method_label(candidate.method).to_owned(),
        validation_label: validation_label(candidate.validation).to_owned(),
        explanation: explanation_for(&candidate),
        candidate,
        preview,
    }
}

pub fn build_catalogue(
    candidates: impl IntoIterator<Item = RecoveryCandidate>,
    query: &CatalogueQuery,
) -> CandidateCatalogue {
    let mut candidates: Vec<_> = candidates
        .into_iter()
        .filter(|candidate| matches_query(candidate, query))
        .collect();
    candidates.sort_by(|left, right| {
        left.source_offset
            .cmp(&right.source_offset)
            .then_with(|| left.id.cmp(&right.id))
    });
    let summary = CatalogueSummary {
        total_candidates: candidates.len(),
        metadata_candidates: candidates
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.method,
                    RecoveryMethod::Fat12DeletedRootMetadata
                        | RecoveryMethod::Fat16DeletedRootMetadata
                        | RecoveryMethod::ExfatDeletedContiguousRootMetadata
                        | RecoveryMethod::NtfsDeletedResidentRecord
                        | RecoveryMethod::NtfsDeletedContiguousNonresident
                )
            })
            .count(),
        carved_candidates: candidates
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.method,
                    RecoveryMethod::SignatureCarvingPng
                        | RecoveryMethod::SignatureCarvingJpeg
                        | RecoveryMethod::SignatureCarvingGif
                        | RecoveryMethod::SignatureCarvingAvi
                        | RecoveryMethod::SignatureCarvingMp4
                        | RecoveryMethod::SignatureCarvingPdf
                        | RecoveryMethod::SignatureCarvingZipOffice
                )
            })
            .count(),
        content_validated_candidates: candidates
            .iter()
            .filter(|candidate| candidate.validation == CandidateValidation::ContentValidated)
            .count(),
        review_recommended_candidates: candidates
            .iter()
            .filter(|candidate| candidate.validation == CandidateValidation::RecoveredUnvalidated)
            .count(),
    };

    CandidateCatalogue {
        summary,
        candidates,
    }
}

fn matches_query(candidate: &RecoveryCandidate, query: &CatalogueQuery) -> bool {
    if let Some(text) = query.text.as_deref().filter(|text| !text.trim().is_empty()) {
        let search_text = text.to_ascii_lowercase();
        let indexed = format!(
            "{} {} {} {} {}",
            candidate.id,
            candidate.evidence_name,
            candidate.file_type,
            method_key(candidate.method),
            validation_key(candidate.validation)
        )
        .to_ascii_lowercase();
        if !indexed.contains(&search_text) {
            return false;
        }
    }
    if !query.methods.is_empty() && !query.methods.contains(&candidate.method) {
        return false;
    }
    if !query.validations.is_empty() && !query.validations.contains(&candidate.validation) {
        return false;
    }
    true
}

pub fn method_key(method: RecoveryMethod) -> &'static str {
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

pub fn validation_key(validation: CandidateValidation) -> &'static str {
    match validation {
        CandidateValidation::MetadataVerified => "metadata_verified",
        CandidateValidation::ContentValidated => "content_validated",
        CandidateValidation::RecoveredUnvalidated => "recovered_unvalidated",
        CandidateValidation::PartialOrErrorAffected => "partial_or_error_affected",
        CandidateValidation::Unavailable => "unavailable",
    }
}

fn method_label(method: RecoveryMethod) -> &'static str {
    match method {
        RecoveryMethod::Fat12DeletedRootMetadata => "Recovered from deleted FAT12 metadata",
        RecoveryMethod::Fat16DeletedRootMetadata => "Recovered from deleted FAT16 metadata",
        RecoveryMethod::ExfatDeletedContiguousRootMetadata => {
            "Recovered from deleted exFAT contiguous metadata"
        }
        RecoveryMethod::NtfsDeletedResidentRecord => {
            "Recovered from deleted NTFS resident-record metadata"
        }
        RecoveryMethod::NtfsDeletedContiguousNonresident => {
            "Recovered from deleted NTFS contiguous metadata"
        }
        RecoveryMethod::SignatureCarvingPng => "Found by PNG signature carving",
        RecoveryMethod::SignatureCarvingJpeg => "Found by JPEG signature carving",
        RecoveryMethod::SignatureCarvingGif => "Found by GIF structural carving",
        RecoveryMethod::SignatureCarvingAvi => "Found by AVI structural carving",
        RecoveryMethod::SignatureCarvingMp4 => "Found by self-contained MP4/MOV structural carving",
        RecoveryMethod::SignatureCarvingPdf => "Found by PDF structural carving",
        RecoveryMethod::SignatureCarvingZipOffice => "Found by ZIP or Open XML structural carving",
    }
}

fn validation_label(validation: CandidateValidation) -> &'static str {
    match validation {
        CandidateValidation::MetadataVerified => "Likely intact",
        CandidateValidation::ContentValidated => "Recovered and checked",
        CandidateValidation::RecoveredUnvalidated => "Recovered — review recommended",
        CandidateValidation::PartialOrErrorAffected => "May be incomplete",
        CandidateValidation::Unavailable => "Not recoverable with this source",
    }
}

fn explanation_for(candidate: &RecoveryCandidate) -> String {
    match candidate.method {
        RecoveryMethod::Fat12DeletedRootMetadata => {
            "This result was found through a deleted FAT12 directory entry and a readable cluster chain. Deleted metadata can be incomplete, so the original filename or folder may not be available.".to_owned()
        }
        RecoveryMethod::Fat16DeletedRootMetadata => {
            "This result was found through a deleted FAT16 directory entry and a readable cluster chain. Deleted metadata can be incomplete, so the original filename or folder may not be available.".to_owned()
        }
        RecoveryMethod::ExfatDeletedContiguousRootMetadata => {
            "This result was found through a deleted exFAT root-directory entry set with a valid checksum and a contiguous former extent that the active allocation bitmap reports as free. Free allocation does not prove the bytes were not overwritten after deletion, so review recovered content carefully.".to_owned()
        }
        RecoveryMethod::NtfsDeletedResidentRecord => {
            "This result was found in a deleted NTFS Master File Table record after its sector fixups and resident attributes were structurally checked. Only content stored inside that record was recovered; this method does not recover non-resident data, alternate streams, or an original folder path.".to_owned()
        }
        RecoveryMethod::NtfsDeletedContiguousNonresident => {
            "This result was found through a deleted NTFS record with valid sector fixups, one uncompressed contiguous former extent, and a current allocation bitmap that reports every extent cluster as free. Free allocation does not prove the former bytes were not overwritten; fragmented, sparse, compressed, encrypted, named, and partial streams are intentionally not recovered.".to_owned()
        }
        RecoveryMethod::SignatureCarvingPng => {
            "This PNG structure was found directly in raw storage bytes. The image structure passed the supported checks, but its original filename and folder are not available from carving alone.".to_owned()
        }
        RecoveryMethod::SignatureCarvingJpeg => {
            "This JPEG structure was found directly in raw storage bytes. Its supported frame and end markers were present, but carving alone cannot establish the original filename, folder, or whether all original bytes were contiguous.".to_owned()
        }
        RecoveryMethod::SignatureCarvingGif => {
            "This GIF data stream was found directly in raw storage bytes. Its supported header, data blocks, and trailer were structurally consistent, but carving does not decode the animation, establish the original filename or folder, or prove that later storage writes did not replace bytes.".to_owned()
        }
        RecoveryMethod::SignatureCarvingAvi => {
            "This standard AVI container was found directly in raw storage bytes. Its declared RIFF boundary and required header and media lists were structurally consistent, but carving does not decode streams, validate every codec payload, establish the original filename or folder, or support OpenDML extensions.".to_owned()
        }
        RecoveryMethod::SignatureCarvingMp4 => {
            "This self-contained MP4/MOV-style container was found directly in raw storage bytes. Its supported file-type, movie metadata, and media-data boxes were structurally bounded, but carving does not validate playback, codecs, sample offsets, originality, or completeness and intentionally refuses fragmented media.".to_owned()
        }
        RecoveryMethod::SignatureCarvingPdf => {
            "This PDF structure was found directly in raw storage bytes. Its supported header, cross-reference pointer, and end marker were consistent, but carving does not parse every object, prove completeness, or establish the original filename or folder.".to_owned()
        }
        RecoveryMethod::SignatureCarvingZipOffice => {
            "This ZIP-based container was found directly in raw storage bytes. Its supported local headers, central directory, and end record were consistent; an Open XML extension also indicates the expected package parts were present. Carving does not decompress files, validate every document part, or establish the original filename or folder.".to_owned()
        }
    }
}

fn preview_for(candidate: &RecoveryCandidate, recovered_bytes: Option<&[u8]>) -> CandidatePreview {
    if candidate.file_type.eq_ignore_ascii_case("txt") {
        if let Some(bytes) = recovered_bytes {
            if let Ok(text) = std::str::from_utf8(bytes) {
                return CandidatePreview {
                    kind: PreviewKind::TextExcerpt,
                    byte_length: candidate.byte_length,
                    source_offset: candidate.source_offset,
                    text_excerpt: Some(bounded_excerpt(text)),
                    facts: Vec::new(),
                    note: "Text preview is a bounded read-only excerpt of recovered bytes."
                        .to_owned(),
                };
            }
        }
    }

    if let Some(bytes) = recovered_bytes {
        if let Some(facts) = candidate_preview_structure(candidate, bytes) {
            return CandidatePreview {
                kind: PreviewKind::StructureSummary,
                byte_length: candidate.byte_length,
                source_offset: candidate.source_offset,
                text_excerpt: None,
                facts,
                note: "Structure summary is a bounded local inspection of recovered bytes. It does not render, open, decompress, execute, or establish the completeness, authenticity, or safety of this file."
                    .to_owned(),
            };
        }
    }

    CandidatePreview {
        kind: PreviewKind::MetadataOnly,
        byte_length: candidate.byte_length,
        source_offset: candidate.source_offset,
        text_excerpt: None,
        facts: Vec::new(),
        note: "No bounded structure summary is available for these recovered bytes. No external renderer or application was invoked."
            .to_owned(),
    }
}

fn candidate_preview_structure(
    candidate: &RecoveryCandidate,
    bytes: &[u8],
) -> Option<Vec<PreviewFact>> {
    match candidate.method {
        RecoveryMethod::SignatureCarvingPng => png_preview_facts(bytes),
        RecoveryMethod::SignatureCarvingJpeg => jpeg_preview_facts(bytes),
        RecoveryMethod::SignatureCarvingGif => gif_preview_facts(bytes),
        RecoveryMethod::SignatureCarvingAvi => avi_preview_facts(bytes),
        RecoveryMethod::SignatureCarvingMp4 => mp4_preview_facts(bytes, &candidate.file_type),
        RecoveryMethod::SignatureCarvingPdf => pdf_preview_facts(bytes),
        RecoveryMethod::SignatureCarvingZipOffice => zip_preview_facts(bytes, &candidate.file_type),
        _ => None,
    }
}

fn png_preview_facts(bytes: &[u8]) -> Option<Vec<PreviewFact>> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    let header = bytes.get(..29)?;
    if header.get(..8)? != PNG_SIGNATURE
        || header.get(8..12)? != [0, 0, 0, 13]
        || header.get(12..16)? != b"IHDR"
    {
        return None;
    }

    let width = u32::from_be_bytes(header.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(header.get(20..24)?.try_into().ok()?);
    let bit_depth = *header.get(24)?;
    let color_type = *header.get(25)?;
    Some(vec![
        preview_fact("Format", "PNG IHDR"),
        preview_fact("Dimensions", format!("{width} × {height} pixels")),
        preview_fact("Bit depth", bit_depth.to_string()),
        preview_fact("Color type", png_color_type_label(color_type)),
    ])
}

fn jpeg_preview_facts(bytes: &[u8]) -> Option<Vec<PreviewFact>> {
    const MAX_HEADER_BYTES: usize = 64 * 1024;
    if bytes.get(..2)? != [0xff, 0xd8] {
        return None;
    }
    let limit = bytes.len().min(MAX_HEADER_BYTES);
    let mut index = 2;
    while index + 8 < limit {
        if bytes[index] != 0xff {
            index += 1;
            continue;
        }
        let marker = bytes[index + 1];
        if marker == 0x00 || marker == 0xff {
            index += 1;
            continue;
        }
        if is_supported_jpeg_sof(marker) {
            let segment_length = u16::from_be_bytes([bytes[index + 2], bytes[index + 3]]);
            if segment_length < 8 {
                return None;
            }
            let precision = bytes[index + 4];
            let height = u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]);
            let width = u16::from_be_bytes([bytes[index + 7], bytes[index + 8]]);
            return Some(vec![
                preview_fact("Format", format!("JPEG frame marker 0x{marker:02X}")),
                preview_fact("Dimensions", format!("{width} × {height} pixels")),
                preview_fact("Sample precision", format!("{precision} bits")),
            ]);
        }
        index += 2;
    }
    None
}

fn gif_preview_facts(bytes: &[u8]) -> Option<Vec<PreviewFact>> {
    let header = bytes.get(..13)?;
    let version = match header.get(..6)? {
        b"GIF87a" => "GIF87a",
        b"GIF89a" => "GIF89a",
        _ => return None,
    };
    let width = u16::from_le_bytes(header.get(6..8)?.try_into().ok()?);
    let height = u16::from_le_bytes(header.get(8..10)?.try_into().ok()?);
    let global_color_table = header.get(10)? & 0x80 != 0;
    let trailer_present = bytes.last().copied() == Some(0x3b);
    Some(vec![
        preview_fact("Format", version),
        preview_fact("Logical screen", format!("{width} × {height} pixels")),
        preview_fact(
            "Global color table",
            if global_color_table {
                "Present"
            } else {
                "Not present"
            },
        ),
        preview_fact(
            "Trailer",
            if trailer_present {
                "Present"
            } else {
                "Not present"
            },
        ),
    ])
}

fn avi_preview_facts(bytes: &[u8]) -> Option<Vec<PreviewFact>> {
    let header = bytes.get(..12)?;
    if header.get(..4)? != b"RIFF" || header.get(8..12)? != b"AVI " {
        return None;
    }
    let declared_size = u32::from_le_bytes(header.get(4..8)?.try_into().ok()?);
    let has_header_list = bytes.windows(4).any(|window| window == b"hdrl");
    let has_media_list = bytes.windows(4).any(|window| window == b"movi");
    Some(vec![
        preview_fact("Format", "RIFF AVI"),
        preview_fact(
            "Declared container bytes",
            declared_size.checked_add(8)?.to_string(),
        ),
        preview_fact(
            "Required lists",
            if has_header_list && has_media_list {
                "hdrl and movi present"
            } else {
                "Not both present"
            },
        ),
    ])
}

fn mp4_preview_facts(bytes: &[u8], file_type: &str) -> Option<Vec<PreviewFact>> {
    const MAX_BOXES: usize = 32;
    let mut offset = 0;
    let mut boxes = Vec::new();
    let mut movie_metadata = false;
    let mut media_data_bytes = None;
    while offset < bytes.len() && boxes.len() < MAX_BOXES {
        let header = bytes.get(offset..offset.checked_add(8)?)?;
        let size = usize::try_from(u32::from_be_bytes(header.get(..4)?.try_into().ok()?)).ok()?;
        if size < 8 || size == 1 || size == 0 {
            return None;
        }
        let end = offset.checked_add(size)?;
        if end > bytes.len() {
            return None;
        }
        let box_type = std::str::from_utf8(header.get(4..8)?).ok()?.to_owned();
        if box_type == "moov" {
            movie_metadata = true;
        }
        if box_type == "mdat" {
            media_data_bytes = Some(size - 8);
        }
        boxes.push(box_type);
        offset = end;
    }
    if boxes.first().map(String::as_str) != Some("ftyp") {
        return None;
    }

    let container = if file_type.eq_ignore_ascii_case("mov") {
        "QuickTime MOV-style container"
    } else {
        "MP4-style ISO Base Media container"
    };
    let mut facts = vec![
        preview_fact("Format", container),
        preview_fact("Top-level boxes", boxes.join(", ")),
        preview_fact(
            "Movie metadata",
            if movie_metadata {
                "moov present"
            } else {
                "Not present"
            },
        ),
    ];
    if let Some(media_data_bytes) = media_data_bytes {
        facts.push(preview_fact(
            "Media-data payload bytes",
            media_data_bytes.to_string(),
        ));
    }
    Some(facts)
}

fn pdf_preview_facts(bytes: &[u8]) -> Option<Vec<PreviewFact>> {
    const HEADER_BYTES: usize = 64;
    const TRAILER_BYTES: usize = 1024;
    let header = bytes.get(..bytes.len().min(HEADER_BYTES))?;
    let version = header.windows(5).position(|window| window == b"%PDF-")?;
    let version_bytes = header
        .get(version + 5..)?
        .split(|byte| byte.is_ascii_whitespace())
        .next()?;
    let version = std::str::from_utf8(version_bytes).ok()?.trim();
    if version.is_empty() {
        return None;
    }
    let trailer_start = bytes.len().saturating_sub(TRAILER_BYTES);
    let eof_present = bytes[trailer_start..]
        .windows(5)
        .any(|window| window == b"%%EOF");
    Some(vec![
        preview_fact("Format", format!("PDF {version}")),
        preview_fact(
            "Final EOF marker",
            if eof_present {
                "Present within the final 1 KiB"
            } else {
                "Not found within the final 1 KiB"
            },
        ),
    ])
}

fn zip_preview_facts(bytes: &[u8], file_type: &str) -> Option<Vec<PreviewFact>> {
    const MAX_TRAILER_BYTES: usize = 64 * 1024;
    const MAX_CENTRAL_ENTRIES: usize = 32;
    const MAX_DISPLAYED_NAMES: usize = 8;
    let trailer_start = bytes.len().saturating_sub(MAX_TRAILER_BYTES);
    let eocd_offset = bytes[trailer_start..]
        .windows(4)
        .rposition(|window| window == b"PK\x05\x06")?
        + trailer_start;
    let eocd = bytes.get(eocd_offset..eocd_offset + 22)?;
    let entry_count = u16::from_le_bytes(eocd.get(10..12)?.try_into().ok()?);
    let central_size = u32::from_le_bytes(eocd.get(12..16)?.try_into().ok()?) as usize;
    let central_offset = u32::from_le_bytes(eocd.get(16..20)?.try_into().ok()?) as usize;
    let central_end = central_offset.checked_add(central_size)?;
    if central_end > bytes.len() {
        return None;
    }

    let mut names = Vec::new();
    let mut offset = central_offset;
    for _ in 0..usize::from(entry_count).min(MAX_CENTRAL_ENTRIES) {
        let record = bytes.get(offset..offset + 46)?;
        if record.get(..4)? != b"PK\x01\x02" {
            return None;
        }
        let name_length = u16::from_le_bytes(record.get(28..30)?.try_into().ok()?) as usize;
        let extra_length = u16::from_le_bytes(record.get(30..32)?.try_into().ok()?) as usize;
        let comment_length = u16::from_le_bytes(record.get(32..34)?.try_into().ok()?) as usize;
        let name_start = offset + 46;
        let name_end = name_start.checked_add(name_length)?;
        let next_offset = name_end
            .checked_add(extra_length)?
            .checked_add(comment_length)?;
        if next_offset > central_end {
            return None;
        }
        if names.len() < MAX_DISPLAYED_NAMES {
            names.push(bounded_package_name(&bytes[name_start..name_end]));
        }
        offset = next_offset;
    }

    let package = match file_type.to_ascii_lowercase().as_str() {
        "docx" => "Open XML word-processing package",
        "xlsx" => "Open XML spreadsheet package",
        "pptx" => "Open XML presentation package",
        _ => "ZIP container",
    };
    let mut facts = vec![
        preview_fact("Format", package),
        preview_fact("Central-directory entries", entry_count.to_string()),
    ];
    if !names.is_empty() {
        facts.push(preview_fact("Sample package entries", names.join(", ")));
    }
    Some(facts)
}

fn preview_fact(label: impl Into<String>, value: impl Into<String>) -> PreviewFact {
    PreviewFact {
        label: label.into(),
        value: value.into(),
    }
}

fn png_color_type_label(color_type: u8) -> String {
    match color_type {
        0 => "Grayscale".to_owned(),
        2 => "Truecolor".to_owned(),
        3 => "Indexed-color".to_owned(),
        4 => "Grayscale with alpha".to_owned(),
        6 => "Truecolor with alpha".to_owned(),
        _ => format!("Unrecognized code {color_type}"),
    }
}

fn is_supported_jpeg_sof(marker: u8) -> bool {
    matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf)
}

fn bounded_package_name(name: &[u8]) -> String {
    const MAX_NAME_CHARACTERS: usize = 96;
    let mut result = String::new();
    for character in String::from_utf8_lossy(name)
        .chars()
        .take(MAX_NAME_CHARACTERS)
    {
        if character.is_control() {
            result.push(' ');
        } else {
            result.push(character);
        }
    }
    if String::from_utf8_lossy(name).chars().count() > MAX_NAME_CHARACTERS {
        result.push('…');
    }
    result
}

fn bounded_excerpt(text: &str) -> String {
    const MAX_PREVIEW_CHARACTERS: usize = 240;
    let mut excerpt = String::new();
    for character in text.chars().take(MAX_PREVIEW_CHARACTERS) {
        if character.is_control() && character != '\n' && character != '\t' {
            excerpt.push(' ');
        } else {
            excerpt.push(character);
        }
    }
    if text.chars().count() > MAX_PREVIEW_CHARACTERS {
        excerpt.push('…');
    }
    excerpt
}

#[cfg(test)]
mod tests {
    use super::{build_catalogue, present_candidate, CatalogueQuery, PreviewKind};
    use ef_core::{CandidateValidation, RecoveryCandidate, RecoveryMethod};

    fn fat_candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "fat12-root-0000".to_owned(),
            evidence_name: "?ELETED.TXT".to_owned(),
            file_type: "txt".to_owned(),
            source_offset: 1536,
            byte_length: 11,
            method: RecoveryMethod::Fat12DeletedRootMetadata,
            validation: CandidateValidation::RecoveredUnvalidated,
            original_path: None,
        }
    }

    fn png_candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "png-carve-0000".to_owned(),
            evidence_name: "carved-png-0000.png".to_owned(),
            file_type: "png".to_owned(),
            source_offset: 4096,
            byte_length: 70,
            method: RecoveryMethod::SignatureCarvingPng,
            validation: CandidateValidation::ContentValidated,
            original_path: None,
        }
    }

    fn docx_candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "zip-carve-0000".to_owned(),
            evidence_name: "carved-zip-0000.docx".to_owned(),
            file_type: "docx".to_owned(),
            source_offset: 8192,
            byte_length: 704,
            method: RecoveryMethod::SignatureCarvingZipOffice,
            validation: CandidateValidation::ContentValidated,
            original_path: None,
        }
    }

    fn jpeg_candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "jpeg-carve-0000".to_owned(),
            evidence_name: "carved-jpeg-0000.jpg".to_owned(),
            file_type: "jpg".to_owned(),
            source_offset: 12288,
            byte_length: 32,
            method: RecoveryMethod::SignatureCarvingJpeg,
            validation: CandidateValidation::ContentValidated,
            original_path: None,
        }
    }

    fn pdf_candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "pdf-carve-0000".to_owned(),
            evidence_name: "carved-pdf-0000.pdf".to_owned(),
            file_type: "pdf".to_owned(),
            source_offset: 16384,
            byte_length: 32,
            method: RecoveryMethod::SignatureCarvingPdf,
            validation: CandidateValidation::ContentValidated,
            original_path: None,
        }
    }

    fn gif_candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "gif-carve-0000".to_owned(),
            evidence_name: "carved-gif-0000.gif".to_owned(),
            file_type: "gif".to_owned(),
            source_offset: 20480,
            byte_length: 35,
            method: RecoveryMethod::SignatureCarvingGif,
            validation: CandidateValidation::ContentValidated,
            original_path: None,
        }
    }

    fn avi_candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "avi-carve-0000".to_owned(),
            evidence_name: "carved-avi-0000.avi".to_owned(),
            file_type: "avi".to_owned(),
            source_offset: 24576,
            byte_length: 40,
            method: RecoveryMethod::SignatureCarvingAvi,
            validation: CandidateValidation::ContentValidated,
            original_path: None,
        }
    }

    fn mp4_candidate() -> RecoveryCandidate {
        RecoveryCandidate {
            id: "mp4-carve-0000".to_owned(),
            evidence_name: "carved-mp4-0000.mp4".to_owned(),
            file_type: "mp4".to_owned(),
            source_offset: 28672,
            byte_length: 72,
            method: RecoveryMethod::SignatureCarvingMp4,
            validation: CandidateValidation::ContentValidated,
            original_path: None,
        }
    }

    fn preview_value<'a>(
        presentation: &'a super::CandidatePresentation,
        label: &str,
    ) -> Option<&'a str> {
        presentation
            .preview
            .facts
            .iter()
            .find(|fact| fact.label == label)
            .map(|fact| fact.value.as_str())
    }

    fn minimal_open_xml_package() -> Vec<u8> {
        let name = b"word/document.xml";
        let mut central_directory = vec![0; 46];
        central_directory[..4].copy_from_slice(b"PK\x01\x02");
        central_directory[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
        central_directory.extend_from_slice(name);

        let mut end_record = vec![0; 22];
        end_record[..4].copy_from_slice(b"PK\x05\x06");
        end_record[8..10].copy_from_slice(&1_u16.to_le_bytes());
        end_record[10..12].copy_from_slice(&1_u16.to_le_bytes());
        end_record[12..16].copy_from_slice(&(central_directory.len() as u32).to_le_bytes());
        end_record[16..20].copy_from_slice(&0_u32.to_le_bytes());
        central_directory.extend_from_slice(&end_record);
        central_directory
    }

    #[test]
    fn sorts_candidates_by_source_offset_and_summarizes_methods() {
        let catalogue = build_catalogue(
            [png_candidate(), fat_candidate()],
            &CatalogueQuery::default(),
        );

        assert_eq!(catalogue.candidates[0].id, "fat12-root-0000");
        assert_eq!(catalogue.candidates[1].id, "png-carve-0000");
        assert_eq!(catalogue.summary.total_candidates, 2);
        assert_eq!(catalogue.summary.metadata_candidates, 1);
        assert_eq!(catalogue.summary.carved_candidates, 1);
        assert_eq!(catalogue.summary.content_validated_candidates, 1);
        assert_eq!(catalogue.summary.review_recommended_candidates, 1);
    }

    #[test]
    fn searches_case_insensitively_across_user_visible_candidate_fields() {
        let catalogue = build_catalogue(
            [fat_candidate(), png_candidate()],
            &CatalogueQuery {
                text: Some("PNG".to_owned()),
                methods: Vec::new(),
                validations: Vec::new(),
            },
        );

        assert_eq!(catalogue.candidates.len(), 1);
        assert_eq!(catalogue.candidates[0].id, "png-carve-0000");
    }

    #[test]
    fn combines_method_and_validation_filters() {
        let catalogue = build_catalogue(
            [fat_candidate(), png_candidate()],
            &CatalogueQuery {
                text: None,
                methods: vec![RecoveryMethod::SignatureCarvingPng],
                validations: vec![CandidateValidation::ContentValidated],
            },
        );

        assert_eq!(catalogue.candidates.len(), 1);
        assert_eq!(catalogue.candidates[0].id, "png-carve-0000");
    }

    #[test]
    fn filters_and_explains_structurally_validated_open_xml_containers() {
        let catalogue = build_catalogue(
            [fat_candidate(), docx_candidate()],
            &CatalogueQuery {
                text: Some("office".to_owned()),
                methods: vec![RecoveryMethod::SignatureCarvingZipOffice],
                validations: vec![CandidateValidation::ContentValidated],
            },
        );
        let presentation = present_candidate(docx_candidate(), Some(b"not rendered"));

        assert_eq!(catalogue.candidates.len(), 1);
        assert_eq!(catalogue.candidates[0].file_type, "docx");
        assert_eq!(catalogue.summary.carved_candidates, 1);
        assert_eq!(presentation.preview.kind, PreviewKind::MetadataOnly);
        assert!(presentation.explanation.contains("central directory"));
        assert!(presentation.explanation.contains("does not decompress"));
    }

    #[test]
    fn provides_a_bounded_text_excerpt_for_recovered_text() {
        let presentation = present_candidate(fat_candidate(), Some(b"recovered\x00 text\n"));

        assert_eq!(presentation.preview.kind, PreviewKind::TextExcerpt);
        assert_eq!(
            presentation.preview.text_excerpt.as_deref(),
            Some("recovered  text\n")
        );
        assert!(presentation
            .explanation
            .contains("deleted FAT12 directory entry"));
    }

    #[test]
    fn provides_bounded_structure_facts_for_supported_binary_candidates() {
        let png = [
            0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', 0, 0, 0, 13, b'I', b'H', b'D', b'R',
            0, 0, 0, 2, 0, 0, 0, 1, 8, 6, 0, 0, 0,
        ];
        let jpeg = [0xff, 0xd8, 0xff, 0xc0, 0, 8, 8, 0, 2, 0, 3, 3, 0, 0, 0, 0];
        let pdf = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n%%EOF\n";

        let png_presentation = present_candidate(png_candidate(), Some(&png));
        assert_eq!(png_presentation.preview.kind, PreviewKind::StructureSummary);
        assert_eq!(
            preview_value(&png_presentation, "Dimensions"),
            Some("2 × 1 pixels")
        );
        assert_eq!(
            preview_value(&png_presentation, "Color type"),
            Some("Truecolor with alpha")
        );

        let jpeg_presentation = present_candidate(jpeg_candidate(), Some(&jpeg));
        assert_eq!(
            jpeg_presentation.preview.kind,
            PreviewKind::StructureSummary
        );
        assert_eq!(
            preview_value(&jpeg_presentation, "Dimensions"),
            Some("3 × 2 pixels")
        );
        assert_eq!(
            preview_value(&jpeg_presentation, "Sample precision"),
            Some("8 bits")
        );

        let pdf_presentation = present_candidate(pdf_candidate(), Some(pdf));
        assert_eq!(pdf_presentation.preview.kind, PreviewKind::StructureSummary);
        assert_eq!(preview_value(&pdf_presentation, "Format"), Some("PDF 1.7"));
        assert_eq!(
            preview_value(&pdf_presentation, "Final EOF marker"),
            Some("Present within the final 1 KiB")
        );

        let docx_presentation =
            present_candidate(docx_candidate(), Some(&minimal_open_xml_package()));
        assert_eq!(
            docx_presentation.preview.kind,
            PreviewKind::StructureSummary
        );
        assert_eq!(
            preview_value(&docx_presentation, "Format"),
            Some("Open XML word-processing package")
        );
        assert_eq!(
            preview_value(&docx_presentation, "Central-directory entries"),
            Some("1")
        );
        assert_eq!(
            preview_value(&docx_presentation, "Sample package entries"),
            Some("word/document.xml")
        );
    }

    #[test]
    fn provides_bounded_structure_facts_for_gif_and_video_candidates() {
        let gif =
            include_bytes!("../../../fixtures/media-carving-multimethod-v1/expected-carved.gif");
        let avi =
            include_bytes!("../../../fixtures/media-carving-multimethod-v1/expected-carved.avi");
        let mp4 =
            include_bytes!("../../../fixtures/media-carving-multimethod-v1/expected-carved.mp4");

        let gif_presentation = present_candidate(gif_candidate(), Some(gif));
        assert_eq!(gif_presentation.preview.kind, PreviewKind::StructureSummary);
        assert_eq!(preview_value(&gif_presentation, "Format"), Some("GIF89a"));
        assert_eq!(
            preview_value(&gif_presentation, "Logical screen"),
            Some("1 × 1 pixels")
        );

        let avi_presentation = present_candidate(avi_candidate(), Some(avi));
        assert_eq!(avi_presentation.preview.kind, PreviewKind::StructureSummary);
        assert_eq!(preview_value(&avi_presentation, "Format"), Some("RIFF AVI"));
        assert_eq!(
            preview_value(&avi_presentation, "Required lists"),
            Some("hdrl and movi present")
        );

        let mp4_presentation = present_candidate(mp4_candidate(), Some(mp4));
        assert_eq!(mp4_presentation.preview.kind, PreviewKind::StructureSummary);
        assert_eq!(
            preview_value(&mp4_presentation, "Format"),
            Some("MP4-style ISO Base Media container")
        );
        assert_eq!(
            preview_value(&mp4_presentation, "Movie metadata"),
            Some("moov present")
        );
    }

    #[test]
    fn keeps_malformed_binary_preview_metadata_only_and_explains_carving_limits() {
        let presentation = present_candidate(png_candidate(), Some(b"not rendered"));

        assert_eq!(presentation.preview.kind, PreviewKind::MetadataOnly);
        assert_eq!(presentation.preview.text_excerpt, None);
        assert!(presentation
            .explanation
            .contains("original filename and folder"));
        assert_eq!(presentation.validation_label, "Recovered and checked");
    }
}
