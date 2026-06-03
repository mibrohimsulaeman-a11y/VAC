# PRD — Tools, MCP, and sandbox

## Overview

VAC tools are capability handlers that may read, write, execute, search, fetch, or validate. Every tool must be classified by trust, risk, policy, and surface visibility.

## Tool metadata requirements

Each tool must declare:

- name,
- description,
- input schema,
- risk level,
- trust requirement,
- side-effect class,
- policy requirement,
- approval behavior,
- output visibility.

## Risk levels

```text
read_only
safe_edit
destructive
network
execute
```

## Tool dispatch requirements

Tool dispatch must:

1. resolve tool metadata,
2. validate input schema,
3. check caller trust,
4. evaluate policy,
5. request approval when required,
6. execute in sandbox when needed,
7. emit structured result events.


## Managed knowledge add-ons

Some external knowledge sources should be exposed as product add-ons rather than manual server setup.

The first planned example is VIL Knowledge Add-on:

- one-click connection from TUI,
- agent tool-calling to retrieve docs/rules/examples,
- read-only by default,
- no direct file mutation,
- connector status visible in `/capabilities` and `/vil`,
- mutating suggestions routed through native VAC policy and approval.

Advanced implementation may use an MCP-compatible connector internally, but product UX should present it as an add-on/connector.

## External tool providers

External tool providers may be supported only when represented as capabilities and policy-gated.

External provider state should be visible in TUI:

- configured,
- reachable,
- trust level,
- exposed tools,
- policy status,
- last error.

## Sandbox requirements

Sandbox policy should classify:

- allowed filesystem roots,
- writable paths,
- allowed environment variables,
- network posture,
- shell/process behavior,
- container/host execution mode when supported.

## TUI requirements

The TUI should show:

- tool registry,
- enabled/disabled state,
- trust/risk level,
- approval requirements,
- current tool call progress,
- result summary,
- sandbox/policy status.

## Acceptance criteria

- Tools cannot execute without policy classification.
- External tools cannot bypass approval.
- Sandbox denial is visible with reason.
- Tool results appear in activity or tool output surface.
