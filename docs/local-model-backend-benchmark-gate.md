# Local Model Backend Benchmark Gate

更新时间：2026-05-17

## 1. Purpose

这份文档只定义一件事：

- 在 HarborOS 单机上，什么时候允许把 Runtime Manager 的 Harbor-managed local
  runtime 或模型标记为默认可用
- 当前权威 runtime gate 是 HarborOS `.82`
- `.182` 保留为历史 Candle gate / 可复验 target
- Builder `.197` 是当前 release builder；`.223` 只负责历史 build / prefetch / spawned benchmark 证据，不作为 live gate

这里冻结的不是具体推理框架，而是 **HarborBeacon 本地推理 facade 与
OpenAI-compatible wire shape**。Candle 是默认主力 runtime，但不是唯一未来
backend。

## 2. Frozen Boundary

- HarborBeacon local inference facade is frozen
- OpenAI-compatible wire shape remains the internal/adapted protocol for chat and embeddings
- HarborBeacon 继续只依赖：
  - `GET /healthz`
  - `POST /v1/chat/completions`
  - `POST /v1/embeddings`
- HarborBeacon 不因为 backend 变化而改 northbound seam：
  - `POST /api/tasks`
  - `POST /api/notifications/deliveries`
- HarborGate 不拥有模型选择权，也不承接 retrieval semantics

## 3. Backend Policy

- Product default is Harbor-managed Candle-first for local LLM / embedding, with
  Runtime Manager owning install/start/stop/health/model-store state.
- Candle default does not mean Candle is the frozen contract. VLM, OCR, ASR,
  sidecars, and future runtimes still compete behind Runtime Manager and the
  local inference facade.
- ISO may preinstall a ~0.5B bootstrap LLM for IM / WebUI natural-language
  parsing, setup guidance, intent classification, and parameter extraction.
  It must lazy-load and must not be marketed as the final answer/RAG model.
- OpenAI-compatible external endpoints are advanced user-managed integrations.
  Harbor does not auto-reuse or manage `127.0.0.1:11434`.
- Only a backend/model combination that passes the relevant benchmark gate may
  be shown as default-ready in Harbor Assistant.
- Cloud fallback is not backend promotion evidence. The product remains local-first; SiliconFlow may be used only as controlled fallback for `semantic.router` and `retrieval.answer`, and it must not be cited as the default local backend.
- Model prefetch/download jobs should record the Hugging Face endpoint used. Mirror priority is Harbor Assistant input -> `HF_ENDPOINT` -> `https://hf-mirror.com`.

## 4. Required Evidence

backend promotion 的证据固定来自 `benchmark-local-model-backend` 产出的
JSON 报告，而不是口头判断。

报告至少要覆盖：

- cold start
- healthz readiness
- Chinese chat probes
- embedding retrieval quality relative to a lexical baseline

## 5. How To Run

### Attached observation

这个模式用于观察一个已经在跑的模型服务，但它 **不构成 promotion 证据**，
因为不会测 cold start。

```bash
cargo run --bin benchmark-local-model-backend -- \
  --base-url http://127.0.0.1:4174/api/inference/v1 \
  --healthz-url http://127.0.0.1:4174/api/inference/healthz \
  --backend candle \
  --output /tmp/local-model-benchmark-candle-attached.json
```

### Spawned promotion gate

这个模式才是正式 gate。它会自己拉起 `harbor-model-api`，测 cold start，
再跑 chat 和 embedding probes。

```bash
cargo run --bin benchmark-local-model-backend -- \
  --spawn-binary ./target/x86_64-unknown-linux-musl/release/harbor-model-api \
  --backend candle \
  --bind 127.0.0.1:4186 \
  --chat-model harbor-local-chat \
  --embedding-model harbor-local-embed \
  --output /tmp/local-model-benchmark-candle.json
```

如果用户在 Advanced Settings 手动配置 external OpenAI-compatible endpoint，
benchmark 可以显式传：

```bash
--upstream-base-url http://127.0.0.1:11434/v1
```

## 6. Promotion Rules

只有在下列条件全部满足时，报告里的 `gate.promotable` 才能为 `true`：

- cold start 在门限内完成
- `/healthz` 报告 ready
- `health.backend.kind` 与期望 backend 一致
- Chinese chat probes 全部通过
- embedding probes 维度稳定、非零
- embedding retrieval quality relative to a lexical baseline 有可见提升

如果任一项不满足：

- 不切默认 backend
- 不改 HarborBeacon 的模型 endpoint contract
- 不改 HarborGate 或 IM seam

### 6.1 Candle Side Lane On HarborOS

- HarborOS `.82` / `.182` 上可以拉起 `candle` 候选实例做 runtime gate；
  Harbor Assistant 的默认产品路径仍通过 Runtime Manager，而不是用户外部 Ollama。
- Bootstrap LLM 候选基线：
  - chat/parser: `Qwen/Qwen2.5-0.5B-Instruct`
  - alternate: `Qwen/Qwen3-0.6B`
- Full local answer / embedding 候选基线：
  - chat: `Qwen/Qwen3-1.7B`
  - embeddings: `jinaai/jina-embeddings-v2-base-zh`
- `.223` 上任何 spawned benchmark 红灯都只算兼容性 / 冷启动历史证据；
  是否允许进入 cutover rehearsal，只看 `.182` 的 target-runtime report。
- `Qwen3.5` 继续延期到后续 loader / backend 轮次，不进入本轮 gate。
- 这条 lane 的最小通过口径是：
  - `/healthz` 报告 `ok` / `ready=true`
  - `/v1/chat/completions` 通过中文探针
  - `/v1/embeddings` 返回稳定、非零、可归一化向量
- Runtime health 必须区分 `runtime alive`、`model available`、`model loaded`；
  healthz 不应为了证明 runtime alive 而强制加载权重。
- 只要 bootstrap LLM 没有通过 cold start、health、中文输入解析和安全边界探针，
  就不能把它写成 ISO 默认可用。
- 只要 full local answer / embedding 没有同时通过 chat 和 embeddings gate，
  就不能把它写成 Harbor Assistant 的默认 answer/RAG 模型。

## 7. Release Gate Integration

`evaluate-release-gate` 可以可选接收 benchmark 报告：

```bash
cargo run --bin evaluate-release-gate -- \
  drift-report.json \
  --require-live \
  --model-benchmark-report /tmp/local-model-benchmark-candle.json \
  --require-model-benchmark \
  --output release-gate-summary.json
```

这条门禁只在“要切默认 backend”的那一波 release 使用。
普通 release 继续可以不要求 model benchmark evidence。

## 8. What This Lane Does Not Change

- 不把 Candle 冻成唯一实现
- 不把 bootstrap LLM 写成完整本地大脑
- 不把 retrieval semantics、ranking、citation、answer ownership 下放到 HarborOS
- 不把 AIoT 设备 evidence contract 变成模型语义 contract
- 不把多模态 RAG 退化成“只换一个推理框架”
