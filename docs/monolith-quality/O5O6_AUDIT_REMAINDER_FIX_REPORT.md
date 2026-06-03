# O5/O6 Audit Remainder Fix Report

Date: 2026-06-01  
Scope: source/static remediation for the latest `physical-extraction` re-audit findings.

## Audit items addressed

| Audit item | Source/static status | Notes |
| --- | --- | --- |
| R-03 / panic via `.expect(...)` | SV-Done | Production `vac-rs/core/src` no longer contains `.expect(...)` outside tests. The last structured-output serialization panic was replaced with a logged fallback payload. |
| R-04 / ChatGPT cloud coupling | SV-Done partial | Legacy ChatGPT remote plugin/skill catalog calls are disabled by default and require explicit opt-in environment variables. Existing local provider/auth flow remains intact. Full enum/copy removal is TV-Pending because it is cargo-sensitive and touches login/auth/provider KEEP policy. |
| R-05 / giant `semantic_split.rs` files | SV-Done for dispatcher shrink | `semantic_split.rs` files generated from `split_manifest.yaml` are now small deterministic dispatchers. The large shards remain intact and hash-verified; deeper domain module extraction is TV-Pending. |
| R-06 / unbounded channels | SV-Done for production `core` + `tui` static surface | Production `core`/`tui` unbounded sender/receiver/channel terms are blocked by gate, with test-only fixtures allowed. App event, resume picker, composer widget, frame requester, file watcher, agent mailbox, and session event channels use bounded queues. |
| S-07 / flat topology | Plan encoded | `architecture-layer-map.yaml` now records the target `crates/<layer>/<name>` layout and blocker status. Physical all-crate relocation is TV-Pending until cargo validation is available. |
| S-12 / debt markers | SV-Done tracking | Technical-debt marker inventory is regenerated and hash-gated. Burn-down is tracked separately. |

## Gate contract

New static gate:

```bash
scripts/check-vac-o5o6-audit-remainder-static.sh
```

The gate verifies:

- no production `.expect(...)` in `vac-rs/core/src`, excluding tests;
- no production unbounded channel sender/receiver/channel terms in `vac-rs/core/src` or `vac-rs/tui/src`, excluding tests;
- every `semantic_split.rs` governed by `split_manifest.yaml` is a small include-only dispatcher;
- legacy ChatGPT remote plugin/skill cloud paths are fail-closed by default;
- the layered workspace target is recorded in `.vac/registry/architecture-layer-map.yaml`.

## Truth status

- SV-Done: source/static remediation and gates.
- TV-Pending: `cargo check`, `cargo test`, `cargo clippy`, live TUI smoke, full physical workspace relocation, full shard-to-domain-module extraction, and full legacy ChatGPT auth enum/copy removal.
