# O5/O6 Structured Blueprint Closeout

Status: `SV-Done` for source/static implementation slices, `TV-Pending` for cargo/test/clippy/benchmark/live TUI.

Baseline artifact: `vac-o5o6-tui-init-no-interactive-final-source.zip`.
Baseline SHA256: `f6436b30d8c017772e764d0054e6d481fe8f38fc984fa8824464f3536991c922`.

## Batch A — Safety Critical

Implemented source/static hardening for:

- `vac approve` binding verification, approval replay store consumption, explicit binding snapshot checks, optional Ed25519 signature policy, and fail-closed missing binding behavior.
- `vac plan execute` command-gated execution path using `CommandRunnerRegistry` and `evaluate_structured_command`, default dry-run, and explicit `--execute` for real execution.
- `vac init` strategy rendering from `.vac/.init/operator_choices.yaml`; TUI choices continue to work without requiring `vac init --interactive`.

## Batch B — TUI Performance

Implemented source/static completion for:

- Per-cell height cache data structures (`CellHeightKey`, `HeightPrefixIndex`, metrics).
- Windowed-render metrics path (`record_windowed_render`) so visible/skipped cells can be instrumented.
- Startup skeleton first-frame rendering path and `VAC_TUI_PROFILE_STARTUP` phase marker.
- Benchmark harness contract remains NotEvaluated without cargo.

## Batch C — Operator UX

Implemented:

- `/capabilities` now opens a `SelectionViewParams` overlay with searchable rows and drill-down metadata.
- `/runtime` continues through the interactive runtime operator console view instead of static workflow execution.

## Batch D — Provider Prune

Implemented:

- `vac-client` keeps generic transport by default and gates legacy ChatGPT host/cookie helpers behind `provider-chatgpt`.
- `custom_ca_probe` requires `provider-chatgpt`.
- Core config no longer defaults to `https://chatgpt.com/backend-api/`.

## Batch E — Layering

Implemented a cargo-ready layer contract:

- Root workspace accepts `crates/*/*` and `crates/*/*/*` member globs.
- Added `vac-rs/crates/layer-map.yaml` and layer directories.
- Full physical crate relocation remains `TV-Pending` because it is cargo-sensitive.

## Batch F — Debt Burn-Down

Implemented priority giant-file dispatcher splits for:

- `control-plane/src/control_plane/workflow_runner/split_001_build_check.rs`
- `tui/src/bottom_pane/textarea.rs`
- `core/src/session/mod.rs`
- `runtime-protocol/src/protocol/thread_history.rs`
- `tui/src/bottom_pane/request_user_input/mod.rs`

Technical-debt marker inventory was regenerated and verified.

## Static gates

The source/static closeout gate is `scripts/check-vac-o5o6-blueprint-all-static.sh`.

## Toolchain status

`cargo`, `rustc`, `clippy`, benchmark timing, and live TUI smoke are not evaluated in this sandbox.
