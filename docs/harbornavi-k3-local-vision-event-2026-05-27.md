# HarborNavi K3 Local Vision Event P0 Report - 2026-05-27

## Scope

- Branch: `harbornavi/k3-local-vision-event-p0`
- HarborBeacon PR: https://github.com/Bean-Harbor/HarborBeacon/pull/17
- HarborNavi tracker: https://github.com/Bean-Harbor/HarborNavi/issues/19
- Goal: validate the K3 Bianbu board as a local camera snapshot/keyframe -> local event -> HarborBeacon audit/event record loop.
- Non-goals in this slice: VPF, cloud VLM fallback, HarborLink, HA/MQTT live delivery, 4-camera P1 load, and real person/pet/package semantic detection.

## Code Changes

- Added `runtime::vision_event` with `LocalVisionEvent`, `SnapshotArtifact`, JSONL ingest, audit projection, HA/MQTT-ready payload projection, and sensitive text rejection.
- Added `POST /api/vision/events` to the local admin API.
- Added `harbornavi-k3-local-vision-smoke` for fixture, HTTP snapshot, and RTSP snapshot smoke tests.
- Added `scripts/build_harbornavi_k3_deb.sh` to build a `riscv64` K3 Debian package containing:
  - `/usr/bin/harboros-beacon`
  - `/usr/bin/harbornavi-k3-local-vision-smoke`
  - `harboros-beacon.service`

## Local And Builder Verification

- Local Windows checks:
  - `cargo test --lib vision_event`: 4 passed.
  - `cargo test --lib model_center -- --test-threads=1`: 13 passed.
  - `cargo run --bin harbornavi-k3-local-vision-smoke -- --fixture --no-post --output-dir target\local-vision-smoke-precommit`: passed.
  - `cargo build --bin harboros-beacon --bin harbornavi-k3-local-vision-smoke`: passed.
- `.197` builder:
  - Rust: `rustc 1.95.0`, `cargo 1.95.0`.
  - Target: `riscv64gc-unknown-linux-gnu`.
  - Linker: `riscv64-linux-gnu-gcc 13.3.0`.
  - `cargo test --lib vision_event`: 4 passed.
  - `cargo test --lib model_center -- --test-threads=1`: 13 passed.
  - Fixture smoke without posting: passed.

## Package

- Package: `harboros-beacon_harbornavi-p0-local-vision-20260527+riscv64_riscv64.deb`
- Version: `0.1.0+harbornavi.p0.localvision.20260527.riscv64`
- Architecture: `riscv64`
- Dependencies: `libc6, openssl, ca-certificates`
- Size: `10560884` bytes
- SHA256: `c466cf29392f5afd29b9e33fa9e4198e44a91648eb9bd940562b2f2d687f9154`
- ELF: RISC-V 64-bit PIE, double-float ABI, dynamic linker `/lib/ld-linux-riscv64-lp64d.so.1`.

## K3 Preflight And Deploy

- Host: K3 Bianbu board at `192.168.3.21`.
- OS: Bianbu `4.0rc2`, kernel `6.18.3-generic`, `riscv64`.
- glibc: `2.43`.
- systemd: `259`.
- Memory: `31 GiB`, no swap.
- Root filesystem free: about `71 GiB`.
- Dependencies present: `openssl`, `ca-certificates`, `ffmpeg`, `ffprobe`, `curl`, `systemctl`.
- Install path: copied the package to `/tmp/harbornavi-p0/` and installed with `dpkg -i`.
- Backup path: `/tmp/harbornavi-p0/backup-local-vision-v2-20260527-105426`.
- Installed package: `harboros-beacon 0.1.0+harbornavi.p0.localvision.20260527.riscv64`.
- Service: `harboros-beacon.service` active and enabled.
- Health: `http://127.0.0.1:4174/healthz` returned HTTP `200`.

## Official Vision Runtime Discovery

- Installed package: `spacemit-onnxruntime 2.0.2+rc5`.
- Runtime command found: `/usr/bin/onnxruntime_perf_test`.
- `onnx_test_runner` help lists `spacemit` as an execution provider.
- Headers include `spine_vision_engine.h`.
- No bundled detection model was found under the probed system paths.
- Conclusion: official runtime candidate exists, but an official person/pet/package detection model recipe is still missing. Current P0 smoke therefore records `present_unverified` and uses CPU snapshot fallback for the event loop.

## Camera Input

- Direct RTSP input from camera `192.168.3.231` succeeded on K3 using the low-resolution H.264 stream.
- Single direct RTSP smoke:
  - HTTP ingest status: `200`.
  - Event type: `motion_like_scene`.
  - JPEG bytes: `22613`.
  - JPEG magic: `ffd8ff`.
  - End-to-end latency: `2254 ms`.
- `.82` snapshot proxy fallback was not needed.

## 30 Minute Local Event Run

- Command shape:
  - `harbornavi-k3-local-vision-smoke --camera-id cam-rtsp-192-168-3-231 --rtsp-url <redacted> --duration-seconds 1800 --interval-seconds 10`
- Output directory: `/tmp/harbornavi-p0/local-vision-rtsp-30min`.
- Observed duration: `1804 s`.
- Total runs: `147`.
- Passed: `147`.
- Failed: `0`.
- Average total latency: `2361 ms`.
- P95 total latency: `2393 ms`.
- Max total latency: `2442 ms`.
- Under 2 seconds: `0 / 147`.
- Under 5 seconds: `147 / 147`.
- Average capture latency: `2357 ms`.
- Average analyze latency: `0 ms`.
- Average JPEG size: `22619` bytes.
- Snapshot files: `147`.
- HarborBeacon event store lines after run: `149`.
- Report size: `263 KiB`; snapshot evidence directory size: `3.8 MiB`.

## Runtime And Safety Checks

- `harboros-beacon.service`: active after the 30 minute run.
- `/healthz`: HTTP `200` after the 30 minute run.
- HarborBeacon RSS after the run: about `10500 KiB`.
- Instantaneous `vmstat` after the run: CPU idle recovered to about `100%`.
- Thermal samples after the run: `59-62 C`.
- `dmesg` scan for `oom|panic|segfault|tcm|killed process`: `0`.
- `journalctl -u harboros-beacon.service` scan for panic/OOM/secret patterns: `0`.
- Report scan:
  - `rtsp://`: `0`
  - `ha_token`: `0`
  - `home_assistant_token`: `0`
  - `camera_credential`: `0`
  - `api_key`: `0`
  - `authorization: bearer`: `0`
  - `sk-`: `0`
  - camera password marker: `0`

## Conclusion

- Pass: K3 can run the deployed HarborBeacon service, ingest local vision events, pull a real RTSP camera snapshot directly, and sustain a 30 minute single-camera local event loop without crashes, OOM, TCM errors, or secret leakage in the report.
- Pass: The single-camera path meets the P0 acceptable latency line of `<5s`.
- Miss: It does not meet the target latency line of `<2s`; observed average is about `2.36s`.
- Gap: This is not yet a real semantic detector for `person / pet / vehicle / package`. The current analyzer is a CPU snapshot fallback that emits `motion_like_scene`.
- Next decision: keep K3 on the main route for the local event pipeline, but the next P0 slice must replace the fallback analyzer with an official SpacemiT/Bianbu ONNX detection recipe or a measured CPU detector baseline, then rerun the same 30 minute gate.
