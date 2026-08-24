#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import io
import json
from pathlib import Path
from zipfile import ZIP_STORED, ZipFile, ZipInfo

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures" / "document-carving-multimethod-v1"
PDF_OFFSET = 1024
DOCX_OFFSET = 16384
IMAGE_LENGTH = 32768


def build_pdf() -> bytes:
    document = bytearray(b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n")
    document.extend(b"2 0 obj\n<< /Type /Pages /Count 0 >>\nendobj\n")
    xref_offset = len(document)
    document.extend(b"xref\n0 3\n0000000000 65535 f \n")
    document.extend(b"0000000009 00000 n \n0000000058 00000 n \n")
    document.extend(b"trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n")
    document.extend(str(xref_offset).encode("ascii"))
    document.extend(b"\n%%EOF")
    return bytes(document)


def fixed_zip_info(name: str) -> ZipInfo:
    info = ZipInfo(name, date_time=(2024, 1, 1, 0, 0, 0))
    info.compress_type = ZIP_STORED
    info.create_system = 3
    info.external_attr = 0o600 << 16
    return info


def build_docx_package() -> bytes:
    output = io.BytesIO()
    entries = {
        "[Content_Types].xml": (
            b'<?xml version="1.0" encoding="UTF-8"?>'
            b'<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>'
        ),
        "_rels/.rels": (
            b'<?xml version="1.0" encoding="UTF-8"?>'
            b'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>'
        ),
        "word/document.xml": (
            b'<?xml version="1.0" encoding="UTF-8"?>'
            b'<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>'
        ),
    }
    with ZipFile(output, mode="w", compression=ZIP_STORED, strict_timestamps=True) as archive:
        for name, contents in entries.items():
            archive.writestr(fixed_zip_info(name), contents)
    return output.getvalue()


def main() -> None:
    pdf = build_pdf()
    docx = build_docx_package()
    if PDF_OFFSET + len(pdf) >= DOCX_OFFSET or DOCX_OFFSET + len(docx) > IMAGE_LENGTH:
        raise RuntimeError("document fixture offsets do not fit the image")

    image = bytearray(IMAGE_LENGTH)
    image[PDF_OFFSET : PDF_OFFSET + len(pdf)] = pdf
    image[DOCX_OFFSET : DOCX_OFFSET + len(docx)] = docx

    FIXTURE.mkdir(parents=True, exist_ok=True)
    source_path = FIXTURE / "source.img"
    pdf_path = FIXTURE / "expected-carved.pdf"
    docx_path = FIXTURE / "expected-carved.docx"
    source_path.write_bytes(image)
    pdf_path.write_bytes(pdf)
    docx_path.write_bytes(docx)

    source_sha256 = hashlib.sha256(image).hexdigest()
    manifest = {
        "fixture_id": "document-carving-multimethod-v1",
        "scenario": "Raw image with a conventional cross-reference PDF and a minimal DOCX-style Open XML ZIP package",
        "source": {"file": source_path.name, "sha256": source_sha256, "byte_length": len(image)},
        "expected_candidates": [
            {
                "id": "pdf-carve-0000",
                "file_type": "pdf",
                "source_range_start": PDF_OFFSET,
                "source_range_length": len(pdf),
            },
            {
                "id": "zip-carve-0000",
                "file_type": "docx",
                "source_range_start": DOCX_OFFSET,
                "source_range_length": len(docx),
            },
        ],
    }
    (FIXTURE / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
