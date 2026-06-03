# O5/O6 Cleanup Compile-Surface Static Report

Status: `SV-Done`, `TV-Pending`
Date: 2026-06-01

## Scope

This slice closes the compile-risk surface from the latest cleanup audit without changing the locked local-agent policy:

- keep login/auth/provider crates, including `device-key`, `aws-auth`, `vac-api`, `vac-client`, and `otel`;
- keep capability dashboard/operator console;
- keep external-agent import/migration;
- keep docs/scripts;
- keep local MCP `@mention` helper path;
- keep remote thread-store as a local fail-closed shim, not as a remote RPC implementation.

## Root cause

The previous local-tool cleanup removed or disabled large non-local surfaces, but the workspace manifest still contained stale path dependency entries for crates that no longer exist in the source artifact:

- `vac-agent-graph-store = { path = "agent-graph-store" }`
- `vac-core-api = { path = "core-api" }`
- `vac-v8-poc = { path = "v8-poc" }`

Those entries are not active workspace members, but they are still compile-surface debt because `cargo metadata` / inherited workspace dependency resolution may reject missing paths once a toolchain is available.

## Changes

- Removed stale missing `workspace.dependencies` path entries from `vac-rs/Cargo.toml`.
- Removed matching stale `cargo-shear` ignore entries for deleted crates.
- Added `scripts/check-vac-o5o6-cleanup-compile-surface-static.sh`.
- Tightened the stale TUI snapshot grep in `scripts/check-vac-tui-local-tool-cleanup-static.sh` so `side_context` snapshots do not false-match `ide_context`.
- Wired the new gate into:
  - `scripts/check-vac-o5-o6-completion-state.sh`
  - `scripts/check-vac-o5o6-all-audit-remainder.sh`
- Added evidence and trajectory registry records for the cleanup compile-surface slice.

## Static gate coverage

The new gate verifies:

- every `workspace.members` entry has a `Cargo.toml`;
- every `workspace.dependencies.*.path` exists;
- every package-local `path` dependency exists;
- every `{ workspace = true }` dependency resolves to a declared workspace dependency;
- locked auth/provider/local-agent crates remain present;
- disabled voice/realtime/audio, IDE/VSCode, and multi-agent stubs remain source-reachable;
- connector Path A remains local MCP mention metadata only;
- remote thread-store RPC remains absent and replaced by `remote_disabled`.

## Validation truth

Source/static validation is complete for this slice. `cargo check`, `cargo test`, `cargo clippy`, and live TUI smoke are still `TV-Pending` because this sandbox has no `cargo`/`rustc` toolchain.
