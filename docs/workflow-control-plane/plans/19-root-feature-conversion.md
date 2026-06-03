# Plan 19 — Convert root features into control-plane manifests


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **complete for root manifest/status synchronization** — typed catalog, conversion report, capability/workflow status sync, and explicit donor/release defers are recorded. Follow-up evidence continues under `19-evidence/`. Further work is normal capability evolution, not Plan 19 conversion debt.
> Stop conditions: stop if manifest status claims ready without validation, donor-backed features lack owner/source metadata, or hidden features are silently ignored.
> ## Status
> | feature_id | capability_id | expected_source_roots | expected_surfaces | expected_policy | expected_validation | expected_ownership | current_status | target_status |

Code evidence:
- `vac-rs/core/src/control_plane/root_feature_conversion.rs`
- `vac-rs/core/src/control_plane/root_feature_catalog.rs`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/19-evidence/2026-05-28-sandbox-root-capability-ready-sync.md`
- `docs/workflow-control-plane/plans/19-evidence/2026-05-28-sandbox-root-status-sync.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **complete for root manifest/status synchronization** — typed catalog, conversion report, capability/workflow status sync, and explicit donor/release defers are recorded. Follow-up evidence continues under `19-evidence/`. Further work is normal capability evolution, not Plan 19 conversion debt.

Target outcome: root product features are represented by control-plane manifests with canonical capabilities, surfaces, policies, workflows, and drift detection.

Outputs: canonical feature catalog, normalized capability manifests, root feature conversion report, hidden/overclaimed diagnostics, donor/release hardening follow-ups, and validation evidence.

Requires / Blocks: requires schemas, registry loader, surface convergence, donor gate, and release gate.

Stop conditions: stop if manifest status claims ready without validation, donor-backed features lack owner/source metadata, or hidden features are silently ignored.

Done criteria: root features map to manifests, drift is detectable, and unresolved donor/release issues are explicit defers.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Model root product features as control-plane capability manifests, with explicit policy, surface, ownership, and validation coverage. Establish a canonical catalog that registry diagnostics and root feature conversion reports compare against, so no root domain drifts hidden, unowned, or overclaimed once donor-backed manifests are introduced.

## Status

- **Phase 1** (canonical catalog + drift sync) — implemented in `root_feature_catalog.rs`.
- **Phase 2** (normalize root capability manifests with explicit reasons) — implemented for the current root set; the live `.vac/capabilities/*.yaml` file count changes as maintenance/runtime-owner manifests are added. The root seed catalog remains the canonical 13-capability requirement; extra/out-of-scope manifests such as `vac.donor_migration` remain governed by their owning plans.
- **Phase 3** (hidden/overclaimed domain detection: `RootFeatureConversionReport`) — implemented in `root_feature_conversion.rs` with hidden source-domain entries and expanded overclaim checks.
- **Phase 4** (donor / release gate hardening) — represented by maintenance workflows and explicit release/PTY non-pass policy; donor-specific production evidence remains Plan 20/release input, not hidden Plan 19 debt.

Code source of truth for this plan:

```text
vac-rs/core/src/control_plane/root_feature_catalog.rs
vac-rs/core/src/control_plane/root_feature_conversion.rs
.vac/capabilities/*.yaml
.vac/surfaces/*.yaml
.vac/workflows/*.yaml
.vac/policies/*.yaml
```

## Canonical root feature catalog

The 13 features below are the source of truth for `vac-rs/core/src/control_plane/root_feature_catalog.rs::ROOT_SEED_CAPABILITY_REQUIREMENTS`. Any drift between this table and the catalog constant must be resolved before Phase 2 promotion runs.

| feature_id | capability_id | expected_source_roots | expected_surfaces | expected_policy | expected_validation | expected_ownership | current_status | target_status |
|---|---|---|---|---|---|---|---|---|
| chat | vac.chat | `vac-rs/core/src/chat/`, `vac-rs/core/src/runtime/chat_*.rs` | surface.tui, surface.slash | vac.policy.approval, vac.policy.tools | `cargo test -p vac-core chat`, `vac doctor registry` | vac-core::chat, vac-core::runtime | ready | ready |
| approvals | vac.approvals | `vac-rs/core/src/control_plane/approval_*.rs` | surface.tui, surface.slash | vac.policy.approval | `cargo test -p vac-core approval`, `vac doctor registry` | vac-core::control_plane::approval | ready | ready |
| tools | vac.tools | `vac-rs/core/src/tools/`, `vac-rs/tools/` | surface.tui, surface.palette, surface.slash | vac.policy.tools, vac.policy.sandbox | `cargo test -p vac-core tools`, `vac doctor registry` | vac-core::tools, vac-tools | ready | ready |
| sandbox | vac.sandbox | `vac-rs/core/src/sandboxing/`, `vac-rs/sandboxing/` | surface.cli, surface.tui | vac.policy.sandbox, vac.policy.filesystem | `cargo test -p vac-sandbox`, `vac doctor policy` | vac-sandbox, vac-core::sandbox | ready | ready |
| sessions | vac.sessions | `vac-rs/core/src/sessions/` | surface.tui, surface.slash, surface.palette | vac.policy.approval, vac.default-local | `cargo test -p vac-core sessions`, `vac doctor registry` | vac-core::sessions | ready | ready |
| workflow | vac.workflow | `vac-rs/core/src/control_plane/workflow_*.rs`, `.vac/workflows/` | surface.tui, surface.slash, surface.palette | vac.policy.approval, vac.policy.tools | `cargo test -p vac-core workflow_runner`, `vac doctor workflow` | vac-core::control_plane::workflow | ready | ready |
| build | vac.build | `vac-rs/core/src/control_plane/build_check.rs`, `vac-rs/cli/src/doctor_cli.rs` | surface.cli | vac.default-local | `vac doctor build .`, targeted `cargo check -p vac-surface-cli` | vac-core::control_plane::build_check, vac-cli::doctor_cli | ready | ready |
| identity | vac.identity | `vac-rs/core/src/identity/`, `vac-rs/cli/src/identity_cli.rs` | surface.cli, surface.tui | vac.policy.approval | `cargo test -p vac-core identity`, `vac doctor registry` | vac-core::identity, vac-cli::identity_cli | ready | ready |
| identity-check | vac.identity.check | `vac-rs/core/src/control_plane/identity_check.rs`, `.vac/workflows/maintenance.identity-check.yaml` | surface.cli, surface.tui | vac.policy.approval | `vac doctor identity`, `cargo test -p vac-core identity_check` | vac-core::identity::check | ready | ready |
| release | vac.release | `vac-rs/cli/src/doctor_cli.rs`, `.vac/workflows/maintenance.release-gate.yaml` | surface.cli | vac.policy.approval | `vac doctor registry`, release-gate workflow run | vac-cli::doctor_cli | ready | ready |
| tui | vac.tui | `vac-rs/tui/src/` | surface.tui | vac.default-local | `cargo test -p vac-surface-tui`, `vac doctor surfaces` | vac-tui | ready | ready |
| ownership | vac.ownership | `vac-rs/core/src/control_plane/ownership_scan.rs`, `vac-rs/cli/src/doctor_cli.rs` | surface.cli, surface.tui | vac.default-local | `vac doctor ownership`, `cargo test -p vac-core ownership_scan` | vac-core::control_plane::ownership_scan | ready | ready |
| architecture | vac.architecture | `vac-rs/core/src/control_plane/architecture_invariants.rs`, `docs/workflow-control-plane/` | surface.cli | vac.default-local | `vac doctor architecture`, `cargo test -p vac-core architecture` | vac-core::control_plane::architecture | ready | ready |

## Canonical surfaces

Defined in `ROOT_SEED_SURFACE_IDS`.

| surface_id | description |
|---|---|
| surface.tui | terminal UI (`vac-rs/tui/`) |
| surface.slash | slash-command surface inside chat / TUI |
| surface.palette | command palette surface inside TUI |
| surface.cli | CLI entrypoints under `vac-rs/cli/src/` |

Capabilities must declare `surfaces:` using exactly these ids; introducing a new surface requires both a catalog update and a surface manifest before any capability may reference it.

## Canonical policies

Defined in `ROOT_SEED_POLICY_IDS`.

| policy_id | description |
|---|---|
| vac.policy.approval | approval gating policy |
| vac.default-local | default local-workspace policy |
| vac.policy.filesystem | filesystem access policy |
| vac.policy.network | network egress policy |
| vac.policy.sandbox | sandbox isolation policy |
| vac.policy.tools | tools execution policy |

## Canonical workflows

Defined in `ROOT_SEED_WORKFLOW_IDS`.

| workflow_id | description |
|---|---|
| maintenance.build-check | build-check maintenance workflow |
| maintenance.identity-check | identity-check maintenance workflow |
| maintenance.release-gate | release-gate maintenance workflow |
| maintenance.ownership-scan | ownership scan maintenance workflow |
| maintenance.no-duplicate-tui | duplicate TUI guard workflow |
| maintenance.donor-migration-gate | donor migration gate workflow; capability ownership remains Plan 20 |

`maintenance.donor-migration-gate` is listed because it is part of the current root seed workflow set in code. The `vac.donor_migration` capability itself remains out of Plan 19 promotion scope and is owned by Plan 20 / donor migration docs.

## Drift findings (2026-05-19 audit)

1. **Plan doc vs catalog (this update fixes).** The previous version of this plan listed 9 features (`chat`, `approvals`, `tools`, `sandbox`, `sessions`, `workflow`, `build`, `identity`, `release`). `root_feature_catalog.rs` lists 13: the doc was missing `identity-check`, `tui`, `ownership`, `architecture`. Resolution: this update aligns the doc to the catalog code (source of truth).
2. **`.vac/capabilities/` vs catalog.** The capability directory now contains 15 YAML manifests; the root seed catalog requires 13. Extra/out-of-scope manifests include `vac.donor_migration` and any other Plan-20/maintenance-adjacent capability not listed in `ROOT_SEED_CAPABILITY_REQUIREMENTS`. `vac.donor_migration` must remain governed by Plan 20 / donor docs and must **not** be added to `ROOT_SEED_CAPABILITY_REQUIREMENTS`.
3. **Release/build evidence policy.** `vac.release` and `vac.build` are ready for default product semantics; full workspace build and real PTY evidence remain explicit non-pass operator inputs until recorded.
4. **Hidden/overclaimed domain checks.** `RootFeatureConversionReport` now exists and includes hidden source-domain entries plus expanded overclaim checks for modules, crates, expected source roots, surfaces, policy, validation, and ownership.
5. **Commit policy during hardening.** Further promotion commits should happen only after focused code-vs-doc audit of uncommitted changes and proportional validation.

## Out of scope

- `vac.approvals` capability YAML, `maintenance.release-gate.yaml`, `maintenance.donor-migration-gate.yaml`, and all `workflow_runner.rs` / `policy_manifest.rs` / `approval_*.rs` / `workflow_browser.rs` / `workflow_progress.rs` / `doctor_cli.rs` edits — owned by Plan 14.
- `vac.donor_migration` promotion — owned by Plan 20.
- Phase 4 hardening (release / donor gate metadata) — deferred until Plan 14 settles.

## Implementation phases

### Phase 1 — Canonical catalog

Implemented. `root_feature_catalog.rs::ROOT_SEED_CAPABILITY_REQUIREMENTS` defines 13 root seed capabilities with expected source roots, surfaces, policy, validation, ownership, current status, and target status.

### Phase 2 — Normalize capability manifests

Partially implemented. Current `.vac/capabilities/` file count is live-state dependent; root seed requirements stay in `ROOT_SEED_CAPABILITY_REQUIREMENTS`. Continue to:

1. Keep any future non-ready manifests with explicit `reason:` fields.
2. Promote to `ready` only when every expected field is populated and evidence exists.
3. Keep full-workspace build and real PTY evidence explicit as release/operator inputs; do not encode missing operator evidence as hidden capability debt.
4. Keep `vac.donor_migration` governed by Plan 20 / donor migration docs, not Plan 19.
5. Validate with `vac doctor registry`, `vac doctor surfaces`, and `vac doctor policy` when code/manifest behavior changes.

### Phase 3 — Hidden / overclaimed domain detection

Implemented in `root_feature_conversion.rs` and connected to ownership/registry diagnostics. Current behavior includes:

- `ConversionState::{Complete, Partial, Hidden, Overclaimed}`.
- `ConversionSeverity::{Info, Warning, Blocked}`.
- hidden source-domain entries surfaced as blocked severity.
- overclaimed detection for modules and crates.
- missing expected source roots, surfaces, policy, validation, and ownership.
- tests for hidden entries, overclaimed modules/crates, expected surface drift, and helper matching.

### Phase 4 — Donor / release gate hardening (deferred)

Pending Plan 14 settlement. Will introduce donor source metadata on `vac.release` and `vac.donor_migration`, and wire `maintenance.release-gate.yaml` + `maintenance.donor-migration-gate.yaml` into the conversion report.

## Validation per phase

```
# Phase 1 (docs-only)
./vac-rs/target/debug/vac doctor registry .

# Phase 2 (manifest normalization)
./vac-rs/target/debug/vac doctor registry .
./vac-rs/target/debug/vac doctor surfaces .
./vac-rs/target/debug/vac doctor policy .

# Phase 3 (code + test)
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core root_feature_conversion -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core ownership_scan -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui capability_dashboard_root_features -- --nocapture
cargo build --manifest-path vac-rs/Cargo.toml -p vac-surface-cli
./vac-rs/target/debug/vac doctor ownership .
./vac-rs/target/debug/vac doctor registry .
```

## Done

- Canonical table mirrors `root_feature_catalog.rs` constants exactly.
- Root seed conversion report enumerates every catalog feature with one of `complete | partial | hidden | overclaimed`.
- Hidden source-domain rows and overclaimed rows are visible through conversion/ownership/registry diagnostics.
- Non-ready capability manifests carry explicit reasons and must not be promoted from docs alone.
- `vac.release` remains non-ready until release evidence is production-ready.
- `vac.donor_migration` remains governed by Plan 20 / donor migration docs and is not part of `ROOT_SEED_CAPABILITY_REQUIREMENTS`.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | SE | Diag | Hidden-domain rows must not be silently skipped | ✅ code now surfaces hidden source-domain entries and severity maps hidden to blocked |
| 2 | P1 | Architect | Report | `RootFeatureConversionReport` must include hidden source domain entries | ✅ implemented via `ownership_scan.unowned_source_domains()` entries |
| 3 | P2 | SE | Detection | Overclaimed detection must cover more than modules | ✅ broadened to crates, expected source roots, surfaces, policy, validation, and ownership |
| 4 | P2 | Planner | Validation | `reason` required for non-ready capabilities | ✅ schema/manifest practice now requires non-ready reason; keep enforcing in manifest edits |
| 5 | P2 | Architect | Anti-drift | Canonical table must be machine-readable | ✅ typed `RootSeedCapabilityRequirement` now carries expected fields |
| 6 | P2 | Reviewer | Test | Negative hidden-domain tests missing | ✅ hidden source-domain and ownership tests added |
| 7 | P3 | Planner | Docs | Plan 19 docs need conversion semantics update | ✅ this doc now references current code-backed semantics |
| 8 | P3 | Reviewer | Test | Conversion report test coverage minimal | ✅ expanded tests for severity, expected source roots, surface drift, crate overclaim, ownership matching |


## 2026-05-28 sandbox closeout

- Root status sync evidence: [19-evidence/2026-05-28-sandbox-root-status-sync.md](19-evidence/2026-05-28-sandbox-root-status-sync.md).
