# VAC workflow control plane implementation plan

This plan changes the root repository pattern before donor implementation begins. The goal is to make VAC a workflow-native product where every capability, policy, surface, validation, and lifecycle is declared through a control plane and executed safely by Rust.


## Current gate before control-plane implementation

ADR-0007 changes the execution order.

Before implementing `.vac` registry, capability dashboard, or donor-backed capabilities, the local product path must be cleaned:

```text
1. Build unblock.
2. Local Runtime Contract.
3. Rewire `vac exec`.
4. Rewire the root `vac` TUI path.
5. Old runtime reachability/delete gate.
6. Then `.vac` control-plane implementation.
```

Reason:

```text
The control plane must sit on the local VAC runtime contract, not on an app-server/protocol-shaped local path.
```

See:

```text
docs/architecture/decisions/ADR-0007-local-runtime-contract.md
docs/architecture/local-runtime-contract.md
docs/workflow-control-plane/plans/00A-build-unblock.md
docs/workflow-control-plane/plans/00B-local-runtime-contract.md
docs/workflow-control-plane/plans/00C-rewire-vac-exec.md
docs/workflow-control-plane/plans/00D-rewire-vac-tui.md
docs/workflow-control-plane/plans/00E-runtime-reachability-delete-gate.md
```

## Product goal

VAC should not look like a renamed terminal app. VAC should look like a workflow-native agentic CLI with a first-class declarative control plane.

Final mental model:

```text
.vac/                  declarative control plane
vac-rs/                runtime, TUI, execution, policy, safety
vac-cli/               package launcher for the `vac` command
donor/vac/             source-only donor material
```

## Core architecture rule

```text
YAML declares.
Rust executes.
TUI observes.
Policy gates.
Approval protects.
Dead code is deleted or quarantined.
```

Workflow files are not shell scripts. They are typed product declarations consumed by the VAC runtime.

## Repository target pattern

```text
.vac/
  capabilities/
    chat.yaml
    approvals.yaml
    tools.yaml
    sandbox.yaml
    sessions.yaml
    workflow.yaml
    manifest.yaml
    memory.yaml
    observability.yaml
    vil.yaml
    vwfd.yaml

  workflows/
    product.chat.submit.yaml
    product.patch.apply.yaml
    product.review.yaml
    product.workflow.run.yaml
    product.tool.call.yaml
    maintenance.build-check.yaml
    maintenance.identity-check.yaml
    maintenance.release-gate.yaml

  policies/
    approval.yaml
    sandbox.yaml
    tools.yaml
    filesystem.yaml

  surfaces/
    tui.yaml
    slash.yaml
    palette.yaml
    cli.yaml

  registry/
    product.yaml
    domains.yaml
    status.yaml
```

Rust target pattern:

```text
vac-rs/
  cli/
  tui/
  core/
  workflow/
  capability/
  policy/
  providers/
```

The existing root layout can migrate gradually. Do not rename or move large crates blindly until build checks are stable.

## Why this comes before donor implementation

The donor source is large. Without a control plane, donor work risks recreating the previous failure mode:

```text
backend exists
TUI does not show it
commands drift
approval is unclear
old frontend code duplicates new frontend code
```

The workflow control plane prevents that. Every capability must be declared, discoverable, policy-bound, and surfaced in the TUI or explicitly classified as CLI-only or retired.

## Non-negotiable constraints

- The only product command is `vac`.
- The only product TUI is the root VAC TUI under `vac-rs/tui`.
- Donor frontend stacks must not become product code.
- Backend-only migration is not accepted.
- Every capability must appear in the capability dashboard or be marked retired/test-only.
- Every workflow must have a schema, owner, status, validation gate, and UI projection.
- Mutating workflows require approval policy.
- Arbitrary shell execution is not allowed as the default workflow primitive.

## Control plane object types

### Capability manifest

A capability describes a product feature and its owned implementation.

Minimal schema:

```yaml
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow runner
status: partial
owner:
  crate: vac-rs/core
  module: workflow
surfaces:
  tui: /workflow
  slash: /workflow
  palette: true
  cli: true
states:
  empty: No workflows found
  loading: Scanning workflows
  success: Workflows loaded
  failure: Workflow scan failed
policy:
  mutates_files: possible
  approval_required_for:
    - run_mutating_step
validation:
  commands:
    - cargo check -p vac-surface-cli
```

### Workflow manifest

A workflow describes an operator flow that composes capabilities.

Minimal schema:

```yaml
schema_version: 1
kind: workflow
id: product.workflow.run
title: Run workflow from VAC TUI
inputs:
  workflow_id:
    type: string
    required: true
steps:
  - id: load
    uses: capability.workflow.load
  - id: validate
    uses: capability.workflow.validate
  - id: dry_run
    uses: capability.workflow.dry_run
  - id: approval
    uses: capability.approval.request
    when: dry_run.mutates_files == true
  - id: execute
    uses: capability.workflow.execute
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
validation:
  commands:
    - cargo check -p vac-surface-cli
```

### Policy manifest

A policy describes product safety and execution rules.

Minimal schema:

```yaml
schema_version: 1
kind: policy
id: policy.filesystem
filesystem:
  read: allow
  write: approval_required
  delete: approval_required
network:
  default: deny
tools:
  mutating: approval_required
```

### Surface manifest

A surface manifest describes what the TUI, slash help, palette, and CLI expose.

Minimal schema:

```yaml
schema_version: 1
kind: surface
id: surface.tui
title: VAC surface catalog
routes:
  - kind: tui
    path: /workflow
    capability: vac.workflow
    owner: vac-rs/tui
    visible: true
    status: partial
  - kind: slash
    command: /capabilities
    capability: vac.capabilities
    owner: vac-rs/tui
    visible: true
    status: planned
capabilities:
  - vac.workflow
  - vac.capabilities
```

## Phase 0 — repair root build

Before implementing the control plane, root build must be repairable.

Current blocker:

```text
vac-rs/vac-api websocket response code is using websocket extension APIs that are not available in the current registry dependency set.
```

Required outcome:

```bash
cd vac-rs
cargo check -p vac-surface-cli
```

Acceptance:

- dependency graph resolves,
- root CLI compiles,
- package launcher metadata still points to `vac`,
- identity terms forbidden by the root instructions do not reappear outside donor source.

## Phase 1 — create declarative control plane skeleton

Create directories:

```text
.vac/capabilities/
.vac/workflows/
.vac/policies/
.vac/surfaces/
.vac/registry/
```

Initial manifests:

```text
.vac/capabilities/chat.yaml
.vac/capabilities/approvals.yaml
.vac/capabilities/tools.yaml
.vac/capabilities/sandbox.yaml
.vac/capabilities/sessions.yaml
.vac/capabilities/workflow.yaml
.vac/capabilities/build.yaml
.vac/capabilities/tui.yaml
.vac/workflows/maintenance.build-check.yaml
.vac/workflows/maintenance.identity-check.yaml
.vac/workflows/maintenance.no-duplicate-tui.yaml
.vac/workflows/maintenance.release-gate.yaml
.vac/policies/approval.yaml
.vac/policies/sandbox.yaml
.vac/surfaces/tui.yaml
.vac/surfaces/slash.yaml
.vac/surfaces/palette.yaml
.vac/surfaces/cli.yaml
```

Do not add donor manifests yet. First model existing root capabilities.

## Phase 2 — add Rust registry loaders

Add minimal Rust registry code under root runtime.

Preferred target modules:

```text
vac-rs/core/src/capability_registry.rs
vac-rs/core/src/workflow_registry.rs
vac-rs/core/src/policy_registry.rs
vac-rs/core/src/surface_registry.rs
```

If `vac-rs/core` has a stricter module structure, place them in the closest existing configuration/runtime module instead.

Loader responsibilities:

- find `.vac` root,
- load YAML files,
- validate `schema_version`, `kind`, `id`, `title`, `status`, `owner`, `surfaces`, `validation`,
- return a typed snapshot,
- report structured errors with file path and field path,
- never execute shell commands while loading.

Acceptance:

```bash
cd vac-rs
cargo test -p vac-core capability_registry
cargo test -p vac-core workflow_registry
cargo check -p vac-surface-cli
```

## Phase 3 — capability dashboard in TUI

Add a root TUI surface that reads the registry snapshot.

User flow:

```text
vac
/capabilities
```

Visible rows:

- capability id,
- title,
- status: ready, partial, planned, broken, retired, CLI-only,
- owner crate/module,
- TUI surface,
- slash command,
- validation command,
- policy requirement.

Required states:

```text
empty: no capabilities found
loading: loading control plane
success: capabilities loaded
failure: schema or file error with path and field
```

Acceptance:

- `/capabilities` or equivalent TUI view is visible,
- invalid YAML shows a clear error,
- every root capability manifest appears,
- no donor capability appears until explicitly added.

## Phase 4 — workflow browser in TUI

Add a read-only workflow browser before building a runner.

User flow:

```text
vac
/workflow
```

Visible rows:

- workflow id,
- title,
- status,
- required inputs,
- number of steps,
- validation commands,
- policy gates,
- owner capability.

Required states:

```text
empty: no workflows found
loading: loading workflows
success: workflows loaded
failure: workflow schema error with path and field
```

Acceptance:

- maintenance workflows appear,
- product workflows appear,
- invalid workflow YAML is visible in the TUI,
- no workflow executes yet unless explicitly implemented in Phase 5.

## Phase 5 — minimal safe workflow runner

Implement a runner only for typed built-in step kinds.

Allowed initial step kinds:

```yaml
uses: capability.build.cargo_check
uses: capability.identity.check
uses: capability.activity.emit
uses: capability.approval.request
uses: capability.tui.pty_gate
```

Forbidden initial behavior:

```text
arbitrary shell script execution
unapproved file mutation
hidden network access
running donor code directly
```

Runner responsibilities:

- validate workflow before execution,
- create lifecycle events: pending, running, waiting_approval, success, failed, cancelled,
- surface progress in TUI,
- stop on policy violation,
- route mutating steps to approval,
- record validation output in activity.

Acceptance:

```text
vac -> /workflow -> maintenance.build-check -> dry-run -> run -> progress visible
```

## Phase 6 — root features become control-plane managed

Before donor work, convert existing root product areas to manifests.

Initial root capability set:

| Capability | Manifest | Required surface |
|---|---|---|
| chat submit | `.vac/capabilities/chat.yaml` | chat input and activity |
| patch approval | `.vac/capabilities/approvals.yaml` | approval UI |
| tools | `.vac/capabilities/tools.yaml` | `/tools` or capability row |
| sandbox | `.vac/capabilities/sandbox.yaml` | status/capability row |
| sessions | `.vac/capabilities/sessions.yaml` | session row or surface |
| workflow | `.vac/capabilities/workflow.yaml` | `/workflow` |
| build check | `.vac/capabilities/build.yaml` | workflow runner |
| root TUI | `.vac/capabilities/tui.yaml` | `/capabilities`, `/workflow`, `/debug-config` |
| release gate | `.vac/workflows/maintenance.release-gate.yaml` | workflow browser |

Acceptance:

- all root feature manifests load,
- the dashboard shows each feature status,
- missing implementation is marked partial/planned, not hidden,
- TUI does not claim a feature is ready unless validation exists.

## Phase 7 — donor work begins through the control plane

Only after Phases 0-6 pass, begin adding donor-backed capability manifests.

First donor-backed capability candidates:

| Donor domain | Root capability | Surface |
|---|---|---|
| VIL/VWFD validation | `vac.vil`, `vac.vwfd` | `/vil`, `/vwfd` |
| workflow runner | `vac.workflow` | `/workflow` |
| manifest/profile | `vac.manifest`, `vac.profile` | `/manifest`, `/profile` |
| memory/context | `vac.memory`, `vac.context` | `/memory`, `/context` |
| trace/signal/evidence | `vac.observability` | `/why`, `/signal`, `/evidence` |

No donor frontend code may be ported as product UI.

## Phase 8 — repo hygiene and dead-code enforcement

Add maintenance workflows:

```text
maintenance.identity-check.yaml
maintenance.no-duplicate-tui.yaml
maintenance.dead-code-scan.yaml
maintenance.release-gate.yaml
```

Rules:

- product source must remain VAC identity only,
- donor frontend stacks must not be product-routed,
- every root backend crate must appear in the capability dashboard or be marked test-only/retired,
- every workflow must have validation,
- every mutating workflow must require approval,
- any copied donor code must have cleanup status.

Acceptance:

```text
vac -> /workflow -> maintenance.identity-check -> pass
vac -> /workflow -> maintenance.no-duplicate-tui -> pass
vac -> /capabilities -> no unknown root domains
```

## Final definition of done

The repository has successfully adopted the workflow control plane pattern when:

- `.vac/` is the visible declarative control plane,
- root TUI can list capabilities and workflows,
- root workflow registry validates schema errors visibly,
- root workflow runner can run safe built-in maintenance workflows,
- root capabilities are registered before donor work,
- donor-backed features are introduced only through capability/workflow manifests,
- no backend-only feature can be called complete,
- no duplicate TUI path exists,
- release gate includes build, identity, TUI, policy, and workflow checks.


---

## Split execution plans

The detailed execution backlog is split under [`docs/workflow-control-plane/plans/`](plans/INDEX.md). Treat those files as the implementation checklist; this document remains the architecture overview.
