# Media carving contract, version 1

## Purpose and safety boundary

This contract defines DiskTrace’s bounded, local-only carving support for **GIF**, **standard RIFF/AVI**, and a narrow **self-contained MP4/MOV** subset. It is intended to find recoverable byte ranges after deletion, metadata loss, a quick format, or a full format where the relevant bytes have not subsequently been overwritten. It is not a decoder, media repair system, playback validator, fragmented-video reconstruction engine, or claim that every former file can be restored.

> A structurally validated candidate means only that the format-specific acceptance checks below passed against a bounded local byte range. It does not establish original filename, directory, ownership, authenticity, safety, playability, codec support, semantic completeness, or preservation of all original bytes.

| Format family | Recovery method | Accepted boundary | Explicit refusals |
| --- | --- | --- | --- |
| GIF87a/GIF89a | `signature_carving_gif` | Supported header, logical-screen descriptor, bounded colour tables, image and extension sub-blocks, and final trailer (`0x3B`). | Truncated data blocks, invalid supported LZW minimum code size, missing trailer, malformed tables, and range overruns. |
| Standard RIFF/AVI | `signature_carving_avi` | `RIFF`/`AVI ` header, declared RIFF size, safely aligned chunks/lists, and required `hdrl` and `movi` lists. | Invalid lengths or padding, missing required lists, RF64, OpenDML extensions, codec inspection, payload decoding, repair, and range overruns. |
| Self-contained MP4/MOV | `signature_carving_mp4` | Recognised MP4-family or QuickTime `ftyp` brand, finite 32-bit top-level boxes, a bounded `moov` with `mvhd` and `trak`, then `mdat`. | Fragmented `moof` media, zero or extended-size boxes, unknown brands, missing movie/media boxes, external references, codec validation, sample-offset validation, repair, and range overruns. |

## Why the MP4/MOV scope is deliberately narrow

ISO Base Media files are a series of sized four-character-code boxes, including `ftyp`, `moov`, `mdat`, and `moof`. The same family supports nested structures, fragments, multiple brands, and specialised media layouts. The first DiskTrace implementation accepts only a self-contained, sequential subset with a bounded movie metadata box before a media data box. It rejects fragmented media rather than presenting a loosely recognised signature as a recovered video. [1]

The AVI boundary is similarly conservative. A standard AVI file is a RIFF container whose declared file size bounds its data, and its expected organisation includes header (`hdrl`) and media (`movi`) lists. DiskTrace validates that envelope only; it does not interpret stream codec payloads or OpenDML extensions. [2]

GIF recovery walks the structured byte stream rather than trusting its six-byte signature. The GIF specification defines bounded data sub-blocks and a stream trailer, allowing the candidate range to end at the trailer without invoking an image renderer. [3]

## Full-format recovery expectations

A full or quick format can remove filesystem metadata while leaving some raw bytes on the medium. In that situation, filesystem metadata recovery may succeed only where the relevant records or allocation structures survive; structural carving may find a supported, contiguous byte range even where its original metadata is gone. Neither technique can guarantee every original file, path, name, fragment, encrypted item, overwritten sector, or data that an SSD has discarded through TRIM or later controller activity.

Established open-source carving guidance reaches the same practical boundary: raw signature recovery can work after severe filesystem damage or reformatting, but its best whole-file result depends on non-fragmented bytes remaining readable, and recovered output must go to a different destination from the source. [4] [5]

## Preview boundary

Evidence Mode can display only bounded local facts. GIF previews show version, logical screen, global-colour-table state, and trailer state. AVI previews show the RIFF form, declared container length, and required-list state. MP4/MOV previews show the container family, up to 32 top-level box names, movie-metadata state, and `mdat` payload length. No preview renders, opens, decodes, decompresses, executes, or invokes an external media application.

## Deterministic verification

`fixtures/media-carving-multimethod-v1/` contains a synthetic raw image with one GIF at offset 1024, one standard AVI at offset 8192, and one self-contained MP4 at offset 16384. The generator writes expected bytes and a source-hash manifest. `scripts/verify-media-recovery.sh` regenerates this fixture and verifies scan discovery, CLI filtering, structure summaries, source-storage destination rejection, receipt-backed byte-for-byte exports, and malformed AVI/fragmented-MP4 refusal controls. The master local matrix invokes this verifier.

## References

[1] [MP4 Registration Authority, Boxes (Atoms)](https://mp4ra.org/registered-types/boxes)

[2] [Microsoft, AVI RIFF File Reference](https://learn.microsoft.com/en-us/windows/win32/directshow/avi-riff-file-reference)

[3] [W3C, GIF89a Specification](https://www.w3.org/Graphics/GIF/spec-gif89a.txt)

[4] [CGSecurity, PhotoRec](https://www.cgsecurity.org/wiki/photoRec)

[5] [CGSecurity, Recovering deleted files using PhotoRec](https://www.cgsecurity.org/testdisk_doc/photorec.html)
