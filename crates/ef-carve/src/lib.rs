use serde::{Deserialize, Serialize};
use thiserror::Error;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const IHDR: [u8; 4] = *b"IHDR";
const IEND: [u8; 4] = *b"IEND";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CarveError {
    #[error("candidate source range is outside the supplied image")]
    CandidateOutsideImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PngCarvedCandidate {
    pub evidence_name: String,
    pub source_offset: u64,
    pub byte_length: u64,
}

pub fn carve_pngs(image: &[u8]) -> Vec<PngCarvedCandidate> {
    let mut candidates = Vec::new();
    let mut search_offset = 0;

    while let Some(relative_offset) = find_subslice(&image[search_offset..], &PNG_SIGNATURE) {
        let start = search_offset + relative_offset;
        if let Some(byte_length) = parse_png_length(&image[start..]) {
            candidates.push(PngCarvedCandidate {
                evidence_name: format!("carved-png-{:04}.png", candidates.len()),
                source_offset: start as u64,
                byte_length: byte_length as u64,
            });
            search_offset = start + byte_length;
        } else {
            search_offset = start + 1;
        }
    }

    candidates
}

pub fn extract_png(image: &[u8], candidate: &PngCarvedCandidate) -> Result<Vec<u8>, CarveError> {
    extract_range(image, candidate.source_offset, candidate.byte_length)
}

fn parse_png_length(bytes: &[u8]) -> Option<usize> {
    if bytes.get(..PNG_SIGNATURE.len())? != PNG_SIGNATURE {
        return None;
    }

    let mut offset = PNG_SIGNATURE.len();
    let mut chunk_index = 0;

    loop {
        let length_bytes = bytes.get(offset..offset + 4)?;
        let chunk_length =
            usize::try_from(u32::from_be_bytes(length_bytes.try_into().ok()?)).ok()?;
        let chunk_type = bytes.get(offset + 4..offset + 8)?;
        let chunk_end = offset.checked_add(12)?.checked_add(chunk_length)?;
        bytes.get(offset..chunk_end)?;

        if chunk_index == 0 && (chunk_type != IHDR || chunk_length != 13) {
            return None;
        }
        if chunk_type == IEND {
            if chunk_length != 0 {
                return None;
            }
            return Some(chunk_end);
        }

        offset = chunk_end;
        chunk_index += 1;
    }
}

const JPEG_SOI: [u8; 2] = [0xff, 0xd8];
const JPEG_EOI: [u8; 2] = [0xff, 0xd9];
const JPEG_MAX_CARVE_LENGTH: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JpegCarvedCandidate {
    pub evidence_name: String,
    pub source_offset: u64,
    pub byte_length: u64,
}

pub fn carve_jpegs(image: &[u8]) -> Vec<JpegCarvedCandidate> {
    let mut candidates = Vec::new();
    let mut search_offset = 0;

    while let Some(relative_offset) = find_subslice(&image[search_offset..], &JPEG_SOI) {
        let start = search_offset + relative_offset;
        if let Some(byte_length) = parse_jpeg_length(&image[start..]) {
            candidates.push(JpegCarvedCandidate {
                evidence_name: format!("carved-jpeg-{:04}.jpg", candidates.len()),
                source_offset: start as u64,
                byte_length: byte_length as u64,
            });
            search_offset = start + byte_length;
        } else {
            search_offset = start + 1;
        }
    }

    candidates
}

pub fn extract_jpeg(image: &[u8], candidate: &JpegCarvedCandidate) -> Result<Vec<u8>, CarveError> {
    extract_range(image, candidate.source_offset, candidate.byte_length)
}

fn parse_jpeg_length(bytes: &[u8]) -> Option<usize> {
    if bytes.get(..JPEG_SOI.len())? != JPEG_SOI {
        return None;
    }

    let limit = bytes.len().min(JPEG_MAX_CARVE_LENGTH);
    let mut offset = JPEG_SOI.len();
    let mut saw_frame = false;

    while offset < limit {
        if *bytes.get(offset)? != 0xff {
            return None;
        }
        while *bytes.get(offset)? == 0xff {
            offset += 1;
        }
        let marker = *bytes.get(offset)?;
        offset += 1;

        if marker == 0xd9 {
            return None;
        }
        if marker == 0xd8 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }

        let length_bytes = bytes.get(offset..offset + 2)?;
        let segment_length = usize::from(u16::from_be_bytes(length_bytes.try_into().ok()?));
        if segment_length < 2 {
            return None;
        }
        let segment_end = offset.checked_add(segment_length)?;
        if segment_end > limit {
            return None;
        }

        if is_jpeg_frame_marker(marker) {
            saw_frame = true;
        }
        if marker == 0xda {
            if !saw_frame {
                return None;
            }
            return find_jpeg_end_of_image(bytes, segment_end, limit);
        }

        offset = segment_end;
    }

    None
}

fn is_jpeg_frame_marker(marker: u8) -> bool {
    matches!(
        marker,
        0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf
    )
}

fn find_jpeg_end_of_image(bytes: &[u8], mut offset: usize, limit: usize) -> Option<usize> {
    while offset + 1 < limit {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }

        match bytes[offset + 1] {
            0x00 | 0xd0..=0xd7 => offset += 2,
            0xd9 => return Some(offset + JPEG_EOI.len()),
            _ => return None,
        }
    }

    None
}

const GIF87A_HEADER: [u8; 6] = *b"GIF87a";
const GIF89A_HEADER: [u8; 6] = *b"GIF89a";
const GIF_TRAILER: u8 = 0x3b;
const GIF_MAX_CARVE_LENGTH: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GifCarvedCandidate {
    pub evidence_name: String,
    pub source_offset: u64,
    pub byte_length: u64,
}

pub fn carve_gifs(image: &[u8]) -> Vec<GifCarvedCandidate> {
    let mut candidates = Vec::new();
    let mut search_offset = 0;

    while search_offset < image.len() {
        let gif87a = find_subslice(&image[search_offset..], &GIF87A_HEADER)
            .map(|offset| search_offset + offset);
        let gif89a = find_subslice(&image[search_offset..], &GIF89A_HEADER)
            .map(|offset| search_offset + offset);
        let Some(start) = (match (gif87a, gif89a) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(offset), None) | (None, Some(offset)) => Some(offset),
            (None, None) => None,
        }) else {
            break;
        };

        if let Some(byte_length) = parse_gif_length(&image[start..]) {
            candidates.push(GifCarvedCandidate {
                evidence_name: format!("carved-gif-{:04}.gif", candidates.len()),
                source_offset: start as u64,
                byte_length: byte_length as u64,
            });
            search_offset = start + byte_length;
        } else {
            search_offset = start + 1;
        }
    }

    candidates
}

pub fn extract_gif(image: &[u8], candidate: &GifCarvedCandidate) -> Result<Vec<u8>, CarveError> {
    extract_range(image, candidate.source_offset, candidate.byte_length)
}

fn parse_gif_length(bytes: &[u8]) -> Option<usize> {
    let header = bytes.get(..6)?;
    if header != GIF87A_HEADER && header != GIF89A_HEADER {
        return None;
    }

    let limit = bytes.len().min(GIF_MAX_CARVE_LENGTH);
    let logical_screen_end = 13;
    let screen = bytes.get(6..logical_screen_end)?;
    let packed_fields = *screen.get(4)?;
    let mut offset = logical_screen_end;
    if packed_fields & 0x80 != 0 {
        offset = offset.checked_add(gif_color_table_length(packed_fields)?)?;
        if offset > limit {
            return None;
        }
    }

    let mut saw_image = false;
    while offset < limit {
        match *bytes.get(offset)? {
            GIF_TRAILER if saw_image => return Some(offset + 1),
            0x21 => {
                offset = parse_gif_extension(bytes, offset, limit)?;
            }
            0x2c => {
                saw_image = true;
                offset = parse_gif_image(bytes, offset, limit)?;
            }
            _ => return None,
        }
    }

    None
}

fn gif_color_table_length(packed_fields: u8) -> Option<usize> {
    let entries = 1_usize.checked_shl(u32::from((packed_fields & 0x07) + 1))?;
    entries.checked_mul(3)
}

fn parse_gif_extension(bytes: &[u8], offset: usize, limit: usize) -> Option<usize> {
    let label_offset = offset.checked_add(1)?;
    bytes.get(label_offset)?;
    let block_length_offset = label_offset.checked_add(1)?;
    let block_length = usize::from(*bytes.get(block_length_offset)?);
    let block_data_start = block_length_offset.checked_add(1)?;
    let sub_blocks_start = block_data_start.checked_add(block_length)?;
    if sub_blocks_start > limit {
        return None;
    }
    parse_gif_sub_blocks(bytes, sub_blocks_start, limit)
}

fn parse_gif_image(bytes: &[u8], offset: usize, limit: usize) -> Option<usize> {
    let descriptor_end = offset.checked_add(10)?;
    let descriptor = bytes.get(offset..descriptor_end)?;
    let packed_fields = *descriptor.get(9)?;
    let mut image_data_start = descriptor_end;
    if packed_fields & 0x80 != 0 {
        image_data_start = image_data_start.checked_add(gif_color_table_length(packed_fields)?)?;
    }
    let lzw_minimum_code_size = *bytes.get(image_data_start)?;
    if !(2..=8).contains(&lzw_minimum_code_size) {
        return None;
    }
    let sub_blocks_start = image_data_start.checked_add(1)?;
    if sub_blocks_start > limit {
        return None;
    }
    parse_gif_sub_blocks(bytes, sub_blocks_start, limit)
}

fn parse_gif_sub_blocks(bytes: &[u8], mut offset: usize, limit: usize) -> Option<usize> {
    loop {
        let length = usize::from(*bytes.get(offset)?);
        offset = offset.checked_add(1)?;
        if length == 0 {
            return Some(offset);
        }
        offset = offset.checked_add(length)?;
        if offset > limit {
            return None;
        }
    }
}

const RIFF_HEADER: [u8; 4] = *b"RIFF";
const AVI_FORM_TYPE: [u8; 4] = *b"AVI ";
const LIST_CHUNK_ID: [u8; 4] = *b"LIST";
const AVI_HEADER_LIST_TYPE: [u8; 4] = *b"hdrl";
const AVI_MEDIA_LIST_TYPE: [u8; 4] = *b"movi";
const AVI_MAX_CARVE_LENGTH: usize = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AviCarvedCandidate {
    pub evidence_name: String,
    pub source_offset: u64,
    pub byte_length: u64,
}

pub fn carve_avis(image: &[u8]) -> Vec<AviCarvedCandidate> {
    let mut candidates = Vec::new();
    let mut search_offset = 0;

    while let Some(relative_offset) = find_subslice(&image[search_offset..], &RIFF_HEADER) {
        let start = search_offset + relative_offset;
        if let Some(byte_length) = parse_avi_length(&image[start..]) {
            candidates.push(AviCarvedCandidate {
                evidence_name: format!("carved-avi-{:04}.avi", candidates.len()),
                source_offset: start as u64,
                byte_length: byte_length as u64,
            });
            search_offset = start + byte_length;
        } else {
            search_offset = start + 1;
        }
    }

    candidates
}

pub fn extract_avi(image: &[u8], candidate: &AviCarvedCandidate) -> Result<Vec<u8>, CarveError> {
    extract_range(image, candidate.source_offset, candidate.byte_length)
}

fn parse_avi_length(bytes: &[u8]) -> Option<usize> {
    let header = bytes.get(..12)?;
    if header.get(..4)? != RIFF_HEADER || header.get(8..12)? != AVI_FORM_TYPE {
        return None;
    }

    let riff_data_length = usize::try_from(read_u32_le(header, 4)?).ok()?;
    let candidate_end = riff_data_length.checked_add(8)?;
    if candidate_end > bytes.len() || !(12..=AVI_MAX_CARVE_LENGTH).contains(&candidate_end) {
        return None;
    }

    let mut offset = 12;
    let mut saw_header_list = false;
    let mut saw_media_list = false;
    while offset < candidate_end {
        let chunk_header_end = offset.checked_add(8)?;
        let chunk_header = bytes.get(offset..chunk_header_end)?;
        let chunk_id = chunk_header.get(..4)?;
        let chunk_length = usize::try_from(read_u32_le(chunk_header, 4)?).ok()?;
        let chunk_end = chunk_header_end.checked_add(chunk_length)?;
        let padded_end = chunk_end.checked_add(chunk_length & 1)?;
        if padded_end > candidate_end {
            return None;
        }

        if chunk_id == LIST_CHUNK_ID {
            let list_type = bytes.get(chunk_header_end..chunk_header_end.checked_add(4)?)?;
            if list_type == AVI_HEADER_LIST_TYPE {
                saw_header_list = true;
            }
            if list_type == AVI_MEDIA_LIST_TYPE {
                saw_media_list = true;
            }
        }
        offset = padded_end;
    }

    (offset == candidate_end && saw_header_list && saw_media_list).then_some(candidate_end)
}

const MP4_FILE_TYPE_BOX: [u8; 4] = *b"ftyp";
const MP4_MOVIE_BOX: [u8; 4] = *b"moov";
const MP4_MEDIA_DATA_BOX: [u8; 4] = *b"mdat";
const MP4_MOVIE_HEADER_BOX: [u8; 4] = *b"mvhd";
const MP4_TRACK_BOX: [u8; 4] = *b"trak";
const MP4_MOVIE_FRAGMENT_BOX: [u8; 4] = *b"moof";
const MP4_MAX_CARVE_LENGTH: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mp4CarvedCandidate {
    pub evidence_name: String,
    pub file_type: String,
    pub source_offset: u64,
    pub byte_length: u64,
}

pub fn carve_mp4s(image: &[u8]) -> Vec<Mp4CarvedCandidate> {
    let mut candidates = Vec::new();
    let mut search_offset = 0;

    while let Some(relative_type_offset) =
        find_subslice(&image[search_offset..], &MP4_FILE_TYPE_BOX)
    {
        let type_offset = search_offset + relative_type_offset;
        let Some(start) = type_offset.checked_sub(4) else {
            search_offset = type_offset + 1;
            continue;
        };
        if let Some((byte_length, file_type)) = parse_mp4_length_and_type(&image[start..]) {
            candidates.push(Mp4CarvedCandidate {
                evidence_name: format!("carved-{file_type}-{:04}.{file_type}", candidates.len()),
                file_type,
                source_offset: start as u64,
                byte_length: byte_length as u64,
            });
            search_offset = start + byte_length;
        } else {
            search_offset = type_offset + 1;
        }
    }

    candidates
}

pub fn extract_mp4(image: &[u8], candidate: &Mp4CarvedCandidate) -> Result<Vec<u8>, CarveError> {
    extract_range(image, candidate.source_offset, candidate.byte_length)
}

fn parse_mp4_length_and_type(bytes: &[u8]) -> Option<(usize, String)> {
    let limit = bytes.len().min(MP4_MAX_CARVE_LENGTH);
    let first_box = parse_mp4_box(bytes, 0, limit)?;
    if first_box.box_type != MP4_FILE_TYPE_BOX || first_box.payload.len() < 8 {
        return None;
    }
    let file_type = mp4_file_type(first_box.payload.get(..4)?)?.to_owned();

    let mut offset = first_box.end;
    let mut saw_movie = false;
    while offset < limit {
        let box_info = parse_mp4_box(bytes, offset, limit)?;
        if box_info.box_type == MP4_MOVIE_FRAGMENT_BOX {
            return None;
        }
        if box_info.box_type == MP4_MOVIE_BOX {
            if !mp4_movie_box_is_bounded(box_info.payload) {
                return None;
            }
            saw_movie = true;
        }
        if box_info.box_type == MP4_MEDIA_DATA_BOX {
            return saw_movie.then_some((box_info.end, file_type));
        }
        offset = box_info.end;
    }

    None
}

struct Mp4Box<'a> {
    box_type: [u8; 4],
    payload: &'a [u8],
    end: usize,
}

fn parse_mp4_box(bytes: &[u8], offset: usize, limit: usize) -> Option<Mp4Box<'_>> {
    let header_end = offset.checked_add(8)?;
    let header = bytes.get(offset..header_end)?;
    let size = usize::try_from(u32::from_be_bytes(header.get(..4)?.try_into().ok()?)).ok()?;
    if size < 8 || size == 1 || size == 0 {
        return None;
    }
    let end = offset.checked_add(size)?;
    if end > limit {
        return None;
    }
    Some(Mp4Box {
        box_type: header.get(4..8)?.try_into().ok()?,
        payload: bytes.get(header_end..end)?,
        end,
    })
}

fn mp4_movie_box_is_bounded(payload: &[u8]) -> bool {
    let mut offset = 0;
    let mut saw_movie_header = false;
    let mut saw_track = false;
    while offset < payload.len() {
        let Some(box_info) = parse_mp4_box(payload, offset, payload.len()) else {
            return false;
        };
        if box_info.box_type == MP4_MOVIE_HEADER_BOX {
            saw_movie_header = true;
        }
        if box_info.box_type == MP4_TRACK_BOX {
            saw_track = true;
        }
        offset = box_info.end;
    }
    saw_movie_header && saw_track
}

fn mp4_file_type(brand: &[u8]) -> Option<&'static str> {
    match brand {
        b"qt  " => Some("mov"),
        b"isom" | b"iso2" | b"mp41" | b"mp42" | b"M4V " | b"avc1" | b"3gp4" | b"3gp5" | b"3gp6"
        | b"3gp7" | b"3gp8" | b"3gp9" => Some("mp4"),
        _ => None,
    }
}

const PDF_HEADER: &[u8] = b"%PDF-";
const PDF_START_XREF: &[u8] = b"startxref";
const PDF_XREF: &[u8] = b"xref";
const PDF_EOF: &[u8] = b"%%EOF";
const PDF_MAX_CARVE_LENGTH: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfCarvedCandidate {
    pub evidence_name: String,
    pub source_offset: u64,
    pub byte_length: u64,
}

pub fn carve_pdfs(image: &[u8]) -> Vec<PdfCarvedCandidate> {
    let mut candidates = Vec::new();
    let mut search_offset = 0;

    while let Some(relative_offset) = find_subslice(&image[search_offset..], PDF_HEADER) {
        let start = search_offset + relative_offset;
        if let Some(byte_length) = parse_pdf_length(&image[start..]) {
            candidates.push(PdfCarvedCandidate {
                evidence_name: format!("carved-pdf-{:04}.pdf", candidates.len()),
                source_offset: start as u64,
                byte_length: byte_length as u64,
            });
            search_offset = start + byte_length;
        } else {
            search_offset = start + 1;
        }
    }

    candidates
}

pub fn extract_pdf(image: &[u8], candidate: &PdfCarvedCandidate) -> Result<Vec<u8>, CarveError> {
    extract_range(image, candidate.source_offset, candidate.byte_length)
}

fn parse_pdf_length(bytes: &[u8]) -> Option<usize> {
    if bytes.get(..PDF_HEADER.len())? != PDF_HEADER
        || !bytes.get(5)?.is_ascii_digit()
        || *bytes.get(6)? != b'.'
        || !bytes.get(7)?.is_ascii_digit()
    {
        return None;
    }

    let limit = bytes.len().min(PDF_MAX_CARVE_LENGTH);
    let mut search_offset = 0;
    while let Some(relative_offset) = find_subslice(&bytes[search_offset..limit], PDF_EOF) {
        let eof_offset = search_offset + relative_offset;
        let candidate_end = eof_offset + PDF_EOF.len();
        if let Some(startxref_offset) = find_last_subslice(&bytes[..eof_offset], PDF_START_XREF) {
            if let Some(xref_offset) =
                parse_startxref_offset(&bytes[..limit], startxref_offset, eof_offset)
            {
                if bytes.get(xref_offset..xref_offset + PDF_XREF.len())? == PDF_XREF {
                    return Some(candidate_end);
                }
            }
        }
        search_offset = eof_offset + 1;
    }

    None
}

fn parse_startxref_offset(
    bytes: &[u8],
    startxref_offset: usize,
    eof_offset: usize,
) -> Option<usize> {
    let mut offset = startxref_offset.checked_add(PDF_START_XREF.len())?;
    while offset < eof_offset && is_pdf_whitespace(*bytes.get(offset)?) {
        offset += 1;
    }
    let digits_start = offset;
    while offset < eof_offset && bytes.get(offset)?.is_ascii_digit() {
        offset += 1;
    }
    if digits_start == offset {
        return None;
    }
    let xref_offset = std::str::from_utf8(bytes.get(digits_start..offset)?)
        .ok()?
        .parse::<usize>()
        .ok()?;
    if xref_offset >= startxref_offset {
        return None;
    }
    while offset < eof_offset {
        if !is_pdf_whitespace(*bytes.get(offset)?) {
            return None;
        }
        offset += 1;
    }
    Some(xref_offset)
}

fn is_pdf_whitespace(byte: u8) -> bool {
    matches!(byte, 0x00 | b'\t' | b'\n' | 0x0c | b'\r' | b' ')
}

const ZIP_LOCAL_FILE_HEADER: [u8; 4] = *b"PK\x03\x04";
const ZIP_CENTRAL_DIRECTORY_HEADER: [u8; 4] = *b"PK\x01\x02";
const ZIP_END_OF_CENTRAL_DIRECTORY: [u8; 4] = *b"PK\x05\x06";
const ZIP_MAX_CARVE_LENGTH: usize = 64 * 1024 * 1024;
const ZIP_END_OF_CENTRAL_DIRECTORY_MINIMUM_LENGTH: usize = 22;
const ZIP_LOCAL_FILE_HEADER_MINIMUM_LENGTH: usize = 30;
const ZIP_CENTRAL_DIRECTORY_HEADER_MINIMUM_LENGTH: usize = 46;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipCarvedCandidate {
    pub evidence_name: String,
    pub file_type: String,
    pub source_offset: u64,
    pub byte_length: u64,
}

pub fn carve_zip_archives(image: &[u8]) -> Vec<ZipCarvedCandidate> {
    let mut candidates = Vec::new();
    let mut search_offset = 0;

    while let Some(relative_offset) = find_subslice(&image[search_offset..], &ZIP_LOCAL_FILE_HEADER)
    {
        let start = search_offset + relative_offset;
        if let Some((byte_length, file_type)) = parse_zip_length_and_type(&image[start..]) {
            candidates.push(ZipCarvedCandidate {
                evidence_name: format!("carved-zip-{:04}.{}", candidates.len(), file_type),
                file_type,
                source_offset: start as u64,
                byte_length: byte_length as u64,
            });
            search_offset = start + byte_length;
        } else {
            search_offset = start + 1;
        }
    }

    candidates
}

pub fn extract_zip(image: &[u8], candidate: &ZipCarvedCandidate) -> Result<Vec<u8>, CarveError> {
    extract_range(image, candidate.source_offset, candidate.byte_length)
}

fn parse_zip_length_and_type(bytes: &[u8]) -> Option<(usize, String)> {
    if bytes.get(..ZIP_LOCAL_FILE_HEADER.len())? != ZIP_LOCAL_FILE_HEADER {
        return None;
    }

    let limit = bytes.len().min(ZIP_MAX_CARVE_LENGTH);
    let mut search_offset = 0;
    while let Some(relative_offset) =
        find_subslice(&bytes[search_offset..limit], &ZIP_END_OF_CENTRAL_DIRECTORY)
    {
        let eocd_offset = search_offset + relative_offset;
        if let Some((candidate_end, entry_names)) =
            parse_zip_end_of_central_directory(&bytes[..limit], eocd_offset)
        {
            return Some((candidate_end, zip_file_type(&entry_names).to_owned()));
        }
        search_offset = eocd_offset + 1;
    }

    None
}

fn parse_zip_end_of_central_directory(
    bytes: &[u8],
    eocd_offset: usize,
) -> Option<(usize, Vec<Vec<u8>>)> {
    let fixed_end = eocd_offset.checked_add(ZIP_END_OF_CENTRAL_DIRECTORY_MINIMUM_LENGTH)?;
    let record = bytes.get(eocd_offset..fixed_end)?;
    if record.get(..4)? != ZIP_END_OF_CENTRAL_DIRECTORY {
        return None;
    }

    let disk_number = read_u16_le(record, 4)?;
    let central_directory_disk = read_u16_le(record, 6)?;
    let entries_on_disk = read_u16_le(record, 8)?;
    let entry_count = read_u16_le(record, 10)?;
    let central_directory_size = usize::try_from(read_u32_le(record, 12)?).ok()?;
    let central_directory_offset = usize::try_from(read_u32_le(record, 16)?).ok()?;
    let comment_length = usize::from(read_u16_le(record, 20)?);
    let candidate_end = fixed_end.checked_add(comment_length)?;

    if candidate_end > bytes.len()
        || disk_number != 0
        || central_directory_disk != 0
        || entries_on_disk == 0
        || entry_count == 0
        || entries_on_disk != entry_count
        || central_directory_size == 0
        || entry_count == u16::MAX
        || central_directory_size == u32::MAX as usize
        || central_directory_offset == u32::MAX as usize
    {
        return None;
    }

    let central_directory_end = central_directory_offset.checked_add(central_directory_size)?;
    if central_directory_end != eocd_offset {
        return None;
    }

    let entry_names = parse_zip_central_directory(
        bytes,
        central_directory_offset,
        central_directory_end,
        usize::from(entry_count),
    )?;
    Some((candidate_end, entry_names))
}

fn parse_zip_central_directory(
    bytes: &[u8],
    central_directory_offset: usize,
    central_directory_end: usize,
    entry_count: usize,
) -> Option<Vec<Vec<u8>>> {
    let mut offset = central_directory_offset;
    let mut names = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        let fixed_end = offset.checked_add(ZIP_CENTRAL_DIRECTORY_HEADER_MINIMUM_LENGTH)?;
        let header = bytes.get(offset..fixed_end)?;
        if header.get(..4)? != ZIP_CENTRAL_DIRECTORY_HEADER {
            return None;
        }
        let file_name_length = usize::from(read_u16_le(header, 28)?);
        let extra_field_length = usize::from(read_u16_le(header, 30)?);
        let comment_length = usize::from(read_u16_le(header, 32)?);
        let local_header_offset = usize::try_from(read_u32_le(header, 42)?).ok()?;
        let variable_end = fixed_end
            .checked_add(file_name_length)?
            .checked_add(extra_field_length)?
            .checked_add(comment_length)?;
        if variable_end > central_directory_end {
            return None;
        }
        let file_name = bytes.get(fixed_end..fixed_end + file_name_length)?.to_vec();
        if !matches_local_file_header(bytes, local_header_offset, &file_name) {
            return None;
        }
        names.push(file_name);
        offset = variable_end;
    }

    (offset == central_directory_end).then_some(names)
}

fn matches_local_file_header(bytes: &[u8], offset: usize, expected_name: &[u8]) -> bool {
    let Some(fixed_end) = offset.checked_add(ZIP_LOCAL_FILE_HEADER_MINIMUM_LENGTH) else {
        return false;
    };
    let Some(header) = bytes.get(offset..fixed_end) else {
        return false;
    };
    if header.get(..4) != Some(&ZIP_LOCAL_FILE_HEADER) {
        return false;
    }
    let Some(file_name_length) = read_u16_le(header, 26).map(usize::from) else {
        return false;
    };
    let Some(extra_field_length) = read_u16_le(header, 28).map(usize::from) else {
        return false;
    };
    let Some(file_name_end) = fixed_end
        .checked_add(file_name_length)
        .and_then(|end| end.checked_add(extra_field_length))
    else {
        return false;
    };
    bytes.get(fixed_end..fixed_end + file_name_length) == Some(expected_name)
        && bytes.get(fixed_end..file_name_end).is_some()
}

fn zip_file_type(entry_names: &[Vec<u8>]) -> &'static str {
    let contains = |name: &[u8]| entry_names.iter().any(|entry| entry.as_slice() == name);
    if contains(b"[Content_Types].xml") && contains(b"_rels/.rels") {
        if contains(b"word/document.xml") {
            return "docx";
        }
        if contains(b"xl/workbook.xml") {
            return "xlsx";
        }
        if contains(b"ppt/presentation.xml") {
            return "pptx";
        }
    }
    "zip"
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let value = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes(value.try_into().ok()?))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let value = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(value.try_into().ok()?))
}

fn find_last_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

fn extract_range(
    image: &[u8],
    source_offset: u64,
    byte_length: u64,
) -> Result<Vec<u8>, CarveError> {
    let start = usize::try_from(source_offset).map_err(|_| CarveError::CandidateOutsideImage)?;
    let length = usize::try_from(byte_length).map_err(|_| CarveError::CandidateOutsideImage)?;
    let end = start
        .checked_add(length)
        .ok_or(CarveError::CandidateOutsideImage)?;
    let bytes = image
        .get(start..end)
        .ok_or(CarveError::CandidateOutsideImage)?;
    Ok(bytes.to_vec())
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::{
        carve_avis, carve_gifs, carve_jpegs, carve_mp4s, carve_pdfs, carve_pngs,
        carve_zip_archives, extract_avi, extract_gif, extract_jpeg, extract_mp4, extract_pdf,
        extract_png, extract_zip, AviCarvedCandidate, CarveError, GifCarvedCandidate,
        JpegCarvedCandidate, Mp4CarvedCandidate, PdfCarvedCandidate, PngCarvedCandidate,
        ZipCarvedCandidate, AVI_FORM_TYPE, AVI_HEADER_LIST_TYPE, AVI_MEDIA_LIST_TYPE, GIF_TRAILER,
        LIST_CHUNK_ID, MP4_FILE_TYPE_BOX, MP4_MEDIA_DATA_BOX, MP4_MOVIE_BOX,
        MP4_MOVIE_FRAGMENT_BOX, MP4_MOVIE_HEADER_BOX, MP4_TRACK_BOX, RIFF_HEADER,
    };

    const VALID_PNG: [u8; 70] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn finds_a_structurally_complete_png_at_its_source_offset() {
        let mut image = vec![0_u8; 127];
        image[32..32 + VALID_PNG.len()].copy_from_slice(&VALID_PNG);

        let candidates = carve_pngs(&image);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].evidence_name, "carved-png-0000.png");
        assert_eq!(candidates[0].source_offset, 32);
        assert_eq!(candidates[0].byte_length, VALID_PNG.len() as u64);
        assert_eq!(
            extract_png(&image, &candidates[0]).expect("extract PNG"),
            VALID_PNG
        );
    }

    #[test]
    fn ignores_a_signature_without_complete_png_structure() {
        let mut image = VALID_PNG.to_vec();
        image.truncate(52);

        assert!(carve_pngs(&image).is_empty());
    }

    #[test]
    fn ignores_a_signature_without_a_valid_first_ihdr_chunk() {
        let mut image = VALID_PNG.to_vec();
        image[12..16].copy_from_slice(b"IDAT");

        assert!(carve_pngs(&image).is_empty());
    }

    #[test]
    fn rejects_extraction_outside_the_image() {
        let candidate = PngCarvedCandidate {
            evidence_name: "carved-png-0000.png".to_owned(),
            source_offset: 16,
            byte_length: 8,
        };

        assert_eq!(
            extract_png(&[0_u8; 20], &candidate).expect_err("reject outside range"),
            CarveError::CandidateOutsideImage
        );
    }

    const VALID_GIF: [u8; 35] = [
        b'G',
        b'I',
        b'F',
        b'8',
        b'9',
        b'a',
        1,
        0,
        1,
        0,
        0x80,
        0,
        0,
        0,
        0,
        0,
        0xff,
        0xff,
        0xff,
        0x2c,
        0,
        0,
        0,
        0,
        1,
        0,
        1,
        0,
        0,
        2,
        2,
        0x44,
        0x01,
        0,
        GIF_TRAILER,
    ];

    #[test]
    fn finds_a_structurally_bounded_gif_at_its_source_offset() {
        let mut image = vec![0_u8; 19];
        image.extend_from_slice(&VALID_GIF);
        image.extend_from_slice(&[0_u8; 7]);

        let candidates = carve_gifs(&image);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].evidence_name, "carved-gif-0000.gif");
        assert_eq!(candidates[0].source_offset, 19);
        assert_eq!(candidates[0].byte_length, VALID_GIF.len() as u64);
        assert_eq!(
            extract_gif(&image, &candidates[0]).expect("extract GIF"),
            VALID_GIF
        );
    }

    #[test]
    fn ignores_truncated_or_structurally_invalid_gifs() {
        let mut missing_trailer = VALID_GIF.to_vec();
        missing_trailer.pop();
        assert!(carve_gifs(&missing_trailer).is_empty());

        let mut invalid_lzw_size = VALID_GIF;
        invalid_lzw_size[29] = 1;
        assert!(carve_gifs(&invalid_lzw_size).is_empty());
    }

    #[test]
    fn rejects_gif_extraction_outside_the_image() {
        let candidate = GifCarvedCandidate {
            evidence_name: "carved-gif-0000.gif".to_owned(),
            source_offset: 16,
            byte_length: 8,
        };

        assert_eq!(
            extract_gif(&[0_u8; 20], &candidate).expect_err("reject outside range"),
            CarveError::CandidateOutsideImage
        );
    }

    const VALID_JPEG: [u8; 35] = [
        0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0x00, 0x00, 0xff, 0xc0, 0x00, 0x0b, 0x08, 0x00, 0x01,
        0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f,
        0x00, 0x11, 0x22, 0xff, 0xd9,
    ];

    #[test]
    fn finds_a_structurally_complete_jpeg_at_its_source_offset() {
        let mut image = vec![0_u8; 110];
        image[24..24 + VALID_JPEG.len()].copy_from_slice(&VALID_JPEG);

        let candidates = carve_jpegs(&image);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].evidence_name, "carved-jpeg-0000.jpg");
        assert_eq!(candidates[0].source_offset, 24);
        assert_eq!(candidates[0].byte_length, VALID_JPEG.len() as u64);
        assert_eq!(
            extract_jpeg(&image, &candidates[0]).expect("extract JPEG"),
            VALID_JPEG
        );
    }

    #[test]
    fn ignores_jpeg_without_a_frame_or_complete_end_marker() {
        let mut missing_frame = VALID_JPEG.to_vec();
        missing_frame[9] = 0xe1;
        assert!(carve_jpegs(&missing_frame).is_empty());

        let mut missing_end = VALID_JPEG.to_vec();
        missing_end.truncate(33);
        assert!(carve_jpegs(&missing_end).is_empty());
    }

    #[test]
    fn rejects_jpeg_extraction_outside_the_image() {
        let candidate = JpegCarvedCandidate {
            evidence_name: "carved-jpeg-0000.jpg".to_owned(),
            source_offset: 16,
            byte_length: 8,
        };

        assert_eq!(
            extract_jpeg(&[0_u8; 20], &candidate).expect_err("reject outside range"),
            CarveError::CandidateOutsideImage
        );
    }

    fn push_le_chunk(output: &mut Vec<u8>, chunk_id: &[u8; 4], payload: &[u8]) {
        output.extend_from_slice(chunk_id);
        output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        output.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            output.push(0);
        }
    }

    fn valid_avi() -> Vec<u8> {
        let mut riff_data = AVI_FORM_TYPE.to_vec();
        let mut header_list = AVI_HEADER_LIST_TYPE.to_vec();
        header_list.extend_from_slice(b"AVIH");
        push_le_chunk(&mut riff_data, &LIST_CHUNK_ID, &header_list);
        push_le_chunk(&mut riff_data, &LIST_CHUNK_ID, &AVI_MEDIA_LIST_TYPE);

        let mut avi = RIFF_HEADER.to_vec();
        avi.extend_from_slice(&(riff_data.len() as u32).to_le_bytes());
        avi.extend_from_slice(&riff_data);
        avi
    }

    fn push_mp4_box(output: &mut Vec<u8>, box_type: &[u8; 4], payload: &[u8]) {
        let size = u32::try_from(payload.len() + 8).expect("test MP4 box length");
        output.extend_from_slice(&size.to_be_bytes());
        output.extend_from_slice(box_type);
        output.extend_from_slice(payload);
    }

    fn valid_mp4() -> Vec<u8> {
        let mut mp4 = Vec::new();
        push_mp4_box(&mut mp4, &MP4_FILE_TYPE_BOX, b"isom\0\0\0\0isom");
        let mut movie = Vec::new();
        push_mp4_box(&mut movie, &MP4_MOVIE_HEADER_BOX, &[]);
        push_mp4_box(&mut movie, &MP4_TRACK_BOX, &[]);
        push_mp4_box(&mut mp4, &MP4_MOVIE_BOX, &movie);
        push_mp4_box(&mut mp4, &MP4_MEDIA_DATA_BOX, b"media");
        mp4
    }

    #[test]
    fn finds_a_bounded_avi_with_required_lists() {
        let avi = valid_avi();
        let mut image = vec![0_u8; 27];
        image.extend_from_slice(&avi);

        let candidates = carve_avis(&image);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].evidence_name, "carved-avi-0000.avi");
        assert_eq!(candidates[0].source_offset, 27);
        assert_eq!(candidates[0].byte_length, avi.len() as u64);
        assert_eq!(
            extract_avi(&image, &candidates[0]).expect("extract AVI"),
            avi
        );
    }

    #[test]
    fn rejects_avi_without_required_media_list() {
        let mut avi = valid_avi();
        let movi = avi
            .windows(AVI_MEDIA_LIST_TYPE.len())
            .position(|window| window == AVI_MEDIA_LIST_TYPE)
            .expect("find movi list");
        avi[movi..movi + 4].copy_from_slice(b"JUNK");

        assert!(carve_avis(&avi).is_empty());
    }

    #[test]
    fn rejects_avi_extraction_outside_the_image() {
        let candidate = AviCarvedCandidate {
            evidence_name: "carved-avi-0000.avi".to_owned(),
            source_offset: 16,
            byte_length: 8,
        };

        assert_eq!(
            extract_avi(&[0_u8; 20], &candidate).expect_err("reject outside range"),
            CarveError::CandidateOutsideImage
        );
    }

    #[test]
    fn finds_a_self_contained_mp4_with_movie_and_media_boxes() {
        let mp4 = valid_mp4();
        let mut image = vec![0_u8; 31];
        image.extend_from_slice(&mp4);

        let candidates = carve_mp4s(&image);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].evidence_name, "carved-mp4-0000.mp4");
        assert_eq!(candidates[0].source_offset, 31);
        assert_eq!(candidates[0].byte_length, mp4.len() as u64);
        assert_eq!(
            extract_mp4(&image, &candidates[0]).expect("extract MP4"),
            mp4
        );
    }

    #[test]
    fn rejects_fragmented_or_incomplete_mp4_content() {
        let mut fragmented = Vec::new();
        push_mp4_box(&mut fragmented, &MP4_FILE_TYPE_BOX, b"isom\0\0\0\0isom");
        push_mp4_box(&mut fragmented, &MP4_MOVIE_FRAGMENT_BOX, &[]);
        fragmented.extend_from_slice(&valid_mp4()[20..]);
        assert!(carve_mp4s(&fragmented).is_empty());

        let mut no_media = valid_mp4();
        let media_box = no_media
            .windows(MP4_MEDIA_DATA_BOX.len())
            .position(|window| window == MP4_MEDIA_DATA_BOX)
            .expect("find media-data box");
        no_media[media_box..media_box + 4].copy_from_slice(b"free");
        assert!(carve_mp4s(&no_media).is_empty());
    }

    #[test]
    fn rejects_mp4_extraction_outside_the_image() {
        let candidate = Mp4CarvedCandidate {
            evidence_name: "carved-mp4-0000.mp4".to_owned(),
            file_type: "mp4".to_owned(),
            source_offset: 16,
            byte_length: 8,
        };

        assert_eq!(
            extract_mp4(&[0_u8; 20], &candidate).expect_err("reject outside range"),
            CarveError::CandidateOutsideImage
        );
    }

    fn valid_pdf() -> Vec<u8> {
        let mut pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n".to_vec();
        let xref_offset = pdf.len();
        pdf.extend_from_slice(
            b"xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 >>\nstartxref\n",
        );
        pdf.extend_from_slice(xref_offset.to_string().as_bytes());
        pdf.extend_from_slice(b"\n%%EOF");
        pdf
    }

    fn write_u16_le(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32_le(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn stored_zip(entry_names: &[&[u8]]) -> Vec<u8> {
        let mut archive = Vec::new();
        let mut central_directory = Vec::new();

        for entry_name in entry_names {
            let local_offset = u32::try_from(archive.len()).expect("test ZIP local offset");
            archive.extend_from_slice(b"PK\x03\x04");
            write_u16_le(&mut archive, 20);
            write_u16_le(&mut archive, 0);
            write_u16_le(&mut archive, 0);
            write_u16_le(&mut archive, 0);
            write_u16_le(&mut archive, 0);
            write_u32_le(&mut archive, 0);
            write_u32_le(&mut archive, 0);
            write_u32_le(&mut archive, 0);
            write_u16_le(
                &mut archive,
                u16::try_from(entry_name.len()).expect("test ZIP name length"),
            );
            write_u16_le(&mut archive, 0);
            archive.extend_from_slice(entry_name);

            central_directory.extend_from_slice(b"PK\x01\x02");
            write_u16_le(&mut central_directory, 20);
            write_u16_le(&mut central_directory, 20);
            write_u16_le(&mut central_directory, 0);
            write_u16_le(&mut central_directory, 0);
            write_u16_le(&mut central_directory, 0);
            write_u16_le(&mut central_directory, 0);
            write_u32_le(&mut central_directory, 0);
            write_u32_le(&mut central_directory, 0);
            write_u32_le(&mut central_directory, 0);
            write_u16_le(
                &mut central_directory,
                u16::try_from(entry_name.len()).expect("test ZIP name length"),
            );
            write_u16_le(&mut central_directory, 0);
            write_u16_le(&mut central_directory, 0);
            write_u16_le(&mut central_directory, 0);
            write_u16_le(&mut central_directory, 0);
            write_u32_le(&mut central_directory, 0);
            write_u32_le(&mut central_directory, local_offset);
            central_directory.extend_from_slice(entry_name);
        }

        let central_directory_offset = u32::try_from(archive.len()).expect("test ZIP CD offset");
        let central_directory_size =
            u32::try_from(central_directory.len()).expect("test ZIP CD length");
        let entry_count = u16::try_from(entry_names.len()).expect("test ZIP entry count");
        archive.extend_from_slice(&central_directory);
        archive.extend_from_slice(b"PK\x05\x06");
        write_u16_le(&mut archive, 0);
        write_u16_le(&mut archive, 0);
        write_u16_le(&mut archive, entry_count);
        write_u16_le(&mut archive, entry_count);
        write_u32_le(&mut archive, central_directory_size);
        write_u32_le(&mut archive, central_directory_offset);
        write_u16_le(&mut archive, 0);
        archive
    }

    #[test]
    fn finds_a_traditional_cross_reference_pdf_at_its_source_offset() {
        let pdf = valid_pdf();
        let mut image = vec![0_u8; 21];
        image.extend_from_slice(&pdf);
        image.extend_from_slice(&[0_u8; 9]);

        let candidates = carve_pdfs(&image);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].evidence_name, "carved-pdf-0000.pdf");
        assert_eq!(candidates[0].source_offset, 21);
        assert_eq!(candidates[0].byte_length, pdf.len() as u64);
        assert_eq!(
            extract_pdf(&image, &candidates[0]).expect("extract PDF"),
            pdf
        );
    }

    #[test]
    fn rejects_pdf_without_a_consistent_startxref_pointer() {
        let mut pdf = valid_pdf();
        let startxref = pdf
            .windows(b"startxref".len())
            .position(|window| window == b"startxref")
            .expect("find startxref");
        let digits = startxref + b"startxref\n".len();
        pdf[digits] = b'9';

        assert!(carve_pdfs(&pdf).is_empty());
    }

    #[test]
    fn finds_a_structurally_consistent_docx_zip_container() {
        let zip = stored_zip(&[b"[Content_Types].xml", b"_rels/.rels", b"word/document.xml"]);
        let mut image = vec![0_u8; 37];
        image.extend_from_slice(&zip);

        let candidates = carve_zip_archives(&image);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].evidence_name, "carved-zip-0000.docx");
        assert_eq!(candidates[0].file_type, "docx");
        assert_eq!(candidates[0].source_offset, 37);
        assert_eq!(candidates[0].byte_length, zip.len() as u64);
        assert_eq!(
            extract_zip(&image, &candidates[0]).expect("extract ZIP"),
            zip
        );
    }

    #[test]
    fn rejects_zip_with_a_mismatched_central_directory_local_offset() {
        let mut zip = stored_zip(&[b"notes.txt"]);
        let central_header = zip
            .windows(b"PK\x01\x02".len())
            .position(|window| window == b"PK\x01\x02")
            .expect("find central directory header");
        zip[central_header + 42..central_header + 46].copy_from_slice(&99_u32.to_le_bytes());

        assert!(carve_zip_archives(&zip).is_empty());
    }

    #[test]
    fn rejects_document_extraction_outside_the_image() {
        let pdf = PdfCarvedCandidate {
            evidence_name: "carved-pdf-0000.pdf".to_owned(),
            source_offset: 16,
            byte_length: 8,
        };
        let zip = ZipCarvedCandidate {
            evidence_name: "carved-zip-0000.zip".to_owned(),
            file_type: "zip".to_owned(),
            source_offset: 16,
            byte_length: 8,
        };

        assert_eq!(
            extract_pdf(&[0_u8; 20], &pdf).expect_err("reject PDF outside range"),
            CarveError::CandidateOutsideImage
        );
        assert_eq!(
            extract_zip(&[0_u8; 20], &zip).expect_err("reject ZIP outside range"),
            CarveError::CandidateOutsideImage
        );
    }
}
