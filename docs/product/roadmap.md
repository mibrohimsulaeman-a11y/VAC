# VAC product roadmap

## Roadmap rule

VAC roadmap now follows ADR-0007.

The product must become local-first before `.vac` registry implementation and donor-backed capability work begin.

```text
local runtime path first
control plane second
donor-backed capabilities third
```

## Phase -1 — Build unblock

Goal: make the root local product buildable without restoring large vendored inputs by default.

Required outcomes:

- `cargo check -p vac-surface-cli` can progress without missing vendored native source for default local development,
- sandbox/vendor/native dependency failures become readiness/doctor issues when possible,
- websocket/API patch remains compile-compatible while legacy compatibility transport paths are being retired,
- no new product command surface is added,
- no donor code migration starts.

Exit gate:

```text
cargo check -p vac-surface-cli reaches product-local code or passes
sandbox readiness is represented as degraded/unavailable instead of hard build failure where feasible
```

## Phase 0 — Local Runtime Contract and legacy transport retirement

Goal: make the root `vac` TUI path and `vac exec` use a local product runtime contract instead of an app-server/protocol-shaped local path.

Required outcomes:

- Local Runtime Contract DTO/event model exists,
- `vac exec` submits tasks through Local Runtime Contract,
- the root `vac` TUI path consumes RuntimeEvent stream for prompt/activity/approval/validation/completion,
- approval request/resolve uses local ApprovalRequest,
- session engine basics can persist prompt/task/approval/validation/evidence records,
- old app-server/API/runtime crates become unreachable from the default local product path or are explicitly deferred,
- reachability audit proves what can be deleted/quarantined.

Exit gate:

```text
cargo check -p vac-surface-cli passes
vac exec no longer depends on app-server client for local prompt submission
the root `vac` TUI path no longer depends on app-server client for local prompt submission
legacy server/API compatibility crates are deleted or classified as deferred capability
```

## Phase 0.5 — Zero-config project workspace

Goal: make `vac` useful in existing user projects before strict `.vac` manifests exist.

Target:

- missing `.vac` does not block ordinary coding assistance,
- VAC can run with an in-memory profile or offer a soft `.vac` workspace,
- soft workspace includes profile, local DB/index/cache boundaries, memory/session/artifact/log locations, and inferred validation hints,
- strict manifests remain an opt-in promotion path, not a first-run requirement.

Validation signal:

```text
user can run `vac` in an arbitrary repo, see inferred project profile/risk/validation hints, and start coding without learning YAML first.
```

## Phase 1 — Control plane skeleton

Goal: make `.vac` the visible product control plane only after the local runtime path is clean.

Required outcomes:

- `.vac/capabilities`, `.vac/workflows`, `.vac/policies`, `.vac/surfaces`, `.vac/registry`,
- root product registry metadata,
- initial manifests for current root features,
- manifest schema decisions recorded,
- Local Runtime Contract event/state ids reflected in manifests where relevant.

## Phase 2 — Registry and diagnostics

Goal: Rust can load and validate the control plane.

Required outcomes:

- capability registry loader,
- workflow registry loader,
- policy registry loader,
- surface registry loader,
- structured diagnostics,
- CLI/TUI-safe error reporting,
- invalid manifests visible in TUI/doctor readiness.

## Phase 3 — TUI visibility

Goal: operator can see control-plane state.

Required outcomes:

- `/capabilities` dashboard,
- `/workflow` browser,
- manifest errors visible in TUI,
- planned/partial/broken status visible,
- readiness/doctor can explain missing runtime/capability dependencies.

## Phase 4 — Safe workflow runner

Goal: typed workflows can execute without arbitrary scripting.

Required outcomes:

- built-in step registry,
- lifecycle state machine,
- progress projection through RuntimeEvent or workflow events,
- policy checks,
- approval integration,
- maintenance workflows for build, identity, and local runtime readiness.

## Phase 5 — Root feature conversion

Goal: existing root product behavior is manifest-governed.

Required outcomes:

- chat capability manifest,
- semantic loop capability manifest,
- approval capability manifest,
- tools capability manifest,
- sandbox/readiness capability manifest,
- sessions capability manifest,
- changeset/evidence capability manifest,
- workflow capability manifest,
- release gate workflow.

## Phase 6 — Donor-backed differentiated capabilities

Goal: bring in unique domain value only after control plane can surface it.

Initial candidates:

- VIL/VWFD validation and workbench,
- workflow runner/progress semantics,
- manifest/profile cockpit,
- context/RAG/memory readiness,
- trace/signal/trajectory/evidence surfaces,
- richer approval/policy model,
- managed connector add-ons.

## Phase 7 — Enterprise hardening

Goal: make VAC reliable for team/operator use.

Required outcomes:

- release gate workflow,
- PTY operator gate,
- dead-code and ownership checks,
- security and privacy workflows,
- audit/evidence model,
- packaging and install validation,
- remote/bridge capabilities only after local-first product remains clean.
