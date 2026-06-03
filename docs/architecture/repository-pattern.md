# VAC repository pattern

## Target shape

VAC uses a workflow-control-plane repository pattern.

```text
.vac/
  capabilities/
  workflows/
  policies/
  surfaces/
  registry/

vac-rs/
  cli/
  tui/
  core/
  workflow/
  capability/
  policy/
  providers/

vac-cli/
  bin/vac.js

donor/vac/
  source-only donor material
```

The layout communicates product architecture before anyone reads code.

## What each root area owns

| Area | Owns | Must not own |
|---|---|---|
| `.vac/` | declarations, registry, policy, surfaces, workflow metadata | runtime execution, scripts, alternate frontend code |
| `vac-rs/` | runtime, TUI, policy enforcement, execution, validators | product identity drift, hidden unregistered capabilities |
| `vac-cli/` | launcher and package entrypoint | product behavior not backed by Rust/TUI |
| `docs/architecture/` | product design contract | implementation progress claims |
| `docs/workflow-control-plane/` | migration execution plans | final product truth after implementation lands |
| `donor/vac/` | source-only reference material | active product runtime |

## Allowed root concepts

VAC root may contain:

- one TUI path,
- one CLI product command,
- one declarative control plane,
- one capability registry,
- one workflow registry,
- one policy/approval model,
- one execution lifecycle model.

## Forbidden root concepts

VAC root must not contain:

- a second product TUI,
- a second command registry,
- duplicate approval bars,
- hidden server/proxy product modes,
- ad-hoc skill packs under `.vac/`,
- backend-only product claims,
- donor frontend imports as product UI.

## Repository hygiene rule

If a crate/module/file is not one of these, it must be classified:

```text
product runtime
product TUI
product launcher
control-plane declaration
architecture/planning docs
donor source material
test fixture
retired/delete candidate
```

Unclassified code is technical debt.
