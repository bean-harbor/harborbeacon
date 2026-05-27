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
- Official Bianbu AI Demo Zoo recipe was found after the initial local probe:
  - Repository: `https://gitee.com/bianbu/spacemit-demo.git`
  - Candidate: `examples/CV/yolov8`
  - Model: `yolov8n_192x320.q.onnx`
  - Runtime API: `providers=["SpaceMITExecutionProvider"]`
  - Postprocess: YOLOv8 DFL, confidence filter, per-class NMS, COCO labels.
- No bundled detection model was installed locally by default; the recipe downloads model/data from `archive.spacemit.com`.
- Conclusion: the official detection recipe exists, but K3 acceleration still needs a TCM runtime fix before the SpaceMIT EP path can be counted as the accelerated baseline.

## Official YOLOv8 Recipe Probe

- Workspace: `/home/harbor/harbornavi-p0/official-yolov8`.
- Dependency added on K3: `python3-opencv`.
- Python runtime:
  - `onnxruntime 1.24.2+spacemit.a1`
  - `python3-spacemit-ort 2.0.2+rc5`
  - `cv2 4.10.0`
  - Available providers: `SpaceMITExecutionProvider`, `CPUExecutionProvider`
- Downloaded official assets:
  - `yolov8n_192x320.q.onnx`, size about `1.9 MiB`, sha256 `d4bf61db2a0925a0126052212479ff5044b621b12c6793420e085d36ae6b5438`
  - `yolov8n_320x320.q.onnx`, size about `1.9 MiB`, sha256 `fcfd8d16a5e6a4b03c438d5b634c1c1f7d2449ab60eb3d328759aae4ae715b8e`
  - COCO `label.txt`, sha256 `bd17f1ee35d5f3c862a4894605855abbb9dda4b0621fdb0ac4c2c8c7bb7e730a`
- SpaceMIT EP result:
  - Official Python demo and a minimal single-thread ONNX session both abort.
  - Error: `mmap tcm block: Invalid argument` and `tcm buffer acquire failed for core id 0/1`.
  - `spine_tcm` query reports version `0.2.0`, `available=0`, `blk_size=393216`, `blk_num=8`, `is_fake_tcm=1`, and block physical addresses as `0`.
  - Clearing `/dev/shm/tcm_sync_standalone` did not resolve the issue.
- CPU provider result:
  - Minimal zero-input run succeeded.
  - Model load: about `225-239 ms`.
  - Single inference on zero input: about `86 ms`, RSS about `59 MiB`.
- Detection quality smoke on CPU:
  - Official test image detected three `person` objects, total `45.59 ms`.
  - Real K3 camera snapshot detected one `refrigerator`, total `42.62 ms`.
- Offline CPU detector pass over the 147 snapshots captured in the 30 minute run:
  - Snapshot count: `147`.
  - Failures: `0`.
  - Average detector latency: `41.55 ms`.
  - P50: `41.52 ms`.
  - P95: `41.68 ms`.
  - Max: `44.50 ms`.
  - RSS: about `185 MiB`.
  - Top detections: `refrigerator` in 143 images, `person` in 2 images, `bottle` in 1 image.
  - Evidence: `/tmp/harbornavi-p0/yolov8-cpu-147-snapshot-summary.json`.

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
- Pass: Official YOLOv8n INT8 detection recipe exists and the model runs correctly with ONNX CPU provider on K3.
- Pass: CPU detector latency is small relative to RTSP capture latency in the current P0 path.
- Blocker: `SpaceMITExecutionProvider` is not usable on this K3 image yet because `spine_tcm` reports unavailable/fake TCM and the provider aborts on TCM buffer acquisition.
- Gap: `package` is not a COCO label. P0 can map `person`, `cat/dog`, and `vehicle` directly; package needs a proxy label or a later custom detector.
- Next decision: keep K3 on the main route for the local event pipeline. For the next implementation slice, wire YOLOv8 CPU provider as the measured fallback detector, and in parallel ask SpacemiT/Bianbu for the K3 TCM/SpaceMIT EP fix so the accelerated path can be re-tested.
