#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET=x86_64-pc-windows-gnu
cd "$ROOT"

if ! rustup target list --installed | grep -qx "$TARGET"; then
    printf '%s\n' "Windows cross-target verification requires the Rust target: $TARGET" >&2
    exit 1
fi

for command in x86_64-w64-mingw32-gcc wine xvfb-run timeout; do
    if ! command -v "$command" >/dev/null 2>&1; then
        printf '%s\n' "Windows cross-target verification requires: $command" >&2
        exit 1
    fi
done

cargo build --release --target "$TARGET" -p ef-desktop -p ef-cli

CLI="target/$TARGET/release/evidenceforge.exe"
DESKTOP="target/$TARGET/release/evidenceforge-desktop.exe"
test -f "$CLI"
test -f "$DESKTOP"

help_output=$(mktemp)
desktop_stdout=$(mktemp)
desktop_stderr=$(mktemp)
cleanup() {
    rm -f "$help_output" "$desktop_stdout" "$desktop_stderr"
}
trap cleanup EXIT

set +e
WINEDEBUG=-all wine "$CLI" --help > "$help_output" 2>&1
set -e
grep -q 'evidenceforge scan <image-path>' "$help_output"

set +e
timeout 8s xvfb-run -a -s '-screen 0 1440x920x24' \
    env WINEDEBUG=-all wine "$DESKTOP" > "$desktop_stdout" 2> "$desktop_stderr"
status=$?
set -e
if [ "$status" -ne 124 ]; then
    cat "$desktop_stdout" "$desktop_stderr" >&2
    exit "$status"
fi
if [ -s "$desktop_stderr" ]; then
    cat "$desktop_stderr" >&2
    exit 1
fi

printf '%s\n' 'Windows x86_64 cross-target compatibility smoke passed'
printf '%s\n' 'Boundary: this verifies Linux-host cross-build and Wine compatibility only; native Windows build, launcher, installer, signing, and usability evidence still require Windows.'
