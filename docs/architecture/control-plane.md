# VAC control plane architecture

## Purpose

The `.vac/` directory is the declarative control plane for the product.

## Scope note — root control plane vs user project workspace

For the VAC product repository, `.vac` is the strict manifest-driven control plane: capabilities, workflows, policies, surfaces, and registry metadata must stay synchronized with Rust validators and doctor output.

For arbitrary user projects, `.vac` starts as a zero-config local agent workspace. VAC may use an in-memory profile or a soft workspace before strict manifests exist. Strict capability/policy/workflow/surface manifests are an opt-in promotion path, not a first-run requirement for existing projects.


It answers:

```text
What capabilities exist?
What workflows exist?
What policies apply?
Where does each feature appear in the TUI/CLI?
What validation proves it works?
What status should the operator see?
```

## Control plane directories

```text
.vac/capabilities/   feature declarations
.vac/workflows/      typed operator workflows
.vac/policies/       safety and execution policy
.vac/surfaces/       TUI/slash/palette/CLI exposure
.vac/registry/       product/domain/status metadata
```

## Control plane responsibilities

The control plane declares:

- capability id,
- owner crate/module,
- lifecycle status,
- TUI/CLI surfaces,
- workflow steps,
- policy gates,
- approval requirements,
- validation commands,
- operator-facing states.

The control plane does not execute code.

## Runtime relationship

```text
.vac YAML
  -> Rust registry loader
  -> typed registry snapshot
  -> policy engine
  -> workflow runner
  -> TUI dashboard/browser/progress
```

## Manifest categories

### Capability manifest

Declares a product feature.

### Workflow manifest

Declares an operator flow that composes capabilities.

### Policy manifest

Declares safety rules and execution constraints.

### Surface manifest

Declares what appears in TUI, slash help, palette, and CLI.

### Registry manifest

Declares product-wide metadata and migration status.

## Design constraints

- Manifests must be typed.
- Manifests must validate before use.
- Invalid manifests must be visible in TUI.
- Mutating workflows must require policy checks.
- YAML must not become arbitrary scripting.
- Every manifest must include owner and status.

## Status model

Recommended capability/workflow statuses:

```text
ready
partial
planned
broken
cli_only
retired
test_only
```

The status shown in TUI must match actual implementation state.
