# PRD — VIL and VWFD native tooling

## Overview

VAC should provide native support for VIL/VWFD project workflows through declared capabilities and TUI workbenches.

This is a differentiated product area and should be added after the workflow control plane exists.


## Relationship to VIL Knowledge Add-on

VIL/VWFD native tooling covers deterministic local behavior: parse, validate, lint, explain, parity, repair preview, and approval-gated mutation. It must work without any external add-on.

The VIL Knowledge Add-on is a separate optional managed connector for docs, rules, examples, rubrics, and team conventions. See `docs/product/domain-prds/vil-native-knowledge-add-on.md` and `docs/architecture/vil-native-and-knowledge-add-on.md`.

## VIL/VWFD capabilities

Initial capabilities:

- parse VWFD document,
- validate workflow definition,
- explain workflow structure,
- compare declared handlers to implementation,
- lint expression conditions,
- preview generated changes,
- surface repair proposals.

## Workbench surfaces

Planned TUI surfaces:

```text
/vil
/vwfd
```

## VIL workbench requirements

The VIL surface should show:

- diagnostics by severity,
- affected file/span,
- validation pass,
- repair proposal,
- approval-gated fix action.

## VWFD workbench requirements

The VWFD surface should show:

- workflow tree,
- triggers,
- handlers,
- step conditions,
- execution mode,
- parity status,
- jump-to-source links.

## Expression validation

Expression linting should report:

- undefined symbol,
- type mismatch,
- unreachable condition,
- unsafe or unsupported expression,
- style suggestion.

## Generation requirements

Any generated code or file mutation must:

- show preview/diff,
- require approval when mutating,
- update workflow progress,
- run validation after write.

## Acceptance criteria

- VIL/VWFD capabilities are declared in `.vac/capabilities` before code migration.
- TUI surfaces exist before marking capability ready.
- Validation failures are visible and actionable.
- Mutating repair/generation routes through approval.
