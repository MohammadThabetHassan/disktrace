# DiskTrace synthetic performance-control corpus v1

## Purpose

This contract expands DiskTrace’s deterministic scan controls beyond one sparse source. The corpus detects compatibility and regression drift across deliberately constructed byte patterns while keeping performance wording narrow and reproducible.

> These sources are **synthetic byte controls**, not disk acquisitions, hardware benchmarks, filesystem workloads, or evidence of real-world recovery throughput.

Every scenario uses only zero-filled files, the repository’s existing synthetic FAT12/PNG seed fixture, and deliberately malformed PNG signatures. No real disk image, recovered payload, credential, personal data, or external device is required or generated.

## Control scenarios

| Scenario ID | Construction | Expected result | Primary purpose |
|---|---|---|---|
| `large-sparse-png-v1` | A 64 MiB sparse source with one embedded existing FAT12/PNG seed at a fixed offset. | Exactly one PNG candidate at the recorded offset. | Retains the existing larger full-buffer regression control. |
| `signature-dense-png-v1` | A 16 MiB zero-filled source with an invalid PNG signature at each fixed 16 KiB interval and one valid embedded seed at 8 MiB. | Exactly one PNG candidate at the recorded seed PNG offset; malformed signatures are refused. | Exercises many structurally rejected signatures while retaining one accepted range. |
| `signature-dense-refusal-v1` | A 16 MiB zero-filled source with only fixed-interval malformed PNG signatures. | No candidates. | Exercises all-refusal behavior without treating a signature alone as a recoverable file. |
| `multi-candidate-png-v1` | A 32 MiB zero-filled source with three non-overlapping copies of the existing seed at fixed offsets. | Exactly three PNG candidates at the recorded offsets. | Detects candidate ordering/count drift across separated accepted ranges. |

The source generator must write files from deterministic offsets and interval geometry. It must reject invalid scenario names, unsafe output paths, non-decimal overrides, overlaps, and a seed that cannot fit. The verifier must scan newly generated temporary sources and assert exact candidate counts, expected method/file type, and every accepted PNG source offset.

## Measurement method

The measurement harness builds the CLI, generates one temporary source per scenario, runs `evidenceforge scan` for each configured repetition, and writes comma-separated rows containing scenario ID, byte count, run number, elapsed nanoseconds, candidate count, expected candidate count, and expected PNG offsets. A row is valid only if the selected candidate identities and offsets match the scenario contract.

The harness is a local measurement aid. It does not run in the ordinary verification matrix because elapsed time is environment dependent. The ordinary matrix runs the exact functional control once per scenario and rejects candidate-count or offset drift.

| Required reporting attribute | Reason |
|---|---|
| Scenario geometry and source size | Prevents comparison of unlike synthetic workloads. |
| Candidate count and PNG offsets | Shows that the timing corresponds to the intended scan result. |
| Repetition count and elapsed nanoseconds | Supports local variance review without setting a fake cross-machine threshold. |
| Machine/environment note in a recorded baseline | Prevents the number from being represented as a universal throughput claim. |
| Explicit non-generalization statement | Prevents synthetic controls from being marketed as real-device or forensic recovery benchmarks. |

## Interpretation boundary

The corpus exercises the current full-buffer scan path, which identifies the source, hashes it, reads it into memory, runs filesystem and structural discovery, checks bounded PNG parity, and serializes scan output. A result may reveal local regression, but it does not measure a physical drive’s bandwidth, fragmented filesystem behavior, multi-gigabyte memory pressure, cache state, malicious-input resilience, or recovered-file completeness.

The signature-dense scenarios must not be used to claim a new parser algorithm, complete cancellation support, windowed whole-scan discovery, adversarial hardening, or a universal malformed-file refusal guarantee. They are narrow compatibility controls for a selected fixed byte pattern.

## Acceptance criteria

| Criterion | Required evidence |
|---|---|
| Deterministic generation | Repeated generation has the declared byte size and candidate geometry. |
| Acceptance controls | The sparse, signature-dense acceptance, and multi-candidate scenarios return the exact expected PNG candidates and no additional candidates. |
| Refusal control | The all-malformed scenario returns no candidates. |
| Legacy parity | Every accepted PNG result still traverses the existing scan parity gate; no alternate public recovery path is introduced. |
| Regression integration | A dedicated verifier runs in `verify-all.sh` and is checked by the static release-documentation contract. |
| Honest documentation | Public status and any baseline record identify the corpus as synthetic local regression evidence, not real-device performance or release readiness. |

## Relationship to source access

This corpus does not change the source-access architecture. PNG discovery remains the only implemented bounded windowed discovery path; all filesystem metadata and non-PNG carving continue through their existing full-buffer compatibility paths. Any later second windowed-carver migration must receive its own method design, range parser, parity controls, cancellation tests, and source-access documentation.
