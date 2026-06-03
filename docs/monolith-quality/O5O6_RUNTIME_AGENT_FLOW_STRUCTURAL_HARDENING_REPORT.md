# O5/O6 Runtime Agent Flow Structural Hardening

## Scope

Continuation after `O5O6_RUNTIME_AGENT_FLOW_AUDITFIX_REPORT.md`.

The goal is to remove the remaining brittle readiness inference around the local
owner-native runtime path and keep runtime Plan Mode on the same semantic plan
validation path as `vac plan validate`.

## Root cause

The previous auditfix already implemented Tier-2 thread routing and Plan Mode
pre/post semantic validation anchors, but the readiness guard still had two weak
spots:

1. `OwnerNativeSession` used a local all-true method-support table.
2. Runtime owner doctor/readiness had to trust a support manifest plus scattered
   source anchors.

That made the gate source/static valid but not structurally clear enough.

## Implementation

- Added `vac-rs/tui/src/owner_native_runtime_support.rs` as the code-owned support
  contract for release-blocking owner-native runtime methods.
- Updated `OwnerNativeSession::assert_owner_runtime_supported` to consume
  `release_blocking_owner_runtime_methods()` from the support contract.
- Updated `.vac/registry/runtime/owner-native-support.yaml` to point at the code
  contract and operation parity registry.
- Aligned `owner_native_operation_parity.rs` so non-critical unsupported controls
  are `NonDefaultFailClosed`, not falsely marked fully owner-native.
- Updated `runtime_owner_gates.rs` to validate the support manifest against the
  code-owned support contract and parity registry anchors.
- Optimized `check-vac-init-registry-validator-contract.sh` to skip generated
  risk-finding payloads owned by scanner gates, preventing no-rustc sandbox
  timeouts.
- Re-pinned O6.1 metadata after adding the runtime support contract source file.

## Validation

Source/static gates run in this sandbox:

- `scripts/check-vac-runtime-agent-flow-fix-static.sh` — PASS
- `scripts/check-vac-init-registry-validator-contract.sh` — PASS static; rustc unit NotEvaluated
- `scripts/check-vac-o6-1-depanic-surface.sh` — PASS
- `scripts/check-vac-init-registry-strictness-contract.sh` — PASS

## SV / TV status

- SV-Done: structural support contract, parity alignment, registry validator timeout fix, O6.1 re-pin.
- TV-Pending: `cargo check`, targeted Rust unit tests, live interactive `vac` default turn, live Plan Mode gated turn, live command evidence positive/negative tests.

## Rollback

Restore `vac-o5o6-runtime-agent-flow-auditfix-final-source.zip` or checkpoint 015,
then reapply this slice with cargo-backed feedback.
