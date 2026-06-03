# ADR-0006 — VIL support uses native core plus managed knowledge add-on

## Status

Accepted for product planning.

## Context

VAC should support users who code in normal repositories and users who work with VIL/VWFD projects. VIL support must be optional so non-VIL users are not burdened, but it must be first-class enough that VIL users feel VAC understands their domain.

A purely native implementation would make deterministic validation strong, but would make knowledge updates, team-specific conventions, and private examples hard to evolve.

A purely add-on implementation would make VAC weaker as a VIL product because basic parse/validate/workbench behavior would depend on external connection status.

## Decision

VAC will implement VIL with two layers:

```text
1. VIL Native Core
   Built-in optional deterministic parser/validator/workbench capability.

2. VIL Knowledge Add-on
   One-click managed connector for docs, rules, examples, templates, review rubrics, and team conventions.
```

The add-on should feel like a product connector. The user should not need to install or operate a local server manually.

## Consequences

### Positive

- Non-VIL repos remain unaffected.
- VIL repos get reliable native validation and TUI workbench behavior.
- Team knowledge can evolve without binary releases.
- Agent can retrieve VIL guidance during autonomous coding.
- Failure of the knowledge add-on does not break native validation.

### Negative

- Two capability surfaces must be modeled clearly.
- Connector auth/status/error UX must exist.
- Evidence summaries must distinguish native validation from retrieved knowledge.

### Neutral

- The add-on may use an MCP-compatible managed connector internally, but product UX should call it VIL Knowledge Add-on or VIL Connector.

## Safety constraints

- Add-on is read-only by default.
- Add-on must not directly mutate files.
- Any mutating repair or generation must route through native VAC policy and approval.
- Credentials must be scoped and stored securely.

## Revisit criteria

Revisit this decision if:

- native VIL validation becomes too large for the core binary,
- managed add-on latency makes coding UX poor,
- team-specific rules require local-only/offline operation,
- VIL support becomes mandatory for all VAC users.
