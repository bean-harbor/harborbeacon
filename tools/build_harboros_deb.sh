#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

PACKAGE_NAME="${HARBOR_DEB_PACKAGE_NAME:-harborbeacon-harboros-release}"
MAINTAINER="${HARBOR_DEB_MAINTAINER:-Harbor Innovations <release@harborinnovations.ai>}"
OUTPUT_DIR="${OUT_DIR:-${REPO_ROOT}/dist/deb}"
BUNDLE_PATH=""
VERSION_OVERRIDE="${HARBOR_DEB_VERSION:-}"

usage() {
  cat <<'EOF'
Usage: build_harboros_deb.sh --bundle PATH [options]

Build a Debian carrier package for a HarborOS release bundle.

Options:
  --bundle PATH        Required harbor-release-<version>.tar.gz bundle.
  --output-dir PATH    Directory for the .deb artifact.
  --version VERSION    Debian package version seed. Defaults to bundle manifest version.
  --package-name NAME  Debian package name. Defaults to harborbeacon-harboros-release.
  --maintainer TEXT    Debian Maintainer field.
  -h, --help           Show this help.
EOF
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 127
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bundle)
      BUNDLE_PATH="${2:-}"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    --version)
      VERSION_OVERRIDE="${2:-}"
      shift 2
      ;;
    --package-name)
      PACKAGE_NAME="${2:-}"
      shift 2
      ;;
    --maintainer)
      MAINTAINER="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "${BUNDLE_PATH}" ]]; then
  echo "--bundle is required" >&2
  usage >&2
  exit 2
fi

require_command awk
require_command cp
require_command dpkg-deb
require_command du
require_command find
require_command python3
require_command sha256sum
require_command sort
require_command tar

BUNDLE_PATH="$(cd "$(dirname "${BUNDLE_PATH}")" && pwd)/$(basename "${BUNDLE_PATH}")"
if [[ ! -f "${BUNDLE_PATH}" ]]; then
  echo "bundle not found: ${BUNDLE_PATH}" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "${WORK_DIR}"
}
trap cleanup EXIT

EXTRACT_DIR="${WORK_DIR}/extract"
mkdir -p "${EXTRACT_DIR}" "${OUTPUT_DIR}"
tar -C "${EXTRACT_DIR}" -xzf "${BUNDLE_PATH}"
mapfile -t BUNDLE_ROOTS < <(find "${EXTRACT_DIR}" -mindepth 1 -maxdepth 1 -type d | sort)
if [[ "${#BUNDLE_ROOTS[@]}" -ne 1 ]]; then
  echo "expected bundle tarball to contain exactly one root directory" >&2
  exit 1
fi
BUNDLE_ROOT="${BUNDLE_ROOTS[0]}"
MANIFEST_PATH="${BUNDLE_ROOT}/manifest.json"

if [[ ! -f "${MANIFEST_PATH}" ]]; then
  echo "manifest.json missing from bundle" >&2
  exit 1
fi
if [[ ! -d "${BUNDLE_ROOT}/install" ]]; then
  echo "install/ directory missing from bundle" >&2
  exit 1
fi
if [[ ! -f "${BUNDLE_ROOT}/install/install_harboros_release.sh" ]]; then
  echo "install/install_harboros_release.sh missing from bundle" >&2
  exit 1
fi
if [[ ! -f "${BUNDLE_ROOT}/install/verify_release_bundle.py" ]]; then
  echo "install/verify_release_bundle.py missing from bundle" >&2
  exit 1
fi

BUNDLE_VERSION="$(python3 - "${MANIFEST_PATH}" "${BUNDLE_ROOT}" <<'PY'
import json
import pathlib
import sys

manifest = pathlib.Path(sys.argv[1])
bundle_root = pathlib.Path(sys.argv[2])
payload = json.loads(manifest.read_text(encoding="utf-8"))
version = str(payload.get("version") or "")
if not version:
    name = bundle_root.name
    version = name.removeprefix("harbor-release-")
print(version)
PY
)"

DEB_VERSION_RAW="${VERSION_OVERRIDE:-${BUNDLE_VERSION}}"
DEB_VERSION="$(python3 - "${DEB_VERSION_RAW}" <<'PY'
import re
import sys

raw = sys.argv[1].strip().removeprefix("harbor-release-")
safe = re.sub(r"[^A-Za-z0-9.+~_-]+", "+", raw)
safe = safe.strip("+._-")
if not safe:
    safe = "0"
if not safe[0].isdigit():
    safe = f"0.0.0+{safe}"
print(safe[:180])
PY
)"

PACKAGE_ROOT="${WORK_DIR}/${PACKAGE_NAME}"
PACKAGE_LIB_DIR="${PACKAGE_ROOT}/usr/lib/${PACKAGE_NAME}"
PACKAGE_DOC_DIR="${PACKAGE_ROOT}/usr/share/doc/${PACKAGE_NAME}"
BUNDLE_FILENAME="$(basename "${BUNDLE_PATH}")"

mkdir -p \
  "${PACKAGE_ROOT}/DEBIAN" \
  "${PACKAGE_ROOT}/usr/sbin" \
  "${PACKAGE_LIB_DIR}/bundles" \
  "${PACKAGE_DOC_DIR}"

cp "${BUNDLE_PATH}" "${PACKAGE_LIB_DIR}/bundles/${BUNDLE_FILENAME}"
sha256sum "${BUNDLE_PATH}" > "${PACKAGE_LIB_DIR}/bundles/${BUNDLE_FILENAME}.sha256"
cp -R "${BUNDLE_ROOT}/install" "${PACKAGE_LIB_DIR}/install"
cp "${MANIFEST_PATH}" "${PACKAGE_DOC_DIR}/manifest.json"
if [[ -f "${BUNDLE_ROOT}/checksums.sha256" ]]; then
  cp "${BUNDLE_ROOT}/checksums.sha256" "${PACKAGE_DOC_DIR}/bundle-checksums.sha256"
fi
if [[ -f "${BUNDLE_ROOT}/media-tools/provenance.json" ]]; then
  cp "${BUNDLE_ROOT}/media-tools/provenance.json" "${PACKAGE_DOC_DIR}/media-tools-provenance.json"
fi

cat > "${PACKAGE_ROOT}/usr/sbin/install-harborbeacon-release" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

PACKAGE_ROOT="/usr/lib/__HARBOR_DEB_PACKAGE_NAME__"
INSTALLER="${PACKAGE_ROOT}/install/install_harboros_release.sh"
BUNDLE="${HARBOR_RELEASE_BUNDLE:-}"

if [[ ! -x "${INSTALLER}" && ! -f "${INSTALLER}" ]]; then
  echo "HarborBeacon release installer not found: ${INSTALLER}" >&2
  exit 1
fi

has_bundle_arg=0
for arg in "$@"; do
  if [[ "${arg}" == "--bundle" ]]; then
    has_bundle_arg=1
    break
  fi
done

if [[ "${has_bundle_arg}" -eq 0 && -z "${BUNDLE}" ]]; then
  mapfile -t bundles < <(find "${PACKAGE_ROOT}/bundles" -maxdepth 1 -type f -name 'harbor-release-*.tar.gz' | sort)
  if [[ "${#bundles[@]}" -eq 0 ]]; then
    echo "no bundled harbor-release-*.tar.gz found under ${PACKAGE_ROOT}/bundles" >&2
    exit 1
  fi
  BUNDLE="${bundles[$((${#bundles[@]} - 1))]}"
fi

if [[ "${has_bundle_arg}" -eq 1 ]]; then
  exec bash "${INSTALLER}" "$@"
fi

exec bash "${INSTALLER}" --bundle "${BUNDLE}" "$@"
EOF
python3 - "${PACKAGE_ROOT}/usr/sbin/install-harborbeacon-release" "${PACKAGE_NAME}" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.write_text(
    path.read_text(encoding="utf-8").replace("__HARBOR_DEB_PACKAGE_NAME__", sys.argv[2]),
    encoding="utf-8",
)
PY

cat > "${PACKAGE_ROOT}/usr/sbin/verify-harborbeacon-release" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

PACKAGE_ROOT="/usr/lib/__HARBOR_DEB_PACKAGE_NAME__"
VERIFIER="${PACKAGE_ROOT}/install/verify_release_bundle.py"
BUNDLE="${HARBOR_RELEASE_BUNDLE:-}"

if [[ ! -f "${VERIFIER}" ]]; then
  echo "HarborBeacon release verifier not found: ${VERIFIER}" >&2
  exit 1
fi

if [[ -z "${BUNDLE}" ]]; then
  mapfile -t bundles < <(find "${PACKAGE_ROOT}/bundles" -maxdepth 1 -type f -name 'harbor-release-*.tar.gz' | sort)
  if [[ "${#bundles[@]}" -eq 0 ]]; then
    echo "no bundled harbor-release-*.tar.gz found under ${PACKAGE_ROOT}/bundles" >&2
    exit 1
  fi
  BUNDLE="${bundles[$((${#bundles[@]} - 1))]}"
fi

exec python3 "${VERIFIER}" "${BUNDLE}" "$@"
EOF
python3 - "${PACKAGE_ROOT}/usr/sbin/verify-harborbeacon-release" "${PACKAGE_NAME}" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.write_text(
    path.read_text(encoding="utf-8").replace("__HARBOR_DEB_PACKAGE_NAME__", sys.argv[2]),
    encoding="utf-8",
)
PY

cat > "${PACKAGE_DOC_DIR}/README.HarborOS.md" <<EOF
# HarborBeacon HarborOS Release Debian Package

This package is a HarborOS release carrier. It installs one verified
harbor-release bundle plus the installer/verifier helper scripts.
It does not start or restart HarborBeacon services during dpkg installation.

Installed payload:

- \`/usr/lib/${PACKAGE_NAME}/bundles/${BUNDLE_FILENAME}\`
- \`/usr/lib/${PACKAGE_NAME}/install/install_harboros_release.sh\`
- \`/usr/lib/${PACKAGE_NAME}/install/verify_release_bundle.py\`
- \`/usr/sbin/install-harborbeacon-release\`
- \`/usr/sbin/verify-harborbeacon-release\`

HarborOS image or first-boot integration should run:

\`\`\`bash
sudo verify-harborbeacon-release --require-execute
sudo install-harborbeacon-release \\
  --install-root /var/lib/harborbeacon-agent-ci \\
  --writable-root /mnt/software/harborbeacon-agent-ci
\`\`\`

The underlying release bundle still owns systemd unit rendering, env-file
generation, bundled ffmpeg/ffprobe installation, service restart, and rollback.
EOF

find "${PACKAGE_ROOT}" -type d -exec chmod 0755 {} +
find "${PACKAGE_ROOT}" -type f -exec chmod 0644 {} +
chmod 0755 \
  "${PACKAGE_ROOT}/usr/sbin/install-harborbeacon-release" \
  "${PACKAGE_ROOT}/usr/sbin/verify-harborbeacon-release"
find "${PACKAGE_LIB_DIR}/install" -type f -name '*.sh' -exec chmod 0755 {} +

INSTALLED_SIZE="$(du -sk "${PACKAGE_ROOT}" | awk '{print $1}')"
cat > "${PACKAGE_ROOT}/DEBIAN/control" <<EOF
Package: ${PACKAGE_NAME}
Version: ${DEB_VERSION}
Section: admin
Priority: optional
Architecture: amd64
Maintainer: ${MAINTAINER}
Installed-Size: ${INSTALLED_SIZE}
Depends: bash, python3, tar, coreutils, systemd
Description: HarborBeacon release bundle for HarborOS
 Carries one verified HarborBeacon/HarborGate/Harbor Assistant release bundle
 for HarborOS image and first-boot integration. The package installs helper
 commands but intentionally leaves service installation and restart to the
 bundled HarborOS installer.
EOF

DEB_PATH="${OUTPUT_DIR}/${PACKAGE_NAME}_${DEB_VERSION}_amd64.deb"
dpkg-deb --build --root-owner-group "${PACKAGE_ROOT}" "${DEB_PATH}"
sha256sum "${DEB_PATH}" > "${DEB_PATH}.sha256"

echo "Built ${DEB_PATH}"
