# HarborAssistant Offline Delivery Runbook

Status: package-ready; artifacts staged on `.82`; live deb install blocked by
TrueNAS rootfs protection and active `/usr` system extensions.

Artifact id: `harborassistant-live-solidify-20260529`
Version: `20260529+harborassistant.live.solidify`
Builder output:
`/home/harbor-innovations/artifacts/harborassistant-live-solidify-20260529/output`
on build host `192.168.1.197`.

## Current Blockers

### Builder path

HarborOS `.82` is reachable from the workstation, but it is still not reachable
from the builder path. Workstation relay staging is possible, but `.197 -> .82`
is not a valid direct deploy path yet.

Observed from `.197` on 2026-05-30 at 20:27 CST:

- route: `192.168.3.82 via 192.168.1.1 dev enp11s0 src 192.168.1.197`
- ping: 2 sent, 0 received
- TCP: `22`, `80`, `443`, `4174`, and `8787` all timed out
- HTTP `80` returned `000` from curl after timeout

### Package install guard

The verified artifacts were staged on `.82` under:

```text
/var/tmp/harborassistant-live-solidify-20260529/
```

`sha256sum -c SHA256SUMS` passed on `.82` for all four artifacts. Rollback
evidence was created before install under:

```text
/var/backups/harborassistant-live-solidify-20260529/20260530-203627/
```

The install did not modify packages because the first `dpkg -i` exited before
unpacking:

```text
Package management tools are disabled on TrueNAS appliances.
```

On this target, `/usr/local/bin/dpkg` points to `pkg_mgmt_disabled`, root and
`/usr` ZFS datasets are `readonly=on`, and `/usr` is overlaid by systemd-sysext
extensions: `amd-xrt-npu`, `intel-bmg-firmware`, and `nvidia`. This matches the
known `.82` package-management constraint recorded on 2026-05-12.

Do not run `disable-rootfs-protection`, unmerge system extensions, chmod the
real `dpkg`, or manually extract deb payloads as a normal HarborAssistant deploy
step. Choose one of these before the next install attempt:

- build a HarborOS ISO/update/boot-environment artifact that installs the debs
  into the image;
- schedule an explicit rootfs-protection maintenance step for `.82`, including
  sysext impact and rollback;
- keep the current live hotfix only as rollback evidence while the package path
  is fixed upstream.

## Artifact Evidence

`SHA256SUMS` on `.197`:

```text
e7cb3176c987f42dbc3518eed924687530a507e8c92a0cfb32b77f324473990f  harboros-beacon_20260529+harborassistant.live.solidify_harborassistant-live-solidify-20260529_linux_amd64.deb
43c8a9e03e3a88279ccff0ebb9cd597ce22471870fca21cd15b7c261cf031b3c  harboros-im-gate_20260529+harborassistant.live.solidify_harborassistant-live-solidify-20260529_linux_amd64.deb
f77d7dad3d84e78f11aed70f000d11d7f4f4ddd7d339cc7535831ad02efbb3c3  truenas-webui_20260529+harborassistant.live.solidify_harborassistant-live-solidify-20260529_all.deb
989aa16a2d90001ad5b2eaedfe101ad898d95ab7e677261495a819380034d3db  truenas-webui_harborassistant-live-solidify-20260529_dist.tgz
```

Package dry validation already passed on `.197`:

- `scripts/verify_harborassistant_offline_delivery.sh` passed against the
  builder output directory.
- `sha256sum -c SHA256SUMS` passed for all four artifacts.
- `harboros-beacon` deb control: package `harboros-beacon`, version
  `20260529+harborassistant.live.solidify`, arch `amd64`.
- Beacon deb contains `/usr/bin/harboros-beacon`,
  `/etc/systemd/system/harboros-beacon.service`, and Candle bootstrap
  `bootstrap-llm/model.safetensors`.
- `harboros-im-gate` deb control: package `harboros-im-gate`, version
  `20260529+harborassistant.live.solidify`, arch `amd64`.
- Gate deb contains `/usr/bin/harboros-im-gate` and
  `/etc/systemd/system/harboros-im-gate.service`.
- `truenas-webui` deb control: package `truenas-webui`, version
  `20260529+harborassistant.live.solidify`, arch `all`.
- WebUI deb contains `/usr/share/truenas/webui/index.html`.
- WebUI dist JS refs: `/api/beacon=70`, `/api/harbor-gate=4`,
  `/api/harbor-beacon=2`, `/api/harbor-assistant=0`.

Re-run the package-only gate on the builder with:

```bash
cd /home/harbor-innovations/src/HarborBeacon-harborassistant-live-solidify-20260529
bash scripts/verify_harborassistant_offline_delivery.sh \
  /home/harbor-innovations/artifacts/harborassistant-live-solidify-20260529/output
```

This gate does not contact `.82`.

## Install Sequence When `.82` Is Package-Writable

Use the target registry before any live action. Confirm the HarborOS target and
credentials for the day, then transfer only the verified artifacts from `.197`
to `.82`.

On `.82`, stage packages under:

```text
/var/tmp/harborassistant-live-solidify-20260529/
```

Preflight:

```bash
cd /var/tmp/harborassistant-live-solidify-20260529
sha256sum -c SHA256SUMS
dpkg-query -W harboros-beacon harboros-im-gate truenas-webui || true
systemctl is-active nginx harboros-beacon.service harboros-im-gate.service || true
findmnt -T / -o TARGET,SOURCE,FSTYPE,OPTIONS
findmnt -T /usr -o TARGET,SOURCE,FSTYPE,OPTIONS
systemd-sysext status || true
command -v dpkg
ls -l "$(command -v dpkg)" /usr/bin/dpkg
```

Continue with `dpkg -i` only when the target is intentionally package-writable.
If `command -v dpkg` resolves to `/usr/local/bin/dpkg -> pkg_mgmt_disabled`,
stop and fix the delivery path instead of extracting debs by hand.

Create rollback evidence before installing:

```bash
sudo install -d -m 0700 /var/backups/harborassistant-live-solidify-20260529
sudo cp -a /etc/nginx/nginx.conf \
  /var/backups/harborassistant-live-solidify-20260529/nginx.conf.before
readlink -f /mnt/.ix-apps/harbor-webui-live/current \
  | sudo tee /var/backups/harborassistant-live-solidify-20260529/webui-live-current.txt
sudo tar -C / -cpf /var/backups/harborassistant-live-solidify-20260529/runtime-files.tar \
  usr/share/truenas/webui \
  usr/bin/harboros-beacon \
  usr/bin/harboros-im-gate \
  etc/systemd/system/harboros-beacon.service \
  etc/systemd/system/harboros-im-gate.service \
  etc/default/harboros-beacon \
  etc/default/harboros-im-gate 2>/dev/null || true
dpkg-query -W harboros-beacon harboros-im-gate truenas-webui \
  | sudo tee /var/backups/harborassistant-live-solidify-20260529/dpkg-before.txt
```

Install order:

```bash
sudo dpkg -i truenas-webui_20260529+harborassistant.live.solidify_harborassistant-live-solidify-20260529_all.deb
sudo dpkg -i harboros-im-gate_20260529+harborassistant.live.solidify_harborassistant-live-solidify-20260529_linux_amd64.deb
sudo dpkg -i harboros-beacon_20260529+harborassistant.live.solidify_harborassistant-live-solidify-20260529_linux_amd64.deb
sudo systemctl daemon-reload
sudo nginx -t
sudo systemctl restart harboros-im-gate.service harboros-beacon.service nginx
```

The successful state must serve Harbor Assistant from the package path
`/usr/share/truenas/webui`. The live hotfix path
`/mnt/.ix-apps/harbor-webui-live/current` is rollback evidence only.

## Live Acceptance Gate

After install:

```bash
systemctl is-active nginx harboros-beacon.service harboros-im-gate.service
curl -fsS http://127.0.0.1:4174/healthz
curl -fsS -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8787/api/setup/status
curl -fsS -o /dev/null -w '%{http_code}\n' http://127.0.0.1/ui/harbor-assistant
curl -fsS -o /dev/null -w '%{http_code}\n' http://127.0.0.1/api/beacon/state
curl -fsS -o /dev/null -w '%{http_code}\n' http://127.0.0.1/api/harbor-gate/api/setup/status
```

Then verify the HarborAssistant product tabs:

- Home Assistant: status, test, sync, entities, services, and install plan.
- Models: runtime list, policy save, endpoint test, and download status.
- Search: document/image/video search and preview URLs.
- Camera/DVR: only after confirming the camera target for the day; run RTSP
  check, snapshot, short recording, timeline, share, and revoke.
- Gate: setup/manage/status pages under `/api/harbor-gate/*`.
- Rules: create a temporary review, enable, pause, discard, then confirm it is
  gone from pending state.

## Rollback

Rollback must preserve the package/service boundary. Do not use rollback to add
new WebUI local state, HarborGate business semantics, or Beacon-owned IM
transport.

If install fails before service restart:

```bash
sudo tar -C / -xpf /var/backups/harborassistant-live-solidify-20260529/runtime-files.tar
sudo cp -a /var/backups/harborassistant-live-solidify-20260529/nginx.conf.before /etc/nginx/nginx.conf
sudo systemctl daemon-reload
sudo nginx -t
sudo systemctl restart harboros-im-gate.service harboros-beacon.service nginx
```

If install succeeds but live acceptance fails:

1. Capture `journalctl -u harboros-beacon.service -u harboros-im-gate.service -u nginx --since -30m`.
2. Restore the backup tar and nginx config above.
3. Re-check `/mnt/.ix-apps/harbor-webui-live/current`; it remains the previous
   hotfix WebUI rollback point.
4. If previous approved debs are available, reinstall them with `dpkg -i`;
   otherwise keep the restored binary/service backup only as emergency rollback
   and produce a replacement package before the next attempt.

## Boundary Notes

- WebUI talks to Beacon through `/api/beacon/*`.
- `/api/harbor-beacon/*` remains a Beacon compatibility alias only.
- WebUI talks to Gate through `/api/harbor-gate/*`.
- HarborGate owns IM setup/manage/status and transport only.
- HarborBeacon owns HarborAssistant product state, model/rules/HA/camera
  management, audit, and artifacts.
- HarborCloud, HarborLink, and harbor-dock are not part of this offline
  solidification lane.
