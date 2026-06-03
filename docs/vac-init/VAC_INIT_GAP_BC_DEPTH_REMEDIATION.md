# VAC-Init GAP-B / GAP-C Remediation

Date: 2026-05-30
Status: SV-Done / TV-Pending

## Scope

This slice closes the source-level gaps called out by the re-audit for:

- GAP-B fixtures/test coverage: multi-policy merge, canonical evidence hashing,
  approval replay invalidation, scanner confidence matrix, and fixture matrix coverage.
- GAP-C semantic-anchor depth: bounded patch guard now has a source-level resolver
  for typed Rust anchors such as `fn:run`, `struct:Config`, `enum:Mode`, `trait:Gate`,
  `mod:workflow`, and `impl:PatchGuardContext`.

Cargo/rustc build, clippy, and workspace tests remain TV-Pending in this source-only artifact.

## Implemented source changes

- `vac_init_patch_guard.rs`
  - added `ResolvedSemanticAnchor`
  - added `SemanticAnchorResolutionError`
  - added `resolve_semantic_anchor_in_source`
  - added `validate_patch_attempt_with_semantic_source`
  - added fail-closed tests for missing and ambiguous anchors
- `vac_init_policy_evaluator.rs`
  - added six-layer most-restrictive-wins fixture test
  - added path-prefix precedence fixture test
- `vac_init_evidence_chain.rs`
  - added UTC-Z timestamp validation
  - added canonical payload fixture tests for LF, sorted keys, comment quoting, and self-hash exclusion
- `vac_init_risk_policy.rs`
  - added confidence band matrix test
  - added detection-method inventory test

## New fixtures

- `tests/fixtures/policy/multi_policy_merge_six_layer.yaml`
- `tests/fixtures/evidence/canonical_hash_vector.yaml`
- `tests/fixtures/approvals/replay_nonce_invalid.yaml`
- `tests/fixtures/risk/scanner_confidence_matrix.yaml`
- `tests/fixtures/patches/semantic_anchor_resolver.yaml`
- `tests/fixtures/fixture_matrix.yaml`

## Gates

- `scripts/check-vac-init-gap-bc-depth-static.sh`

This gate is rustc-free and checks that the fixtures, source symbols, test vectors, manifests,
workflow, and CLI surface wiring are present. It does not replace the existing standalone rustc
unit gates; it only provides reproducible static evidence in source-only sandbox artifacts.

## Remaining work

- TV-Pending: run `cargo build --offline --workspace --locked`, `cargo clippy`, and `cargo test`.
- Crypto-signature implementation remains deferred. The approval path still supports `algorithm: none` as interim evidence; replay invalidation is source-covered.
- Migration runtime depth remains a later GAP-C item.
