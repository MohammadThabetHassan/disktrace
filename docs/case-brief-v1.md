# Local case brief contract, version 1

A **case brief** is a human-readable Markdown summary generated locally from a completed DiskTrace session. It is available from Evidence Mode through **Save case brief** and from the optional CLI:

```sh
cargo run -p ef-cli -- case-brief /path/to/session.evidenceforge.json /path/to/case-brief.md
```

## Included observations

The brief recomputes and records the current source-integrity result and recorded-export audit when it is generated. It includes the session identifier, source path and byte length, SHA-256 and BLAKE3 source values, candidate totals, recovery-method and validation summaries, candidate metadata inventory, recorded export paths, current export-audit states, and an explicit limitations section.

| Included | Purpose |
| --- | --- |
| Current source status | Records whether the current local source matches the session’s saved byte length, SHA-256, and BLAKE3 identity. |
| Candidate inventory | Makes the bounded recovery observations readable without presenting recovered payloads. |
| Export audit | Records whether the persisted receipt and current recovered output hashes match the session’s recorded receipt. |
| Limitations | Prevents the report from being mistaken for an authenticity, malware-safety, legal-admissibility, or completeness claim. |

## Excluded data and boundaries

The brief contains **no source-image bytes, recovered-file payload bytes, cloud upload, telemetry, direct-device acquisition, or automatic publication**. It does not replace a forensic acquisition log, chain-of-custody record, write-blocking process, independently verified evidence-handling procedure, or professional legal advice.

> A verified brief is a repeatable local integrity observation at the time it is generated. It does not prove who created a file, whether every deleted item was recovered, or that a recovered file is safe to open.

When retaining the report, keep it with the local session manifest, related receipt files, and source image so its observations can be reproduced later.
