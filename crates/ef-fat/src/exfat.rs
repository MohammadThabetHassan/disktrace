use serde::{Deserialize, Serialize};
use thiserror::Error;

const EXFAT_FILESYSTEM_NAME: &[u8; 8] = b"EXFAT   ";
const EXFAT_BOOT_SIGNATURE: u16 = 0xaa55;
const EXFAT_EXTENDED_BOOT_SIGNATURE: u32 = 0xaa55_0000;
const EXFAT_MAIN_BOOT_REGION_SECTORS: usize = 12;
const EXFAT_FAT_EOC_MIN: u32 = 0xffff_fff8;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExfatError {
    #[error("image is too small to contain an exFAT main boot region")]
    ImageTooSmall,
    #[error("exFAT file system name is invalid")]
    InvalidFilesystemName,
    #[error("exFAT boot signature is invalid")]
    InvalidBootSignature,
    #[error("exFAT extended boot signature is invalid")]
    InvalidExtendedBootSignature,
    #[error("exFAT main boot checksum is invalid")]
    InvalidBootChecksum,
    #[error("unsupported exFAT sector shift: {0}")]
    UnsupportedBytesPerSectorShift(u8),
    #[error("unsupported exFAT cluster shift: {0}")]
    UnsupportedSectorsPerClusterShift(u8),
    #[error("unsupported exFAT layout")]
    UnsupportedLayout,
    #[error("exFAT structure extends beyond the supplied image")]
    StructureOutsideImage,
    #[error("invalid exFAT cluster chain")]
    InvalidClusterChain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExfatGeometry {
    pub bytes_per_sector: u32,
    pub sectors_per_cluster: u32,
    pub volume_length_sectors: u64,
    pub fat_offset_sectors: u32,
    pub fat_length_sectors: u32,
    pub cluster_heap_offset_sectors: u32,
    pub cluster_count: u32,
    pub root_directory_first_cluster: u32,
    pub root_directory_offset: u64,
    pub allocation_bitmap_offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedExfatRootFile {
    pub evidence_name: String,
    pub attributes: u16,
    pub first_cluster: u32,
    pub byte_length: u64,
    pub directory_entry_offset: u64,
}

#[derive(Debug, Clone)]
pub struct ExfatVolume<'a> {
    image: &'a [u8],
    geometry: ExfatGeometry,
    cluster_size: usize,
    fat_offset: usize,
    fat_length: usize,
    cluster_heap_offset: usize,
}

impl<'a> ExfatVolume<'a> {
    pub fn parse(image: &'a [u8]) -> Result<Self, ExfatError> {
        if image.len() < 512 {
            return Err(ExfatError::ImageTooSmall);
        }
        if image.get(3..11) != Some(EXFAT_FILESYSTEM_NAME) {
            return Err(ExfatError::InvalidFilesystemName);
        }
        if read_u16(image, 510)? != EXFAT_BOOT_SIGNATURE {
            return Err(ExfatError::InvalidBootSignature);
        }
        if image
            .get(11..64)
            .is_none_or(|bytes| bytes.iter().any(|byte| *byte != 0))
        {
            return Err(ExfatError::UnsupportedLayout);
        }

        let bytes_per_sector_shift = *image.get(108).ok_or(ExfatError::ImageTooSmall)?;
        if !(9..=12).contains(&bytes_per_sector_shift) {
            return Err(ExfatError::UnsupportedBytesPerSectorShift(
                bytes_per_sector_shift,
            ));
        }
        let bytes_per_sector = 1_usize
            .checked_shl(u32::from(bytes_per_sector_shift))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let main_boot_region_length = bytes_per_sector
            .checked_mul(EXFAT_MAIN_BOOT_REGION_SECTORS)
            .ok_or(ExfatError::UnsupportedLayout)?;
        ensure_range(image, 0, main_boot_region_length)?;

        for sector_index in 1_usize..=8 {
            let signature_offset = sector_index
                .checked_mul(bytes_per_sector)
                .and_then(|offset| offset.checked_add(bytes_per_sector - 4))
                .ok_or(ExfatError::UnsupportedLayout)?;
            if read_u32(image, signature_offset)? != EXFAT_EXTENDED_BOOT_SIGNATURE {
                return Err(ExfatError::InvalidExtendedBootSignature);
            }
        }
        validate_main_boot_checksum(image, bytes_per_sector)?;

        let volume_length_sectors = read_u64(image, 72)?;
        let fat_offset_sectors = read_u32(image, 80)?;
        let fat_length_sectors = read_u32(image, 84)?;
        let cluster_heap_offset_sectors = read_u32(image, 88)?;
        let cluster_count = read_u32(image, 92)?;
        let root_directory_first_cluster = read_u32(image, 96)?;
        let filesystem_revision = read_u16(image, 104)?;
        let sectors_per_cluster_shift = *image.get(109).ok_or(ExfatError::ImageTooSmall)?;
        let number_of_fats = *image.get(110).ok_or(ExfatError::ImageTooSmall)?;

        if filesystem_revision >> 8 != 1 || number_of_fats != 1 {
            return Err(ExfatError::UnsupportedLayout);
        }
        let maximum_cluster_shift = 25_u8
            .checked_sub(bytes_per_sector_shift)
            .ok_or(ExfatError::UnsupportedLayout)?;
        if sectors_per_cluster_shift > maximum_cluster_shift {
            return Err(ExfatError::UnsupportedSectorsPerClusterShift(
                sectors_per_cluster_shift,
            ));
        }
        let sectors_per_cluster = 1_usize
            .checked_shl(u32::from(sectors_per_cluster_shift))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let cluster_size = bytes_per_sector
            .checked_mul(sectors_per_cluster)
            .ok_or(ExfatError::UnsupportedLayout)?;
        let minimum_volume_length_sectors = u64::try_from(1_048_576_usize / bytes_per_sector)
            .map_err(|_| ExfatError::UnsupportedLayout)?;
        if volume_length_sectors < minimum_volume_length_sectors
            || fat_offset_sectors < 24
            || fat_length_sectors == 0
            || cluster_count == 0
            || root_directory_first_cluster < 2
            || root_directory_first_cluster > cluster_count.saturating_add(1)
        {
            return Err(ExfatError::UnsupportedLayout);
        }

        let volume_bytes = usize::try_from(volume_length_sectors)
            .ok()
            .and_then(|sectors| sectors.checked_mul(bytes_per_sector))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let fat_offset = usize::try_from(fat_offset_sectors)
            .ok()
            .and_then(|sectors| sectors.checked_mul(bytes_per_sector))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let fat_length = usize::try_from(fat_length_sectors)
            .ok()
            .and_then(|sectors| sectors.checked_mul(bytes_per_sector))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let cluster_heap_offset = usize::try_from(cluster_heap_offset_sectors)
            .ok()
            .and_then(|sectors| sectors.checked_mul(bytes_per_sector))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let required_fat_length = usize::try_from(cluster_count)
            .ok()
            .and_then(|count| count.checked_add(2))
            .and_then(|entries| entries.checked_mul(4))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let minimum_cluster_heap_sector = u64::from(fat_offset_sectors)
            .checked_add(u64::from(fat_length_sectors))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let cluster_heap_end = usize::try_from(cluster_count)
            .ok()
            .and_then(|count| count.checked_mul(cluster_size))
            .and_then(|length| cluster_heap_offset.checked_add(length))
            .ok_or(ExfatError::UnsupportedLayout)?;
        let maximum_cluster_count = volume_length_sectors
            .checked_sub(u64::from(cluster_heap_offset_sectors))
            .and_then(|sectors| sectors.checked_div(u64::try_from(sectors_per_cluster).ok()?))
            .ok_or(ExfatError::UnsupportedLayout)?;

        if fat_length < required_fat_length
            || u64::from(cluster_heap_offset_sectors) < minimum_cluster_heap_sector
            || cluster_heap_end > volume_bytes
            || u64::from(cluster_count) != maximum_cluster_count
        {
            return Err(ExfatError::UnsupportedLayout);
        }

        ensure_range(image, 0, volume_bytes)?;
        ensure_range(image, fat_offset, fat_length)?;
        ensure_range(image, cluster_heap_offset, cluster_size)?;
        let root_directory_offset = cluster_offset(
            cluster_heap_offset,
            cluster_size,
            root_directory_first_cluster,
            cluster_count,
        )?;

        Ok(Self {
            image,
            geometry: ExfatGeometry {
                bytes_per_sector: bytes_per_sector as u32,
                sectors_per_cluster: sectors_per_cluster as u32,
                volume_length_sectors,
                fat_offset_sectors,
                fat_length_sectors,
                cluster_heap_offset_sectors,
                cluster_count,
                root_directory_first_cluster,
                root_directory_offset: root_directory_offset as u64,
                allocation_bitmap_offset: None,
            },
            cluster_size,
            fat_offset,
            fat_length,
            cluster_heap_offset,
        })
    }

    pub fn geometry(&self) -> &ExfatGeometry {
        &self.geometry
    }

    fn cluster_offset(&self, cluster: u32) -> Result<usize, ExfatError> {
        cluster_offset(
            self.cluster_heap_offset,
            self.cluster_size,
            cluster,
            self.geometry.cluster_count,
        )
    }

    fn next_cluster(&self, cluster: u32) -> Result<u32, ExfatError> {
        if cluster < 2 || cluster > self.geometry.cluster_count.saturating_add(1) {
            return Err(ExfatError::InvalidClusterChain);
        }
        let entry_offset = usize::try_from(cluster)
            .ok()
            .and_then(|index| index.checked_mul(4))
            .ok_or(ExfatError::InvalidClusterChain)?;
        if entry_offset + 4 > self.fat_length {
            return Err(ExfatError::InvalidClusterChain);
        }
        read_u32(self.image, self.fat_offset + entry_offset)
            .map_err(|_| ExfatError::InvalidClusterChain)
    }

    fn root_directory_clusters(&self) -> Result<Vec<u32>, ExfatError> {
        let mut clusters = Vec::new();
        let mut cluster = self.geometry.root_directory_first_cluster;

        for _ in 0..=self.geometry.cluster_count {
            if cluster < 2
                || cluster > self.geometry.cluster_count.saturating_add(1)
                || clusters.contains(&cluster)
            {
                return Err(ExfatError::InvalidClusterChain);
            }
            clusters.push(cluster);
            let next = self.next_cluster(cluster)?;
            if next >= EXFAT_FAT_EOC_MIN {
                return Ok(clusters);
            }
            cluster = next;
        }

        Err(ExfatError::InvalidClusterChain)
    }

    pub fn find_deleted_root_files(&self) -> Vec<DeletedExfatRootFile> {
        let Ok(entries) = self.root_directory_entries() else {
            return Vec::new();
        };
        let Ok(allocation_bitmap) = self.allocation_bitmap(&entries) else {
            return Vec::new();
        };
        let mut files = Vec::new();
        let mut index = 0;

        while index < entries.len() {
            let (entry_offset, entry) = entries[index];
            if entry[0] == 0x00 {
                break;
            }
            if entry[0] != EXFAT_DELETED_FILE_ENTRY || index + 2 >= entries.len() {
                index += 1;
                continue;
            }

            let entry_set = [entry, entries[index + 1].1, entries[index + 2].1];
            if let Some(candidate) =
                self.parse_deleted_file_entry_set(entry_offset, entry_set, &allocation_bitmap)
            {
                files.push(candidate);
                index += 3;
            } else {
                index += 1;
            }
        }

        files
    }

    pub fn source_offset_for_candidate(
        &self,
        candidate: &DeletedExfatRootFile,
    ) -> Result<u64, ExfatError> {
        self.validate_deleted_candidate(candidate)?;
        Ok(self.cluster_offset(candidate.first_cluster)? as u64)
    }

    pub fn read_deleted_file(
        &self,
        candidate: &DeletedExfatRootFile,
    ) -> Result<Vec<u8>, ExfatError> {
        self.validate_deleted_candidate(candidate)?;
        let start = self.cluster_offset(candidate.first_cluster)?;
        let length =
            usize::try_from(candidate.byte_length).map_err(|_| ExfatError::InvalidClusterChain)?;
        let end = start
            .checked_add(length)
            .ok_or(ExfatError::StructureOutsideImage)?;
        let bytes = self
            .image
            .get(start..end)
            .ok_or(ExfatError::StructureOutsideImage)?;
        Ok(bytes.to_vec())
    }

    fn root_directory_entries(
        &self,
    ) -> Result<Vec<(usize, &[u8; DIRECTORY_ENTRY_SIZE])>, ExfatError> {
        let mut entries = Vec::new();
        for cluster in self.root_directory_clusters()? {
            let cluster_offset = self.cluster_offset(cluster)?;
            let cluster_bytes = self
                .image
                .get(cluster_offset..cluster_offset + self.cluster_size)
                .ok_or(ExfatError::StructureOutsideImage)?;
            for entry_offset in (0..self.cluster_size).step_by(DIRECTORY_ENTRY_SIZE) {
                let entry: &[u8; DIRECTORY_ENTRY_SIZE] = cluster_bytes
                    .get(entry_offset..entry_offset + DIRECTORY_ENTRY_SIZE)
                    .ok_or(ExfatError::StructureOutsideImage)?
                    .try_into()
                    .map_err(|_| ExfatError::StructureOutsideImage)?;
                entries.push((cluster_offset + entry_offset, entry));
                if entry[0] == 0x00 {
                    return Ok(entries);
                }
            }
        }
        Ok(entries)
    }

    fn allocation_bitmap(
        &self,
        entries: &[(usize, &[u8; DIRECTORY_ENTRY_SIZE])],
    ) -> Result<Vec<u8>, ExfatError> {
        for &(_, entry) in entries {
            if entry[0] != EXFAT_ALLOCATION_BITMAP_ENTRY || entry[1] & 0x01 != 0 {
                continue;
            }
            let first_cluster = read_u32(entry, 20)?;
            let byte_length =
                usize::try_from(read_u64(entry, 24)?).map_err(|_| ExfatError::UnsupportedLayout)?;
            let minimum_length = usize::try_from(self.geometry.cluster_count)
                .ok()
                .and_then(|count| count.checked_add(7))
                .map(|bits| bits / 8)
                .ok_or(ExfatError::UnsupportedLayout)?;
            if first_cluster < 2 || byte_length < minimum_length {
                return Err(ExfatError::UnsupportedLayout);
            }
            return self.read_fat_chain_bytes(first_cluster, byte_length);
        }
        Err(ExfatError::UnsupportedLayout)
    }

    fn parse_deleted_file_entry_set(
        &self,
        directory_entry_offset: usize,
        entries: [&[u8; DIRECTORY_ENTRY_SIZE]; 3],
        allocation_bitmap: &[u8],
    ) -> Option<DeletedExfatRootFile> {
        let [primary, stream, name] = entries;
        if primary[1] != 2
            || stream[0] != EXFAT_DELETED_STREAM_EXTENSION_ENTRY
            || name[0] != EXFAT_DELETED_FILE_NAME_ENTRY
            || !deleted_entry_set_checksum_matches(entries)
        {
            return None;
        }

        let attributes = read_u16(primary, 4).ok()?;
        if attributes & EXFAT_DIRECTORY_ATTRIBUTE != 0 {
            return None;
        }
        let flags = stream[1];
        if flags & EXFAT_ALLOCATION_POSSIBLE_AND_NO_FAT_CHAIN
            != EXFAT_ALLOCATION_POSSIBLE_AND_NO_FAT_CHAIN
        {
            return None;
        }
        let name_length = usize::from(stream[3]);
        if name_length == 0 || name_length > EXFAT_FILE_NAME_CODE_UNITS_PER_ENTRY {
            return None;
        }
        let valid_data_length = read_u64(stream, 8).ok()?;
        let first_cluster = read_u32(stream, 20).ok()?;
        let byte_length = read_u64(stream, 24).ok()?;
        if valid_data_length == 0 || valid_data_length != byte_length {
            return None;
        }
        let evidence_name = decode_exfat_file_name(name, name_length)?;
        let candidate = DeletedExfatRootFile {
            evidence_name,
            attributes,
            first_cluster,
            byte_length,
            directory_entry_offset: directory_entry_offset as u64,
        };
        self.candidate_extent_is_free(&candidate, allocation_bitmap)
            .ok()
            .and_then(|is_free| is_free.then_some(candidate))
    }

    fn validate_deleted_candidate(
        &self,
        candidate: &DeletedExfatRootFile,
    ) -> Result<(), ExfatError> {
        let entries = self.root_directory_entries()?;
        let allocation_bitmap = self.allocation_bitmap(&entries)?;
        if !self.candidate_extent_is_free(candidate, &allocation_bitmap)? {
            return Err(ExfatError::InvalidClusterChain);
        }
        Ok(())
    }

    fn read_fat_chain_bytes(
        &self,
        first_cluster: u32,
        byte_length: usize,
    ) -> Result<Vec<u8>, ExfatError> {
        if byte_length == 0 {
            return Err(ExfatError::UnsupportedLayout);
        }
        let required_clusters = byte_length
            .checked_add(self.cluster_size - 1)
            .ok_or(ExfatError::InvalidClusterChain)?
            / self.cluster_size;
        let mut output = Vec::with_capacity(byte_length);
        let mut clusters = Vec::with_capacity(required_clusters);
        let mut cluster = first_cluster;

        for index in 0..required_clusters {
            if cluster < 2
                || cluster > self.geometry.cluster_count.saturating_add(1)
                || clusters.contains(&cluster)
            {
                return Err(ExfatError::InvalidClusterChain);
            }
            clusters.push(cluster);
            let offset = self.cluster_offset(cluster)?;
            let bytes_to_copy = (byte_length - output.len()).min(self.cluster_size);
            output.extend_from_slice(&self.image[offset..offset + bytes_to_copy]);
            let next = self.next_cluster(cluster)?;
            if index + 1 < required_clusters {
                if !(2..EXFAT_FAT_EOC_MIN).contains(&next) {
                    return Err(ExfatError::InvalidClusterChain);
                }
                cluster = next;
            } else if next < EXFAT_FAT_EOC_MIN {
                return Err(ExfatError::InvalidClusterChain);
            }
        }
        Ok(output)
    }

    fn candidate_extent_is_free(
        &self,
        candidate: &DeletedExfatRootFile,
        allocation_bitmap: &[u8],
    ) -> Result<bool, ExfatError> {
        if candidate.byte_length == 0 || candidate.first_cluster < 2 {
            return Ok(false);
        }
        let byte_length =
            usize::try_from(candidate.byte_length).map_err(|_| ExfatError::InvalidClusterChain)?;
        let required_clusters = byte_length
            .checked_add(self.cluster_size - 1)
            .ok_or(ExfatError::InvalidClusterChain)?
            / self.cluster_size;
        let first_cluster_index = usize::try_from(candidate.first_cluster - 2)
            .map_err(|_| ExfatError::InvalidClusterChain)?;
        let final_cluster_index = first_cluster_index
            .checked_add(required_clusters)
            .ok_or(ExfatError::InvalidClusterChain)?;
        if final_cluster_index
            > usize::try_from(self.geometry.cluster_count)
                .map_err(|_| ExfatError::InvalidClusterChain)?
        {
            return Ok(false);
        }
        let source_offset = self.cluster_offset(candidate.first_cluster)?;
        ensure_range(self.image, source_offset, byte_length)?;

        for cluster_index in first_cluster_index..final_cluster_index {
            let bitmap_byte = *allocation_bitmap
                .get(cluster_index / 8)
                .ok_or(ExfatError::StructureOutsideImage)?;
            if bitmap_byte & (1 << (cluster_index % 8)) != 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

const DIRECTORY_ENTRY_SIZE: usize = 32;
const EXFAT_DELETED_FILE_ENTRY: u8 = 0x05;
const EXFAT_DELETED_STREAM_EXTENSION_ENTRY: u8 = 0x40;
const EXFAT_DELETED_FILE_NAME_ENTRY: u8 = 0x41;
const EXFAT_ALLOCATION_BITMAP_ENTRY: u8 = 0x81;
const EXFAT_DIRECTORY_ATTRIBUTE: u16 = 0x0010;
const EXFAT_ALLOCATION_POSSIBLE_AND_NO_FAT_CHAIN: u8 = 0x03;
const EXFAT_FILE_NAME_CODE_UNITS_PER_ENTRY: usize = 15;

fn deleted_entry_set_checksum_matches(entries: [&[u8; DIRECTORY_ENTRY_SIZE]; 3]) -> bool {
    let expected = u16::from_le_bytes([entries[0][2], entries[0][3]]);
    let mut checksum = 0_u16;

    for (entry_index, entry) in entries.iter().enumerate() {
        for (byte_index, byte) in entry.iter().copied().enumerate() {
            if entry_index == 0 && matches!(byte_index, 2 | 3) {
                continue;
            }
            let normalized = if byte_index == 0 { byte | 0x80 } else { byte };
            checksum = checksum.rotate_right(1).wrapping_add(u16::from(normalized));
        }
    }
    checksum == expected
}

fn decode_exfat_file_name(
    entry: &[u8; DIRECTORY_ENTRY_SIZE],
    name_length: usize,
) -> Option<String> {
    let mut code_units = Vec::with_capacity(name_length);
    for index in 0..name_length {
        let offset = 2 + index * 2;
        let code_unit = u16::from_le_bytes([*entry.get(offset)?, *entry.get(offset + 1)?]);
        if code_unit == 0 || is_invalid_exfat_file_name_code_unit(code_unit) {
            return None;
        }
        code_units.push(code_unit);
    }
    let name = String::from_utf16(&code_units).ok()?;
    if matches!(name.as_str(), "." | "..") {
        return None;
    }
    Some(name)
}

fn is_invalid_exfat_file_name_code_unit(code_unit: u16) -> bool {
    code_unit <= 0x001f
        || matches!(
            code_unit,
            0x0022 | 0x002a | 0x002f | 0x003a | 0x003c | 0x003e | 0x003f | 0x005c | 0x007c
        )
}

fn validate_main_boot_checksum(image: &[u8], bytes_per_sector: usize) -> Result<(), ExfatError> {
    let mut checksum = 0_u32;
    let checksum_input_length = bytes_per_sector
        .checked_mul(11)
        .ok_or(ExfatError::UnsupportedLayout)?;
    ensure_range(image, 0, checksum_input_length)?;

    for (offset, byte) in image
        .iter()
        .copied()
        .enumerate()
        .take(checksum_input_length)
    {
        if matches!(offset, 106 | 107 | 112) {
            continue;
        }
        checksum = checksum.rotate_right(1).wrapping_add(u32::from(byte));
    }

    let checksum_sector_offset = checksum_input_length;
    ensure_range(image, checksum_sector_offset, bytes_per_sector)?;
    for offset in (checksum_sector_offset..checksum_sector_offset + bytes_per_sector).step_by(4) {
        if read_u32(image, offset)? != checksum {
            return Err(ExfatError::InvalidBootChecksum);
        }
    }
    Ok(())
}

fn cluster_offset(
    cluster_heap_offset: usize,
    cluster_size: usize,
    cluster: u32,
    cluster_count: u32,
) -> Result<usize, ExfatError> {
    if cluster < 2 || cluster > cluster_count.saturating_add(1) {
        return Err(ExfatError::InvalidClusterChain);
    }
    let cluster_index =
        usize::try_from(cluster - 2).map_err(|_| ExfatError::InvalidClusterChain)?;
    cluster_heap_offset
        .checked_add(
            cluster_index
                .checked_mul(cluster_size)
                .ok_or(ExfatError::InvalidClusterChain)?,
        )
        .ok_or(ExfatError::InvalidClusterChain)
}

fn read_u16(image: &[u8], offset: usize) -> Result<u16, ExfatError> {
    let bytes = image
        .get(offset..offset + 2)
        .ok_or(ExfatError::StructureOutsideImage)?;
    Ok(u16::from_le_bytes(
        bytes.try_into().expect("fixed u16 range"),
    ))
}

fn read_u32(image: &[u8], offset: usize) -> Result<u32, ExfatError> {
    let bytes = image
        .get(offset..offset + 4)
        .ok_or(ExfatError::StructureOutsideImage)?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("fixed u32 range"),
    ))
}

fn read_u64(image: &[u8], offset: usize) -> Result<u64, ExfatError> {
    let bytes = image
        .get(offset..offset + 8)
        .ok_or(ExfatError::StructureOutsideImage)?;
    Ok(u64::from_le_bytes(
        bytes.try_into().expect("fixed u64 range"),
    ))
}

fn ensure_range(image: &[u8], offset: usize, length: usize) -> Result<(), ExfatError> {
    if offset
        .checked_add(length)
        .map(|end| end <= image.len())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(ExfatError::StructureOutsideImage)
    }
}

#[cfg(test)]
mod tests {
    use super::{DeletedExfatRootFile, ExfatError, ExfatVolume, DIRECTORY_ENTRY_SIZE};

    const BYTES_PER_SECTOR: usize = 512;
    const VOLUME_SECTORS: usize = 2048;
    const FAT_OFFSET_SECTORS: usize = 24;
    const FAT_LENGTH_SECTORS: usize = 16;
    const CLUSTER_HEAP_OFFSET_SECTORS: usize = 40;
    const CLUSTER_COUNT: usize = VOLUME_SECTORS - CLUSTER_HEAP_OFFSET_SECTORS;
    const ROOT_CLUSTER: usize = 2;
    const BITMAP_CLUSTER: usize = 3;
    const CONTENT_CLUSTER: usize = 4;
    const CONTENT: &[u8] = b"exfat recovered\n";

    fn write_u16(image: &mut [u8], offset: usize, value: u16) {
        image[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(image: &mut [u8], offset: usize, value: u32) {
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(image: &mut [u8], offset: usize, value: u64) {
        image[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn set_entry_set_checksum(entries: &mut [[u8; DIRECTORY_ENTRY_SIZE]; 3]) {
        let mut checksum = 0_u16;
        for (entry_index, entry) in entries.iter().enumerate() {
            for (byte_index, byte) in entry.iter().copied().enumerate() {
                if entry_index == 0 && matches!(byte_index, 2 | 3) {
                    continue;
                }
                checksum = checksum.rotate_right(1).wrapping_add(u16::from(byte));
            }
        }
        entries[0][2..4].copy_from_slice(&checksum.to_le_bytes());
    }

    fn write_boot_checksum(image: &mut [u8]) {
        let mut checksum = 0_u32;
        for (offset, byte) in image
            .iter()
            .copied()
            .take(BYTES_PER_SECTOR * 11)
            .enumerate()
        {
            if matches!(offset, 106 | 107 | 112) {
                continue;
            }
            checksum = checksum.rotate_right(1).wrapping_add(u32::from(byte));
        }
        for offset in (BYTES_PER_SECTOR * 11..BYTES_PER_SECTOR * 12).step_by(4) {
            write_u32(image, offset, checksum);
        }
    }

    fn sample_exfat_image() -> Vec<u8> {
        let mut image = vec![0_u8; VOLUME_SECTORS * BYTES_PER_SECTOR];
        image[0] = 0xeb;
        image[1] = 0x76;
        image[2] = 0x90;
        image[3..11].copy_from_slice(b"EXFAT   ");
        write_u64(&mut image, 72, VOLUME_SECTORS as u64);
        write_u32(&mut image, 80, FAT_OFFSET_SECTORS as u32);
        write_u32(&mut image, 84, FAT_LENGTH_SECTORS as u32);
        write_u32(&mut image, 88, CLUSTER_HEAP_OFFSET_SECTORS as u32);
        write_u32(&mut image, 92, CLUSTER_COUNT as u32);
        write_u32(&mut image, 96, ROOT_CLUSTER as u32);
        write_u16(&mut image, 104, 0x0100);
        image[108] = 9;
        image[109] = 0;
        image[110] = 1;
        image[112] = 0xff;
        write_u16(&mut image, 510, 0xaa55);
        for sector in 1..=8 {
            write_u32(
                &mut image,
                sector * BYTES_PER_SECTOR + BYTES_PER_SECTOR - 4,
                0xaa55_0000,
            );
        }

        let fat_offset = FAT_OFFSET_SECTORS * BYTES_PER_SECTOR;
        write_u32(&mut image, fat_offset + ROOT_CLUSTER * 4, 0xffff_ffff);
        write_u32(&mut image, fat_offset + BITMAP_CLUSTER * 4, 0xffff_ffff);
        let cluster_heap = CLUSTER_HEAP_OFFSET_SECTORS * BYTES_PER_SECTOR;
        let root_offset = cluster_heap;
        let bitmap_offset = cluster_heap + BYTES_PER_SECTOR;
        let content_offset = cluster_heap + 2 * BYTES_PER_SECTOR;

        image[root_offset] = 0x81;
        image[root_offset + 20..root_offset + 24]
            .copy_from_slice(&(BITMAP_CLUSTER as u32).to_le_bytes());
        image[root_offset + 24..root_offset + 32]
            .copy_from_slice(&((CLUSTER_COUNT + 7).div_ceil(8) as u64).to_le_bytes());

        let mut entries = [[0_u8; DIRECTORY_ENTRY_SIZE]; 3];
        entries[0][0] = 0x85;
        entries[0][1] = 2;
        entries[0][4..6].copy_from_slice(&0x0020_u16.to_le_bytes());
        entries[1][0] = 0xc0;
        entries[1][1] = 0x03;
        entries[1][3] = 9;
        entries[1][8..16].copy_from_slice(&(CONTENT.len() as u64).to_le_bytes());
        entries[1][20..24].copy_from_slice(&(CONTENT_CLUSTER as u32).to_le_bytes());
        entries[1][24..32].copy_from_slice(&(CONTENT.len() as u64).to_le_bytes());
        entries[2][0] = 0xc1;
        entries[2][2..20].copy_from_slice(&[
            b'r', 0, b'e', 0, b'c', 0, b'o', 0, b'v', 0, b'e', 0, b'r', 0, b'.', 0, b't', 0,
        ]);
        set_entry_set_checksum(&mut entries);
        entries[0][0] &= 0x7f;
        entries[1][0] &= 0x7f;
        entries[2][0] &= 0x7f;
        for (index, entry) in entries.iter().enumerate() {
            let offset = root_offset + (index + 1) * DIRECTORY_ENTRY_SIZE;
            image[offset..offset + DIRECTORY_ENTRY_SIZE].copy_from_slice(entry);
        }

        image[bitmap_offset] = 0b0000_0011;
        image[content_offset..content_offset + CONTENT.len()].copy_from_slice(CONTENT);
        write_boot_checksum(&mut image);
        image
    }

    #[test]
    fn parses_checked_exfat_geometry_and_recovers_a_contiguous_deleted_root_file() {
        let image = sample_exfat_image();
        let volume = ExfatVolume::parse(&image).expect("parse exFAT image");
        let candidate = volume
            .find_deleted_root_files()
            .into_iter()
            .next()
            .expect("find deleted exFAT candidate");

        assert_eq!(volume.geometry().bytes_per_sector, 512);
        assert_eq!(volume.geometry().root_directory_offset, 20480);
        assert_eq!(candidate.evidence_name, "recover.t");
        assert_eq!(candidate.first_cluster, CONTENT_CLUSTER as u32);
        assert_eq!(
            volume
                .source_offset_for_candidate(&candidate)
                .expect("locate candidate"),
            21504
        );
        assert_eq!(
            volume
                .read_deleted_file(&candidate)
                .expect("recover candidate"),
            CONTENT
        );
    }

    #[test]
    fn rejects_an_invalid_main_boot_checksum() {
        let mut image = sample_exfat_image();
        image[72] ^= 0x01;

        assert_eq!(
            ExfatVolume::parse(&image).expect_err("reject invalid checksum"),
            ExfatError::InvalidBootChecksum
        );
    }

    #[test]
    fn ignores_a_deleted_entry_set_when_a_content_cluster_is_allocated() {
        let mut image = sample_exfat_image();
        let bitmap_offset = CLUSTER_HEAP_OFFSET_SECTORS * BYTES_PER_SECTOR + BYTES_PER_SECTOR;
        image[bitmap_offset] |= 1 << (CONTENT_CLUSTER - 2);

        let volume = ExfatVolume::parse(&image).expect("parse exFAT image");
        assert!(volume.find_deleted_root_files().is_empty());
    }

    #[test]
    fn rejects_a_candidate_with_an_invalid_or_reused_extent() {
        let image = sample_exfat_image();
        let volume = ExfatVolume::parse(&image).expect("parse exFAT image");
        let candidate = DeletedExfatRootFile {
            evidence_name: "recover.t".to_owned(),
            attributes: 0x0020,
            first_cluster: 2,
            byte_length: CONTENT.len() as u64,
            directory_entry_offset: 20512,
        };

        assert!(volume.read_deleted_file(&candidate).is_err());
    }
}
