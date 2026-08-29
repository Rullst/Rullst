#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: test-packaged-distribution.sh VERSION [PACKAGE_DIR]}"
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

packages_dir="$work_dir/packages"
consumer_dir="$work_dir/consumer"
install_root="$work_dir/install"
projects_dir="$work_dir/projects"
mkdir -p "$packages_dir" "$consumer_dir/src" "$install_root" "$projects_dir"

for crate in "${crates[@]}"; do
  archive="$package_dir/${crate}-${version}.crate"
  if [ ! -f "$archive" ]; then
    echo "Missing package archive: $archive"
    exit 1
  fi
  tar -xzf "$archive" -C "$packages_dir"
done

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

{
  printf '[package]\nname = "rullst-packaged-consumer"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[dependencies]\n'
  for crate in "${crates[@]}"; do
    if [ "$crate" = "rullst" ]; then
      printf '"rullst" = { version = "=%s", features = ["orm", "queue-sqlite", "nexus", "studio", "auth", "mailer", "redis", "oauth", "ai", "capital", "security", "iot", "telemetry"] }\n' "$version"
    else
      printf '"%s" = "=%s"\n' "$crate" "$version"
    fi
  done
} > "$consumer_dir/Cargo.toml"
printf 'fn main() {}\n' > "$consumer_dir/src/main.rs"
append_package_patches "$consumer_dir/Cargo.toml"

export CARGO_NET_OFFLINE=true
export RULLST_DISABLE_UPDATE_CHECK=true
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$work_dir/target}"

"$cargo_bin" check \
  --manifest-path "$consumer_dir/Cargo.toml" \
  --offline \
  --all-targets

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

  append_package_patches "$manifest"
  "$cargo_bin" check \
    --manifest-path "$manifest" \
    --offline \
    --all-targets
done

printf 'verified packaged consumer and %s installed-CLI blueprints for Rullst %s\n' \
  "${#blueprints[@]}" \
  "$version"
