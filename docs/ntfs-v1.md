# NTFS Deleted Resident-File Recovery v1

## Scope

This increment provides a **read-only, Master File Table–bounded NTFS recovery path** for deleted base file records whose on-record metadata and resident unnamed `$DATA` content remain internally consistent. It is deliberately narrower than general NTFS recovery.

An NTFS FILE record contains a header, a variable-length sequence of attributes, and an end marker. The record header identifies `FILE`, supplies the update-sequence-array offset and size, locates the first attribute, records the real and allocated record sizes, and contains flags that indicate whether the record is in use or a directory.[1] NTFS writes an update-sequence number at the end of each protected sector while keeping the original words in its update-sequence array; a reader must compare each sector trailer before restoring the original words.[2]

The NTFS boot record stores the `NTFS    ` system identifier, bytes per sector, sectors per cluster, volume sector count, the logical-cluster number of `$MFT`, and the MFT record-size encoding. A negative record-size value encodes a power-of-two byte count; for example, `-10` denotes 1,024 bytes.[3] `$FILE_NAME` is always resident, carries its UTF-16 name length at value offset `0x40`, and supports at most 255 Unicode characters.[4]

## Initial acceptance boundary

| Layer | Required checks |
|---|---|
| **Volume boot record** | Valid NTFS OEM identifier, boot signature, supported sector and cluster geometry, non-zero MFT logical-cluster number, and a supported positive or negative MFT record-size encoding. |
| **MFT record** | In-range MFT record, `FILE` signature, allocated and real sizes within the configured record size, a valid update-sequence array, matching update-sequence number in every protected sector, and fixup restoration before parsing attributes. |
| **Deleted regular-file state** | In-use flag clear, directory flag clear, base-record reference zero, and no extension-record or attribute-list handling. |
| **Metadata attributes** | At least one resident `$FILE_NAME` attribute with a bounded valid UTF-16 name, and exactly one unnamed resident `$DATA` attribute. |
| **Content** | Resident data length and offset fully bounded within the attribute and fixed-up record. DiskTrace exports exactly that byte range. |

> **Recovered — review recommended** means the deleted MFT record, its sector fixups, its resident name metadata, and its resident data attribute were structurally intact at scan time. It does **not** prove that any non-resident data, alternate stream, original parent path, or metadata outside the accepted record remains recoverable.

## Explicit refusals

This version refuses NTFS volumes with unsupported geometry, invalid boot fields, unavailable MFT records, invalid file-record fixups, extension records, directory records, in-use records, malformed or overlapping attributes, non-resident data, named data streams, attribute lists, files with no acceptable resident `$FILE_NAME`, files with ambiguous resident data, and all directory-index traversal. It does not follow runlists, parse `$Bitmap`, recover non-resident or sparse/compressed/encrypted files, reconstruct folder paths, repair records, or infer content after record reuse.

## Non-resident contiguous recovery extension

The second NTFS slice adds a separate, deliberately constrained path for deleted regular files whose unnamed `$DATA` attribute is non-resident. A non-resident attribute identifies its starting and last virtual cluster numbers, mapping-pairs offset, compression-unit size, allocated size, real data size, and initialized data size in its 64-byte header.[5] Its runlist represents each run by a variable-size length and a signed logical-cluster offset relative to the preceding run; a zero header terminates the list, while a zero-sized offset field represents a sparse run.[6]

| Layer | Required checks |
|---|---|
| **Deleted file record** | All resident-record checks still apply to the `FILE` header, fixups, deleted base-record state, and a valid resident `$FILE_NAME`. |
| **Non-resident `$DATA`** | Exactly one unnamed `$DATA`, non-resident flag set, no compressed/encrypted/sparse flags, compression unit zero, starting VCN zero, initialized data length equal to data length, data length non-zero, and allocated size equal to a whole number of clusters. |
| **Runlist** | Exactly one terminated run, non-zero length and signed offset widths, no sparse run, positive in-range logical cluster number, run cluster count exactly matching the VCN span and allocated length, and no trailing run bytes. |
| **Allocation state** | The known `$Bitmap` MFT record must itself pass fixup and bounded data checks, and every candidate data cluster must be currently marked free. |

> **Recovered — review recommended** means metadata preserves one valid former extent and the current NTFS allocation bitmap reports that extent free. This is evidence of current allocation state, **not proof that the former file bytes were never overwritten**.

This extension refuses fragmented, sparse, compressed, encrypted, named, multi-attribute, or partially initialized non-resident files. It also refuses an unreadable or malformed allocation bitmap, candidate extents touching `$MFT` or `$Bitmap`, and any currently allocated candidate cluster. It does not parse arbitrary MFT runlists, reconstruct paths, recover alternate data streams, or repair deleted metadata.

## Deterministic evidence

The final fixture will use a synthetic NTFS image whose boot record identifies a bounded MFT, whose deleted FILE record carries valid update-sequence protection, a resident `$FILE_NAME`, and a resident unnamed `$DATA` attribute. The verifier will test discovery, direct export, receipt provenance, persisted-session recovery, rejected fixup corruption, and rejected non-resident data.

## References

[1]: https://flatcap.github.io/linux-ntfs/ntfs/concepts/file_record.html "File Record — NTFS Documentation"

[2]: https://flatcap.github.io/linux-ntfs/ntfs/concepts/fixup.html "Fixup — NTFS Documentation"

[3]: https://flatcap.github.io/linux-ntfs/ntfs/files/boot.html "$Boot — NTFS Documentation"

[4]: https://flatcap.github.io/linux-ntfs/ntfs/attributes/file_name.html "$FILE_NAME — NTFS Documentation"

[5]: https://flatcap.github.io/linux-ntfs/ntfs/concepts/attribute_header.html "Attribute Header — NTFS Documentation"

[6]: https://flatcap.github.io/linux-ntfs/ntfs/concepts/data_runs.html "Data Runs — NTFS Documentation"
