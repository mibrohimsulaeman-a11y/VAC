# O5/O6 Runtime Engine Wiring + TUI Parity Fix Report

Status: **SV-Done / TV-Pending**
Date: 2026-05-31

## Scope

This slice fixes the two new audit streams:

1. VAC-Init engine/runtime wiring: runtime gates and CLI commands must call the
   existing `vac_init_*` control-plane engines instead of relying on file/directory
   existence stubs.
2. TUI visual parity: operator screens must render panel geometry and use runtime
   data sources on production paths instead of flat text descriptions and sample
   scheduler/idle state.

## Runtime engine wiring changes

- Added `vac_init_runtime_bridge.rs` as the boundary between live runtime call
  sites and pure VAC-Init engines.
- `apply_patch.rs` now builds `VacInitRuntimePatchFileChange` records and calls
  `evaluate_vac_init_runtime_patch_contract`.
- Patch scope validation now flows through:
  - active plan parsing,
  - capability status lookup,
  - `validate_semantic_plan`,
  - `validate_patch_attempt_with_semantic_source`,
  - pre-plan and pre-patch runtime gates.
- `unified_exec.rs` now calls `evaluate_vac_init_runtime_command_contract` before
  execution and `write_vac_init_runtime_command_evidence` after completion.
- Command evaluation now flows through `CommandRunnerRegistry::with_defaults`,
  `evaluate_structured_command`, approval replay validation, and evidence writer.
- `vac init` now advances lifecycle through `advance_init_state`, validates the
  state record, writes `state.yaml` via engine render, and emits lifecycle evidence.
- `vac plan validate`, `vac why`, and `vac registry migrate` now delegate to the
  matching engine bridge instead of only using local subset validators.

## TUI visual parity changes

- Capability dashboard no longer emits the text-only `layout left: ...` and
  `metrics [...]` lines.
- Dashboard metrics, navigation, registry, and diagnostics now render through
  boxed panel geometry (`box_lines`) and column composition (`render_columns`).
- Runtime jobs production path now uses
  `AutopilotSchedulerState::from_workspace(config.cwd)` instead of
  `monitor_only_sample()`.
- Idle production path now uses `IdleViewState::live` instead of the hardcoded
  sample task/update list.
- Startup copy no longer hardcodes `gpt-4o`, `327 files indexed`, or old sample
  recent task data.
- `operator_ui_styles.rs` now documents that it styles precomputed panel geometry
  rather than intentionally preserving a flat-text-only model.

## Static validation

- `scripts/check-vac-runtime-engine-wiring-static.sh`: PASS
- `scripts/check-vac-tui-visual-parity-static.sh`: PASS
- `scripts/check-vac-o5o6-all-audit-remainder.sh`: PASS
- `scripts/check-vac-o6-1-depanic-surface.sh`: PASS
- `scripts/check-vac-o5-o6-completion-state.sh`: PASS
- `scripts/check-vac-init-registry-strictness-contract.sh`: PASS static, rustc unit NotEvaluated
- `scripts/check-vac-init-registry-validator-contract.sh`: PASS static, rustc unit NotEvaluated
- `scripts/check-vac-o6-2-safety-coverage.sh`: PASS
- `scripts/check-legal-release-blockers.sh`: PASS
- `scripts/check-tui-source-artifact-hygiene.sh`: PASS
- `scripts/check-vac-o5o6-upload-sha256.sh`: PASS with current `/mnt/data` upload bundle env

## TV status

No `TV-Done` is claimed. The following remain pending until a stable toolchain and
vendor environment can run without sandbox reset:

- `cargo build --offline --workspace --locked`
- `cargo clippy --offline --workspace -- -D warnings`
- `cargo test --offline --workspace`
- live TUI screenshot/pixel gate
- runtime negative tests for out-of-plan patch/command rejection
