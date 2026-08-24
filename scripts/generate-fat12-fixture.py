import hashlib
import json
from pathlib import Path

project_root = Path(__file__).resolve().parent.parent
fixture_dir = project_root / "fixtures" / "fat12-deleted-file-v1"
fixture_dir.mkdir(parents=True, exist_ok=True)
text_content = b"recover me\n"
png_content = bytes((
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0,
    0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99, 0x3D, 0x1D, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
))
image = bytearray(512 * 10)

image[0:3] = bytes((0xEB, 0x3C, 0x90))
image[3:11] = b"EFORGE  "
image[11:13] = (512).to_bytes(2, "little")
image[13] = 1
image[14:16] = (1).to_bytes(2, "little")
image[16] = 1
image[17:19] = (16).to_bytes(2, "little")
image[19:21] = (10).to_bytes(2, "little")
image[21] = 0xF8
image[22:24] = (1).to_bytes(2, "little")
image[510:512] = bytes((0x55, 0xAA))

fat = 512
image[fat:fat + 5] = bytes((0xF8, 0xFF, 0xFF, 0xFF, 0x0F))

root = 1024
image[root:root + 8] = b"ACTIVE  "
image[root + 8:root + 11] = b"TXT"
image[root + 11] = 0x20
image[root + 26:root + 28] = (3).to_bytes(2, "little")
image[root + 28:root + 32] = (6).to_bytes(4, "little")

deleted = root + 32
image[deleted] = 0xE5
image[deleted + 1:deleted + 8] = b"ELETED "
image[deleted + 8:deleted + 11] = b"TXT"
image[deleted + 11] = 0x20
image[deleted + 26:deleted + 28] = (2).to_bytes(2, "little")
image[deleted + 28:deleted + 32] = len(text_content).to_bytes(4, "little")

cluster_two_offset = 1536
png_offset = 4096
image[cluster_two_offset:cluster_two_offset + len(text_content)] = text_content
image[cluster_two_offset + 512:cluster_two_offset + 518] = b"active"
image[png_offset:png_offset + len(png_content)] = png_content

image_path = fixture_dir / "source.img"
text_expected_path = fixture_dir / "expected-recovered.txt"
png_expected_path = fixture_dir / "expected-carved.png"
manifest_path = fixture_dir / "manifest.json"
image_path.write_bytes(image)
text_expected_path.write_bytes(text_content)
png_expected_path.write_bytes(png_content)
manifest = {
    "fixture_id": "fat12_png_multimethod_v1",
    "scenario": "deleted_fat12_root_entry_and_metadata_free_png_signature",
    "source": {
        "path": "source.img",
        "sha256": hashlib.sha256(image).hexdigest(),
        "byte_length": len(image),
    },
    "expected_candidates": [
        {
            "id": "fat12-root-0000",
            "evidence_name": "?ELETED.TXT",
            "source_offset": cluster_two_offset,
            "byte_length": len(text_content),
            "content_sha256": hashlib.sha256(text_content).hexdigest(),
            "method": "fat12_deleted_root_metadata",
            "validation": "recovered_unvalidated",
        },
        {
            "id": "png-carve-0000",
            "evidence_name": "carved-png-0000.png",
            "source_offset": png_offset,
            "byte_length": len(png_content),
            "content_sha256": hashlib.sha256(png_content).hexdigest(),
            "method": "signature_carving_png",
            "validation": "content_validated",
        },
    ],
}
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
