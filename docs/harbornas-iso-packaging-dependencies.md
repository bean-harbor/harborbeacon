# HarborBeacon / HarborGate / Harbor Assistant for HarborOS ISO 集成 Handoff

面向：HarborNAS ISO 集成与打包维护者
日期：2026-05-08
状态：给 HarborOS / ISO 集成同事的嵌入文档与依赖清单

## 0. 交付口径

这份文档不是 HarborBeacon 团队自建 ISO 的说明。HarborOS / ISO 集成同事
拥有最终镜像构建流程；HarborBeacon 侧只交付可嵌入 HarborOS 的服务、
WebUI 产品面、配置模板、验证门禁和边界约束。

当前我们按对方集成要求已经完成的关键收敛：

- HarborGate 已改为 Rust-only runtime；当前 ISO 不再 vendor Python runtime。
- HarborBeacon 对 Harbor Assistant / inference / turn API 收敛为
  `harborbeacon.service` 单端口 `4174`。
- HarborGate 作为独立 `harboros-im-gate.service`，默认只监听 `127.0.0.1:8787`。
- Harbor Assistant 的最终 UI 来源是 `HarborNAS-webui` production dist；HarborBeacon
  仓内 `frontend/harbor-assistant` 仅作为 API 验证 shell。
- Harbor Assistant-only 入口已经收敛完成：active UI 只保留 `/ui/harbor-assistant`，active WebUI proxy 只保留 `/api/beacon/**`。
- HarborGate setup / messages 入口已对齐 Harbor Assistant 的消息连接 tab；
  不新增旧 Harbor UI 兼容入口或旧同源 API 前缀。
- `ffmpeg` 和 `ffprobe` 已提升为 HarborBeacon release bundle 必需 artifact；
  默认使用 BtbN `ffmpeg-master-latest-linux64-lgpl.tar.xz`，安装到
  `<install-root>/runtime/media-tools/bin/` 后写入 `HARBOR_FFMPEG_BIN` /
  `HARBOR_FFPROBE_BIN`。
- GitHub Actions 已新增 `.deb` 交付：artifact 名称
  `harborbeacon-harboros-deb-<version>`，package 名称
  `harborbeacon-harboros-release`，用于 HarborOS ISO / first-boot 集成。

最新 handoff 参考 baseline：

- Harbor Assistant-only cleanup: HarborNAS WebUI / HarborBeacon active docs and
  API prefixes are aligned to `/ui/harbor-assistant` and
  `/api/beacon/**` only.
- Media tools packaging: release bundle must carry BtbN LGPL static
  `media-tools/bin/ffmpeg` and `media-tools/bin/ffprobe`; installer fail-fast
  unless the explicit dev override is used.
- Debian delivery: `.github/workflows/build-deb.yml` builds the release bundle,
  verifies it, wraps it as `harborbeacon-harboros-release_<version>_amd64.deb`,
  and uploads the `.deb` plus checksum as an Actions artifact.
- `.82` camera proof: after installing bundled media tools, TP-IPC validation
  passed and `/api/beacon/cameras/<device-id>/snapshot.jpg` returned
  `200 image/jpeg`.

历史 RC2 参考：

- HarborBeacon: `a5f6da0`
- HarborGate: `57ff759`
- HarborNAS WebUI: `8e3f04d`
- `.82` RC2 release:
  `20260430-rc2-beacona5f6da0-gate57ff759`
- RC2 bundle sha256:
  `7119842506d38aac82c7e236b7f96a054244bb50be07c5e6b001ac7b0683484c`

## 1. 集成目标与边界

本清单用于让 HarborOS / ISO 集成同事把 HarborBeacon、HarborGate，以及
HarborNAS WebUI 中的 Harbor Assistant 产品面合并进 HarborOS 镜像或首装包。

当前按三个仓库集成：

| 仓库 | ISO 里承载的产品面 | 主要职责 |
|---|---|---|
| `HarborBeacon` | HarborBeacon 后端 runtime | 单端口业务核心、Admin API、Turn API、模型代理、审批、artifact、audit、知识库、设备配置状态 |
| `HarborGate` | HarborGate IM gateway | IM 适配器、平台 transport、route key、平台凭据、通知投递、绑定/配置入口 |
| `HarborNAS-webui` | Harbor Assistant UI | HarborNAS WebUI 内的原生 Harbor Assistant 页面，内嵌搜索和摄像头 tab |

需要特别注意：

- Harbor Assistant 是 HarborNAS WebUI 内的唯一公开 Harbor 入口；当前 live 路径为 `/ui/harbor-assistant`。
- Search、Camera、Messages、Settings 都是 Harbor Assistant 内部 tab，不是独立公开入口或独立打包目标。
- `HarborBeacon/frontend/harbor-assistant` 只是 API 校验壳，不应作为最终 Harbor Assistant 产品 UI 交付。
- 旧 Harbor UI 入口不再作为 active route、redirect 或打包目标；旧同源 API prefix 也不再作为 active API prefix。
- HarborBeacon 与 HarborGate 当前 active contract 是 v2.0。服务间 active 请求必须使用：

```text
X-Contract-Version: 2.0
```

## 2. 运行时硬边界

HarborOS 镜像可以把服务安装在同一系统内，但运行时边界必须保持独立：

- HarborBeacon 与 HarborGate 只通过 HTTP/JSON 合约通信。
- 两个仓库不能互相 import runtime code。
- 两个仓库不能共享 runtime state 文件。
- HarborBeacon 拥有 business state、conversation、continuation、approval、artifact、audit。
- HarborGate 拥有 IM adapter、transport、route key lifecycle、平台凭据、outbound delivery。
- HarborBeacon 不保存 IM 平台原始凭据。
- HarborBeacon 不直接投递 IM 消息。
- HarborGate 不解释 HarborBeacon business conversation 的内部语义。
- HarborGate 只能 opaque 保存 conversation handle 和 continuation values。
- HarborNAS WebUI 只通过 `/api/beacon/**` proxy 调 HarborBeacon Admin API。
- AIoT camera / LAN device 管理属于 Home Device Domain，不并入 HarborOS System Domain。
- 模型执行是 HarborBeacon Model Center 的共享能力层，不属于 HarborOS System Domain、Home Device Domain 或 HarborGate IM transport。
- 镜像默认保持 local-first；SiliconFlow 只作为 `llm-cloud-siliconflow` OpenAI-compatible fallback preset，范围限定为 `semantic.router` 与 `retrieval.answer`。
- API key 必须通过 endpoint secret redaction 返回；Harbor Assistant 空 API key 保存不得覆盖已保存 secret。
- 本地模型下载默认使用 `https://hf-mirror.com`，优先级为 Harbor Assistant 输入 mirror -> `HF_ENDPOINT` -> 默认 mirror。

v2.0 禁止回退项：

- 不恢复 HarborGate active 调用 `/api/tasks`。
- 不恢复 public `args.resume_token` continuation。
- 不用 transport `session_id` 作为 HarborBeacon business conversation truth。
- 不把 IM 凭据长期归属放回 HarborBeacon。
- 不在本轮加入 group chat。
- 不引入 v1.5/v2.0 runtime dual-stack，除非架构决策明确反转。

## 3. 服务拓扑、端口与 systemd 单元

建议 HarborOS 镜像或首装包预置以下主服务：

| 服务 | 默认监听 | 必需性 | 说明 |
|---|---:|---|---|
| `harborbeacon.service` | `0.0.0.0:4174` | 必需 | Harbor Assistant / `/api/web/turns` / `/api/inference/*` 单端口 API |
| `harboros-im-gate.service` | `127.0.0.1:8787` | 必需 | HarborGate IM Gateway 与进程内 adapter runtime |

建议启动顺序：

```text
harborbeacon
harborgate
```

HarborNAS WebUI 本身不新增 Harbor Assistant 后端服务；Harbor Assistant
只是 WebUI 产品面，后端统一通过 HarborBeacon `:4174` 和 HarborGate
`:8787` 提供。

## 4. HarborNAS WebUI 集成

Harbor Assistant 的最终 UI 在 `HarborNAS-webui` 仓库。它不是三个独立
WebUI 包，而是一个 production dist 里的统一产品面：

| 入口 | WebUI 路径 | 集成要求 |
|---|---|---|
| Harbor Assistant 主入口 | `/ui/harbor-assistant` | 唯一公开入口；默认进入搜索 tab |

Harbor Assistant 内部 tab 与组件关系：

| Tab | 内部组件 | 说明 |
|---|---|---|
| 搜索 | `SearchComponent` | 用户侧 multimodal retrieval，不作为独立打包目标 |
| 摄像头 | `CameraComponent` | 摄像头/DVR/回放，不作为独立打包目标 |
| 消息连接 | Harbor Assistant 内建面板 | Feishu / Weixin setup、redacted gateway status |
| 设置 | Harbor Assistant 内建面板 | AI、模型、知识库、camera 配置 |

WebUI 构建要求：

```text
Node.js >= 24.13.1
Yarn 4.9.2
Angular 21.x
@truenas/ui-components ~0.1.12
```

生产构建命令：

```bash
yarn install --immutable
yarn build:prod
```

HarborNAS WebUI 需要为 Harbor Assistant 预留同源 proxy。后端 API 前缀只保留：

```text
/api/beacon/**
```

转发到 HarborBeacon Admin API：

```text
http://127.0.0.1:4174/api/**
```

当前开发 proxy 等价规则：

```text
/api/beacon/state -> http://127.0.0.1:4174/api/state
```

ISO / production 侧可以由 HarborNAS nginx、middleware proxy 或等价 WebUI
proxy 实现，但必须保持 `/api/beacon/**` 与 HarborOS 原生 `/api/**`
分离，避免把 HarborBeacon admin API 混入 HarborOS middleware API namespace。

## 5. Harbor Assistant 功能清单

Harbor Assistant 是 HarborNAS WebUI 内的 operator/admin/user assistant
surface，不是独立服务；集成同事不需要按 Search、Camera、Messages、Settings
分别打包。

当前需要 HarborBeacon Admin API 支撑的功能：

- Overview 状态：HarborOS principal、writable root、默认 CIDR、默认 camera、HarborBeacon connectivity。
- IM / Gateway 状态：平台状态、连接状态、Feishu / Weixin setup URLs、redacted connector status。
- 通知目标管理：读取、设置默认目标、删除目标。
- 硬件 readiness：CPU、memory、GPU/NPU、推荐模型 profile。
- RAG readiness：知识库能力、索引状态、阻塞项。
- 知识库设置：knowledge root、include/exclude、索引触发、索引 job 状态。
- 文件浏览：为知识库目录选择和校验提供后端支持。
- 模型配置：model endpoints、endpoint 测试、model policies。
- 本地模型：local catalog、download jobs、cancel download。
- HarborOS 状态：HarborOS user、writable root、system capability snapshot。
- IM capability map：WebUI 侧展示 HarborOS 与 IM 能力映射。
- AIoT / camera 管理：discovery scan、manual device add、default camera、device metadata。
- 设备凭据：保存设备凭据，UI 只展示 configured/redacted 状态。
- RTSP / snapshot：RTSP check、snapshot task、camera live/snapshot URLs。
- evidence / validation：device evidence、device validation run。
- share link：camera share-link create/revoke/list。

Harbor Assistant 典型 API 通过 WebUI proxy 调用：

```text
GET    /api/beacon/state
GET    /api/beacon/gateway/status
GET    /api/beacon/admin/notification-targets
POST   /api/beacon/admin/notification-targets/default
DELETE /api/beacon/admin/notification-targets/:id
GET    /api/beacon/hardware/readiness
GET    /api/beacon/rag/readiness
GET    /api/beacon/knowledge/settings
PUT    /api/beacon/knowledge/settings
POST   /api/beacon/knowledge/index/run
GET    /api/beacon/knowledge/index/status
GET    /api/beacon/files/browse
GET    /api/beacon/harboros/status
GET    /api/beacon/harboros/im-capability-map
GET    /api/beacon/models/endpoints
POST   /api/beacon/models/endpoints
PATCH  /api/beacon/models/endpoints/:id
POST   /api/beacon/models/endpoints/:id/test
GET    /api/beacon/models/policies
PUT    /api/beacon/models/policies
GET    /api/beacon/models/local-catalog
GET    /api/beacon/models/local-downloads
POST   /api/beacon/models/local-downloads
POST   /api/beacon/models/local-downloads/:job_id/cancel
POST   /api/beacon/discovery/scan
POST   /api/beacon/devices/manual
POST   /api/beacon/devices/default-camera
PATCH  /api/beacon/devices/:id
POST   /api/beacon/defaults
POST   /api/beacon/devices/:id/credentials
POST   /api/beacon/devices/:id/rtsp-check
GET    /api/beacon/devices/:id/evidence
POST   /api/beacon/devices/:id/validation/run
POST   /api/beacon/cameras/:id/share-link
POST   /api/beacon/share-links/:id/revoke
POST   /api/beacon/cameras/:id/snapshot
GET    /api/beacon/share-links
```

## 6. 搜索 tab / Search 组件

Search 现在是 Harbor Assistant 搜索 tab 内嵌的用户侧 multimodal
retrieval 组件，不是独立后端服务，也不是独立 WebUI 打包目标。

WebUI 路径：

```text
/ui/harbor-assistant?tab=search
```

后端 API：

```text
POST /api/beacon/knowledge/search
GET  /api/beacon/knowledge/preview
```

检索能力：

- 支持 documents / images / videos waterfall results。
- 支持 filter：`all`、`images`、`text`、`videos`。
- 请求可控制 `limit`、`include_documents`、`include_images`、`include_videos`。
- 响应包含 `reply_pack`、`supported_modalities`、`pending_modalities`。
- 响应包含 `status`、`degraded`、`blockers`、`warnings`，用于 UI 展示 degraded mode。
- evidence 字段需要保留，用于证明命中来源不是 filename shortcut：
  - `content_source_kinds`
  - `content_indexed`
  - `content_match_used`
  - `filename_match_used`
- preview 通过 HarborBeacon Admin API 受控返回，不能让 WebUI 直接读取任意本地文件路径。

## 7. HarborBeacon 后端能力清单

HarborBeacon 需要作为 ISO 内的业务核心 runtime 发布。

核心 Rust binaries：

```text
harborbeacon-service
validate-contract-schemas
run-e2e-suite
```

建议一并打包的 release / 验证 binaries：

```text
run-drift-matrix
evaluate-release-gate
harborbeacon-agent
benchmark-local-model-backend
```

主要 HTTP surfaces：

| Surface | 地址 | 说明 |
|---|---|---|
| HarborBeacon API | `http://127.0.0.1:4174` 或 `0.0.0.0:4174` | `/api/admin/*`、`/api/web/*`、`/api/inference/*` |
| HarborGate API | `http://127.0.0.1:8787` | IM Gateway、setup、status、delivery |

HarborBeacon active endpoints：

```text
GET  /healthz
POST /api/web/turns
POST /api/turns       # deprecated alias only
GET  /api/inference/healthz
POST /api/inference/v1/chat/completions
POST /api/inference/v1/embeddings
```

HarborBeacon service-to-service turn API 要求：

- `Authorization: Bearer <shared-token>`
- `X-Contract-Version: 2.0`
- 非 v2.0 contract version 应返回 contract mismatch。
- unknown route 使用 shared non-200 error envelope。

Admin API 支撑：

- Harbor Assistant state、account management、gateway status。
- release readiness 与 readiness history。
- hardware / RAG readiness。
- knowledge settings、search、preview、index run/status/jobs。
- files browse。
- HarborOS status 和 IM capability map。
- model endpoints、model policies、local catalog/downloads。
- feature availability。
- access members / roles / default delivery surface。
- tasks approvals approve/reject。
- notification targets。
- binding QR / setup mobile。
- camera live MJPEG、snapshot、share links、analyze、validation。
- discovery scan、manual device add、device credentials、RTSP check、device evidence。

## 8. HarborGate IM 能力清单

HarborGate 默认是 Rust runtime，负责 IM transport、setup/admin、runtime supervisor 和 delivery；Python 版只保留在旧 release artifact / Git 历史中。

Live adapters：

```text
feishu
weixin
webhook
```

Placeholder adapters：

```text
telegram
discord
slack
whatsapp
signal
email
wecom
```

核心命令：

```text
harborgate
```

历史命令别名仅存在于旧 release artifact：

```text
im-agent
im-agent-weixin-login
im-agent-weixin-ingress-probe
```

核心 HTTP routes：

```text
GET  /health
POST /messages/<platform>
POST /api/notifications/deliveries
GET  /api/gateway/status
GET  /setup/qr
GET  /setup/qr.svg
GET  /setup
GET  /api/setup/status
POST /api/setup/feishu/configure
```

Notification delivery endpoint：

```text
POST /api/notifications/deliveries
```

要求：

- active v2.0 traffic 使用 `X-Contract-Version: 2.0`。
- 如果设置 `IM_AGENT_SERVICE_TOKEN`，调用方必须使用 `Authorization: Bearer <token>`。
- route resolution 主要通过 `destination.route_key`。
- 没有 `route_key` 时只允许按 contract 定义 fallback 到 destination platform/id/recipient。
- delivery idempotency 使用 `delivery.idempotency_key`。
- gateway status 必须保持 redacted，不泄露平台凭据。

Feishu 能力：

- 默认 websocket / long-connection receive mode。
- 可选 webhook mode。
- 支持 `im.message.receive_v1` direct-message text。
- live send 由 `FEISHU_ENABLE_LIVE_SEND=1` 控制。
- setup portal 支持手机配置 app id / secret / verification token。
- group event gate 保持在 adapter 边界；本轮不承诺 group chat readiness。

Weixin 能力：

- QR login helper。
- iLink relay long-poll runner。
- private DM text inbound normalization。
- text outbound reply with cached `context_token`。
- ingress probe 区分 waiting-for-private-text 和 poll failure。
- 本轮不包含 group chat、image/file/voice send/receive、Weixin webhook mode。

可选 LLM backend：

```text
LLM_BASE_URL
LLM_API_KEY
LLM_MODEL
```

如果 HarborGate 直接启用 HarborBeacon Task API mode，必须确认代码已切到 v2.0 `/api/turns` client；旧的 task client mode 属于 release gate drift item。

## 9. 构建依赖

### HarborBeacon release bundle

推荐在 Linux builder 构建，不在 HarborOS 目标机上编译。

必需工具：

```text
Rust stable
rustup
cargo
python3
tar
sha256sum
find
file
```

推荐目标：

```text
x86_64-unknown-linux-musl
```

musl 静态构建额外需要：

```text
cargo-zigbuild
zig 0.15.1
```

`tools/build_release_bundle.sh` 当前会构建：

```text
harborbeacon-service
validate-contract-schemas
run-e2e-suite
```

### HarborGate runtime package

Rust 产物：

```text
harborgate/bin/harborgate
```

HarborGate 主线从 2026-05-01 起是 Rust-only runtime；ISO 不再 vendor
HarborGate Python runtime bundles。需要 Python 版时回滚到旧 release
artifact，而不是在当前 ISO 内切 runtime。

### HarborNAS WebUI

生产 WebUI 构建：

```bash
yarn install --immutable
yarn build:prod
```

Harbor Assistant 作为 HarborNAS WebUI dist 的一部分进入 ISO；搜索和摄像头
能力是它的内部 tab。不要把 `HarborBeacon/frontend/harbor-assistant` 当成
HarborNAS 最终 UI 构建来源。

## 10. 运行时系统包

建议 ISO 预装：

```text
systemd
python3 >= 3.11
python3-venv
python3-pip
ca-certificates
curl 或 wget
tar
sha256sum
```

HarborOS control path 需要目标系统具备 HarborOS middleware / CLI 能力。HarborBeacon southbound 优先级保持：

```text
middleware API -> midcli -> browser -> MCP
```

`ffmpeg` 和 `ffprobe` 不再作为人工预装项；HarborBeacon release bundle
必须自带 BtbN LGPL static `media-tools/bin/ffmpeg` 和
`media-tools/bin/ffprobe`，因为 RTSP snapshot、视频片段、媒体探测、
camera evidence 都会用到。安装脚本必须把二者复制到 exec-capable install
root 并写入：

```text
HARBOR_FFMPEG_BIN=<install-root>/runtime/media-tools/bin/ffmpeg
HARBOR_FFPROBE_BIN=<install-root>/runtime/media-tools/bin/ffprobe
```

构建侧支持离线输入：

```text
HARBOR_MEDIA_TOOLS_ARCHIVE=<path-to-ffmpeg-master-latest-linux64-lgpl.tar.xz>
HARBOR_MEDIA_TOOLS_SHA256=<archive-sha256>
```

每个 bundle 还必须通过 `tools/verify_release_bundle.py --require-execute`
确认 `manifest.json`、`checksums.sha256`、`media-tools/provenance.json` 和
两个二进制一致。

## 11. 可选 AI / 多模态依赖

基础 ISO 不强制内置大模型，但建议预留模型目录和配置。

OpenAI-compatible LLM upstream 可选：

```text
Ollama
vLLM
llama.cpp server
LM Studio
任意 OpenAI-compatible API
```

默认 upstream 示例：

```text
http://127.0.0.1:11434/v1
```

VLM sidecar 可选依赖：

```text
torch
transformers
Pillow
HuggingFaceTB/SmolVLM-256M-Instruct
```

VLM 默认配置：

```text
HARBOR_VLM_SIDECAR_ENABLE=0
HARBOR_VLM_BIND=<internal-loopback-bind>
HARBOR_VLM_MODEL_ID=HuggingFaceTB/SmolVLM-256M-Instruct
HARBOR_VLM_MODEL_PATH=<writable-root>/models/huggingfacetb-smolvlm-256m-instruct
HARBOR_VLM_DEVICE=cpu
HARBOR_VLM_LOCAL_FILES_ONLY=1
```

YOLO / vision bridge 可选依赖：

```text
ultralytics
opencv-python-headless
yolov8n.pt
```

如果 ISO 离线交付，建议提前 vendor：

- HarborGate Rust binary。
- HarborBeacon Rust release binaries。
- HarborNAS WebUI production dist。
- `yolov8n.pt`，如果启用 YOLO。
- VLM model directory，如果启用内部 VLM backend。

## 12. 安装目录与状态目录

建议默认目录：

```text
/var/lib/harborbeacon-agent-ci
/var/lib/harborbeacon-agent-ci/current
/var/lib/harborbeacon-agent-ci/runtime
/var/lib/harborbeacon-agent-ci/logs
/var/lib/harborbeacon-agent-ci/captures
/mnt/software/harborbeacon-agent-ci
/mnt/software/harborbeacon-models
```

用途：

| 路径 | 用途 |
|---|---|
| `/var/lib/harborbeacon-agent-ci` | exec-capable install root |
| `/var/lib/harborbeacon-agent-ci/current` | 当前 release symlink |
| `/var/lib/harborbeacon-agent-ci/runtime` | runtime state 根目录 |
| `/var/lib/harborbeacon-agent-ci/logs` | 服务日志辅助目录 |
| `/var/lib/harborbeacon-agent-ci/captures` | snapshot / video / artifact capture |
| `/mnt/software/harborbeacon-agent-ci` | HarborOS writable root |
| `/mnt/software/harborbeacon-models` | 模型缓存目录 |

状态隔离要求：

- HarborBeacon admin state、device registry、task conversations 独立保存。
- HarborGate sessions、platform credential state、Weixin state 独立保存。
- HarborNAS WebUI 不直接读写这些 state 文件，只通过 HTTP API。

## 13. 环境变量与凭据归属

建议环境文件：

```text
/etc/default/harborbeacon-agent-hub
```

核心配置：

```text
WORKSPACE_ROOT=/var/lib/harborbeacon-agent-ci/current
HARBOR_HTTP_BIND=0.0.0.0:4174
HARBOR_PUBLIC_ORIGIN=http://harborbeacon.local:4174
HARBOR_ASSISTANT_DIST=/var/lib/harborbeacon-agent-ci/current/harbor-assistant/dist/harbor-assistant
HARBOR_HARBOROS_USER=<service-user>
HARBOR_HARBOROS_WRITABLE_ROOT=/mnt/software/harborbeacon-agent-ci
HARBOR_KNOWLEDGE_INDEX_ROOT=/mnt/software/harborbeacon-agent-ci/knowledge-index

HARBOR_TASK_API_ADMIN_STATE=/var/lib/harborbeacon-agent-ci/runtime/admin-console.json
HARBOR_TASK_API_DEVICE_REGISTRY=/var/lib/harborbeacon-agent-ci/runtime/device-registry.json
HARBOR_TASK_API_CONVERSATIONS=/var/lib/harborbeacon-agent-ci/runtime/task-api-conversations.json
HARBOR_TASK_API_BEARER_TOKEN=<shared-token>

HARBOR_TASK_API_URL=http://127.0.0.1:4174
HARBORBEACON_WEB_API_URL=http://127.0.0.1:4174
HARBORBEACON_WEB_API_TOKEN=<shared-token>

HARBOR_MODEL_API_BASE_URL=http://127.0.0.1:4174/api/inference/v1
HARBOR_MODEL_API_TOKEN=<shared-token>
HARBOR_MODEL_API_BACKEND=openai_proxy
HARBOR_MODEL_API_UPSTREAM_BASE_URL=http://127.0.0.1:11434/v1
HARBOR_MODEL_API_CHAT_MODEL=harbor-local-chat
HARBOR_MODEL_API_EMBEDDING_MODEL=harbor-local-embed

HARBORGATE_BASE_URL=http://127.0.0.1:8787
HARBORGATE_BEARER_TOKEN=<shared-token>
IM_AGENT_SERVICE_TOKEN=<shared-token>
IM_AGENT_CONTRACT_VERSION=2.0
IM_AGENT_HOST=127.0.0.1
IM_AGENT_PORT=8787
IM_AGENT_DATA_DIR=/var/lib/harborbeacon-agent-ci/runtime/harborgate/sessions
IM_AGENT_STATE_DIR=/var/lib/harborbeacon-agent-ci/runtime/harborgate
IM_AGENT_PUBLIC_ORIGIN=http://harborbeacon.local:8787
WEIXIN_STATE_DIR=/var/lib/harborbeacon-agent-ci/runtime/harborgate/weixin

HARBORBEACON_ADMIN_API_URL=http://127.0.0.1:4174
HARBORBEACON_ADMIN_API_TOKEN=<shared-token>

HARBOR_RELEASE_INSTALL_ROOT=/var/lib/harborbeacon-agent-ci
HARBOR_LOG_ROOT=/var/lib/harborbeacon-agent-ci/logs
HARBOR_CAPTURE_ROOT=/var/lib/harborbeacon-agent-ci/captures
```

IM 平台凭据只属于 HarborGate。

Feishu 可选配置：

```text
FEISHU_APP_ID=<feishu-app-id>
FEISHU_APP_SECRET=<feishu-app-secret>
FEISHU_CONNECTION_MODE=websocket
FEISHU_ENABLE_LIVE_SEND=1
FEISHU_VERIFICATION_TOKEN=<optional-token>
FEISHU_ENCRYPT_KEY=<optional-key>
```

Weixin 可选配置：

```text
WEIXIN_STATE_DIR=/var/lib/harborbeacon-agent-ci/runtime/harborgate/weixin
WEIXIN_ACCOUNT_ID=<account-id>
WEIXIN_BOT_TOKEN=<bot-token>
WEIXIN_BASE_URL=https://ilinkai.weixin.qq.com
```

首次启用 Weixin 通常需要：

```bash
open http://<harborgate-host>:8787/setup/weixin
```

扫码、状态写入、poll runtime 都由 Rust `harboros-im-gate.service` 处理；release
bundle 不再提供单独的 Python login / runner helper。

## 14. Release bundle 建议内容

建议 HarborBeacon / HarborGate release bundle：

```text
bin/
  harborbeacon-service
  validate-contract-schemas
  run-e2e-suite

harborgate/
  bin/harborgate

templates/
  bin/
    harbor-agent-hub-helper
    harborgate
    run-harborbeacon-service
  systemd/
    harborbeacon.service.template
    harboros-im-gate.service.template
  harborbeacon-agent-hub.env.template

install/
  install_harboros_release.sh
  rollback_harboros_release.sh
  verify_release_bundle.py

media-tools/
  bin/ffmpeg
  bin/ffprobe
  NOTICE.txt
  provenance.json

manifest.json
checksums.sha256
```

HarborNAS WebUI dist 建议由 HarborNAS ISO 自身 WebUI 打包流程产出，包含：

```text
/ui/harbor-assistant
```

如果继续使用 `tools/build_release_bundle.sh`，注意它当前仍支持
`HARBOR_ASSISTANT_DIST_SOURCE` / `frontend/harbor-assistant` 过渡路径。ISO 正式集成时
应优先使用 HarborNAS WebUI production dist，并把 `frontend/harbor-assistant`
标注为 repo-local validation shell。

## 14.1 Debian carrier package 交付

HarborOS 同事可以优先从 GitHub Actions 下载
`harborbeacon-harboros-deb-<version>` artifact。该 artifact 内包含：

```text
harborbeacon-harboros-release_<version>_amd64.deb
harborbeacon-harboros-release_<version>_amd64.deb.sha256
```

如果仓库配置了 R2 secrets / vars，tag 或手工触发还会把同一份 `.deb`
上传到 R2 `deb/` 路径；R2 是可选分发面，不是唯一来源。

`.deb` 是 carrier package：`dpkg -i` 只安装 release bundle、installer、
verifier 和 helper，不自动启动或重启服务，也不写入现场凭据。HarborOS ISO
post-install 或 first-boot 流程需要显式调用：

```bash
sudo verify-harborbeacon-release --require-execute
sudo install-harborbeacon-release \
  --install-root /var/lib/harborbeacon-agent-ci \
  --writable-root /mnt/software/harborbeacon-agent-ci
```

`verify-harborbeacon-release` 校验 bundle 结构、checksums、manifest 和 bundled
`ffmpeg` / `ffprobe` 可执行性；`install-harborbeacon-release` 再调用 bundle 内
`install_harboros_release.sh` 完成 systemd、env-file、media tools 和 rollback
布局。

## 15. 安装与启停建议

安装脚本需要完成：

- 校验 `checksums.sha256`。
- 解包到 `/var/lib/harborbeacon-agent-ci/releases/<version>`。
- 更新 `/var/lib/harborbeacon-agent-ci/current` symlink。
- 写入 `/etc/default/harborbeacon-agent-hub`。
- 安装 bundled `ffmpeg` / `ffprobe` 到
  `/var/lib/harborbeacon-agent-ci/runtime/media-tools/bin/`。
- 写入 `HARBOR_FFMPEG_BIN` 与 `HARBOR_FFPROBE_BIN`，并确认二者 `-version`
  可执行。
- 安装 systemd units。
- `systemctl daemon-reload`。
- enable core services。
- 默认启动 core services。
- disable/remove legacy units：`agent-hub-admin-api.service`、`assistant-task-api.service`、
  `harbor-model-api.service`、`harbor-vlm-sidecar.service`、`harborgate-weixin-runner.service`。
- Weixin / Feishu 等 adapter runtime 由 `harboros-im-gate.service` 按配置在进程内启动。

Core services：

```text
harborbeacon.service
harboros-im-gate.service
```

## 16. 安装后验证命令

基础服务状态：

```bash
systemctl status harborbeacon.service
systemctl status harboros-im-gate.service
```

如果安装了 helper：

```bash
/var/lib/harborbeacon-agent-ci/bin/harbor-agent-hub-helper status
/var/lib/harborbeacon-agent-ci/bin/harbor-agent-hub-helper health
/var/lib/harborbeacon-agent-ci/bin/harbor-agent-hub-helper logs gateway --lines 120
```

基础 HTTP health：

```bash
curl http://127.0.0.1:4174/healthz
curl http://127.0.0.1:4174/api/inference/healthz
curl http://127.0.0.1:8787/health
```

HarborGate status：

```bash
curl http://127.0.0.1:8787/api/gateway/status \
  -H "X-Contract-Version: 2.0" \
  -H "Authorization: Bearer <shared-token>"
```

HarborBeacon turn API v2.0 smoke：

```bash
curl -X POST http://127.0.0.1:4174/api/web/turns \
  -H "Content-Type: application/json" \
  -H "X-Contract-Version: 2.0" \
  -H "Authorization: Bearer <shared-token>" \
  --data @turn-smoke.json
```

Harbor Assistant WebUI smoke：

```text
打开 /ui/harbor-assistant
确认页面请求 /api/beacon/state
确认后端实际转发到 http://127.0.0.1:4174/api/state
确认 Gateway、Knowledge、Models、Devices 面板不出现 proxy 404
```

Harbor Assistant 搜索 tab smoke：

```text
打开 /ui/harbor-assistant?tab=search
确认页面请求 /api/beacon/knowledge/search
确认后端实际转发到 http://127.0.0.1:4174/api/knowledge/search
确认返回结果包含 documents / images / videos / reply_pack
确认 degraded / blockers / warnings 能在 UI 中呈现
```

Harbor Assistant route smoke：

```text
GET /ui/harbor-assistant -> Harbor Assistant
确认 WebUI active route 清单只包含 Harbor Assistant 这一组 Harbor UI 入口
确认 WebUI active proxy 清单只包含 /api/beacon/** 这一组 HarborBeacon API 前缀
```

Media tools smoke：

```text
verify-harborbeacon-release --require-execute -> ok
harbor-agent-hub-helper health -> media_tools.ffmpeg.ok=true and media_tools.ffprobe.ok=true
GET /api/beacon/cameras/<device-id>/snapshot.jpg -> 200 image/jpeg
```

Release gate 工具：

```bash
validate-contract-schemas
run-e2e-suite
run-drift-matrix
evaluate-release-gate
```

## 17. v2.0 Release Gate

ISO 集成验收时必须确认：

- HarborGate active path 不再调用 `/api/tasks`。
- Active service-to-service traffic 使用 `X-Contract-Version: 2.0`。
- Active request builder 不再发出 public `args.resume_token`。
- HarborBeacon 不把 transport `session_id` 当作 business conversation truth。
- HarborGate 不解析 HarborBeacon `active_frame.kind` 来做业务路由。
- HarborBeacon 不保存 IM 原始平台凭据。
- HarborBeacon 不直接投递 IM 消息。
- Harbor Assistant `/ui/harbor-assistant` 通过 `/api/beacon/**` 访问 HarborBeacon Admin API。
- Harbor Assistant search tab 通过真实 `/api/beacon/knowledge/search` 获取索引结果。
- HarborNAS WebUI `/api/beacon/**` proxy 不覆盖 HarborOS 原生 `/api/**`。
- Harbor Assistant 显示 HarborBeacon unified inference health，不直接访问模型 sidecar 端口。
- Weixin group chat 不出现在本轮 readiness claim 中。

RC2 live smoke 参考：

- `GET /ui/harbor-assistant` -> `200`
- `GET /ui/harbor-assistant?tab=search` -> redirects/maps to Harbor Assistant search tab
- `GET /ui/harbor-assistant?tab=camera` -> redirects/maps to Harbor Assistant camera tab
- `POST /api/beacon/knowledge/search` query `春天的照片` -> one VLM
  content-indexed image, `filename_match_used=false`
- `GET /api/beacon/knowledge/preview` for that image -> `image/jpeg`
- protected `POST /api/web/turns` content retrieval -> `turn.status=completed`
- protected `POST /api/web/turns` local-first architecture explanation ->
  `turn.status=completed`

## 18. Local-First Promotion Gate

ISO 默认可以保留 `HARBOR_MODEL_API_BACKEND=openai_proxy`，但产品说法必须明确：

- 默认策略是 local-first。
- cloud 只有在 privacy/resource policy 放行时作为受控 fallback。
- SiliconFlow 是当前 `.82` fallback proof，不是默认架构。

只有当 `.82` local model benchmark report 里的 `gate.promotable=true`，才允许规划把默认 backend 切到 `candle` 的单独 cutover rehearsal。否则 ISO 应保留 openai-compatible seam 和 fallback policy，不把 local model 写成已默认启用。

## 19. 已知风险与需要 HarborNAS owner 确认的问题

已知风险：

- HarborGate README 标注：如果启用 `HARBORBEACON_TASK_API_URL`，必须确认代码已从历史 task client 切到 v2.0 `/api/turns` client；否则属于 release gate blocker。
- `tools/build_release_bundle.sh` 支持 `frontend/harbor-assistant` 验证 shell；正式 ISO 应以 `HarborNAS-webui` 的 `/ui/harbor-assistant` 为准。
- VLM / YOLO / 本地 LLM 模型较大，是否内置会显著影响 ISO 体积。
- `ffmpeg` 和 `ffprobe` 对 camera / video / snapshot / RTSP credential
  validation 很关键；若 release bundle 缺 `media-tools/bin/ffmpeg` 或
  `media-tools/bin/ffprobe`，installer 默认必须 fail-fast。
- `/mnt/software` 是否稳定可写会影响 writable root 默认值；如果不可写，安装脚本需要 fallback 到 `<install-root>/writable`。
- HarborBeacon 团队不拥有最终 ISO 构建流程；如果镜像侧需要改目录、
  proxy、服务 enable 策略或首启配置，应由 HarborOS / ISO owner 决策后
  反馈到本 handoff 文档。

需要 HarborNAS owner 决策：

- ISO 是否接受预编译 Rust 静态二进制 release bundle，而不是目标机编译。
- production `/api/beacon/**` proxy 由 nginx、middleware proxy，还是 HarborNAS WebUI 服务层实现。
- HarborGate Feishu setup portal / QR onboarding 是否直接暴露给 LAN 手机访问。
- Weixin runner 是否默认安装并 disabled，还是仅在用户开启 Weixin 后安装。
- 是否默认内置 YOLO / VLM 模型，还是首启后按需下载。
- HarborNAS WebUI 是否把 Harbor Assistant 放入默认导航，还是 feature flag 控制。
