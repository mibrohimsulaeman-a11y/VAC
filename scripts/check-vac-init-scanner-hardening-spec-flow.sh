#!/usr/bin/env bash
# SUITE_SKIP: source-backed static contract; historical logs were pruned.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-scanner-hardening.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_grep() {
  local pattern="$1"
  local path="$2"
  if ! grep -qE "$pattern" "$path"; then
    echo "FAIL: missing pattern in $path: $pattern" >&2
    exit 1
  fi
}

require_absent() {
  local pattern="$1"
  local path="$2"
  if grep -qE "$pattern" "$path"; then
    echo "FAIL: forbidden pattern in $path: $pattern" >&2
    exit 1
  fi
}

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs
require_file vac-rs/crates/surfaces/cli/src/init_cli.rs
require_file .vac/registry/plans/plan.scanner-hardening.spec-flow.yaml
require_file .vac/capabilities/scanner-hardening-spec-flow.yaml
require_file .vac/workflows/maintenance.scanner-hardening-spec-flow.yaml
require_file .vac/.init/source_inventory.yaml
require_file .vac/.init/source_inventory/by-class/product.yaml
require_file .vac/.init/source_inventory/by-class/test.yaml
require_file .vac/.init/source_inventory/by-class/donor_reference.yaml
require_file .vac/.init/source_inventory/by-class/donor_quarantined.yaml
require_file .vac/.init/risk_findings.yaml
require_file .vac/.init/risk_findings/index.yaml
require_file .vac/.init/risk_findings/full.yaml
require_file .vac/.init/risk_findings/by-risk/credential_read.yaml
require_file .vac/.init/risk_findings/by-scope/product.yaml
require_file .vac/.init/risk_findings/by-scope/test.yaml
require_file .vac/.init/policy_inference_report.yaml
require_file .vac/.init/scanner_doctor_report.yaml

require_grep 'build_vac_init_live_scanner_report_files' vac-rs/crates/surfaces/cli/src/init_cli.rs
require_grep 'LiveSourceClass::DonorQuarantined' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs
require_grep 'alternatives: Vec<String>' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs
require_grep 'id: finding.live-init.report' .vac/.init/risk_findings.yaml
require_grep 'full: .vac/.init/risk_findings/full.yaml' .vac/.init/risk_findings.yaml
require_absent '^findings:' .vac/.init/risk_findings.yaml
require_grep 'ownership:' .vac/.init/risk_findings/full.yaml
require_grep 'ownership_not_evaluated_findings:' .vac/.init/policy_inference_report.yaml
require_grep 'ownership_status: NotEvaluated' .vac/.init/scanner_doctor_report.yaml
require_grep 'scanner_doctor_report.yaml' vac-rs/crates/surfaces/cli/src/init_cli.rs
require_grep 'RescanAst' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_cli_runtime.rs
require_grep 'rescan_ast' vac-rs/crates/surfaces/cli/src/init_cli.rs

if command -v "$RUSTC_BIN" >/dev/null 2>&1; then
  "$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs -o "$TMPROOT/vac_init_live_scanner_policy_test"
  "$TMPROOT/vac_init_live_scanner_policy_test" --nocapture
else
  echo "scanner rustc unit gate: NotEvaluated (rustc not found: $RUSTC_BIN)" >&2
fi

PY_STDERR="$TMPROOT/python-stderr.log"
if ! python3 - <<'PY' 2>"$PY_STDERR"
from pathlib import Path
import yaml

root = Path('.')
summary = yaml.safe_load(Path('.vac/.init/risk_findings.yaml').read_text())
index = yaml.safe_load(Path('.vac/.init/risk_findings/index.yaml').read_text())
full = yaml.safe_load(Path('.vac/.init/risk_findings/full.yaml').read_text())
policy = yaml.safe_load(Path('.vac/.init/policy_inference_report.yaml').read_text())
source_inventory = yaml.safe_load(Path('.vac/.init/source_inventory.yaml').read_text())
doctor = yaml.safe_load(Path('.vac/.init/scanner_doctor_report.yaml').read_text())

summary_total = summary['summary']['total_findings']
full_total = full['summary']['total_findings']
assert summary_total == full_total, (summary_total, full_total)
assert len(full.get('findings') or []) == full_total, (len(full.get('findings') or []), full_total)
assert sum((index.get('by_risk') or {}).values()) == full_total
assert sum((index.get('by_scope') or {}).values()) == full_total
assert 'findings' not in summary, 'risk_findings.yaml must be summary-only'

source_summary = source_inventory['summary']
for key in ['product_runtime', 'product_test', 'donor_reference', 'donor_quarantined', 'generated']:
    assert key in source_summary, f'missing source class {key}'

for finding in full.get('findings') or []:
    for field in ['id', 'file', 'line', 'pattern', 'inferred_risk', 'scope', 'confidence', 'method', 'ambiguous', 'alternatives', 'ownership']:
        assert field in finding, f'missing {field} in {finding.get("id")}'
    assert finding['method'] == 'ast_exact'
    ownership = finding['ownership']
    for field in ['status', 'capability', 'quarantine', 'reason']:
        assert field in ownership, f'missing ownership.{field} in {finding.get("id")}'
    assert finding['scope'] != 'donor_reference'
    assert finding['scope'] != 'donor_quarantined'

rules = policy.get('rules') or []
assert doctor['summary']['risk_findings'] == full_total
assert doctor['summary']['ownership_status'] in ['NotEvaluated', 'pass', 'blocked']
assert doctor['checks']['full_storage'] == 'pass'
assert doctor['checks']['policy_fail_closed'] == 'pass'
assert rules, 'policy rules missing'
for rule in rules:
    assert rule.get('decision') != 'allow' or rule.get('risk') == 'safe_read', f"broad allow forbidden: {rule}"

runtime_cred = [r for r in rules if r.get('risk') == 'credential_read' and r.get('scope') == 'product_runtime']
if runtime_cred:
    assert all(r.get('decision') == 'deny' for r in runtime_cred), runtime_cred

print('scanner hardening consistency: PASS')
PY
then
  cat "$PY_STDERR" >&2
  exit 1
fi

printf 'vac-init scanner hardening spec-flow: PASS\n'
