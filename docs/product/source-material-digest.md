# Source material digest

## Donor corpus read

The root product docs were rebuilt after reviewing the donor documentation corpus.

Corpus summary:

```text
source root: donor/vac/docs
files reviewed: 94
approximate text size: 545K characters
```

## Donor material categories consumed

| Category | Product docs rebuilt from it |
|---|---|
| Product requirements | Master PRD and domain PRDs. |
| TUI requirements | CLI/TUI PRD and TUI operator surface architecture. |
| Workflow planning | Workflow control plane PRD and roadmap. |
| Runtime integration | Runtime/session/scheduler PRD and execution lifecycle. |
| Approval/security/privacy | Approval, policy, governance, security, and privacy docs. |
| Tool/MCP docs | Tools, MCP, sandbox, and external tool policy PRD. |
| VIL/VWFD docs | VIL/VWFD native tooling PRD. |
| Release/runbook docs | Release operations and observability PRD. |
| Wiring plans | Product hygiene, capability dashboard, and no-backend-only rules. |
| ADRs | Migration and architecture constraints. |

## Rewrite rules applied

The root docs are not copied verbatim. They are rewritten to match the current VAC product architecture:

- `.vac` is the declarative control plane.
- Rust executes typed capabilities and workflow steps.
- TUI is the operator surface.
- Donor code is source-only until manifest-backed.
- Old frontend/runtime paths are not product routes.
- Product docs avoid obsolete command surfaces unless they are explicitly planned.
- Domain features must have capability manifests before implementation.

## Material intentionally not preserved as root product requirements

Some donor material described older implementation details that are not product requirements in the new architecture:

- alternate frontend paths,
- standalone skill-pack directory model under `.vac`,
- service/proxy/debug commands as default product CLI surfaces,
- backend-only success criteria,
- implementation-specific crate names from donor source,
- old release packaging mechanics that conflict with workflow-native release gates.

These ideas may still be referenced during migration, but they are not root product truth.
