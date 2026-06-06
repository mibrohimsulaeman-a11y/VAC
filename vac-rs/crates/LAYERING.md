# VAC workspace layer target

This directory is the Cargo-ready target topology for VAC control-plane layering.
`vac-rs/Cargo.toml` accepts `crates/*/*` and `crates/*/*/*` members so crates can be moved one slice at a time without renaming packages first.

Layer order:
- `control-plane/`
- `surfaces/`
- `capabilities/`
- `runtime/`
- `providers/`
- `integrations/`
- `foundation/`

Physical relocation remains Cargo-sensitive and must be validated with targeted `cargo check` when a Rust toolchain is available.

## Migration status

All workspace crates are now recorded under their target layer in `layer-map.yaml`; there are no residual flat crates. `agent-identity` (providers) and `otel` (integrations) were already physically relocated and are now listed in the map. `core` (`vac-core`) is intentionally retained at the workspace root as the apex crate: it depends on `control-plane` and every lower layer and is consumed only by the `surfaces` layer (cli, tui), and several static contract gates pin its path at `vac-rs/core/`. It is tracked under `root_crates` rather than a layer.
