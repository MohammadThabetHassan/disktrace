#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

for path in \
    scripts/generate-sbom.sh \
    docs/sbom-provenance-v1.md; do
    test -s "$path"
done

test -x scripts/generate-sbom.sh
sh -n scripts/generate-sbom.sh
grep -q 'clean tracked source revision' scripts/generate-sbom.sh
grep -q 'cargo-cyclonedx 0.5.9' scripts/generate-sbom.sh
grep -q -- '--spec-version 1.5' scripts/generate-sbom.sh
grep -q -- '--target all' scripts/generate-sbom.sh
grep -q 'SOURCE_DATE_EPOCH' scripts/generate-sbom.sh
grep -q 'SHA256SUMS' scripts/generate-sbom.sh
grep -q 'sbom-provenance.json' scripts/generate-sbom.sh
grep -q 'review artifact' docs/sbom-provenance-v1.md
grep -q 'does not create an attestation' docs/sbom-provenance-v1.md
grep -q 'cargo-cyclonedx 0.5.9' docs/sbom-provenance-v1.md

printf '%s\n' 'SBOM provenance configuration verification passed'
