# HarborNavi P1 Family Summary And Audit

This P1 slice keeps family-readable event interpretation inside HarborBeacon.
It projects an existing `StoredLocalVisionEvent` into a metadata-only
`FamilyEventSummary` plus an audit record.

## Boundary

- Input: stored `LocalVisionEvent` fixture or event-store record.
- Output: `FamilyEventSummary` and `audit_kind=local_vision_event.family_summary_built`.
- VLM active: use the local VLM summary.
- VLM degraded: fall back to the YOLO/local event summary.
- No cloud LLM, HarborGate, HarborLink, HarborCloud, image upload, or IM delivery is called.

The output must not contain RTSP URLs, HA tokens, API keys, private keys,
camera credentials, local snapshot paths, upload URLs, or raw image bytes.

## Smoke

Use a stored event fixture:

```powershell
cargo run --bin harbornavi-family-summary-audit-smoke -- --event-json .\fixtures\stored-local-vision-event.json
```

Expected evidence:

- `ok=true`
- `classification=family-summary-audit-ready`
- `family_summary.source=yolo|vlm|degraded`
- `audit_record.audit_kind=local_vision_event.family_summary_built`
- `audit_record.metadata_only=true`
- `audit_record.secret_scan=clean`

## Live Follow-Up

When K3 and the onsite network are reachable again, replay this projection
against the latest K3 event-store record after a live camera run. Live HA/MQTT
publish remains tracked separately in HarborNavi issue `#9`.
