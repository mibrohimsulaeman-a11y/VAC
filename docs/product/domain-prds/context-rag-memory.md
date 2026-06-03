# PRD — Context, RAG, Ingest, and Memory

## Overview

VAC needs a context pipeline so it can work in large repositories without forcing the user to explain the project from scratch every time.

This PRD covers:

```text
vac.ingest
vac.search_rank
vac.context_engine
vac.rag
vac.working_memory
vac.episodic_memory
vac.semantic_memory
vac.team_memory
```

## Product goal

VAC should retrieve the right project context, remember useful local decisions, and remain privacy-aware.

The user should feel:

```text
VAC understands the repo and recent work without reading everything blindly.
```

## User problems

- Large repos exceed model context.
- Users repeat architectural context in every prompt.
- Agent edits the wrong files because it lacks repo orientation.
- Search results are noisy.
- Memory can become unsafe if secrets or stale facts are stored.

## Capability model

| Capability | Purpose | Priority |
|---|---|---:|
| `vac.ingest` | Index project files, metadata, and change state. | P1 |
| `vac.search_rank` | Rank files/chunks by task relevance. | P1 |
| `vac.context_engine` | Build task context from chunks, history, and constraints. | P1/P2 |
| `vac.rag` | Embedding/vector retrieval and similarity search. | P2 |
| `vac.working_memory` | Short-lived task memory. | P1/P2 |
| `vac.episodic_memory` | Session/task history memory. | P2 |
| `vac.semantic_memory` | Reusable project knowledge with attribution. | P2 |
| `vac.team_memory` | Shared/team-scoped knowledge with stronger governance. | P2/P3 |

## User flows

### Flow 1 — Normal coding request

```text
User: Fix failing tests in auth module.
VAC:
  1. ingests current repo state or uses fresh index
  2. ranks likely files and tests
  3. builds bounded context
  4. runs semantic coding loop
```

### Flow 2 — Ask why or where

```text
User: Where is auth token validation implemented?
VAC:
  1. searches indexed context
  2. returns ranked files
  3. explains confidence and source paths
```

### Flow 3 — Remember local decision

```text
User: For this repo, prefer adapter pattern for provider integrations.
VAC:
  1. classifies memory scope
  2. checks privacy policy
  3. stores attributed project memory if allowed
```

## Ingest requirements

Project ingest should detect:

- repo root,
- language/framework signals,
- build/test files,
- ignored paths,
- config files,
- recent changes,
- VIL/VWFD markers when present,
- size and indexing constraints.

Ingest must respect ignore rules and policy-denied paths.

## Search/ranking requirements

Search/ranking should support:

- file/path search,
- symbol-like search when available,
- text search,
- recent change boost,
- task target boost,
- validation/test file boost,
- domain marker boost.

TUI should show ranked context source paths when relevant.

## Context engine requirements

Context engine should produce:

```yaml
context_bundle:
  task_id: task_123
  sources:
    - path: crates/auth/src/error.rs
      reason: likely target file
      confidence: high
  constraints:
    max_tokens: 12000
  redaction_status: clean
```

It must track why each source was selected.

## RAG requirements

RAG is optional and should not block MVP.

When enabled, it must define:

- embedding provider,
- index storage path,
- freshness policy,
- redaction policy,
- rebuild trigger,
- source attribution,
- failure fallback to lexical search.

## Memory tiers

### Working memory

Short-lived task facts. Cleared or summarized after task.

### Episodic memory

Session and task history. Useful for resume and repeated work.

### Semantic memory

Reusable project facts. Must be attributed and editable/removable.

### Team memory

Shared rules or conventions. Requires trust zones, connector policy, and stronger governance.

## Safety requirements

- Do not store secret-like values in memory.
- Do not send sensitive context externally without policy evaluation.
- Memory entries must have source and scope.
- Stale memories must be visible and replaceable.
- Team memory must not silently override local project truth.

## TUI surfaces

```text
/context
/memory
/status
/capabilities
```

TUI should show:

- context sources used,
- index freshness,
- memory scope,
- redaction status,
- disabled/degraded indexing state,
- recovery hints.

## Acceptance criteria

### MVP

- VAC can build a task context bundle from repo search.
- Context sources are attributable.
- Ignore/policy-denied paths are respected.
- TUI can show context readiness or degradation.

### Safety

- Secrets are not stored as memory.
- External sends are policy-classified.
- Memory storage has scope and attribution.

### UX

- User sees why files were selected.
- User can tell whether index/context is stale.
- User can disable or clear memory scope when supported.
