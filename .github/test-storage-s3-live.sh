#!/usr/bin/env bash
set -euo pipefail

test_command=(cargo test)
if [[ "${1:-}" == --coverage && "$#" -eq 1 ]]; then
  test_command=(cargo llvm-cov --no-report)
elif [[ "$#" -ne 0 ]]; then
  echo 'Usage: test-storage-s3-live.sh [--coverage]' >&2
  exit 1
fi

# Independent S3 implementation; this image is a disposable test fixture only.
# Garage v2.4.1, Linux amd64 manifest from the upstream dxflrs/garage image.
image='dxflrs/garage@sha256:0d7c74fc8ca6fef68a5a941c0e7558c8b1e92ba3588fa7505400e1350456c796'
fixture_dir="$(mktemp -d)"
container_id=''
cleanup() {
  if [[ -n "$container_id" ]]; then docker rm --force "$container_id" >/dev/null 2>&1 || true; fi
  rm -rf -- "$fixture_dir"
}
trap cleanup EXIT

python3 - "$fixture_dir/garage.toml" <<'PY'
import secrets
import sys
from pathlib import Path
Path(sys.argv[1]).write_text('''metadata_dir = "/data/meta"
data_dir = "/data/blocks"
db_engine = "sqlite"
replication_factor = 1
rpc_bind_addr = "127.0.0.1:3901"
rpc_public_addr = "127.0.0.1:3901"
rpc_secret = "''' + secrets.token_hex(32) + '''"
[s3_api]
s3_region = "auto"
api_bind_addr = "0.0.0.0:3900"
root_domain = ".s3.localhost"
''')
PY
mkdir "$fixture_dir/data"

container_id="$(docker run --detach --read-only --cap-drop ALL \
  --user "$(id -u):$(id -g)" \
  --security-opt no-new-privileges --memory 384m --cpus 1 --pids-limit 128 \
  --tmpfs /tmp:rw,noexec,nosuid,size=128m \
  --publish 127.0.0.1::3900 \
  --mount "type=bind,src=$fixture_dir/garage.toml,dst=/etc/garage.toml,readonly" \
  --mount "type=bind,src=$fixture_dir/data,dst=/data" \
  --env GARAGE_DEFAULT_ACCESS_KEY=GK11111111111111111111111111111111 \
  --env GARAGE_DEFAULT_SECRET_KEY=2222222222222222222222222222222222222222222222222222222222222222 \
  --env GARAGE_DEFAULT_BUCKET=private-files \
  "$image" /garage server --single-node --default-bucket)"

wait_ready() {
  local storage_test_status
  for attempt in $(seq 1 30); do
    export RULLST_STORAGE_TEST_ENDPOINT="http://$(docker port "$container_id" 3900/tcp)"
    storage_test_status="$(curl --silent --output /dev/null --write-out '%{http_code}' \
      --noproxy '*' --max-time 1 "$RULLST_STORAGE_TEST_ENDPOINT/" || true)"
    if [[ "$storage_test_status" == 400 || "$storage_test_status" == 403 ]] \
      && docker exec "$container_id" /garage bucket info private-files >/dev/null 2>&1; then return; fi
    sleep 1
  done
  echo 'Disposable S3 service failed to initialize' >&2
  docker logs --tail 40 "$container_id" >&2
  return 1
}
wait_ready

"${test_command[@]}" --locked -p rullst-core --no-default-features --features storage-s3 \
  --test storage_s3_live -- --ignored --exact \
  private_object_journey_rejects_unsigned_tampered_and_expired_grants

docker restart "$container_id" >/dev/null
wait_ready
"${test_command[@]}" --locked -p rullst-core --no-default-features --features storage-s3 \
  --test storage_s3_live -- --ignored --exact private_objects_persist_after_service_restart
