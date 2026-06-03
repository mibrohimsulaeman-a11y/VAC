# Plan 33 — Sandbox default Cargo cutover evidence

Status: complete for the default local product Cargo path; wider app-server crate deletion remains explicitly deferred to non-default compatibility/history consumers.

## What changed

- `vac-rs/tui/Cargo.toml` now has `default = []`.
- `vac-app-server-client` is the only optional app-server compatibility dependency under `legacy-app-server-compat`; `vac-app-server` and `vac-app-server-protocol` are no longer direct `vac-tui` dependencies.
- Runtime-owner gates distinguish default dependencies from optional non-default compatibility dependencies.
- Optional compatibility edges are allowed only when the manifest declares `default_product_path = false`; default app-server dependencies still report as `app_server_dependency_present`.

## Delete/defer proof

The default local TUI path no longer requires app-server Cargo dependencies directly. The app-server crates are not deleted from the workspace because protocol/test/history consumers still exist outside this TUI default-path slice.

## Guardrail

A future regression that re-adds a non-optional app-server dependency to `vac-rs/tui/Cargo.toml` must fail the Plan 32/33 static check.
