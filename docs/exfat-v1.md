# exFAT Deleted Entry Recovery v1

## Scope

This increment provides a **read-only, root-directory-only exFAT recovery path** for deleted regular files whose entry set remains structurally intact and whose former content extent is a contiguous sequence of clusters currently marked free in the active allocation bitmap. It is intentionally narrower than general exFAT recovery.

The exFAT specification defines a boot region with main and backup boot sectors, volume geometry, a cluster heap, and a root directory described through the active FAT. It requires validation of a boot region’s checksum and valid field ranges before using its boot sector.[1] It also defines a File entry set as one File primary entry followed immediately by exactly one Stream Extension entry and one or more File Name entries, with a set checksum that implementations must validate before using the set.[1]

## Input acceptance

| Layer | Required checks |
|---|---|
| **Main boot region** | `EXFAT   ` identifier, boot signature, valid extended-boot signatures, a matching main boot checksum sector, supported sector and cluster shifts, one FAT, valid volume geometry, and an in-range root-directory cluster. |
| **Root directory** | A readable, loop-free root FAT chain within the declared cluster heap, and a readable active allocation-bitmap entry. |
| **Deleted entry set** | An inactive File primary type, exactly two inactive secondary entries, an inactive Stream Extension, one inactive File Name entry, a valid stored set checksum after restoring only the in-use bit for checksum evaluation, a non-empty valid UTF-16 name, and a regular-file attribute. |
| **Data extent** | Allocation possible and `NoFatChain` flags set, valid data length equal to data length, a non-zero in-range first cluster, a bounded contiguous extent, and every cluster in that extent currently clear in the active allocation bitmap. |

> **Recovered — review recommended** means the deleted entry set was structurally consistent and its contiguous former cluster extent is currently reported free. It does **not** prove that those clusters still hold all original bytes or were never overwritten after deletion.

## Extraction boundary

For an accepted candidate, the source range begins at the first cluster of the recorded extent. DiskTrace reads exactly the recorded `DataLength` from the contiguous extent and never writes to the source image. Candidate identifiers are deterministic in root-directory order.

The allocation bitmap is evidence about current allocation state, not historical ownership. DiskTrace records this method as metadata-based recovery and does not promote it to content validation without a file-format-specific validator.

## Explicit refusals

This version refuses a volume when boot-region or geometry checks fail. It refuses deleted files with a fragmented FAT chain, a directory attribute, vendor extensions, more than one File Name entry, missing or malformed secondary entries, checksum mismatch, unallocated or ambiguous metadata, invalid UTF-16 names, missing allocation bitmap, allocated candidate clusters, out-of-range extents, invalid data-length relation, multi-FAT/TexFAT layouts, deleted subdirectories, and recursive directory traversal.

The parser does not repair boot sectors, recover files from deleted subdirectories, follow fragmented deleted FAT chains, interpret upcase tables, validate timestamps or file-name hashes, inspect backup boot recovery, or infer content after clusters are reused.

## Deterministic fixture

`fixtures/exfat-contiguous-deleted-v1/` holds a minimal 512-byte-sector exFAT image with a valid main boot checksum, root-directory FAT chain, allocation bitmap, one deleted contiguous text file, and a known source extent. Its verifier checks discovery, safe export, receipt provenance, saved-session recovery, and rejection of invalid checksum, allocated-cluster, and malformed-entry controls.

## Reference

[1]: https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification "exFAT File System Specification — Microsoft Learn"
