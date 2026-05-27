#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target="${RUST_TARGET:-riscv64gc-unknown-linux-gnu}"
deb_arch="${DEB_ARCH:-riscv64}"
date_stamp="${HARBORNAVI_BUILD_DATE:-$(date +%Y%m%d)}"
release_label="${RELEASE_VERSION:-harbornavi-p0-local-vision-${date_stamp}+riscv64}"
debian_version="${DEBIAN_VERSION:-0.1.0+harbornavi.p0.localvision.${date_stamp}.riscv64}"
out_dir="${OUT_DIR:-${repo_root}/dist/harbornavi-k3-debs}"
build_root="${repo_root}/target/harbornavi-k3-deb"
pkg_name="harboros-beacon_${release_label}_${deb_arch}"
pkg_dir="${build_root}/${pkg_name}"

if [[ "$target" != "riscv64gc-unknown-linux-gnu" ]]; then
  echo "error: K3 package target must be riscv64gc-unknown-linux-gnu, got ${target}" >&2
  exit 2
fi

if [[ "$deb_arch" != "riscv64" ]]; then
  echo "error: K3 Debian architecture must be riscv64, got ${deb_arch}" >&2
  exit 2
fi

command -v dpkg-deb >/dev/null || {
  echo "error: dpkg-deb is required" >&2
  exit 2
}

command -v riscv64-linux-gnu-gcc >/dev/null || {
  echo "error: riscv64-linux-gnu-gcc is required" >&2
  exit 2
}

export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER="${CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER:-riscv64-linux-gnu-gcc}"

cargo build --release --target "$target" --bin harboros-beacon
cargo build --release --target "$target" --bin harbornavi-k3-local-vision-smoke

rm -rf "$build_root"
mkdir -p "$pkg_dir/DEBIAN"
mkdir -p "$pkg_dir/usr/bin"
mkdir -p "$pkg_dir/etc/systemd/system"
mkdir -p "$pkg_dir/usr/share/doc/harboros-beacon"

cp "target/${target}/release/harboros-beacon" "$pkg_dir/usr/bin/harboros-beacon"
cp "target/${target}/release/harbornavi-k3-local-vision-smoke" "$pkg_dir/usr/bin/harbornavi-k3-local-vision-smoke"
chmod 0755 "$pkg_dir/usr/bin/harboros-beacon" "$pkg_dir/usr/bin/harbornavi-k3-local-vision-smoke"

cp debian/harboros-beacon.service "$pkg_dir/etc/systemd/system/harboros-beacon.service"

sed \
  -e "s/VERSION_PLACEHOLDER/${debian_version}/g" \
  -e "s/ARCH_PLACEHOLDER/${deb_arch}/g" \
  debian/control > "$pkg_dir/DEBIAN/control"
printf 'X-HarborNavi-Version: %s\n' "$release_label" >> "$pkg_dir/DEBIAN/control"

cp debian/postinst "$pkg_dir/DEBIAN/postinst"
cp debian/prerm "$pkg_dir/DEBIAN/prerm"
chmod 0755 "$pkg_dir/DEBIAN/postinst" "$pkg_dir/DEBIAN/prerm"

cat > "$pkg_dir/usr/share/doc/harboros-beacon/harbornavi-k3-package.txt" <<EOF
HarborNavi K3 local vision event package
release_label=${release_label}
debian_version=${debian_version}
rust_target=${target}
deb_arch=${deb_arch}
EOF

mkdir -p "$out_dir"
dpkg-deb --build "$pkg_dir" "${out_dir}/${pkg_name}.deb"

sha256sum "${out_dir}/${pkg_name}.deb" > "${out_dir}/${pkg_name}.deb.sha256"
file "target/${target}/release/harboros-beacon" > "${out_dir}/${pkg_name}.file.txt"
dpkg-deb --info "${out_dir}/${pkg_name}.deb" > "${out_dir}/${pkg_name}.info.txt"
dpkg-deb --contents "${out_dir}/${pkg_name}.deb" > "${out_dir}/${pkg_name}.contents.txt"

cat <<EOF
package=${out_dir}/${pkg_name}.deb
sha256=${out_dir}/${pkg_name}.deb.sha256
info=${out_dir}/${pkg_name}.info.txt
contents=${out_dir}/${pkg_name}.contents.txt
file=${out_dir}/${pkg_name}.file.txt
release_label=${release_label}
debian_version=${debian_version}
EOF
