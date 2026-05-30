# HarborNavi P1 HA/MQTT Event Contract

## Status

Offline contract smoke for HarborNavi issue `#9`.

K3, `.82`, and the camera are currently unreachable from the Windows workspace,
so this pass validates the Beacon-owned event projection without connecting to
Home Assistant or an MQTT broker. Live HA/MQTT publish remains follow-up
evidence.

## Payload

`LocalVisionEvent` is projected to a metadata-only HA/MQTT payload:

- `event_id`
- `camera_id`
- `event_type`
- `confidence`
- `labels`
- `summary`
- `started_at`
- `analyzer`
- `latency_ms`
- `vlm_status`

The payload intentionally omits snapshot artifacts, local file paths, image
bytes, upload URLs, RTSP URLs, HA tokens, API keys, private keys, and camera
credentials.

## Offline Smoke

Use a stored `LocalVisionEvent` fixture:

```powershell
cargo run --bin harbornavi-ha-mqtt-event-contract-smoke -- `
  --event-json C:\path\to\stored-event.json `
  --topic-hint harbornavi/dev-k3/local_vision/events
```

Expected result:

- `ok=true`
- `classification=ha-mqtt-contract-ready`
- `audit_record.audit_kind=local_vision_event.ha_mqtt_payload_built`
- `audit_record.secret_scan=clean`
- `ha_mqtt_payload` contains only the contract fields above

## Failure Classification

- `ha-unavailable`: Home Assistant cannot receive or process the event.
- `mqtt-unavailable`: MQTT broker or publish path is unavailable.
- `permission-denied`: allowlist or service policy blocks the action.
- `payload-secret-risk`: payload contains sensitive material.
- `contract-regression`: payload shape drifts from the P1 contract.

## Live Follow-up

When K3 is reachable again, rerun with the latest real event and publish through
the selected HA/MQTT path while verifying Beacon health and HarborLink heartbeat
remain stable.
