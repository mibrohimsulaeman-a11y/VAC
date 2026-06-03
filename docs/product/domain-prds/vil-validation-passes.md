# PRD — VIL Validation Passes and Semantic IR

## Overview

VAC's VIL-native capability requires deterministic validation beyond generic code review.

This PRD covers:

```text
vac.vil_detect
vac.vil_expr
vac.vil_ir
vac.vil_validation_passes
vac.vwfd_parity
vac.vwfd_diff
vac.vil_repair_preview
vac.vil_generation_preview
```

## Product goal

For VIL/VWFD repositories, VAC should understand workflow definitions, expressions, handlers, and semantic correctness.

The user should feel:

```text
VAC understands VIL as a native domain, not as generic text files.
```

## Capability model

| Capability | Purpose | Priority |
|---|---|---:|
| `vac.vil_detect` | Detect VIL/VWFD project markers. | P1/P2 |
| `vac.vil_expr` | Parse/lint/validate VIL expressions. | P2 |
| `vac.vil_ir` | Build semantic IR for code/workflow analysis. | P2 |
| `vac.vil_validation_passes` | Run deterministic validation passes. | P2 |
| `vac.vwfd_parity` | Check workflow declarations against handlers. | P2 |
| `vac.vwfd_diff` | Show domain-aware changes to workflow definitions. | P2 |
| `vac.vil_repair_preview` | Preview domain fixes before approval. | P2 |
| `vac.vil_generation_preview` | Preview generated handlers/artifacts before mutation. | P2 |

## Validation pass categories

Initial pass categories:

```text
schema
expression_parse
symbol_resolution
type_check
handler_parity
condition_reachability
side_effect_policy
workflow_graph
implementation_link
compliance
```

## Diagnostic model

```yaml
diagnostic:
  id: vil_diag_123
  severity: error
  pass: handler_parity
  file: workflow.vwfd.yaml
  span: line 42
  message: Handler is declared but implementation is missing.
  repair_available: true
```

Severity:

```text
info
warning
error
blocked
```

## Semantic IR requirements

Semantic IR should support:

- workflow tree,
- triggers,
- steps,
- handlers,
- expressions,
- symbols,
- implementation links,
- type/ownership hints where available,
- diffable semantic nodes.

## Repair/generation requirements

Repairs and generation must be preview-first.

Any mutation must route through:

```text
changeset preview -> policy -> approval -> bounded patch -> validation -> evidence
```

## TUI surfaces

```text
/vil
/vwfd
/evidence
/changeset
```

TUI should show:

- validation pass list,
- diagnostics grouped by severity,
- workflow tree,
- handler parity,
- repair preview,
- validation after fix.

## Acceptance criteria

### MVP

- VIL/VWFD detection can enable domain surfaces.
- VWFD files can be listed and parsed when supported.
- Diagnostics include severity, file, pass, and message.
- Repair/generation is preview-first.

### Safety

- Mutating repair requires approval.
- Generated files are bounded and validated.
- Domain validation does not require knowledge add-on connection.

### UX

- User sees what validation pass failed.
- User can inspect workflow tree.
- User can preview domain-aware fix before applying.
