# Plan 33 — Default product Cargo path retirement evidence

Status: complete for the default product path. Workspace crate deletion remains deferred.

## Default-path result

- `vac-rs/tui/Cargo.toml` declares `default = []`.
- `vac-app-server-client` is the only optional app-server compatibility dependency under `legacy-app-server-compat`; `vac-app-server` is no longer a direct `vac-tui` dependency.
- `vac-app-server-protocol` is no longer a direct `vac-tui` dependency.
- `vac-runtime-protocol` is the default DTO owner crate used by the TUI facade.

## Defer classification

The app-server crates remain in the workspace as quarantine/legacy compatibility material, not as the default local product path. This is intentional delete/defer proof: do not delete workspace crates until non-TUI consumers and historical test paths are removed or re-owned.

## Guardrail

`vac doctor runtime-owner-gates` treats non-optional app-server dependencies in watched runtime-owner manifests as `app_server_dependency_present`. Optional compatibility is only allowed when `default_product_path = false` is declared in package metadata.
