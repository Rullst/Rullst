#!/usr/bin/env bash
# Provision only a disposable hosted Ubuntu acceptance machine.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && "${RUNNER_ENVIRONMENT:-}" == github-hosted ]]
test "$(sysctl -n kernel.apparmor_restrict_unprivileged_userns)" = 1
rustup toolchain install 1.96.0 --profile minimal --target wasm32-unknown-unknown
sudo apt-get update -qq
sudo apt-get install -y --no-install-recommends bubblewrap binutils
profile_dir=$(mktemp -d)
trap 'rm -rf -- "$profile_dir"' EXIT
(cd "$profile_dir" && apt-get download apparmor-profiles)
dpkg-deb --extract "$profile_dir"/apparmor-profiles_*.deb "$profile_dir/extracted"
python3 .github/prepare-labs-host.py \
  "$profile_dir/extracted/usr/share/apparmor/extra-profiles/bwrap-userns-restrict" \
  "$profile_dir/rullst-labs-ci"
# Keep protected ancestors and the reviewed child capability denial. Never
# disable global AppArmor or relax user-namespace policy for these tests.
stat -c '%u:%a %n' / /usr /usr/lib
test ! -e /usr/lib/rullst-labs-ci
test ! -e /etc/apparmor.d/rullst-labs-ci
sudo install -d -m 0755 /usr/lib/rullst-labs-ci
sudo install -m 0755 /usr/bin/bwrap /usr/lib/rullst-labs-ci/bwrap
sudo install -m 0644 "$profile_dir/rullst-labs-ci" /etc/apparmor.d/rullst-labs-ci
sudo apparmor_parser --add /etc/apparmor.d/rullst-labs-ci
df -h . /tmp
