# HarborNavi K3 Deployment Report - 2026-05-27

## Summary

- Target: K3 Bianbu board at `192.168.3.21`.
- Deployment mode: Bianbu userland `.deb`, no HarborOS image flash.
- HarborBeacon branch: `harbornavi/mlp-vpf-p0`.
- Deployed commit: `b7fcfd5 Add HarborNavi K3 riscv64 package path`.
- Result: passed. `harboros-beacon.service` runs on K3 as a systemd service,
  binds only `127.0.0.1:4174`, returns `/healthz` HTTP 200, and the MLP/VPF
  guard smoke passes on-device.

This deployment proves the P0 guard slice can run as the real HarborBeacon
service on K3. It does not deploy the full VPF detector engine yet, and it does
not configure a cloud API key.

## Builder

- Builder: `.197`.
- Source path:
  `/home/harbor-innovations/src/HarborBeacon-harbornavi-mlp-vpf-p0`.
- GitHub access from `.197` was unavailable during this run. The branch was
  updated through a local `git bundle`, then fast-forwarded from `5ce91a3` to
  `b7fcfd5`.

Toolchain:

```text
rustc 1.95.0 (59807616e 2026-04-14)
cargo 1.95.0 (f2d3ce0bd 2026-03-21)
riscv64gc-unknown-linux-gnu
riscv64-linux-gnu-gcc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
```

Host validation:

```text
cargo test --lib privacy --quiet
privacy: 5 passed

cargo test --lib model_center -- --test-threads=1
model_center: 17 passed

cargo run --bin harbornavi-k3-guard-smoke -- --output /tmp/harbornavi-k3-guard-smoke-host.json
guard smoke: ok=true
```

Package build:

```bash
OUT_DIR=/home/harbor-innovations/artifacts/harbornavi-k3-debs-20260527 \
RELEASE_VERSION=harbornavi-p0-20260527+riscv64 \
HARBORNAVI_BUILD_DATE=20260527 \
bash scripts/build_harbornavi_k3_deb.sh
```

Artifact:

```text
package=/home/harbor-innovations/artifacts/harbornavi-k3-debs-20260527/harboros-beacon_harbornavi-p0-20260527+riscv64_riscv64.deb
size=10778088 bytes
sha256=125915fb243b9e0b555f457b5712441c5f746c60d8ec76c55426b69a9af7ef83
version=0.1.0+harbornavi.p0.20260527.riscv64
architecture=riscv64
depends=libc6, openssl, ca-certificates
x-harbornavi-version=harbornavi-p0-20260527+riscv64
```

Binary type:

```text
ELF 64-bit LSB pie executable, UCB RISC-V, RVC, double-float ABI,
dynamically linked, interpreter /lib/ld-linux-riscv64-lp64d.so.1,
for GNU/Linux 4.15.0, not stripped
```

## K3 Preflight

Target state before install:

```text
hostname=harbor-s3
os=Bianbu 4.0rc2
kernel=Linux 6.18.3-generic riscv64
glibc=2.43
systemd=259
rootfs=/dev/sda3 117G total, 71G available
memory=31Gi total, about 30Gi available
port_4174=not occupied
openssl=installed
ca-certificates=installed
ffmpeg=installed
ffprobe=installed
harboros-beacon.service=inactive
```

Install workspace:

```text
/tmp/harbornavi-p0
```

Backup directory:

```text
/tmp/harbornavi-p0/backup-20260527-095732
```

The package SHA-256 was verified on K3 before install.

## Install

Install command:

```bash
sudo dpkg -i /tmp/harbornavi-p0/harboros-beacon_harbornavi-p0-20260527+riscv64_riscv64.deb
sudo systemctl daemon-reload
sudo systemctl enable harboros-beacon.service
sudo systemctl restart harboros-beacon.service
```

Package state:

```text
install ok installed 0.1.0+harbornavi.p0.20260527.riscv64 riscv64
```

Service state:

```text
harboros-beacon.service: active
Main PID: 212123
Exec: /usr/bin/harboros-beacon --bind 127.0.0.1:4174 ...
RSS: about 12 MiB
```

The service listens only on loopback:

```text
LISTEN 0 128 127.0.0.1:4174 0.0.0.0:*
```

Health check:

```text
GET http://127.0.0.1:4174/healthz
HTTP/1.1 200 OK
status=ok
service=harborbeacon
topology=single-port
```

## Guard Smoke

On-device command:

```bash
/usr/bin/harbornavi-k3-guard-smoke --output /tmp/harbornavi-p0/guard-smoke.json
```

Result:

```text
ok=true
vlm_cloud_requires_vpf_manifest: ok=true, status=blocked, code=vpf_manifest_required
embedding_route_does_not_select_cloud: ok=true, status=disabled
semantic_router_does_not_select_cloud: ok=true, status=disabled
llm_cloud_fallback_redacted_audit: ok=true, status=active,
  privacy_transform=redacted_text,
  audit_prompt_storage=redacted,
  has_api_key_field=false
```

The default running K3 model policies also show the P0-safe state:

```text
retrieval.embed: strict_local
retrieval.ocr: strict_local
retrieval.vision_summary: strict_local, degraded, cloud_fallback=false
semantic.router: local-first NSP route
retrieval.answer: allow_redacted_cloud for text answer fallback
```

## Log Scan

Evidence files on K3:

```text
/tmp/harbornavi-p0/guard-smoke.json
/tmp/harbornavi-p0/model-policies.json
/tmp/harbornavi-p0/model-endpoints.json
/tmp/harbornavi-p0/journal-harboros-beacon.txt
/tmp/harbornavi-p0/dmesg-tail.txt
```

Scan result:

```text
runtime_log_match_count=0
secret_log_match_count=0
```

Patterns checked included panic, segfault, OOM, kernel panic, API key, HA token,
RTSP URL, private key, camera credential, and upload URL.

## Notes

- The first post-install follow-up command attempted `systemctl daemon-reload`
  without sudo after `dpkg -i` had completed. That user-level command was denied
  by systemd; rerunning `daemon-reload`, `enable`, and `restart` with sudo
  completed successfully.
- No K3 rollback was needed because the installed package, service start,
  health check, guard smoke, and log scan all passed.
- The real VPF detector engine is still the next implementation phase.
