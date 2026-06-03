# O5.3 Utils Consolidation Report

Status: Planned / NotEvaluated

This report registers candidates for a future `vac-utils` consolidation. No crate merge is performed in this slice because the build gate is unavailable.

## Candidate utility crates

```json
[
  {
    "crate_dir": "async-utils",
    "rs_bytes": 2044,
    "cargo_toml": "vac-rs/async-utils/Cargo.toml"
  },
  {
    "crate_dir": "git-utils",
    "rs_bytes": 95411,
    "cargo_toml": "vac-rs/git-utils/Cargo.toml"
  }
]
```

## Small crate candidates by Rust LOC/bytes proxy

```json
[
  {
    "crate_dir": "collaboration-mode-templates",
    "rs_bytes": 280,
    "cargo_toml": "vac-rs/collaboration-mode-templates/Cargo.toml"
  }
]
```

## Required next gate

Before any utility crate merge:

```text
cargo build --workspace --offline
cargo test --workspace --offline
```

Until those run, O5.3 remains `Planned_NotEvaluated`.
