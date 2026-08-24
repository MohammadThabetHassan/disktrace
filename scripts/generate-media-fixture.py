#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures" / "media-carving-multimethod-v1"
GIF_OFFSET = 1024
AVI_OFFSET = 8192
MP4_OFFSET = 16384
IMAGE_LENGTH = 32768


def le_chunk(chunk_id: bytes, payload: bytes) -> bytes:
    chunk = bytearray(chunk_id)
    chunk.extend(len(payload).to_bytes(4, "little"))
    chunk.extend(payload)
    if len(payload) % 2:
        chunk.append(0)
    return bytes(chunk)


def build_gif() -> bytes:
    return bytes(
        [
            *b"GIF89a",
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
            0xFF,
            0xFF,
            0xFF,
            0x2C,
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
            0x3B,
        ]
    )


def build_avi() -> bytes:
    riff_data = bytearray(b"AVI ")
    riff_data.extend(le_chunk(b"LIST", b"hdrlAVIH"))
    riff_data.extend(le_chunk(b"LIST", b"movi"))
    return b"RIFF" + len(riff_data).to_bytes(4, "little") + bytes(riff_data)


def be_box(box_type: bytes, payload: bytes) -> bytes:
    return (len(payload) + 8).to_bytes(4, "big") + box_type + payload


def build_mp4() -> bytes:
    movie = be_box(b"mvhd", b"") + be_box(b"trak", b"")
    return (
        be_box(b"ftyp", b"isom\0\0\0\0isom")
        + be_box(b"moov", movie)
        + be_box(b"mdat", b"synthetic-media-data")
    )


def main() -> None:
    gif = build_gif()
    avi = build_avi()
    mp4 = build_mp4()
    if (
        GIF_OFFSET + len(gif) >= AVI_OFFSET
        or AVI_OFFSET + len(avi) >= MP4_OFFSET
        or MP4_OFFSET + len(mp4) > IMAGE_LENGTH
    ):
        raise RuntimeError("media fixture offsets do not fit the image")

    image = bytearray(IMAGE_LENGTH)
    image[GIF_OFFSET : GIF_OFFSET + len(gif)] = gif
    image[AVI_OFFSET : AVI_OFFSET + len(avi)] = avi
    image[MP4_OFFSET : MP4_OFFSET + len(mp4)] = mp4

    FIXTURE.mkdir(parents=True, exist_ok=True)
    source_path = FIXTURE / "source.img"
    source_path.write_bytes(image)
    (FIXTURE / "expected-carved.gif").write_bytes(gif)
    (FIXTURE / "expected-carved.avi").write_bytes(avi)
    (FIXTURE / "expected-carved.mp4").write_bytes(mp4)

    manifest = {
        "fixture_id": "media-carving-multimethod-v1",
        "scenario": "Raw image with structurally bounded GIF, standard RIFF/AVI, and self-contained MP4 candidates",
        "source": {
            "file": source_path.name,
            "sha256": hashlib.sha256(image).hexdigest(),
            "byte_length": len(image),
        },
        "expected_candidates": [
            {
                "id": "gif-carve-0000",
                "file_type": "gif",
                "source_range_start": GIF_OFFSET,
                "source_range_length": len(gif),
            },
            {
                "id": "avi-carve-0000",
                "file_type": "avi",
                "source_range_start": AVI_OFFSET,
                "source_range_length": len(avi),
            },
            {
                "id": "mp4-carve-0000",
                "file_type": "mp4",
                "source_range_start": MP4_OFFSET,
                "source_range_length": len(mp4),
            },
        ],
    }
    (FIXTURE / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
