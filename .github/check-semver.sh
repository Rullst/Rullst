#!/usr/bin/env bash
set -euo pipefail

repository_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repository_root"

semver_tmp=$(mktemp -d "${RUNNER_TEMP:-/tmp}/rullst-semver.XXXXXX")
trap 'rm -rf -- "${semver_tmp:?}"' EXIT

metadata_path="$semver_tmp/metadata.json"
cargo metadata --no-deps --format-version 1 > "$metadata_path"

# Both sides of the comparison are documented from a workspace with its
# committed Cargo.lock: the current tree here, and the published baseline's
# source commit. cargo-semver-checks would instead re-resolve each crate from
# scratch, and crates.io has yanked releases the published graphs need (every
# zip before 9.0.3: utoipa-swagger-ui in 13.0.0-alpha.1 Studio, libduckdb-sys
# for the ORM `duckdb` feature), so that resolution fails. The JSON is built the
# way cargo-semver-checks 0.51 builds it, with its default feature selection.
rustdoc_target="${CARGO_TARGET_DIR:-$repository_root/target}/semver-rustdoc"

# Every feature (explicit, and implicit for optional dependencies without a
# `dep:` reference) except cargo-semver-checks' heuristically private ones:
# unstable, nightly, bench, no_std and the _, unstable- and unstable_ prefixes.
heuristic_features_filter='
  def private_feature:
    . as $feature
    | (["unstable", "nightly", "bench", "no_std"] | any(. == $feature))
      or startswith("_") or startswith("unstable-") or startswith("unstable_");
  .packages[]
  | select(.name == $package)
  | ([.features[][] | select(startswith("dep:")) | ltrimstr("dep:")]) as $explicit
  | ([.dependencies[] | select(.optional) | (.rename // .name)]
     | unique
     | map(select(. as $dependency | $explicit | any(. == $dependency) | not))) as $implicit
  | ((.features | keys) + $implicit)
  | unique
  | map(select(private_feature | not))
  | join(",")
'

# Usage: rustdoc_json WORKSPACE_MANIFEST PACKAGE METADATA_JSON OUTPUT_JSON
rustdoc_json() {
  local workspace_manifest=$1 package=$2 package_metadata=$3 output=$4
  local features lib_name
  features=$(jq -r --arg package "$package" "$heuristic_features_filter" "$package_metadata")
  lib_name=$(
    jq -er --arg package "$package" '
      [.packages[] | select(.name == $package) | .targets[]
       | select(any(.kind[]; . == "lib")) | .name][0]
    ' "$package_metadata"
  )
  local -a feature_arguments=()
  if [[ -n "$features" ]]; then
    feature_arguments=(--features "$features")
  fi
  RUSTC_BOOTSTRAP=1 \
    RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cap-lints=allow" \
    RUSTDOCFLAGS="${RUSTDOCFLAGS:+$RUSTDOCFLAGS }-Z unstable-options --document-private-items --document-hidden-items --output-format=json --cap-lints=allow" \
    cargo doc \
      --manifest-path "$workspace_manifest" \
      --locked \
      --package "$package" \
      --lib \
      --no-deps \
      --target-dir "$rustdoc_target" \
      ${feature_arguments[@]+"${feature_arguments[@]}"}
  cp "$rustdoc_target/doc/${lib_name//-/_}.json" "$output"
}

package_list="$semver_tmp/packages.txt"
case $# in
  0)
    jq -r '.[]' .github/release-order.json > "$package_list"
    ;;
  1)
    selected_package=$1
    if [[ ! "$selected_package" =~ ^[a-zA-Z0-9_-]+$ ]]; then
      echo "Invalid selected package name: $selected_package" >&2
      exit 2
    fi
    if ! jq -e --arg package "$selected_package" \
      'any(.[]; . == $package)' .github/release-order.json >/dev/null; then
      echo "Selected package is absent from .github/release-order.json: $selected_package" >&2
      exit 2
    fi
    printf '%s\n' "$selected_package" > "$package_list"
    ;;
  *)
    echo "usage: $0 [package-from-release-order]" >&2
    exit 2
    ;;
esac

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
      if [[ ! "$baseline" =~ ^[0-9A-Za-z][0-9A-Za-z.+-]{0,127}$ ]]; then
        echo "Invalid baseline version returned for $package" >&2
        exit 1
      fi
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

      baseline_root="$semver_tmp/$package-$baseline"
      if ! awk -v root="$package-$baseline" '
        index($0, "\\") != 0 { exit 1 }
        $0 == root || index($0, root "/") == 1 {
          relative = substr($0, length(root) + 2)
          count = split(relative, components, "/")
          for (part_index = 1; part_index <= count; part_index++) {
            if (components[part_index] == "..") { exit 1 }
          }
          next
        }
        { exit 1 }
      ' "$archive_members"; then
        echo "Published package $package $baseline contains an unsafe archive path" >&2
        exit 1
      fi
      tar --extract --file "$archive_path" --directory "$semver_tmp" \
        --no-same-owner --no-same-permissions

      # The published package names the commit it was built from. Rebuild
      # that commit's workspace with its own Cargo.lock, after checking that
      # its library source is the published one.
      vcs_member="$package-$baseline/.cargo_vcs_info.json"
      if ! grep -Fxq "$vcs_member" "$archive_members" ||
        ! grep -Fxq "$package-$baseline/Cargo.toml.orig" "$archive_members"; then
        echo "Published package $package $baseline does not record its source commit and manifest" >&2
        exit 1
      fi
      vcs_info="$semver_tmp/$package-$baseline.vcs.json"
      tar -xOf "$archive_path" "$vcs_member" > "$vcs_info"
      if jq -e '.git.dirty == true' "$vcs_info" >/dev/null; then
        echo "Published package $package $baseline was packaged from a modified tree" >&2
        exit 1
      fi
      source_commit=$(jq -er '.git.sha1 | select(test("^[0-9a-f]{40}$"))' "$vcs_info")
      package_path=$(
        jq -er '.path_in_vcs | select(test("^[A-Za-z0-9_-]+(/[A-Za-z0-9_-]+)*$"))' "$vcs_info"
      )
      if ! git cat-file -e "$source_commit^{commit}" 2>/dev/null; then
        git fetch --quiet --no-tags --depth=1 origin "$source_commit"
      fi
      baseline_tree="$semver_tmp/source-$source_commit"
      if [[ ! -d "$baseline_tree" ]]; then
        mkdir "$baseline_tree"
        git archive --format=tar "$source_commit" | tar -x -C "$baseline_tree"
      fi
      if ! cmp -s "$baseline_root/Cargo.toml.orig" "$baseline_tree/$package_path/Cargo.toml"; then
        echo "Published $package $baseline manifest differs from commit $source_commit" >&2
        exit 1
      fi
      while IFS= read -r member; do
        relative=${member#"$package-$baseline/"}
        case "$relative" in
          */) continue ;;
          src/* | build.rs) ;;
          *) continue ;;
        esac
        if ! cmp -s "$baseline_root/$relative" "$baseline_tree/$package_path/$relative"; then
          echo "Published $package $baseline differs from commit $source_commit at $relative" >&2
          exit 1
        fi
      done < "$archive_members"

      baseline_metadata="$semver_tmp/$package-$baseline.metadata.json"
      cargo metadata --manifest-path "$baseline_tree/Cargo.toml" \
        --no-deps --format-version 1 > "$baseline_metadata"
      if ! jq -e --arg package "$package" --arg manifest "/$package_path/Cargo.toml" '
          any(.packages[]; .name == $package and (.manifest_path | endswith($manifest)))
        ' "$baseline_metadata" >/dev/null; then
        echo "Commit $source_commit has no workspace package $package at $package_path" >&2
        exit 1
      fi

      echo "::group::$package against crates.io $baseline (commit $source_commit)"
      baseline_json="$semver_tmp/$package-$baseline.baseline.json"
      current_json="$semver_tmp/$package.current.json"
      rustdoc_json "$baseline_tree/Cargo.toml" "$package" "$baseline_metadata" "$baseline_json"
      rustdoc_json "$repository_root/Cargo.toml" "$package" "$metadata_path" "$current_json"
      cargo semver-checks check-release \
        --baseline-rustdoc "$baseline_json" \
        --current-rustdoc "$current_json"
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
done < "$package_list"
