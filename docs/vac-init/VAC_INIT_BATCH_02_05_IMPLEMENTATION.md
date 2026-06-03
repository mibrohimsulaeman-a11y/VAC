# VAC-Init Batch 2-5 Implementation Notes

Baseline artifact: `vac-source-vac-init-foundation-plan-and-schema.zip`.

This batch implements the next four VAC-Init foundation contracts from the v1-alpha refined specification:

- Batch 2: typed manifest contracts for capability, policy, surface, and workflow.
- Batch 3: dependency-free registry validator and doctor-ready diagnostic vocabulary.
- Batch 4: `vac init` lifecycle state machine and resume-safe state record.
- Batch 5: source inventory and ownership scanner foundation.

The implementation is intentionally dependency-free in the new modules so each contract can be validated with direct `rustc --test` in sandbox conditions before full crate-level serde/CLI integration is used as a later gate.

## Batch 2 — Typed Manifest Contracts

Implemented in:

```text
vac-rs/core/src/control_plane/vac_init_manifest_contract.rs
```

Contract types:

```text
CapabilityManifestContract
PolicyManifestContract
SurfaceManifestContract
WorkflowManifestContract
StructuredValidationCommand
CapabilityLifecycleStatus
PolicyAction
PolicyDecision
RiskLevel
```

Validation functions:

```text
validate_capability_contract
validate_policy_contract
validate_surface_contract
validate_workflow_contract
merge_policy_decisions
```

Important behavior:

- `planned` capabilities must not expose executable surfaces.
- `ready` capabilities require owner, ownership, policy, surface, and validation.
- `deprecated` capabilities require deprecation metadata.
- high-risk validation commands cannot use `approval=never`.
- surface routes cannot expose planned capabilities as visible executable routes.
- workflow steps must use allowed `capability.*` or `step.*` vocabulary.
- policy decisions merge with most-restrictive-wins: `deny > approval_required > allow`.

## Batch 3 — Registry Validator

Implemented in:

```text
vac-rs/core/src/control_plane/vac_init_registry_validator.rs
```

Contract types:

```text
RegistryManifestRecord
RegistryValidationReport
RegistryValidatorDiagnostic
VacInitRegistryKind
```

Validation functions:

```text
validate_registry_records
validate_registry_yaml_documents
validate_registry_path
```

Important behavior:

- missing envelope fields are blocked;
- unknown manifest kind is blocked;
- invalid ID is blocked;
- duplicate manifest ID is blocked;
- schema version mismatch is blocked;
- `ready` capabilities require owner, ownership, policy, surfaces, and validation;
- current-workspace compatibility kinds remain accepted as info diagnostics until migration.

## Batch 4 — Init Lifecycle State Machine

Implemented in:

```text
vac-rs/core/src/control_plane/vac_init_lifecycle.rs
```

State progression:

```text
uninitialized
-> discovered
-> partition_selected
-> policy_inferred
-> manifests_synthesized
-> doctor_verified
-> ready
```

Failure states:

```text
scan_failed
operator_cancelled
policy_conflict
ownership_missing
doctor_failed
```

Important behavior:

- invalid transitions are rejected;
- `ready` is terminal;
- failure states increment retry count;
- state record renders a valid `kind: init_state` envelope;
- non-ready state records should be resumed by future CLI wiring.

## Batch 5 — Source Inventory and Ownership Scanner

Implemented in:

```text
vac-rs/core/src/control_plane/vac_init_ownership_scanner.rs
```

Contract types:

```text
SourceInventoryEntry
SourceInventoryReport
OwnershipTargetPattern
OwnershipFileRecord
OwnershipReport
OwnershipCoverageStatus
QuarantineLevel
OwnershipActionRequired
```

Important behavior:

- source files are classified as Rust source, tests, fixtures, manifests, docs, scripts, generated, vendor, build output, or other;
- `target/`, vendor, generated output, and node modules are ignored for ownership writes;
- path targets support deterministic simple glob matching;
- unowned source code is hard-quarantined;
- overclaimed files block writes and emit resolve-overclaim actions;
- partial ownership is represented distinctly from complete ownership.

## `.vac` registration

Added capabilities:

```text
vac.init.manifest-contracts
vac.init.registry-validator
vac.init.lifecycle
vac.init.ownership-scanner
```

Added workflows:

```text
maintenance.vac-init-manifest-contracts
maintenance.vac-init-registry-validator
maintenance.vac-init-lifecycle
maintenance.vac-init-ownership-scanner
maintenance.vac-init-batch2-5
```

Updated surfaces:

```text
.vac/surfaces/cli.yaml
.vac/surfaces/palette.yaml
.vac/surfaces/tui.yaml
```

Updated registry domains:

```text
.vac/registry/domains.yaml
```

## Deferred to later batches

This batch does not yet implement:

- full serde YAML loaders for the new typed contracts;
- live `vac init` interactive CLI;
- full AST risk scanner;
- policy evaluator merge engine beyond the manifest-level decision merge helper;
- semantic plan validator;
- approval replay protection;
- evidence chain writer.

Those remain scheduled for Batch 6+.
