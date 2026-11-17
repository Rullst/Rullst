#!/usr/bin/env bash
# The dependency and bounded source audits of the release evidence bundle:
#
#   bash .github/release-evidence-audit.sh <evidence-directory>
#
# writes cargo-audit.json, security-evidence.md and sbom-cyclonedx-1.5.json
# there. release.yml runs it on the tag. The `Release evidence audit` job of
# supply-chain.yml runs it on pull requests that can change its result, so a
# governed exception missing here fails before a tag does (13.0.0-alpha.5
# failed on its tag because `cargo rullst audit` lacked them).
#
# Both commands take every active exception of
# docs/src/security-advisory-exceptions.md; the `Advisory exception
# governance` job of supply-chain.yml compares these flags with the ledger
# and deny.toml. Run from the repository root with cargo-audit installed.
set -euo pipefail

if [[ $# -ne 1 || -z "$1" ]]; then
  echo "usage: $0 <evidence-directory>" >&2
  exit 2
fi
evidence_dir="$1"
mkdir -p "$evidence_dir"

# Any advisory outside the ledger blocks the release. The JSON result is
# retained as tag-bound evidence.
cargo audit --json --ignore RUSTSEC-2026-0459 --ignore RUSTSEC-2026-0440 --ignore RUSTSEC-2026-0543 \
  > "$evidence_dir/cargo-audit.json"

# This command intentionally emits NOT CHECKED / NOT EVALUATED for controls
# that a bounded source audit cannot establish. It runs cargo-audit again, so
# it takes the same governed exceptions.
cargo run --locked -p cargo-rullst --bin rullst -- \
  audit --idor --compliance --sbom \
  --audit-ignore RUSTSEC-2026-0459 --audit-ignore RUSTSEC-2026-0440 --audit-ignore RUSTSEC-2026-0543
mv SECURITY_COMPLIANCE.md "$evidence_dir/security-evidence.md"
mv sbom-cyclonedx.json "$evidence_dir/sbom-cyclonedx-1.5.json"
