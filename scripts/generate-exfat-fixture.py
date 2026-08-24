#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures" / "exfat-contiguous-deleted-v1"
BYTES_PER_SECTOR = 512
VOLUME_SECTORS = 2048
FAT_OFFSET_SECTORS = 24
FAT_LENGTH_SECTORS = 16
CLUSTER_HEAP_OFFSET_SECTORS = 40
CLUSTER_COUNT = VOLUME_SECTORS - CLUSTER_HEAP_OFFSET_SECTORS
ROOT_CLUSTER = 2
BITMAP_CLUSTER = 3
CONTENT_CLUSTER = 4
CONTENT = b"exfat recovered\n"
NAME = "recover.txt"


def write_u16(image: bytearray, offset: int, value: int) -> None:
    image[offset : offset + 2] = value.to_bytes(2, "little")


def write_u32(image: bytearray, offset: int, value: int) -> None:
    image[offset : offset + 4] = value.to_bytes(4, "little")


def write_u64(image: bytearray, offset: int, value: int) -> None:
    image[offset : offset + 8] = value.to_bytes(8, "little")


def set_entry_checksum(entries: list[bytearray]) -> None:
    checksum = 0
    for entry_index, entry in enumerate(entries):
        for byte_index, byte in enumerate(entry):
            if entry_index == 0 and byte_index in (2, 3):
                continue
            checksum = ((checksum >> 1) | ((checksum & 1) << 15))
            checksum = (checksum + byte) & 0xFFFF
    entries[0][2:4] = checksum.to_bytes(2, "little")


def write_boot_checksum(image: bytearray) -> None:
    checksum = 0
    for offset, byte in enumerate(image[: BYTES_PER_SECTOR * 11]):
        if offset in (106, 107, 112):
            continue
        checksum = ((checksum >> 1) | ((checksum & 1) << 31))
        checksum = (checksum + byte) & 0xFFFFFFFF
    for offset in range(BYTES_PER_SECTOR * 11, BYTES_PER_SECTOR * 12, 4):
        write_u32(image, offset, checksum)


def main() -> None:
    image = bytearray(VOLUME_SECTORS * BYTES_PER_SECTOR)
    image[0:3] = b"\xEB\x76\x90"
    image[3:11] = b"EXFAT   "
    write_u64(image, 72, VOLUME_SECTORS)
    write_u32(image, 80, FAT_OFFSET_SECTORS)
    write_u32(image, 84, FAT_LENGTH_SECTORS)
    write_u32(image, 88, CLUSTER_HEAP_OFFSET_SECTORS)
    write_u32(image, 92, CLUSTER_COUNT)
    write_u32(image, 96, ROOT_CLUSTER)
    write_u16(image, 104, 0x0100)
    image[108] = 9
    image[109] = 0
    image[110] = 1
    image[112] = 0xFF
    write_u16(image, 510, 0xAA55)
    for sector in range(1, 9):
        write_u32(image, sector * BYTES_PER_SECTOR + BYTES_PER_SECTOR - 4, 0xAA550000)

    fat_offset = FAT_OFFSET_SECTORS * BYTES_PER_SECTOR
    write_u32(image, fat_offset + ROOT_CLUSTER * 4, 0xFFFFFFFF)
    write_u32(image, fat_offset + BITMAP_CLUSTER * 4, 0xFFFFFFFF)
    cluster_heap_offset = CLUSTER_HEAP_OFFSET_SECTORS * BYTES_PER_SECTOR
    root_offset = cluster_heap_offset
    bitmap_offset = cluster_heap_offset + BYTES_PER_SECTOR
    content_offset = cluster_heap_offset + 2 * BYTES_PER_SECTOR

    image[root_offset] = 0x81
    write_u32(image, root_offset + 20, BITMAP_CLUSTER)
    write_u64(image, root_offset + 24, (CLUSTER_COUNT + 7) // 8)

    primary = bytearray(32)
    stream = bytearray(32)
    filename = bytearray(32)
    primary[0] = 0x85
    primary[1] = 2
    write_u16(primary, 4, 0x0020)
    stream[0] = 0xC0
    stream[1] = 0x03
    stream[3] = len(NAME)
    write_u64(stream, 8, len(CONTENT))
    write_u32(stream, 20, CONTENT_CLUSTER)
    write_u64(stream, 24, len(CONTENT))
    filename[0] = 0xC1
    filename[2 : 2 + len(NAME.encode("utf-16le"))] = NAME.encode("utf-16le")
    entries = [primary, stream, filename]
    set_entry_checksum(entries)
    for entry in entries:
        entry[0] &= 0x7F
    for index, entry in enumerate(entries, start=1):
        offset = root_offset + index * 32
        image[offset : offset + 32] = entry

    image[bitmap_offset] = 0b00000011
    image[content_offset : content_offset + len(CONTENT)] = CONTENT
    write_boot_checksum(image)

    FIXTURE.mkdir(parents=True, exist_ok=True)
    source_path = FIXTURE / "source.img"
    expected_path = FIXTURE / "expected-recovered.txt"
    source_path.write_bytes(image)
    expected_path.write_bytes(CONTENT)
    manifest = {
        "fixture_id": "exfat-contiguous-deleted-v1",
        "scenario": "Checksummed exFAT volume with one deleted root file in a contiguous, currently free cluster extent",
        "source": {
            "file": source_path.name,
            "sha256": hashlib.sha256(image).hexdigest(),
            "byte_length": len(image),
        },
        "expected_candidates": [
            {
                "id": "exfat-root-0000",
                "evidence_name": NAME,
                "file_type": "txt",
                "source_range_start": content_offset,
                "source_range_length": len(CONTENT),
            }
        ],
    }
    (FIXTURE / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
