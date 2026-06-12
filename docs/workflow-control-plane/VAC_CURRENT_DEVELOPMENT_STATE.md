# VAC v1.9 current development state

This document summarizes the current development state for VAC v1.9 as tracked in the source tree. It is intentionally conservative: static verification, runtime smoke evidence, and cargo gates must be reported separately and must not be inferred from documentation alone.

## Repository baseline

VAC is a local-control-plane-first agentic CLI/TUI. The runtime authority path is compiled local state, not remote service defaults. Source-controlled configuration and manifests are compiled into local control-plane outputs, and runtime components authorize from those compiled outputs and local DB state.

Key local state classes:

- Source authority: Rust crates, scripts, docs, manifests, and tracked fixtures.
- Compiled authority: generated compiled JSON under `.vac/cache/compiled`.
- Runtime state: `.vac/db/runtime.db`, session-local state, evidence, indexes, and exports.

Runtime state is not release source. It may be included in a state-export ZIP for handoff/debugging, but that ZIP is an evidence artifact, not canonical source authority.

## Current verification language

Use these labels consistently:

- `SV-Done`: statically verified without requiring `rustc`, cargo tests, or runtime execution.
- `TV-Done`: tool/runtime verified in the current worktree with captured evidence.
- `TV-Pending / NotEvaluated`: not run in the current worktree, even if it passed in a previous session.

Do not claim GitHub CI green unless the remote workflow/status was explicitly checked.

## Current expected gates

The usual local verification set includes:

```bash
cargo metadata --manifest-path vac-rs/Cargo.toml --locked --format-version 1
cargo fmt --manifest-path vac-rs/Cargo.toml --all -- --check
cargo check --manifest-path vac-rs/Cargo.toml --workspace --all-targets --locked
cargo clippy --manifest-path vac-rs/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path vac-rs/Cargo.toml --workspace --all-targets --locked
bash scripts/vac-v19-final-sv-gate.sh
python3 scripts/vac-runtime-realpath-e2e.py .
python3 scripts/check-checkpoint-integrity.py .
python3 scripts/check-docs-current-state.py .
python3 scripts/check-brand-allowlist.py .
```

TUI lifecycle changes additionally require the targeted TUI static gates and PTY smoke evidence. If Shift+Tab cannot be observed by the PTY driver, report that specifically and retain `/plan` visual evidence plus static Shift+Tab source-path evidence.

## Handoff rule

A state-export ZIP is useful for moving or inspecting a local repo state, but it can become large because it may contain logs, evidence, and generated artifacts. For source transfer, prefer a source ZIP or Git commit that excludes runtime-local state.
