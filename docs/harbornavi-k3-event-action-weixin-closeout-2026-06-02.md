# HarborNavi K3 Event-Action Weixin Closeout - 2026-06-02

## Summary

HarborNavi K3 now has a candidate event-to-action loop for the household
assistant surface:

- Camera events can be summarized, inspected, and sent to the default
  notification target.
- Low-risk Home Assistant actions can be executed through one shared allowlist.
- Weixin `general.message` is constrained to a deterministic household entry
  menu instead of a general internet chatbot.
- Diagnostics include redacted workflow evidence for event notification,
  Home Assistant service actions, notification target readiness, and Gate
  delivery observability.
- RTSP short-clip capture is video-only for MP4 packaging, avoiding TP-Link /
  Tapo G.711 A-law audio failures.

HarborGate v2.0 remains unchanged. Gate owns Weixin transport, route keys,
platform credentials, delivery, and delivery hints. HarborBeacon owns business
interpretation, action validation, audit, artifacts, and redacted evidence.
Group chat remains out of scope.

## Candidate Package

- K3 target: `192.168.3.21`.
- Installed Beacon package:
  `0.1.0+harbornavi.k3.weixincliphotfix.r1.20260602.riscv64`.
- Beacon package SHA256:
  `dc7118ee49ad12aeb60902ba11a8d101a87103decefa33b4b5aefb2732f7ea68`.
- Installed WebUI package: `harbornavi-assistant-webui 20260602.18`.
- Installed Gate package: `harboros-im-gate 0.1.0+harbornavi.k3.gate.20260601.2`.
- Services: `harboros-beacon.service`, `harboros-im-gate.service`, `nginx`,
  and `harborlink-dev-k3.service` all active.
- Beacon health: `ok`.

## Live Evidence

Direct v2 turn API smoke through `/api/turns` passed on K3:

| Command | Result |
|---|---|
| `你能干什么` | completed; returned the K3 household-entry capability menu |
| `深圳天气怎么样` | completed; returned explicit boundary text for external realtime data |
| `最近事件` | completed; returned latest redacted vision event summary |
| `通知最新事件` | completed; delivered to the default notification target |
| `开灯` | completed; executed `light.turn_on` for the unique test light |
| `关灯` | completed; executed `light.turn_off` for the unique test light |
| `状态` | completed; reported Weixin connected, default target available, HA synced, camera and event availability |
| `录视频` | needs_input; created a cover-frame image and `camera.clip_confirmation` active frame |
| `录视频 -> 要` | completed; returned a `video/mp4` full-clip artifact with `native_video` delivery hint |

The recording failure root cause was the camera RTSP stream exposing G.711
A-law audio. The old FFmpeg command attempted to stream-copy optional audio into
MP4, causing header write failure. The candidate package maps only the video
track and uses `-an`, so the short clip succeeds without widening the public
task or delivery contract.

## Redaction And Boundary Check

- Recent Beacon/Gate journal scan after the live smoke found `0` hits for RTSP
  URLs, Bearer tokens, API keys, HA token patterns, private keys, password-like
  fields, or local raw-image leaks.
- Recent recording-error scan found `0` hits for the prior MP4 audio/container
  failure patterns.
- Notification and Weixin delivery continue to use HarborGate v2.0 delivery
  contracts. HarborBeacon does not own platform credentials or parse route keys.
- The smoke above is direct v2 turn API evidence. User-origin Weixin private-DM
  screenshots can be added later as product evidence, but the code path and
  contract evidence are not blocked on that screenshot.

## Verification Commands

Beacon local verification for this candidate:

- `cargo fmt --check`
- `cargo check --bin agent-hub-admin-api`
- `cargo test --bin agent-hub-admin-api home_assistant`
- `cargo test --bin agent-hub-admin-api diagnostics`
- `cargo test --lib task_api`
- `cargo test --lib home_assistant`
- `cargo test --lib vision_event`
- `cargo test --lib clip_capture_args_drop_audio_for_mp4_container`

WebUI local verification for this candidate:

- `corepack yarn check:harbor-assistant-i18n`
- `corepack yarn check:harbor-assistant-delivery`
- focused Harbor Assistant specs for event detail, notify state/result, and HA
  supported/unsupported controls
- `corepack yarn build:harbornavi-k3`

## Residual Risk

- True user-origin Weixin private-DM evidence should still be captured before a
  broader external demo.
- Default notification target onboarding remains out of scope; this package only
  reports readiness or blocker state.
- Weather, news, stocks, public internet realtime providers, group chat,
  WebRTC/VPF, Cloud VLM, and four-camera long-stability claims remain out of
  scope for this candidate.
