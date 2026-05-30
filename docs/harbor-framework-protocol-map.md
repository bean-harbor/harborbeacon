# Harbor Framework Protocol Map

## Purpose

This document is the canonical HarborBeacon-centered framework and protocol map for the Harbor family repositories. Phase 1 is documentation only: it freezes the current responsibilities, public protocol vocabulary, and build/package adaptation points without refactoring runtime code or changing public APIs.

HarborBeacon is the shared business-core framework. The same runtime, task, policy, audit, model, artifact, and device-control foundation is packaged differently for HarborOS Nexus amd64 and HarborNavi K3 riscv64.

## Repository Role

HarborBeacon owns the core framework:

- Task turn ingestion and response normalization.
- Runtime planning, policy, approval, artifacts, audit, and notification intent.
- Model Center and local model policy.
- HarborOS system control routing.
- Home Device Domain adapters and camera/vision event ingestion.
- IM delivery intent through HarborGate, while keeping transport ownership outside HarborBeacon.

HarborBeacon does not own:

- HarborGate channel adapters or platform credential storage.
- HarborCloud account, entitlement, WebRTC signaling, or AWS IoT control plane.
- HarborLink MQTT connector state.
- HarborDock Android UI state.
- HarborNAS WebUI display state.

## Shared Frame

The active collaboration frame is:

- HarborBeacon is the business-core framework and source of task/runtime semantics.
- HarborGate is the IM/channel edge.
- HarborCloud is the cloud control plane.
- HarborLink is the Hub-side outbound connector.
- harbor-dock is the Android/Paper client surface.
- HarborNAS-webui is the HarborOS WebUI surface.

Every repository keeps its own local copy of this map from its own perspective, and those maps should reference each other rather than relying on memory.

## Northbound Interfaces

HarborBeacon northbound interfaces are existing contracts. This document adds no new route or payload.

- Task turn API: `POST /api/web/turns`.
- Task turn alias: `POST /api/turns`.
- Contract header: `X-Contract-Version: 2.0`.
- Turn envelope: `TaskTurnEnvelope`.
- Required routing vocabulary: `conversation.handle`, `transport.route_key`, `active_frame`, `continuation`, `delivery_hints`.
- WebUI/API alias family: `/api/beacon/*`, `/api/harbor-beacon/*`, `/api/harbor-assistant/*`, and `/api/admin/*` alias into HarborBeacon admin/product APIs.
- Product/admin surfaces include gateway status, release readiness, hardware readiness, RAG/knowledge, file browse, camera readiness, HarborOS status, Home Assistant status, Model Center, feature availability, vision events, approvals, notification targets, discovery, and devices.
- Notification delivery contract is HarborBeacon intent to HarborGate delivery, not HarborBeacon platform transport ownership.

The active IM contract is HTTP/JSON v2.0. Future v3.0 work must preserve the same ownership split unless a later architecture decision explicitly changes it.

## Core Ownership

Core framework modules:

- `src/runtime/task_api.rs`: task turn envelope, canonical conversation handle, route key handling, active frame, continuation, delivery hints, and task response conversion.
- `src/runtime/admin_console.rs`: product/admin state and action aggregation.
- `src/runtime/model_center.rs`: local model catalog, policy, status, and feature availability.
- `src/orchestrator/*`: planning, route priority, executors, HarborOS operations, model selection, and vision execution.
- Approval, artifact, audit, and notification modules: enforce durable runtime traceability around task execution.

The business core owns semantic task state. It treats external route identifiers as opaque transport routing hints and does not import transport runtime state from other repositories.

## Southbound Interfaces

HarborBeacon southbound interfaces stay split by domain.

HarborOS System Domain:

- Preferred path: `middleware_api`.
- Fallback path: `midcli`.
- Browser and MCP fallback remain lower priority and should not become the primary HarborOS control plane.
- Existing operations include service status/control and HarborOS file operations where supported by the middleware contract.

Home Device Domain:

- Home Assistant REST/WebSocket remains the preferred home-state integration point.
- Native camera/device protocols are adapters, including RTSP, ONVIF, mDNS/SSDP discovery, and Matter scaffolding where present.
- HarborLink is the outbound Hub connector and MQTT command/ack bridge, not HarborBeacon core state.

IM Delivery Domain:

- HarborBeacon emits delivery intent and notification payloads.
- HarborGate owns channel transport, platform formatting, delivery fanout, and credentials.

## Build And Deployment Fit

| Target | Package/build identity | Runtime fit | Notes |
| --- | --- | --- | --- |
| Nexus / HarborOS amd64 | `harboros-beacon` Debian package, systemd service, `harbor-model-api` support | Local HarborOS framework and same-origin WebUI integration | Uses `/var/lib/harboros-beacon`, `/etc/default/harboros-beacon*`, Candle bootstrap, and HarborOS middleware-first control. |
| HarborNavi K3 riscv64 | K3 Debian package plus `harbornavi-k3-local-vision-smoke` | Local vision smoke, RTSP/YOLO path, `LocalVisionEvent` ingestion | Builds for riscv64 Linux target and validates Bianbu/K3 local vision without changing the shared framework contract. |

The target-specific packaging changes deployment shape only. It must not fork the northbound task contract or push device/cloud/UI concerns into the HarborBeacon core.

## Frozen Boundaries

- Keep `X-Contract-Version: 2.0`, `TaskTurnEnvelope`, `conversation.handle`, `transport.route_key`, `active_frame`, `continuation`, `delivery_hints`, and the notification delivery contract stable unless a later versioned contract replaces them.
- Keep HarborGate transport, HarborCloud entitlement, HarborLink MQTT, HarborDock UI intent, and WebUI display state outside HarborBeacon business core.
- Keep HarborOS system control and Home Device Domain integrations separate.
- Treat older IM/task documents as historical unless they are explicitly named by the current v2.0 contract or a later architecture decision.

## Cross-Repo References

- Bean-Harbor/HarborBeacon: `docs/harbor-framework-protocol-map.md` and `docs/HarborBeacon-Harbor-Collaboration-Contract-v2.md`.
- Bean-Harbor/HarborGate: `docs/harbor-framework-protocol-map.md` and `docs/HarborBeacon-HarborGate-Agent-Contract-v2.0.md`.
- Bean-Harbor/HarborCloud: `docs/harbor-framework-protocol-map.md`, `docs/roadmap.zh.md`, and `docs/architecture.md`.
- Bean-Harbor/HarborLink: `docs/harbor-framework-protocol-map.md` and `docs/protocol.md`.
- Bean-Harbor/harbor-dock: `docs/harbor-framework-protocol-map.md` and `docs/project-scope.md`.
- HarborNAS/webui: `docs/harbor-framework-protocol-map.md` and `docs/harbor-assistant-webui-integration.md`.

## Verification Scope

For Phase 1, verification is documentation and contract hygiene only:

- Confirm the six protocol maps reference the same active vocabulary.
- Confirm obsolete task ingress claims and stale contract paths are not presented as current authority.
- Run whitespace/diff checks for each added document.
- Do not run live deploy, do not touch targets, and do not change runtime behavior.
