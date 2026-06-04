#!/usr/bin/env bash
# Rustc-free guard for VAC-Init GAP-B fixtures and GAP-C depth remediation.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
python3 - <<'PY'
from pathlib import Path
import sys

root = Path('.')
errors = []

def require_file(path: str) -> str:
    p = root / path
    if not p.is_file():
        errors.append(f"missing required file: {path}")
        return ""
    return p.read_text(encoding='utf-8', errors='ignore')

def require_token(path: str, token: str) -> None:
    text = require_file(path)
    if token not in text:
        errors.append(f"missing token in {path}: {token}")

required_files = [
    'tests/fixtures/policy/multi_policy_merge_six_layer.yaml',
    'tests/fixtures/evidence/canonical_hash_vector.yaml',
    'tests/fixtures/approvals/replay_nonce_invalid.yaml',
    'tests/fixtures/risk/scanner_confidence_matrix.yaml',
    'tests/fixtures/patches/semantic_anchor_resolver.yaml',
    'tests/fixtures/fixture_matrix.yaml',
    '.vac/capabilities/vac-init-gap-bc-depth.yaml',
    '.vac/workflows/maintenance.vac-init-gap-bc-depth.yaml',
]
for path in required_files:
    require_file(path)

fixture_matrix = require_file('tests/fixtures/fixture_matrix.yaml')
for category in [
    'schema', 'state_machine', 'policy', 'workspace', 'evidence', 'plans',
    'patches', 'doctor', 'memory', 'trajectory', 'approvals', 'risk'
]:
    if f"  {category}:" not in fixture_matrix:
        errors.append(f"fixture matrix missing category: {category}")

for path, tokens in {
    'vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs': [
        'pub struct ResolvedSemanticAnchor',
        'pub enum SemanticAnchorResolutionError',
        'pub fn resolve_semantic_anchor_in_source',
        'pub fn validate_patch_attempt_with_semantic_source',
        'patch.anchor.unresolved',
        'fn resolves_function_anchor_to_line_range',
        'fn semantic_source_validation_requests_refresh_when_anchor_missing',
        'fn semantic_source_validation_rejects_range_outside_resolved_anchor',
    ],
    'vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_policy_evaluator.rs': [
        'fn six_policy_layers_most_restrictive_wins',
        'fn path_specific_rule_participates_in_precedence_merge',
        'PolicyLayerKind::ApprovalSession',
    ],
    'vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs': [
        'InvalidTimestamp',
        'fn is_utc_z_timestamp',
        'fn canonical_payload_uses_lf_sorted_keys_and_quotes_comment_like_values',
        'fn evidence_record_requires_utc_z_timestamp',
        '!payload.contains("self_hash")',
    ],
    'vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_approval_binding.rs': [
        'fn rejects_plan_hash_change',
        'fn rejects_diff_hash_change',
        'fn rejects_policy_snapshot_change',
        'fn rejects_expired_approval',
        'fn rejects_replay_nonce',
    ],
    'vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_risk_policy.rs': [
        'fn confidence_fixture_matrix_covers_all_labels',
        'fn risk_pattern_inventory_covers_detection_methods',
        'RiskDetectionMethod::FilenamePattern',
    ],
}.items():
    for token in tokens:
        require_token(path, token)

surface = require_file('.vac/surfaces/cli.yaml')
if 'vac.init.gap-bc-depth' not in surface:
    errors.append('surface.cli missing vac.init.gap-bc-depth capability')
if 'bash scripts/check-vac-init-gap-bc-depth-static.sh' not in surface:
    errors.append('surface.cli missing gap-bc-depth gate route')

if errors:
    for error in errors:
        print(f'FAIL: {error}', file=sys.stderr)
    sys.exit(1)

print('vac-init GAP-B/C depth static gate: PASS')
print('fixtures: multi-policy, canonical-hash, approval-replay, risk-confidence, semantic-anchor')
print('depth: semantic anchor resolver + fail-closed validation registered')
PY
