# VAC capability PRD coverage matrix

## Purpose

This matrix maps VAC capability families from `docs/product/CAPABILITY_MAP.md` to their PRD and architecture source-of-truth.

Individual `vac.*` capabilities will eventually be declared in `.vac/capabilities/*.yaml`. Product PRDs stay grouped by domain/family to avoid one document per small capability.

## Coverage matrix

| Capability family | Capability ids | Product PRD | Architecture contract |
|---|---|---|---|
| Default coding loop | `vac.chat`, `vac.semantic_loop`, `vac.repo_orientation`, `vac.semantic_plan`, `vac.patch`, `vac.validation`, `vac.evidence`, `vac.session` | `domain-prds/autonomous-semantic-coding.md` | `../architecture/semantic-coding-loop.md`, `../architecture/local-runtime-contract.md` |
| CLI/TUI | `vac.capabilities`, `vac.workflow_browser`, TUI input/status/approval surfaces | `domain-prds/cli-and-tui.md` | `../architecture/tui-operator-surface.md` |
| Workflow control plane | `vac.workflow_runner`, `vac.workflow_progress`, `vac.runtime_approval_bridge`, capability/workflow/policy/surface registry | `domain-prds/workflow-control-plane.md`, `domain-prds/runtime-scheduler-sessions.md` | `../architecture/control-plane.md`, `../architecture/workflow-model.md`, `../architecture/local-runtime-contract.md` |
| Approval/policy/governance | `vac.approval`, `vac.policy`, `vac.sandbox` | `domain-prds/approvals-policy-governance.md` | `../architecture/policy-and-approval.md` |
| Trust/redaction | `vac.trust_zones`, `vac.redaction` | `domain-prds/approvals-policy-governance.md`, `domain-prds/observability-privacy-release.md` | `../architecture/trust-zones-and-redaction.md` |
| Session engine | `vac.session_engine`, `vac.local_runtime_owner`, `vac.tui_session_runtime`, `vac.session_control`, `vac.transcript`, `vac.compaction`, `vac.checkpoint`, `vac.usage_tracking`, `vac.resume_candidates` | `domain-prds/runtime-scheduler-sessions.md` | `../architecture/session-engine.md`, `../architecture/local-runtime-contract.md` |
| Tool contract/tools | `vac.tool_contract`, `vac.tools` | `domain-prds/tools-mcp-sandbox.md` | `../architecture/tool-contract.md` |
| Managed connectors | `vac.connectors`, `vac.connector_lifecycle`, `vac.connector_acl`, `vac.connector_config`, `vac.elicitation`, `vac.vil_knowledge` | `domain-prds/vil-native-knowledge-add-on.md`, `domain-prds/tools-mcp-sandbox.md` | `../architecture/managed-connectors.md`, `../architecture/vil-native-and-knowledge-add-on.md` |
| Changeset/evidence | `vac.changeset`, `vac.semantic_diff`, `vac.repo_navigator`, `vac.patch_preview`, `vac.backups`, `vac.restore`, `vac.vwfd_diff` | `domain-prds/changeset-diff-evidence.md` | `../architecture/changeset-evidence.md` |
| Doctor/readiness | `vac.doctor`, `vac.provider_readiness`, readiness/status surfaces | `domain-prds/onboarding-doctor-readiness.md` | `../architecture/local-runtime-contract.md` |
| Context/RAG/memory | `vac.ingest`, `vac.search_rank`, `vac.context_engine`, `vac.rag`, `vac.working_memory`, `vac.episodic_memory`, `vac.semantic_memory`, `vac.team_memory` | `domain-prds/context-rag-memory.md` | `../architecture/trust-zones-and-redaction.md` |
| Model routing/inference | `vac.model_router`, `vac.provider_readiness`, `vac.streaming`, `vac.token_budget`, `vac.provider_fallback`, `vac.local_inference` | `domain-prds/local-inference-and-model-routing.md` | `../architecture/local-runtime-contract.md` |
| VIL native | `vac.vil_detect`, `vac.vil_expr`, `vac.vil_ir`, `vac.vil_validation_passes`, `vac.vwfd_parity`, `vac.vwfd_workbench`, `vac.vil_repair_preview`, `vac.vil_generation_preview` | `domain-prds/vil-vwfd-native.md`, `domain-prds/vil-validation-passes.md`, `domain-prds/vil-native-knowledge-add-on.md` | `../architecture/vil-native-and-knowledge-add-on.md` |
| Agent orchestration | `vac.agent_orchestration`, `vac.tri_lane`, `vac.subagent_policy`, `vac.agent_checkpoints`, `vac.context_budget`, `vac.reasoning_fsm` | `domain-prds/agent-orchestration.md` | `../architecture/semantic-coding-loop.md` |
| Trace/signal/trajectory | `vac.signal`, `vac.trace`, `vac.trajectory`, `vac.why`, `vac.evidence_export` | `domain-prds/trace-signal-trajectory.md`, `domain-prds/observability-privacy-release.md` | `../architecture/changeset-evidence.md`, `../architecture/trust-zones-and-redaction.md` |
| Import/export/restore | `vac.import_export`, `vac.restore`, `vac.migrate`, `vac.update_catalog`, `vac.backups`, `vac.recovery` | `domain-prds/import-export-restore.md` | `../architecture/changeset-evidence.md`, `../architecture/session-engine.md` |
| TUI action quality | `vac.action_registry`, `vac.keybindings`, `vac.overlays`, `vac.file_picker`, `vac.recorder`, `vac.replay`, `vac.recent_commands` | `domain-prds/tui-action-recorder-replay.md` | `../architecture/tui-operator-surface.md` |
| Metrics/statusline | `vac.metrics`, `vac.statusline`, `vac.rate_limits`, `vac.feedback` | `domain-prds/observability-privacy-release.md` | `../architecture/runtime.md` |
| Scheduler/hooks/monitor | `vac.scheduler`, `vac.cron`, `vac.hooks`, `vac.monitor`, `vac.file_watcher`, `vac.autopilot_triggers` | `domain-prds/scheduler-hooks-monitor.md`, `domain-prds/runtime-scheduler-sessions.md` | `../architecture/runtime.md` |
| Remote/bridge deferred | `vac.bridge`, `vac.remote_sessions`, `vac.permission_mediation`, `vac.acp`, `vac.teleport` | `domain-prds/remote-bridge-deferred.md` | `../architecture/decisions/ADR-0007-local-runtime-contract.md` |

## Coverage status

### 2026-05-23 route/surface validation note

The root TUI cockpit is represented by `vac.tui` for `/`, `/capabilities`, `/workflow`, `/debug-config`, `/activity`, `/approvals`, and `/evidence`. Capability-specific PRD rows still describe domain ownership, but the visible TUI route ownership is centralized to prevent duplicate route IDs and false-green registry diagnostics.

Workflow PRD coverage treats operator approval waits as safe paused lifecycle states; `vac doctor workflow` should fail only on explicit blocked/failed/cancelled workflow diagnostics.

```text
strategy-level product docs: covered
capability-family PRDs: covered
P0 backend contracts: covered
root seed catalog and current `.vac` file count are separate; check `vac doctor registry .` for the live count
full product capability map: not fully materialized as individual manifests yet
```

Current `.vac/capabilities/*.yaml` seed set:

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

## Next coverage step

Promote from seed coverage to full product coverage gradually:

1. Keep existing `.vac/capabilities/*.yaml` manifests synchronized with status, owner, policy, surface, and validation evidence.
2. Add new manifests only when a capability has a clear implementation path, product surface, and validation gate.
3. For capability families in `CAPABILITY_MAP.md` that do not yet have individual manifests, treat the PRD row as strategy coverage, not execution-ready manifest coverage.
