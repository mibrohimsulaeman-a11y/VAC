# O5.1 Protocol Dedup Slice

Status: implemented as a compatibility shim; build/test remain `NotEvaluated` until Cargo/vendor gates are available.

## Scope

This slice deduplicates the byte-identical implementation duplicated between:

```text
vac-rs/app-server-protocol/src/
vac-rs/runtime-protocol/src/
```

The canonical implementation is now `vac-runtime-protocol`; `vac-app-server-protocol` remains as a thin public compatibility re-export so downstream crates do not need a broad import rewrite in this slice.

## Applied

- `vac-rs/app-server-protocol/src/lib.rs` now re-exports `vac_runtime_protocol::*`.
- Duplicate implementation files under `vac-rs/app-server-protocol/src/` were removed.
- `vac-rs/app-server-protocol/Cargo.toml` now depends on `vac-runtime-protocol`.

## Not claimed

- Full workspace build is not claimed.
- Workspace E2E is not claimed.
- Schema fixture tests are not claimed.
