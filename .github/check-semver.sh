#!/usr/bin/env bash
set -euo pipefail

repository_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repository_root"

semver_tmp=$(mktemp -d "${RUNNER_TEMP:-/tmp}/rullst-semver.XXXXXX")
trap 'rm -rf -- "${semver_tmp:?}"' EXIT

metadata_path="$semver_tmp/metadata.json"
cargo metadata --no-deps --format-version 1 > "$metadata_path"

while IFS= read -r package; do
  if [[ ! "$package" =~ ^[a-zA-Z0-9_-]+$ ]]; then
    echo "Invalid package name in .github/release-order.json: $package" >&2
    exit 1
  fi

  target_kind=$(
    jq -er --arg package "$package" '
      [.packages[] | select(.name == $package) | .targets[].kind[]] |
      if length == 0 then error("release package is absent from cargo metadata")
      elif any(. == "lib") then "library"
      elif any(. == "proc-macro") then "procedural macro"
      else "non-library"
      end
    ' "$metadata_path"
  )

  response_path="$semver_tmp/$package.json"
  status=$(curl --silent --show-error --location --retry 3 \
    --user-agent 'Rullst-SemVer-CI/1.0 (officialrullst@gmail.com)' \
    --output "$response_path" --write-out '%{http_code}' \
    "https://crates.io/api/v1/crates/$package")

  case "$status" in
    200)
      baseline=$(jq -er '[.versions[] | select(.yanked == false)][0].num' "$response_path")
      if [[ "$target_kind" != "library" ]]; then
        echo "::notice title=SemVer tool boundary::$package $baseline is a $target_kind target; cargo-semver-checks cannot compare this API surface"
        continue
      fi

      archive_path="$semver_tmp/$package-$baseline.crate"
      archive_members="$semver_tmp/$package-$baseline.members"
      baseline_manifest="$semver_tmp/$package-$baseline.Cargo.toml"
      manifest_member="$package-$baseline/Cargo.toml"

      curl --silent --show-error --fail --location --retry 3 \
        --user-agent 'Rullst-SemVer-CI/1.0 (officialrullst@gmail.com)' \
        --output "$archive_path" \
        "https://crates.io/api/v1/crates/$package/$baseline/download"
      tar -tf "$archive_path" > "$archive_members"
      if ! grep -Fxq "$manifest_member" "$archive_members"; then
        echo "Published package $package $baseline has no normalized Cargo.toml" >&2
        exit 1
      fi
      tar -xOf "$archive_path" "$manifest_member" > "$baseline_manifest"

      if ! grep -Eq '^\[lib\][[:space:]]*$' "$baseline_manifest"; then
        echo "::notice title=SemVer baseline boundary::$package $baseline has no published library target to compare"
        continue
      fi
      if awk '
        /^\[lib\][[:space:]]*$/ { in_lib = 1; next }
        /^\[/ { in_lib = 0 }
        in_lib && /^proc-macro[[:space:]]*=[[:space:]]*true[[:space:]]*$/ { found = 1 }
        END { exit found ? 0 : 1 }
      ' "$baseline_manifest"; then
        echo "::notice title=SemVer tool boundary::$package $baseline is a procedural macro target; cargo-semver-checks cannot compare this API surface"
        continue
      fi

      echo "::group::$package against crates.io $baseline"
      cargo semver-checks check-release \
        --package "$package" \
        --baseline-version "$baseline"
      echo "::endgroup::"
      ;;
    404)
      echo "::notice title=SemVer baseline::$package is not published yet; no registry API baseline exists"
      ;;
    *)
      echo "crates.io returned HTTP $status while resolving $package" >&2
      exit 1
      ;;
  esac
done < <(jq -r '.[]' .github/release-order.json)
