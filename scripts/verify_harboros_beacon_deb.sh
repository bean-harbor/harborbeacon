#!/usr/bin/env bash
set -euo pipefail

deb_path="${1:?usage: verify_harboros_beacon_deb.sh <deb-path>}"
if [ ! -f "$deb_path" ]; then
  echo "deb package not found: $deb_path" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

bash -n debian/postinst
bash -n debian/prerm

dpkg-deb --contents "$deb_path" >"$tmp_dir/contents.txt"
dpkg-deb --control "$deb_path" "$tmp_dir/control"
dpkg-deb --fsys-tarfile "$deb_path" | tar -tf - >"$tmp_dir/files.txt"

grep -F "Package: harboros-beacon" "$tmp_dir/control/control" >/dev/null
grep -F "Architecture: amd64" "$tmp_dir/control/control" >/dev/null
grep -E '(^|/)usr/bin/harboros-beacon$' "$tmp_dir/files.txt" >/dev/null
grep -E '(^|/)etc/systemd/system/harboros-beacon.service$' "$tmp_dir/files.txt" >/dev/null
grep -E '(^|/)mnt/software/harborbeacon-agent-ci/model-store/runtimes/harbor-candle/bootstrap-llm/model\.safetensors$' "$tmp_dir/files.txt" >/dev/null

grep -F "ExecStart=/usr/bin/harboros-beacon --bind 127.0.0.1:4174" debian/harboros-beacon.service >/dev/null
grep -F -- "--harbor-assistant-dist /usr/share/truenas/webui" debian/harboros-beacon.service >/dev/null
grep -F "HARBOR_MODEL_API_BASE_URL" debian/postinst >/dev/null
grep -F "http://127.0.0.1:4174/api/inference/v1" debian/postinst >/dev/null
grep -F "HARBOR_MODEL_API_BACKEND" debian/postinst >/dev/null

sha256sum "$deb_path" >"${deb_path}.sha256"
echo "Verified HarborBeacon deb package: $deb_path"
