# VAC-Init Production Hardening C — Runtime Gate Enforcement

Status: implemented in sandbox artifact `vac-init-prod-hardening-c1-c4`.

## Scope

Production Hardening C binds the VAC-Init control plane to runtime decision points:

1. **C1 Pre-Plan Gate** — agent intake cannot proceed without an approved Semantic Plan, executable capability, loaded policy, and ownership report.
2. **C2 Pre-Patch Gate** — patch executor cannot mutate files unless the patch is inside the approved plan scope and passes patch guard.
3. **C3 Pre-Command Gate** — command runner cannot execute free-form commands, unknown runners, denied policy actions, or approval-required actions without persisted approval request.
4. **C4 Evidence Completion Gate** — task completion is blocked without evidence, valid chain, passing validation, no new orphan files, and approval reference when required.

## Code-backed contract

The runtime enforcement vocabulary and tests live in:

```text
vac-rs/core/src/control_plane/vac_init_runtime_enforcement.rs
```

The module is dependency-free and exposes:

```text
evaluate_pre_plan_gate
evaluate_pre_patch_gate
evaluate_pre_command_gate
evaluate_evidence_completion_gate
RuntimeEnforcementCoverage
```

These functions are exported through `vac-rs/core/src/control_plane/mod.rs` so later runtime phases can wire them into concrete agent intake, patch executor, command runner, and task lifecycle code.

## Runtime decisions

Each gate returns a typed `RuntimeGateReport`:

```text
Allow
ApprovalRequired
Block
```

Fail-closed behavior is mandatory. Missing plan, missing policy, missing ownership report, free-form commands, unresolved approval request, or missing evidence are blocking states.

## Validation

The primary targeted gate is:

```bash
bash scripts/check-vac-init-runtime-gate-enforcement-contract.sh
```

It compiles and runs the Rust gate tests, checks public exports, validates the `.vac` capability/workflow wiring, and confirms every validation command is structured.
