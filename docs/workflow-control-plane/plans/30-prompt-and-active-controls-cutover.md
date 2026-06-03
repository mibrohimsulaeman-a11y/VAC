# Plan 30 — Prompt submit and active controls cutover


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE_DEFAULT_PATH**

Source status lines:
> ## Current status
> Status: **complete for the default owner-native TUI product path** — Plan 30A–30G now have an owner-native operation parity registry in `vac-rs/tui/src/owner_native_operation_parity.rs`, the default TUI session path no longer carries release-blocking unimplemented operation gaps, and `BLOCKED-OPERATOR` is treated as non-pass release evidence rather than green evidence. Optional legacy app-server compatibility remains non-default quarantine/delete-defer material owned by Plan 33, not a Plan 30 runtime blocker.
> Slice status evidence:

Code evidence:
- `vac-rs/tui/src/owner_native_operation_parity.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/30-evidence/2026-05-25-30A-30E-completion.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-25-30F-operator-ux-gate.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-25-30G-skills-external-agent.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-25-wave3a-plan30-closeout-reconciliation.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-26-30G-blocker3-closure.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-26-30G-external-agent-import-completion.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-26-30G-plugin-provider.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-26-30G-provider-blockers.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-26-30G-residual-gap-audit.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-28-sandbox-owner-native-parity-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Make the local runtime owner the default path for prompt submit and active runtime controls.

This is the first plan where the user-visible chat path can move off app-server ownership, but only after Plans 27–29 are green.

## Current status

Status: **complete for the default owner-native TUI product path** — Plan 30A–30G now have an owner-native operation parity registry in `vac-rs/tui/src/owner_native_operation_parity.rs`, the default TUI session path no longer carries release-blocking unimplemented operation gaps, and `BLOCKED-OPERATOR` is treated as non-pass release evidence rather than green evidence. Optional legacy app-server compatibility remains non-default quarantine/delete-defer material owned by Plan 33, not a Plan 30 runtime blocker.

Slice status evidence:

- [30A–30E completion record](30-evidence/2026-05-25-30A-30E-completion.md)
- [30F operator UX gate evidence](30-evidence/2026-05-25-30F-operator-ux-gate.md)
- [30G skills/plugins + external-agent evidence](30-evidence/2026-05-25-30G-skills-external-agent.md)
- [30G provider blocker evidence](30-evidence/2026-05-26-30G-provider-blockers.md)
- [30G W5C plugin provider evidence](30-evidence/2026-05-26-30G-plugin-provider.md)
- [30G external-agent import completion evidence](30-evidence/2026-05-26-30G-external-agent-import-completion.md)
- [30G blocker #3 closure evidence](30-evidence/2026-05-26-30G-blocker3-closure.md)
- [Wave 3A closeout reconciliation](30-evidence/2026-05-25-wave3a-plan30-closeout-reconciliation.md)

- **30A** — implemented 2026-05-24; prompt submit/read-only-first owner command bus is in place.
- **30B** — implemented 2026-05-25; user-confirmed interrupt/steer/compact/shell controls.
- **30C** — implemented 2026-05-25; user-confirmed review + goals controls.
- **30D** — implemented 2026-05-25; user-confirmed realtime controls.
- **30E** — implemented 2026-05-25; user-confirmed config/account/memory controls.
- **30F** — terminal `BLOCKED-OPERATOR` accepted for planning purposes; no real TTY pass is claimed.
- **30G** — complete for default owner-native operation parity. Historical plugin/session/import blocker notes are retained below only as audit history; the active default path is represented by `vac-rs/tui/src/owner_native_operation_parity.rs`, and remaining remote/legacy compatibility is non-default Plan 33 material.
- **Plan 31** — complete for the default DTO owner-native path; no longer blocked by Plan 30 provider gaps.

This is a behavior-changing plan. It must not start until startup, server-request registry, and event stream replacement are proven.

## Target outcome

Prompt submit and active runtime controls default to the local runtime owner while TUI UX remains stable and unsupported controls are explicit blockers or defers.

## Outputs

- owner-backed `turn_start`,
- migrated active controls or explicit blockers,
- full TUI/core/CLI validation evidence,
- PTY operator evidence or `BLOCKED-OPERATOR`,
- rollback/fallback notes for any temporary seam.

## Requires / Blocks

Requires:

- Plan 27 retained-resource startup,
- Plan 28 server-request registry,
- Plan 29 event stream replacement.

Blocks:

- Plan 31 protocol compatibility retirement,
- Plan 33 app-server Cargo retirement.

## Prerequisites

- retained-resource startup is available,
- server-request registry is proven,
- event stream replacement is proven,
- current TUI compatibility adapter still preserves UX.

## Non-goals

- No protocol cleanup beyond what is required for cutover.
- No Cargo app-server dependency removal yet.
- No unsupported active control silently left on app-server in the final state.

## Cutover targets

### Prompt path

- `turn_start`
- bottom input submit
- normal prompt streaming
- approval/input during prompt execution

### Active controls

- turn steer
- turn interrupt
- startup interrupt
- realtime start/audio/stop
- shell command
- background terminal cleanup
- compact
- rollback or explicit defer
- review start
- thread goal get/set/clear
- memory mode set
- config reload
- logout
- memory reset

## Superseded historical notes

Older notes that say Plan 30G remains partial are retained as audit history only. They are superseded by the 2026-05-28 default owner-native parity closeout.

## Implementation slices

### 30A — Prompt submit behind feature/seam

Route prompt submit through local owner while retaining rollback to old path during validation.

**Read-only-first sub-step (absorbs umbrella slice 24G).** Before moving prompt submit, route one low-risk read-only command — thread list/read or account read — through the owner command bus. Prove the command bus shape with the read-only command first; only then move prompt submit. Skipping this makes the prompt-submit cutover a chat-only false-green.

Implementation note 2026-05-24: `vac-local-runtime-owner::RuntimeCommandBus` owns the read-only account/thread-list command shape and the owner-backed start-turn command. The TUI direct retained-manager path now reads account state through the owner bus before routing prompt submit through the same bus; the app-server backend remains the rollback path when retained managers are unavailable.

### 30B — Interrupt/steer/compact/shell controls

Move core chat controls and prove they affect the active runtime operation.

Implementation note 2026-05-25: P-30B moved the direct retained-manager path for `turn_interrupt`, `turn_steer`, `thread_compact_start`, and `thread_shell_command` behind `vac-local-runtime-owner::RuntimeCommandBus`; this slice is user-confirmed implemented. App-server protocol requests remain the explicit fallback when retained managers are unavailable; startup interrupt still delegates through the owner-backed turn interrupt path. Manual TTY product smoke remains pending for 30F.

### 30C — Review and goals

Move review start and thread goal operations, preserving validation and UI state.

Implementation note 2026-05-25: P-30C routes direct local-runtime `review/start` and thread goal get/set/clear through `RuntimeCommandBus` while preserving explicit app-server fallback when retained managers are unavailable; this slice is user-confirmed implemented. Review target validation and thread goal objective/budget validation stay enforced on the owner path before runtime mutation; TUI real manual smoke remains pending.

### 30D — Realtime controls

Move realtime start/audio/stop with feature-gate checks and error behavior preserved.

Implementation note 2026-05-25: P-30D routes direct local-runtime realtime start/audio/stop through `RuntimeCommandBus` commands while preserving the explicit app-server backend fallback for non-retained sessions; this slice is user-confirmed implemented. The owner command bus performs the `RealtimeConversation` feature-gate check before submitting realtime core ops, so disabled realtime remains a hard error rather than a bypass. Manual TTY/audio smoke remains pending; do not treat this slice as product-smoke final.

### 30E — Config/account/memory controls

Move reload/logout/memory reset/memory mode where lower-level APIs are ready. Explicitly defer anything still blocked.

Implementation note 2026-05-25: direct TUI config reload, account logout, memory reset, and thread memory-mode set now cross `vac-local-runtime-owner::RuntimeCommandBus` before touching retained local-runtime resources; this slice is user-confirmed implemented. The app-server `config/batchWrite`, `account/logout`, `memory/reset`, and `thread/memoryMode/set` requests remain explicit fallback paths when retained managers/config are unavailable; account read already used the owner read command from 30A, and rate-limit surfaces remain app-server sourced.

### 30F — Operator UX gate

Run real TTY test or record `BLOCKED-OPERATOR`.

Implementation note 2026-05-25 Wave 3A: [30F evidence](30-evidence/2026-05-25-30F-operator-ux-gate.md) records `BLOCKED-OPERATOR: no interactive TTY available in this environment`. This is accepted as the terminal 30F planning state, not as a product-smoke pass. If release policy later requires a real TTY pass, that is a separate operator gate and must not be inferred from this record.

### 30G — Skills/plugins surface and external-agent config

Migrate skills/plugins surface and external-agent config to the local runtime owner. **Absorbs umbrella capability rows "Skills/plugins" and "External-agent config".**

Implementation note 2026-05-25: P-30G added owner command-bus shapes for skills, plugins, and external-agent config. Current-cwd `skills/list` uses `RuntimeCommandBus`; arbitrary-cwd skills now resolves per-cwd config through the owner bus. Plugin surface still fails closed in the owner bus with `PluginSurfaceOwnerProviderRequired(operation)`. External-agent config detect/import now has an owner-native provider path for normal detect/import, while background plugin/session import completion remains explicit blocker work. Historical 2026-05-25 note superseded by 2026-05-28 owner-native parity closeout.

W5 verification 2026-05-25: arbitrary-cwd `skills/list` now resolves per-cwd config and routes through the owner command bus. W5B verification 2026-05-26: external-agent detect/import DTOs and provider extraction now compile through `vac-local-runtime-owner`; TUI retained/direct paths route detect/import through `RuntimeCommandBus` and retain compatibility fallback only for explicit background import or unexpected owner errors. Plugin operations still return `PluginSurfaceOwnerProviderRequired(operation)`. Historical 2026-05-26 note superseded by 2026-05-28 owner-native parity closeout for the default path.

Historical W3F decision 2026-05-25: provider extraction was deferred when the workspace was dirty and provider-target files were not isolated. Supersession 2026-05-28: Plan 30G is complete for the default owner-native operation parity registry, and Plan 31 has closed the default DTO owner-native path.

W5B provider review 2026-05-26: [30G provider blocker evidence](30-evidence/2026-05-26-30G-provider-blockers.md) records the narrowed remaining blockers. W5C verification 2026-05-26: [30G W5C plugin provider evidence](30-evidence/2026-05-26-30G-plugin-provider.md) records owner-native retained/direct local plugin list/read/install/uninstall/set-enabled routing through `RuntimeCommandBus`; remote marketplace plugin parity and external-agent import completion still lack owner-owned contracts.

L1-continuation 2026-05-26: [30G external-agent import completion evidence](30-evidence/2026-05-26-30G-external-agent-import-completion.md) records the chosen owner-native blocking-equivalent shape for plugin import completion. Supersession 2026-05-28: the default path is governed by the owner-native parity registry; retained/direct fallback wiring is non-default compatibility material rather than a Plan 30 blocker.

L24 2026-05-26: [30G blocker #3 closure evidence](30-evidence/2026-05-26-30G-blocker3-closure.md) re-ran the overlap pre-flight and inspected allowed `background_requests.rs` fallback candidates. Supersession 2026-05-28: default-path parity is closed; remaining retained/direct compatibility is non-default Plan 33 material.

- skills list / current-cwd path through owner,
- arbitrary-cwd fallback is an explicit blocker if lower-level config resolution is not ready; do not silently route through app-server,
- external-agent config: isolate lower-level provider before the app-server edge can be removed (Plan 33 prerequisite),
- onboarding/attach UX must remain stable.

If lower-level APIs are not ready, mark the unsupported path as an explicit blocker in this slice. Do not silently fall back to app-server in the final state.

## Validation

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
```

PTY gate:

```text
launch ./target/debug/vac
type /
verify slash-command list visible
submit a normal prompt
verify streaming/activity/progress visible
exercise approval/input if available
exercise interrupt if safe
press Ctrl-C
verify clean exit
```

## Stop conditions

Stop if:

- approvals or user input regress during prompt execution,
- active controls silently fall back to app-server in the final path,
- no real PTY is available and the result is not recorded as `BLOCKED-OPERATOR`,
- Ctrl-C/shutdown cannot be verified.

## Done criteria

- Prompt submit defaults to local runtime owner.
- Active controls either run through local owner or have explicit blockers.
- TUI UX remains stable.
- PTY is passed or `BLOCKED-OPERATOR` is recorded.


## 2026-05-28 sandbox closeout

- Default owner-native parity evidence: [30-evidence/2026-05-28-sandbox-owner-native-parity-closeout.md](30-evidence/2026-05-28-sandbox-owner-native-parity-closeout.md).
- Remaining legacy app-server compatibility is non-default Plan 33 delete/defer evidence, not a default runtime path.
