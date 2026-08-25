# DiskTrace v0.1.0 — draft release notes

> **Draft for a future authorized release.** This document is local-only and does not correspond to a Git tag, hosted workflow run, published release, or downloadable public artifact.

## Overview

DiskTrace is a local-first, cross-platform desktop application for reviewing bounded recovery candidates from disk-image files. It is designed for ordinary recovery workflows while making source identity, method limitations, candidate validation, destination safety, receipts, and repeatable local evidence visible.

## Included recovery methods

The proposed initial release supports deleted FAT12 and FAT16 root-directory metadata, deleted contiguous exFAT root metadata, deleted resident and narrow contiguous non-resident NTFS records, and bounded structural carving for PNG, JPEG, GIF, standard RIFF/AVI, self-contained MP4/MOV, PDF, and ZIP/Open XML. PNG, JPEG, and GIF discovery use method-specific bounded source windows with exact legacy-candidate parity enforcement; all other discovery, recovery, and export paths retain full-buffer compatibility behavior. Each method records candidate metadata and validation state; none promises complete recovery or original-file availability.

## Evidence and safety workflow

DiskTrace scans local image paths without writing to them. It records source byte length, SHA-256, and BLAKE3 values in local sessions; blocks recovery when the current source no longer matches; requires a separate destination; creates a receipt for each export; audits recorded receipts and current recovered output hashes; and can create a local Markdown case brief that contains no source-image or recovered-file payload bytes.

## Platform artifacts

A locally verified Linux x86_64 bundle and a Linux-host Windows x86_64 cross-target compatibility ZIP can be generated with checksums and bounded smoke verification. The exact hosted native Windows workflow additionally verifies its portable bundle, installer creation/checksums, disposable silent install/uninstall mechanics, and SBOM review artifact. These results remain CI evidence, not manual Windows acceptance, signing, SmartScreen, accessibility, upgrade, or broad hardware evidence. Hosted macOS 14 ARM64 validation produces an unsigned review binary only; it is not a consumer macOS package, Intel-macOS, signing, notarization, Gatekeeper, or manual-acceptance claim.

## Verification evidence required before publication

Before any actual v0.1.0 publication, the owner must select an exact source target and artifact scope; run the full local matrix, artifact verifiers, and dependency review on that revision; obtain green hosted Linux, native Windows, macOS 14 ARM64, and CodeQL workflows on that exact target; verify every advertised artifact on its native target platform; record final hashes, byte sizes, SBOM/advisory disposition, manual acceptance outcomes, known limits, governance, and security-reporting route; and review the final notes. Only after that evidence exists may the owner separately authorize an annotated semantic tag, release page, approved assets, and any signing or notarization action.

## Known limits

DiskTrace does not acquire live devices, bypass encryption, write to source images, upload data, provide cloud telemetry, guarantee every deleted file can be recovered, establish legal chain of custody, prove ownership or authenticity, assess malware safety, or replace professional forensic procedures. It does not claim whole-scan streaming, parser-loop cancellation, fragmented/overwritten/TRIM/controller-discarded recovery, encrypted recovery, manual accessibility acceptance, signed distribution, or production-release status.
