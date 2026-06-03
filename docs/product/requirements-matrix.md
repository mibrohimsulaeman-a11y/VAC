# VAC product requirements matrix

## Currentness note

This matrix is the product requirements layer, not the executable manifest inventory. As of the 2026-05-23 docs audit:

```text
root seed catalog and current `.vac` file count are separate; check `vac doctor registry .` for the live count
full CAPABILITY_MAP target: broader than current seed manifests
```

When a requirement moves from product intent to implementation, update the relevant source-of-truth in the same slice:

```text
docs/product/CAPABILITY_MAP.md
.vac/capabilities/*.yaml
.vac/surfaces/*.yaml
.vac/workflows/*.yaml
.vac/policies/*.yaml
vac-rs/core/src/control_plane/root_feature_catalog.rs
```

Do not mark a requirement product-ready from this matrix alone; product-ready requires manifest, owner, policy, surface, validation, and operator-visible evidence.

Scope boundary:

```text
VAC product repo requirements: strict manifest-driven control plane
User project first-run UX: zero-config local agent workspace, strict manifests optional
```


## Requirement categories

| ID | Category | Requirement | Initial priority |
|---|---|---|---|
| R-CLI-001 | CLI | `vac` launches the root TUI. | P0 |
| R-CLI-002 | CLI | Initial CLI surface is small and product-focused. | P0 |
| R-CLI-003 | CLI | Old service/proxy/cloud/debug surfaces do not appear in default help. | P0 |
| R-BLD-001 | Build | `cargo check -p vac-surface-cli` passes for default local development. | P0 |
| R-BLD-002 | Build | Missing optional native/vendor sandbox inputs surface as readiness issues when feasible, not default build blockers. | P0 |
| R-LRT-001 | Local runtime | The root `vac` TUI path and `vac exec` use Local Runtime Contract for local prompt submission. | P0 |
| R-LRT-002 | Local runtime | Runtime emits product-level events for task, tool, approval, validation, completion, and failure. | P0 |
| R-LRT-003 | Local runtime | Default local path does not require old app-server client/protocol stack. | P0 |
| R-TUI-001 | TUI | TUI shows input, activity, status, and approval state. | P0 |
| R-TUI-002 | TUI | TUI includes capability dashboard. | P1 |
| R-TUI-003 | TUI | TUI includes workflow browser. | P1 |
| R-TUI-004 | TUI | Workflow progress is visible step-by-step. | P1 |
| R-TUI-005 | TUI | Only one product TUI path exists. | P0 |
| R-TUI-006 | TUI | Donor shell/frontend stack is never wired as product UI. | P0 |
| R-CTL-001 | Control plane | `.vac/capabilities` declares product capabilities. | P1 |
| R-CTL-002 | Control plane | `.vac/workflows` declares typed workflows. | P1 |
| R-CTL-003 | Control plane | `.vac/policies` declares safety policy. | P1 |
| R-CTL-004 | Control plane | `.vac/surfaces` declares TUI/slash/palette/CLI exposure. | P1 |
| R-RT-001 | Runtime | Rust loads and validates manifests after local runtime path is clean. | P1 |
| R-RT-002 | Runtime | Rust executes only typed workflow steps initially. | P1 |
| R-RT-003 | Runtime | Runtime emits structured lifecycle events. | P1 |
| R-APP-001 | Approval | One approval model gates mutating work. | P0 |
| R-APP-002 | Approval | Approval requests show risk, reason, preview, and affected resources. | P0/P1 |
| R-SEC-001 | Security | Secret-like values are redacted before unsafe exposure. | P0/P1 |
| R-SEC-002 | Security | Filesystem/network/tool mutation is policy-classified. | P0/P1 |
| R-SEC-003 | Security | Trust zones classify local project, sensitive data, connectors, model providers, and untrusted output. | P0/P1 |
| R-SES-001 | Sessions | Sessions can be resumed or inspected when supported. | P1 |
| R-SES-002 | Sessions | Changes and approvals are associated with session lifecycle. | P0/P1 |
| R-SES-003 | Sessions | Session engine records prompt, task, approval, validation, checkpoint, and evidence summaries. | P0/P1 |
| R-SES-004 | Sessions | Pending approvals can survive TUI refresh/resume. | P1 |
| R-TOOLS-001 | Tools | Tools have trust/risk metadata. | P0/P1 |
| R-TOOLS-002 | Tools | Tools use formal result envelopes and render hints. | P0/P1 |
| R-TOOLS-003 | Tools | External/connector-backed tools are policy-gated. | P1/P2 |
| R-CONN-001 | Connectors | Managed connectors have lifecycle, ACL, config scope, and attribution. | P1 |
| R-CONN-002 | Connectors | Knowledge connectors are read-only by default. | P1 |
| R-CHG-001 | Changeset | Planned edits produce changeset preview when possible. | P0/P1 |
| R-CHG-002 | Changeset | Evidence links changed files to approvals and validations. | P0/P1 |
| R-CHG-003 | Changeset | Restore/checkpoint metadata is surfaced when available. | P1 |
| R-DOC-001 | Doctor | Readiness/doctor shows provider, sandbox, runtime, connector, policy, and manifest readiness. | P0/P1 |
| R-DOC-002 | Doctor | Readiness failures include recovery hints. | P0/P1 |
| R-MEM-001 | Context | Project ingest/search/context pipeline is represented as capability before implementation. | P1/P2 |
| R-MEM-002 | Context | Memory/RAG storage respects redaction, attribution, and policy. | P2 |
| R-VIL-001 | VIL | VIL/VWFD validation is represented as declared capability. | P2 |
| R-VIL-002 | VIL | VIL/VWFD workbench surfaces diagnostics and explain/parity output. | P2 |
| R-VIL-003 | VIL | VIL IR/expression/validation passes are native optional capabilities. | P2 |
| R-OBS-001 | Observability | Activity/evidence/why surfaces explain what happened. | P1/P2 |
| R-OBS-002 | Observability | Trace/signal/trajectory capabilities are documented before donor migration. | P2 |
| R-REL-001 | Release | Release gate includes build, identity, TUI, workflow, policy, and local runtime checks. | P1 |
| R-HYG-001 | Hygiene | Unowned backend code is visible as broken/unowned or deleted. | P1 |
| R-HYG-002 | Hygiene | Donor-backed code is not product-ready until manifest-backed and TUI/CLI-visible. | P0 |
| R-HYG-003 | Hygiene | Old runtime/server/API crates are deleted or deferred after reachability drops. | P0/P1 |
| R-PTY-001 | Operator gate | Real PTY dogfood validates TUI input, help, prompt submit, approval/progress, and clean exit. | P0/P1 |

| R-WS-001 | Workspace | `vac` works in an existing repo when `.vac` is missing by using an in-memory profile or offering a soft workspace. | P0 |
| R-WS-002 | Workspace | Soft `.vac` workspace creation does not force strict capability/policy/workflow YAML migration. | P0 |
| R-WS-003 | Workspace | `.vac/.gitignore` or equivalent guard keeps DB, index, sessions, logs, cache, artifacts, and tmp local-only by default. | P0/P1 |
| R-WS-004 | Workspace | Promotion from soft profile to curated/strict manifests is explicit, user-approved, and doctor-visible. | P1 |

## Priority definition

| Priority | Meaning |
|---|---|
| P0 | Required before local product path can be trusted. |
| P1 | Required for workflow-control-plane MVP. |
| P2 | Required for differentiated advanced product value. |
| P3 | Later enhancement after core product is stable. |
