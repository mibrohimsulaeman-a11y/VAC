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
