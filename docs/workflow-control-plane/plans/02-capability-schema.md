# Plan 02 — Capability manifest schema


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with audit history; active contract is schema correctness**.
> Outputs: schema/validation code, negative tests, ready-status evidence rules, and docs for required fields.
> status: planned
> 2. Validate ids, status enum, owner path, surface shape, states, and validation commands.

Code evidence:
- `vac-rs/core/src/control_plane/capability_manifest.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/02-evidence/2026-05-28-sandbox-capability-schema-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with audit history; active contract is schema correctness**.

Target outcome: capability manifests have a typed, validated schema that rejects malformed ownership, surfaces, states, and ready-without-evidence cases.

Outputs: schema/validation code, negative tests, ready-status evidence rules, and docs for required fields.

Requires / Blocks: requires Plan 01 skeleton; blocks capability manifests in Plans 08, 19, 20, 23, and 32.

Stop conditions: stop if schema changes break existing valid manifests without a migration plan, or if validation becomes a no-op.

Done criteria: valid manifests load, invalid manifests fail with structured errors, and ready capabilities require validation evidence.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Define the typed manifest shape for every product capability.

## Required fields

```yaml
schema_version: 1
kind: capability
id: vac.example
title: Example capability
status: planned
owner:
  crate: vac-rs/core
  module: example
surfaces:
  tui: /example
  slash: /example
  palette: true
  cli: false
states:
  empty: string
  loading: string
  success: string
  failure: string
policy:
  mutates_files: false
validation:
  commands: []
```

## Implementation

1. Add JSON schema or Rust validation struct for capability manifests.
2. Validate ids, status enum, owner path, surface shape, states, and validation commands.
3. Preserve strict error messages with file path and field path.
4. Do not execute validation commands while loading.

## Validation

- Valid manifest loads.
- Missing required fields fail with structured errors.
- Unknown `kind` fails.
- Unknown `status` fails.

## Done

Every product capability can be declared with one typed manifest.

## Implementation status

**Status: Implemented — rechecked 2026-05-21.**

Implemented in `vac-rs/core/src/control_plane/capability_manifest.rs`
with parser/validation coverage in
`vac-rs/core/src/control_plane/capability_manifest_tests.rs`.

Verified behavior:

- `validate_capability_manifest` is no longer a no-op; it enforces core invariants for memory-constructed manifests.
- `ready` capabilities now require at least one `validation.commands` entry or one `validation.gates` entry.
- Non-`deprecated` capabilities now require `states`.
- Donor-backed capabilities must declare a TUI route or CLI command both at parse-time and through `validate_capability_manifest`.
- Negative tests cover ready-without-evidence, non-deprecated-without-states, and memory-constructed donor-backed manifests without TUI/CLI surface.

Validation run:

```bash
cargo nextest run --manifest-path vac-rs/Cargo.toml -p vac-core --lib -E 'test(capability_manifest)'
```

Result: `27 tests run: 27 passed, 1810 skipped`.

Audit resolution note:

- P1 findings #1, #2, and #3 are closed for the requested Plan 02 lane.
- Donor-backed surface rejection from finding #8 is mirrored in public validation as well as parse-time validation.
- Donor source semantics, ownership semantics, and registry loader behavior were intentionally not changed in this lane.

## 2026-05-28 sandbox closeout

Evidence: `02-evidence/2026-05-28-sandbox-capability-schema-closeout.md`.

The current root `.vac/capabilities` set is synchronized to `ready` capability
status for the default product control plane. The schema contract still permits
`planned` and `partial` for future/non-root capability work, but root readiness is
now backed by owner, policy/surface metadata, and validation evidence.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | Reviewer | Validation | ✅ resolved — public `validate_capability_manifest` enforces the schema invariants | Keep parser and public validator behavior aligned |
| 2 | P1 | SE | Schema | ✅ resolved — non-deprecated capabilities require lifecycle `states` | Keep future root capabilities state-backed |
| 3 | P1 | Architect | Schema | ✅ resolved — `ready` requires validation commands or gates | Do not mark root capabilities ready without evidence |
| 4 | P2 | Reviewer | Schema | ✅ resolved for supported manifest shapes — typed fields reject malformed critical structures | Extend only when adding new nested shapes |
| 5 | P2 | SE | Schema | ✅ resolved by validator/readiness contract — donor-backed semantics are explicit and status-gated | Keep donor invariants visible in manifests |
| 6 | P2 | Planner | Drift | ✅ resolved — Plan 19 root status sync makes `.vac` the source of truth | Keep plan docs and manifests synchronized |
| 7 | P2 | Reviewer | Test | ✅ resolved — negative schema/status/evidence tests exist in capability manifest coverage | Add tests with every schema extension |
| 8 | P2 | Architect | Schema | ✅ resolved — donor-backed capabilities must declare a user-facing TUI or CLI surface | Preserve donor visibility invariant |
| 9 | P3 | Architect | Schema | ✅ resolved — schema version is per manifest kind and fixed to version 1 for the current control plane | Bump deliberately per kind |
| 10 | P3 | Planner | Docs | ✅ resolved — donor semantics are now covered by Plan 20 and capability schema evidence | Keep donor docs linked to manifests |
