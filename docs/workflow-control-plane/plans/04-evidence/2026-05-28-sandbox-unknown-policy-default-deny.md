# Plan 04 Evidence — Unknown workflow step policy default-deny

Date: 2026-05-28
Environment: ChatGPT sandbox source checkpoint

## Result

Custom/unknown workflow step `uses` values no longer bypass policy evaluation.
`evaluate_workflow_step_policy_decisions` returns
`PolicyDecisionReport::UnknownPolicy` for unknown step vocabulary entries.
`UnknownPolicy` is treated as approval-required / non-green by the workflow
execution surface.

## Code evidence

- `vac-rs/core/src/control_plane/workflow_runner.rs`
  - `evaluate_workflow_step_policy_decisions`
  - test `custom_step_policy_decision_defaults_to_unknown_policy`

## Validation

- `rustfmt --edition 2024 --check vac-rs/core/src/control_plane/workflow_runner.rs`
- static source check for `PolicyDecisionReport::UnknownPolicy` on custom step

## Operator meaning

Custom workflow extensions can still exist, but they must be explicitly reviewed.
A step cannot fall through as ungated work just because the built-in vocabulary
has no matching typed policy intent.
