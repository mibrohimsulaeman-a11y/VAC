# O5/O6 + VAC-Init GAP-B/C Depth Remediation Report

Date: 2026-05-30
Status: SV-Done / TV-Pending

## Root cause from latest audit

The previous audit-remediation artifact fixed O6.2 metrics and the sandbox-suite false skip, but it did not touch the implementation gaps that remained open:

- GAP-B: missing dedicated fixtures for multi-policy merge, canonical evidence hashing, approval replay, and scanner confidence.
- GAP-C: patch guard had only a boolean `semantic_anchor_resolved`; it did not resolve symbol anchors itself.
- Registry validator gate returned `rc=127` when rustc was absent, making static strictness evidence less reproducible.
- Upload checksum gate was effectively a source-artifact no-op unless upload bundles were manually present.

## Changes implemented

### GAP-B fixtures

Added fixture records:

- `tests/fixtures/policy/multi_policy_merge_six_layer.yaml`
- `tests/fixtures/evidence/canonical_hash_vector.yaml`
- `tests/fixtures/approvals/replay_nonce_invalid.yaml`
- `tests/fixtures/risk/scanner_confidence_matrix.yaml`
- `tests/fixtures/patches/semantic_anchor_resolver.yaml`
- `tests/fixtures/fixture_matrix.yaml`

### GAP-C semantic anchor resolver

`vac-rs/control-plane/src/control_plane/vac_init_patch_guard.rs` now includes:

- `ResolvedSemanticAnchor`
- `SemanticAnchorResolutionError`
- `resolve_semantic_anchor_in_source`
- `validate_patch_attempt_with_semantic_source`

Resolution supports typed Rust anchors such as `fn:name`, `struct:Name`, `enum:Name`, `trait:Name`, `mod:name`, `type:Name`, `const:NAME`, `static:NAME`, and `impl:Type`. Missing or ambiguous anchors remain fail-closed and report `patch.anchor.unresolved` / ambiguity.

### Fixture-backed unit-test additions

Source-level Rust tests were added for:

- six-layer most-restrictive-wins policy merge
- path-specific policy precedence
- canonical evidence payload LF/sorted-key/self-hash-exclusion/comment-quote behavior
- UTC-Z evidence timestamp validation
- scanner confidence-band matrix
- detection-method inventory coverage
- semantic anchor resolution and fail-closed missing anchor handling

These tests are not TV-Done until rustc/cargo runs; this artifact only claims source-level implementation.

### Registry and checksum gate hardening

- `scripts/check-vac-init-registry-validator-contract.sh` now runs static registry preflight even when rustc is unavailable, and labels the rustc unit gate `NotEvaluated` instead of returning `127`.
- `scripts/check-vac-o5o6-upload-sha256.sh` now supports `VAC_UPLOAD_SHA256SUMS` and `VAC_UPLOAD_DIR`. With `/mnt/data/SHA256SUMS.txt` and `/mnt/data`, it verified `vac-merged-part-001..006.zip` as `OK`.

## Validation actually run

Targeted source-level gates:

```text
bash scripts/check-vac-init-gap-bc-depth-static.sh                       PASS
bash scripts/check-vac-init-registry-strictness-contract.sh              PASS (rustc unit NotEvaluated)
bash scripts/check-vac-init-registry-validator-contract.sh               PASS (rustc unit NotEvaluated)
bash scripts/check-vac-o6-2-safety-coverage.sh                           PASS
bash scripts/check-vac-o6-quality-triage.sh                              PASS
bash scripts/check-vac-o5-o6-completion-state.sh                         PASS
bash scripts/check-tui-source-artifact-hygiene.sh                        PASS
VAC_UPLOAD_SHA256SUMS=/mnt/data/SHA256SUMS.txt VAC_UPLOAD_DIR=/mnt/data \
  bash scripts/check-vac-o5o6-upload-sha256.sh                           PASS
```

Logs:

- `docs/monolith-quality/logs/gap-bc-depth-targeted-gates-summary.log`
- `docs/monolith-quality/logs/gap-bc-depth-upload-sha256.log`
- `docs/monolith-quality/logs/gap-bc-depth-o6-quality.log`
- `docs/monolith-quality/logs/gap-bc-depth-completion.log`
- `docs/monolith-quality/logs/gap-bc-depth-artifact-hygiene.log`

## Remaining truth

- Cargo build/clippy/test remain TV-Pending because rustc/cargo are not available in this source-only sandbox state.
- Crypto signing for approval/evidence remains deferred; replay invalidation is covered, but `algorithm: none` is still the interim mode.
- Migration runtime depth is not expanded in this slice.
- UI/scheduler GAP-D and monolith/legal GAP-E remain future work except for the upload checksum gate hardening noted above.
