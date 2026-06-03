# .vac/capabilities

Declarative capability manifests for the VAC control plane. A capability names a feature domain that the product exposes through one or more surfaces and gates with a policy.

- **Schema source**: [`vac-rs/control-plane/src/control_plane/capability_manifest.rs`](../../vac-rs/control-plane/src/control_plane/capability_manifest.rs)
- **Schema doc**: [`docs/workflow-control-plane/schema/capability-manifest.schema.md`](../../docs/workflow-control-plane/schema/capability-manifest.schema.md) (index: [`docs/workflow-control-plane/schema/INDEX.md`](../../docs/workflow-control-plane/schema/INDEX.md))
- **Doctor gate**: `vac doctor registry .`

## Invariants

The loader (`capability_manifest.rs`) enforces:

- `schema_version: 1`, `kind: capability`.
- `id` is non-empty, lowercase ASCII (`a-z 0-9 . _ -`), must start with `vac.`, no whitespace.
- `status` ∈ `planned | partial | ready | deprecated | disabled`. `planned` and `partial` require a non-empty `reason`.
- Non-deprecated capabilities declare visible `states` for operator surfaces (`empty`, `loading`, `success`, `failure` at minimum for the seed set).
- `owner` is either a non-empty path string (e.g. `"vac-rs/cli::doctor_cli"`) or an object `{ crate, module, team? }`.
- `surfaces` declares at least one visible surface (`tui.routes[]`, `slash.commands[]`, `palette: true`, or `cli.enabled` / `cli.commands[]`).
- `policy` declares at least one of `risk`, `mutates_files`, `network`, `redaction`, or `approval_required_for[]`.
- `validation` provides `commands[]` (real shell-compatible commands) and/or `gates[]` (referenced workflow step ids).
- Donor-backed capabilities (`donor_source.cleanup_status: donor_only`) must expose at least a TUI route or a CLI command, and `donor_source.root_target` must point under `vac-rs/{core,cli,tui,app-server}/`.

## File naming

`<capability-suffix>.yaml` — capability id with the `vac.` prefix dropped and dots converted to hyphens for compound ids (`vac.identity.check` → `identity-check.yaml`, `vac.tui.pty_gate` → `tui-pty-gate.yaml`). One capability per file.

## Canonical example

Lifted verbatim from [`build.yaml`](build.yaml):

```yaml
schema_version: 1
kind: capability
id: vac.build
title: Build
status: ready
owner:
  crate: vac-surface-cli
  module: main
ownership:
  crates:
    - vac-cli
  modules:
    - main
description: Build and maintenance validation gates for the root product.
reason: |
  CLI doctor build entry-point, `surface.cli` route (`vac doctor build`), approval-gated `maintenance.build-check`, and targeted build readiness evidence are wired. Full workspace build remains operator-gated release evidence, not a non-ready capability label.
depends_on:
  - vac.workflow
surfaces:
  palette: true
states:
  - empty
  - loading
  - success
  - failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
```

## Active capabilities

`approvals`, `architecture`, `build`, `chat`, `donor_migration`, `identity`, `identity.check`, `local_runtime_owner`, `ownership`, `release`, `runtime_approval_bridge`, `sandbox`, `sessions`, `tools`, `tui`, `tui.pty_gate`, `tui_session_runtime`, `workflow`.

Each id appears in [`.vac/registry/domains.yaml`](../registry/domains.yaml) with the `vac.` prefix stripped. Run `vac doctor registry .` after touching any manifest to keep that link enforced.
