# PRD — VIL Native and Knowledge Add-on

## Overview

VAC will support VIL through an optional hybrid product model:

```text
VIL Native Core        built-in deterministic VIL/VWFD capability
VIL Knowledge Add-on   one-click managed connector for knowledge/rules/examples
```

This keeps VAC excellent for normal coding while making VIL projects feel first-class.

## Product promise

```text
VAC works as a great coding CLI for any repo.
When the repo is VIL, VAC becomes VIL-native.
When the VIL Knowledge Add-on is enabled, VAC becomes VIL-aware and team-aware without local setup.
```

## User problem

Users who code with VIL need two different things:

1. deterministic local behavior such as parse, validate, lint, parity, and workbench UI,
2. evolving knowledge such as docs, rules, examples, team conventions, and review rubrics.

Forcing all VIL behavior into native runtime would slow knowledge iteration. Making all VIL behavior an add-on would weaken core UX and require connectivity for basic validation.

## Product decision

Use native core for deterministic behavior and managed add-on for knowledge.

## VIL Native Core

Native core owns:

- VWFD document parsing,
- VWFD schema validation,
- VIL expression linting,
- workflow tree explain,
- handler parity checks,
- semantic diagnostics,
- repair preview,
- approval-gated fix application,
- `/vil` and `/vwfd` TUI surfaces.

Native VIL must work without the add-on.

## VIL Knowledge Add-on

The add-on owns:

- VIL docs search,
- VIL Way rules retrieval,
- examples retrieval,
- best-practice corpus,
- project templates,
- review rubrics,
- migration recipes,
- team-specific conventions,
- internal platform guidance when connected.

The add-on should be enabled through one-click product UX. The user should not need to set up a server manually.

## User flow

### Non-VIL repository

```text
vac
VIL Native: inactive
VIL Knowledge: optional
```

VAC behaves like a normal autonomous coding CLI.

### VIL repository without add-on

```text
vac
/vil
VIL Native: ready
VIL Knowledge: not connected
```

User can still parse, validate, inspect, and diagnose VIL/VWFD files.

### VIL repository with add-on

```text
vac
/vil
VIL Native: ready
VIL Knowledge: connected
Rule packs:
  - VIL Way
  - VWFD Production Patterns
  - Team Conventions
```

Agent can retrieve rules and examples while coding.

## Agent behavior

When user asks:

```text
Make this VIL handler production-ready.
```

VAC should:

1. run native VIL/VWFD scan,
2. retrieve relevant knowledge/rules if add-on is connected,
3. create semantic plan,
4. classify risk,
5. show preview/approval for mutations,
6. apply changes through VAC runtime,
7. rerun native validation,
8. produce evidence summary including rules used.

## Capability requirements

| Capability | Type | Required | Description |
|---|---|---:|---|
| `vac.vil_native` | native | optional core | Local VIL parser/validator/workbench. |
| `vac.vwfd_workbench` | native | optional core | VWFD inspect/explain/parity surface. |
| `vac.vil_knowledge` | managed add-on | optional | Knowledge/rules/examples connector. |
| `vac.vil_rules` | add-on or local policy | optional | Active rule packs and team conventions. |

## TUI requirements

### `/vil`

Must show:

- detected VIL project status,
- native validator readiness,
- knowledge add-on connection status,
- active rule packs,
- diagnostics summary,
- actions: validate, explain, connect knowledge add-on.

### `/vwfd`

Must show:

- VWFD files,
- workflow tree,
- triggers,
- handlers,
- conditions,
- parity status,
- diagnostics,
- repair preview.

### `/capabilities`

Must show native and add-on capability states separately:

```text
vac.vil_native        ready | partial | planned | inactive
vac.vwfd_workbench    ready | partial | planned | inactive
vac.vil_knowledge     optional | connected | unavailable
vac.vil_rules         optional | connected | none
```

## Safety requirements

The add-on is read-only by default.

Allowed by default:

- docs search,
- examples retrieval,
- rule lookup,
- rubric retrieval,
- explanation.

Not allowed directly:

- file writes,
- patch application,
- shell commands,
- dependency installation,
- configuration mutation.

Any mutating behavior must run through native VAC runtime, policy, approval, and evidence.

## Control-plane examples

Native capability:

```yaml
schema_version: 1
kind: capability
id: vac.vil_native
title: VIL Native
status: planned
surfaces:
  tui: /vil
  slash: /vil
  palette: true
policy:
  mutates_files: false
  network: false
```

Knowledge add-on capability:

```yaml
schema_version: 1
kind: capability
id: vac.vil_knowledge
title: VIL Knowledge Add-on
status: optional
surfaces:
  tui: /vil
  slash: /vil
  palette: true
addon:
  type: managed_connector
  protocol: mcp_compatible
  connection: one_click
  required: false
policy:
  mutates_files: false
  network: connector_scoped
```

## Acceptance criteria

### MVP

- Non-VIL repos are not burdened by VIL requirements.
- VIL repo detection can mark VIL Native as available or planned.
- `/vil` works without knowledge add-on connection.
- Knowledge add-on appears as optional connector, not manual server setup.
- Agent can use add-on knowledge when connected.
- Add-on failure degrades gracefully.

### Safety

- Add-on tools cannot directly mutate project files.
- Add-on-derived suggestions still require VAC approval before mutation.
- Evidence summary distinguishes native validation from retrieved knowledge.
- Connector credentials are scoped and stored securely.

### Product

- User sees VIL as native capability, not mandatory dependency.
- User sees knowledge add-on as easy connector, not infrastructure setup.
- Team-specific VIL rules can be updated without rebuilding VAC.
