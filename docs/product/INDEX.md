# VAC product documentation index

This directory is the root product documentation set for VAC.

It rebuilds the useful product requirements from the donor documentation into the current VAC-native architecture:

```text
.vac/                  declarative control plane
vac-rs/                Rust runtime, TUI, policy, execution
vac-cli/               `vac` launcher
donor/vac/             source-only reference material
```

## Product docs

- [VAC capability map](CAPABILITY_MAP.md)
- [Capability PRD coverage matrix](CAPABILITY_PRD_COVERAGE.md)
- [Master PRD](MASTER_PRD.md)
- [Product requirements matrix](requirements-matrix.md)
- [Product roadmap](roadmap.md)
- [Source material digest](source-material-digest.md)

## Domain PRDs

- [CLI and TUI](domain-prds/cli-and-tui.md)
- [Autonomous semantic coding](domain-prds/autonomous-semantic-coding.md)
- [Workflow control plane](domain-prds/workflow-control-plane.md)
- [Approvals, policy, and governance](domain-prds/approvals-policy-governance.md)
- [Runtime, scheduler, and sessions](domain-prds/runtime-scheduler-sessions.md)
- [Tools, MCP, and sandbox](domain-prds/tools-mcp-sandbox.md)
- [Agent orchestration](domain-prds/agent-orchestration.md)
- [VIL and VWFD native tooling](domain-prds/vil-vwfd-native.md)
- [VIL Native and Knowledge Add-on](domain-prds/vil-native-knowledge-add-on.md)
- [Observability, privacy, and release operations](domain-prds/observability-privacy-release.md)
- [Remote Bridge and Deferred Enterprise Runtime](domain-prds/remote-bridge-deferred.md)
- [Scheduler, Hooks, Monitor, and Autopilot Triggers](domain-prds/scheduler-hooks-monitor.md)
- [VIL Validation Passes and Semantic IR](domain-prds/vil-validation-passes.md)
- [Local Inference and Model Routing](domain-prds/local-inference-and-model-routing.md)
- [TUI Action Registry, Recorder, and Replay](domain-prds/tui-action-recorder-replay.md)
- [Import, Export, Restore, and Migration](domain-prds/import-export-restore.md)
- [Trace, Signal, Trajectory, and Why](domain-prds/trace-signal-trajectory.md)
- [Context, RAG, Ingest, and Memory](domain-prds/context-rag-memory.md)
- [Onboarding, Doctor, and Readiness](domain-prds/onboarding-doctor-readiness.md)
- [Changeset, Diff, and Evidence](domain-prds/changeset-diff-evidence.md)

## Relationship to architecture docs

Product docs define what VAC should deliver. Architecture docs define how VAC is structured.

```text
docs/product/       product requirements
docs/architecture/  architecture contract
docs/workflow-control-plane/ implementation plans
docs/donor-migration/ donor intake rules
```
