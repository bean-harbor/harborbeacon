# HarborNavi MLP/VPF P0 Branch

## Status

- Branch: `harbornavi/mlp-vpf-p0`
- Created for: HarborNavi K3 P0 issue `Bean-Harbor/HarborNavi#18`
- Created on: 2026-05-27
- Owning lane: `harbor-framework`
- Architecture owner: `harbor-architect`
- Current code guard commit:
  `5ce91a3 Add HarborNavi MLP VPF privacy guards`
- Current K3 package/deployment commit:
  `b7fcfd5 Add HarborNavi K3 riscv64 package path`

This branch is the HarborBeacon implementation lane for HarborNavi model
lifecycle and visual privacy policy work. It is not a fork of HarborNavi and
must not copy HarborNavi product orchestration into HarborBeacon.

## Purpose

Implement the HarborNavi P0 rules for:

- `MLP` / Model Lifecycle Policy.
- `VPF` / Visual Privacy Filter.
- `NSP` as the resident local semantic parser lane.
- Embedding, VLM, and LLM as on-demand model lanes.
- Redacted cloud fallback for LLM and VLM only.
- Local-only execution for NSP, Embedding, HA service execution, camera control,
  HarborOS commands, and device control.

The first code increment implements policy, manifest, audit, and fail-closed
guards. The follow-up K3 package increment proves those guards can run as the
real `harboros-beacon.service` on K3. The full ONNX image-redaction engine is
still a separate follow-up.

## Boundary Rules

- HarborNavi owns product acceptance, GitHub Project tracking, SKU policy, and
  K3 validation reports.
- HarborBeacon owns model-center policy, model endpoint selection, redaction
  gate enforcement, candidate text/fact generation, and audit records.
- AIoT and HarborLink own camera snapshot, still-frame, keyframe, and camera
  artifact acquisition.
- VPF may consume local media artifacts and create redacted derivatives, but it
  must not own camera capture, camera control, live transport, or raw media
  retention.
- HarborGate owns IM delivery only and must not own model policy or visual
  privacy semantics.
- HarborOS System Domain work remains separate from Home Device Domain work.

This branch may update the older HarborBeacon cloud-fallback rule that excluded
VLM from cloud fallback. The new HarborNavi rule is narrower and explicit:
cloud VLM is allowed only after VPF produces a `cloud_safe=true` redaction
manifest and the payload scan passes.

## Merge Criteria

Before this branch can merge back to HarborBeacon mainline:

- Route-policy tests prove `semantic.router` / NSP stays local-only for
  HarborNavi K3 profile.
- Embedding route tests prove cloud endpoints are not selectable.
- LLM fallback tests prove cloud selection requires approved privacy policy and
  audit does not persist API keys or complete sensitive prompts.
- VLM fallback tests prove missing, invalid, or unsafe VPF manifests block cloud
  endpoint selection.
- Payload-scan tests reject RTSP URLs, HA tokens, camera credentials, local
  paths, API keys, and original image payload references.
- Existing model endpoint redaction and local-first fallback tests continue to
  pass.
- Documentation links HarborBeacon implementation evidence back to
  `Bean-Harbor/HarborNavi#18`.

Rollback is simple before merge: drop this branch. After merge, rollback means
reverting the MLP/VPF policy and guard commits as one unit, without changing the
HarborGate IM contract or AIoT camera ownership.

## K3 RISC-V Build Lane

K3 is a RISC-V board, so HarborBeacon binaries that run on K3 must be built for:

```text
riscv64gc-unknown-linux-gnu
```

The existing HarborBeacon release runbooks and GitHub release workflow are
currently x86_64-oriented. They must not be treated as K3 artifacts.

Recommended build route:

- Build host: `.197` / HarborBeacon build host.
- Rust target: `riscv64gc-unknown-linux-gnu`.
- Linker/toolchain: `riscv64-linux-gnu-gcc` and matching GNU binutils.
- K3 native build remains a diagnostic fallback, not the preferred release
  lane.

Builder setup commands:

```bash
rustup target add riscv64gc-unknown-linux-gnu
sudo apt-get update
sudo apt-get install -y gcc-riscv64-linux-gnu g++-riscv64-linux-gnu libc6-dev-riscv64-cross
```

Cargo linker configuration:

```toml
[target.riscv64gc-unknown-linux-gnu]
linker = "riscv64-linux-gnu-gcc"
```

Minimum smoke command:

```bash
cargo build --target riscv64gc-unknown-linux-gnu --bin harbor-model-api
```

The first RISC-V build report should record:

- builder host and checkout path;
- `rustc`, `cargo`, and `rustup target list --installed`;
- `riscv64-linux-gnu-gcc --version`;
- build command and result;
- dependency failures, especially TLS, native C dependencies, linker, Candle,
  and `reqwest` issues.

Initial builder precheck:

- `docs/harbornavi-k3-riscv-precheck-2026-05-27.md`

First K3 service deployment:

- `docs/harbornavi-k3-deployment-2026-05-27.md`

## Implemented P0 Guard Slice

Commit `5ce91a3` implements the first HarborNavi MLP/VPF code slice:

- `runtime::privacy` defines `RedactionManifest`, VLM redaction context
  validation, fail-closed error reasons, and cloud payload scanning.
- `run_vlm_summary_with_context` allows cloud VLM only with a valid VPF
  manifest, `cloud_safe=true`, `metadata_stripped=true`, distinct
  source/redacted artifact ids, and a payload scan pass.
- The legacy `run_vlm_summary_with_state` path remains available but cannot
  silently choose cloud VLM because it has no redaction context.
- Cloud VLM reads the redacted artifact path from context and keeps local image
  paths out of cloud diagnostics.
- `semantic.router` / NSP and `retrieval.embed` are fixed local-only at
  endpoint resolution time; cloud endpoints are not selectable for those
  routes.
- LLM cloud fallback remains available for approved text routes and records
  redacted audit policy markers without exposing API keys.

Validation completed:

```text
local: cargo test --lib privacy
local: cargo test --lib model_center -- --test-threads=1
local: cargo run --bin harbornavi-k3-guard-smoke
.197: cargo test --lib privacy --quiet
.197: cargo test --lib model_center -- --test-threads=1
.197: CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc \
        cargo build --target riscv64gc-unknown-linux-gnu --bin harbor-model-api
.197: scripts/build_harbornavi_k3_deb.sh produced
      harboros-beacon_harbornavi-p0-20260527+riscv64_riscv64.deb
K3: dpkg -i installed the riscv64 package
K3: harboros-beacon.service active on 127.0.0.1:4174
K3: /healthz returned HTTP 200
K3: /usr/bin/harbornavi-k3-guard-smoke ok=true
K3: journal/dmesg runtime and secret scans returned 0 matches
```

## Next Implementation Step

After the policy gate is reviewed, implement the real VPF engine behind the same
guard:

- local face/head/license plate/OCR detector selection;
- redacted artifact generation and manifest persistence;
- integration from `camera.analyze` / snapshot artifacts into
  `run_vlm_summary_with_context`;
- fixture coverage for fail-closed redaction errors and cloud payload scans.
