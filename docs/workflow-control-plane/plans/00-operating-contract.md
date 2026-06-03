# Plan 00 — Operating contract and architecture invariants


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented / invariant baseline**.

Code evidence:
- `vac-rs/core/src/control_plane/architecture_invariants.rs`
- `vac-rs/tui`
- `vac-rs/core`
- `vac-cli/bin/vac.js`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented / invariant baseline**.

Target outcome: every control-plane and runtime migration plan preserves one product command, one product TUI, explicit ownership, visible operator state, and evidence-backed validation.

Outputs: architecture invariants, operational rules, validation expectations, and stop conditions that apply to all downstream plans.

Requires / Blocks: root prerequisite for all Phase 00 and control-plane plans; blocks any plan that would create a second product command/TUI or hide validation state.

Stop conditions: stop if a downstream plan violates one-command/one-TUI invariants, lacks validation evidence, or requires broad unrelated cleanup.

Done criteria: invariants are documented, referenced by the plan index, and reflected in downstream plan contracts.


## Goal

Make the workflow control plane the primary repository pattern before any donor-backed product work starts.

## Scope

- Root product command: `vac`
- Root TUI: `vac-rs/tui`
- Root runtime: `vac-rs/core`
- Package launcher: `vac-cli/bin/vac.js`
- Declarative control plane: `.vac/`

## Implementation

1. Treat `.vac/` as the source of product capability/workflow/policy/surface declarations.
2. Treat Rust as the only execution/safety/runtime implementation.
3. Treat TUI as the required operator visibility layer.
4. Reject backend-only feature work.
5. Reject duplicate frontend/runtime constructs.

## Required invariants

```text
YAML declares.
Rust executes.
TUI observes.
Policy gates.
Approval protects.
Dead code is deleted or quarantined.
```

## Validation

- Root docs describe VAC as workflow-native.
- No product docs tell implementers to create another TUI.
- Every future plan references active root path only.

## Done

The repo has one clear implementation contract and every later plan inherits it.
