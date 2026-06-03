# VAC root project instructions

This repository is the VAC product root.

## Product identity

- Product name: VAC
- Product command: `vac`
- Product frontend: `vac-rs/tui`
- Product core: `vac-rs/core`
- Donor source: `donor/vac/`

Do not add alternative product names. Do not implement a second TUI.

## Source boundaries

### Root source

The root source is the only product implementation path. Work here should move the product toward VAC behavior and production readiness.

Important root areas:

- `vac-rs/tui/` — active product TUI
- `vac-rs/core/` — active product core
- `vac-rs/cli/` — active product CLI for `vac`
- `vac-cli/` — packaging/Node launcher for `vac`
- `docs/` — product docs for VAC

### Donor source

`donor/vac/` contains source-only donor code from the previous VAC repository. It is not the product root.

Allowed donor usage:

- inspect domain logic
- port selected backend modules into the root product architecture
- compare previous plans, workflows, manifests, VIL/VWFD, approval, memory, signal, trajectory, and policy code

Forbidden donor usage:

- do not run donor TUI as the product frontend
- do not revive `donor/vac/crates/vac_shell_*` as a second active TUI
- do not revive `donor/vac/crates/vac_tui_runtime`
- do not copy duplicate render loops, approval bars, slash registries, command palettes, or task panels

## Implementation rule

A VAC feature is not implemented unless it is reachable from the root `vac` command and visible in the root TUI.

Backend-only ports from donor source are not accepted. Every port must have:

1. an active root TUI or CLI surface,
2. visible operator states,
3. tests or a validation command,
4. a cleanup decision for the donor copy.


## Rust validation speed rule

Use this rule for Rust work in this repository and in any other Rust project unless a local instruction says otherwise.

- Prefer **one build, many binary runs** for repeated CLI validations.
- Build the relevant binary once with `cargo build`, then run the produced binary directly from `target/debug/...` for repeated doctor/check/list commands.
- Avoid repeated `cargo run ...` for the same binary in one validation pass; it repeats Cargo planning and rebuild checks.
- Use narrow `cargo test` filters for targeted code changes. Use broader tests only when the touched area requires it.
- For docs/manifest-only edits, do not run heavy full-workspace tests unless the manifest gate itself requires them.
- Do not run `cargo clean` as a routine fix. It saves disk only temporarily and makes the next validation much slower.
- If a Rust build/test/doctor validation is likely to take more than a couple of minutes, report the estimate before polling again.

Fast validation pattern:

```bash
cargo test --manifest-path path/to/Cargo.toml -p target-crate relevant_filter -- --nocapture
cargo build --manifest-path path/to/Cargo.toml -p cli-crate
./target/debug/<binary> doctor registry .
./target/debug/<binary> doctor surfaces .
```

For this repository, the common VAC pattern is:

```bash
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics -- --nocapture
cargo build --manifest-path vac-rs/Cargo.toml -p vac-cli
./vac-rs/target/debug/vac doctor registry .
./vac-rs/target/debug/vac doctor policy .
./vac-rs/target/debug/vac doctor surfaces .
./vac-rs/target/debug/vac doctor workflow .
```

## Rust style

In `vac-rs/`, keep strict Rust quality expectations:

- keep `format!` arguments inline when possible,
- collapse collapsible `if` statements,
- use method references over redundant closures,
- avoid ambiguous positional boolean/`Option` parameters where practical,
- keep match statements exhaustive when possible,
- do not modify sandbox environment variable semantics without an explicit migration plan.

## Current cleanup priority

1. Keep root product surfaces VAC-only.
2. Keep only one product TUI.
3. Move donor backend value into the root product architecture deliberately.
4. Delete, quarantine, or ignore donor frontend stacks after useful backend logic is ported.
5. Validate the root `vac` CLI/TUI before donor implementation begins.

## Workflow control plane docs

- The workflow control plane docs live in [`docs/workflow-control-plane/INDEX.md`](docs/workflow-control-plane/INDEX.md).
- Before adding a backend-only product feature, add or update the matching capability manifest under `.vac/capabilities/`.
- Keep capability dashboard output, manifest metadata, and plan docs aligned.
- Keep legal notices and third-party attributions in [`docs/legal/NOTICES.md`](docs/legal/NOTICES.md); do not mix them into VAC product identity, command naming, or root architecture rules.
