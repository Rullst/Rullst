#!/usr/bin/env bash
# Package selected release crates twice and require byte-identical archives.
# The second run uses a fresh checkout of the same commit at another path,
# with new file mtimes, a restrictive umask and its own empty target
# directory. Only `.crate` archives are covered; native binaries are not.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

crates=("$@")
if [[ ${#crates[@]} -eq 0 ]]; then
  crates=(rullst-macros rullst-orm-macros)
fi
for crate in "${crates[@]}"; do
  if [[ ! "$crate" =~ ^[a-z][a-z0-9-]*$ ]] ||
    ! jq -e --arg crate "$crate" 'index($crate) != null' .github/release-order.json >/dev/null; then
    echo "Not an admitted release crate: $crate" >&2
    exit 2
  fi
done

cargo_bin="${CARGO:-cargo}"
work="$(mktemp -d)"
cleanup() {
  git -C "$repo_root" worktree remove --force "$work/checkout" >/dev/null 2>&1 || true
  rm -rf -- "$work"
}
trap cleanup EXIT

(umask 077 && git -C "$repo_root" worktree add --quiet --detach "$work/checkout" HEAD)

package_args=()
for crate in "${crates[@]}"; do package_args+=(--package "$crate"); done

package_from() {
  (cd "$1" && CARGO_TARGET_DIR="$2" "$cargo_bin" package "${package_args[@]}" \
    --all-features --locked --no-verify --quiet)
}

package_from "$repo_root" "$work/first"
(umask 077 && package_from "$work/checkout" "$work/second")

metadata="$("$cargo_bin" metadata --locked --no-deps --format-version 1)"
status=0
for crate in "${crates[@]}"; do
  version="$(jq -er --arg crate "$crate" \
    '.packages[] | select(.name == $crate) | .version' <<<"$metadata")"
  archive="${crate}-${version}.crate"
  first="$(sha256sum "$work/first/package/$archive" | cut -d ' ' -f 1)"
  second="$(sha256sum "$work/second/package/$archive" | cut -d ' ' -f 1)"
  if [[ "$first" == "$second" ]]; then
    printf 'reproducible  %s  %s\n' "$first" "$archive"
  else
    printf 'DIFFERENT     %s  %s (second: %s)\n' "$first" "$archive" "$second" >&2
    status=1
  fi
done
exit "$status"
