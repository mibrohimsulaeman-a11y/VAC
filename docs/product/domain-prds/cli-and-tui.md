# PRD — CLI and TUI

## Overview

The CLI and TUI form the primary operator entrypoint for VAC.

The CLI should be small and product-focused. The TUI should be the operator cockpit for capabilities, workflows, approvals, activity, diagnostics, and session state.

## CLI requirements

### Required initial commands

```text
vac
vac exec
vac review
vac apply
vac resume
vac completion
```

### Planned commands

```text
vac workflow
vac capabilities
```

These may remain planned until the control-plane registry and TUI surfaces exist.

### Removed from initial product surface

The default CLI must not expose internal service, proxy, cloud, or debug surfaces as product commands. Those can return later only if represented as capabilities and workflows.

## TUI requirements

The root TUI must provide:

- chat/input composer,
- activity log,
- status bar,
- approval surface,
- capability dashboard,
- workflow browser,
- workflow progress,
- policy/diagnostic surface.

## TUI layout target

```text
status / session / model / policy
activity and conversation
capability or workflow panel
approval/progress detail
input composer
```

## Input requirements

- plain text submits a task,
- slash commands route through declared surfaces,
- command palette aligns with surface manifests,
- `Ctrl-C` exits cleanly,
- paste and Enter behavior must work in a real PTY.

## Capability dashboard

The TUI must show:

- capability id,
- status,
- owner,
- surfaces,
- validation,
- policy class,
- errors.

## Workflow browser

The TUI must show:

- workflow id,
- title,
- status,
- inputs,
- steps,
- policy gates,
- validation commands.

## Approval surface

The TUI approval surface must be singular. It must show risk, reason, resources, preview, and approve/reject controls.

## Acceptance criteria

- `vac` opens the root TUI.
- `vac --help` is small and product-focused.
- `/capabilities` shows declared capabilities once implemented.
- `/workflow` shows declared workflows once implemented.
- No backend feature is considered ready if it has no TUI or CLI surface.
