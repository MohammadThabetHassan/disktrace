#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

for path in \
    README.md \
    LICENSE \
    CONTRIBUTING.md \
    SECURITY.md \
    CODE_OF_CONDUCT.md \
    CHANGELOG.md \
    docs/architecture.md \
    docs/safety-and-evidence.md \
    docs/release-process.md \
    docs/dependency-advisories.md \
    docs/security-scanning-v1.md \
    docs/sbom-provenance-v1.md \
    docs/macos-validation-v1.md \
    docs/design-skill-adoption-v1.md \
    docs/linux-distribution-v1.md \
    docs/windows-distribution-v1.md \
    docs/local-release-evidence-v1.md \
    docs/case-brief-v1.md \
    docs/session-persistence-v1.md \
    docs/media-carving-v1.md \
    docs/avi-mp4-resilience-corpus-v1.md \
    docs/source-access-architecture-v1.md \
    docs/source-window-discovery-v1.md \
    docs/jpeg-window-discovery-v1.md \
    docs/gif-window-discovery-v1.md \
    docs/pdf-window-discovery-v1.md \
    docs/zip-window-discovery-v1.md \
    docs/legacy-discovery-cancellation-v1.md \
    docs/performance-control-corpus-v1.md \
    docs/fat32-feasibility-v1.md \
    docs/maintainer-runbook-v1.md \
    docs/desktop-acceptance-v1.md \
    docs/future-github-launch-v1.md \
    docs/release-scorecard-v1.md \
    docs/release-candidate-v0.1.0.md \
    docs/release-candidate-acceptance-kit-v1.md \
    docs/release-decision-v1.md \
    docs/project-status.md \
    docs/release-scorecard-v1.md \
    docs/gui-workflow-v1.md \
    .gitattributes \
    rust-toolchain.toml \
    scripts/package-linux-bundle.sh \
    scripts/verify-linux-bundle.sh \
    scripts/verify-desktop-ui.sh \
    scripts/verify-export-audit.sh \
    scripts/verify-case-brief.sh \
    scripts/generate-media-fixture.py \
    scripts/verify-media-recovery.sh \
    scripts/generate-large-sparse-fixture.sh \
    scripts/measure-large-sparse-scan.sh \
    scripts/verify-large-sparse-control.sh \
    scripts/verify-windowed-png-discovery.sh \
    scripts/verify-windowed-jpeg-discovery.sh \
    scripts/verify-windowed-gif-discovery.sh \
    scripts/generate-scan-control-fixture.sh \
    scripts/verify-scan-control-corpus.sh \
    scripts/measure-scan-control-corpus.sh \
    scripts/summarize-scan-control-corpus.py \
    scripts/generate-sbom.sh \
    scripts/verify-sbom.sh \
    scripts/package-windows-bundle.ps1 \
    scripts/verify-windows-bundle.ps1 \
    scripts/build-windows-installer.ps1 \
    scripts/verify-windows-installer.ps1 \
    scripts/verify-windows-config.sh \
    scripts/verify-windows-cross-target.sh \
    scripts/package-windows-cross-target.sh \
    scripts/verify-windows-cross-target-bundle.sh \
    scripts/generate-local-release-evidence.sh \
    scripts/verify-local-release-evidence.sh \
    installer/windows/evidenceforge.iss \
    .github/workflows/verify.yml \
    .github/workflows/windows-release.yml \
    .github/workflows/codeql.yml \
    .github/workflows/macos-verify.yml \
    .github/dependabot.yml \
    .github/CODEOWNERS \
    .github/ISSUE_TEMPLATE/bug-report.yml \
    .github/ISSUE_TEMPLATE/recovery-method-proposal.yml \
    .github/ISSUE_TEMPLATE/config.yml; do
    test -s "$path"
done

grep -q 'Apache License' LICENSE
grep -q 'license = "Apache-2.0"' Cargo.toml
grep -q 'Apache License 2.0' README.md
test -s docs/assets/disktrace-logo.png
grep -q 'docs/assets/disktrace-logo.png' README.md
grep -q '## Safety boundary' README.md
grep -q 'sh scripts/verify-all.sh' README.md
grep -q '## Distribution status' README.md
grep -q 'linux-distribution-v1.md' README.md
grep -q 'windows-distribution-v1.md' README.md
grep -q 'project-status.md' README.md
grep -q 'release-candidate-v0.1.0.md' README.md
grep -q 'release-candidate-acceptance-kit-v1.md' README.md
grep -q 'GUI workflow' README.md
grep -q 'synthetic performance-control corpus' README.md
grep -q 'macOS validation contract' README.md
grep -q 'JPEG windowed-discovery contract' README.md
grep -q 'GIF windowed-discovery contract' README.md
grep -q 'PDF windowed-discovery contract' README.md
grep -q 'ZIP/Open XML windowed-discovery contract' README.md
grep -q 'bounded PNG/JPEG/GIF/PDF/ZIP/Open XML discovery parity controls' README.md
grep -q 'Optional command-line workflows' README.md
grep -q 'audit-session' README.md
grep -q 'case-brief' README.md
grep -q 'project-status.md' README.md
grep -q 'privacy-first' CONTRIBUTING.md
grep -q 'private vulnerability-reporting channel' SECURITY.md
grep -q 'github.com/MohammadThabetHassan/disktrace/security/advisories/new' SECURITY.md
grep -q 'Do \*\*not\*\* post sensitive findings' SECURITY.md
grep -q 'rustfmt' rust-toolchain.toml
grep -q 'clippy' rust-toolchain.toml
grep -q 'fixtures/\*\*/expected-\*.txt text eol=lf' .gitattributes
grep -q '\*.img binary' .gitattributes
grep -q 'cargo clippy --workspace --all-targets -- -D warnings' scripts/verify-all.sh
grep -q "RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps" scripts/verify-all.sh
grep -q 'cargo audit' scripts/verify-all.sh
grep -q 'sh scripts/verify-desktop-ui.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-export-audit.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-case-brief.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-media-recovery.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-media-resilience-corpus.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-large-sparse-control.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-windowed-png-discovery.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-windowed-jpeg-discovery.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-windowed-gif-discovery.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-windowed-pdf-discovery.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-windowed-zip-discovery.sh' scripts/verify-all.sh
grep -q 'sh scripts/verify-scan-control-corpus.sh' scripts/verify-all.sh
grep -q 'cargo audit' docs/release-process.md
grep -q 'local release-evidence contract' docs/release-process.md
grep -q 'release-candidate-v0.1.0.md' docs/release-process.md
grep -q 'manual-acceptance kit' docs/release-process.md
grep -q 'protected `main`' docs/release-process.md
grep -q 'explicit authorization' docs/release-process.md
grep -q 'Decision: no public-release action' docs/release-decision-v1.md
grep -q 'not authorize a semantic version' docs/release-decision-v1.md
grep -q 'does not claim universal recovery' docs/release-decision-v1.md
grep -q '8a3eb6ca143103a69ba8a1d9773140256d6d1cc6' docs/release-decision-v1.md
grep -q 'PNG, JPEG, GIF, PDF, and ZIP/Open XML discovery' docs/release-decision-v1.md
grep -q 'owner separately authorizes' docs/release-decision-v1.md
grep -q 'Release target record' docs/release-scorecard-v1.md
grep -q '8a3eb6ca143103a69ba8a1d9773140256d6d1cc6' docs/release-scorecard-v1.md
grep -q 'Authorization boundary' docs/release-scorecard-v1.md
grep -q 'Status: preparation only' docs/release-candidate-v0.1.0.md
grep -q 'Candidate source target' docs/release-candidate-v0.1.0.md
grep -q 'Publication authorization' docs/release-candidate-v0.1.0.md
grep -q 'manual-acceptance kit' docs/release-candidate-v0.1.0.md
grep -q 'No action is implied' docs/release-candidate-v0.1.0.md
grep -q 'Preparation-only operator aid' docs/release-candidate-acceptance-kit-v1.md
grep -q 'Windows x86_64' docs/release-candidate-acceptance-kit-v1.md
grep -q 'Automated companions and the kit are not manual acceptance' docs/release-candidate-acceptance-kit-v1.md
grep -q 'strict 90+ public-release readiness score' docs/release-scorecard-v1.md
grep -q 'RUSTSEC-2026-0192' docs/dependency-advisories.md
grep -q 'zero known vulnerabilities' docs/dependency-advisories.md
grep -q '2026-08-26' docs/dependency-advisories.md
grep -q 'does not prove the absence of all vulnerabilities' docs/security-scanning-v1.md
grep -q 'security-events: write' docs/security-scanning-v1.md
grep -q 'build-free mode' docs/security-scanning-v1.md
grep -q 'does not create an attestation' docs/sbom-provenance-v1.md
grep -q 'cargo-cyclonedx 0.5.9' docs/sbom-provenance-v1.md
grep -q 'unsigned ARM64 desktop binary' docs/macos-validation-v1.md
grep -q 'universal macOS compatibility' docs/macos-validation-v1.md
grep -q 'permissions:' .github/workflows/verify.yml
grep -q 'contents: read' .github/workflows/verify.yml
grep -q 'cargo install cargo-audit --version 0.22.2 --locked' .github/workflows/verify.yml
grep -q 'libxkbcommon-x11-0' .github/workflows/verify.yml
grep -q 'sh scripts/verify-all.sh' .github/workflows/verify.yml
grep -q 'security-events: write' .github/workflows/codeql.yml
grep -q 'languages: rust' .github/workflows/codeql.yml
grep -q 'build-mode: none' .github/workflows/codeql.yml
grep -q 'security-extended' .github/workflows/codeql.yml
grep -q 'runs-on: macos-14' .github/workflows/macos-verify.yml
grep -q 'contents: read' .github/workflows/macos-verify.yml
grep -q 'evidenceforge-desktop' .github/workflows/macos-verify.yml
grep -q 'unsigned-review-binary' .github/workflows/macos-verify.yml
grep -q 'windows-2022' .github/workflows/windows-release.yml
grep -q 'cargo clippy --workspace --all-targets -- -D warnings' .github/workflows/windows-release.yml
grep -q 'contents: read' .github/workflows/windows-release.yml
grep -q 'package-windows-bundle.ps1' .github/workflows/windows-release.yml
grep -q 'build-windows-installer.ps1' .github/workflows/windows-release.yml
grep -q 'verify-windows-installer.ps1' .github/workflows/windows-release.yml
grep -q 'cargo install cargo-cyclonedx --version 0.5.9 --locked' .github/workflows/windows-release.yml
grep -q 'generate-sbom.sh dist/sbom' .github/workflows/windows-release.yml
grep -q 'Verify synthetic scan-control corpus' .github/workflows/windows-release.yml
grep -q 'verify-scan-control-corpus.sh' .github/workflows/windows-release.yml
grep -q 'Windows installer acceptance verification must run on a Windows host' scripts/verify-windows-installer.ps1
grep -q 'Native installer acceptance gate' docs/windows-distribution-v1.md
grep -q 'launch the GUI' docs/windows-distribution-v1.md
grep -q 'PrivilegesRequired=lowest' installer/windows/evidenceforge.iss
grep -q 'ArchitecturesAllowed=x64compatible' installer/windows/evidenceforge.iss
grep -q 'launch-disktrace.sh' scripts/package-linux-bundle.sh
grep -q 'install-disktrace-launcher.sh' scripts/package-linux-bundle.sh
grep -q 'clean committed source revision' scripts/package-linux-bundle.sh
grep -q 'source_state": "clean-committed"' scripts/package-linux-bundle.sh
grep -q 'release-candidate-acceptance-kit-v1.md' scripts/package-linux-bundle.sh
grep -q 'Start DiskTrace.cmd' scripts/package-windows-bundle.ps1
grep -q "source_state = 'clean-committed'" scripts/package-windows-bundle.ps1
grep -q 'release-candidate-acceptance-kit-v1.md' scripts/package-windows-bundle.ps1
grep -q 'Start a local recovery session' crates/ef-desktop/src/main.rs
grep -q 'Recovery workspace' crates/ef-desktop/src/main.rs
grep -q 'Recovery workflow' crates/ef-desktop/src/main.rs
grep -q 'quiet casework hierarchy' docs/gui-workflow-v1.md
grep -q 'Recheck source' docs/gui-workflow-v1.md
grep -q 'Audit exports' docs/gui-workflow-v1.md
grep -q 'Save case brief' docs/gui-workflow-v1.md
grep -q 'case-brief requires' crates/ef-cli/src/main.rs
grep -q 'owner has authorized an initial' docs/future-github-launch-v1.md
grep -q 'Draft for a future authorized release' docs/release-notes-v0.1.0-draft.md
grep -q 'PNG, JPEG, GIF, PDF, and ZIP/Open XML discovery' docs/release-notes-v0.1.0-draft.md
grep -q 'manual-acceptance kit' docs/release-notes-v0.1.0-draft.md
grep -q 'cross-target compatibility smoke' docs/windows-distribution-v1.md
grep -q 'Linux-host portable cross-target ZIP' docs/windows-distribution-v1.md
grep -q 'Local release-evidence contract' docs/local-release-evidence-v1.md
grep -q 'evidence cards with method and validation badges' docs/gui-workflow-v1.md
grep -q 'bounded adaptive widths' docs/gui-workflow-v1.md
grep -q 'Keyboard commands' docs/gui-workflow-v1.md
grep -q 'self-contained MP4/MOV' docs/gui-workflow-v1.md
grep -q 'quick or full format' docs/gui-workflow-v1.md
grep -q 'Supported recovery methods' README.md
grep -q 'rejects unrecognized' docs/session-persistence-v1.md
grep -q 'self-contained MP4/MOV' docs/media-carving-v1.md
grep -q 'verify-media-recovery.sh' docs/media-carving-v1.md
grep -q 'AVI and MP4/MOV resilience corpus' docs/media-carving-v1.md
grep -q 'not a new recovery method' docs/avi-mp4-resilience-corpus-v1.md
grep -q 'without a panic' docs/avi-mp4-resilience-corpus-v1.md
grep -q 'source-access architecture' README.md
grep -q 'bounded sliding window' docs/source-access-architecture-v1.md
grep -q 'source identity verification' docs/source-access-architecture-v1.md
grep -q 'same bounded-discovery stage now includes JPEG' docs/source-access-architecture-v1.md
grep -q 'same bounded-discovery stage now includes GIF' docs/source-access-architecture-v1.md
grep -q 'same bounded-discovery stage now includes PDF' docs/source-access-architecture-v1.md
grep -q 'same bounded-discovery stage now includes ZIP/Open XML' docs/source-access-architecture-v1.md
grep -q 'Primary window length' docs/source-window-discovery-v1.md
grep -q 'legacy `carve_pngs`' docs/source-window-discovery-v1.md
grep -q 'Explicit non-claims' docs/source-window-discovery-v1.md
grep -q 'one byte after each non-final primary range' docs/jpeg-window-discovery-v1.md
grep -q '128 MiB' docs/jpeg-window-discovery-v1.md
grep -q 'full-streaming or whole-scan' docs/jpeg-window-discovery-v1.md
grep -q 'five bytes after each primary window' docs/gif-window-discovery-v1.md
grep -q '64 MiB' docs/gif-window-discovery-v1.md
grep -q 'legacy `ef-carve::carve_gifs`' docs/gif-window-discovery-v1.md
grep -q 'four bytes after each primary window' docs/pdf-window-discovery-v1.md
grep -q '64 MiB' docs/pdf-window-discovery-v1.md
grep -q 'legacy `ef-carve::carve_pdfs`' docs/pdf-window-discovery-v1.md
grep -q 'three bytes after each primary window' docs/zip-window-discovery-v1.md
grep -q '64 MiB' docs/zip-window-discovery-v1.md
grep -q 'legacy `ef-carve::carve_zip_archives`' docs/zip-window-discovery-v1.md
grep -q 'after every completed legacy discovery method stage' docs/source-access-architecture-v1.md
grep -q 'not full parser-level cancellation' docs/source-access-architecture-v1.md
grep -q 'parser-loop cancellation' docs/legacy-discovery-cancellation-v1.md
grep -q 'itemized progress percentage' docs/legacy-discovery-cancellation-v1.md
grep -q 'synthetic byte controls' docs/performance-control-corpus-v1.md
grep -q 'not disk acquisitions' docs/performance-control-corpus-v1.md
grep -q 'signature-dense-refusal-v1' docs/performance-control-corpus-v1.md
grep -q 'hardware benchmarks' docs/performance-control-corpus-v1.md
grep -q 'FAT32 deleted-file recovery claim' docs/fat32-feasibility-v1.md
grep -q 'root directory in a cluster chain' docs/fat32-feasibility-v1.md
grep -q 'exact commit SHA' docs/maintainer-runbook-v1.md
grep -q 'Never silently replace a published artifact' docs/maintainer-runbook-v1.md
grep -q 'No completed manual record is asserted' docs/desktop-acceptance-v1.md
grep -q 'Automated companion evidence' docs/desktop-acceptance-v1.md
grep -q 'not a substitute for the reviewer' docs/desktop-acceptance-v1.md
grep -q 'accessibility certification' docs/desktop-acceptance-v1.md
grep -q 'real disk images' .github/ISSUE_TEMPLATE/bug-report.yml
grep -q 'synthetic, minimized controls' .github/ISSUE_TEMPLATE/recovery-method-proposal.yml
grep -q 'blank_issues_enabled: false' .github/ISSUE_TEMPLATE/config.yml
grep -q 'CodeQL Rust analysis' docs/project-status.md
grep -q 'macOS 14 ARM64 workspace validation' docs/project-status.md
grep -q 'SBOM transparency' docs/project-status.md
grep -q 'not an attestation' docs/project-status.md
grep -q 'controlled decision package' docs/project-status.md
grep -q 'cooperative stage checkpoints' docs/project-status.md
grep -q 'PNG, JPEG, GIF, PDF, and ZIP/Open XML discovery' docs/project-status.md
grep -q 'GitHub private vulnerability reporting is enabled' docs/project-status.md
grep -q 'contains no downloaded executable code' docs/design-skill-adoption-v1.md
grep -q 'defines a single `Palette`' docs/design-skill-adoption-v1.md
grep -q 'package-ecosystem: cargo' .github/dependabot.yml
grep -q 'cargo-patch-and-minor' .github/dependabot.yml
grep -q 'action-patch-and-minor' .github/dependabot.yml
grep -q '@MohammadThabetHassan' .github/CODEOWNERS
grep -q 'dtolnay/rust-toolchain' .github/dependabot.yml
grep -q 'open-pull-requests-limit: 2' .github/dependabot.yml

test -x scripts/verify-all.sh
test -x scripts/package-linux-bundle.sh
test -x scripts/verify-linux-bundle.sh
test -x scripts/verify-desktop-ui.sh
test -x scripts/verify-export-audit.sh
test -x scripts/verify-case-brief.sh
test -x scripts/verify-media-recovery.sh
test -x scripts/verify-media-resilience-corpus.sh
test -x scripts/generate-large-sparse-fixture.sh
test -x scripts/measure-large-sparse-scan.sh
test -x scripts/verify-large-sparse-control.sh
test -x scripts/verify-windowed-png-discovery.sh
test -x scripts/verify-windowed-jpeg-discovery.sh
test -x scripts/verify-windowed-pdf-discovery.sh
test -x scripts/verify-windowed-zip-discovery.sh
test -x scripts/generate-scan-control-fixture.sh
test -x scripts/verify-scan-control-corpus.sh
test -x scripts/measure-scan-control-corpus.sh
test -x scripts/summarize-scan-control-corpus.py
test -x scripts/generate-sbom.sh
test -x scripts/verify-sbom.sh
test -x scripts/verify-windows-config.sh
test -x scripts/verify-windows-cross-target.sh
test -x scripts/package-windows-cross-target.sh
test -x scripts/verify-windows-cross-target-bundle.sh
test -x scripts/generate-local-release-evidence.sh
test -x scripts/verify-local-release-evidence.sh
sh scripts/verify-windows-config.sh
cargo metadata --no-deps --format-version 1 >/dev/null

printf '%s\n' 'release documentation verification passed'
