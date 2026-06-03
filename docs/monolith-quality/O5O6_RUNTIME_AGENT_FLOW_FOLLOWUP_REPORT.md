# O5/O6 Runtime Agent Flow Follow-up Report

## Scope

This follow-up hardens the runtime-agent-flow slice after the first source/static
implementation. It applies the explicit product decision that Plan Mode must not
be a separated capability path: runtime Plan Mode now requires a canonical active
semantic plan and gates the turn through the same shared validation engine used
by `vac plan validate`.

## Root Cause Fixed

1. `AppServerBootstrap` carried a duplicate `plan_type` field in the first
   runtime-flow patch. That is a compile-level defect and is now removed.
2. `assert_owner_runtime_supported` only bound the required-method table to an
   unused local variable. It is now a real support table check, so the static
   gate rejects the previous no-op pattern.
3. Plan Mode validation called the shared validator, but the active plan path was
   implicit and missing from the source artifact. Runtime Plan Mode now requires
   `.vac/registry/runtime/active_plan.yaml` and fails closed with an explicit
   error when the plan is absent.
4. Adding the active runtime plan changed the O6.1 runtime-file inventory from
   1253 to 1254 while the actual panic-surface counts stayed unchanged. The O6.1
   registry is re-pinned with the new scanner hash.
5. The registry strictness scan was bounded to exclude generated `.vac/.init/risk_findings/**` payloads; those large scanner artifacts remain covered by scanner-specific gates, while strictness focuses on source-controlled manifests and init state.
6. The aggregate O5/O6 gate duplicated the expensive O6.1 scan and could time out
   in source-only sandboxes. The dedicated O6.1 gate still rescans; the aggregate
   gate now uses `VAC_O6_1_RESCAN=0` and checks the pinned registry values.

## Validation

PASS source/static:

- `scripts/check-vac-runtime-agent-flow-fix-static.sh`
- `scripts/check-vac-o6-1-depanic-surface.sh`
- `scripts/check-vac-o5o6-all-audit-remainder.sh`
- `scripts/check-vac-o6-quality-triage.sh`
- `VAC_O6_1_RESCAN=0 scripts/check-vac-o5-o6-monolith-quality-slice.sh`
- `scripts/check-tui-source-artifact-hygiene.sh`
- `scripts/check-vac-o5-o6-completion-state.sh`
- `scripts/check-vac-o6-2-safety-coverage.sh`
- `scripts/check-vac-init-runtime-gate-callsite-integration.sh`
- `scripts/check-vac-runtime-engine-wiring-static.sh`
- `scripts/check-legal-release-blockers.sh`
- `scripts/check-vac-init-why-contract.sh`
- `scripts/check-vac-init-evidence-chain-contract.sh`
- `scripts/check-vac-init-registry-strictness-contract.sh`
- `scripts/check-vac-tui-visual-parity-static.sh`
- `scripts/check-tui-operator-snapshot-contract.sh`

## Status

- SV-Done: source/static wiring, active Plan Mode plan gate, no-op guard removal,
  O6.1 re-pin, aggregate timeout hardening.
- TV-Pending: `cargo check`, `cargo test`, `cargo clippy`, live interactive `vac`
  smoke test, live Plan Mode gated turn, and live command/evidence positive and
  negative tests.

## Rollback

Restore `vac-o5o6-runtime-agent-flow-followup-checkpoint-011-source.zip` to the
state before the O6.1 aggregate optimization, or restore
`vac-o5o6-runtime-agent-flow-fix-final-source.zip` to return to the previous
runtime-agent-flow patch.
