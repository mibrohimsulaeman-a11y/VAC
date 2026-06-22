> **VAC v1.9 status note:** This report is historical platform evidence. Current installation and runtime instructions are in `GETTING-STARTED.md`; current source authority is `.vac/registry/compiled` JSON. URLs inside this report that point to example endpoints are fixture data, not current hosted documentation.

# VAC Windows testing report

This file supersedes an older pre-v1.9 report. No Windows binary or release download is claimed by this sandbox checkpoint.

## Current status

- Source/static validation: SV-Done in sandbox.
- Windows cargo build/test: TV-Pending.
- Windows TUI/PTY visual validation: TV-Pending.
- Installer/package validation: TV-Pending.

## Required local loop

```powershell
cargo metadata --manifest-path vac-rs/Cargo.toml
cargo fmt --manifest-path vac-rs/Cargo.toml --check
cargo check --manifest-path vac-rs/Cargo.toml
cargo clippy --manifest-path vac-rs/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path vac-rs/Cargo.toml
```

Record actual logs before changing this status.
