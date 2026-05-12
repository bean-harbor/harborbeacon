# HarborBeacon

## Behavioral guidelines
Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.


## Project Focus

HarborBeacon is the business-core repo in a multi-repo Harbor system.

Current priority:

- execute the HarborBeacon x HarborGate Contract v2.0 upgrade control pack
- replace the v1.5 IM seam with the v2.0 turn / conversation / continuation seam
- keep repo ownership boundaries frozen while the public HTTP/JSON contract moves

HarborBeacon owns:

- task and business state
- resumable workflow state
- approvals
- artifacts
- audit trail
- business conversation continuity

HarborGate owns:

- IM adapters and transport
- route key lifecycle
- platform credentials
- outbound delivery

## Read First

Before changing architecture, contracts, routing, approval flow, or cutover logic, read:

1. `C:\Users\beanw\OpenSource\HarborGate\HarborBeacon-HarborGate-Agent-Contract-v2.0.md`
2. `HarborBeacon-HarborGate-v2.0-Upgrade-Runbook.md`
3. `HarborBeacon-Harbor-Collaboration-Contract-v2.md`
4. `HarborBeacon-LocalAgent-Roadmap.md`
5. `HarborBeacon-LocalAgent-Plan.md`
6. `HarborBeacon-LocalAgent-V2-Assistant-Skills-Roadmap.md`
7. `HarborBeacon-Skill-Spec-v1.md`

If work touches the HarborBeacon <-> HarborGate HTTP boundary, also read:

- `C:\Users\beanw\OpenSource\HarborGate\HarborBeacon-HarborGate-Agent-Contract-v2.0.md`
- `C:\Users\beanw\OpenSource\HarborGate\HarborBeacon-HarborGate-v2.0-Upgrade-Runbook.md`

## Hard Boundaries

- Do not import HarborGate runtime code into HarborBeacon.
- Do not share runtime state files across repos.
- Do not add long-term IM platform credential ownership back into HarborBeacon.
- Do not add direct IM message delivery back into HarborBeacon after cutover.
- Do not interpret `route_key` platform semantics inside HarborBeacon.
- Do not reintroduce `args.resume_token` as the public continuation mechanism.
- Do not treat transport `session_id` as HarborBeacon business conversation truth.
- Do not add v1.5/v2.0 runtime dual-stack compatibility unless the user explicitly reverses the current v2.0 direct-upgrade decision.
- Do not add group chat to this upgrade wave.
- Do not collapse AIoT device control into HarborOS system control.
- Preserve the southbound priority: `middleware API -> midcli -> browser -> MCP`.

## Ownership Map

- `harbor-framework`: shared runtime, task/session lifecycle, approvals, artifacts, audit, orchestration, local inference, account/workspace state
- `harbor-im-gateway`: external IM repo ownership, transport, route keys, platform payloads, credentials, delivery
- `harbor-hos-control`: HarborOS System Domain, middleware integration, `midcli`, system control paths
- `harbor-aiot`: Home Device Domain, camera and LAN AIoT native adapters, ONVIF/RTSP/vendor-cloud/device protocols
- `harbor-architect`: cross-lane boundary changes, release gates, rollback gates, cutover sequencing, final acceptance

Escalate to architecture mode when a change crosses lanes or widens a frozen contract.

## Default Working Style

Use these operating patterns on every non-trivial task:

1. Context engineering
   Load only the docs and files relevant to the task before editing. For cross-boundary work, start from the contract docs, then read the exact source and tests involved.
2. API and interface design
   Treat public HTTP/JSON contracts, turn envelopes, approval semantics, audit records, and skill manifests as contract-first surfaces. For the current IM seam, v2.0 is an explicitly approved breaking upgrade; do not add v1.5 compatibility scaffolding.
3. Deprecation and migration
   For cutovers and legacy-path removal, use feature flags, explicit migration steps, rollback notes, and removal checklists. Do not delete old paths until the replacement is verified.
4. Test-driven development
   For behavior changes and bug fixes, write or update the failing test first. Prefer boundary-focused tests in `tests/contracts`, `tests/fallback`, `tests/policy`, `tests/test_orchestrator`, and `tests/test_skills`.
5. Debugging and error recovery
   Stop on unexpected failures. Preserve the exact error, reproduce it, localize the failing layer, fix the root cause, and add a regression guard.
6. CI/CD and automation
   Keep contract checks, fallback checks, release-gate tooling, and workflow automation aligned. Do not relax quality gates to make a change pass.

## User Operating Preference

- After required target-registry confirmation has been satisfied for the day, execute requested HarborBeacon validation/deploy/live-test steps directly instead of re-confirming each substep.
- When a live target, credential, deployment, or external service problem blocks the requested path, stop and ask the user for direction instead of trying multiple unrelated recovery attempts.
- Keep normal local compile/test/debug loops autonomous when the issue is clearly inside the current code change and does not require new target, credential, or operational decisions.
- When the user says "WebUI" or "Webui" in HarborOS validation or product review context, default to the real HarborOS `.82` WebUI rather than a local `127.0.0.1:4174` Harbor Assistant/admin debug page.

## Verification

Run the smallest relevant verification set, then widen if the change touches shared behavior:

- Rust build: `cargo build --release`
- Rust tests: `cargo test`
- Python tests: `pytest`
- Harbor Assistant WebUI build: run `corepack yarn build:prod` in the synced `HarborNAS-webui` repo when the integrated WebUI changes

Use the contract and release tools when relevant:

- `target/release/validate-contract-schemas`
- `target/release/run-e2e-suite`
- `target/release/run-drift-matrix`
- `target/release/evaluate-release-gate`

## v2.0 Drift Guards

Current active-path drift checks must fail the work if any of these remain after
the relevant upgrade phase:

- `X-Contract-Version: 1.5` on active HarborGate/HarborBeacon traffic
- HarborGate active calls to `/api/tasks`
- public `args.resume_token` continuation emission
- HarborBeacon business state keyed by transport `source.session_id`
- HarborGate routing on Beacon business `active_frame.kind`
- group-chat readiness claims

## HarborBeacon-Specific Reminders

- This repo already has a product/runtime `skills/` system. Do not confuse runtime skills with development-process skills.
- External agent-skill packs should guide how the coding agent works; they should not be dropped directly into HarborBeacon's runtime skill registry without review.
- When modifying `src/skills/`, `skills/`, planner, router, or policy code, verify both the runtime contract and the lane boundary.
