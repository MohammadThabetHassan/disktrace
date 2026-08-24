# FAT32 deleted-entry feasibility v1

## Decision

DiskTrace does **not** add a FAT32 deleted-file recovery claim in the current increment. The existing FAT12 and FAT16 implementations rely on fixed root-directory layout and small deterministic fixtures. FAT32 stores the root directory in a cluster chain and requires independent validation of 28-bit FAT entry handling, active/deleted directory traversal, allocation boundaries, and recovery behavior after deletion.

> A FAT32 label without a root-chain, allocation, and refusal corpus would broaden the project’s recovery claim without sufficient evidence. DiskTrace therefore continues to refuse FAT32 recovery rather than presenting a best-effort parser as validated recovery.

## Implementation prerequisites

| Requirement | Why it is required before routing candidates |
| --- | --- |
| FAT32 geometry parser | It must validate bytes-per-sector, sectors-per-cluster, reserved sectors, FAT count, FAT32 sectors-per-FAT, total sectors, root cluster, data offset, FAT bounds, and the FAT32 cluster-count threshold with checked arithmetic. |
| Root-directory chain walker | The root directory cannot be treated as a fixed offset. It must follow valid 28-bit FAT entries, reject cycles, reject reserved/bad/free entries before a required continuation, and bound every cluster read. |
| Deleted short-entry filter | It must retain only deleted 8.3 file entries, reject long-name, directory, and volume-label entries, preserve the unknown first short-name character, and record the directory-entry offset. |
| Conservative extraction rule | A candidate may be recovered only when the directory metadata, first cluster, required byte length, and validated chain satisfy the declared contiguous or retained-chain policy. Zero-length and malformed entries need explicit refusal/empty-file controls. |
| Deterministic corpus | A generated FAT32 image must include a positive deleted candidate, a root-chain continuation, a malformed or cyclic chain refusal, and a candidate with overwritten/free allocation evidence that is not presented as recovered. |
| Workflow parity | Candidate method, source offset, bytes, stable identity, session persistence, exact-range preview, receipt-backed export, and audit behavior must match the established workflow contracts. |

## Evidence constraints

The FAT32 threshold is substantially larger than the FAT16 threshold, so a valid fixture is necessarily larger than the existing FAT12/FAT16 controls. A generated sparse image may establish parser behavior but cannot establish real-device, fragmented, overwritten, TRIM-affected, encrypted, or controller-discarded recovery performance.

A FAT32 implementation must not be committed merely to raise a feature count. It should be introduced only with the complete parser, generator, refusal fixtures, workflow integration, documentation, and full local plus hosted verification. Until then, the supported-recovery list remains unchanged.

## Next review trigger

Revisit this decision when the source-window performance corpus and release-control work have produced a stable baseline, or when a complete FAT32 fixture and root-chain implementation can be developed as one bounded, independently testable increment.
