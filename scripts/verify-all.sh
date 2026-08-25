#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

sh scripts/verify-release-docs.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
cargo audit
cargo test --workspace
sh scripts/verify-desktop-ui.sh

sh scripts/verify-foundation.sh
sh scripts/verify-fat12-recovery.sh
sh scripts/verify-fat16-jpeg-recovery.sh
sh scripts/verify-session-persistence.sh
sh scripts/verify-export-audit.sh
sh scripts/verify-case-brief.sh
sh scripts/verify-document-carving.sh
sh scripts/verify-media-recovery.sh
sh scripts/verify-exfat-recovery.sh
sh scripts/verify-ntfs-resident-recovery.sh
sh scripts/verify-ntfs-contiguous-recovery.sh
sh scripts/verify-windowed-png-discovery.sh
sh scripts/verify-large-sparse-control.sh
sh scripts/verify-scan-control-corpus.sh

cargo build --workspace

if command -v xvfb-run >/dev/null 2>&1; then
    set +e
    timeout 5s xvfb-run -a cargo run -q -p ef-desktop \
        > /tmp/evidenceforge-desktop-smoke.stdout \
        2> /tmp/evidenceforge-desktop-smoke.stderr
    status=$?
    set -e
    case "$status" in
        124)
            printf '%s\n' 'desktop smoke launch passed'
            ;;
        0)
            printf '%s\n' 'desktop smoke launch exited cleanly'
            ;;
        *)
            printf '%s\n' "desktop smoke launch failed with exit status $status" >&2
            cat /tmp/evidenceforge-desktop-smoke.stdout >&2
            cat /tmp/evidenceforge-desktop-smoke.stderr >&2
            exit "$status"
            ;;
    esac
else
    printf '%s\n' 'desktop smoke launch skipped: xvfb-run is unavailable'
fi

printf '%s\n' 'EvidenceForge full verification passed'
