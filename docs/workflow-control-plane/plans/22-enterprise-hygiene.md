# Plan 22 — Enterprise hygiene and documentation alignment


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented / documentation alignment baseline**.
> - Capability dashboard and docs agree on status.
> - **Closed gap:** release/build readiness is now status-synchronized through `.vac` manifests; full workspace build and real PTY evidence remain operator-gated release inputs instead of partial capability labels.
> ## 2026-05-28 domain/status sync

Code evidence:
- `vac-rs/core/src/control_plane/docs_maintenance.rs`
- `docs/PROJECT_STATE_CURRENT.md`
- `docs/legal/NOTICES.md`

Evidence docs:
- `docs/workflow-control-plane/plans/22-evidence/2026-05-28-sandbox-domain-status-sync.md`
- `docs/workflow-control-plane/plans/22-evidence/2026-05-28-sandbox-product-status-ready.md`
- `docs/workflow-control-plane/plans/22-evidence/2026-05-28-sandbox-registry-status-ready.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented / documentation alignment baseline**.

Target outcome: enterprise-facing hygiene, docs, naming, diagnostics, and operator guidance are aligned with the control-plane architecture.

Outputs: docs updates, hygiene checks, validation notes, and explicit follow-up findings.

Requires / Blocks: requires prior control-plane plan outputs to be stable enough to document.

Stop conditions: stop if docs would claim unsupported product behavior or hide known blocked/deferred gates.

Done criteria: docs reflect actual behavior, validation notes are current, and unresolved enterprise gaps are explicit follow-ups.


## Goal

Keep docs, manifests, TUI, and build gates aligned.

## Implementation

1. Update root README/AGENTS to point to control plane docs.
2. Add docs index for workflow control plane.
3. Add contribution rule: no backend-only feature without capability manifest.
4. Add maintenance checks for stale docs, missing manifests, or architecture invariant regressions.
5. Keep legal notices separate from product identity.

## Validation

- New contributor sees `.vac` pattern first.
- Product docs do not contradict manifests.
- Capability dashboard and docs agree on status.

## Current implementation slice

- README and AGENTS point to the workflow control-plane docs index.
- The docs index now names the maintenance checks used by contributors.
- `vac doctor docs` checks for stale docs links and missing manifest directories.
- `vac doctor architecture` checks the hardening contract, including legacy transport quarantine.

## Done

The repository is understandable as a VAC-native workflow system.

## Implementation audit — 2026-05-22

Reviewer roles: code reviewer, software engineer, and software architect.

### Findings

- **Closed gap:** root README and AGENTS now point contributors to the workflow control-plane docs before they add product behavior.
- **Closed gap:** legal notices are separated into `docs/legal/NOTICES.md` so product identity, command names, and architecture rules stay VAC-only.
- **Closed gap:** `vac doctor docs` treats legal notices as a required docs file and verifies the README legal-notices link.
- **Closed gap:** CLI surface metadata now declares release/build/donor doctor routes and maps architecture/ownership doctor routes to their owning capabilities.
- **Closed gap:** release/build readiness is now status-synchronized through `.vac` manifests; full workspace build and real PTY evidence remain operator-gated release inputs instead of partial capability labels.
- **Reviewer note:** local build validation can be expensive, so use one `cargo build` and reuse `./vac-rs/target/debug/vac` for all doctor commands.

### Validation notes

- Source checks expect `docs_checked=7` because legal notices are now part of required docs hygiene.
- `vac.release` is ready for release-gate semantics; missing full-workspace or real PTY evidence remains explicit non-pass operator evidence.
- The Plan 22 changes are docs/manifest/control-plane hygiene changes; they do not change VAC product identity or add donor product surfaces.


## 2026-05-28 domain/status sync

- `.vac/registry/domains.yaml` is aligned with current ready capability/workflow status.
- `vac.approvals` and `vac.build` are promoted to ready with code-backed readiness reports and evidence files.
- Evidence: `22-evidence/2026-05-28-sandbox-domain-status-sync.md`.


## 2026-05-28 sandbox registry ready sync

Root registry status, domains, capability statuses, workflow statuses, and surface route statuses are synchronized for the default product control-plane. Evidence: [22-evidence/2026-05-28-sandbox-registry-status-ready.md](22-evidence/2026-05-28-sandbox-registry-status-ready.md).
