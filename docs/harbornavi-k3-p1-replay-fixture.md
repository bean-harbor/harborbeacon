# HarborNavi K3 P1 Replay Fixture

Date: 2026-05-30

## Purpose

Make the `.82` three-channel replay relay reproducible for HarborNavi P1 and
EVT reruns. K3 remains the system under test; `.82` only generates replay RTSP
streams so K3 load numbers are not polluted by source generation.

This fixture supports the P1 mixed simulation shape:

- `cam-real-231`: K3 direct to the real low-bitrate H.264 camera stream.
- `cam-sim-1`, `cam-sim-2`, `cam-sim-3`: K3 direct to `.82` MediaMTX replay
  streams on RTSP TCP port `8554`.

The result is load evidence, not four independent camera compatibility
evidence.

## Files

- Runner: `scripts/harbornavi_p1_replay_fixture.sh`
- Example K3 manifest: `docs/examples/harbornavi-k3-4ch-cameras.example.json`
- `.82` fixture root: `/var/tmp/harbornavi-p1`

`/tmp` and `/home` may be mounted `noexec` on `.82`, so the fixture root is
fixed under `/var/tmp/harbornavi-p1`.

## .82 Setup

Copy or checkout this repository on `.82`, then run the helper there.

Record a short local H.264 sample from the real camera:

```bash
export CAM_REAL_231_RTSP='rtsp://...'
export HARBORNAVI_REPLAY_RECORD_SECONDS=300
bash scripts/harbornavi_p1_replay_fixture.sh record-sample
```

Start the relay:

```bash
bash scripts/harbornavi_p1_replay_fixture.sh start
bash scripts/harbornavi_p1_replay_fixture.sh status
```

The helper writes a MediaMTX config that contains only publisher paths. It does
not write camera credentials or raw RTSP source URLs to the config, manifest, or
report. The replay sample file is local evidence and should stay under
`/var/tmp/harbornavi-p1/samples` with `0600` permissions.

Stop after the run:

```bash
bash scripts/harbornavi_p1_replay_fixture.sh stop
```

## K3 Environment

On K3, set the real camera source and the non-secret replay URLs before running
`harbornavi-k3-multi-vision-smoke`:

```bash
export CAM_REAL_231_RTSP='rtsp://...'
export CAM_SIM_1_RTSP='rtsp://192.168.3.82:8554/p1-sim-1'
export CAM_SIM_2_RTSP='rtsp://192.168.3.82:8554/p1-sim-2'
export CAM_SIM_3_RTSP='rtsp://192.168.3.82:8554/p1-sim-3'
```

Use the example manifest as the starting point. It references the environment
variables through `sourceSecretRef`, uses `persistent_ffmpeg`, and enables VLM
sampling only for the real camera.

## Verification

Run a short fixture smoke first:

```bash
harbornavi-k3-multi-vision-smoke \
  --camera-manifest docs/examples/harbornavi-k3-4ch-cameras.example.json \
  --duration-seconds 180 \
  --output-dir /tmp/harbornavi-p1/replay-fixture-smoke
```

Then run the P1 evidence pass:

```bash
harbornavi-k3-multi-vision-smoke \
  --camera-manifest docs/examples/harbornavi-k3-4ch-cameras.example.json \
  --duration-seconds 1800 \
  --output-dir /tmp/harbornavi-p1/replay-fixture-30min
```

Acceptance remains:

- aggregate event success rate `>=99%`
- per-camera p95 snapshot-to-event `<5s`
- sampled real-camera VLM completion `>=95%`
- Beacon and HarborLink remain active
- reports, journals, event store, and cloud payloads contain no RTSP URL, HA
  token, API key, private key, camera credential, upload URL, local snapshot
  path, or raw image bytes
