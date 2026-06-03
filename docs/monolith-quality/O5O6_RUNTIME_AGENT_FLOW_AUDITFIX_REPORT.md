# O5/O6 Runtime Agent Flow Auditfix Report

## Scope

This slice implements the remaining findings from the latest runtime-agent-flow
re-audit. It preserves the product decision that there must be no separate Plan
Mode capability path: Plan Mode continues to call the same shared semantic plan
validation engine used by `vac plan validate`, and this slice adds the missing
hardening around owner-native support, lifecycle readiness, and command evidence.

## Root Cause Fixed

1. Owner-native Tier-1 runtime turn execution was already present, but Tier-2
   thread operations still degraded to deferred or empty behavior. `thread_list`,
   `thread_read`, `resume_thread`, and `branch_thread` now route through
   `ThreadStore`/`ThreadManager` source paths.
2. Runtime readiness was still partially inferred from brittle source text scans.
   `.vac/registry/runtime/owner-native-support.yaml` is now the structural
   support contract consumed by init doctor and runtime-owner gates.
3. Plan Mode had a pre-turn semantic gate. This slice adds a static completion
   gate hook after `Op::UserTurn` submission so the source path records B-pre and
   B-post semantics. Live after-stream timing remains TV-Pending.
4. Command gate/evidence wiring was already present, but this slice keeps it in
   the validation matrix and evidence chain so it is not treated as a separate or
   optional path.
5. Registry strictness did not know the new `runtime_owner_support` manifest
   kind. The kind is now registered in the strictness gate.

## Implementation Summary

- `vac-rs/tui/src/runtime_owner_session.rs`
  - Thread list/read/resume/branch now use `ThreadStore` and `ThreadManager`.
  - `read_account` no longer returns a critical `unsupported_report`.
  - Plan Mode keeps the shared semantic pre-gate and adds a static completion
    gate after turn submission.
- `vac-rs/cli/src/init_cli.rs`
  - `build_init_doctor_report` now uses the owner-native support manifest and
    active runtime plan instead of brittle source text scans.
- `vac-rs/cli/src/doctor/runtime_owner_gates.rs`
  - Adds support-manifest validation and critical unsupported regression checks.
- `.vac/registry/runtime/owner-native-support.yaml`
  - Declares release-blocking methods, Plan Mode pre/post status, and command
    gate/evidence status.

## Validation

PASS source/static:

- `scripts/check-vac-runtime-agent-flow-fix-static.sh`
- `scripts/check-vac-init-runtime-gate-callsite-integration.sh`
- `scripts/check-vac-runtime-engine-wiring-static.sh`
- `scripts/check-vac-init-why-contract.sh`
- `scripts/check-vac-init-evidence-chain-contract.sh`
- `scripts/check-legal-release-blockers.sh`
- `scripts/check-tui-source-artifact-hygiene.sh`
- `scripts/check-vac-o6-1-depanic-surface.sh`
- `scripts/check-vac-o6-2-safety-coverage.sh`
- `scripts/check-vac-o6-quality-triage.sh`
- `scripts/check-vac-o5-o6-completion-state.sh`
- `scripts/check-vac-o5-o6-monolith-quality-slice.sh`
- `scripts/check-vac-tui-visual-parity-static.sh`
- `scripts/check-tui-operator-snapshot-contract.sh`
- `scripts/check-vac-o5o6-all-audit-remainder.sh`
- `scripts/check-vac-init-registry-strictness-contract.sh`
- `scripts/check-vac-init-registry-validator-contract.sh` source/static preflight

## Status

- SV-Done: structural owner-native support gate, Tier-2 source routing, Plan Mode
  pre/post source hooks, init doctor de-brittling, command gate/evidence static
  verification.
- TV-Pending: `cargo check`, `cargo test`, `cargo clippy`, live interactive
  `vac` default turn, live Plan Mode gated turn, live thread list/read/resume/
  branch replay, and live command evidence positive/negative tests.

## Rollback

Restore `vac-o5o6-runtime-agent-flow-auditfix-checkpoint-013-source.zip` to keep
only the first Tier-2/runtime support patch, or restore
`vac-o5o6-runtime-agent-flow-followup-final-source.zip` to return to the prior
Plan Mode active-plan follow-up baseline.
