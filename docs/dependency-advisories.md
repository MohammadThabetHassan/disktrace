# Dependency advisory register

DiskTrace treats a known dependency vulnerability as a release blocker. This register records reviewed non-vulnerability advisories that remain after the local RustSec scan, so they are visible to maintainers and are not mistaken for a clean advisory state.

## Current reviewed advisory

| Advisory | Locked package | Classification | Dependency path | Decision |
|---|---:|---|---|---|
| [RUSTSEC-2026-0192][1] | `ttf-parser` 0.25.1 | Unmaintained; no patched version is published | `ef-desktop` → `eframe` → `egui` → `epaint` → `ab_glyph` → `owned_ttf_parser` → `ttf-parser` | Retain temporarily as a transitive GUI text-rendering dependency and reassess on every eframe upgrade. |

The RustSec advisory states that `ttf-parser` is unmaintained and lists `skrifa` as an alternative, but does not report a vulnerability or provide a patched version.[1] DiskTrace does not call this dependency directly; it is brought in by the selected eframe 0.33.3 GUI stack. Replacing it independently would require an upstream GUI-framework change and would not be a safe local lockfile substitution.

The local preflight on 2026-08-26 for verified pre-release source `8a3eb6ca143103a69ba8a1d9773140256d6d1cc6` found **zero known vulnerabilities** and this one allowed maintenance warning. The project uses an explicit OpenGL eframe configuration rather than the unused WGPU renderer, which removes a separate unmaintained `paste` dependency that was present only through that renderer’s macOS backend.

## Release handling

This advisory does not authorize a misleading clean-security claim. A maintainer must rerun `cargo audit`, update this register if the dependency path or advisory changes, and block release if the advisory is reclassified as a vulnerability. A compatible eframe upgrade or upstream change that removes the dependency should be preferred over a local fork. If a public release carries this warning, the release record must link to this document and state that the scan passed with the reviewed non-vulnerability advisory.

## References

[1]: https://rustsec.org/advisories/RUSTSEC-2026-0192.html "RUSTSEC-2026-0192: ttf-parser is unmaintained"
