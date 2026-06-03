# O5/O6 Compile-Fix Targeted TV Report — 2026-05-31

## Scope

This checkpoint preserves compiler-driven fixes made after the audit-closeout and runtime-engine/TUI-parity slices. The goal is to convert the most relevant touched crates from source/static verification into targeted cargo verification without claiming full workspace build/clippy/test completion.

## Fixes

- `tui/src/session_protocol_compat.rs`: added missing owner-native DTO re-exports (`ThreadGoal`, `ThreadSortKey`, `TurnPlanStepStatus`, `AdditionalPermissionProfile`, `AutoReviewDecisionSource`, `Hook*`, `ModelVerification`, `NetworkAccess`, `NetworkApproval*`).
- `cli/src/doctor_cli.rs`: imported `serde_yaml::Value` and `std::fs` for YAML/migration doctor helpers.
- O6.1 panic-surface registry was repinned to the current tokenizer output after the compile-shaped source split changed counted runtime paths.

## Targeted cargo verification

All cargo commands below were run offline using uploaded Rust 1.93.0 toolchain and vendored crates, with `CARGO_BUILD_RUSTC_WRAPPER=''` to bypass missing `sccache` and `RUSTY_V8_ARCHIVE` pointed at the vendored `librusty_v8.a`.

| Gate | Result |
| --- | --- |
| `cargo metadata --offline` | PASS |
| `cargo check --offline -p vac-protocol` | PASS |
| `cargo check --offline -p vac-core` | PASS |
| `cargo check --offline -p vac-surface-tui` | PASS, warnings only |
| `cargo check --offline -p vac-surface-cli` | PASS, warnings only |

## Static gates rerun

- `scripts/check-vac-o5-2-semantic-split-hash.sh`: PASS
- `scripts/check-vac-o6-1-depanic-surface.sh`: PASS (`total_runtime_panic_surface=251`, `cfg_test_lines_removed=134182`)
- `scripts/check-vac-o5-5-donor-delete-gate.sh`: PASS
- `scripts/check-vac-o5-o6-completion-state.sh`: PASS
- `scripts/check-vac-o6-2-safety-coverage.sh`: PASS
- `scripts/check-vac-o5-o6-monolith-quality-slice.sh`: PASS

## Remaining TV-Pending

Full workspace `cargo build`, full workspace `cargo clippy`, and full workspace `cargo test` are still not claimed. Live TUI screenshot/pixel parity is still TV-Pending.
