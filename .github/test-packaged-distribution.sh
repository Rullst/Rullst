#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: test-packaged-distribution.sh VERSION [PACKAGE_DIR] [--supervision-candidate]}"
package_dir="${2:-target/package}"
cargo_bin="${CARGO:-cargo}"

package_dir="$(cd "$package_dir" && pwd -P)"
repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
mapfile -t crates < <(
  sed -nE 's/^[[:space:]]*"([^"]+)"[,]?$/\1/p' \
    "$repository_root/.github/release-order.json"
)

if [ "${#crates[@]}" -eq 0 ]; then
  echo "No release packages were found in .github/release-order.json."
  exit 1
fi

temp_base="$(cd "${TMPDIR:-/tmp}" && pwd -P)"
work_dir="$(mktemp -d "$temp_base/rullst-packaged-distribution.XXXXXX")"
work_dir="$(cd "$work_dir" && pwd -P)"

cleanup() {
  case "$work_dir" in
    "$temp_base"/rullst-packaged-distribution.*)
      rm -rf -- "$work_dir"
      ;;
    *)
      echo "Refusing to remove unexpected temporary path: $work_dir" >&2
      return 1
      ;;
  esac
}
trap cleanup EXIT

candidate=false
media_candidate=false
case "${3:-}" in
  "") ;;
  --supervision-candidate|--v13-candidates)
    if jq -e 'index("rullst-supervision") != null' "$repository_root/.github/release-order.json" > /dev/null; then
      echo "Remove candidate mode after supervision enters the release inventory." >&2
      exit 1
    fi
    candidate=true
    if [ "$3" = --v13-candidates ]; then media_candidate=true; fi
    ;;
  *) echo "Unknown packaged-distribution mode." >&2; exit 1 ;;
esac
if [ "$#" -gt 3 ]; then echo "Too many packaged-distribution arguments." >&2; exit 1; fi

packages_dir="$work_dir/packages"
consumer_dir="$work_dir/consumer"
install_root="$work_dir/install"
projects_dir="$work_dir/projects"
mkdir -p "$packages_dir" "$consumer_dir/src" "$install_root" "$projects_dir"
export CARGO_NET_OFFLINE=true
export RULLST_DISABLE_UPDATE_CHECK=true
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$work_dir/target}"

for crate in "${crates[@]}"; do
  archive="$package_dir/${crate}-${version}.crate"
  if [ ! -f "$archive" ]; then
    echo "Missing package archive: $archive"
    exit 1
  fi
  tar -xzf "$archive" -C "$packages_dir"
done

if [ "$candidate" = true ]; then
  candidate_archive="$package_dir/rullst-supervision-${version}.crate"
  # The caller must audit the complete archive set before extraction.
  tar -xzf "$candidate_archive" -C "$packages_dir"
  candidate_source="$packages_dir/rullst-supervision-${version}"
  python3 - "$candidate_source/Cargo.toml" "$version" <<'PYVERIFY'
import sys, tomllib
from pathlib import Path
package = tomllib.loads(Path(sys.argv[1]).read_text())["package"]
assert package["name"] == "rullst-supervision"
assert package["version"] == sys.argv[2]
assert package["publish"] is False, "candidate rehearsal must remain unpublished"
PYVERIFY
  "$cargo_bin" test --manifest-path "$candidate_source/Cargo.toml" --offline --locked --all-features
fi

if [ "$media_candidate" = true ]; then
  bash "$repository_root/.github/test-media-package.sh" "$version" "$package_dir"
fi

toml_path() {
  local path="$1"
  if command -v cygpath >/dev/null 2>&1; then
    path="$(cygpath -m "$path")"
  fi
  path="${path//\\/\\\\}"
  printf '%s' "$path"
}

append_package_patches() {
  local manifest="$1"
  printf '\n[patch.crates-io]\n' >> "$manifest"
  for crate in "${crates[@]}"; do
    printf '"%s" = { path = "%s" }\n' \
      "$crate" \
      "$(toml_path "$packages_dir/${crate}-${version}")" \
      >> "$manifest"
  done
}

umbrella_manifest="$packages_dir/rullst-${version}/Cargo.toml"
umbrella_features="$(
  python3 - "$umbrella_manifest" <<'PY'
import json
import sys
import tomllib
from pathlib import Path

manifest = tomllib.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
features = sorted(name for name in manifest.get("features", {}) if name != "default")
if not features:
    raise SystemExit("packaged rullst manifest has no public features")
print(", ".join(json.dumps(name) for name in features))
PY
)"

{
  printf '[package]\nname = "rullst-packaged-consumer"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[dependencies]\n'
  for crate in "${crates[@]}"; do
    if [ "$crate" = "rullst" ]; then
      printf '"rullst" = { version = "=%s", default-features = false, features = [%s] }\n' \
        "$version" "$umbrella_features"
    else
      printf '"%s" = "=%s"\n' "$crate" "$version"
    fi
  done
} > "$consumer_dir/Cargo.toml"
printf 'fn main() {}\n' > "$consumer_dir/src/main.rs"
append_package_patches "$consumer_dir/Cargo.toml"

"$cargo_bin" check \
  --manifest-path "$consumer_dir/Cargo.toml" \
  --offline \
  --all-targets

# Run only the optional privacy composition here. The full-feature consumer
# above retains all-target compilation without linking every native adapter
# again merely to execute a SQLite facade contract.
privacy_dir="$work_dir/privacy-consumer"
mkdir -p "$privacy_dir/tests"
{
  printf '[package]\nname = "rullst-packaged-privacy"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["privacy-challenge-tokens", "privacy-sqlite", "privacy-consent-sqlite"] }\n' "$version"
  printf '\n[dev-dependencies]\ntempfile = "3"\n'
} > "$privacy_dir/Cargo.toml"
cp "$repository_root/.github/fixtures/privacy-facade.rs" "$privacy_dir/tests/privacy_facade.rs"
append_package_patches "$privacy_dir/Cargo.toml"
"$cargo_bin" test --manifest-path "$privacy_dir/Cargo.toml" --offline \
  --test privacy_facade

cli_package="$packages_dir/cargo-rullst-${version}"
if [ ! -f "$cli_package/Cargo.lock" ]; then
  echo "The packaged cargo-rullst archive must include Cargo.lock."
  exit 1
fi
# The CLI depends on packages from the same release train. Point the extracted
# manifest at the extracted archives so a first publication can be verified
# offline before those package names exist in the registry.
append_package_patches "$cli_package/Cargo.toml"

"$cargo_bin" install \
  --path "$cli_package" \
  --root "$install_root" \
  --offline \
  --locked \
  --force

rullst_bin="$install_root/bin/rullst"
if [ -f "${rullst_bin}.exe" ]; then
  rullst_bin="${rullst_bin}.exe"
fi
if [ ! -x "$rullst_bin" ]; then
  echo "The packaged CLI did not install the rullst binary."
  exit 1
fi
"$rullst_bin" --version

blueprints=(blank lms saas blog portfolio erp)
for blueprint in "${blueprints[@]}"; do
  app_name="packaged-${blueprint}"
  (
    cd "$projects_dir"
    "$rullst_bin" new "$app_name" \
      --default \
      --blueprint "$blueprint" \
      --skip-initial-migration
  )

  manifest="$projects_dir/$app_name/Cargo.toml"
  if grep -Eq '(^|[[:space:]])path[[:space:]]*=' "$manifest"; then
    echo "Generated $blueprint manifest unexpectedly references a source path."
    exit 1
  fi
  if ! grep -Fq "rullst = { version = \"$version\"" "$manifest"; then
    echo "Generated $blueprint manifest does not use packaged version $version."
    exit 1
  fi

  if [[ "$blueprint" == saas || "$blueprint" == lms ]]; then
    (
      cd "$projects_dir/$app_name"
      # Require registry-only output from the installed CLI; the archive patch
      # below is the sole source substitution in this unpublished rehearsal.
      tenant_args=()
      if [[ "$blueprint" == saas ]]; then tenant_args=(--tenant-ref archive-tenant); fi
      "$rullst_bin" make:age-gate --blueprint "$blueprint" "${tenant_args[@]}" \
        --minimum-age 18 --policy-version archive-v1 --replay-store sqlite
      "$rullst_bin" make:privacy --blueprint "$blueprint" "${tenant_args[@]}" \
        --purpose-version archive-v1 --validity-seconds 3600
      if [[ "$blueprint" == lms && "$candidate" == true ]]; then
        "$rullst_bin" make:supervision --supervision-source "$candidate_source" --policy-version archive-v1 --notice-version archive-v1 --retention-seconds 3600 --session-seconds 600
      fi
      "$rullst_bin" generate:ai-context --check
    )
    python3 - "$manifest" "$version" <<'PY'
import sys, tomllib
from pathlib import Path
manifest = tomllib.loads(Path(sys.argv[1]).read_text())
dependency = manifest['dependencies']['rullst-privacy']
assert set(dependency) == {'version', 'default-features', 'features'}, dependency
assert dependency['version'] == '=' + sys.argv[2]
assert dependency['default-features'] is False
assert set(dependency['features']) == {'challenge-tokens', 'sqlite', 'consent-sqlite'}
PY
  fi

  append_package_patches "$manifest"
  "$cargo_bin" check \
    --manifest-path "$manifest" \
    --offline \
    --all-targets
done

printf 'verified packaged consumer and %s installed-CLI blueprints for Rullst %s\n' \
  "${#blueprints[@]}" \
  "$version"
