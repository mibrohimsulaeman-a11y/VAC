#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-hardcoded-readiness.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

reject_pattern() {
  local pattern="$1"
  local label="$2"
  local out="$TMPROOT/scoreboard-grep.txt"
  if grep -RIn --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules \
      --include='*.rs' "$pattern" vac-rs/crates/control-plane/control-plane/src/control_plane vac-rs/crates/surfaces/cli/src >"$out"; then
    echo "FAIL: hardcoded readiness pattern remains: $label" >&2
    cat "$out" >&2
    fail=1
  fi
}

reject_pattern 'ReadinessScore::FiveOfFive' 'ReadinessScore::FiveOfFive'
reject_pattern 'ProductionRcScoreboard::production_rc1' 'ProductionRcScoreboard::production_rc1'
reject_pattern 'production_rc_scoreboard_is_all_five_of_five' 'scoreboard all-five test'
reject_pattern 'DoctorCheckReport::new(DoctorKind::Registry, DoctorStatus::Pass, Vec::new())' 'hardcoded registry pass'
reject_pattern 'DoctorCheckReport::new(DoctorKind::Surfaces, DoctorStatus::Pass, Vec::new())' 'hardcoded surfaces pass'
reject_pattern 'DoctorCheckReport::new(DoctorKind::Policy, DoctorStatus::Pass, Vec::new())' 'hardcoded policy pass'
reject_pattern 'DoctorCheckReport::new(DoctorKind::Ownership, DoctorStatus::Pass, Vec::new())' 'hardcoded ownership pass'
reject_pattern 'DoctorCheckReport::new(DoctorKind::Workflow, DoctorStatus::Pass, Vec::new())' 'hardcoded workflow pass'

if grep -RIn --exclude-dir=target --exclude-dir=.git --include='doctor_cli.rs' \
    'hard_quarantine_count: 0\|unowned_file_count: 0' vac-rs/crates/surfaces/cli/src >"$TMPROOT/release-context.txt"; then
  echo "FAIL: release context still hardcodes ownership counts" >&2
  cat "$TMPROOT/release-context.txt" >&2
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

printf 'no hardcoded readiness scoreboard: PASS\n'
