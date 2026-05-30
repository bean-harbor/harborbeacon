#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <harborassistant-live-solidify-output-dir>" >&2
  exit 2
fi

output_dir=$1
artifact_id="harborassistant-live-solidify-20260529"
version="20260529+harborassistant.live.solidify"

fail() {
  echo "offline delivery verification failed: $*" >&2
  exit 1
}

require_file() {
  local path=$1
  [[ -f "$path" ]] || fail "missing $path"
}

require_grep() {
  local pattern=$1
  local file=$2
  local description=$3
  grep -Eq "$pattern" "$file" || fail "$description not found in $file"
}

require_fixed() {
  local needle=$1
  local file=$2
  local description=$3
  grep -Fq "$needle" "$file" || fail "$description not found in $file"
}

count_glob() {
  local pattern=$1
  compgen -G "$pattern" | wc -l | tr -d '[:space:]'
}

cd "$output_dir" || fail "cannot enter $output_dir"

require_file manifest.json
require_file SHA256SUMS
require_file input.sha256

require_fixed "\"artifact_id\": \"$artifact_id\"" manifest.json "artifact id"
require_fixed "\"version\": \"$version\"" manifest.json "version"

echo "[sha256] checking artifact checksums"
sha256sum -c SHA256SUMS

[[ "$(count_glob "harboros-beacon_*_${artifact_id}_linux_amd64.deb")" == "1" ]] || fail "expected exactly one Beacon deb"
[[ "$(count_glob "harboros-im-gate_*_${artifact_id}_linux_amd64.deb")" == "1" ]] || fail "expected exactly one Gate deb"
[[ "$(count_glob "truenas-webui_*_${artifact_id}_all.deb")" == "1" ]] || fail "expected exactly one WebUI deb"
[[ "$(count_glob "truenas-webui_${artifact_id}_dist.tgz")" == "1" ]] || fail "expected exactly one WebUI dist tar"

beacon_deb=$(compgen -G "harboros-beacon_*_${artifact_id}_linux_amd64.deb")
gate_deb=$(compgen -G "harboros-im-gate_*_${artifact_id}_linux_amd64.deb")
webui_deb=$(compgen -G "truenas-webui_*_${artifact_id}_all.deb")
webui_dist=$(compgen -G "truenas-webui_${artifact_id}_dist.tgz")

check_control() {
  local deb=$1
  local package=$2
  local arch=$3
  local got_package got_version got_arch
  got_package=$(dpkg-deb -f "$deb" Package)
  got_version=$(dpkg-deb -f "$deb" Version)
  got_arch=$(dpkg-deb -f "$deb" Architecture)
  [[ "$got_package" == "$package" ]] || fail "$deb package is $got_package, expected $package"
  [[ "$got_version" == "$version" ]] || fail "$deb version is $got_version, expected $version"
  [[ "$got_arch" == "$arch" ]] || fail "$deb arch is $got_arch, expected $arch"
  echo "[control] $package $got_version $got_arch"
}

check_deb_scripts() {
  local deb=$1
  local tmp=$2
  mkdir -p "$tmp"
  dpkg-deb -e "$deb" "$tmp/control"
  [[ -f "$tmp/control/postinst" ]] || fail "$deb missing postinst"
  [[ -f "$tmp/control/prerm" ]] || fail "$deb missing prerm"
  bash -n "$tmp/control/postinst"
  bash -n "$tmp/control/prerm"
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

check_control "$beacon_deb" harboros-beacon amd64
check_deb_scripts "$beacon_deb" "$tmp/beacon"
dpkg-deb -c "$beacon_deb" > "$tmp/beacon.contents"
require_grep '\./usr/bin/harboros-beacon$' "$tmp/beacon.contents" "Beacon binary"
require_grep '\./etc/systemd/system/harboros-beacon\.service$' "$tmp/beacon.contents" "Beacon systemd unit"
require_grep 'bootstrap-llm/model\.safetensors$' "$tmp/beacon.contents" "Beacon Candle bootstrap model"

check_control "$gate_deb" harboros-im-gate amd64
check_deb_scripts "$gate_deb" "$tmp/gate"
dpkg-deb -c "$gate_deb" > "$tmp/gate.contents"
require_grep '\./usr/bin/harboros-im-gate$' "$tmp/gate.contents" "Gate binary"
require_grep '\./etc/systemd/system/harboros-im-gate\.service$' "$tmp/gate.contents" "Gate systemd unit"

check_control "$webui_deb" truenas-webui all
dpkg-deb -c "$webui_deb" > "$tmp/webui.contents"
require_grep '\./usr/share/truenas/webui/index\.html$' "$tmp/webui.contents" "WebUI index"

mkdir "$tmp/webui-dist"
tar -xf "$webui_dist" -C "$tmp/webui-dist"

count_dist_refs() {
  local pattern=$1
  { grep -Roh --include='*.js' "$pattern" "$tmp/webui-dist" 2>/dev/null || true; } \
    | wc -l \
    | tr -d '[:space:]'
}

api_beacon_count=$(count_dist_refs '/api/beacon')
api_gate_count=$(count_dist_refs '/api/harbor-gate')
api_legacy_count=$(count_dist_refs '/api/harbor-beacon')
api_old_count=$(count_dist_refs '/api/harbor-assistant')

[[ "$api_beacon_count" -gt 0 ]] || fail "WebUI dist has no /api/beacon references"
[[ "$api_gate_count" -gt 0 ]] || fail "WebUI dist has no /api/harbor-gate references"
[[ "$api_old_count" == "0" ]] || fail "WebUI dist still references /api/harbor-assistant"

echo "[webui-dist] /api/beacon=$api_beacon_count /api/harbor-gate=$api_gate_count /api/harbor-beacon=$api_legacy_count /api/harbor-assistant=$api_old_count"
echo "HarborAssistant offline delivery artifacts are package-ready."
