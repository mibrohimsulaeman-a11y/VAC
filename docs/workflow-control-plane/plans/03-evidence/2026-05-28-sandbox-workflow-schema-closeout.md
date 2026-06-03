# Plan 03 Evidence — Workflow schema closeout sweep

Date: 2026-05-28
Environment: ChatGPT sandbox source checkpoint

## Result

The remaining Plan 03 audit follow-ups are closed for the default workflow
manifest contract:

- built-in workflow step vocabulary and policy intents are set-equal, not only
  length-equal;
- step ids and validation gate ids now share a documented lowercase grammar;
- validation gates must both match the grammar and reference an existing step id;
- custom/unknown step policy fall-through is handled by Plan 04 default-deny
  policy evaluation.

## Code evidence

- `vac-rs/core/src/control_plane/workflow_manifest.rs`
  - `validate_step_identifier`
  - validation gate grammar check before step-id reference check
- `vac-rs/core/src/control_plane/workflow_manifest_tests.rs`
  - `rejects_invalid_step_id_grammar`
  - `rejects_invalid_validation_gate_grammar`
- `vac-rs/core/src/control_plane/workflow_runner.rs`
  - `workflow_step_vocabulary_is_internally_consistent` now asserts set equality
    between `WORKFLOW_STEP_VOCABULARY` and `WORKFLOW_STEP_POLICY_INTENTS`.

## Validation

- `rustfmt --edition 2024 --check` on changed Rust files
- static source checks for the new grammar validator and set-equality assertion

## Operator meaning

Workflow manifests cannot introduce ambiguous step/gate identifiers or drift the
safe-runner vocabulary away from policy intent coverage.
