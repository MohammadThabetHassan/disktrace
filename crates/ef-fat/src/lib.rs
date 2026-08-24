pub mod exfat;
pub mod ntfs;
pub use exfat::{DeletedExfatRootFile, ExfatError, ExfatGeometry, ExfatVolume};
pub use ntfs::{
    DeletedNtfsContiguousFile, DeletedNtfsResidentFile, NtfsError, NtfsGeometry, NtfsVolume,
};

use serde::{Deserialize, Serialize};
use std::cmp::min;
use thiserror::Error;

const DIRECTORY_ENTRY_SIZE: usize = 32;
const FAT12_EOC_MIN: u16 = 0x0ff8;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Fat12Error {
    #[error("image is too small to contain a FAT12 boot sector")]
    ImageTooSmall,
    #[error("unsupported bytes per sector: {0}")]
    UnsupportedBytesPerSector(u16),
    #[error("unsupported FAT12 volume layout")]
    UnsupportedLayout,
    #[error("FAT12 structure extends beyond the supplied image")]
    StructureOutsideImage,
    #[error("invalid FAT12 cluster chain")]
    InvalidClusterChain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fat12Geometry {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub fat_count: u8,
    pub root_entry_count: u16,
    pub sectors_per_fat: u16,
    pub total_sectors: u32,
    pub root_directory_offset: u64,
    pub data_offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedRootFile {
    pub evidence_name: String,
    pub attributes: u8,
    pub first_cluster: u16,
    pub byte_length: u32,
    pub directory_entry_offset: u64,
}

#[derive(Debug, Clone)]
pub struct Fat12Volume<'a> {
    image: &'a [u8],
    geometry: Fat12Geometry,
    fat_offset: usize,
    fat_length: usize,
    cluster_size: usize,
    data_offset: usize,
}

impl<'a> Fat12Volume<'a> {
    pub fn parse(image: &'a [u8]) -> Result<Self, Fat12Error> {
        if image.len() < 512 {
            return Err(Fat12Error::ImageTooSmall);
        }

        let bytes_per_sector = read_u16(image, 11)?;
        if bytes_per_sector != 512 {
            return Err(Fat12Error::UnsupportedBytesPerSector(bytes_per_sector));
        }

        let sectors_per_cluster = image[13];
        let reserved_sector_count = read_u16(image, 14)?;
        let fat_count = image[16];
        let root_entry_count = read_u16(image, 17)?;
        let total_sectors_16 = read_u16(image, 19)?;
        let sectors_per_fat = read_u16(image, 22)?;
        let total_sectors_32 = read_u32(image, 32)?;
        let total_sectors = if total_sectors_16 == 0 {
            total_sectors_32
        } else {
            u32::from(total_sectors_16)
        };

        if sectors_per_cluster == 0
            || reserved_sector_count == 0
            || fat_count == 0
            || root_entry_count == 0
            || sectors_per_fat == 0
            || total_sectors == 0
        {
            return Err(Fat12Error::UnsupportedLayout);
        }

        let bytes_per_sector_usize = usize::from(bytes_per_sector);
        let root_directory_bytes = usize::from(root_entry_count)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(Fat12Error::UnsupportedLayout)?;
        let root_directory_sectors = root_directory_bytes
            .checked_add(bytes_per_sector_usize - 1)
            .ok_or(Fat12Error::UnsupportedLayout)?
            / bytes_per_sector_usize;
        let fat_offset = usize::from(reserved_sector_count)
            .checked_mul(bytes_per_sector_usize)
            .ok_or(Fat12Error::UnsupportedLayout)?;
        let fat_length = usize::from(sectors_per_fat)
            .checked_mul(bytes_per_sector_usize)
            .ok_or(Fat12Error::UnsupportedLayout)?;
        let root_directory_offset = fat_offset
            .checked_add(
                usize::from(fat_count)
                    .checked_mul(fat_length)
                    .ok_or(Fat12Error::UnsupportedLayout)?,
            )
            .ok_or(Fat12Error::UnsupportedLayout)?;
        let data_offset = root_directory_offset
            .checked_add(
                root_directory_sectors
                    .checked_mul(bytes_per_sector_usize)
                    .ok_or(Fat12Error::UnsupportedLayout)?,
            )
            .ok_or(Fat12Error::UnsupportedLayout)?;
        let cluster_size = usize::from(sectors_per_cluster)
            .checked_mul(bytes_per_sector_usize)
            .ok_or(Fat12Error::UnsupportedLayout)?;
        let volume_bytes = usize::try_from(total_sectors)
            .ok()
            .and_then(|sectors| sectors.checked_mul(bytes_per_sector_usize))
            .ok_or(Fat12Error::UnsupportedLayout)?;
        let data_sectors = usize::try_from(total_sectors)
            .ok()
            .and_then(|sectors| {
                sectors.checked_sub(
                    usize::from(reserved_sector_count)
                        + usize::from(fat_count) * usize::from(sectors_per_fat)
                        + root_directory_sectors,
                )
            })
            .ok_or(Fat12Error::UnsupportedLayout)?;
        let cluster_count = data_sectors / usize::from(sectors_per_cluster);
        if cluster_count == 0 || cluster_count >= 4085 {
            return Err(Fat12Error::UnsupportedLayout);
        }

        ensure_range(image, 0, volume_bytes)?;
        ensure_range(image, fat_offset, fat_length)?;
        ensure_range(image, root_directory_offset, root_directory_bytes)?;
        ensure_range(image, data_offset, cluster_size)?;

        Ok(Self {
            image,
            geometry: Fat12Geometry {
                bytes_per_sector,
                sectors_per_cluster,
                reserved_sector_count,
                fat_count,
                root_entry_count,
                sectors_per_fat,
                total_sectors,
                root_directory_offset: root_directory_offset as u64,
                data_offset: data_offset as u64,
            },
            fat_offset,
            fat_length,
            cluster_size,
            data_offset,
        })
    }

    pub fn geometry(&self) -> &Fat12Geometry {
        &self.geometry
    }

    pub fn find_deleted_root_files(&self) -> Vec<DeletedRootFile> {
        let root_directory_offset = self.geometry.root_directory_offset as usize;
        let mut files = Vec::new();

        for index in 0..usize::from(self.geometry.root_entry_count) {
            let entry_offset = root_directory_offset + index * DIRECTORY_ENTRY_SIZE;
            let entry = &self.image[entry_offset..entry_offset + DIRECTORY_ENTRY_SIZE];
            let first_byte = entry[0];
            if first_byte == 0x00 {
                break;
            }
            if first_byte != 0xe5
                || is_long_filename_entry(entry)
                || is_directory(entry)
                || is_volume_label(entry)
            {
                continue;
            }

            files.push(DeletedRootFile {
                evidence_name: render_deleted_short_name(entry),
                attributes: entry[11],
                first_cluster: u16::from_le_bytes([entry[26], entry[27]]),
                byte_length: u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]),
                directory_entry_offset: entry_offset as u64,
            });
        }

        files
    }

    pub fn source_offset_for_candidate(
        &self,
        candidate: &DeletedRootFile,
    ) -> Result<u64, Fat12Error> {
        Ok(self.cluster_offset(candidate.first_cluster)? as u64)
    }

    pub fn read_deleted_file(&self, candidate: &DeletedRootFile) -> Result<Vec<u8>, Fat12Error> {
        if candidate.byte_length == 0 {
            return Ok(Vec::new());
        }
        if candidate.first_cluster < 2 {
            return Err(Fat12Error::InvalidClusterChain);
        }

        let expected_length =
            usize::try_from(candidate.byte_length).map_err(|_| Fat12Error::InvalidClusterChain)?;
        let required_clusters = expected_length
            .checked_add(self.cluster_size - 1)
            .ok_or(Fat12Error::InvalidClusterChain)?
            / self.cluster_size;
        let mut cluster = candidate.first_cluster;
        let mut output = Vec::with_capacity(expected_length);
        let mut visited = Vec::new();

        for index in 0..required_clusters {
            if cluster < 2 || visited.contains(&cluster) {
                return Err(Fat12Error::InvalidClusterChain);
            }
            visited.push(cluster);
            let cluster_offset = self.cluster_offset(cluster)?;
            let bytes_remaining = expected_length - output.len();
            let bytes_to_copy = min(bytes_remaining, self.cluster_size);
            output.extend_from_slice(&self.image[cluster_offset..cluster_offset + bytes_to_copy]);

            if index + 1 < required_clusters {
                let next_cluster = self.next_cluster(cluster)?;
                if next_cluster >= FAT12_EOC_MIN {
                    return Err(Fat12Error::InvalidClusterChain);
                }
                cluster = next_cluster;
            }
        }

        Ok(output)
    }

    fn cluster_offset(&self, cluster: u16) -> Result<usize, Fat12Error> {
        let cluster_index = usize::from(
            cluster
                .checked_sub(2)
                .ok_or(Fat12Error::InvalidClusterChain)?,
        );
        let offset = self
            .data_offset
            .checked_add(
                cluster_index
                    .checked_mul(self.cluster_size)
                    .ok_or(Fat12Error::InvalidClusterChain)?,
            )
            .ok_or(Fat12Error::InvalidClusterChain)?;
        ensure_range(self.image, offset, self.cluster_size)?;
        Ok(offset)
    }

    fn next_cluster(&self, cluster: u16) -> Result<u16, Fat12Error> {
        let cluster_index = usize::from(cluster);
        let entry_offset = cluster_index
            .checked_add(cluster_index / 2)
            .ok_or(Fat12Error::InvalidClusterChain)?;
        if entry_offset + 1 >= self.fat_length {
            return Err(Fat12Error::InvalidClusterChain);
        }
        let low = u16::from(self.image[self.fat_offset + entry_offset]);
        let high = u16::from(self.image[self.fat_offset + entry_offset + 1]);
        let packed = low | (high << 8);
        Ok(if cluster & 1 == 0 {
            packed & 0x0fff
        } else {
            packed >> 4
        })
    }
}

const FAT16_EOC_MIN: u16 = 0xfff8;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Fat16Error {
    #[error("image is too small to contain a FAT16 boot sector")]
    ImageTooSmall,
    #[error("unsupported bytes per sector: {0}")]
    UnsupportedBytesPerSector(u16),
    #[error("unsupported FAT16 volume layout")]
    UnsupportedLayout,
    #[error("FAT16 structure extends beyond the supplied image")]
    StructureOutsideImage,
    #[error("invalid FAT16 cluster chain")]
    InvalidClusterChain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fat16Geometry {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub fat_count: u8,
    pub root_entry_count: u16,
    pub sectors_per_fat: u16,
    pub total_sectors: u32,
    pub root_directory_offset: u64,
    pub data_offset: u64,
}

#[derive(Debug, Clone)]
pub struct Fat16Volume<'a> {
    image: &'a [u8],
    geometry: Fat16Geometry,
    fat_offset: usize,
    fat_length: usize,
    cluster_size: usize,
    data_offset: usize,
}

impl<'a> Fat16Volume<'a> {
    pub fn parse(image: &'a [u8]) -> Result<Self, Fat16Error> {
        if image.len() < 512 {
            return Err(Fat16Error::ImageTooSmall);
        }

        let bytes_per_sector = read_u16_fat16(image, 11)?;
        if bytes_per_sector != 512 {
            return Err(Fat16Error::UnsupportedBytesPerSector(bytes_per_sector));
        }

        let sectors_per_cluster = image[13];
        let reserved_sector_count = read_u16_fat16(image, 14)?;
        let fat_count = image[16];
        let root_entry_count = read_u16_fat16(image, 17)?;
        let total_sectors_16 = read_u16_fat16(image, 19)?;
        let sectors_per_fat = read_u16_fat16(image, 22)?;
        let total_sectors_32 = read_u32_fat16(image, 32)?;
        let total_sectors = if total_sectors_16 == 0 {
            total_sectors_32
        } else {
            u32::from(total_sectors_16)
        };

        if sectors_per_cluster != 1
            || reserved_sector_count == 0
            || fat_count != 1
            || root_entry_count == 0
            || sectors_per_fat == 0
            || total_sectors == 0
        {
            return Err(Fat16Error::UnsupportedLayout);
        }

        let bytes_per_sector_usize = usize::from(bytes_per_sector);
        let root_directory_bytes = usize::from(root_entry_count)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(Fat16Error::UnsupportedLayout)?;
        let root_directory_sectors = root_directory_bytes
            .checked_add(bytes_per_sector_usize - 1)
            .ok_or(Fat16Error::UnsupportedLayout)?
            / bytes_per_sector_usize;
        let fat_offset = usize::from(reserved_sector_count)
            .checked_mul(bytes_per_sector_usize)
            .ok_or(Fat16Error::UnsupportedLayout)?;
        let fat_length = usize::from(sectors_per_fat)
            .checked_mul(bytes_per_sector_usize)
            .ok_or(Fat16Error::UnsupportedLayout)?;
        let root_directory_offset = fat_offset
            .checked_add(fat_length)
            .ok_or(Fat16Error::UnsupportedLayout)?;
        let data_offset = root_directory_offset
            .checked_add(
                root_directory_sectors
                    .checked_mul(bytes_per_sector_usize)
                    .ok_or(Fat16Error::UnsupportedLayout)?,
            )
            .ok_or(Fat16Error::UnsupportedLayout)?;
        let cluster_size = bytes_per_sector_usize;
        let volume_bytes = usize::try_from(total_sectors)
            .ok()
            .and_then(|sectors| sectors.checked_mul(bytes_per_sector_usize))
            .ok_or(Fat16Error::UnsupportedLayout)?;
        let data_sectors = usize::try_from(total_sectors)
            .ok()
            .and_then(|sectors| {
                sectors.checked_sub(
                    usize::from(reserved_sector_count)
                        + fat_length / bytes_per_sector_usize
                        + root_directory_sectors,
                )
            })
            .ok_or(Fat16Error::UnsupportedLayout)?;
        let cluster_count = data_sectors / usize::from(sectors_per_cluster);
        if !(4085..65525).contains(&cluster_count) {
            return Err(Fat16Error::UnsupportedLayout);
        }

        ensure_range_fat16(image, 0, volume_bytes)?;
        ensure_range_fat16(image, fat_offset, fat_length)?;
        ensure_range_fat16(image, root_directory_offset, root_directory_bytes)?;
        ensure_range_fat16(image, data_offset, cluster_size)?;

        Ok(Self {
            image,
            geometry: Fat16Geometry {
                bytes_per_sector,
                sectors_per_cluster,
                reserved_sector_count,
                fat_count,
                root_entry_count,
                sectors_per_fat,
                total_sectors,
                root_directory_offset: root_directory_offset as u64,
                data_offset: data_offset as u64,
            },
            fat_offset,
            fat_length,
            cluster_size,
            data_offset,
        })
    }

    pub fn geometry(&self) -> &Fat16Geometry {
        &self.geometry
    }

    pub fn find_deleted_root_files(&self) -> Vec<DeletedRootFile> {
        let root_directory_offset = self.geometry.root_directory_offset as usize;
        let mut files = Vec::new();

        for index in 0..usize::from(self.geometry.root_entry_count) {
            let entry_offset = root_directory_offset + index * DIRECTORY_ENTRY_SIZE;
            let entry = &self.image[entry_offset..entry_offset + DIRECTORY_ENTRY_SIZE];
            let first_byte = entry[0];
            if first_byte == 0x00 {
                break;
            }
            if first_byte != 0xe5
                || is_long_filename_entry(entry)
                || is_directory(entry)
                || is_volume_label(entry)
            {
                continue;
            }
            files.push(DeletedRootFile {
                evidence_name: render_deleted_short_name(entry),
                attributes: entry[11],
                first_cluster: u16::from_le_bytes([entry[26], entry[27]]),
                byte_length: u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]),
                directory_entry_offset: entry_offset as u64,
            });
        }

        files
    }

    pub fn source_offset_for_candidate(
        &self,
        candidate: &DeletedRootFile,
    ) -> Result<u64, Fat16Error> {
        Ok(self.cluster_offset(candidate.first_cluster)? as u64)
    }

    pub fn read_deleted_file(&self, candidate: &DeletedRootFile) -> Result<Vec<u8>, Fat16Error> {
        if candidate.byte_length == 0 {
            return Ok(Vec::new());
        }
        if candidate.first_cluster < 2 {
            return Err(Fat16Error::InvalidClusterChain);
        }

        let expected_length =
            usize::try_from(candidate.byte_length).map_err(|_| Fat16Error::InvalidClusterChain)?;
        let required_clusters = expected_length
            .checked_add(self.cluster_size - 1)
            .ok_or(Fat16Error::InvalidClusterChain)?
            / self.cluster_size;
        let mut cluster = candidate.first_cluster;
        let mut output = Vec::with_capacity(expected_length);
        let mut visited = Vec::new();

        for index in 0..required_clusters {
            if cluster < 2 || visited.contains(&cluster) {
                return Err(Fat16Error::InvalidClusterChain);
            }
            visited.push(cluster);
            let cluster_offset = self.cluster_offset(cluster)?;
            let bytes_remaining = expected_length - output.len();
            let bytes_to_copy = min(bytes_remaining, self.cluster_size);
            output.extend_from_slice(&self.image[cluster_offset..cluster_offset + bytes_to_copy]);

            if index + 1 < required_clusters {
                let next_cluster = self.next_cluster(cluster)?;
                if !(2..FAT16_EOC_MIN).contains(&next_cluster) {
                    return Err(Fat16Error::InvalidClusterChain);
                }
                cluster = next_cluster;
            }
        }

        Ok(output)
    }

    fn cluster_offset(&self, cluster: u16) -> Result<usize, Fat16Error> {
        let cluster_index = usize::from(
            cluster
                .checked_sub(2)
                .ok_or(Fat16Error::InvalidClusterChain)?,
        );
        let offset = self
            .data_offset
            .checked_add(
                cluster_index
                    .checked_mul(self.cluster_size)
                    .ok_or(Fat16Error::InvalidClusterChain)?,
            )
            .ok_or(Fat16Error::InvalidClusterChain)?;
        ensure_range_fat16(self.image, offset, self.cluster_size)?;
        Ok(offset)
    }

    fn next_cluster(&self, cluster: u16) -> Result<u16, Fat16Error> {
        let entry_offset = usize::from(cluster)
            .checked_mul(2)
            .ok_or(Fat16Error::InvalidClusterChain)?;
        if entry_offset + 1 >= self.fat_length {
            return Err(Fat16Error::InvalidClusterChain);
        }
        Ok(u16::from_le_bytes([
            self.image[self.fat_offset + entry_offset],
            self.image[self.fat_offset + entry_offset + 1],
        ]))
    }
}

fn read_u16_fat16(image: &[u8], offset: usize) -> Result<u16, Fat16Error> {
    ensure_range_fat16(image, offset, 2)?;
    Ok(u16::from_le_bytes([image[offset], image[offset + 1]]))
}

fn read_u32_fat16(image: &[u8], offset: usize) -> Result<u32, Fat16Error> {
    ensure_range_fat16(image, offset, 4)?;
    Ok(u32::from_le_bytes([
        image[offset],
        image[offset + 1],
        image[offset + 2],
        image[offset + 3],
    ]))
}

fn ensure_range_fat16(image: &[u8], offset: usize, length: usize) -> Result<(), Fat16Error> {
    if offset
        .checked_add(length)
        .map(|end| end <= image.len())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(Fat16Error::StructureOutsideImage)
    }
}

fn read_u16(image: &[u8], offset: usize) -> Result<u16, Fat12Error> {
    ensure_range(image, offset, 2)?;
    Ok(u16::from_le_bytes([image[offset], image[offset + 1]]))
}

fn read_u32(image: &[u8], offset: usize) -> Result<u32, Fat12Error> {
    ensure_range(image, offset, 4)?;
    Ok(u32::from_le_bytes([
        image[offset],
        image[offset + 1],
        image[offset + 2],
        image[offset + 3],
    ]))
}

fn ensure_range(image: &[u8], offset: usize, length: usize) -> Result<(), Fat12Error> {
    if offset
        .checked_add(length)
        .map(|end| end <= image.len())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(Fat12Error::StructureOutsideImage)
    }
}

fn is_long_filename_entry(entry: &[u8]) -> bool {
    entry[11] & 0x0f == 0x0f
}

fn is_directory(entry: &[u8]) -> bool {
    entry[11] & 0x10 != 0
}

fn is_volume_label(entry: &[u8]) -> bool {
    entry[11] & 0x08 != 0
}

fn render_deleted_short_name(entry: &[u8]) -> String {
    let base = format!("?{}", render_short_component(&entry[1..8]));
    let extension = render_short_component(&entry[8..11]);
    if extension.is_empty() {
        base
    } else {
        format!("{base}.{extension}")
    }
}

fn render_short_component(bytes: &[u8]) -> String {
    bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != b' ')
        .map(|byte| {
            if byte.is_ascii_graphic() {
                char::from(byte)
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        DeletedRootFile, Fat12Error, Fat12Volume, Fat16Error, Fat16Volume, DIRECTORY_ENTRY_SIZE,
    };

    const DELETED_CONTENT: &[u8] = b"recover me\n";

    fn sample_image() -> Vec<u8> {
        let mut image = vec![0_u8; 512 * 10];
        image[0] = 0xeb;
        image[1] = 0x3c;
        image[2] = 0x90;
        image[3..11].copy_from_slice(b"EFORGE  ");
        image[11..13].copy_from_slice(&512_u16.to_le_bytes());
        image[13] = 1;
        image[14..16].copy_from_slice(&1_u16.to_le_bytes());
        image[16] = 1;
        image[17..19].copy_from_slice(&16_u16.to_le_bytes());
        image[19..21].copy_from_slice(&10_u16.to_le_bytes());
        image[21] = 0xf8;
        image[22..24].copy_from_slice(&1_u16.to_le_bytes());
        image[510] = 0x55;
        image[511] = 0xaa;

        let fat = 512;
        image[fat] = 0xf8;
        image[fat + 1] = 0xff;
        image[fat + 2] = 0xff;
        image[fat + 3] = 0xff;
        image[fat + 4] = 0x0f;

        let root = 1024;
        image[root..root + 8].copy_from_slice(b"ACTIVE  ");
        image[root + 8..root + 11].copy_from_slice(b"TXT");
        image[root + 11] = 0x20;
        image[root + 26..root + 28].copy_from_slice(&3_u16.to_le_bytes());
        image[root + 28..root + 32].copy_from_slice(&6_u32.to_le_bytes());

        let deleted = root + DIRECTORY_ENTRY_SIZE;
        image[deleted] = 0xe5;
        image[deleted + 1..deleted + 8].copy_from_slice(b"ELETED ");
        image[deleted + 8..deleted + 11].copy_from_slice(b"TXT");
        image[deleted + 11] = 0x20;
        image[deleted + 26..deleted + 28].copy_from_slice(&2_u16.to_le_bytes());
        image[deleted + 28..deleted + 32]
            .copy_from_slice(&(DELETED_CONTENT.len() as u32).to_le_bytes());

        let data = 1536;
        image[data..data + DELETED_CONTENT.len()].copy_from_slice(DELETED_CONTENT);
        image[data + 512..data + 518].copy_from_slice(b"active");
        image
    }

    #[test]
    fn discovers_a_deleted_short_name_root_entry() {
        let image = sample_image();
        let volume = Fat12Volume::parse(&image).expect("parse FAT12 image");
        let candidates = volume.find_deleted_root_files();

        assert_eq!(volume.geometry().root_directory_offset, 1024);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].evidence_name, "?ELETED.TXT");
        assert_eq!(candidates[0].first_cluster, 2);
        assert_eq!(candidates[0].byte_length, DELETED_CONTENT.len() as u32);
        assert_eq!(candidates[0].directory_entry_offset, 1056);
    }

    #[test]
    fn extracts_the_retained_fat12_cluster_chain() {
        let image = sample_image();
        let volume = Fat12Volume::parse(&image).expect("parse FAT12 image");
        let candidate = volume
            .find_deleted_root_files()
            .into_iter()
            .next()
            .expect("find deleted candidate");

        let source_offset = volume
            .source_offset_for_candidate(&candidate)
            .expect("locate candidate source bytes");
        let recovered = volume
            .read_deleted_file(&candidate)
            .expect("read deleted file");

        assert_eq!(source_offset, 1536);
        assert_eq!(recovered, DELETED_CONTENT);
    }

    #[test]
    fn rejects_invalid_boot_sector() {
        let mut image = sample_image();
        image[11..13].copy_from_slice(&1024_u16.to_le_bytes());

        let error = Fat12Volume::parse(&image).expect_err("reject unsupported sector size");

        assert_eq!(error, Fat12Error::UnsupportedBytesPerSector(1024));
    }

    #[test]
    fn rejects_a_cluster_chain_that_ends_before_file_length() {
        let mut image = sample_image();
        let deleted_entry_offset = 1024 + DIRECTORY_ENTRY_SIZE;
        image[deleted_entry_offset + 28..deleted_entry_offset + 32]
            .copy_from_slice(&1024_u32.to_le_bytes());
        let volume = Fat12Volume::parse(&image).expect("parse FAT12 image");
        let candidate = volume
            .find_deleted_root_files()
            .into_iter()
            .next()
            .expect("find deleted candidate");

        let error = volume
            .read_deleted_file(&candidate)
            .expect_err("reject incomplete chain");

        assert_eq!(error, Fat12Error::InvalidClusterChain);
    }

    #[test]
    fn preserves_deleted_candidate_fields_for_serialization() {
        let candidate = DeletedRootFile {
            evidence_name: "?ELETED.TXT".to_owned(),
            attributes: 0x20,
            first_cluster: 2,
            byte_length: 11,
            directory_entry_offset: 1056,
        };

        assert_eq!(candidate.evidence_name, "?ELETED.TXT");
    }

    fn sample_fat16_image() -> Vec<u8> {
        const TOTAL_SECTORS: usize = 4120;
        const FAT_SECTORS: usize = 17;
        const ROOT_ENTRIES: usize = 32;
        const ROOT_SECTORS: usize = 2;
        let mut image = vec![0_u8; TOTAL_SECTORS * 512];
        image[0] = 0xeb;
        image[1] = 0x3c;
        image[2] = 0x90;
        image[3..11].copy_from_slice(b"EFORGE16");
        image[11..13].copy_from_slice(&512_u16.to_le_bytes());
        image[13] = 1;
        image[14..16].copy_from_slice(&1_u16.to_le_bytes());
        image[16] = 1;
        image[17..19].copy_from_slice(&(ROOT_ENTRIES as u16).to_le_bytes());
        image[19..21].copy_from_slice(&(TOTAL_SECTORS as u16).to_le_bytes());
        image[21] = 0xf8;
        image[22..24].copy_from_slice(&(FAT_SECTORS as u16).to_le_bytes());
        image[510] = 0x55;
        image[511] = 0xaa;

        let fat = 512;
        image[fat..fat + 2].copy_from_slice(&0xfff8_u16.to_le_bytes());
        image[fat + 2..fat + 4].copy_from_slice(&0xffff_u16.to_le_bytes());
        image[fat + 4..fat + 6].copy_from_slice(&0xffff_u16.to_le_bytes());

        let root = (1 + FAT_SECTORS) * 512;
        let deleted = root;
        image[deleted] = 0xe5;
        image[deleted + 1..deleted + 8].copy_from_slice(b"ECOVER ");
        image[deleted + 8..deleted + 11].copy_from_slice(b"TXT");
        image[deleted + 11] = 0x20;
        image[deleted + 26..deleted + 28].copy_from_slice(&2_u16.to_le_bytes());
        image[deleted + 28..deleted + 32]
            .copy_from_slice(&(DELETED_CONTENT.len() as u32).to_le_bytes());

        let data = root + ROOT_SECTORS * 512;
        image[data..data + DELETED_CONTENT.len()].copy_from_slice(DELETED_CONTENT);
        image
    }

    #[test]
    fn discovers_and_extracts_a_deleted_fat16_root_file() {
        let image = sample_fat16_image();
        let volume = Fat16Volume::parse(&image).expect("parse FAT16 image");
        let candidate = volume
            .find_deleted_root_files()
            .into_iter()
            .next()
            .expect("find deleted candidate");

        assert_eq!(volume.geometry().data_offset, 10240);
        assert_eq!(candidate.evidence_name, "?ECOVER.TXT");
        assert_eq!(
            volume
                .source_offset_for_candidate(&candidate)
                .expect("locate candidate"),
            10240
        );
        assert_eq!(
            volume
                .read_deleted_file(&candidate)
                .expect("recover candidate"),
            DELETED_CONTENT
        );
    }

    #[test]
    fn rejects_a_non_fat16_cluster_count() {
        let mut image = sample_fat16_image();
        image[19..21].copy_from_slice(&100_u16.to_le_bytes());

        let error = Fat16Volume::parse(&image).expect_err("reject non-FAT16 geometry");

        assert_eq!(error, Fat16Error::UnsupportedLayout);
    }
}
