# VAC-Init Batch 13-15 Implementation

Baseline: `vac-source-vac-init-bounded-loop-evidence-batch6-12.zip`

This batch implements the next three VAC-Init control-plane contracts:

1. Batch 13 — `vac why` safe rationale / trajectory index.
2. Batch 14 — memory governance.
3. Batch 15 — doctor aggregate / release gate.

## Batch 13 — Safe Rationale and `vac why`

File:

```text
vac-rs/core/src/control_plane/vac_init_safe_rationale.rs
```

The module models:

- `WhyQuery`
- `TrajectorySpan`
- `TrajectoryIndex`
- `WhyResult`
- `SafeRationaleSummary`
- `SafeMemoryRef`
- `SafeApprovalRef`

Contract rules:

- workspace paths must be normalized and relative;
- `vac why` must return diagnostics, not panic, for empty lookup;
- raw/private chain-of-thought is explicitly excluded;
- output may contain safe summaries, policy refs, evidence refs, and memory refs;
- depth is bounded by `MAX_WHY_DEPTH`.

## Batch 14 — Memory Governance

File:

```text
vac-rs/core/src/control_plane/vac_init_memory_governance.rs
```

The module models:

- `MemoryTier`
- `MemorySource`
- `MemoryContentType`
- `MemoryRecord`
- `MemoryAccess`

Contract rules:

- credential-like content is rejected in every tier;
- working memory must be task/session bounded;
- team memory requires operator/human-admin source and approval ref;
- semantic/team memory writes are approval-gated;
- tier size limits are enforced.

## Batch 15 — Doctor Aggregate Release Gate

File:

```text
vac-rs/core/src/control_plane/vac_init_doctor_release.rs
```

The module models:

- `DoctorKind`
- `DoctorStatus`
- `DoctorFinding`
- `DoctorCheckReport`
- `DoctorAggregateReport`
- `ReleaseGateContext`

Release gate rules:

- required doctors must be present;
- fatal doctor findings yield exit code `2`;
- failed/skipped doctors block release with exit code `1`;
- warnings keep exit code `0`;
- no policy loaded is fail-closed;
- hard quarantine, unowned files, and broken evidence chains block release.

## Validation

Primary aggregate gate:

```bash
bash scripts/check-vac-init-batch13-15-contract.sh
```

Targeted tests, when Rust toolchain is available:

```bash
rustc --edition 2024 --test vac-rs/core/src/control_plane/vac_init_safe_rationale.rs
rustc --edition 2024 --test vac-rs/core/src/control_plane/vac_init_memory_governance.rs
rustc --edition 2024 --test vac-rs/core/src/control_plane/vac_init_doctor_release.rs
```
