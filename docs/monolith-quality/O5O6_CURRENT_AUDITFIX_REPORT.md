# O5/O6 Current Auditfix Report

Date: 2026-06-01
Artifact baseline: `vac-o5o6-auditfix-remainder-final-source.zip`
Slice id: `o5o6.current-auditfix`

## Audit inputs

The latest read-only audit reported a healthy trajectory but still flagged:

- C-03: workspace topology still flat.
- C-04: `vac-core` remains central for capability logic.
- C-05: remaining giant files, specifically `runtime-protocol/src/protocol/v2.rs` and `tui/src/bottom_pane/chat_composer/split_002_footerflash.rs`.
- C-07: TUI unbounded channel risk.
- C-08: reachable panic risk via `unwrap`/`expect`/`panic!`.
- C-09: ChatGPT/cloud coupling still present.

## Changes

### C-05 targeted giant-file split

- Replaced `vac-rs/runtime-protocol/src/protocol/v2.rs` with a small dispatcher that includes `v2_parts/v2_part_000.rs` through `v2_part_009.rs`.
- Replaced `vac-rs/tui/src/bottom_pane/chat_composer/split_002_footerflash.rs` with a small dispatcher that includes `footer_flash_parts/footer_flash_000.rs`.
- Extracted the former inline `mod tests` body into `split_002_footerflash_tests.rs` and linked it with `#[cfg(test)]`.
- Split the large `impl ChatComposer` block into bounded shards under `footer_flash_parts/chat_composer_impl_parts/`.
- Updated semantic split manifests and staging hashes for modified shard sources.

### C-07 production channel discipline

- Added `scripts/check-vac-o5o6-current-auditfix-static.sh` to reject production `unbounded_channel`, `UnboundedSender`, and `UnboundedReceiver` in `vac-rs/core/src` and `vac-rs/tui/src` while excluding inline `#[cfg(test)]` fixtures.
- Existing targeted bounded hot-path checks remain in place.

### C-08 production panic surface

- Removed production `.unwrap(...)` / `.expect(...)` from touched TUI paths.
- Replaced lock handling and session-id conversion with fail-safe fallbacks and `tracing::warn!`.
- Remaining `panic!` sites are invariant/template assertions and remain TV-Pending for cargo-backed review.

### C-09 legacy ChatGPT cloud isolation

- Added explicit opt-in gates for legacy ChatGPT account backend/login and curated plugin backup paths:
  - `VAC_ENABLE_LEGACY_CHATGPT_ACCOUNT_BACKEND`
  - `VAC_ENABLE_LEGACY_CHATGPT_ACCOUNT_LOGIN`
  - `VAC_ENABLE_LEGACY_CHATGPT_CURATED_PLUGINS_BACKUP`
  - existing remote plugin/skill gates remain enforced.

### C-03/C-04 architecture target contract

- Expanded `.vac/registry/architecture-layer-map.yaml` with `capability_extraction_target` and `crates/foundation/runtime-protocol` target path.
- Full physical relocation to `vac-rs/crates/<layer>/<name>` remains TV-Pending because it changes Cargo path topology and cannot be validated without `cargo`/`rustc`.

## Validation

Static/source gates run in sandbox:

- `scripts/check-vac-o5o6-current-auditfix-static.sh`
- `scripts/check-vac-o5o6-audit-remainder-static.sh`
- `scripts/check-vac-o5o6-bounded-hotpath-static.sh`
- `scripts/check-vac-o5-2-semantic-split-hash.sh`
- `scripts/check-vac-o5-2-godfile-staging-all.sh`
- `scripts/check-vac-o5o6-cloud-coupling-isolation-static.sh`
- `scripts/check-vac-o5o6-architecture-extraction-static.sh`
- `scripts/check-vac-o5o6-cleanup-compile-surface-static.sh`
- `scripts/check-vac-o5o6-big-refactor-static.sh`
- `scripts/check-vac-technical-debt-markers-static.sh`
- `scripts/check-vac-standard-cargo-hygiene-static.sh`

## Truth status

- SV-Done: targeted source/static fixes for C-05, production C-07, production C-08 unwrap/expect, C-09 fail-closed gates, and C-03/C-04 architecture contract.
- TV-Pending: `cargo check`, `cargo test`, `cargo clippy`, full physical workspace relocation, full capability extraction from `core`, and live TUI smoke.
