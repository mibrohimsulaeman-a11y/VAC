# VAC runtime architecture

## Runtime role

Rust owns execution. The runtime turns declarations into safe, observable behavior.

Runtime responsibilities:

- load `.vac` manifests,
- validate schemas,
- resolve capability handlers,
- enforce policy,
- request approval,
- execute typed workflow steps,
- emit lifecycle events,
- update TUI projections,
- produce validation results.

## Runtime pipeline

```text
Registry loader
  -> typed registry snapshot
  -> capability resolver
  -> workflow planner
  -> policy evaluator
  -> approval coordinator
  -> step executor
  -> event bus
  -> TUI projection
```

## Capability handler boundary

A capability handler is Rust code that implements a declared capability action.

Examples:

```text
capability.build.cargo_check
capability.identity.check
capability.approval.request
capability.activity.emit
capability.workflow.validate
capability.patch.apply
```

Handlers must be registered explicitly. A workflow may not call random functions by name.

## Workflow execution state

Every workflow run has:

- run id,
- workflow id,
- input snapshot,
- step list,
- current step,
- lifecycle state,
- policy decisions,
- approval decisions,
- output summary,
- validation result,
- error/recovery hint.

## Event model

Runtime emits structured events:

```text
workflow.started
workflow.step.started
workflow.step.waiting_approval
workflow.step.completed
workflow.step.failed
workflow.cancelled
workflow.completed
capability.started
capability.completed
policy.denied
approval.requested
approval.accepted
approval.rejected
validation.completed
```

String logs are not a substitute for structured events.

## Failure model

Every failure must include:

- failing manifest or runtime component,
- field path or step id when applicable,
- operator-safe message,
- recovery hint,
- whether retry is safe.

## Runtime anti-patterns

- hidden execution outside workflow runner,
- mutating step without policy check,
- approval implemented inside feature-specific code,
- workflow state only visible in logs,
- direct CLI subcommand bypassing capability registry,
- feature handler not represented in `.vac/capabilities`.
