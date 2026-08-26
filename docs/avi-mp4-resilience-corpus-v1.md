# AVI and MP4/MOV resilience corpus v1

## Purpose

This deterministic corpus strengthens regression evidence for the remaining full-buffer AVI and MP4/MOV discovery paths. It uses small synthetic byte controls that exercise malformed declared sizes, truncated headers, malformed prefixes before valid candidates, and adjacent valid candidates. It is a refusal-and-ordering regression layer, not a new recovery method or a source-access migration.

> A passing corpus result means that these selected synthetic controls completed without a panic and preserved the current supported-carver refusal and ordering behavior. It does not establish safety for every hostile input, fuzzing coverage, a memory-use bound for arbitrary image sizes, streaming discovery, fragmented-media recovery, codec validation, media playback safety, or general performance.

## Covered legacy behavior

| Method | Existing bounded acceptance rule | Resilience controls |
| --- | --- | --- |
| AVI | A RIFF/AVI candidate must remain within the existing 2 GiB structural cap and contain complete `hdrl` and `movi` lists. | Empty/truncated RIFF control, oversized declared RIFF length, malformed prefix before a valid candidate, and adjacent valid-candidate ordering. |
| MP4/MOV | A supported first `ftyp` box must be finite and recognized; a bounded `moov` with `mvhd` and `trak` must precede `mdat`; fragmented `moof` is refused. | Empty/truncated box control, unsupported extended/zero/oversized declared box lengths, malformed prefix before a valid candidate, and adjacent valid-candidate ordering. |

## Required assertions

Every corpus control must prove all applicable properties:

1. The carver returns normally without a panic.
2. A malformed or truncated declaration produces no candidate rather than an invented range.
3. A malformed prefix cannot suppress a later structurally valid candidate.
4. Adjacent valid candidates retain source-order offsets and deterministic evidence names.
5. The controls operate on minimized synthetic bytes and never open, execute, render, or export media payloads.

## Explicit non-claims

AVI and MP4/MOV discovery continue to use the existing full-buffer compatibility path. This corpus does not migrate them to bounded source windows, alter their caps, add MOV/AVI codec parsing, add fragmented recovery, add a cancellation-latency guarantee, create a fuzzer, establish exhaustive hostile-input resilience, or authorize a release.

## Verification

The corpus runs through the `ef-carve` unit suite and the ordinary `sh scripts/verify-all.sh` matrix. Its method and non-claim boundaries are also enforced by the release-document contract.
