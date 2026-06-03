# VIL Native and VIL Knowledge Add-on architecture

## Decision

VAC will support VIL through a hybrid product architecture:

```text
VIL Native Core              optional built-in deterministic capability
VIL Knowledge Add-on          one-click managed connector for knowledge/rules/examples
```

This means VIL is not mandatory for every VAC user, but when a project is VIL/VWFD-based, VAC can become VIL-aware without forcing manual server setup.

## Product rule

```text
VAC works as a strong coding CLI for any repo.
In VIL repos, VAC becomes VIL-native.
With the VIL Knowledge Add-on, VAC becomes VIL-aware and team-aware without local setup.
```

## Why hybrid

VIL support has two different classes of work.

### Deterministic product behavior

These belong in native VAC runtime because they must be fast, reliable, offline-capable, policy-aware, and TUI-visible:

- VWFD parsing,
- VWFD schema validation,
- VIL expression linting,
- handler parity checks,
- workflow tree explain,
- semantic diagnostics,
- safe repair preview,
- approval-gated fix application,
- `/vil` and `/vwfd` workbenches.

### Knowledge and rule guidance

These belong in a managed add-on because they evolve faster than the binary and may be team-specific:

- VIL docs search,
- VIL Way rules,
- best-practice corpus,
- examples retrieval,
- review rubrics,
- project templates,
- team-specific conventions,
- internal platform guidance,
- migration recipes.

## User experience

The user must not need to install or operate a local server.

Target setup flow:

```text
vac
/vil
VIL Native: ready
VIL Knowledge: not connected
[Connect VIL Knowledge]
```

After enabling:

```text
VIL Native: ready
VIL Knowledge: connected
Rule packs:
  - VIL Way
  - VWFD Production Patterns
  - Vastar Infra Conventions
```

The user should experience this as a connector/add-on, not as manual MCP server administration.

## Agent tool-calling flow

When the user asks for VIL work:

```text
Make this VIL handler production-ready.
```

VAC should perform:

```text
1. Native scan
   - parse VWFD
   - inspect handlers
   - run deterministic validation

2. Knowledge lookup
   - retrieve relevant VIL Way rules
   - retrieve matching examples
   - retrieve team conventions when enabled

3. Semantic plan
   - propose fix
   - explain rules used
   - classify risk

4. Approval checkpoint
   - preview changed files
   - ask approval if mutating

5. Execution
   - apply bounded patch through VAC runtime

6. Validation
   - rerun native VIL/VWFD validation

7. Evidence
   - show rules used
   - show validation result
   - show changed files and why
```

## Trust and safety

The VIL Knowledge Add-on is read-only by default.

Allowed by default:

- docs search,
- examples retrieval,
- rule lookup,
- review rubric retrieval,
- explanation.

Not allowed directly:

- write files,
- apply patches,
- run shell commands,
- install dependencies,
- mutate project configuration.

Any mutating action must route through native VAC policy and approval.

## Capability split

```text
vac.vil_native
  deterministic local parser/validator/workbench

vac.vwfd_workbench
  native workflow-definition inspection and validation

vac.vil_knowledge
  optional managed knowledge connector

vac.vil_rules
  optional rule-pack surface from add-on or local policy
```

## TUI surface split

### `/vil`

Shows:

- native status,
- detected VIL project status,
- validator readiness,
- knowledge add-on connection status,
- active rule packs,
- diagnostics summary.

### `/vwfd`

Shows:

- VWFD file tree,
- workflow tree,
- triggers,
- handlers,
- expression diagnostics,
- parity status,
- repair preview.

### `/capabilities`

Shows:

```text
vac.vil_native        ready | partial | planned
vac.vwfd_workbench    ready | partial | planned
vac.vil_knowledge     optional | connected | unavailable
vac.vil_rules         optional | connected | none
```

## Control-plane representation

Example capability manifest for native VIL:

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
  cli: false
policy:
  mutates_files: false
  network: false
validation:
  commands:
    - cargo check -p vac-surface-cli
```

Example capability manifest for the add-on:

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
  cli: false
addon:
  type: managed_connector
  protocol: mcp_compatible
  connection: one_click
  required: false
policy:
  mutates_files: false
  network: connector_scoped
  approval_required_for:
    - tool_call_with_side_effects
validation:
  commands: []
```

## Implementation constraints

- Native VIL cannot require the add-on.
- The add-on cannot be required for `/vil` to be useful.
- The add-on must not mutate files directly.
- Add-on results must be cited or attributed as retrieved knowledge in evidence summaries.
- Connector credentials must be stored securely.
- Add-on failure must degrade gracefully: native validation still works.
- VIL features are optional for non-VIL repos.

## Acceptance criteria

- VAC works normally in non-VIL repositories.
- VIL repository detection can enable VIL surfaces without manual config.
- `/vil` shows native readiness even when the knowledge add-on is disconnected.
- One-click add-on connection status is visible.
- Agent can use VIL knowledge when connected.
- Mutating fixes are always applied by VAC runtime, not directly by add-on tools.
- Evidence summary lists native validation and knowledge/rules used.
