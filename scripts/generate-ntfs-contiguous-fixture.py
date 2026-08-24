#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures" / "ntfs-deleted-contiguous-v1"
BYTES_PER_SECTOR = 512
VOLUME_SECTORS = 4096
MFT_CLUSTER = 4
RECORD_SIZE = 1024
BITMAP_RECORD = 6
DELETED_RECORD = 7
DATA_CLUSTER = 64
NAME = "extent.txt"
CONTENT = b"ntfs contiguous\n"


def write_u16(target: bytearray, offset: int, value: int) -> None:
    target[offset : offset + 2] = value.to_bytes(2, "little")


def write_u32(target: bytearray, offset: int, value: int) -> None:
    target[offset : offset + 4] = value.to_bytes(4, "little")


def write_u64(target: bytearray, offset: int, value: int) -> None:
    target[offset : offset + 8] = value.to_bytes(8, "little")


def resident_attribute(attribute_type: int, value: bytes, instance: int) -> bytes:
    length = ((24 + len(value) + 7) // 8) * 8
    attribute = bytearray(length)
    write_u32(attribute, 0, attribute_type)
    write_u32(attribute, 4, length)
    attribute[8] = 0
    write_u16(attribute, 10, 0)
    write_u16(attribute, 12, 0)
    write_u16(attribute, 14, instance)
    write_u32(attribute, 16, len(value))
    write_u16(attribute, 20, 24)
    attribute[24 : 24 + len(value)] = value
    return bytes(attribute)


def nonresident_single_run_attribute(first_cluster: int, data_length: int, instance: int) -> bytes:
    attribute = bytearray(72)
    write_u32(attribute, 0, 0x80)
    write_u32(attribute, 4, len(attribute))
    attribute[8] = 1
    write_u16(attribute, 10, 0)
    write_u16(attribute, 12, 0)
    write_u16(attribute, 14, instance)
    write_u64(attribute, 16, 0)
    write_u64(attribute, 24, 0)
    write_u16(attribute, 32, 64)
    write_u16(attribute, 34, 0)
    write_u64(attribute, 40, BYTES_PER_SECTOR)
    write_u64(attribute, 48, data_length)
    write_u64(attribute, 56, data_length)
    attribute[64:68] = bytes((0x11, 0x01, first_cluster, 0x00))
    return bytes(attribute)


def fixed_up_record(record_number: int, flags: int, attributes: bytes) -> bytes:
    first_attribute_offset = 56
    used_size = first_attribute_offset + len(attributes) + 4
    record = bytearray(RECORD_SIZE)
    record[0:4] = b"FILE"
    write_u16(record, 4, 48)
    write_u16(record, 6, 3)
    write_u16(record, 16, 1)
    write_u16(record, 20, first_attribute_offset)
    write_u16(record, 22, flags)
    write_u32(record, 24, used_size)
    write_u32(record, 28, RECORD_SIZE)
    write_u16(record, 40, 2)
    write_u32(record, 44, record_number)
    record[first_attribute_offset : first_attribute_offset + len(attributes)] = attributes
    write_u32(record, first_attribute_offset + len(attributes), 0xFFFFFFFF)
    first_trailer = bytes(record[BYTES_PER_SECTOR - 2 : BYTES_PER_SECTOR])
    second_trailer = bytes(record[RECORD_SIZE - 2 : RECORD_SIZE])
    write_u16(record, 48, 0xA5A5)
    record[50:52] = first_trailer
    record[52:54] = second_trailer
    write_u16(record, BYTES_PER_SECTOR - 2, 0xA5A5)
    write_u16(record, RECORD_SIZE - 2, 0xA5A5)
    return bytes(record)


def file_name_value() -> bytes:
    value = bytearray(66 + len(NAME.encode("utf-16le")))
    value[64] = len(NAME)
    value[65] = 1
    value[66:] = NAME.encode("utf-16le")
    return bytes(value)


def main() -> None:
    image = bytearray(VOLUME_SECTORS * BYTES_PER_SECTOR)
    image[0:3] = b"\xEB\x52\x90"
    image[3:11] = b"NTFS    "
    write_u16(image, 11, BYTES_PER_SECTOR)
    image[13] = 1
    image[21] = 0xF8
    write_u64(image, 40, VOLUME_SECTORS)
    write_u64(image, 48, MFT_CLUSTER)
    write_u64(image, 56, MFT_CLUSTER + 1)
    image[64] = 0xF6
    image[68] = 0xF6
    write_u16(image, 510, 0xAA55)

    mft_offset = MFT_CLUSTER * BYTES_PER_SECTOR
    image[mft_offset : mft_offset + RECORD_SIZE] = fixed_up_record(0, 1, b"")

    bitmap = bytearray(VOLUME_SECTORS // 8)
    for cluster in range(MFT_CLUSTER, MFT_CLUSTER + 16):
        bitmap[cluster // 8] |= 1 << (cluster % 8)
    bitmap_attributes = resident_attribute(0x80, bytes(bitmap), 0)
    bitmap_record_offset = mft_offset + BITMAP_RECORD * RECORD_SIZE
    image[bitmap_record_offset : bitmap_record_offset + RECORD_SIZE] = fixed_up_record(
        BITMAP_RECORD, 1, bitmap_attributes
    )

    attributes = resident_attribute(0x30, file_name_value(), 0)
    attributes += nonresident_single_run_attribute(DATA_CLUSTER, len(CONTENT), 1)
    deleted_record_offset = mft_offset + DELETED_RECORD * RECORD_SIZE
    image[deleted_record_offset : deleted_record_offset + RECORD_SIZE] = fixed_up_record(
        DELETED_RECORD, 0, attributes
    )
    data_offset = DATA_CLUSTER * BYTES_PER_SECTOR
    image[data_offset : data_offset + len(CONTENT)] = CONTENT

    FIXTURE.mkdir(parents=True, exist_ok=True)
    source_path = FIXTURE / "source.img"
    expected_path = FIXTURE / "expected-recovered.txt"
    source_path.write_bytes(image)
    expected_path.write_bytes(CONTENT)
    manifest = {
        "fixture_id": "ntfs-deleted-contiguous-v1",
        "scenario": "NTFS volume with a free contiguous former extent retained by a deleted non-resident MFT record",
        "source": {
            "file": source_path.name,
            "sha256": hashlib.sha256(image).hexdigest(),
            "byte_length": len(image),
        },
        "expected_candidates": [
            {
                "id": "ntfs-contiguous-0000",
                "evidence_name": NAME,
                "file_type": "txt",
                "source_range_start": DATA_CLUSTER * BYTES_PER_SECTOR,
                "source_range_length": len(CONTENT),
            }
        ],
    }
    (FIXTURE / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
