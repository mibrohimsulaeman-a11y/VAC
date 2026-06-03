# Plan 08 — Initial root capability and workflow manifests


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **implemented seed set; active contract is canonical root manifest coverage**.
> Stop conditions: stop if seed manifests claim ready status without validation or conflict with current root feature catalog.
> ## Completion status
> - [x] `.vac/registry/status.yaml` records the guarded root seed coverage contract in schema-compatible notes.

Code evidence:
- `.vac/capabilities`
- `.vac/workflows`
- `.vac/registry/status.yaml`
- `vac-rs/core/src/control_plane/root_feature_catalog.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/08-evidence/2026-05-28-sandbox-root-seed-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented seed set; active contract is canonical root manifest coverage**.

Target outcome: initial root capabilities/workflows/policies exist as valid seeds for the control-plane registry.

Outputs: root `.vac` manifests, validation commands, guard behavior evidence, and commit/history notes.

Requires / Blocks: requires schemas and registry loader; blocks feature conversion and dashboard/browser usefulness.

Stop conditions: stop if seed manifests claim ready status without validation or conflict with current root feature catalog.

Done criteria: all seed manifests validate and map to real product capabilities or explicit planned placeholders.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Model existing root product features before adding donor-backed capabilities, then guard that seed snapshot so root coverage cannot silently regress.

## Completion status

Completed in Plan 08 hardening pass.

### Delivered

- [x] Root capability manifests exist under `.vac/capabilities/` for canonical product areas.
- [x] Root capability coverage now includes `vac.architecture`, `vac.ownership`, `vac.identity.check`, and `vac.tui` in addition to chat, approvals, tools, sandbox, sessions, workflow, build, identity, and release.
- [x] Baseline policy manifests exist under `.vac/policies/`.
- [x] TUI, slash, palette, and CLI surface manifests exist under `.vac/surfaces/`.
- [x] Maintenance workflow manifests exist under `.vac/workflows/` for build, identity, release, ownership, TUI uniqueness, and donor migration gates.
- [x] `vac doctor registry .` now renders root seed coverage and fails when canonical root seed coverage is missing or unsafe.
- [x] `.vac/registry/status.yaml` records the guarded root seed coverage contract in schema-compatible notes.

## Canonical root seed coverage

The canonical root seed catalog lives in `vac-rs/core/src/control_plane/root_feature_catalog.rs`.

`vac doctor registry .` checks the following canonical capability ids:

```text
vac.chat
vac.approvals
vac.tools
vac.sandbox
vac.sessions
vac.workflow
vac.build
vac.identity
vac.identity.check
vac.release
vac.tui
vac.ownership
vac.architecture
```

It also requires:

```text
.vac/policies/approval.yaml        # vac.policy.approval
.vac/policies/default-local.yaml   # vac.default-local
.vac/policies/filesystem.yaml      # vac.policy.filesystem
.vac/policies/network.yaml         # vac.policy.network
.vac/policies/sandbox.yaml         # vac.policy.sandbox
.vac/policies/tools.yaml           # vac.policy.tools
.vac/surfaces/tui.yaml
.vac/surfaces/slash.yaml
.vac/surfaces/palette.yaml
.vac/surfaces/cli.yaml
.vac/workflows/maintenance.build-check.yaml
.vac/workflows/maintenance.identity-check.yaml
.vac/workflows/maintenance.release-gate.yaml
```

## Guard behavior

The root seed coverage report blocks when:

- a canonical root capability is missing,
- `.vac/policies` is empty,
- a canonical baseline policy manifest is missing,
- a required surface manifest is missing,
- a required root maintenance workflow is missing,
- a canonical root capability lacks ownership metadata,
- a canonical root capability lacks policy metadata,
- a canonical root capability lacks validation evidence,
- a canonical root capability declares `donor_source`, or
- a `ready` root capability is not represented in a surface manifest, or
- a `ready` workflow has unsupported runner steps.

This keeps root seed status honest: `ready` means surfaced and validated; incomplete areas stay `partial` or `planned`.

## Current implementation slice

- Initial capability manifests exist under `.vac/capabilities/`.
- The root capability set includes `vac.identity.check` for maintenance identity scanning.
- The root capability set includes `vac.architecture` for operating-contract hardening.
- The root capability set includes `vac.ownership` for ownership coverage.
- The root capability set includes `vac.tui` for root TUI uniqueness scanning.
- Initial maintenance workflow manifests exist under `.vac/workflows/`.
- The registry loader reads the seeded root snapshot from the repo root.
- Registry diagnostics now include root seed coverage as a hard gate.
- The canonical root seed catalog is defined in `root_feature_catalog.rs` so it can be updated and reviewed deliberately.
- Registry diagnostics require canonical baseline policy coverage, not just a non-empty policy directory.
- Registry diagnostics block `ready` workflows whose steps are not fully supported by the typed safe runner.
- Release and donor migration workflows include `capability.root_seed.coverage` as an explicit prerequisite step.
- The `/capabilities` TUI dashboard renders root seed coverage through the registry diagnostics section.

## Validation commands

```bash
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics -- --nocapture
cargo build --manifest-path vac-rs/Cargo.toml -p vac-surface-cli
./vac-rs/target/debug/vac doctor registry .
./vac-rs/target/debug/vac doctor policy .
./vac-rs/target/debug/vac doctor surfaces .
./vac-rs/target/debug/vac doctor workflow .
```

## Commit trail

- `docs(plan08): document root seed hardening` — historical Plan 08 seed documentation.
- `feat(plan08): guard root seed coverage` — registry diagnostic hardening for canonical root seed coverage.

## Done

Existing root product areas are declared and guarded before donor-backed work continues. Operator UX is explicit: the registry doctor now says whether root seed coverage is pass/blocked, with precise missing feature diagnostics instead of silent manifest drift.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | Planner | Drift | ✅ resolved — root seed workflow coverage is synchronized with the current canonical required set | Keep catalog and docs aligned |
| 2 | P1 | Planner | Drift | ✅ resolved — `.vac` manifests are the current source of truth and Plan 19 is synchronized | Keep status sync evidence current |
| 3 | P1 | Architect | Attribution | ✅ resolved — surface routes are attributed to `vac.architecture` and `vac.ownership` | Preserve surface attribution checks |
| 4 | P2 | Planner | Manifest | ✅ resolved — root reasons/status are synchronized to ready/default path evidence | Keep reasons only for non-ready states |
| 5 | P2 | SE | Manifest | ✅ resolved — donor-source semantics are validated and owned by Plan 20 donor gate evidence | Keep donor manifests explicit |
| 6 | P2 | Planner | Docs | ✅ resolved — Plan 08 now describes current root seed coverage and ready status | Keep docs synced with catalog |
| 7 | P3 | Planner | Docs | ✅ resolved — counters use the current required workflow set instead of old prose | Keep numbers generated/reviewed with root catalog |
