# DiskTrace v0.1.0 — draft release notes

> **Draft for a future authorized release.** This document is local-only and does not correspond to a Git tag, hosted workflow run, published release, or downloadable public artifact.

## Overview

DiskTrace is a local-first, cross-platform desktop application for reviewing bounded recovery candidates from disk-image files. It is designed for ordinary recovery workflows while making source identity, method limitations, candidate validation, destination safety, receipts, and repeatable local evidence visible.

## Included recovery methods

The initial release supports deleted FAT12 and FAT16 root-directory metadata, deleted contiguous exFAT root metadata, deleted resident and contiguous non-resident NTFS records, and bounded PNG, JPEG, PDF, ZIP, and Open XML signature carving. Each method records its candidate metadata and validation state; none promises complete recovery or original-file availability.

## Evidence and safety workflow

DiskTrace scans local image paths without writing to them. It records source byte length, SHA-256, and BLAKE3 values in local sessions; blocks recovery when the current source no longer matches; requires a separate destination; creates a receipt for each export; audits recorded receipts and current recovered output hashes; and can create a local Markdown case brief that contains no source-image or recovered-file payload bytes.

## Platform artifacts

A locally verified Linux x86_64 bundle and a Linux-host Windows x86_64 cross-target compatibility ZIP can be generated with checksums and bounded smoke verification. The Windows compatibility ZIP is not native Windows release evidence. Native Windows installer, signing, SmartScreen, accessibility, clean-machine, and macOS validation remain release gates.

## Verification evidence required before publication

Before any actual v0.1.0 publication, the owner must authorize a repository and commit identity, run the full local matrix on the intended revision, execute hosted validation on that revision, verify artifacts on their native target platforms, prepare final hashes and known limits, configure the chosen governance and security-reporting route, create an annotated semantic tag, and review a draft release with assets before publishing.

## Known limits

DiskTrace does not acquire live devices, bypass encryption, write to source images, upload data, provide cloud telemetry, guarantee every deleted file can be recovered, establish legal chain of custody, prove ownership or authenticity, assess malware safety, or replace professional forensic procedures.
