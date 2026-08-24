#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures" / "ntfs-deleted-resident-v1"
BYTES_PER_SECTOR = 512
VOLUME_SECTORS = 4096
MFT_CLUSTER = 4
RECORD_SIZE = 1024
NAME = "gone.txt"
CONTENT = b"ntfs recovered\n"


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
    write_u64(record, 32, 0)
    write_u16(record, 40, 2)
    write_u32(record, 44, record_number)
    record[first_attribute_offset : first_attribute_offset + len(attributes)] = attributes
    write_u32(record, first_attribute_offset + len(attributes), 0xFFFFFFFF)
    write_u16(record, 48, 0xA5A5)
    write_u16(record, 50, 0x1111)
    write_u16(record, 52, 0x2222)
    write_u16(record, BYTES_PER_SECTOR - 2, 0xA5A5)
    write_u16(record, RECORD_SIZE - 2, 0xA5A5)
    return bytes(record)


def deleted_record() -> bytes:
    filename_value = bytearray(66 + len(NAME) * 2)
    filename_value[64] = len(NAME)
    filename_value[65] = 1
    filename_value[66 : 66 + len(NAME.encode("utf-16le"))] = NAME.encode("utf-16le")
    attributes = resident_attribute(0x30, bytes(filename_value), 0)
    attributes += resident_attribute(0x80, CONTENT, 1)
    return fixed_up_record(1, 0, attributes)


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
    image[mft_offset + RECORD_SIZE : mft_offset + 2 * RECORD_SIZE] = deleted_record()

    FIXTURE.mkdir(parents=True, exist_ok=True)
    source_path = FIXTURE / "source.img"
    expected_path = FIXTURE / "expected-recovered.txt"
    source_path.write_bytes(image)
    expected_path.write_bytes(CONTENT)
    manifest = {
        "fixture_id": "ntfs-deleted-resident-v1",
        "scenario": "NTFS volume with a fixed-up deleted MFT record containing one resident unnamed data stream",
        "source": {
            "file": source_path.name,
            "sha256": hashlib.sha256(image).hexdigest(),
            "byte_length": len(image),
        },
        "expected_candidates": [
            {
                "id": "ntfs-resident-0000",
                "evidence_name": NAME,
                "file_type": "txt",
                "source_range_start": 3264,
                "source_range_length": len(CONTENT),
            }
        ],
    }
    (FIXTURE / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
