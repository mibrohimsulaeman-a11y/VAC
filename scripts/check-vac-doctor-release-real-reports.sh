#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

grep -q 'fn build_real_doctor_aggregate' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
grep -q 'load_control_plane_registry_report(path)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
grep -q 'load_surface_doctor_report(path)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
grep -q 'load_policy_doctor_report_for_path(path)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
grep -q 'load_ownership_scan_report(path)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
grep -q 'load_workflow_run_report(path)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
grep -q 'count_release_ownership_context(path)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs

bash scripts/check-no-hardcoded-readiness-scoreboard.sh

printf 'doctor release real reports: PASS\n'
