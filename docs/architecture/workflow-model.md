# VAC workflow model

## Definition

A workflow is a typed operator flow that composes capabilities into a multi-step task.

Workflow examples:

```text
submit chat task
review changes
apply patch
run build check
validate identity
run release gate
execute product workflow
```

## Workflow manifest responsibilities

A workflow manifest declares:

- id,
- title,
- status,
- inputs,
- steps,
- UI projection,
- policy gates,
- validation commands.

## Step model

Each step has:

- id,
- capability reference,
- input mapping,
- policy requirement,
- condition when applicable,
- expected output,
- retry/cancel behavior when supported.

Example:

```yaml
steps:
  - id: build
    uses: capability.build.cargo_check
    with:
      package: vac-cli
```

## Initial allowed step references

```text
capability.build.cargo_check
capability.identity.check
capability.activity.emit
capability.approval.request
capability.tui.pty_gate
```

## Workflow lifecycle

```text
pending
running
waiting_approval
success
failed
cancelled
```

## Dry-run

A workflow should support dry-run when it can affect files, tools, network, or configuration.

Dry-run should show:

- steps that would run,
- resources affected,
- policy gates,
- required approvals,
- validation commands,
- known risks.

## Workflow anti-patterns

- arbitrary shell step as default primitive,
- no UI projection,
- no policy checks,
- no lifecycle state,
- mutating workflow without dry-run or approval,
- workflow that cannot be listed in the TUI.
