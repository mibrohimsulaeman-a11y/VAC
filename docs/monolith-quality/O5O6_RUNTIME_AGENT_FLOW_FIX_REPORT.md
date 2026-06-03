# O5/O6 Runtime Entrypoints and Agent Flow Fix Report

Status: **SV-Done / TV-Pending**
Date: 2026-06-01
Artifact baseline: `vac-o5o6-tui-widget-style-output-path-final-source.zip`

## Scope

This slice fixes the runtime audit findings for the interactive `vac` owner-native path and the agent execution flow:

1. Replace critical `OwnerNativeSession` startup/turn/shell unsupported stubs.
2. Add a guard rail that rejects critical owner-runtime unsupported regressions.
3. Keep command runtime gate/evidence callsites visible and bound.
4. Replace `vac init` `pending_runtime_cli` doctor placeholder with a concrete doctor report.
5. Integrate `vac plan validate` into the existing runtime Plan Mode path instead of leaving plan validation as a separate capability path.

## Architectural decision

Plan validation is no longer treated as a separate capability lane when the TUI is in Plan Mode. The runtime owner path checks `collaboration_mode == ModeKind::Plan` and invokes the shared `validate_vac_init_plan_yaml_with_engine(...)` before submitting `Op::UserTurn`. The CLI `vac plan validate` remains a wrapper around the same engine, but the runtime Plan Mode path is now a first-class gate.

The runtime Plan Mode gate is intentionally fail-closed. It validates `.vac/registry/runtime/active_plan.yaml` from the current VAC root before execution. If the active plan is missing or invalid, the turn is blocked before it reaches the core session.

## Implemented source changes

### `vac-rs/tui/src/runtime_owner_session.rs`

- `InProcessAppServerClient` now owns a `ThreadManager` built from config/auth/thread store.
- `OwnerNativeSession::start_thread_with_session_start_source` starts a real thread and forwards core runtime events.
- `OwnerNativeSession::turn_start` submits `Op::UserTurn` with model, sandbox, approval policy, permission profile, output schema, personality, and collaboration mode preserved.
- Runtime Plan Mode calls `validate_vac_init_plan_yaml_with_engine(...)` before `Op::UserTurn`.
- `turn_steer` uses `VACThread::steer_input`.
- `turn_interrupt` and `startup_interrupt` submit `Op::Interrupt`.
- `thread_shell_command` submits `Op::RunUserShellCommand` after rejecting empty commands.
- `thread_compact_start` and `thread_background_terminals_clean` submit their matching runtime operations.
- Non-critical methods degrade explicitly (empty/no-op/deferred error) instead of using critical startup/turn unsupported stubs.

### `vac-rs/core/src/runtime_gate_callsite_integration.rs`

Adds production source anchors for:

- `evaluate_vac_init_pre_plan_gate`
- `evaluate_vac_init_pre_patch_gate`
- `evaluate_vac_init_pre_command_gate`
- `evaluate_vac_init_evidence_completion_gate`

These anchors keep the exported runtime boundaries statically provable outside the control-plane module itself.

### `vac-rs/cli/src/init_cli.rs`

- Adds `build_init_doctor_report(...)`.
- Writes a real `doctor_report.yaml` with registry, policy, ownership, runtime owner, command gate evidence, and Plan Mode runtime gate checks.
- `DoctorVerified` and `Ready` are now gated by doctor pass/fail, not by a placeholder report.
- Removes `pending_runtime_cli` readiness semantics.

### `scripts/check-vac-runtime-agent-flow-fix-static.sh`

Adds a static regression gate that:

- requires ThreadManager-backed owner-native startup/turn/shell paths;
- rejects critical unsupported owner-runtime methods;
- requires runtime Plan Mode to use the shared plan validation engine;
- verifies doctor readiness no longer uses `pending_runtime_cli`;
- verifies command gate/evidence source wiring remains present.

## Validation

PASS:

```text
scripts/check-vac-runtime-agent-flow-fix-static.sh
scripts/check-vac-init-runtime-gate-callsite-integration.sh
scripts/check-vac-runtime-engine-wiring-static.sh
scripts/check-tui-source-artifact-hygiene.sh
scripts/check-legal-release-blockers.sh
scripts/check-vac-init-why-contract.sh
scripts/check-vac-init-evidence-chain-contract.sh (static PASS; cargo NotEvaluated)
```

NotEvaluated / TV-Pending:

```text
scripts/check-vac-init-command-gate-contract.sh (rustc not found)
scripts/check-vac-init-semantic-plan-contract.sh (rustc not found)
scripts/check-vac-init-runtime-gate-enforcement-contract.sh (rustc not found)
scripts/check-vac-init-policy-evaluator-contract.sh (rustc not found)
scripts/check-vac-init-cli-runtime-foundation-contract.sh (rustc not found)
scripts/check-vac-init-registry-validator-contract.sh (timeout after rustc-not-found prelude)
cargo check --manifest-path vac-rs/Cargo.toml -p vac-surface-tui
cargo check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui runtime_owner_session
live interactive vac smoke tests
live command policy/evidence tests
```

Reason: this sandbox does not provide `rustc`/`cargo`, and the registry validator gate timed out after reporting the missing Rust toolchain. No cargo or live TUI result is claimed.

## Residual risk

- Rust typechecking is still required for async boundary and DTO shape confirmation.
- The runtime event bridge currently forwards lifecycle notifications first; rich item/approval event DTO mapping remains a follow-up once cargo-backed feedback is available.
- Runtime Plan Mode now fail-closes on `.vac/registry/runtime/active_plan.yaml`; the UI path that creates/activates that file must be verified in a live smoke test.
- Command evidence source wiring is present through unified-exec/runtime gate bridge, but positive/negative command execution evidence tests still require the runtime toolchain.

## Status

- Owner-native critical startup/turn/shell stubs: **SV-Done**
- Plan Mode uses shared plan validation engine: **SV-Done**
- Doctor placeholder removed: **SV-Done**
- Static guard gate added: **SV-Done**
- Cargo/live runtime: **TV-Pending**
