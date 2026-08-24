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
    docs/design-skill-adoption-v1.md \
    docs/linux-distribution-v1.md \
    docs/windows-distribution-v1.md \
    docs/local-release-evidence-v1.md \
    docs/case-brief-v1.md \
    docs/session-persistence-v1.md \
    docs/media-carving-v1.md \
    docs/source-access-architecture-v1.md \
    docs/future-github-launch-v1.md \
    docs/release-notes-v0.1.0-draft.md \
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
    scripts/package-windows-bundle.ps1 \
    scripts/verify-windows-bundle.ps1 \
    scripts/build-windows-installer.ps1 \
    scripts/verify-windows-config.sh \
    scripts/verify-windows-cross-target.sh \
    scripts/package-windows-cross-target.sh \
    scripts/verify-windows-cross-target-bundle.sh \
    scripts/generate-local-release-evidence.sh \
    scripts/verify-local-release-evidence.sh \
    installer/windows/evidenceforge.iss \
    .github/workflows/verify.yml \
    .github/workflows/windows-release.yml \
    .github/dependabot.yml; do
    test -s "$path"
done

grep -q 'Apache License' LICENSE
grep -q 'license = "Apache-2.0"' Cargo.toml
grep -q 'Apache License 2.0' README.md
grep -q '## Safety boundary' README.md
grep -q 'sh scripts/verify-all.sh' README.md
grep -q '## Distribution status' README.md
grep -q 'linux-distribution-v1.md' README.md
grep -q 'windows-distribution-v1.md' README.md
grep -q 'project-status.md' README.md
grep -q 'GUI workflow' README.md
grep -q 'Optional command-line workflows' README.md
grep -q 'audit-session' README.md
grep -q 'case-brief' README.md
grep -q 'project-status.md' README.md
grep -q 'privacy-first' CONTRIBUTING.md
grep -q 'private security-reporting channel' SECURITY.md
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
grep -q 'sh scripts/verify-large-sparse-control.sh' scripts/verify-all.sh
grep -q 'cargo audit' docs/release-process.md
grep -q 'local release-evidence contract' docs/release-process.md
grep -q 'protected `main`' docs/release-process.md
grep -q 'explicit authorization' docs/release-process.md
grep -q 'Release target record' docs/release-scorecard-v1.md
grep -q 'Authorization boundary' docs/release-scorecard-v1.md
grep -q 'strict 90+ public-release readiness score' docs/release-scorecard-v1.md
grep -q 'RUSTSEC-2026-0192' docs/dependency-advisories.md
grep -q 'zero known vulnerabilities' docs/dependency-advisories.md
grep -q 'permissions:' .github/workflows/verify.yml
grep -q 'contents: read' .github/workflows/verify.yml
grep -q 'cargo install cargo-audit --version 0.22.2 --locked' .github/workflows/verify.yml
grep -q 'libxkbcommon-x11-0' .github/workflows/verify.yml
grep -q 'sh scripts/verify-all.sh' .github/workflows/verify.yml
grep -q 'windows-2022' .github/workflows/windows-release.yml
grep -q 'cargo clippy --workspace --all-targets -- -D warnings' .github/workflows/windows-release.yml
grep -q 'contents: read' .github/workflows/windows-release.yml
grep -q 'package-windows-bundle.ps1' .github/workflows/windows-release.yml
grep -q 'build-windows-installer.ps1' .github/workflows/windows-release.yml
grep -q 'PrivilegesRequired=lowest' installer/windows/evidenceforge.iss
grep -q 'ArchitecturesAllowed=x64compatible' installer/windows/evidenceforge.iss
grep -q 'launch-evidenceforge.sh' scripts/package-linux-bundle.sh
grep -q 'install-desktop-launcher.sh' scripts/package-linux-bundle.sh
grep -q 'Start DiskTrace.cmd' scripts/package-windows-bundle.ps1
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
grep -q 'source-access architecture' README.md
grep -q 'bounded sliding window' docs/source-access-architecture-v1.md
grep -q 'source identity verification' docs/source-access-architecture-v1.md
grep -q 'contains no downloaded executable code' docs/design-skill-adoption-v1.md
grep -q 'defines a single `Palette`' docs/design-skill-adoption-v1.md
grep -q 'package-ecosystem: cargo' .github/dependabot.yml
grep -q 'cargo-patch-and-minor' .github/dependabot.yml
grep -q 'action-patch-and-minor' .github/dependabot.yml
grep -q 'dtolnay/rust-toolchain' .github/dependabot.yml
grep -q 'open-pull-requests-limit: 2' .github/dependabot.yml

test -x scripts/verify-all.sh
test -x scripts/package-linux-bundle.sh
test -x scripts/verify-linux-bundle.sh
test -x scripts/verify-desktop-ui.sh
test -x scripts/verify-export-audit.sh
test -x scripts/verify-case-brief.sh
test -x scripts/verify-media-recovery.sh
test -x scripts/generate-large-sparse-fixture.sh
test -x scripts/measure-large-sparse-scan.sh
test -x scripts/verify-large-sparse-control.sh
test -x scripts/verify-windows-config.sh
test -x scripts/verify-windows-cross-target.sh
test -x scripts/package-windows-cross-target.sh
test -x scripts/verify-windows-cross-target-bundle.sh
test -x scripts/generate-local-release-evidence.sh
test -x scripts/verify-local-release-evidence.sh
sh scripts/verify-windows-config.sh
cargo metadata --no-deps --format-version 1 >/dev/null

printf '%s\n' 'release documentation verification passed'
