# Migration: generated control plane -> lock + cache model

This rebuild collapses a 391-file / ~202k-line authored tree into a thin authored layer
plus regenerated `derived/` state. Same concept (bounded agent, fail-closed, evidence chain,
`vac why`), proportional implementation.

## What changed

| Before (generated)                                                   | After (lock + cache)                                          |
|----------------------------------------------------------------------|---------------------------------------------------------------|
| `capabilities/*.yaml` with hand-enumerated `targets:` module lists   | `ownership:` globs + in-source `#![vac::owner=...]`; resolved map is DERIVED |
| `~25 vac.init.* capability manifests`                                | one `capabilities/vac.init.yaml` with `scopes:`              |
| `.init/source_inventory*`, `risk_findings/by-*`                      | `derived/inventory.yaml`, `derived/risk.yaml` (git-ignored)  |
| `registry/ownership/report.yaml`                                     | `derived/ownership.yaml` (git-ignored)                       |
| `surfaces/*.yaml` with hand-listed `capabilities:`                   | routes only; `derived/surface-coverage.yaml`                 |
| `plan.o5o6.*`, `evidence/*`, `trajectory/*` (per-cycle file spam)    | single `ledger/findings.yaml` + `ledger/waivers.yaml`        |
| `acceptance: pass_or_recorded_pending`                               | `acceptance.mode: binary`, `unknown_is: fail`                |
| agent-authored `evidence.*-not-evaluated.yaml`                       | runner-emitted `evidence/` only; absence == fail             |
| control plane exempt from anti-bloat                                 | `[budget.authored]` in `vac.toml`, gated by `vac doctor budget .` |

## Schema version

Authored manifests pin `schema_version: 1`. Loaders MUST reject legacy-shaped capability/workflow
manifests (enumerated `targets`, `pass_or_recorded_pending`) and point to this file.

## How to regenerate derived state

```bash
vac scan .     # writes derived/ownership.yaml, derived/inventory.yaml, derived/risk.yaml, derived/surface-coverage.yaml
vac doctor .   # all gates, fail-closed
```
