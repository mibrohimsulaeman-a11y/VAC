# PRD — Workflow control plane

## Overview

The workflow control plane is the product structure that makes VAC workflow-native.

It lives under `.vac/` and declares capabilities, workflows, policies, surfaces, and registry metadata.

## Scope note — root control plane vs user project workspace

For the VAC product repository, `.vac` is the strict manifest-driven control plane: capabilities, workflows, policies, surfaces, and registry metadata must stay synchronized with Rust validators and doctor output.

For arbitrary user projects, `.vac` starts as a zero-config local agent workspace. VAC may use an in-memory profile or a soft workspace before strict manifests exist. Strict capability/policy/workflow/surface manifests are an opt-in promotion path, not a first-run requirement for existing projects.


## Goals

- Make all product features discoverable.
- Make all operator workflows explicit.
- Make policy requirements machine-readable.
- Make TUI and CLI surfaces consistent.
- Prevent hidden backend features.
- Prevent duplicate frontend/runtime paths.

## Control plane directories

```text
.vac/capabilities/
.vac/workflows/
.vac/policies/
.vac/surfaces/
.vac/registry/
```

## Capability manifest requirements

Each capability must declare:

- id,
- title,
- status,
- owner,
- surfaces,
- states,
- policy,
- validation.

## Workflow manifest requirements

Each workflow must declare:

- id,
- title,
- status,
- inputs,
- typed steps,
- UI projection,
- policy gates,
- validation.

## Policy manifest requirements

Policies must classify:

- filesystem access,
- network access,
- tool execution,
- sandbox posture,
- approval requirements,
- secret handling.

## Surface manifest requirements

Surfaces must map capabilities to:

- TUI routes,
- slash commands,
- palette commands,
- CLI commands.

## Runtime requirements

Rust must:

- load manifests,
- validate schemas,
- emit structured diagnostics,
- resolve typed capability handlers,
- execute workflows safely,
- enforce policy,
- publish lifecycle events.

## TUI requirements

The TUI must show control-plane state through:

- capability dashboard,
- workflow browser,
- workflow progress,
- manifest diagnostics.

## Acceptance criteria

- `.vac` skeleton exists.
- Registry loader validates manifests.
- TUI can show capabilities and workflows.
- Safe built-in workflows can run.
- Donor-backed features cannot enter product without manifest metadata.
