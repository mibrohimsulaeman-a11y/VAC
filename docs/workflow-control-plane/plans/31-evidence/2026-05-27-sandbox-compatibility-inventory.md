# Plan 31 Sandbox Compatibility Inventory — 2026-05-27

Status: `31A-INVENTORY-RECORDED`

This evidence records the current compatibility footprint before attempting DTO replacement or deletion. It does not claim Plan 31 completion.

## Findings

- Active TUI/app-server source or manifest matches: 44.
- Watched Cargo app-server dependency matches in `vac-rs/{cli,core,exec,local-runtime-owner,tui}/Cargo.toml`: 4.
- `vac-rs/tui/Cargo.toml` still declares app-server crates, so Plan 33 must remain blocked.
- The runtime-owner gate now reports these as `app_server_dependency_present` warning-level findings rather than letting them appear as green owner proof.

## Classification

| Area | Classification | Required follow-up |
|---|---|---|
| `vac-rs/tui/Cargo.toml` app-server dependencies | active compatibility dependency | Plan 31C/33B replacement/removal or explicit non-default defer. |
| `vac-rs/tui/src/app_server_session.rs` references | active compatibility bridge | Lane-owned conversion before default path can be claimed no-app-server. |
| docs/evidence references | historical/evidence | Keep allowed, but they must not satisfy runtime gates as green source proof. |

## Stop condition retained

No dependency removal is attempted in this batch because Plan 30G provider gaps and app-server-session fallback ownership are still open.
