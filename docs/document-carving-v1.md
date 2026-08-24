# Document Signature Carving v1

## Scope

This version adds **bounded, structural carving** for traditional cross-reference-table PDF files and ordinary single-disk ZIP containers, including the ZIP-based Open XML packages commonly used by Word, Excel, and PowerPoint. It is a recovery-candidate discovery feature, not a complete document parser, repair engine, malware scanner, or semantic validator.

PDF documents have a header, body, cross-reference information, and trailer; the trailer includes `startxref` and `%%EOF` markers.[1] ZIP archives use internal record signatures, a local file header for each stored entry, corresponding central-directory records, and an end-of-central-directory record.[2] Open XML documents are ZIP packages that include a `[Content_Types].xml` part and relationships parts, with package-specific content such as `word/document.xml`, `xl/workbook.xml`, or `ppt/presentation.xml`.[3]

## Carving contracts

| Candidate | Required structural checks | Output label | Intentional exclusions |
|---|---|---|---|
| **PDF** | A `%PDF-<major>.<minor>` header; a bounded `%%EOF`; a preceding `startxref` integer; and an in-range offset that points to a traditional `xref` table. | `pdf`, `Content validated` | XRef streams, damaged pointer repair, incremental-update interpretation, encrypted-content validation, object-stream parsing, and rendering. |
| **ZIP** | A local file header; a bounded end-of-central-directory record with an exact comment boundary; same-disk fields; a non-empty central directory; a matching entry count; and each central entry pointing to an in-range local header. | `zip`, `Content validated` | ZIP64, split archives, self-extracting prefixes, encrypted central directories, decompression, CRC verification, and archive repair. |
| **Open XML package** | All ZIP checks plus `[Content_Types].xml`, `_rels/.rels`, and one known application part. | `docx`, `xlsx`, or `pptx`, `Content validated` | XML-schema validation, relationships traversal, macro detection, content extraction, or proof that the document opens in a specific office suite. |

> **Content validated** means the carver confirmed the stated byte-level container structure. It does not mean the full document was parsed, decrypted, rendered, malware-scanned, semantically reviewed, or guaranteed complete.

## Bounds and deterministic behavior

Both carvers limit an individual candidate to **64 MiB**. A candidate that lacks a required boundary or fails a structural relation is ignored rather than exported as a weaker result. When a valid candidate is found, scanning resumes at its end, yielding deterministic identifiers and preventing duplicate discovery of contained bytes.

The ZIP parser uses offsets relative to the detected local-file-header boundary. This intentionally rejects self-extracting archives and container fragments whose central-directory offsets are relative to a different file origin. The PDF parser accepts only a conventional cross-reference table because that provides a testable, conservative v1 boundary.

## Evidence and user experience

PDF, ZIP, and Open XML candidates flow through the same local-only recovery pipeline as existing image candidates. They receive deterministic candidate identifiers, source offsets, safe no-overwrite exports, artifact hashes, recovery receipts, session-history entries, catalogue explanations, and desktop method filters. Binary previews stay metadata-only.

If a user needs a result beyond these boundaries, DiskTrace should describe it as **not available in this version**, rather than repairing bytes or promising a usable file.

## Deterministic test fixture

`fixtures/document-carving-multimethod-v1/` contains a synthetic raw image with one complete traditional PDF and one minimal DOCX-style Open XML package at known source offsets. Its verifier confirms discovery, filtering, safe export, receipt provenance, persisted-session recovery, and rejection of malformed PDF and ZIP controls.

## References

[1]: https://pypdf.readthedocs.io/en/latest/dev/pdf-format.html "The PDF Format — pypdf documentation"
[2]: https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT "APPNOTE.TXT — ZIP File Format Specification"
[3]: https://learn.microsoft.com/en-us/office/open-xml/about-the-open-xml-sdk "About the Open XML SDK for Office — Microsoft Learn"
