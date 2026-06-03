# VAC TUI Operator UI Hardening Plan 1-10

Status: active hardening plan  
Baseline: `vac-source-tui-operator-ui-batch3.zip`  
Scope: Batch 1-3 operator-console implementation hardening, not a feature expansion wave.

This document is the source-controlled plan for hardening the Batch 1-3 TUI work. The goal is to move the implementation from snapshot/source proof toward runtime-consistent, policy-safe, spec-driven operator UI behavior while keeping sandbox validation lightweight.

## Operating Rules

- Use the latest source artifact as the baseline.
- Treat every produced source zip as the commit boundary.
- Do not require full workspace build/link as a gate.
- Do not add dependencies unless unavoidable.
- Do not include `target/`, `.git/`, compiled objects, or cache output in source artifacts.
- Keep `.vac` as the control-plane source of truth for capabilities, surfaces, workflows, and gates.
- Prefer deterministic renderer, static contract checks, direct `rustc --test`, YAML parse, and snapshot harnesses.

## Hardening 1 — Baseline Audit & Regression Lock

Purpose: lock Batch 1-3 behavior before deeper refactor work.

Implementation:

- Add `scripts/check-tui-hardening-regression-lock.sh`.
- Re-run the operator UI contract, visual contract, snapshot contract, ANSI contract, live adapter contract, PTY contract, autopilot scheduler contract, and status output contract.
- Keep the validation gate small enough to run without full workspace linking.

Acceptance:

- All Batch 1-3 validation scripts pass.
- `/status` forbidden legacy rows remain absent.
- Snapshot/ANSI/layout gates remain deterministic.
- Source hygiene confirms no build output in source artifacts.

## Hardening 2 — Semantic Renderer Contract Cleanup

Purpose: replace keyword-only style inference with a semantic render contract.

Implementation:

- Introduce `OperatorSpanSpec` and `OperatorLineSpec` in `operator_style.rs`.
- Add `OperatorStyleRole::Plain` so semantic spans can preserve unstyled text.
- Add semantic plain and ANSI text emitters:
  - `operator_line_specs_to_plain_text`
  - `style_operator_text_specs`
- Add `render_operator_snapshot_specs` and `render_operator_snapshot_ansi_text` in `operator_ui.rs`.
- Add screen-aware semantic mapping via `OperatorSemanticScreen`.
- Add body-level semantic renderers for runtime jobs and capability dashboard:
  - `render_autopilot_scheduler_line_specs`
  - `render_capability_dashboard_shell_specs`
- Update the live ratatui adapter to prefer `style_operator_lines_from_specs`.
- Keep `style_operator_lines_from_strings` as compatibility fallback only.

Acceptance:

- Plain snapshots are unchanged.
- ANSI snapshots strip back to the exact plain snapshot.
- Approval popup marks `DESTRUCTIVE` as a danger span, not just a full-line keyword match.
- Runtime jobs and capability dashboard use semantic specs in live adapter wiring.

## Hardening 3 — `/status` Deep Hardening

Purpose: keep `/status` operator-safe and provider/model-focused.

Implemented in Hardening 3:

- Added `vac-rs/tui/src/status/output_contract.rs` as the dependency-light display contract.
- Centralized required/optional status labels in `StatusDisplayField`.
- Added `StatusProviderModelUsage` as the forward contract for registry-backed multi-provider/model rows.
- Made `Model provider` a required display row; default `vastar` is no longer suppressed.
- Added conditional `Model providers` registry display when multiple providers are available in `config.model_providers`.
- Removed the obsolete runtime rate-limit display helper path from `status/card.rs`.
- Set the `/status` slash command to local-only behavior: it no longer emits `AppEvent::RefreshRateLimits` and no longer queues status refresh handles.
- `add_status_output` now passes an explicit empty account-limit snapshot set so cached background limits cannot leak back into `/status`.
- Strengthened `scripts/check-tui-status-output-contract.sh` so it checks contract registration, required fields, forbidden runtime display fragments, snapshot hygiene, slash-command refresh policy, cached-limit isolation, and multi-provider/model readiness.
- Added docs: `docs/tui/OPERATOR_UI_HARDENING_03_STATUS_OUTPUT.md` and `docs/validation/TUI_STATUS_OUTPUT_GATE.md`.

Acceptance:

- Runtime status card does not contain forbidden display strings.
- Status snapshots do not contain removed `Limits:` / `Credits:` rows.
- Token usage and context window remain present.
- Active provider remains present even for default `vastar`.
- Multi-provider registry summary is available when provider registry data exists.
- Required labels come from the status display contract instead of ad-hoc renderer literals.
- `/status` does not trigger rate-limit or credit network refreshes, even for ChatGPT-authenticated sessions.

## Hardening 4 — Capability Dashboard Runtime Fidelity

Purpose: ensure the dashboard is manifest-driven, not decorative.

Implementation target:

- Build dashboard state from `.vac/capabilities`, `.vac/policies`, `.vac/surfaces`, and `.vac/workflows`.
- Maintain metrics: capabilities, owned domains, unowned domains, valid percent.
- Diagnostics must render YAML/control-plane errors instead of leaving a blank panel.
- Keep status enum aligned with current `.vac` states such as `ready`.

Acceptance:

- Dashboard snapshot contains metrics, registry summary, table rows, and diagnostics.
- YAML parse failure produces a visible diagnostics card.

## Hardening 5 — Agent Streaming Runtime Wiring

Purpose: ensure the snapshot model is used when the agent is running.

Implementation target:

- Conversation timeline renders user prompt, agent message, and turn metadata.
- Tool timeline displays only the five latest tools.
- Tool states remain `queued`, `running`, `streaming`, `passed`, `failed`, `cancelled`.
- Thinking/status line and context bar remain visible.
- Composer remains present during streaming.

Acceptance:

- `TOOL_TIMELINE_LIMIT = 5` is preserved.
- The runtime path slices from the tail of tool calls.
- Snapshot and static gates verify context usage and composer presence.

## Hardening 6 — Approval Popup Safety Contract

Purpose: keep approval UX explicit and policy-backed.

Implementation target:

- Approval popup shows action kind, command/operation, cwd, runtime/network/write context, risk, policy reason, batch progress, and hotkeys.
- Destructive bash commands show `DESTRUCTIVE` clearly.
- Renderer cannot auto-approve; it only displays a policy request/decision.

Acceptance:

- `DESTRUCTIVE`, `approve once`, `approve+remember`, `reject`, and `reject with reason` remain in the approval surface.
- No renderer code mutates approval policy or bypasses approval queue.

## Hardening 7 — Autopilot Scheduler Monitor-Only Contract

Purpose: make scheduler UI useful without turning it into an unsafe executor.

Implementation target:

- `/runtime` and `/runtime/jobs` show scheduler status, pid, uptime, mode, env, queued/running counts, job list, inspect card, tokens, spend, retry, and actions.
- Actions such as cancel/retry/open/attach remain policy-gated.
- Manifest declares monitor-only behavior by default.

Acceptance:

- `.vac/capabilities/autopilot-scheduler.yaml` and readiness workflow stay registered.
- Snapshot contains monitor-only policy text and action hints.

## Hardening 8 — Idle / First Launch Visual Fidelity

Purpose: improve terminal density and screenshot fidelity without hardcoding runtime state.

Implementation target:

- Keep header, startup snapshot, composer, and bottom statusline consistent across fixed viewports.
- Add 120x36, 140x40, and 180x48 matrix snapshots for target screens.
- Improve clipping, spacing, separators, and bottom chrome.

Acceptance:

- Snapshot matrix renders first launch, idle, agent working, approval popup, runtime jobs, and capability dashboard.
- The renderer clips without changing line count or losing composer/statusline.

## Hardening 9 — `.vac` Spec-Driven Consolidation

Purpose: ensure all TUI hardening is visible in the control plane.

Implementation target:

- Each hardening area has a capability manifest or is referenced by `vac.tui`.
- Surface routes and slash/palette entries are registered where relevant.
- Workflows reference validation scripts and docs.

Acceptance:

- YAML parse passes for all `.vac` files.
- Capability dashboard can surface the hardening capabilities.

## Hardening 10 — Source Hygiene & Artifact Discipline

Purpose: keep every source artifact safe to reuse as the next baseline.

Implementation target:

- Check for `target/`, `.git/`, `.rlib`, `.rmeta`, `.o`, and unexpectedly large files before zipping.
- Write manifest, validation log, and SHA256 with each artifact.

Acceptance:

- Source zip contains source/docs/scripts/snapshots only.
- No build/cache output is shipped.
- Validation log states passed and blocked gates honestly.

## Current Slice Status

- Hardening 1: implemented in this artifact via `scripts/check-tui-hardening-regression-lock.sh` and supporting gates.
- Hardening 2: implemented in this artifact via semantic span/line specs, semantic ANSI snapshots, and live adapter wiring.
- Hardening 3: implemented via the `/status` display contract, runtime helper cleanup, local-only slash-command behavior, cached-limit isolation, and stronger status output gate.
- Hardening 4-10: planned here and ready for subsequent source artifact slices.

## Hardening 4-10 Completion Addendum

Hardening 4-10 is implemented in `vac-source-tui-operator-ui-hardening4-10.zip` scope:

- Hardening 4: typed capability dashboard runtime state and diagnostics.
- Hardening 5: agent streaming runtime contract with last-five tool timeline and metadata.
- Hardening 6: explicit approval risk/safety model.
- Hardening 7: autopilot monitor-only policy-gated action model.
- Hardening 8: 120x36, 140x40, 180x48 plain/ANSI snapshot matrix.
- Hardening 9: `.vac` capability/workflow/surface consolidation.
- Hardening 10: source artifact hygiene gate.

Aggregate validation:

```bash
bash scripts/check-tui-hardening-4-10-contract.sh
```
