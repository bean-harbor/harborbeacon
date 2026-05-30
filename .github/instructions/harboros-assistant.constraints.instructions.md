---
applyTo: "**"
description: "Always enforce HarborBeacon assistant architecture constraints: HarborOS core-integration, plugin-based skills, API->midcli->browser->mcp routing, and command-line-first execution."
---

# HarborBeacon Assistant Constraints (Always On)

## Product boundary

- Integrate only the orchestrator core into HarborOS: runtime, planner, router, policy, audit, and HarborOS action adapter.
- HarborBeacon is pre-installed in HarborOS as the business-core assistant runtime, not as the IM transport owner.
- HarborGate is the IM/channel edge for Feishu, WeCom, Telegram, Discord, DingTalk, Slack, MQTT, and future channel transports.
- Users interact through WebUI/API directly with HarborBeacon, or through HarborGate when the origin is an IM/channel surface.
- HarborDock remote home/camera workflows belong to the HarborCloud + HarborLink + HarborDock boundary unless an approved assistant turn explicitly enters the Beacon/Gate seam.
- Keep non-core capabilities as plugins (skills): video editing, browser automation, third-party software control.
- Do not move plugin-specific logic into HarborOS core unless it is required for platform safety or governance.

## HarborBeacon IM integration

- HarborBeacon runs on the same machine as HarborOS and is part of the pre-installed image.
- IM channel configuration is HarborGate-owned and may be surfaced by HarborOS/WebUI through redacted setup/status APIs.
- HarborBeacon autonomy levels (ReadOnly / Supervised / Full) must align with assistant risk levels (LOW / MEDIUM-HIGH / admin-only).
- All IM-originated commands flow through HarborGate into the v2.0 turn contract, then through the same HarborBeacon policy, audit, and routing pipeline as WebUI/API commands.

## Execution policy

- For HarborOS System Domain operations, use deterministic route priority: `middleware_api -> midcli -> browser -> mcp`.
- For HarborOS domain operations, never use browser or MCP if API or midcli route is available.
- Keep Home Device Domain control separate from HarborOS system control; do not route Home Assistant, RTSP, ONVIF, HarborLink MQTT, or HarborCloud entitlement through generic HarborOS shell paths.
- Prefer command-line execution for capability expansion.
- For HarborOS CLI route, prefer `midcli` over generic shell commands.

## Model capability policy

- Treat NSP as `semantic.router`; do not introduce a separate NSP model type.
- Keep `semantic.router` fixed to the Harbor-managed Candle CPU bootstrap endpoint `semantic-router-local-cpu`.
- The bootstrap model is `Qwen/Qwen2.5-0.5B-Instruct`; if its runtime or artifact is missing, report explicit degraded state.
- Do not route `semantic.router` to cloud endpoints, user-managed external OpenAI-compatible LLMs, `retrieval.answer` model selection, iGPU/NPU experiments, or detector evaluation paths.

## Safety and governance

- Enforce dry-run for high-risk and destructive operations when preview is supported.
- Enforce explicit approval for `HIGH` and `CRITICAL` risk actions.
- Enforce path/service validation and deny unsafe operations by default.
- Record structured audit events for every task step: selected route, fallback, inputs, outcome, and duration.

## Delivery mode

- Ship in vertical slices: runnable code first, docs second.
- Keep contract tests and fallback tests updated with every capability change.
- Treat `midcli-only` availability as `degraded` where policy allows; do not block release by default.
