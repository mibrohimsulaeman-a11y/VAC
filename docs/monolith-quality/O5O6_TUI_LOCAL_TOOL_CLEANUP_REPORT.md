# O5/O6 TUI Local Coding Tool Cleanup Report

Status: **SV-Done / TV-Pending**

## Scope

This slice narrows the artifact toward a local **TUI + CLI coding tool** while preserving the locked surfaces:

- login/auth/provider crates;
- capability dashboard and operator console;
- external-agent import/migration;
- docs and scripts;
- local MCP `@mention` connector helpers;
- local rollout/thread-store path.

## Removed or disabled

- Realtime voice/audio implementation and `realtime-webrtc` workspace crate.
- IDE/VSCode IPC implementation (`ide_context/ipc`, prompt fetch, Windows pipe); `/ide` now degrades with a local-tool hint.
- Interactive multi-agent collaboration behavior; the module is now a compatibility stub, while external-agent import crates remain.
- ChatGPT Apps cloud directory fetch/orchestration; connector helpers now derive labels from local MCP tool metadata and use `vac://mcp-connectors/...` local URLs.
- Dangling pnpm workspace entries for missing packages.
- Remote thread-store RPC source, replaced with a fail-closed local shim.

## Safety/performance follow-up

- `DESTRUCTIVE` approval label now uses red danger styling.
- Operator draw/read-only contract is locked by a static gate: draw/tick paths must not execute the autopilot scheduler.
- `wrapping.rs` now preallocates range vectors for repeated wrap computations.

## Validation

Source/static gates passed:

- `scripts/check-vac-tui-local-tool-cleanup-static.sh`
- `scripts/check-vac-tui-operator-draw-readonly-contract.sh`
- `scripts/check-vac-control-plane-audit-findings-static.sh`
- `scripts/check-vac-o6-1-depanic-surface.sh`
- `scripts/check-vac-o6-2-safety-coverage.sh`
- `scripts/check-vac-o5o6-all-audit-remainder.sh`

## TV-Pending

Cargo and live terminal verification were not run in this sandbox because `cargo`/`rustc` are unavailable.

Required TV:

```bash
cargo check --manifest-path vac-rs/Cargo.toml -p vac-surface-tui -p vac-core -p vac-chatgpt -p vac-connectors -p vac-thread-store
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui approval_overlay operator_console markdown_render wrapping
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core connectors app_tool_policy
```

Live smoke still needed for `/runtime`, `/capabilities`, `/workflow`, approval popup, `/ide` degraded hint, and disabled realtime command.
