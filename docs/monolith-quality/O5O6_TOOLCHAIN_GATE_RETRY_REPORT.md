# O5/O6 Toolchain Gate Retry Report — 2026-05-30

## Scope

This session used the uploaded `vac-o5o6-sandbox-continuation-applied.zip` baseline and uploaded offline toolchain/vendor bundles to move O5/O6 from structural verification toward cargo verification.

## Bundle verification and restore

Observed before the sandbox reset:

- `sha256sum -c SHA256SUMS.txt`: PASS for `vac-merged-part-001.zip` through `vac-merged-part-006.zip`.
- Merged bundle restored file count: `205291`.
- Reconstructed `vac-rs/vendor/` crate count: `1151`.
- Rust toolchain observed:
  - `rustc 1.93.0 (254b59607 2026-01-19)`
  - `cargo 1.93.0 (083ac5135 2025-12-15)`
- Native precheck observed:
  - `cmake`: present
  - `clang`: present
  - `clang++`: present
  - `ninja`: present
  - `pkg-config`: present
  - `gn`: absent
  - `librusty_v8.a`: present in vendored V8 archive before reset

After the reset, SHA256 was re-run in the source-only environment and remained PASS. See `logs/sha256-current.log`.

## Cargo gates attempted

### Full workspace build

Command attempted:

```bash
cargo build --offline --workspace --locked
```

Result: `AbortedBySandboxReset` before a completed pass/fail result. No TV-Done claim is made.

### Targeted protocol build, first run

Command attempted:

```bash
cargo build --offline -p vac-runtime-protocol -p vac-app-server-protocol
```

Result: real `rustc` failure before patch:

```text
error[E0753]: expected outer doc comment
 --> protocol/src/protocol/legacy_include.rs:1:1
```

Root cause: O5.2 staged a large source file behind `include!(...)`; that legacy file still started with module-level inner doc comments (`//!`). Rust rejects those comments in this `include!` expansion position.

### Compiler-driven source fix

Applied fix:

- Converted staged `legacy_include.rs` inner doc comments (`//!`) to include-safe regular comments (`//`) where present.
- Re-pinned `current_legacy_sha256` in wrappers/state/gates.
- Re-ran O5.2 staging static gate: PASS.

Changed staged legacy files with converted comment lines:

- `vac-rs/tui/src/chatwidget/legacy_include.rs`: 31 lines
- `vac-rs/tui/src/bottom_pane/chat_composer/legacy_include.rs`: 123 lines
- `vac-rs/protocol/src/protocol/legacy_include.rs`: 4 lines
- `vac-rs/tui/src/legacy_app_server_session/legacy_include.rs`: 9 lines
- `vac-rs/tui/src/history_cell/legacy_include.rs`: 11 lines

No remaining `//!` / `/*!` lines exist in staged `legacy_include.rs` files.

### Targeted protocol build, retry after fix

Command attempted:

```bash
cargo build --offline -p vac-runtime-protocol -p vac-app-server-protocol
```

Result: `AbortedBySandboxReset`. Last observed phase before reset: compiling `vac-runtime-protocol`, after `vac-protocol` and related dependencies had progressed.

Because the reset destroyed target/log state, `cargo build`, `cargo clippy`, and `cargo test` remain TV-Pending.

## Static suite after source rehydration

Final source-only static suite:

```text
suite_total=53 passed=21 failed=3 skipped_tv_pending=29
```

The 3 remaining failures are toolchain-path failures in source-only mode after reset:

- `check-vac-init-batch2-5-contract.sh` → nested script cannot find `rustc`.
- `check-vac-init-batch6-12-contract.sh` → nested script cannot find `rustc`.
- `check-vac-init-deterministic-hardening-p0-p4.sh` → nested script cannot find `rustc`.

O5/O6-specific gates passed after the compiler-driven fix:

- `check-vac-o5-1c-app-server-protocol-consumer-migration.sh`: PASS
- `check-vac-o5-2-godfile-staging-all.sh`: PASS
- `check-vac-o5-2-godfile-staging.sh`: PASS
- `check-vac-o5-3-utils-consolidation-report.sh`: PASS
- `check-vac-o5-4-workspace-consolidation-report.sh`: PASS
- `check-vac-o5-5-donor-delete-gate.sh`: PASS
- `check-vac-o5-o6-completion-state.sh`: PASS
- `check-vac-o5-o6-monolith-quality-slice.sh`: PASS
- `check-vac-o6-2-safety-coverage.sh`: PASS
- `check-vac-o6-3-docs-pruning-classification.sh`: PASS
- `check-vac-o6-4-e2e-expansion-scaffold.sh`: PASS
- `check-vac-o6-quality-triage.sh`: PASS

## Truth table

| Slice | Status |
| --- | --- |
| O5.1c | SV-Done, TV-Pending. Static migration gate passes; cargo consumer gate did not complete. |
| O5.2 | SV-Done for staged include wrappers and compiler-driven doc-comment fix; TV-Pending due reset. |
| O6.1 | SV-Done for runtime panic baseline; TV-Pending due reset. |
| O6.2 SAFETY | Superseded by audit remediation: previous `558/558` was retired; current source-runtime static coverage is `491/491` with test/cfg(test) exclusions and stale generic SAFETY rejection. TV-Pending for compiler/geiger confirmation. |
| O5.3/O5.4 | Report/static gates pass; actual merge/consolidation not performed because TV gate blocked. |
| O5.5 | Donor delete remains blocked by design until schema-bin relocation + architecture invariant update + completed cargo verification. |
| O6.3 | Docs pruning classification gate passes; destructive archive/delete not performed in this blocked run. |
| O6.4 | Scaffold remains NotEvaluated. |

