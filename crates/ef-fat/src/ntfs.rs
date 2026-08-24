use serde::{Deserialize, Serialize};
use thiserror::Error;

const NTFS_OEM_ID: &[u8; 8] = b"NTFS    ";
const NTFS_BOOT_SIGNATURE: u16 = 0xaa55;
const FILE_RECORD_SIGNATURE: &[u8; 4] = b"FILE";
const ATTRIBUTE_END: u32 = 0xffff_ffff;
const ATTRIBUTE_LIST: u32 = 0x20;
const ATTRIBUTE_FILE_NAME: u32 = 0x30;
const ATTRIBUTE_DATA: u32 = 0x80;
const FILE_RECORD_IN_USE: u16 = 0x0001;
const FILE_RECORD_DIRECTORY: u16 = 0x0002;
const RESIDENT_ATTRIBUTE_HEADER_SIZE: usize = 24;
const FILE_NAME_MINIMUM_VALUE_SIZE: usize = 66;
const MAX_MFT_RECORDS_TO_SCAN: usize = 4096;
const NTFS_BITMAP_RECORD_INDEX: usize = 6;
const NON_RESIDENT_ATTRIBUTE_HEADER_SIZE: usize = 64;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NtfsError {
    #[error("image is too small to contain an NTFS boot sector")]
    ImageTooSmall,
    #[error("NTFS OEM identifier is invalid")]
    InvalidOemIdentifier,
    #[error("NTFS boot signature is invalid")]
    InvalidBootSignature,
    #[error("unsupported NTFS bytes per sector: {0}")]
    UnsupportedBytesPerSector(u16),
    #[error("unsupported NTFS sectors per cluster: {0}")]
    UnsupportedSectorsPerCluster(u8),
    #[error("unsupported NTFS file-record size encoding: {0}")]
    UnsupportedRecordSize(i8),
    #[error("unsupported NTFS volume layout")]
    UnsupportedLayout,
    #[error("NTFS structure extends beyond the supplied image")]
    StructureOutsideImage,
    #[error("NTFS file record fixup is invalid")]
    InvalidRecordFixup,
    #[error("NTFS file record is invalid")]
    InvalidFileRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NtfsGeometry {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub volume_length_sectors: u64,
    pub mft_logical_cluster_number: u64,
    pub mft_offset: u64,
    pub file_record_size: u32,
    pub cluster_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedNtfsResidentFile {
    pub evidence_name: String,
    pub record_index: u64,
    pub record_offset: u64,
    pub data_offset_within_record: u32,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedNtfsContiguousFile {
    pub evidence_name: String,
    pub record_index: u64,
    pub record_offset: u64,
    pub first_logical_cluster: u64,
    pub cluster_count: u64,
    pub byte_length: u64,
}

#[derive(Debug, Clone)]
pub struct NtfsVolume<'a> {
    image: &'a [u8],
    geometry: NtfsGeometry,
    bytes_per_sector: usize,
    cluster_size: usize,
    file_record_size: usize,
    mft_offset: usize,
    volume_bytes: usize,
}

#[derive(Debug, Clone)]
struct ParsedResidentRecord {
    evidence_name: String,
    data_offset_within_record: usize,
    data_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SingleNtfsRun {
    first_logical_cluster: usize,
    cluster_count: usize,
    data_length: usize,
}

#[derive(Debug, Clone)]
struct NtfsAllocationBitmap {
    bytes: Vec<u8>,
    physical_run: Option<SingleNtfsRun>,
}

#[derive(Debug, Clone)]
struct ParsedNonresidentRecord {
    evidence_name: String,
    run: SingleNtfsRun,
}

impl<'a> NtfsVolume<'a> {
    pub fn parse(image: &'a [u8]) -> Result<Self, NtfsError> {
        if image.len() < 512 {
            return Err(NtfsError::ImageTooSmall);
        }
        if image.get(3..11) != Some(NTFS_OEM_ID) {
            return Err(NtfsError::InvalidOemIdentifier);
        }
        if read_u16(image, 510)? != NTFS_BOOT_SIGNATURE {
            return Err(NtfsError::InvalidBootSignature);
        }

        let bytes_per_sector = read_u16(image, 11)?;
        if !matches!(bytes_per_sector, 512 | 1024 | 2048 | 4096) {
            return Err(NtfsError::UnsupportedBytesPerSector(bytes_per_sector));
        }
        let sectors_per_cluster = *image.get(13).ok_or(NtfsError::ImageTooSmall)?;
        if sectors_per_cluster == 0
            || !sectors_per_cluster.is_power_of_two()
            || sectors_per_cluster > 128
        {
            return Err(NtfsError::UnsupportedSectorsPerCluster(sectors_per_cluster));
        }
        if image
            .get(14..21)
            .is_none_or(|bytes| bytes.iter().any(|byte| *byte != 0))
        {
            return Err(NtfsError::UnsupportedLayout);
        }

        let volume_length_sectors = read_u64(image, 40)?;
        let mft_logical_cluster_number = read_u64(image, 48)?;
        let record_size_encoding = *image.get(64).ok_or(NtfsError::ImageTooSmall)? as i8;
        let bytes_per_sector_usize = usize::from(bytes_per_sector);
        let cluster_size = bytes_per_sector_usize
            .checked_mul(usize::from(sectors_per_cluster))
            .ok_or(NtfsError::UnsupportedLayout)?;
        let file_record_size = decode_file_record_size(record_size_encoding, cluster_size)?;
        let volume_bytes = usize::try_from(volume_length_sectors)
            .ok()
            .and_then(|sectors| sectors.checked_mul(bytes_per_sector_usize))
            .ok_or(NtfsError::UnsupportedLayout)?;
        let mft_offset = usize::try_from(mft_logical_cluster_number)
            .ok()
            .and_then(|cluster| cluster.checked_mul(cluster_size))
            .ok_or(NtfsError::UnsupportedLayout)?;

        if volume_length_sectors == 0
            || mft_logical_cluster_number == 0
            || file_record_size < bytes_per_sector_usize
            || file_record_size % bytes_per_sector_usize != 0
        {
            return Err(NtfsError::UnsupportedLayout);
        }
        ensure_range(image, 0, volume_bytes)?;
        if mft_offset
            .checked_add(file_record_size)
            .map(|end| end > volume_bytes)
            .unwrap_or(true)
        {
            return Err(NtfsError::UnsupportedLayout);
        }
        ensure_range(image, mft_offset, file_record_size)?;

        Ok(Self {
            image,
            geometry: NtfsGeometry {
                bytes_per_sector,
                sectors_per_cluster,
                volume_length_sectors,
                mft_logical_cluster_number,
                mft_offset: mft_offset as u64,
                file_record_size: file_record_size as u32,
                cluster_size: cluster_size as u32,
            },
            bytes_per_sector: bytes_per_sector_usize,
            cluster_size,
            file_record_size,
            mft_offset,
            volume_bytes,
        })
    }

    pub fn geometry(&self) -> &NtfsGeometry {
        &self.geometry
    }

    pub fn find_deleted_resident_files(&self) -> Vec<DeletedNtfsResidentFile> {
        let mut files = Vec::new();
        for record_index in 0..self.records_to_scan() {
            let Ok(record_offset) = self.record_offset(record_index) else {
                break;
            };
            let raw = &self.image[record_offset..record_offset + self.file_record_size];
            if raw.iter().all(|byte| *byte == 0) {
                continue;
            }
            let Ok(record) = self.fixed_up_record(raw) else {
                continue;
            };
            let Ok(Some(parsed)) = self.parse_deleted_resident_record(&record, record_index) else {
                continue;
            };
            files.push(DeletedNtfsResidentFile {
                evidence_name: parsed.evidence_name,
                record_index: record_index as u64,
                record_offset: record_offset as u64,
                data_offset_within_record: parsed.data_offset_within_record as u32,
                byte_length: parsed.data_length as u64,
            });
        }
        files
    }

    pub fn find_deleted_contiguous_files(&self) -> Vec<DeletedNtfsContiguousFile> {
        let Ok(bitmap) = self.allocation_bitmap() else {
            return Vec::new();
        };
        let mut files = Vec::new();
        for record_index in 0..self.records_to_scan() {
            let Ok(record_offset) = self.record_offset(record_index) else {
                break;
            };
            let raw = &self.image[record_offset..record_offset + self.file_record_size];
            if raw.iter().all(|byte| *byte == 0) {
                continue;
            }
            let Ok(record) = self.fixed_up_record(raw) else {
                continue;
            };
            let Ok(Some(parsed)) = self.parse_deleted_nonresident_record(&record, record_index)
            else {
                continue;
            };
            let Ok(true) = self.nonresident_extent_is_recoverable(&parsed.run, &bitmap) else {
                continue;
            };
            files.push(DeletedNtfsContiguousFile {
                evidence_name: parsed.evidence_name,
                record_index: record_index as u64,
                record_offset: record_offset as u64,
                first_logical_cluster: parsed.run.first_logical_cluster as u64,
                cluster_count: parsed.run.cluster_count as u64,
                byte_length: parsed.run.data_length as u64,
            });
        }
        files
    }

    pub fn source_offset_for_contiguous_candidate(
        &self,
        candidate: &DeletedNtfsContiguousFile,
    ) -> Result<u64, NtfsError> {
        let parsed = self.validate_deleted_contiguous_candidate(candidate)?;
        let offset = parsed
            .run
            .first_logical_cluster
            .checked_mul(self.cluster_size)
            .ok_or(NtfsError::StructureOutsideImage)?;
        Ok(offset as u64)
    }

    pub fn read_deleted_contiguous_file(
        &self,
        candidate: &DeletedNtfsContiguousFile,
    ) -> Result<Vec<u8>, NtfsError> {
        let parsed = self.validate_deleted_contiguous_candidate(candidate)?;
        self.read_single_run(&parsed.run)
    }

    pub fn source_offset_for_candidate(
        &self,
        candidate: &DeletedNtfsResidentFile,
    ) -> Result<u64, NtfsError> {
        let parsed = self.validate_deleted_candidate(candidate)?;
        let record_offset = self.record_offset(
            usize::try_from(candidate.record_index).map_err(|_| NtfsError::InvalidFileRecord)?,
        )?;
        let source_offset = record_offset
            .checked_add(parsed.data_offset_within_record)
            .ok_or(NtfsError::InvalidFileRecord)?;
        Ok(source_offset as u64)
    }

    pub fn read_deleted_file(
        &self,
        candidate: &DeletedNtfsResidentFile,
    ) -> Result<Vec<u8>, NtfsError> {
        let parsed = self.validate_deleted_candidate(candidate)?;
        let record_offset = self.record_offset(
            usize::try_from(candidate.record_index).map_err(|_| NtfsError::InvalidFileRecord)?,
        )?;
        let raw = self
            .image
            .get(record_offset..record_offset + self.file_record_size)
            .ok_or(NtfsError::StructureOutsideImage)?;
        let fixed_record = self.fixed_up_record(raw)?;
        let end_within_record = parsed
            .data_offset_within_record
            .checked_add(parsed.data_length)
            .ok_or(NtfsError::StructureOutsideImage)?;
        Ok(fixed_record
            .get(parsed.data_offset_within_record..end_within_record)
            .ok_or(NtfsError::StructureOutsideImage)?
            .to_vec())
    }

    fn records_to_scan(&self) -> usize {
        let available = self.volume_bytes.saturating_sub(self.mft_offset) / self.file_record_size;
        available.min(MAX_MFT_RECORDS_TO_SCAN)
    }

    fn record_offset(&self, index: usize) -> Result<usize, NtfsError> {
        let offset = self
            .mft_offset
            .checked_add(
                index
                    .checked_mul(self.file_record_size)
                    .ok_or(NtfsError::StructureOutsideImage)?,
            )
            .ok_or(NtfsError::StructureOutsideImage)?;
        ensure_range(self.image, offset, self.file_record_size)?;
        if offset + self.file_record_size > self.volume_bytes {
            return Err(NtfsError::StructureOutsideImage);
        }
        Ok(offset)
    }

    fn fixed_up_record(&self, raw: &[u8]) -> Result<Vec<u8>, NtfsError> {
        if raw.len() != self.file_record_size || raw.get(0..4) != Some(FILE_RECORD_SIGNATURE) {
            return Err(NtfsError::InvalidFileRecord);
        }
        let usa_offset = usize::from(read_u16(raw, 4)?);
        let usa_count = usize::from(read_u16(raw, 6)?);
        let sector_count = self.file_record_size / self.bytes_per_sector;
        if usa_count != sector_count + 1 || usa_offset < 48 {
            return Err(NtfsError::InvalidRecordFixup);
        }
        let usa_length = usa_count
            .checked_mul(2)
            .ok_or(NtfsError::InvalidRecordFixup)?;
        if usa_offset
            .checked_add(usa_length)
            .map(|end| end > self.file_record_size)
            .unwrap_or(true)
        {
            return Err(NtfsError::InvalidRecordFixup);
        }
        let update_sequence_number = read_u16(raw, usa_offset)?;
        if update_sequence_number == 0 {
            return Err(NtfsError::InvalidRecordFixup);
        }

        let mut record = raw.to_vec();
        for sector_index in 1..=sector_count {
            let trailer_offset = sector_index
                .checked_mul(self.bytes_per_sector)
                .and_then(|offset| offset.checked_sub(2))
                .ok_or(NtfsError::InvalidRecordFixup)?;
            if read_u16(&record, trailer_offset)? != update_sequence_number {
                return Err(NtfsError::InvalidRecordFixup);
            }
            let replacement = read_u16(raw, usa_offset + sector_index * 2)?;
            record[trailer_offset..trailer_offset + 2].copy_from_slice(&replacement.to_le_bytes());
        }
        Ok(record)
    }

    fn parse_deleted_resident_record(
        &self,
        record: &[u8],
        record_index: usize,
    ) -> Result<Option<ParsedResidentRecord>, NtfsError> {
        if record.get(0..4) != Some(FILE_RECORD_SIGNATURE) {
            return Err(NtfsError::InvalidFileRecord);
        }
        let first_attribute_offset = usize::from(read_u16(record, 20)?);
        let flags = read_u16(record, 22)?;
        let used_size =
            usize::try_from(read_u32(record, 24)?).map_err(|_| NtfsError::InvalidFileRecord)?;
        let allocated_size =
            usize::try_from(read_u32(record, 28)?).map_err(|_| NtfsError::InvalidFileRecord)?;
        let base_record_reference = read_u64(record, 32)?;
        let declared_record_number =
            usize::try_from(read_u32(record, 44)?).map_err(|_| NtfsError::InvalidFileRecord)?;

        if flags & FILE_RECORD_IN_USE != 0
            || flags & FILE_RECORD_DIRECTORY != 0
            || base_record_reference != 0
            || declared_record_number != record_index
            || allocated_size != self.file_record_size
            || used_size > self.file_record_size
            || first_attribute_offset < 48
            || first_attribute_offset >= used_size
            || first_attribute_offset % 8 != 0
        {
            return Ok(None);
        }

        let mut attribute_offset = first_attribute_offset;
        let mut last_type = 0_u32;
        let mut file_name: Option<String> = None;
        let mut resident_data: Option<(usize, usize)> = None;

        loop {
            let attribute_type = read_u32(record, attribute_offset)?;
            if attribute_type == ATTRIBUTE_END {
                return match (file_name, resident_data) {
                    (Some(evidence_name), Some((data_offset_within_record, data_length))) => {
                        Ok(Some(ParsedResidentRecord {
                            evidence_name,
                            data_offset_within_record,
                            data_length,
                        }))
                    }
                    _ => Ok(None),
                };
            }
            if attribute_type < last_type {
                return Err(NtfsError::InvalidFileRecord);
            }
            last_type = attribute_type;
            let attribute_length = usize::try_from(read_u32(record, attribute_offset + 4)?)
                .map_err(|_| NtfsError::InvalidFileRecord)?;
            let non_resident = *record
                .get(attribute_offset + 8)
                .ok_or(NtfsError::InvalidFileRecord)?;
            let name_length = usize::from(
                *record
                    .get(attribute_offset + 9)
                    .ok_or(NtfsError::InvalidFileRecord)?,
            );
            let name_offset = usize::from(read_u16(record, attribute_offset + 10)?);
            if attribute_length < RESIDENT_ATTRIBUTE_HEADER_SIZE
                || attribute_length % 8 != 0
                || attribute_offset
                    .checked_add(attribute_length)
                    .map(|end| end > used_size)
                    .unwrap_or(true)
                || non_resident != 0
                || name_length != 0
                || name_offset != 0
                || attribute_type == ATTRIBUTE_LIST
            {
                return Err(NtfsError::InvalidFileRecord);
            }

            let value_length = usize::try_from(read_u32(record, attribute_offset + 16)?)
                .map_err(|_| NtfsError::InvalidFileRecord)?;
            let value_offset = usize::from(read_u16(record, attribute_offset + 20)?);
            if value_offset < RESIDENT_ATTRIBUTE_HEADER_SIZE
                || value_offset
                    .checked_add(value_length)
                    .map(|end| end > attribute_length)
                    .unwrap_or(true)
            {
                return Err(NtfsError::InvalidFileRecord);
            }
            let value_start = attribute_offset
                .checked_add(value_offset)
                .ok_or(NtfsError::InvalidFileRecord)?;

            match attribute_type {
                ATTRIBUTE_FILE_NAME => {
                    if file_name.is_some() {
                        return Ok(None);
                    }
                    file_name = Some(parse_file_name_value(
                        record
                            .get(value_start..value_start + value_length)
                            .ok_or(NtfsError::InvalidFileRecord)?,
                    )?);
                }
                ATTRIBUTE_DATA => {
                    if resident_data.is_some() {
                        return Ok(None);
                    }
                    resident_data = Some((value_start, value_length));
                }
                _ => {}
            }

            attribute_offset = attribute_offset
                .checked_add(attribute_length)
                .ok_or(NtfsError::InvalidFileRecord)?;
            if attribute_offset >= used_size {
                return Err(NtfsError::InvalidFileRecord);
            }
        }
    }

    fn parse_deleted_nonresident_record(
        &self,
        record: &[u8],
        record_index: usize,
    ) -> Result<Option<ParsedNonresidentRecord>, NtfsError> {
        let Some((first_attribute_offset, used_size)) =
            self.deleted_base_record_bounds(record, record_index)?
        else {
            return Ok(None);
        };
        let mut attribute_offset = first_attribute_offset;
        let mut last_type = 0_u32;
        let mut file_name: Option<String> = None;
        let mut data_run: Option<SingleNtfsRun> = None;

        loop {
            let attribute_type = read_u32(record, attribute_offset)?;
            if attribute_type == ATTRIBUTE_END {
                return match (file_name, data_run) {
                    (Some(evidence_name), Some(run)) => {
                        Ok(Some(ParsedNonresidentRecord { evidence_name, run }))
                    }
                    _ => Ok(None),
                };
            }
            if attribute_type < last_type || attribute_type == ATTRIBUTE_LIST {
                return Err(NtfsError::InvalidFileRecord);
            }
            last_type = attribute_type;
            let attribute_length = self.attribute_length(record, attribute_offset, used_size)?;
            let non_resident = *record
                .get(attribute_offset + 8)
                .ok_or(NtfsError::InvalidFileRecord)?;
            let name_length = usize::from(
                *record
                    .get(attribute_offset + 9)
                    .ok_or(NtfsError::InvalidFileRecord)?,
            );
            let name_offset = usize::from(read_u16(record, attribute_offset + 10)?);
            if name_length != 0 || name_offset != 0 {
                return Ok(None);
            }

            match attribute_type {
                ATTRIBUTE_FILE_NAME => {
                    if non_resident != 0 || file_name.is_some() {
                        return Ok(None);
                    }
                    let value =
                        self.resident_attribute_value(record, attribute_offset, attribute_length)?;
                    file_name = Some(parse_file_name_value(value)?);
                }
                ATTRIBUTE_DATA => {
                    if non_resident != 1 || data_run.is_some() {
                        return Ok(None);
                    }
                    data_run = Some(self.parse_single_nonresident_run(
                        record,
                        attribute_offset,
                        attribute_length,
                    )?);
                }
                _ => {
                    if non_resident != 0 {
                        return Ok(None);
                    }
                    let _ =
                        self.resident_attribute_value(record, attribute_offset, attribute_length)?;
                }
            }

            attribute_offset = attribute_offset
                .checked_add(attribute_length)
                .ok_or(NtfsError::InvalidFileRecord)?;
            if attribute_offset >= used_size {
                return Err(NtfsError::InvalidFileRecord);
            }
        }
    }

    fn deleted_base_record_bounds(
        &self,
        record: &[u8],
        record_index: usize,
    ) -> Result<Option<(usize, usize)>, NtfsError> {
        if record.get(0..4) != Some(FILE_RECORD_SIGNATURE) {
            return Err(NtfsError::InvalidFileRecord);
        }
        let first_attribute_offset = usize::from(read_u16(record, 20)?);
        let flags = read_u16(record, 22)?;
        let used_size =
            usize::try_from(read_u32(record, 24)?).map_err(|_| NtfsError::InvalidFileRecord)?;
        let allocated_size =
            usize::try_from(read_u32(record, 28)?).map_err(|_| NtfsError::InvalidFileRecord)?;
        let base_record_reference = read_u64(record, 32)?;
        let declared_record_number =
            usize::try_from(read_u32(record, 44)?).map_err(|_| NtfsError::InvalidFileRecord)?;

        if flags & FILE_RECORD_IN_USE != 0
            || flags & FILE_RECORD_DIRECTORY != 0
            || base_record_reference != 0
            || declared_record_number != record_index
            || allocated_size != self.file_record_size
            || used_size > self.file_record_size
            || first_attribute_offset < 48
            || first_attribute_offset >= used_size
            || first_attribute_offset % 8 != 0
        {
            return Ok(None);
        }
        Ok(Some((first_attribute_offset, used_size)))
    }

    fn attribute_length(
        &self,
        record: &[u8],
        attribute_offset: usize,
        used_size: usize,
    ) -> Result<usize, NtfsError> {
        let attribute_length = usize::try_from(read_u32(record, attribute_offset + 4)?)
            .map_err(|_| NtfsError::InvalidFileRecord)?;
        if attribute_length < RESIDENT_ATTRIBUTE_HEADER_SIZE
            || attribute_length % 8 != 0
            || attribute_offset
                .checked_add(attribute_length)
                .map(|end| end > used_size)
                .unwrap_or(true)
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        Ok(attribute_length)
    }

    fn resident_attribute_value<'b>(
        &self,
        record: &'b [u8],
        attribute_offset: usize,
        attribute_length: usize,
    ) -> Result<&'b [u8], NtfsError> {
        let value_length = usize::try_from(read_u32(record, attribute_offset + 16)?)
            .map_err(|_| NtfsError::InvalidFileRecord)?;
        let value_offset = usize::from(read_u16(record, attribute_offset + 20)?);
        if value_offset < RESIDENT_ATTRIBUTE_HEADER_SIZE
            || value_offset
                .checked_add(value_length)
                .map(|end| end > attribute_length)
                .unwrap_or(true)
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        let value_start = attribute_offset
            .checked_add(value_offset)
            .ok_or(NtfsError::InvalidFileRecord)?;
        record
            .get(value_start..value_start + value_length)
            .ok_or(NtfsError::InvalidFileRecord)
    }

    fn parse_single_nonresident_run(
        &self,
        record: &[u8],
        attribute_offset: usize,
        attribute_length: usize,
    ) -> Result<SingleNtfsRun, NtfsError> {
        if attribute_length < NON_RESIDENT_ATTRIBUTE_HEADER_SIZE {
            return Err(NtfsError::InvalidFileRecord);
        }
        let flags = read_u16(record, attribute_offset + 12)?;
        let starting_vcn = read_u64(record, attribute_offset + 16)?;
        let last_vcn = read_u64(record, attribute_offset + 24)?;
        let mapping_pairs_offset = usize::from(read_u16(record, attribute_offset + 32)?);
        let compression_unit = read_u16(record, attribute_offset + 34)?;
        let allocated_size = usize::try_from(read_u64(record, attribute_offset + 40)?)
            .map_err(|_| NtfsError::InvalidFileRecord)?;
        let data_size = usize::try_from(read_u64(record, attribute_offset + 48)?)
            .map_err(|_| NtfsError::InvalidFileRecord)?;
        let initialized_size = usize::try_from(read_u64(record, attribute_offset + 56)?)
            .map_err(|_| NtfsError::InvalidFileRecord)?;
        if flags != 0
            || starting_vcn != 0
            || last_vcn < starting_vcn
            || compression_unit != 0
            || mapping_pairs_offset < NON_RESIDENT_ATTRIBUTE_HEADER_SIZE
            || mapping_pairs_offset >= attribute_length
            || data_size == 0
            || initialized_size != data_size
            || allocated_size == 0
            || allocated_size % self.cluster_size != 0
        {
            return Err(NtfsError::InvalidFileRecord);
        }

        let mappings_start = attribute_offset
            .checked_add(mapping_pairs_offset)
            .ok_or(NtfsError::InvalidFileRecord)?;
        let mappings_end = attribute_offset
            .checked_add(attribute_length)
            .ok_or(NtfsError::InvalidFileRecord)?;
        let mappings = record
            .get(mappings_start..mappings_end)
            .ok_or(NtfsError::InvalidFileRecord)?;
        let header = *mappings.first().ok_or(NtfsError::InvalidFileRecord)?;
        let length_width = usize::from(header & 0x0f);
        let offset_width = usize::from(header >> 4);
        if header == 0 || !(1..=8).contains(&length_width) || !(1..=8).contains(&offset_width) {
            return Err(NtfsError::InvalidFileRecord);
        }
        let entry_length = 1_usize
            .checked_add(length_width)
            .and_then(|length| length.checked_add(offset_width))
            .ok_or(NtfsError::InvalidFileRecord)?;
        if entry_length >= mappings.len()
            || mappings[entry_length] != 0
            || mappings[entry_length + 1..].iter().any(|byte| *byte != 0)
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        let cluster_count = read_unsigned(&mappings[1..1 + length_width])?;
        let relative_lcn = read_signed(&mappings[1 + length_width..entry_length])?;
        let first_logical_cluster =
            usize::try_from(relative_lcn).map_err(|_| NtfsError::InvalidFileRecord)?;
        let expected_cluster_count =
            usize::try_from(last_vcn + 1).map_err(|_| NtfsError::InvalidFileRecord)?;
        let allocated_cluster_count = allocated_size / self.cluster_size;
        if cluster_count == 0
            || cluster_count != expected_cluster_count
            || cluster_count != allocated_cluster_count
            || data_size > allocated_size
            || first_logical_cluster == 0
            || first_logical_cluster
                .checked_add(cluster_count)
                .map(|end| end > self.volume_cluster_count())
                .unwrap_or(true)
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        Ok(SingleNtfsRun {
            first_logical_cluster,
            cluster_count,
            data_length: data_size,
        })
    }

    fn allocation_bitmap(&self) -> Result<NtfsAllocationBitmap, NtfsError> {
        let record_offset = self.record_offset(NTFS_BITMAP_RECORD_INDEX)?;
        let raw = &self.image[record_offset..record_offset + self.file_record_size];
        let record = self.fixed_up_record(raw)?;
        if record.get(0..4) != Some(FILE_RECORD_SIGNATURE)
            || usize::try_from(read_u32(&record, 44)?).map_err(|_| NtfsError::InvalidFileRecord)?
                != NTFS_BITMAP_RECORD_INDEX
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        let first_attribute_offset = usize::from(read_u16(&record, 20)?);
        let used_size =
            usize::try_from(read_u32(&record, 24)?).map_err(|_| NtfsError::InvalidFileRecord)?;
        if first_attribute_offset < 48
            || first_attribute_offset >= used_size
            || used_size > self.file_record_size
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        let mut attribute_offset = first_attribute_offset;
        let mut bitmap: Option<NtfsAllocationBitmap> = None;
        loop {
            let attribute_type = read_u32(&record, attribute_offset)?;
            if attribute_type == ATTRIBUTE_END {
                return bitmap.ok_or(NtfsError::InvalidFileRecord);
            }
            let attribute_length = self.attribute_length(&record, attribute_offset, used_size)?;
            let non_resident = *record
                .get(attribute_offset + 8)
                .ok_or(NtfsError::InvalidFileRecord)?;
            let name_length = usize::from(
                *record
                    .get(attribute_offset + 9)
                    .ok_or(NtfsError::InvalidFileRecord)?,
            );
            let name_offset = usize::from(read_u16(&record, attribute_offset + 10)?);
            if name_length != 0 || name_offset != 0 || attribute_type == ATTRIBUTE_LIST {
                return Err(NtfsError::InvalidFileRecord);
            }
            if attribute_type == ATTRIBUTE_DATA {
                if bitmap.is_some() {
                    return Err(NtfsError::InvalidFileRecord);
                }
                bitmap = Some(match non_resident {
                    0 => NtfsAllocationBitmap {
                        bytes: self
                            .resident_attribute_value(&record, attribute_offset, attribute_length)?
                            .to_vec(),
                        physical_run: None,
                    },
                    1 => {
                        let run = self.parse_single_nonresident_run(
                            &record,
                            attribute_offset,
                            attribute_length,
                        )?;
                        NtfsAllocationBitmap {
                            bytes: self.read_single_run(&run)?,
                            physical_run: Some(run),
                        }
                    }
                    _ => return Err(NtfsError::InvalidFileRecord),
                });
            }
            attribute_offset = attribute_offset
                .checked_add(attribute_length)
                .ok_or(NtfsError::InvalidFileRecord)?;
            if attribute_offset >= used_size {
                return Err(NtfsError::InvalidFileRecord);
            }
        }
    }

    fn nonresident_extent_is_recoverable(
        &self,
        run: &SingleNtfsRun,
        bitmap: &NtfsAllocationBitmap,
    ) -> Result<bool, NtfsError> {
        let required_bitmap_bytes = self.volume_cluster_count().div_ceil(8);
        if bitmap.bytes.len() < required_bitmap_bytes || self.runs_overlap_mft(run) {
            return Ok(false);
        }
        if let Some(bitmap_run) = &bitmap.physical_run {
            if runs_overlap(run, bitmap_run)? {
                return Ok(false);
            }
        }
        for logical_cluster in
            run.first_logical_cluster..run.first_logical_cluster + run.cluster_count
        {
            let bitmap_byte = *bitmap
                .bytes
                .get(logical_cluster / 8)
                .ok_or(NtfsError::StructureOutsideImage)?;
            if bitmap_byte & (1 << (logical_cluster % 8)) != 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn runs_overlap_mft(&self, run: &SingleNtfsRun) -> bool {
        let mft_first_cluster = self.mft_offset / self.cluster_size;
        let protected_record_count = NTFS_BITMAP_RECORD_INDEX + 1;
        let mft_core_bytes = protected_record_count.saturating_mul(self.file_record_size);
        let mft_core_cluster_count = mft_core_bytes
            .checked_add(self.cluster_size - 1)
            .map(|size| size / self.cluster_size)
            .unwrap_or(usize::MAX);
        run.first_logical_cluster < mft_first_cluster.saturating_add(mft_core_cluster_count)
            && mft_first_cluster < run.first_logical_cluster.saturating_add(run.cluster_count)
    }

    fn read_single_run(&self, run: &SingleNtfsRun) -> Result<Vec<u8>, NtfsError> {
        let start = run
            .first_logical_cluster
            .checked_mul(self.cluster_size)
            .ok_or(NtfsError::StructureOutsideImage)?;
        let end = start
            .checked_add(run.data_length)
            .ok_or(NtfsError::StructureOutsideImage)?;
        Ok(self
            .image
            .get(start..end)
            .ok_or(NtfsError::StructureOutsideImage)?
            .to_vec())
    }

    fn volume_cluster_count(&self) -> usize {
        self.volume_bytes / self.cluster_size
    }

    fn validate_deleted_contiguous_candidate(
        &self,
        candidate: &DeletedNtfsContiguousFile,
    ) -> Result<ParsedNonresidentRecord, NtfsError> {
        let record_index =
            usize::try_from(candidate.record_index).map_err(|_| NtfsError::InvalidFileRecord)?;
        let record_offset = self.record_offset(record_index)?;
        let raw = &self.image[record_offset..record_offset + self.file_record_size];
        let record = self.fixed_up_record(raw)?;
        let parsed = self
            .parse_deleted_nonresident_record(&record, record_index)?
            .ok_or(NtfsError::InvalidFileRecord)?;
        let bitmap = self.allocation_bitmap()?;
        if parsed.evidence_name != candidate.evidence_name
            || parsed.run.first_logical_cluster
                != usize::try_from(candidate.first_logical_cluster)
                    .map_err(|_| NtfsError::InvalidFileRecord)?
            || parsed.run.cluster_count
                != usize::try_from(candidate.cluster_count)
                    .map_err(|_| NtfsError::InvalidFileRecord)?
            || parsed.run.data_length
                != usize::try_from(candidate.byte_length)
                    .map_err(|_| NtfsError::InvalidFileRecord)?
            || candidate.record_offset != record_offset as u64
            || !self.nonresident_extent_is_recoverable(&parsed.run, &bitmap)?
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        Ok(parsed)
    }

    fn validate_deleted_candidate(
        &self,
        candidate: &DeletedNtfsResidentFile,
    ) -> Result<ParsedResidentRecord, NtfsError> {
        let record_index =
            usize::try_from(candidate.record_index).map_err(|_| NtfsError::InvalidFileRecord)?;
        let record_offset = self.record_offset(record_index)?;
        let raw = &self.image[record_offset..record_offset + self.file_record_size];
        let record = self.fixed_up_record(raw)?;
        let parsed = self
            .parse_deleted_resident_record(&record, record_index)?
            .ok_or(NtfsError::InvalidFileRecord)?;
        if parsed.evidence_name != candidate.evidence_name
            || parsed.data_offset_within_record
                != usize::try_from(candidate.data_offset_within_record)
                    .map_err(|_| NtfsError::InvalidFileRecord)?
            || parsed.data_length
                != usize::try_from(candidate.byte_length)
                    .map_err(|_| NtfsError::InvalidFileRecord)?
            || candidate.record_offset != record_offset as u64
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        Ok(parsed)
    }
}

fn decode_file_record_size(encoding: i8, cluster_size: usize) -> Result<usize, NtfsError> {
    let size = if encoding > 0 {
        cluster_size
            .checked_mul(
                usize::try_from(encoding)
                    .map_err(|_| NtfsError::UnsupportedRecordSize(encoding))?,
            )
            .ok_or(NtfsError::UnsupportedRecordSize(encoding))?
    } else if encoding < 0 {
        let exponent = u32::try_from(-i16::from(encoding))
            .map_err(|_| NtfsError::UnsupportedRecordSize(encoding))?;
        1_usize
            .checked_shl(exponent)
            .ok_or(NtfsError::UnsupportedRecordSize(encoding))?
    } else {
        return Err(NtfsError::UnsupportedRecordSize(encoding));
    };
    if !(512..=65_536).contains(&size) {
        return Err(NtfsError::UnsupportedRecordSize(encoding));
    }
    Ok(size)
}

fn read_unsigned(bytes: &[u8]) -> Result<usize, NtfsError> {
    if bytes.is_empty() || bytes.len() > 8 {
        return Err(NtfsError::InvalidFileRecord);
    }
    let mut value = 0_u64;
    for (index, byte) in bytes.iter().copied().enumerate() {
        value |= u64::from(byte) << (index * 8);
    }
    usize::try_from(value).map_err(|_| NtfsError::InvalidFileRecord)
}

fn read_signed(bytes: &[u8]) -> Result<i64, NtfsError> {
    if bytes.is_empty() || bytes.len() > 8 {
        return Err(NtfsError::InvalidFileRecord);
    }
    let mut raw = 0_u64;
    for (index, byte) in bytes.iter().copied().enumerate() {
        raw |= u64::from(byte) << (index * 8);
    }
    let bit_count = bytes.len() * 8;
    let sign_extended = if bit_count < 64 && raw & (1_u64 << (bit_count - 1)) != 0 {
        raw | (!0_u64 << bit_count)
    } else {
        raw
    };
    Ok(sign_extended as i64)
}

fn runs_overlap(left: &SingleNtfsRun, right: &SingleNtfsRun) -> Result<bool, NtfsError> {
    let left_end = left
        .first_logical_cluster
        .checked_add(left.cluster_count)
        .ok_or(NtfsError::InvalidFileRecord)?;
    let right_end = right
        .first_logical_cluster
        .checked_add(right.cluster_count)
        .ok_or(NtfsError::InvalidFileRecord)?;
    Ok(left.first_logical_cluster < right_end && right.first_logical_cluster < left_end)
}

fn parse_file_name_value(value: &[u8]) -> Result<String, NtfsError> {
    if value.len() < FILE_NAME_MINIMUM_VALUE_SIZE {
        return Err(NtfsError::InvalidFileRecord);
    }
    let name_length = usize::from(value[64]);
    if name_length == 0 || name_length > 255 || value[65] > 3 {
        return Err(NtfsError::InvalidFileRecord);
    }
    let byte_length = name_length
        .checked_mul(2)
        .ok_or(NtfsError::InvalidFileRecord)?;
    let name_bytes = value
        .get(66..66 + byte_length)
        .ok_or(NtfsError::InvalidFileRecord)?;
    let mut code_units = Vec::with_capacity(name_length);
    for bytes in name_bytes.chunks_exact(2) {
        let code_unit = u16::from_le_bytes([bytes[0], bytes[1]]);
        if code_unit == 0
            || code_unit < 0x20
            || matches!(
                code_unit,
                0x0022 | 0x002a | 0x002f | 0x003a | 0x003c | 0x003e | 0x003f | 0x005c | 0x007c
            )
        {
            return Err(NtfsError::InvalidFileRecord);
        }
        code_units.push(code_unit);
    }
    let name = String::from_utf16(&code_units).map_err(|_| NtfsError::InvalidFileRecord)?;
    if matches!(name.as_str(), "." | "..") {
        return Err(NtfsError::InvalidFileRecord);
    }
    Ok(name)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, NtfsError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or(NtfsError::StructureOutsideImage)?;
    Ok(u16::from_le_bytes(
        value.try_into().expect("fixed u16 range"),
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, NtfsError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(NtfsError::StructureOutsideImage)?;
    Ok(u32::from_le_bytes(
        value.try_into().expect("fixed u32 range"),
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, NtfsError> {
    let value = bytes
        .get(offset..offset + 8)
        .ok_or(NtfsError::StructureOutsideImage)?;
    Ok(u64::from_le_bytes(
        value.try_into().expect("fixed u64 range"),
    ))
}

fn ensure_range(bytes: &[u8], offset: usize, length: usize) -> Result<(), NtfsError> {
    if offset
        .checked_add(length)
        .map(|end| end <= bytes.len())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(NtfsError::StructureOutsideImage)
    }
}

#[cfg(test)]
mod tests {
    use super::{DeletedNtfsContiguousFile, DeletedNtfsResidentFile, NtfsError, NtfsVolume};

    const BYTES_PER_SECTOR: usize = 512;
    const VOLUME_SECTORS: usize = 4096;
    const MFT_CLUSTER: usize = 4;
    const RECORD_SIZE: usize = 1024;
    const NAME: &str = "gone.txt";
    const CONTENT: &[u8] = b"ntfs recovered\n";

    fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn resident_attribute(attribute_type: u32, value: &[u8], instance: u16) -> Vec<u8> {
        let length = (24 + value.len()).next_multiple_of(8);
        let mut attribute = vec![0_u8; length];
        write_u32(&mut attribute, 0, attribute_type);
        write_u32(&mut attribute, 4, length as u32);
        attribute[8] = 0;
        write_u16(&mut attribute, 10, 0);
        write_u16(&mut attribute, 12, 0);
        write_u16(&mut attribute, 14, instance);
        write_u32(&mut attribute, 16, value.len() as u32);
        write_u16(&mut attribute, 20, 24);
        attribute[24..24 + value.len()].copy_from_slice(value);
        attribute
    }

    fn nonresident_single_run_attribute(
        first_logical_cluster: u8,
        data_length: usize,
        instance: u16,
    ) -> Vec<u8> {
        let mut attribute = vec![0_u8; 72];
        write_u32(&mut attribute, 0, 0x80);
        write_u32(&mut attribute, 4, 72);
        attribute[8] = 1;
        write_u16(&mut attribute, 10, 0);
        write_u16(&mut attribute, 12, 0);
        write_u16(&mut attribute, 14, instance);
        write_u64(&mut attribute, 16, 0);
        write_u64(&mut attribute, 24, 0);
        write_u16(&mut attribute, 32, 64);
        write_u16(&mut attribute, 34, 0);
        write_u64(&mut attribute, 40, BYTES_PER_SECTOR as u64);
        write_u64(&mut attribute, 48, data_length as u64);
        write_u64(&mut attribute, 56, data_length as u64);
        attribute[64] = 0x11;
        attribute[65] = 1;
        attribute[66] = first_logical_cluster;
        attribute
    }

    fn fixed_up_record(record_number: u32, flags: u16, attributes: &[u8]) -> Vec<u8> {
        let first_attribute_offset = 56;
        let used_size = first_attribute_offset + attributes.len() + 4;
        let mut record = vec![0_u8; RECORD_SIZE];
        record[0..4].copy_from_slice(b"FILE");
        write_u16(&mut record, 4, 48);
        write_u16(&mut record, 6, 3);
        write_u16(&mut record, 16, 1);
        write_u16(&mut record, 20, first_attribute_offset as u16);
        write_u16(&mut record, 22, flags);
        write_u32(&mut record, 24, used_size as u32);
        write_u32(&mut record, 28, RECORD_SIZE as u32);
        write_u64(&mut record, 32, 0);
        write_u16(&mut record, 40, 2);
        write_u32(&mut record, 44, record_number);
        record[first_attribute_offset..first_attribute_offset + attributes.len()]
            .copy_from_slice(attributes);
        write_u32(
            &mut record,
            first_attribute_offset + attributes.len(),
            0xffff_ffff,
        );

        let first_trailer =
            u16::from_le_bytes([record[BYTES_PER_SECTOR - 2], record[BYTES_PER_SECTOR - 1]]);
        let second_trailer = u16::from_le_bytes([record[RECORD_SIZE - 2], record[RECORD_SIZE - 1]]);
        write_u16(&mut record, 48, 0xa5a5);
        write_u16(&mut record, 50, first_trailer);
        write_u16(&mut record, 52, second_trailer);
        write_u16(&mut record, BYTES_PER_SECTOR - 2, 0xa5a5);
        write_u16(&mut record, RECORD_SIZE - 2, 0xa5a5);
        record
    }

    fn deleted_record() -> Vec<u8> {
        let mut file_name_value = vec![0_u8; 66 + NAME.len() * 2];
        file_name_value[64] = NAME.len() as u8;
        file_name_value[65] = 1;
        for (index, code_unit) in NAME.encode_utf16().enumerate() {
            let offset = 66 + index * 2;
            file_name_value[offset..offset + 2].copy_from_slice(&code_unit.to_le_bytes());
        }
        let mut attributes = resident_attribute(0x30, &file_name_value, 0);
        attributes.extend_from_slice(&resident_attribute(0x80, CONTENT, 1));
        fixed_up_record(1, 0, &attributes)
    }

    fn sample_nonresident_ntfs_image() -> Vec<u8> {
        const BITMAP_RECORD: usize = 6;
        const DELETED_RECORD: usize = 7;
        const DATA_CLUSTER: usize = 64;
        let mut image = sample_ntfs_image();
        let mft_offset = MFT_CLUSTER * BYTES_PER_SECTOR;
        let mut bitmap = vec![0_u8; VOLUME_SECTORS / 8];
        for cluster in MFT_CLUSTER..MFT_CLUSTER + 16 {
            bitmap[cluster / 8] |= 1 << (cluster % 8);
        }
        let bitmap_attributes = resident_attribute(0x80, &bitmap, 0);
        image[mft_offset + BITMAP_RECORD * RECORD_SIZE
            ..mft_offset + (BITMAP_RECORD + 1) * RECORD_SIZE]
            .copy_from_slice(&fixed_up_record(
                BITMAP_RECORD as u32,
                1,
                &bitmap_attributes,
            ));

        let mut file_name_value = vec![0_u8; 66 + NAME.len() * 2];
        file_name_value[64] = NAME.len() as u8;
        file_name_value[65] = 1;
        for (index, code_unit) in NAME.encode_utf16().enumerate() {
            let offset = 66 + index * 2;
            file_name_value[offset..offset + 2].copy_from_slice(&code_unit.to_le_bytes());
        }
        let mut attributes = resident_attribute(0x30, &file_name_value, 0);
        attributes.extend_from_slice(&nonresident_single_run_attribute(
            DATA_CLUSTER as u8,
            CONTENT.len(),
            1,
        ));
        image[mft_offset + DELETED_RECORD * RECORD_SIZE
            ..mft_offset + (DELETED_RECORD + 1) * RECORD_SIZE]
            .copy_from_slice(&fixed_up_record(DELETED_RECORD as u32, 0, &attributes));
        let data_offset = DATA_CLUSTER * BYTES_PER_SECTOR;
        image[data_offset..data_offset + CONTENT.len()].copy_from_slice(CONTENT);
        image
    }

    fn sample_ntfs_image() -> Vec<u8> {
        let mut image = vec![0_u8; VOLUME_SECTORS * BYTES_PER_SECTOR];
        image[0..3].copy_from_slice(b"\xeb\x52\x90");
        image[3..11].copy_from_slice(b"NTFS    ");
        write_u16(&mut image, 11, BYTES_PER_SECTOR as u16);
        image[13] = 1;
        image[21] = 0xf8;
        write_u64(&mut image, 40, VOLUME_SECTORS as u64);
        write_u64(&mut image, 48, MFT_CLUSTER as u64);
        write_u64(&mut image, 56, MFT_CLUSTER as u64 + 1);
        image[64] = 0xf6;
        image[68] = 0xf6;
        write_u16(&mut image, 510, 0xaa55);

        let mft_offset = MFT_CLUSTER * BYTES_PER_SECTOR;
        let active = fixed_up_record(0, 1, &[]);
        let deleted = deleted_record();
        image[mft_offset..mft_offset + RECORD_SIZE].copy_from_slice(&active);
        image[mft_offset + RECORD_SIZE..mft_offset + RECORD_SIZE * 2].copy_from_slice(&deleted);
        image
    }

    #[test]
    fn recovers_a_deleted_fixed_up_resident_file() {
        let image = sample_ntfs_image();
        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        let candidate = volume
            .find_deleted_resident_files()
            .into_iter()
            .next()
            .expect("find deleted resident record");

        assert_eq!(volume.geometry().mft_offset, 2048);
        assert_eq!(volume.geometry().file_record_size, 1024);
        assert_eq!(candidate.evidence_name, NAME);
        assert_eq!(candidate.record_index, 1);
        assert_eq!(
            volume
                .source_offset_for_candidate(&candidate)
                .expect("locate resident data"),
            3264
        );
        assert_eq!(
            volume
                .read_deleted_file(&candidate)
                .expect("extract resident data"),
            CONTENT
        );
    }

    #[test]
    fn ignores_a_record_with_an_invalid_fixup() {
        let mut image = sample_ntfs_image();
        let second_record_offset = MFT_CLUSTER * BYTES_PER_SECTOR + RECORD_SIZE;
        image[second_record_offset + RECORD_SIZE - 2] = 0;

        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        assert!(volume.find_deleted_resident_files().is_empty());
    }

    #[test]
    fn refuses_non_resident_data_attributes() {
        let mut image = sample_ntfs_image();
        let second_record_offset = MFT_CLUSTER * BYTES_PER_SECTOR + RECORD_SIZE;
        let data_attribute_offset = second_record_offset + 56 + 112;
        image[data_attribute_offset + 8] = 1;

        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        assert!(volume.find_deleted_resident_files().is_empty());
    }

    #[test]
    fn restores_resident_content_that_crosses_a_fixup_protected_sector_trailer() {
        let content = vec![b'R'; 320];
        let mut file_name_value = vec![0_u8; 66 + NAME.len() * 2];
        file_name_value[64] = NAME.len() as u8;
        file_name_value[65] = 1;
        for (index, code_unit) in NAME.encode_utf16().enumerate() {
            let offset = 66 + index * 2;
            file_name_value[offset..offset + 2].copy_from_slice(&code_unit.to_le_bytes());
        }
        let mut attributes = resident_attribute(0x30, &file_name_value, 0);
        attributes.extend_from_slice(&resident_attribute(0x80, &content, 1));
        let record = fixed_up_record(1, 0, &attributes);
        let mut image = sample_ntfs_image();
        let second_record_offset = MFT_CLUSTER * BYTES_PER_SECTOR + RECORD_SIZE;
        image[second_record_offset..second_record_offset + RECORD_SIZE].copy_from_slice(&record);

        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        let candidate = volume
            .find_deleted_resident_files()
            .into_iter()
            .next()
            .expect("find resident candidate");

        assert_eq!(
            volume
                .read_deleted_file(&candidate)
                .expect("recover content"),
            content
        );
    }

    #[test]
    fn recovers_a_deleted_nonresident_single_run_when_its_clusters_are_free() {
        let image = sample_nonresident_ntfs_image();
        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        let candidate = volume
            .find_deleted_contiguous_files()
            .into_iter()
            .next()
            .expect("find deleted non-resident candidate");

        assert_eq!(candidate.evidence_name, NAME);
        assert_eq!(candidate.record_index, 7);
        assert_eq!(candidate.first_logical_cluster, 64);
        assert_eq!(candidate.cluster_count, 1);
        assert_eq!(
            volume
                .source_offset_for_contiguous_candidate(&candidate)
                .expect("locate run"),
            32768
        );
        assert_eq!(
            volume
                .read_deleted_contiguous_file(&candidate)
                .expect("extract run"),
            CONTENT
        );
    }

    #[test]
    fn ignores_a_nonresident_candidate_after_its_cluster_is_reallocated() {
        let mut image = sample_nonresident_ntfs_image();
        let bitmap_value_offset = MFT_CLUSTER * BYTES_PER_SECTOR + 6 * RECORD_SIZE + 56 + 24;
        image[bitmap_value_offset + 64 / 8] |= 1 << (64 % 8);

        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        assert!(volume.find_deleted_contiguous_files().is_empty());
    }

    #[test]
    fn refuses_a_nonresident_record_with_an_unterminated_second_run() {
        let mut image = sample_nonresident_ntfs_image();
        let data_attribute_offset = MFT_CLUSTER * BYTES_PER_SECTOR + 7 * RECORD_SIZE + 56 + 112;
        image[data_attribute_offset + 67] = 0x11;

        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        assert!(volume.find_deleted_contiguous_files().is_empty());
    }

    #[test]
    fn rejects_an_unrelated_nonresident_candidate_during_extraction() {
        let image = sample_nonresident_ntfs_image();
        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        let candidate = DeletedNtfsContiguousFile {
            evidence_name: NAME.to_owned(),
            record_index: 7,
            record_offset: 9216,
            first_logical_cluster: 64,
            cluster_count: 1,
            byte_length: 0,
        };

        assert!(volume.read_deleted_contiguous_file(&candidate).is_err());
    }

    #[test]
    fn rejects_an_invalid_ntfs_boot_signature() {
        let mut image = sample_ntfs_image();
        image[510] = 0;

        assert_eq!(
            NtfsVolume::parse(&image).expect_err("reject boot signature"),
            NtfsError::InvalidBootSignature
        );
    }

    #[test]
    fn rejects_an_unrelated_candidate_during_extraction() {
        let image = sample_ntfs_image();
        let volume = NtfsVolume::parse(&image).expect("parse NTFS image");
        let candidate = DeletedNtfsResidentFile {
            evidence_name: NAME.to_owned(),
            record_index: 1,
            record_offset: 3072,
            data_offset_within_record: 0,
            byte_length: CONTENT.len() as u64,
        };

        assert!(volume.read_deleted_file(&candidate).is_err());
    }
}
