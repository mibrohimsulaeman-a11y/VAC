# VAC CLI

VAC is the product in this repository.

## Product identity

- Product command: `vac`
- Product TUI: `vac-rs/tui`
- Product core: `vac-rs/core`
- Product CLI: `vac-rs/cli`
- Package launcher: `vac-cli/bin/vac.js`
- Donor source: `donor/vac/`

Do not implement a second TUI. Do not revive the old donor TUI stacks as product code.


## Rust development loop

When validating Rust work, prefer a fast, cache-friendly loop:

- For repeated CLI doctor/check commands, build the binary once and run it directly from `./vac-rs/target/debug/vac` instead of using repeated `cargo run` invocations.
- For docs/manifest-only work, run the narrowest relevant `cargo test` filter first, then reuse the built binary for doctor commands.
- Avoid `cargo clean` unless cache corruption or disk recovery requires it; it forces a full rebuild and slows follow-up development.
- If a Rust validation step is expected to take more than a couple of minutes, report that expectation before polling for progress.

Example:

```bash
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics -- --nocapture
cargo build --manifest-path vac-rs/Cargo.toml -p vac-cli
./vac-rs/target/debug/vac doctor registry .
./vac-rs/target/debug/vac doctor policy .
./vac-rs/target/debug/vac doctor surfaces .
./vac-rs/target/debug/vac doctor workflow .
```

This reuses the same Cargo artifacts that `cargo run` would create, but avoids repeated Cargo planning and rebuild checks for each doctor command.

## Workflow control plane

The declarative product control plane lives under [`docs/workflow-control-plane/`](docs/workflow-control-plane/INDEX.md) and `.vac/`.

Rules of thumb:

- add a capability manifest before adding a backend-only product feature,
- surface every capability in the root TUI or CLI,
- keep the plan docs aligned with the manifests and dashboard output,
- treat donor code as source material, not product code.

Contribution gate: do not land backend-only product work. Add or update the capability manifest, root CLI/TUI surface, and validation command in the same change that introduces product behavior.

Self-check before opening a change: run `./vac-rs/target/debug/vac doctor docs .` to verify the docs index links, README/AGENTS pointers, and `.vac/` manifest directories are aligned.

Legal notices and third-party attributions are tracked separately from VAC product identity in [`docs/legal/NOTICES.md`](docs/legal/NOTICES.md).

## Current migration rule

All new work must target the root VAC architecture and the root `vac` command. Donor code may be ported only when it is wired into the root product flow.

```text
root product: /home/emp/Documents/VAC/vastar-agentic-cli
source donor: donor/vac
forbidden: donor/vac TUI as product frontend
```

## What to port from donor/vac

Prioritize backend/domain code that makes VAC unique:

- workflow and manifest/profile domain logic
- VIL/VWFD parser, validator, and workflow tooling
- approval/policy enhancements that improve the VAC approval flow
- memory/RAG/context modules if they integrate into the VAC TUI
- signal/trajectory/evidence modules if they become visible in the VAC TUI

Do not port old frontend stacks as product code:

- `donor/vac/crates/vac_shell_*`
- `donor/vac/crates/vac_tui_runtime`
- old VAC command aliases and duplicate TUI registries
- duplicate approval bars, task panels, render loops, or slash registries

## Definition of done for donor migration

A donor feature is not migrated until it is visible and usable from the root `vac` command.

Required for every feature:

1. It is wired into the root VAC CLI/TUI path.
2. It has a clear operator flow.
3. It has visible empty/loading/success/failure states.
4. It does not create duplicate TUI infrastructure.
5. The old donor copy is deleted, quarantined, or explicitly marked as donor-only after porting.

Backend-only ports are not accepted.
