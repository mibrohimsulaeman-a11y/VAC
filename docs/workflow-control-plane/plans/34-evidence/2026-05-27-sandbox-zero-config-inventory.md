# Plan 34 Sandbox Zero-Config Inventory — 2026-05-27

Status: `34A-INVENTORY-RECORDED`

This inventory records current missing-`.vac` behavior signals and the first narrow Rust-owned classifier/doctor diagnostic baseline. It does not claim full zero-config CLI/TUI prompt behavior.

## Current product-repo behavior

- Strict VAC product gates still expect `.vac` manifests and should remain strict for the product repository.
- `vac doctor runtime-owner-gates` treats `.vac/capabilities/local_runtime_owner.yaml` as required and fails when it is absent.
- Registry/manifest doctors are product-repo validation surfaces; they should not be silently reinterpreted as arbitrary-user-project chat readiness.

## Zero-config implication

Plan 34 should introduce a separate Rust-owned user-project fallback contract rather than weakening strict product-repo gates. Missing `.vac` for arbitrary user projects should become setup warning/in-memory profile behavior only in the future zero-config path, not by changing current Plan 32 product gates.

## Next slice candidate

Implement a narrow `ProjectWorkspaceMode`/missing-`.vac` classifier in Rust with tests before wiring it into prompt submission or doctor UX.

## 34B sandbox implementation note

`vac-rs/core/src/project_workspace.rs` now provides a side-effect-free classifier for missing-`.vac` posture:

- arbitrary user project without `.vac` -> `ProjectWorkspaceMode::InMemory` + `StrictGateStatus::SetupWarning`, ordinary prompt allowed, durable writes still approval-required,
- strict product repository without `.vac` -> `ProjectWorkspaceMode::Strict` + `StrictGateStatus::FatalMissingVac`, strict gates fail closed,
- existing `.vac` -> `StrictGateStatus::Satisfied`.

The classifier is intentionally not wired into prompt submission yet. A diagnostic-only `vac doctor project-workspace <path>` surface is present so operators can see the classification without creating `.vac` or weakening strict product gates.
