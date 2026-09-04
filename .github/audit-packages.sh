#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: audit-packages.sh VERSION [PACKAGE_DIR]}"
package_dir="${2:-target/package}"
max_archive_bytes=$((10 * 1024 * 1024))

mapfile -t crates < <(jq -r '.[]' .github/release-order.json)
mapfile -t archives < <(
  find "$package_dir" -maxdepth 1 -type f -name '*.crate' -printf '%f\n' |
    sort
)

if [ "${#archives[@]}" -ne "${#crates[@]}" ]; then
  echo "Expected ${#crates[@]} package archives, found ${#archives[@]}."
  printf 'Found: %s\n' "${archives[@]}"
  exit 1
fi

for crate in "${crates[@]}"; do
  archive="$package_dir/${crate}-${version}.crate"
  root="${crate}-${version}/"

  if [ ! -f "$archive" ]; then
    echo "Missing package archive: $archive"
    exit 1
  fi

  archive_bytes="$(stat --format='%s' "$archive")"
  if [ "$archive_bytes" -le 0 ] || [ "$archive_bytes" -gt "$max_archive_bytes" ]; then
    echo "Package archive has an invalid or unexpected size: $archive ($archive_bytes bytes)"
    exit 1
  fi

  mapfile -t entries < <(tar -tzf "$archive")
  has_cargo=false
  has_license=false
  has_readme=false
  has_source=false

  for entry in "${entries[@]}"; do
    if [[ "$entry" != "$root"* || "$entry" == /* || "$entry" == *"/../"* ]]; then
      echo "Unsafe path in $archive: $entry"
      exit 1
    fi

    relative="${entry#"$root"}"
    lower="${relative,,}"
    case "$lower" in
      .env|.env.*|*/.env|*/.env.*|.git|.git/*|*/.git|*/.git/*|credentials|credentials.*|*/credentials|*/credentials.*|secrets|secrets.*|*/secrets|*/secrets.*|id_rsa|*/id_rsa|id_ed25519|*/id_ed25519|*.pem|*.p12|*.pfx|*.key)
        echo "Potential secret material in $archive: $relative"
        exit 1
        ;;
      *.db|*.db-*|*.sqlite|*.sqlite-*|*.sqlite3|*.sqlite3-*|*.wal|*.shm|memdb_*|*/memdb_*|*_test_db|*_test_db_*)
        echo "Runtime database state must not be packaged: $relative"
        exit 1
        ;;
    esac

    case "$relative" in
      Cargo.toml) has_cargo=true ;;
      LICENSE) has_license=true ;;
      README|README.*) has_readme=true ;;
      src/*) has_source=true ;;
    esac
  done

  if [ "$has_cargo" != true ] || [ "$has_license" != true ] || [ "$has_readme" != true ] || [ "$has_source" != true ]; then
    echo "Required Cargo.toml, LICENSE, README, or src content missing from $archive."
    exit 1
  fi

  if ! tar -xOzf "$archive" "${root}LICENSE" | cmp --silent LICENSE -; then
    echo "Packaged license differs from the repository license: $archive"
    exit 1
  fi

  printf 'audited %s: %s files, %s bytes\n' "$(basename "$archive")" "${#entries[@]}" "$archive_bytes"
done
