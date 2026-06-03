# VAC-Init Implementation Map

Status: Batch 0 baseline map  
Source baseline: `vac-source-tui-operator-ui-hardening4-10.zip`

## Spec-to-Code Map

| Spec Area | Target Source | Batch | Current Status | Notes |
|---|---|---:|---|---|
| Schema Envelope | `vac-rs/core/src/control_plane/schema_envelope.rs` | 1 | implemented | Dependency-free parser validates `schema_version`, `kind`, and `id`. |
| Kind Registry | `vac-rs/core/src/control_plane/kind_registry.rs` | 1 | implemented | Contains refined v1-alpha spec kinds; compatibility names remain parser-known for migration diagnostics while strict active `.vac` manifests are spec-only. |
| Capability Manifest | `vac-rs/core/src/control_plane/capability_manifest.rs` | 2 | existing/partial | Existing loader exists; refined spec alignment continues in Batch 2. |
| Policy Manifest | `vac-rs/core/src/control_plane/policy_manifest.rs` | 2 | existing/partial | Existing policy doctor model exists; fail-closed evaluator is Batch 7. |
| Workflow Manifest | `vac-rs/core/src/control_plane/workflow_manifest.rs` | 2 | existing/partial | Existing workflow manifest exists; refined typed step vocabulary continues later. |
| Surface Manifest | `vac-rs/core/src/control_plane/surface_manifest.rs` | 2 | existing/partial | Existing surface loader exists and already supports current route statuses. |
| Registry Validator | `vac-rs/core/src/control_plane/registry.rs`, `registry_diagnostics.rs` | 3 | existing/partial | Current registry doctor exists; schema envelope integration is next. |
| Init Lifecycle | `vac-rs/core/src/control_plane/init_state.rs` | 4 | missing | State machine not implemented yet. |
| Ownership Scanner | `vac-rs/core/src/control_plane/ownership_scan.rs` | 5 | existing/partial | Existing ownership reports exist; VAC-Init source inventory hardening remains. |
| Risk Scanner | `vac-rs/core/src/control_plane/risk_scanner.rs` | 6 | missing | Pattern/confidence scanner not implemented yet. |
| Policy Evaluator | `vac-rs/core/src/control_plane/policy_evaluator.rs` | 7 | missing | Fail-closed most-restrictive merge pending. |
| Structured Command | `vac-rs/core/src/control_plane/structured_command.rs` | 8 | missing | Pre-command gate pending. |
| Semantic Plan | `vac-rs/core/src/control_plane/semantic_plan.rs` | 9 | missing | Bounded plan validator pending. |
| Approval Binding | `vac-rs/core/src/control_plane/approval_request.rs` | 10 | partial | Approval store/lifecycle exists; replay protection contract pending. |
| Patch Guard | `vac-rs/core/src/control_plane/patch_guard.rs` | 11 | missing | Bounded patch contract pending. |
| Evidence Chain | `vac-rs/core/src/control_plane/evidence.rs` | 12 | missing | Canonical hash chain pending. |
| `vac why` | `vac-rs/core/src/control_plane/trajectory.rs`, `vac-rs/cli/src/why.rs` | 13 | missing | Safe rationale lookup pending. |
| Memory Governance | `vac-rs/core/src/control_plane/memory_record.rs` | 14 | missing | Tiered memory validator pending. |
| Doctor Aggregate | `vac-rs/cli/src/doctor.rs`, `doctor_report.rs` | 15 | partial | Individual doctors exist; release taxonomy pending. |
| TUI Integration | `vac-rs/tui/src/*` | 16 | partial | Operator UI hardening done; real VAC-Init data binding pending. |
| Fixtures | `tests/fixtures/*` | 17 | missing | Matrix pending. |
| Final Release Gate | `scripts/check-vac-init-*.sh` | 18 | partial | Batch 0-1 gates added. |

## Current Strictness Notes

The refined spec names `registry_status` as the root registry descriptor kind. Production Hardening A migrated active root descriptors to spec-kind manifests:

- `.vac/registry/product.yaml` now uses `kind: registry_status` with `legacy_kind: product`;
- `.vac/registry/status.yaml` now uses `kind: registry_status` with `legacy_kind: status`;
- `.vac/registry/donor-inventory.yaml` now uses `kind: registry_status` with `legacy_kind: donor_inventory`.

The parser can still describe legacy compatibility names for migration diagnostics, but `scripts/check-vac-init-registry-strictness-contract.sh` blocks them as active manifest kinds.

## Batch 0-1 Files

New Rust modules:

- `vac-rs/core/src/control_plane/schema_envelope.rs`;
- `vac-rs/core/src/control_plane/kind_registry.rs`.

New scripts:

- `scripts/check-vac-init-schema-envelope-contract.sh`;
- `scripts/check-vac-init-baseline-contract.sh`.

New `.vac` manifests:

- `.vac/capabilities/vac-init-schema-envelope.yaml`;
- `.vac/capabilities/vac-init-control-plane.yaml`;
- `.vac/workflows/maintenance.vac-init-baseline-audit.yaml`;
- `.vac/workflows/maintenance.vac-init-schema-envelope.yaml`.

Updated `.vac` manifest:

- `.vac/registry/domains.yaml` now has envelope `id: registry.domains` and includes `init.control_plane`.

## Batch 0-1 Acceptance

- All spec-required kind names exist in code.
- Unknown kind is rejected.
- Missing `kind` is rejected.
- Missing `id` is rejected.
- Bad non-dotted id is rejected by the strictness gate; the former root product descriptor now uses `id: vac.registry.product` with `legacy_id: vac` for audit context.
- Existing `.vac/**/*.yaml` files parse through top-level envelope scanning.
- Source artifact hygiene remains clean.

## Batch 2-5 Implementation Map Update

| Spec Area | Batch | Implementation File | Gate |
|---|---:|---|---|
| Capability/Policy/Surface/Workflow contracts | 2 | `vac-rs/core/src/control_plane/vac_init_manifest_contract.rs` | `scripts/check-vac-init-manifest-structs-contract.sh` |
| Registry validator + doctor diagnostics foundation | 3 | `vac-rs/core/src/control_plane/vac_init_registry_validator.rs` | `scripts/check-vac-init-registry-validator-contract.sh` |
| `vac init` lifecycle state machine | 4 | `vac-rs/core/src/control_plane/vac_init_lifecycle.rs` | `scripts/check-vac-init-lifecycle-contract.sh` |
| Source inventory + ownership scanner | 5 | `vac-rs/core/src/control_plane/vac_init_ownership_scanner.rs` | `scripts/check-vac-init-ownership-scanner-contract.sh` |

Batch 2-5 intentionally keeps the new modules dependency-free. The existing serde-backed manifest loaders remain available for current runtime behavior, while these modules encode the refined VAC-Init contracts as testable Rust models.
