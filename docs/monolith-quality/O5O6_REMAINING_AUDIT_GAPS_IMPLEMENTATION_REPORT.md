# O5/O6 Remaining Audit Gaps Implementation Report

Date: 2026-05-30
Artifact baseline: `vac-o5o6-gap-bc-depth-source.zip`
Result: `SV-Done / TV-Pending`

## Scope

This slice implements the remaining audit backlog that can be safely completed at source/static level without a stable cargo environment:

- GAP-C crypto-signature path for approval and evidence records.
- GAP-C migration runtime depth: add/rename/remove/type-change/version-bump, dry-run/apply/rollback, and registry-doctor verification.
- GAP-D TUI/scheduler static state fixtures and no-`rustc` fallback gates.
- GAP-E legal/provenance notices and release-blocker truth labels.
- Gate hardening so direct contract scripts do not fail with `rc=127` when cargo/rustc is unavailable.

## Implemented source changes

### Approval signing

`vac_init_approval_binding.rs` now includes:

- `ApprovalResponseSignature`
- `ApprovalSignaturePolicy::{AllowUnsigned, RequireEd25519}`
- `approval_signature_payload`
- `verify_approval_ed25519_signature`
- `validate_approval_binding_with_signature_policy`

The default legacy/interim path can still allow unsigned approvals explicitly through `AllowUnsigned`; signed enforcement is a separate fail-closed policy path.

### Evidence signing

`vac_init_evidence_chain.rs` now includes:

- `EvidenceSignatureEnvelope`
- `EvidenceSignatureError`
- `evidence_signature_payload`
- `verify_evidence_ed25519_signature`

The signature payload is the canonical evidence payload, which continues to exclude `self_hash`.

### Migration runtime depth

`vac_init_migration_runtime.rs` now includes:

- full migration action vocabulary: `AddField`, `RemoveField`, `RenameField`, `ChangeKind`, `ChangeId`, `ChangeType`, `VersionBump`
- inverse action validation
- dry-run/apply/rollback planning
- registry-doctor verification requirement
- YAML scalar text transform helpers for migration previews and deterministic rollback tests

### TUI/scheduler fixture depth

Added `tests/fixtures/tui/scheduler_monitor_states.yaml` covering empty/loading/success/failure scheduler-monitor states and explicit no-bypass policy.

### Legal/provenance

`THIRD_PARTY_NOTICES.md` now explicitly tracks:

- OpenAI Codex CLI Apache-2.0 provenance.
- Ratatui MIT attribution.
- dependency-attribution/cargo-about remaining `NotEvaluated` until actual toolchain validation.

## Validation actually run

See `docs/monolith-quality/logs/remaining-audit-gaps-targeted-gates.log`.

Targeted static gates passed:

- `scripts/check-vac-init-remaining-audit-gaps-static.sh`
- `scripts/check-vac-init-approval-binding-contract.sh`
- `scripts/check-vac-init-evidence-chain-contract.sh`
- `scripts/check-vac-init-migration-runtime-contract.sh`
- `scripts/check-vac-init-tui-real-data-contract.sh`
- `scripts/check-vac-init-gap-bc-depth-static.sh`
- `scripts/check-vac-init-registry-strictness-contract.sh`
- `scripts/check-vac-init-registry-validator-contract.sh`
- `scripts/check-vac-o6-2-safety-coverage.sh`
- `scripts/check-vac-o6-quality-triage.sh`
- `scripts/check-vac-o5-o6-completion-state.sh`
- `scripts/check-legal-release-blockers.sh`
- TUI static runtime contracts and artifact hygiene gates.
- external upload SHA verification against `/mnt/data/SHA256SUMS.txt`.

## Still TV-Pending

No cargo build/clippy/test was completed in this artifact. This is source/static implementation only. The new Ed25519 code adds a dependency edge from `vac-core` to workspace `ed25519-dalek`; this must be verified with cargo in the next stable toolchain session.

## Still not claimed done

The following remain blocked or intentionally not claimed:

- Full workspace `cargo build --offline --workspace --locked`.
- Workspace `cargo clippy -D warnings`.
- Workspace `cargo test`.
- O5.2 semantic god-file split beyond mechanical staging.
- O5.5 donor crate deletion and binary relocation.
- O6.1 full de-panic remediation.
- cargo-about/cargo-deny generated legal dependency report.
