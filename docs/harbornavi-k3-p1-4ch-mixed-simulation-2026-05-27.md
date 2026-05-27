# HarborNavi K3 P1 Four-Channel Mixed Simulation

Date: 2026-05-27

## Purpose

Validate the first P1 multi-camera load shape when only one physical camera is available:

- `cam-real-231`: real low-bitrate H.264 RTSP camera.
- `cam-sim-1`, `cam-sim-2`, `cam-sim-3`: replay RTSP streams hosted outside K3.

This is a multi-channel load and stability test. It does not replace a later compatibility pass with four different real camera models.

## Implementation

- Added `harbornavi-k3-multi-vision-smoke`.
- The multi runner accepts `--camera-manifest cameras.json`.
- Each camera is executed as an isolated `harbornavi-k3-local-vision-smoke` worker.
- The aggregate report records per-camera success rate, p95 total latency, capture latency, detector latency, event ingest latency, detection counts, and a result classification:
  - `pass`
  - `capacity-risk`
  - `capture-bottleneck`
  - `analyzer-bottleneck`
  - `system-risk`
- The aggregate report excludes raw RTSP URLs, camera credentials, image bytes, and local snapshot paths.
- The single-camera runner now supports `--redact-paths` for P1 report generation.

Example manifest: `docs/examples/harbornavi-k3-4ch-cameras.example.json`.

## K3 Test Shape

- Baseline: single real camera for 10 minutes.
- P1 mixed run: four cameras for 30 minutes, one frame per camera every 10 seconds.
- Expected event count: about `4 * 180 = 720`, with minor variation allowed due processing time.
- Pass line:
  - aggregate success rate `>= 99%`
  - per-camera p95 snapshot-to-event `< 5s`
  - no Beacon/Link crash, OOM, panic, or reboot
  - HarborLink `dev-k3` remains online and responds to cloud ping

## Verification

Local verification completed:

- `cargo test --bin harbornavi-k3-multi-vision-smoke`: passed.
- `cargo test --bin harbornavi-k3-local-vision-smoke`: passed.
- `cargo build --bin harbornavi-k3-multi-vision-smoke --bin harbornavi-k3-local-vision-smoke`: passed.
- Fixture multi-camera smoke with two in-process fixture cameras: passed.

K3 deployment and live four-channel evidence are recorded in the HarborNavi P1 report.
