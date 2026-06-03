# Workflow manifest schema (v1)

This document describes the YAML manifest format consumed by
`vac-core::control_plane::workflow_manifest`. Manifests live under
`.vac/workflows/<id>.workflow.yaml` and are loaded by the workflow
registry.

## Top-level shape

| Field            | Type                                  | Required | Notes                                                                 |
| ---------------- | ------------------------------------- | -------- | --------------------------------------------------------------------- |
| `schema_version` | `u32`                                 | yes      | Must equal `1`. Higher versions are rejected.                         |
| `kind`           | enum                                  | yes      | Must equal `workflow`.                                                |
| `id`             | string                                | yes      | Must start with `product.`, `maintenance.`, or `vac.`. No whitespace. |
| `title`          | string                                | yes      | Human-readable name.                                                  |
| `status`         | enum                                  | yes      | `planned`, `partial`, `ready`, or `disabled`.                         |
| `inputs`         | map<string, WorkflowInputSpec>        | yes      | May be empty.                                                         |
| `steps`          | array<WorkflowStep>                   | yes      | Must contain at least one step.                                       |
| `ui`             | WorkflowUi                            | yes      | Root projection hints.                                                |
| `policy`         | WorkflowPolicy                        | yes      | See [Policy block](#policy-block).                                    |
| `validation`     | WorkflowValidation                    | yes      | See [Validation gates](#validation-gates).                            |

The deserializer uses `serde(deny_unknown_fields)` on every record;
unknown keys are rejected with a precise field path.

## WorkflowStep

| Field    | Type                | Required | Notes                                                                                       |
| -------- | ------------------- | -------- | ------------------------------------------------------------------------------------------- |
| `id`     | string              | yes      | Unique within the workflow. No whitespace.                                                  |
| `uses`   | string              | yes      | Must start with `capability.`. See [Capability vocabulary](#capability-vocabulary).         |
| `when`   | string              | no       | Condition expression. See [Condition expressions](#condition-expressions-when).             |
| `policy` | WorkflowStepPolicy  | no       | Same shape as the top-level policy, every field optional. At least one must be set.         |
| `ui`     | WorkflowStepUi      | no       | Optional projection overrides for the step. At least one hint must be set when present.     |

## Condition expressions (`when`)

`when` accepts a strict boolean expression over input identifiers.
The grammar is:

```
expression := or_expr
or_expr    := and_expr ('or' and_expr)*
and_expr   := unary ('and' unary)*
unary      := 'not' unary | primary
primary    := identifier | '(' expression ')'
```

- Keywords: `and`, `or`, `not` (case-sensitive, reserved).
- Identifiers match `[A-Za-z_][A-Za-z0-9_.\-]*`.
- Parentheses group sub-expressions.

The following are **rejected**:

- Symbolic boolean operators (`&&`, `||`, `!`).
- Comparison and equality operators (`==`, `!=`, `<`, `>`, `<=`, `>=`).
- String, number, or boolean literals (for example `prompt == ""`).
- Trailing characters after a complete expression.

Validation errors surface as `workflow.invalid_condition` with the
parser's reason appended.

## Policy block

The top-level `policy` field is mandatory and must declare at least
one decision. Step-level policies share the same shape and the same
"at least one decision" rule.

| Field                   | Type                  | Notes                                                                  |
| ----------------------- | --------------------- | ---------------------------------------------------------------------- |
| `default_risk`          | `string` (optional)   | Free-form risk tier, for example `low`, `medium`, `high`.              |
| `mutates_files`         | `bool` \| `string`    | Polymorphic; either a boolean or a labelled string (e.g. `scoped`).    |
| `network`               | `bool` \| `string`    | Same polymorphism (e.g. `false`, `blocked`, `outbound_only`).          |
| `redaction`             | `bool` \| `string`    | Same polymorphism (e.g. `true`, `secrets_only`).                       |
| `approval_required_for` | `array<string>`       | Tags requiring human approval before execution.                        |

`WorkflowPolicyValue` is encoded with `#[serde(untagged)]` so YAML
producers may emit either booleans (`mutates_files: false`) or strings
(`mutates_files: scoped`). The runtime treats the two variants
distinctly and refuses to silently coerce.

Empty policies (all fields unset and `approval_required_for` empty)
are rejected with `workflow.empty_policy`.

## Capability vocabulary

Each step's `uses` field must resolve through one of two paths:

1. **Built-in vocabulary** — `WORKFLOW_STEP_VOCABULARY` declared in
   `workflow_runner.rs`. Entries map a capability id to a Rust
   handler. Vocabulary members are always accepted.
2. **Registry-known capability** — when the registry passes a set of
   known capability ids to
   `validate_workflow_manifest_against_known_capabilities`, the
   member set is the **union** of (1) and the registry-provided set.

If `uses` is outside both sets the workflow is rejected with
`workflow.unknown_capability`. The plain `parse_workflow_manifest`
entry point only enforces the `capability.` prefix and is intentionally
loose so registry diagnostics can run on partially-mapped workflows.

## Validation gates

`validation.gates` must reference existing step ids. Each gate is the
id of the step whose success counts as a workflow-level checkpoint.

- A `ready` workflow must declare at least one of
  `validation.commands` or `validation.gates`. Otherwise the manifest
  is rejected with `workflow.ready_without_validation`.
- A gate referencing a missing step id is rejected with
  `workflow.invalid_gate_ref`.

## Error catalog

| Code                                  | Trigger                                                              |
| ------------------------------------- | -------------------------------------------------------------------- |
| `workflow.unsupported_schema`         | `schema_version != 1`.                                               |
| `workflow.invalid_id`                 | `id` missing required prefix or contains whitespace.                 |
| `workflow.empty_steps`                | `steps` array empty.                                                 |
| `workflow.duplicate_step_id`          | Two steps share an `id`.                                             |
| `workflow.invalid_uses_prefix`        | `uses` does not start with `capability.`.                            |
| `workflow.unknown_capability`         | Registry validation: `uses` not in vocabulary ∪ known set.           |
| `workflow.invalid_condition`          | `when` rejected by the condition parser.                             |
| `workflow.invalid_surface`            | UI surface does not start with `/`.                                  |
| `workflow.empty_policy`               | All policy decisions unset.                                          |
| `workflow.ready_without_validation`   | `status: ready` and no validation gate or command.                   |
| `workflow.invalid_gate_ref`           | `validation.gates[i]` does not reference an existing step id.        |
