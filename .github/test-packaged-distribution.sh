#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: test-packaged-distribution.sh VERSION [PACKAGE_DIR] [--supervision-candidate|--v13-candidates]}"
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
# Archives normalize source timestamps. Preserve their bytes while preventing
# stale source reuse when an operator supplies a cached CARGO_TARGET_DIR.
find "$packages_dir" -type f -exec touch {} +

if [ "$candidate" = true ]; then
  candidate_archive="$package_dir/rullst-supervision-${version}.crate"
  # The caller must audit the complete archive set before extraction.
  tar -xzf "$candidate_archive" -C "$packages_dir"
  candidate_source="$packages_dir/rullst-supervision-${version}"
  find "$candidate_source" -type f -exec touch {} +
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
  bash "$repository_root/.github/test-labs-package.sh" "$version" "$package_dir"
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

# Exercise the PostgreSQL public facade against the extracted archives, using
# the same independent-pool/fault/restart contracts as the source package.
consent_dir="$work_dir/consent-postgres-consumer"
mkdir -p "$consent_dir/tests/consent"
{
  printf '[package]\nname = "rullst-packaged-consent"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[features]\ndefault = ["consent", "consent-postgres"]\nconsent = []\nconsent-postgres = []\nconsent-sqlite = []\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["privacy-consent-postgres"] }\n' "$version"
  printf 'tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "sync", "time"] }\n'
  printf 'sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "postgres", "tls-rustls-ring"] }\n'
} > "$consent_dir/Cargo.toml"
cp "$repository_root/rullst-privacy/tests/consent.rs" "$consent_dir/tests/consent.rs"
cp "$repository_root/rullst-privacy/tests/consent/adapter_failures.rs" "$consent_dir/tests/consent/adapter_failures.rs"
cp -R "$repository_root/rullst-privacy/tests/consent/postgres" "$consent_dir/tests/consent/postgres"
python3 - "$consent_dir/tests/consent.rs" <<'PY'
from pathlib import Path
import sys
source = Path(sys.argv[1])
text = source.read_text()
assert 'use rullst_privacy::consent::*;' in text
source.write_text(text.replace('use rullst_privacy::consent::*;', 'use rullst::privacy::consent::*;'))
PY
append_package_patches "$consent_dir/Cargo.toml"
"$cargo_bin" generate-lockfile --manifest-path "$consent_dir/Cargo.toml" --offline
"$cargo_bin" test --manifest-path "$consent_dir/Cargo.toml" --offline --locked --test consent
python3 "$repository_root/.github/check-privacy-postgres.py" \
  --suite consent --manifest-path "$consent_dir/Cargo.toml"

# Email-login contracts run through only the extracted facade/auth archives.
login_dir="$work_dir/email-login-consumer"
mkdir -p "$login_dir/tests/email_login"
{
  printf '[package]\nname = "rullst-packaged-email-login"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[features]\ndefault = ["email-login-sqlite", "email-login-postgres"]\nemail-login-sqlite = []\nemail-login-postgres = []\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["auth-email-login-sqlite", "auth-email-login-postgres", "mail"] }\n' "$version"
  printf 'tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "process", "io-std"] }\n'
  printf 'sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "any", "sqlite", "postgres", "tls-rustls-ring"] }\n'
  printf 'tempfile = "3"\nurl = "2.5.8"\nrand = "0.10.1"\nchrono = "0.4.45"\nserde = { version = "1", features = ["derive"] }\nserde_json = "1"\n'
} > "$login_dir/Cargo.toml"
cp "$repository_root/.github/fixtures/email-login-mail-facade.rs" "$login_dir/tests/email_login_mail.rs"
for test in email_login_contract email_login_restart; do
  cp "$repository_root/rullst-auth/tests/$test.rs" "$login_dir/tests/$test.rs"
done
for test in support lifecycle failures locking limits postgres; do
  cp "$repository_root/rullst-auth/tests/email_login/$test.rs" "$login_dir/tests/email_login/$test.rs"
done
python3 - "$login_dir/tests" <<'LOGIN_PY'
from pathlib import Path
import sys
for source in Path(sys.argv[1]).rglob('*.rs'):
    source.write_text(source.read_text().replace('rullst_auth::', 'rullst::auth::'))
LOGIN_PY
append_package_patches "$login_dir/Cargo.toml"
"$cargo_bin" generate-lockfile --manifest-path "$login_dir/Cargo.toml" --offline
"$cargo_bin" test --manifest-path "$login_dir/Cargo.toml" --offline --locked
python3 "$repository_root/.github/check-auth-recovery-postgres.py" \
  --suite email-login --manifest-path "$login_dir/Cargo.toml"

# Verify token lifecycle and exact-route HTTP policy through extracted packages,
# including independent processes and an actual PostgreSQL service restart.
api_dir="$work_dir/api-token-consumer"
mkdir -p "$api_dir/tests/api_tokens"
{
  printf '[package]\nname = "rullst-packaged-api-tokens"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[features]\ndefault = ["api-tokens-sqlite", "api-tokens-postgres"]\napi-tokens-sqlite = []\napi-tokens-postgres = []\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["auth-api-tokens-sqlite", "auth-api-tokens-postgres"] }\n' "$version"
  printf 'tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "process", "io-std"] }\n'
  printf 'sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "any", "sqlite", "postgres", "tls-rustls-ring"] }\n'
  printf 'tempfile = "3"\nurl = "2.5.8"\nrand = "0.10.1"\naxum = "0.8.9"\nserde = { version = "1", features = ["derive"] }\nserde_json = "1"\n'
} > "$api_dir/Cargo.toml"
for test in api_token_contract api_token_http api_token_restart; do
  cp "$repository_root/rullst-auth/tests/$test.rs" "$api_dir/tests/$test.rs"
done
cp "$repository_root/rullst-auth/tests/api_tokens/"*.rs "$api_dir/tests/api_tokens/"
python3 - "$api_dir/tests" <<'API_PY'
from pathlib import Path
import sys
for source in Path(sys.argv[1]).rglob('*.rs'):
    source.write_text(source.read_text().replace('rullst_auth::', 'rullst::auth::').replace('rullst_core::', 'rullst::'))
API_PY
append_package_patches "$api_dir/Cargo.toml"
"$cargo_bin" generate-lockfile --manifest-path "$api_dir/Cargo.toml" --offline
"$cargo_bin" test --manifest-path "$api_dir/Cargo.toml" --offline --locked
python3 "$repository_root/.github/check-auth-recovery-postgres.py" \
  --suite api-tokens --manifest-path "$api_dir/Cargo.toml"

mail_pg_dir="$work_dir/mail-postgres-consumer"
mkdir -p "$mail_pg_dir/tests/suppression_postgres"
{
  printf '[package]\nname = "rullst-packaged-mail-postgres"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[features]\ndefault = ["postgres"]\npostgres = []\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["mail-postgres"] }\n' "$version"
  printf 'tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "process", "io-std"] }\n'
  printf 'sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "postgres", "tls-rustls-ring"] }\n'
  printf 'url = "2.5.8"\nrand = "0.10.1"\nasync-trait = "0.1"\nserde = { version = "1", features = ["derive"] }\nserde_json = "1"\n'
} > "$mail_pg_dir/Cargo.toml"
for test in postgres_suppression postgres_suppression_restart; do
  cp "$repository_root/rullst-mail/tests/$test.rs" "$mail_pg_dir/tests/$test.rs"
done
cp "$repository_root/rullst-mail/tests/suppression_postgres/"*.rs "$mail_pg_dir/tests/suppression_postgres/"
python3 - "$mail_pg_dir/tests" <<'MAIL_PG_PY'
from pathlib import Path
import sys
for source in Path(sys.argv[1]).rglob('*.rs'):
    source.write_text(source.read_text().replace('rullst_mail::', 'rullst::mail::').replace('rullst_core::', 'rullst::'))
MAIL_PG_PY
append_package_patches "$mail_pg_dir/Cargo.toml"
"$cargo_bin" generate-lockfile --manifest-path "$mail_pg_dir/Cargo.toml" --offline
"$cargo_bin" test --manifest-path "$mail_pg_dir/Cargo.toml" --offline --locked
python3 "$repository_root/.github/check-mail-postgres.py" --manifest-path "$mail_pg_dir/Cargo.toml"

recurring_dir="$work_dir/recurring-consumer"
mkdir -p "$recurring_dir/tests/recurring"
{
  printf '[package]\nname = "rullst-packaged-recurring"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[features]\ndefault = ["schedules-postgres", "sqlite"]\nschedules-postgres = []\nsqlite = []\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["messaging-schedules-postgres", "messaging-sqlite"] }\n' "$version"
  printf 'tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "process", "io-std", "io-util"] }\n'
  printf 'sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "postgres", "tls-rustls-ring"] }\n'
  printf 'url = "2.5.8"\nuuid = { version = "1", features = ["v4"] }\nserde = { version = "1", features = ["derive"] }\nserde_json = "1"\n'
} > "$recurring_dir/Cargo.toml"
for test in recurring_contract recurring_restart; do
  cp "$repository_root/rullst-messaging/tests/$test.rs" "$recurring_dir/tests/$test.rs"
done
cp "$repository_root/rullst-messaging/tests/recurring/"*.rs "$recurring_dir/tests/recurring/"
python3 - "$recurring_dir/tests" <<'RECURRING_PY'
from pathlib import Path
import sys
for source in Path(sys.argv[1]).rglob('*.rs'):
    source.write_text(source.read_text().replace('rullst_messaging::', 'rullst::messaging::'))
RECURRING_PY
append_package_patches "$recurring_dir/Cargo.toml"
"$cargo_bin" generate-lockfile --manifest-path "$recurring_dir/Cargo.toml" --offline
"$cargo_bin" test --manifest-path "$recurring_dir/Cargo.toml" --offline --locked
python3 "$repository_root/.github/check-recurring-postgres.py" --manifest-path "$recurring_dir/Cargo.toml"

webhooks_dir="$work_dir/webhooks-consumer"
mkdir -p "$webhooks_dir/tests/webhook"
{
  printf '[package]\nname = "rullst-packaged-webhooks"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[features]\ndefault = ["webhooks"]\nwebhooks = []\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["messaging-webhooks"] }\n' "$version"
  printf 'tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "process", "io-std", "io-util", "net"] }\n'
  printf 'sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "sqlite"] }\n'
  printf 'uuid = { version = "1", features = ["v4"] }\nbase64 = "0.23.0"\nserde = { version = "1", features = ["derive"] }\nserde_json = "1"\n'
  printf 'reqwest = { version = "0.13.5", default-features = false, features = ["rustls"] }\n'
  printf 'tokio-rustls = { version = "0.26.4", default-features = false, features = ["aws_lc_rs", "tls12"] }\n'
  printf 'rcgen = { version = "=0.14.10", default-features = false, features = ["aws_lc_rs", "pem", "zeroize"] }\n'
} > "$webhooks_dir/Cargo.toml"
for test in webhook_contract webhook_restart; do
  cp "$repository_root/rullst-messaging/tests/$test.rs" "$webhooks_dir/tests/$test.rs"
done
cp "$repository_root/rullst-messaging/tests/webhook/"*.rs "$webhooks_dir/tests/webhook/"
python3 - "$webhooks_dir/tests" <<'WEBHOOKS_PY'
from pathlib import Path
import sys
for source in Path(sys.argv[1]).rglob('*.rs'):
    source.write_text(source.read_text().replace('rullst_messaging::', 'rullst::messaging::'))
WEBHOOKS_PY
append_package_patches "$webhooks_dir/Cargo.toml"
"$cargo_bin" generate-lockfile --manifest-path "$webhooks_dir/Cargo.toml" --offline
"$cargo_bin" test --manifest-path "$webhooks_dir/Cargo.toml" --offline --locked

storage_dir="$work_dir/storage-consumer"
mkdir -p "$storage_dir/tests"
{
  printf '[package]\nname = "rullst-packaged-storage"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["storage-s3", "security"] }\n' "$version"
} > "$storage_dir/Cargo.toml"
cp "$repository_root/.github/fixtures/storage-facade.rs" "$storage_dir/tests/storage_facade.rs"
append_package_patches "$storage_dir/Cargo.toml"
"$cargo_bin" test --manifest-path "$storage_dir/Cargo.toml" --offline --test storage_facade

multipart_dir="$work_dir/multipart-consumer"
mkdir -p "$multipart_dir/tests"
{
  printf '[package]\nname = "rullst-packaged-multipart"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n'
  printf '[features]\ndefault = ["storage-s3", "storage-multipart"]\nstorage-s3 = []\nstorage-multipart = []\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["storage-multipart"] }\n' "$version"
  printf 'base64 = "0.23"\nring = "0.17"\nserde_json = "1"\naxum = "0.8"\nreqwest = { version = "0.13", default-features = false, features = ["rustls"] }\n'
  printf 'tokio = { version = "1", features = ["macros", "rt-multi-thread", "net", "time"] }\n'
} > "$multipart_dir/Cargo.toml"
for test in storage_multipart storage_multipart_failures storage_multipart_live storage_s3_live; do
  cp "$repository_root/rullst-core/tests/$test.rs" "$multipart_dir/tests/$test.rs"
done
python3 - "$multipart_dir/tests" <<'MULTIPART_PY'
import sys
from pathlib import Path
for source in Path(sys.argv[1]).glob('*.rs'):
    source.write_text(source.read_text().replace('rullst_core::', 'rullst::'))
MULTIPART_PY
append_package_patches "$multipart_dir/Cargo.toml"
"$cargo_bin" generate-lockfile --manifest-path "$multipart_dir/Cargo.toml" --offline
"$cargo_bin" test --manifest-path "$multipart_dir/Cargo.toml" --offline --locked
bash "$repository_root/.github/test-storage-s3-live.sh" --manifest-path "$multipart_dir/Cargo.toml"

session_dir="$work_dir/session-consumer"
mkdir -p "$session_dir/tests"
{
  printf '[package]\nname = "rullst-packaged-sessions"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["auth-sessions-sqlite"] }\n' "$version"
  printf '\n[dev-dependencies]\ntempfile = "3"\n'
} > "$session_dir/Cargo.toml"
cp "$repository_root/.github/fixtures/session-facade.rs" "$session_dir/tests/session_facade.rs"
append_package_patches "$session_dir/Cargo.toml"
"$cargo_bin" test --manifest-path "$session_dir/Cargo.toml" --offline --test session_facade

messaging_redis_dir="$work_dir/messaging-redis-consumer"
mkdir -p "$messaging_redis_dir/tests"
{
  printf '[package]\nname = "rullst-packaged-messaging-redis"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["messaging-redis"] }\n' "$version"
} > "$messaging_redis_dir/Cargo.toml"
cp "$repository_root/.github/fixtures/messaging-redis-facade.rs" "$messaging_redis_dir/tests/messaging_redis_facade.rs"
append_package_patches "$messaging_redis_dir/Cargo.toml"
"$cargo_bin" test --manifest-path "$messaging_redis_dir/Cargo.toml" --offline --test messaging_redis_facade

live_dir="$work_dir/live-recovery-consumer"
mkdir -p "$live_dir/tests"
{
  printf '[package]\nname = "rullst-packaged-live-recovery"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false }\n' "$version"
} > "$live_dir/Cargo.toml"
cp "$repository_root/.github/fixtures/live-recovery-facade.rs" "$live_dir/tests/live_recovery_facade.rs"
append_package_patches "$live_dir/Cargo.toml"
"$cargo_bin" test --manifest-path "$live_dir/Cargo.toml" --offline --test live_recovery_facade

tracing_dir="$work_dir/tracing-consumer"
mkdir -p "$tracing_dir/tests"
{
  printf '[package]\nname = "rullst-packaged-tracing"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["telemetry"] }\n' "$version"
  printf 'tracing = "0.1.44"\ntracing-subscriber = "0.3"\n'
} > "$tracing_dir/Cargo.toml"
cp "$repository_root/.github/fixtures/distributed-tracing-facade.rs" "$tracing_dir/tests/telemetry_facade.rs"
append_package_patches "$tracing_dir/Cargo.toml"
"$cargo_bin" test --manifest-path "$tracing_dir/Cargo.toml" --offline --test telemetry_facade

partial_dir="$work_dir/partial-update-consumer"
mkdir -p "$partial_dir/tests"
{
  printf '[package]\nname = "rullst-packaged-partial-update"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n'
  printf 'rullst = { version = "=%s", default-features = false, features = ["strict-sqlite"] }\n' "$version"
  printf 'tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread"] }\nsqlx = { version = "0.9.0", default-features = false }\ntracing = "0.1.44"\n'
  cat <<'TOML'
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(feature, values("redis"))'] }
TOML
} > "$partial_dir/Cargo.toml"
cp "$repository_root/.github/fixtures/partial-update-facade.rs" "$partial_dir/tests/partial_update.rs"
append_package_patches "$partial_dir/Cargo.toml"
"$cargo_bin" test --manifest-path "$partial_dir/Cargo.toml" --offline --test partial_update

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
