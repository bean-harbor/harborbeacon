# HarborBeacon Local Agent Planning Package

This repository contains the completed planning deliverables for a HarborBeacon local-first AI agent project, including architecture, roadmap, quick reference, meeting guide, launch checklist, and document index.

## Current IM Contract Control Pack

The active HarborBeacon <-> HarborGate seam is the v2.0 upgrade control pack.

- Active contract: `C:\Users\beanw\OpenSource\HarborGate\HarborBeacon-HarborGate-Agent-Contract-v2.0.md`
- HarborBeacon runbook: `HarborBeacon-HarborGate-v2.0-Upgrade-Runbook.md`
- Cutover gates: `docs/im-v2.0-cutover-rollback-observability-gates.md`

The previous v1.5 seam is historical only. Current implementation work should
move toward `POST /api/web/turns`, `conversation.handle`, `active_frame`,
`continuation`, `delivery_hints`, and `X-Contract-Version: 2.0`.

## Rust Runtime (New)

The project now includes a Rust runtime that compiles into a standalone binary for easier HarborOS integration.

- Binary name: `harborbeacon-agent`
- Entry point: `src/main.rs`
- Core modules: `src/orchestrator/` and `src/skills/`
- Planner module: `src/planner/`
- Migrated scripts module: `src/scripts/`
- Route priority: `middleware_api -> midcli -> browser -> mcp`

Build and run:

- `cargo build --release`
- `./target/release/harborbeacon-agent --plan examples/plan_service_status.json`

Additional migrated script binaries:

- `./target/release/validate-contract-schemas --skip-live --report validate-contract-report.rust.json`
- `./target/release/run-e2e-suite --env env-a --report e2e-report.rust.json`
- `./target/release/run-drift-matrix --harbor-ref develop --upstream-ref master --report drift-matrix-report.rust.json`
- `./target/release/evaluate-release-gate drift-matrix-report.rust.json --output release-gate-summary.rust.json`

Useful flags:

- `--disable-middleware`: skip `middleware_api` route
- `--disable-midcli`: skip `midcli` route
- `--midcli-passthrough`: execute real `midcli` command instead of preview mode
- `--approval-token` and `--required-approval-token`: pass HIGH/CRITICAL approval gate
- `--force-dry-run`: force all actions into dry-run mode

## Documents
- HarborBeacon-LocalAgent-Plan.md
- HarborBeacon-LocalAgent-Roadmap.md
- HarborBeacon-LocalAgent-QuickRef.md
- HarborBeacon-LocalAgent-MeetingGuide.md
- HarborBeacon-LocalAgent-LaunchChecklist.md
- HarborBeacon-LocalAgent-DocumentIndex.md
- HarborBeacon-LocalAgent-V2-Assistant-Skills-Roadmap.md
- HarborBeacon-Skill-Spec-v1.md
- HarborBeacon-Middleware-Endpoint-Contract-v1.md
- HarborBeacon-Files-BatchOps-Contract-v1.md
- HarborBeacon-Planner-TaskDecompose-Contract-v1.md
- HarborBeacon-Contract-E2E-Test-Plan-v1.md
- HarborBeacon-CI-Contract-Pipeline-Checklist-v1.md
- HarborBeacon-GitHub-Actions-Workflow-Draft-v1.md

## V2 Additions

- `HarborBeacon-LocalAgent-V2-Assistant-Skills-Roadmap.md`: assistant + skills integration roadmap with HarborOS control-plane-first policy (`middleware API > midcli > browser > MCP`).
- `HarborBeacon-Skill-Spec-v1.md`: standard skill contract (manifest schema, runtime envelope, routing, risk and test requirements), including HarborOS `middleware` API binding and `midcli` fallback rules.
- `HarborBeacon-Middleware-Endpoint-Contract-v1.md`: executable endpoint contract for `system.harbor_ops`, including action normalization, API/CLI mapping, error model, and compatibility matrix template.
- `HarborBeacon-Files-BatchOps-Contract-v1.md`: executable endpoint contract for `files.batch_ops`, including path policy, route fallback chain, CLI template constraints, and compatibility matrix template.
- `HarborBeacon-Planner-TaskDecompose-Contract-v1.md`: executable planner contract for `planner.task_decompose`, including step schema, dependency rules, route-candidate policy, and release gates.
- `HarborBeacon-Contract-E2E-Test-Plan-v1.md`: end-to-end validation plan across planner + execution contracts, including environment matrix, fallback checks, drift checks, and release exit criteria.
- `HarborBeacon-CI-Contract-Pipeline-Checklist-v1.md`: CI job checklist that maps all contract governance to merge, nightly, and pre-release pipeline stages.
- `HarborBeacon-GitHub-Actions-Workflow-Draft-v1.md`: initial GitHub Actions workflow draft mapping contract governance into concrete PR, nightly, and release workflows.

## HarborBeacon — IM 接入与用户交互层

HarborBeacon 是基于 [ZeroClaw](https://github.com/punkpeye/zeroclaw) 二次开发的 AI 助手，**预装在 HarborOS 中**（与 HarborOS 运行在同一台机器上）。用户通过即时通讯工具与 HarborBeacon 交互，HarborBeacon 通过 CLI、MCP、API 控制 HarborOS 各项功能。

### 架构总览

```text
[IM Channels]          [HarborBeacon]                [Orchestrator]            [HarborOS]
  飞书 / 企微            channels.py                 router / planner         middleware API
  Telegram / Discord  →  mcp_adapter.py           →  policy / audit        →  midcli
  钉钉 / Slack / MQTT    autonomy.py                 skills / executors       system services
                         tool_descriptions.py
```

### 支持的 IM 通道

| 通道 | 枚举值 | 说明 |
|---|---|---|
| 飞书 | `FEISHU` | Lark/飞书机器人 webhook |
| 企业微信 | `WECOM` | 企业微信应用消息 |
| Telegram | `TELEGRAM` | Telegram Bot API |
| Discord | `DISCORD` | Discord Bot gateway |
| 钉钉 | `DINGTALK` | 钉钉机器人 |
| Slack | `SLACK` | Slack App / Bot |
| MQTT | `MQTT` | 轻量级 IoT 消息协议 |

### 自主级别

| 级别 | 说明 | 对应风险 |
|---|---|---|
| `ReadOnly` | 只读操作，无需审批 | LOW |
| `Supervised` | 需逐次审批令牌 | MEDIUM / HIGH |
| `Full` | 完全自主执行 | 仅限管理员配置 |

### 代码包: `harborbeacon/`

- `channels.py`: IM 通道注册、消息路由、意图解析
- `mcp_adapter.py`: MCP 工具适配（ReadOnly 守卫、审批令牌）
- `autonomy.py`: 自主级别与风险等级映射
- `tool_descriptions.py`: skill manifest → MCP/TOML 工具描述转换

## Model Center And Multimodal RAG v1

HarborBeacon 现在已经把“模型能力可配置”作为 framework 层的一部分接进来，而不是把 OCR 或后续 RAG 依赖写死在代码里。

Current scope:

- retrieval 继续由 HarborBeacon 自己生成 citations / reply pack
- `document + image + OCR` 已进入同一条检索主线
- `audio / video / VLM` 仍保持 pending，不抢第一阶段交付

Admin surfaces:

- `GET/POST /api/models/endpoints`
- `PATCH /api/models/endpoints/:id`
- `POST /api/models/endpoints/:id/test`
- `GET/PUT /api/models/policies`
- `GET /api/feature-availability`
- `GET /admin/models`

Current provider model:

- local: `tesseract`, Ollama, vLLM, llama.cpp, LM Studio, other OpenAI-compatible endpoints
- cloud: controlled OpenAI-compatible fallback, currently preset as `llm-cloud-siliconflow`
- model execution is a shared capability layer, not a HarborOS / AIoT / IM business domain
- secrets are persisted server-side and returned through the admin API in redacted form; empty API key saves do not overwrite an existing endpoint secret
- local model downloads prefer Harbor Assistant mirror input, then `HF_ENDPOINT`, then `https://hf-mirror.com`

Current defaults:

- `retrieval.ocr` prefers a local `tesseract` slot
- `retrieval.embed` prefers local OpenAI-compatible endpoints
- `semantic.router` supports local-first with controlled cloud fallback
- `retrieval.answer` supports local-first with cloud fallback
- `retrieval.vision_summary` is present in policy but remains local/sidecar only until a VLM is configured
- HarborOS commands, AIoT control, OCR, VLM, and embedding routes do not use cloud fallback by default

Runtime-truth rule:

- `GET /api/feature-availability` is the grouped read-model for runtime truth, route policy, account management, and gateway status
- local runtime truth from `/api/inference/healthz` may override stale stored endpoint projection for the built-in LLM/embedder rows
- Harbor Assistant keeps `projection_mismatch` visible instead of silently flattening runtime truth back into stored admin state
- LLM fallback audit records selected endpoint, attempted endpoints, and fallback reason without logging plaintext keys or full sensitive prompts

## Executable CI Scaffold

- `.github/workflows/contract-pr-check.yml`: PR and branch validation for contract schema checks plus contract, fallback, and policy test suites.
- `.github/workflows/contract-nightly-e2e.yml`: nightly/manual E2E matrix scaffold for `env-a` and `env-b`.
- `.github/workflows/contract-release-drift.yml`: release-branch drift matrix and release gate workflow.
- `target/release/validate-contract-schemas`: validates that required contract documents and route-priority rules stay aligned.
- `target/release/run-e2e-suite`: emits scaffolded E2E, latency, and audit reports for workflow wiring.
- `target/release/run-drift-matrix`: emits the initial drift-matrix artifact for release gating.
- `target/release/evaluate-release-gate`: converts drift output into a blocking/non-blocking release decision.
- `tests/contracts`, `tests/fallback`, `tests/policy`: minimal pytest suites that keep the documented routing, fallback, and governance rules from regressing.

Current scope note: the default CI path runs Rust binaries in documentation-only mode, and the same binaries can switch into live HarborBeacon integration mode when `midclt` and/or `cli` are available.

## Live Integration Mode

The four Rust binaries now support live HarborBeacon probing through `middleware` and `midcli`.

- Middleware transport: local `midclt call ...`
- MidCLI transport: non-interactive `cli -m csv -c ...`
- Safe default probes: `service.query` for the configured probe service and `filesystem.listdir` for a configured path

Key environment variables:

- `HARBOR_MIDDLEWARE_BIN`: middleware CLI binary, default `midclt`
- `HARBOR_MIDCLI_BIN`: midcli binary, default `cli`
- `HARBOR_MIDCLI_URL`, `HARBOR_MIDCLI_USER`, `HARBOR_MIDCLI_PASSWORD`: remote midcli connection parameters when probing over websocket
- `HARBOR_PROBE_SERVICE`: safe service probe target, default `ssh`
- `HARBOR_FILESYSTEM_PATH`: safe filesystem probe path, default `/mnt`
- `HARBOR_SOURCE_REPO_PATH`, `UPSTREAM_SOURCE_REPO_PATH`: optional source trees used by `run-drift-matrix` for source-level capability comparison
- `HARBOR_ALLOW_MUTATIONS`: set to `1` to let E2E execute approved write operations instead of preview-only
- `HARBOR_APPROVAL_TOKEN`: approval token passed into HIGH-risk operations such as service restart and file move
- `HARBOR_REQUIRED_APPROVAL_TOKEN`: optional expected token value for the local script gate
- `HARBOR_APPROVER_ID`: approver identity written into mutation results for audit correlation
- `HARBOR_MUTATION_ROOT`: sandbox root for mutation fixtures, default `/mnt/software/harborbeacon-agent-ci`

Release/install note:

- the exec-capable release root may live under `/var/lib/harborbeacon-agent-ci`
- the HarborOS mutation root / writable root can still remain `/mnt/software/harborbeacon-agent-ci`
- installer env now exposes that writable path explicitly through `HARBOR_HARBOROS_WRITABLE_ROOT`

HarborOS `.182` resident stack checks:

- after install, use `/var/lib/harborbeacon-agent-ci/bin/harbor-agent-hub-helper status`
- use `/var/lib/harborbeacon-agent-ci/bin/harbor-agent-hub-helper health` to probe HarborBeacon, HarborBeacon inference, HarborGate, and `GET /api/gateway/status`
- use `sudo /var/lib/harborbeacon-agent-ci/bin/harbor-agent-hub-helper logs gateway --lines 120` for the HarborGate journal when `.182` keeps journald access restricted

Typical usage:

- `./target/release/validate-contract-schemas --require-live`
- `./target/release/run-e2e-suite --env env-a --require-live`
- `./target/release/run-drift-matrix --harbor-ref develop --upstream-ref master`
- `./target/release/evaluate-release-gate drift-matrix-report.json --require-live`

Controlled mutation example:

- `HARBOR_ALLOW_MUTATIONS=1 HARBOR_APPROVAL_TOKEN=approved HARBOR_REQUIRED_APPROVAL_TOKEN=approved HARBOR_MUTATION_ROOT=/mnt/software/harborbeacon-agent-ci ./target/release/run-e2e-suite --env env-a --require-live`

### Windows Remote MidCLI Shim

For Windows workstations that do not have HarborOS native `cli` installed locally,
use the repository shim in `tools/` to proxy midcli-compatible commands over
WebSocket to a remote HarborOS host.

- Shim entry command: `tools/cli.cmd`
- Python implementation: `tools/harbor_cli_shim.py`
- Supported commands:
  - `service query ... WHERE service == '...'`
  - `filesystem listdir path=...`
  - `service restart|start|stop service=...`
  - `filesystem copy ...` and `filesystem move ...`

Example (PowerShell):

- `$env:HARBOR_MIDCLI_BIN = (Resolve-Path .\tools\cli.cmd).Path`
- `$env:HARBOR_MIDCLI_URL = 'ws://<harbor-host>/websocket'`
- `$env:HARBOR_MIDCLI_USER = '<username>'`
- `$env:HARBOR_MIDCLI_PASSWORD = '<password>'`
- `./target/release/run-e2e-suite.exe --env env-a --require-live --report rust-live-e2e-report.json`

For reviewable smoke runs, the repo now ships both verifier entrypoints:

- Windows: `.\tools\run_harboros_vm_smoke.ps1`
- Debian/Linux: `bash ./tools/run_harboros_vm_smoke.sh`

Current live policy note: the verified HarborOS mutation sandbox on `192.168.3.182`
is `/mnt/software/harborbeacon-agent-ci`; do not assume `/data` is writable on
that target unless operators explicitly provision and validate it.
