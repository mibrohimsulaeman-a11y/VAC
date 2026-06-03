# VAC capability map

## Purpose

This document maps the full intended VAC capability set after reviewing root product docs, root architecture docs, workflow-control-plane plans, and donor backend source material.

It is the product-level capability map. It does not authorize direct donor code migration. Every capability still needs a `.vac/capabilities/*.yaml` declaration, policy classification, TUI/CLI surface, validation, and implementation plan before it can be considered product-ready.

## Scope note — product capabilities vs user project workspace

For the VAC product repository, product-ready capabilities still require strict `.vac` manifests, policy, surfaces, validation, and implementation ownership. For arbitrary user projects, zero-config `.vac` workspace bootstrap is different: VAC can operate from an in-memory or soft project profile, and strict manifests are optional promotion artifacts.


## Current executable seed manifest note

### 2026-05-23 surface/registry route ownership sync

`vac.tui` is the canonical owner for root TUI cockpit routes: `/`, `/capabilities`, `/workflow`, `/debug-config`, `/activity`, `/approvals`, and `/evidence`. Feature capabilities keep their CLI, slash, and palette surfaces, while the root TUI catalog observes those workflows through the single `vac.tui` operator surface. This avoids route-key duplication while keeping capability/workflow visibility intact.

Historical migration and donor-quarantine documents may still quote retired identifiers as evidence. Those files are explicitly exempted in the identity scanner rather than treated as live product vocabulary; active manifests and surfaces remain the source of truth for current ownership.

Workflow doctor diagnostics now treat `waiting_approval` as a safe paused state, not a failed gate. Hard failures remain explicit blocked steps, failed checks, or cancellations.

This map is broader than the current executable `.vac` seed set. As of the 2026-05-23 docs audit, current seed manifests are:

```text
vac.approvals
vac.architecture
vac.build
vac.chat
vac.donor_migration
vac.identity
vac.identity.check
vac.local_runtime_owner
vac.ownership
vac.release
vac.runtime_approval_bridge
vac.sandbox
vac.sessions
vac.tools
vac.tui
vac.tui.pty_gate
vac.tui_session_runtime
vac.workflow
```

Naming distinction:

```text
CAPABILITY_MAP ids = product-level target taxonomy
.vac/capabilities/*.yaml ids = executable seed/current manifest taxonomy
```

Known intentional taxonomy differences:

Newly promoted local-runtime seed manifests are mapped to existing PRD families and architecture contracts instead of creating one-off product-doc silos:

| Current executable seed | PRD family | Source docs |
|---|---|---|
| `vac.local_runtime_owner` | Session engine | `domain-prds/runtime-scheduler-sessions.md`, `../architecture/local-runtime-contract.md` |
| `vac.tui_session_runtime` | Session engine | `domain-prds/runtime-scheduler-sessions.md`, `../architecture/local-runtime-contract.md` |
| `vac.runtime_approval_bridge` | Workflow control plane | `domain-prds/runtime-scheduler-sessions.md`, `../architecture/local-runtime-contract.md` |

| Product-level target | Current executable seed | Note |
|---|---|---|
| `vac.approval` | `vac.approvals` | seed manifest covers approval family; do not rename without schema/manifest migration |
| `vac.session` | `vac.sessions` | seed manifest covers session resume/inspection family |
| `vac.capabilities` | `vac.tui` + `vac.workflow` surfaces | dashboard is represented through TUI/control-plane surfaces, not a separate seed manifest yet |
| `vac.doctor` | `vac.workflow`, `vac.build`, `vac.release`, `vac.donor_migration`, etc. | doctor routes are spread across capability-specific gates |

Seed ids that are operational/control-plane focused and not listed as product target rows here:

```text
vac.architecture
vac.build
vac.donor_migration
vac.identity
vac.identity.check
vac.ownership
vac.release
vac.tui
vac.tui.pty_gate
vac.workflow
```

Do not treat a missing one-to-one match as stale by itself. It becomes stale only if a doc claims a product target is execution-ready without a matching manifest/surface/policy/validation path, or if an existing seed manifest changes without updating this note and the relevant PRD/plan.

## Product framing

VAC must remain simple for normal users:

```text
vac
> fix this bug
```

Internally, VAC should provide workflow-native safety and explainability:

```text
semantic loop
  -> capability registry
  -> policy and approval
  -> bounded execution
  -> validation
  -> evidence
  -> session continuity
```

Advanced systems such as VIL-native tooling, connectors, RAG, multi-agent orchestration, scheduler hooks, and remote bridge capabilities must be optional or staged. They must not clutter the default coding experience.

## Capability tiers

```text
Tier 0: default coding agent UX
Tier 1: workflow-native safety and operator visibility
Tier 2: domain-native and advanced autonomy
Tier 3: enterprise/remote/daemon-style capabilities
Quarantine: donor frontend/runtime paths that must not become product routes
```

## Tier 0 — default coding agent UX

These capabilities make VAC usable as a normal terminal coding agent without requiring users to understand workflow architecture.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Chat/input composer | `vac.chat` | Accept plain-language coding requests. | P0 |
| Semantic coding loop | `vac.semantic_loop` | Intent, orient, plan, classify, approve, execute, validate, diagnose, retry, summarize. | P0 |
| Repo orientation | `vac.repo_orientation` | Read-only project scan, file discovery, build/test signal detection. | P0 |
| Semantic planning | `vac.semantic_plan` | Operator-readable plan before mutation. | P0 |
| Bounded patch execution | `vac.patch` | Apply edits inside explicit bounds. | P0 |
| Targeted validation | `vac.validation` | Run relevant checks after changes. | P0 |
| Evidence summary | `vac.evidence` | Explain changed files, validations, approvals, and remaining risks. | P0 |
| Session resume basics | `vac.session` | Resume or inspect interrupted work. | P0/P1 |

### Zero-config project workspace

| Capability | Capability id | Description | Priority |
|---|---|---|---|
| Project workspace bootstrap | `vac.project_workspace` | Auto-create or infer a soft `.vac` local agent workspace for existing projects without forcing strict YAML/control-plane migration. | P0/P1 |
| Project profile | `vac.project_profile` | Store inferred stack, validation commands, risk hints, and adoption mode (`in_memory`, `soft`, `curated`, `strict`). | P0/P1 |

## Tier 1 — workflow-native safety and operator visibility

These capabilities make VAC safer and more inspectable than prompt-only coding agents.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Capability dashboard | `vac.capabilities` | Show every product capability, owner, status, policy, surface, validation. | P0 |
| Workflow browser | `vac.workflow_browser` | List typed workflows and their steps, inputs, policy, validation. | P0/P1 |
| Workflow runner | `vac.workflow_runner` | Execute typed workflows with lifecycle events and policy checks. | P1 |
| Workflow progress | `vac.workflow_progress` | TUI progress projection for pending/running/approval/success/failure/cancelled states. | P1 |
| Runtime approval bridge | `vac.runtime_approval_bridge` | Bridge local runtime requests into the root operator approval model and workflow-control-plane lifecycle. | P1 |
| Policy evaluator | `vac.policy` | Classify allowed, denied, approval-required, unavailable actions. | P0 |
| Approval coordinator | `vac.approval` | One approval model for mutating/risky actions. | P0 |
| Sandbox/isolation | `vac.sandbox` | File/process/network execution posture and enforcement. | P0/P1 |
| Trust zones | `vac.trust_zones` | Permission and trust boundaries for tools, agents, connectors, and data. | P0/P1 |
| Redaction engine | `vac.redaction` | Prevent raw secret exposure in logs, TUI, connector calls, and evidence. | P0/P1 |
| Tool contract | `vac.tool_contract` | Formal tool spec, permission class, result envelope, render hints. | P0 |
| Tool registry | `vac.tools` | Register and dispatch tool calls through policy and approval. | P0/P1 |
| Doctor/readiness | `vac.doctor` | Detect missing provider/model/sandbox/manifest/tool readiness and show repair hints. | P0/P1 |

## Tier 1.5 — session, transcript, and durable execution

Donor backend review shows that durable session semantics are more important than initially documented.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Session engine | `vac.session_engine` | Submit-message lifecycle, transcript durability, slash processing, gates, usage tracking. | P0 |
| Local runtime owner | `vac.local_runtime_owner` | Own the local runtime bootstrap, retained resources, and session boundary without app-server ownership leakage. | P0/P1 |
| TUI session runtime | `vac.tui_session_runtime` | Route root TUI session lifecycle through the local runtime path while preserving operator session semantics. | P1 |
| Session control | `vac.session_control` | Session snapshots, TUI-state snapshots, checkpoint dirs, stale cleanup, resume candidates. | P0/P1 |
| Transcript model | `vac.transcript` | Durable event rows for messages, tool calls, approvals, compaction boundaries. | P0/P1 |
| Compaction boundary | `vac.compaction` | Context compaction state and explanation. | P1 |
| Checkpoints | `vac.checkpoint` | Restore points before risky mutation. | P1 |
| Usage tracking | `vac.usage_tracking` | Token/cost/tool usage summary when available. | P1 |
| Resume ranking | `vac.resume_candidates` | Rank sessions suitable for continuation. | P1 |

## Tier 1.5 — changeset, diff, and evidence

Donor backend review shows that VAC should have semantic diff and changeset capabilities beyond generic patch apply.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Semantic changeset | `vac.changeset` | Structured before/after file state, changed entries, and repo navigation. | P0/P1 |
| Semantic diff | `vac.semantic_diff` | File diff enriched with reasons, risk, workflow links, and domain deltas. | P1 |
| VWFD-aware diff | `vac.vwfd_diff` | Detect workflow step/expression/handler deltas in VWFD projects. | P2 |
| Repo navigator | `vac.repo_navigator` | File tree and changed-file navigation for review/evidence. | P1 |
| Patch preview | `vac.patch_preview` | TUI preview before applying changes. | P0/P1 |
| Backups and restore | `vac.backups` | Backup modified files and restore specific changes when supported. | P1 |

## Tier 1.5 — managed connectors and add-ons

The VIL Knowledge Add-on is one example of a larger managed connector capability.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Managed connectors | `vac.connectors` | One-click connector/add-on UX for knowledge and external context. | P1 |
| Connector lifecycle | `vac.connector_lifecycle` | Disconnected, connecting, connected, degraded, auth-expired, unavailable. | P1 |
| Connector ACL | `vac.connector_acl` | Channel/tool/resource access control for connectors. | P1 |
| Connector config scopes | `vac.connector_config` | Project, user, team, and environment-specific connector settings. | P1 |
| Elicitation | `vac.elicitation` | Connector-driven request for operator input with policy visibility. | P2 |
| VIL Knowledge Add-on | `vac.vil_knowledge` | Managed connector for VIL docs, rules, examples, rubrics, and team conventions. | P2 |

## Tier 1.5 — context, RAG, ingest, and memory

Donor backend review shows this should be treated as a pipeline, not isolated features.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Project ingest | `vac.ingest` | Root detection, file indexing, ignore rules, pending change snapshots. | P1 |
| Search/ranking | `vac.search_rank` | BM25 or similar ranking for relevant files and paths. | P1 |
| Context engine | `vac.context_engine` | Semantic chunking, context window, attention routing. | P1/P2 |
| RAG index | `vac.rag` | Embeddings, vector/similarity search, index lifecycle. | P2 |
| Working memory | `vac.working_memory` | Short-lived task memory. | P1/P2 |
| Episodic memory | `vac.episodic_memory` | Session/task history and decision context. | P2 |
| Semantic memory | `vac.semantic_memory` | Persistent reusable knowledge with attribution and policy. | P2 |
| Team memory | `vac.team_memory` | Shared memory/rules with stronger privacy and governance controls. | P2/P3 |

## Tier 2 — VIL-native domain capabilities

VIL support is optional but first-class when a repository uses VIL/VWFD.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| VIL native detection | `vac.vil_detect` | Detect VIL/VWFD repo markers and enable domain surfaces. | P1/P2 |
| VIL expression engine | `vac.vil_expr` | Expression AST, parser, symbol table, validation reports. | P2 |
| VWFD workbench | `vac.vwfd_workbench` | Inspect/explain workflow tree, triggers, handlers, conditions. | P2 |
| VIL semantic IR | `vac.vil_ir` | AST pipeline, type resolution, borrow/lifetime analysis, refactor safety. | P2 |
| VIL validation passes | `vac.vil_validation_passes` | Multi-pass validation, VIL Way compliance, parity pass, severity model. | P2 |
| VWFD parity | `vac.vwfd_parity` | Check declared workflow handlers against implementation. | P2 |
| VIL repair preview | `vac.vil_repair_preview` | Preview suggested fixes before approval-gated apply. | P2 |
| VIL generation preview | `vac.vil_generation_preview` | Preview generated handlers/artifacts before mutation. | P2 |

## Tier 2 — model routing and inference

Model/provider capability should be productized as readiness, routing, and budget control.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Model router | `vac.model_router` | Select provider/model based on task, readiness, policy, and budget. | P1/P2 |
| Provider readiness | `vac.provider_readiness` | Show provider status, missing credentials, and model availability. | P1 |
| Streaming output | `vac.streaming` | Stream model/tool progress into TUI activity. | P1 |
| Token budget | `vac.token_budget` | Track and limit task/session token usage when available. | P1/P2 |
| Provider fallback | `vac.provider_fallback` | Fallback chain when a provider is unavailable or unsuitable. | P2 |
| Local inference | `vac.local_inference` | Optional local model inference for constrained/offline tasks. | P2/P3 |

## Tier 2 — agent orchestration

Advanced autonomy should be staged after the semantic coding loop, approval, policy, and session engine are stable.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Agent orchestration | `vac.agent_orchestration` | Multi-role planning/execution/review under one workflow lifecycle. | P2 |
| Tri-lane orchestration | `vac.tri_lane` | Specialist lanes/roles for complex tasks. | P2/P3 |
| Subagent policy | `vac.subagent_policy` | Bound nested agents by depth, approval, tool, and trust limits. | P2 |
| Agent checkpoints | `vac.agent_checkpoints` | Checkpoint agent state before risky work or retries. | P2 |
| Context budget | `vac.context_budget` | Bound subagent/context growth. | P2 |
| Reasoning state machine | `vac.reasoning_fsm` | Make agent loop state observable and stoppable. | P2/P3 |

## Tier 2 — trace, signal, trajectory, and why

These capabilities turn autonomous work into inspectable evidence.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Signal buffers | `vac.signal` | Bounded output buffers, scoring, distillation, rewind store. | P1/P2 |
| Trace recording | `vac.trace` | Session and workflow records with redaction and export. | P1/P2 |
| Trajectory analysis | `vac.trajectory` | Explain decisions and score decision quality. | P2 |
| Why reports | `vac.why` | User-facing explanation of why actions were taken. | P1/P2 |
| Evidence export | `vac.evidence_export` | Export summaries/traces for review or handoff. | P2 |

## Tier 2 — import, export, restore, and migration

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Session import/export | `vac.import_export` | Move or review session/evidence data safely. | P2 |
| Restore | `vac.restore` | Restore files/session state from approved checkpoints/backups. | P1/P2 |
| Migration | `vac.migrate` | Migrate config/schema/session formats. | P2 |
| Update catalog | `vac.update_catalog` | Catalog product/runtime updates when supported. | P3 |
| Recovery | `vac.recovery` | Surface restore/retry/recovery actions in TUI. | P1 |

## Tier 2 — TUI action quality and replayability

These improve operator UX and make TUI behavior testable.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Action registry | `vac.action_registry` | Canonical action ids, keybindings, command palette metadata. | P1/P2 |
| Keybindings | `vac.keybindings` | Consistent keyboard contract and discoverability. | P1/P2 |
| Overlay contract | `vac.overlays` | Overlay stack, focus restore, render order, escape precedence. | P1/P2 |
| File picker | `vac.file_picker` | Fast project file selection for tasks/review. | P2 |
| Recorder | `vac.recorder` | Record TUI input sessions for debugging. | P2 |
| Replay | `vac.replay` | Replay TUI input traces for regression tests. | P2 |
| Recent commands | `vac.recent_commands` | Quick reuse of recent actions/prompts. | P2 |

## Tier 2 — metrics and production feedback

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Metrics | `vac.metrics` | Local system/task metrics with privacy controls. | P2 |
| Statusline | `vac.statusline` | Compact runtime/provider/sandbox/approval/readiness status. | P1/P2 |
| Rate limits | `vac.rate_limits` | Show and respect provider/tool rate limits. | P2 |
| Feedback | `vac.feedback` | Optional product feedback/crash evidence with privacy gates. | P3 |

## Tier 2/P3 — scheduler, hooks, monitor, and autopilot triggers

These are powerful but should not make the initial product feel daemon-heavy.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Scheduler | `vac.scheduler` | Queue jobs, run scheduled workflows, handle async work. | P2 |
| Cron | `vac.cron` | Time-based workflow triggers. | P2/P3 |
| Hooks | `vac.hooks` | Repo or lifecycle hooks with policy gates. | P2/P3 |
| Monitor | `vac.monitor` | Watch task/repo/process status. | P2/P3 |
| File watcher | `vac.file_watcher` | Trigger checks or workflows when files change. | P2/P3 |
| Autopilot triggers | `vac.autopilot_triggers` | Run bounded autonomous work from triggers. | P3 |

## Tier 3 — enterprise/remote bridge capabilities

These should be deferred until local workflow-native product is stable.

| Capability | Product id | Description | Priority |
|---|---|---|---|
| Bridge | `vac.bridge` | Remote session glue and protocol boundary. | P3 |
| Permission mediation | `vac.permission_mediation` | Mediate permissions across remote/client boundaries. | P3 |
| Remote sessions | `vac.remote_sessions` | Remote or delegated sessions. | P3 |
| ACP-compatible protocol | `vac.acp` | External client/server protocol compatibility when needed. | P3 |
| Teleport/session handoff | `vac.teleport` | Move session execution context when supported. | P3 |

## Quarantine — donor frontend/shell stack

The donor shell stack contains many useful concepts, but it must not become a product route because VAC must have one product TUI path.

Quarantine families:

```text
vac_shell_*
vac_tui_runtime
```

Allowed extraction:

- activity concepts,
- approval detail concepts,
- diff review concepts,
- model switcher concepts,
- session browser concepts,
- status bar concepts,
- event projection concepts,
- keymap/overlay/action concepts.

Forbidden migration:

- creating a second product TUI,
- importing donor shell runtime as active frontend,
- making donor shell command registry a second command source,
- duplicating approval UI outside root approval model.

## Priority summary

### P0 before major donor implementation

```text
semantic coding loop
capability dashboard
policy and approval
tool contract
session engine basics
managed connector core contract
changeset/evidence basics
trust zones and redaction
doctor/readiness
```

### P1 product foundation

```text
workflow runner/progress
transcript/checkpoints/resume
context ingest/search
patch preview/backups
provider readiness/token budget
trace/signal/why basics
restore/recovery
TUI action registry basics
```

### P2 differentiation

```text
VIL expression/IR/validation
VIL knowledge add-on
RAG and memory
agent orchestration
local inference
scheduler/hooks/file watcher
recorder/replay
metrics/statusline
```

### P3 deferred enterprise/remote

```text
bridge
remote sessions
permission mediation across remote boundaries
teleport/session handoff
update catalog
advanced feedback/telemetry
```

## Capability completion rule

A capability is not product-ready until it has:

- `.vac/capabilities/<id>.yaml`,
- owner crate/module,
- TUI or CLI surface,
- empty/loading/success/failure states,
- policy classification,
- approval behavior if mutating,
- validation command,
- evidence/output behavior,
- migration status if donor-backed.

Backend-only implementation does not count.
