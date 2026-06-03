# VAC product architecture

## Product identity

VAC is not a prompt-only coding helper. VAC is a workflow-native agentic CLI for controlled, repeatable, policy-bound software work.

The product command is:

```bash
vac
```

The product operator surface is the root TUI.

## Product thesis

```text
Software agents become trustworthy when their capabilities, workflows, policies, approvals, and progress are explicit and visible.
```

VAC turns agentic work into visible operator workflows.

## What VAC optimizes for

VAC optimizes for:

- repeatable software workflows,
- clear capability ownership,
- visible progress,
- safe mutation,
- approval-aware execution,
- policy-as-product behavior,
- auditable outcomes,
- dead-code prevention,
- TUI-first operator clarity.

VAC does not optimize for hidden magic or one-off backend feature accumulation.

## Product layers

```text
User/operator
  -> TUI surface
  -> workflow browser / capability dashboard
  -> workflow runner
  -> policy and approval gates
  -> runtime capability handlers
  -> tools, files, model providers, validators
```

## Primary operator experience

The first-class operator flow should look like this:

```text
vac
  -> TUI opens
  -> user sees status, capabilities, workflows
  -> user selects or types a task
  -> VAC resolves capability/workflow
  -> VAC dry-runs or prepares execution when needed
  -> policy determines approval requirements
  -> user approves or rejects risky steps
  -> VAC executes typed steps
  -> TUI shows lifecycle and validation
  -> final output includes patch/evidence/status
```

## Product boundaries

VAC product code must be reachable from the root command or explicitly declared as future/planned/retired.

A product feature is not real unless it has:

- a capability manifest,
- an owner,
- a policy classification,
- a TUI or CLI surface,
- visible empty/loading/success/failure states,
- validation commands,
- cleanup status if sourced from donor material.

## Design smell checklist

The following are product smells:

- backend code without a capability manifest,
- command visible in help but absent from TUI/capability dashboard,
- hidden service command in the main CLI,
- feature-specific approval UI outside the root approval model,
- workflow that executes arbitrary shell by default,
- donor frontend code imported as product UI,
- debug command presented as product feature,
- state machine with no TUI progress projection.
