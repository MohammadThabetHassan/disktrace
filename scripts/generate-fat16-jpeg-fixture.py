import hashlib
import json
from pathlib import Path

project_root = Path(__file__).resolve().parent.parent
fixture_dir = project_root / "fixtures" / "fat16-jpeg-multimethod-v1"
fixture_dir.mkdir(parents=True, exist_ok=True)

text_content = b"fat16 recovered text\n"
jpeg_content = bytes((
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x0B,
    0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xDA, 0x00,
    0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x11, 0x22, 0xFF, 0xD9,
))

bytes_per_sector = 512
total_sectors = 4120
fat_sectors = 17
root_entries = 32
root_sectors = 2
image = bytearray(bytes_per_sector * total_sectors)

image[0:3] = bytes((0xEB, 0x3C, 0x90))
image[3:11] = b"EFORGE16"
image[11:13] = bytes_per_sector.to_bytes(2, "little")
image[13] = 1
image[14:16] = (1).to_bytes(2, "little")
image[16] = 1
image[17:19] = root_entries.to_bytes(2, "little")
image[19:21] = total_sectors.to_bytes(2, "little")
image[21] = 0xF8
image[22:24] = fat_sectors.to_bytes(2, "little")
image[510:512] = bytes((0x55, 0xAA))

fat_offset = bytes_per_sector
image[fat_offset:fat_offset + 2] = (0xFFF8).to_bytes(2, "little")
image[fat_offset + 2:fat_offset + 4] = (0xFFFF).to_bytes(2, "little")
image[fat_offset + 4:fat_offset + 6] = (0xFFFF).to_bytes(2, "little")

root_offset = (1 + fat_sectors) * bytes_per_sector
deleted = root_offset
image[deleted] = 0xE5
image[deleted + 1:deleted + 8] = b"ECOVER "
image[deleted + 8:deleted + 11] = b"TXT"
image[deleted + 11] = 0x20
image[deleted + 26:deleted + 28] = (2).to_bytes(2, "little")
image[deleted + 28:deleted + 32] = len(text_content).to_bytes(4, "little")

data_offset = root_offset + root_sectors * bytes_per_sector
jpeg_offset = data_offset + 1024
malformed_jpeg_offset = jpeg_offset + 256
image[data_offset:data_offset + len(text_content)] = text_content
image[jpeg_offset:jpeg_offset + len(jpeg_content)] = jpeg_content
image[malformed_jpeg_offset:malformed_jpeg_offset + 8] = bytes((0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xD9))

image_path = fixture_dir / "source.img"
text_expected_path = fixture_dir / "expected-recovered.txt"
jpeg_expected_path = fixture_dir / "expected-carved.jpg"
manifest_path = fixture_dir / "manifest.json"

image_path.write_bytes(image)
text_expected_path.write_bytes(text_content)
jpeg_expected_path.write_bytes(jpeg_content)
manifest = {
    "fixture_id": "fat16_jpeg_multimethod_v1",
    "scenario": "deleted_fat16_root_entry_and_metadata_free_jpeg_signature",
    "source": {
        "path": "source.img",
        "sha256": hashlib.sha256(image).hexdigest(),
        "byte_length": len(image),
    },
    "expected_candidates": [
        {
            "id": "fat16-root-0000",
            "evidence_name": "?ECOVER.TXT",
            "source_offset": data_offset,
            "byte_length": len(text_content),
            "content_sha256": hashlib.sha256(text_content).hexdigest(),
            "method": "fat16_deleted_root_metadata",
            "validation": "recovered_unvalidated",
        },
        {
            "id": "jpeg-carve-0000",
            "evidence_name": "carved-jpeg-0000.jpg",
            "source_offset": jpeg_offset,
            "byte_length": len(jpeg_content),
            "content_sha256": hashlib.sha256(jpeg_content).hexdigest(),
            "method": "signature_carving_jpeg",
            "validation": "content_validated",
        },
    ],
}
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
