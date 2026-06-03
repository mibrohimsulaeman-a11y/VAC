# Plan 03 — Workflow manifest schema


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with audit history; active contract is workflow manifest safety**.
> ## Status
> status: planned
> 2. Validate workflow id, title, status, inputs, steps, ui projection, and validation.

Code evidence:
- `vac-rs/core/src/control_plane/workflow_manifest.rs`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/03-evidence/2026-05-28-sandbox-workflow-schema-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with audit history; active contract is workflow manifest safety**.

Target outcome: workflow manifests define safe typed steps, dependencies, policy requirements, and validation metadata without allowing arbitrary execution by default.

Outputs: workflow schema, parser/validator tests, unknown-field rejection, typed step validation, and docs for required fields.

Requires / Blocks: requires Plan 01 skeleton; blocks safe runner, maintenance workflows, release gate, and PTY/runtime-owner workflows.

Stop conditions: stop if a workflow step can execute untyped shell/process behavior without explicit policy and validation.

Done criteria: malformed workflows fail deterministically, valid workflows load, and safe-step semantics are covered by tests.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Define typed workflow manifests as product operator flows, not free-form scripts.

## Status

Implemented in `vac-core` as `vac_core::control_plane::workflow_manifest`.
The initial loader enforces schema version `1`, denies unknown fields, validates typed step references, and keeps arbitrary shell execution out of the manifest shape.

## Required fields

```yaml
schema_version: 1
kind: workflow
id: product.example
title: Example workflow
status: planned
inputs: {}
steps: []
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
validation:
  commands: []
```

## Implementation

1. Add schema/struct for workflow manifests.
2. Validate workflow id, title, status, inputs, steps, ui projection, and validation.
3. Forbid arbitrary shell execution in initial schema.
4. Permit only typed `uses: capability.*` style step references in the first version.

## Validation

- Workflow with valid typed steps loads.
- Workflow with arbitrary command field fails until an explicit safe runner supports it.
- Missing UI projection fails.
- Known-capability validation is exposed separately so registry-backed resolution can be wired later without loosening the schema.

## Done

A workflow is a typed product declaration that the TUI can list before it can run.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | Reviewer | Schema | ✅ resolved — condition lexer/tokenizer rejects invalid trailing/symbolic chars | Keep condition parser strict |
| 2 | P2 | Architect | Validation | ✅ resolved — registry load validates workflow uses against built-in vocabulary plus known capabilities | Keep combined loader authoritative |
| 3 | P2 | Reviewer | Schema | ✅ resolved — safe-runner vocabulary and policy intents now assert set equality | Keep `uses` sets synchronized |
| 4 | P2 | Reviewer | Schema | ✅ resolved — validation gates must match step-id grammar and reference existing step ids | Preserve gate-to-step linkage |
| 5 | P2 | Planner | Docs | ✅ resolved — workflow policy block is documented and parsed | Keep schema docs current |
| 6 | P3 | Reviewer | Test | ✅ resolved — negative tests cover step/gate identifier grammar | Extend when dependency fields are added |
| 7 | P3 | Architect | Schema | ✅ resolved — current step/gate id grammar is documented in evidence; dependency fields remain out of schema v1 | Add a new grammar only with a dependency field |

---

## Resolution log (2026-05-21)

Subset findings ditutup sebagai bagian dari hardening Plan 03 (commit pending, no `git add`):

- **#1 (P1)** — Condition lexer strict: `ConditionParser::new` di `vac-rs/core/src/control_plane/workflow_manifest.rs` membangun token list dari karakter terbatas (`and`/`or`/`not`/`(`/`)`/identifier). `next_token` me-return `Err` untuk karakter tak dikenal; `read_identifier` hanya match keyword exact. Tests: `condition_parser_accepts_keyword_operators`, `condition_parser_rejects_legacy_symbolic_operators` (`&&`/`||`/`!`), `condition_parser_rejects_equality_operators` (`==`/`!=`).
- **#2 (P2)** — Known-capability validation: `validate_workflow_manifest_against_known_capabilities` di-hook via combined `load_workflow_registry`. `validate_workflow_steps` menerima `Option<&HashSet<String>>` dan menerapkan **union check**: `step.uses` diterima bila ada di `WORKFLOW_STEP_VOCABULARY` (via `capability.` prefix strip) ATAU di `known_capabilities`. Tests: `validates_uses_against_vocabulary_with_union_known_set` + `rejects_unknown_capability_when_resolved_against_registry`.
- **#4 (P2)** — `validation.gates` reject string yang bukan step id. Test: `rejects_validation_gate_that_does_not_reference_step_id`.
- **#5 (P2)** — Plan doc updated: `docs/workflow-control-plane/schema/workflow-manifest.schema.md` mendokumentasikan field `policy` (default_risk, mutates_files, approval_required_for, reason). Parse test: `parses_policy_block_with_all_fields`.

Remaining findings #3, #6, and #7 were closed by the 2026-05-28 sandbox closeout recorded in `03-evidence/2026-05-28-sandbox-workflow-schema-closeout.md`:

- vocabulary vs policy intent now has set-equality coverage in `workflow_step_vocabulary_is_internally_consistent`;
- step ids and validation gates share a lowercase grammar enforced by `validate_step_identifier`;
- validation gates are grammar-checked before the existing reference-to-step-id check.

**Validation**:
- Historical: `cargo nextest run -p vac-core --lib -E 'test(workflow_manifest)'` → **17/17 pass** (16 baseline + 1 vocab-union).
- Sandbox closeout: `rustfmt --edition 2024 --check` on changed Rust files plus static source checks for set-equality and identifier-grammar coverage.
