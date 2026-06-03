# Plan 23 Evidence — L-23OPS operator evidence status

Date: 2026-05-27
Lane: L-23OPS — Plan 23 operator evidence status
Repo: `/home/emp/Documents/VAC/vastar-agentic-cli`
Baseline checked: local `main` == `origin/main` at `da28de8c484d97065e6a6b7047f88c452bb011a0`
Result: `BLOCKED-OPERATOR`

## Scope

This lane updates Plan 23 docs/evidence only. It does not edit Rust code, `Cargo.toml`, or broad `.vac` manifests.

## Current source-of-truth paths

- Plan source of truth: `docs/workflow-control-plane/plans/23-pty-operator-gate.md`
- Manual PTY runbook: `docs/validation/TUI_PTY_DOGFOOD_GATE.md`
- Prior operator handoff evidence: `docs/workflow-control-plane/plans/23-evidence/2026-05-26-L19-real-operator-runbook.md`
- Current evidence path: `docs/workflow-control-plane/plans/23-evidence/2026-05-27-L23OPS-operator-evidence-status.md`

## Pre-flight facts captured in this lane

- `git fetch origin main` completed.
- Local `HEAD` matched `origin/main` at `da28de8c484d97065e6a6b7047f88c452bb011a0` before this docs-only edit.
- Baseline minimum `da28de8` is an ancestor of `HEAD`.
- Existing unrelated dirty work was present outside this lane, including scheduled-audit docs and `vac-rs/Cargo.lock`; no Plan 23 scoped file was dirty before this edit.
- Relevant docs were searched with the requested Plan 23/operator/evidence/gate terms.

## Operator evidence inventory

Plan 23 still requires captured operator evidence for a release-grade pass. The required evidence remains:

1. root `vac` launches in a real operator PTY,
2. alternate screen enters and restores cleanly,
3. `/` opens the slash-command list without layout crash,
4. normal safe input reaches the composer,
5. prompt submission is exercised only when safe test credentials/config are intentionally available,
6. approval/input overlays are exercised only when a deterministic fixture exists,
7. `Ctrl-C` exits cleanly and releases terminal state,
8. captured diagnostics are private/redacted,
9. the final result records `passed`, `failed`, `blocked_operator`, or `skipped_not_applicable`, with `blocked_operator` treated as non-pass evidence by release consumers.

## Current repo facts

The current repo already records the Plan 23 contract and handoff path:

- `.vac/capabilities/tui-pty-gate.yaml` declares `vac.tui.pty_gate` and includes `blocked_operator` in its states.
- `.vac/workflows/maintenance.tui-pty-gate.yaml` is present with `status: ready`, `mutates_files: false`, `network: false`, `redaction: true`, and a `validate` gate using `capability.tui.pty_gate`.
- `docs/validation/TUI_PTY_DOGFOOD_GATE.md` defines the required real-terminal verification flow and says `blocked_operator` is truthful non-pass evidence.
- `docs/workflow-control-plane/plans/23-evidence/2026-05-26-L19-real-operator-runbook.md` already records the human handoff checklist and keeps Plan 23 not release-complete until a real operator run is captured.

## Decision

Do **not** mark Plan 23 done from this lane. No new human-owned real PTY transcript, screenshot summary, log snippet, exit code evidence, or post-run working-tree evidence was provided in this session.

Plan 23 status remains:

```text
BLOCKED-OPERATOR: implementation and runbook are present, but real operator PTY evidence has not been captured in this lane.
```

## Deferred / blocker list

- Missing human/operator real PTY run on the current candidate ref.
- Missing redacted transcript or screenshot summary showing root `vac`, `/` slash list, safe composer input, and clean `Ctrl-C` exit.
- Missing `/tmp/vac-pty-gate.log` or equivalent redacted telemetry proving `StartTask` / runtime event flow for a prompt path when safe to run.
- Missing explicit exit-code capture and post-run `git status --short` from the real PTY session.
- Release promotion remains blocked on a `passed` Plan 23 evidence file or an explicit release policy decision accepting the non-pass `BLOCKED-OPERATOR` state.

## Next operator lane

Run the checklist in `docs/workflow-control-plane/plans/23-evidence/2026-05-26-L19-real-operator-runbook.md` from a real interactive terminal and save a new dated evidence file in `docs/workflow-control-plane/plans/23-evidence/` with `Result: PASS`, `FAIL`, or `BLOCKED-OPERATOR`.
