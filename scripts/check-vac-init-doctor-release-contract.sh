#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

test -f vac-rs/control-plane/src/control_plane/vac_init_doctor_release.rs
grep -q "pub enum DoctorKind" vac-rs/control-plane/src/control_plane/vac_init_doctor_release.rs
grep -q "pub struct DoctorAggregateReport" vac-rs/control-plane/src/control_plane/vac_init_doctor_release.rs
grep -q "pub const REQUIRED_DOCTORS" vac-rs/control-plane/src/control_plane/vac_init_doctor_release.rs
grep -q "fail-closed: no policy loaded" vac-rs/control-plane/src/control_plane/vac_init_doctor_release.rs
grep -q "broken evidence chain blocks release" vac-rs/control-plane/src/control_plane/vac_init_doctor_release.rs
grep -q "aggregate_doctor_release" vac-rs/control-plane/src/control_plane/vac_init_doctor_release.rs
grep -q "vac_init_doctor_release" vac-rs/control-plane/src/control_plane/mod.rs
test -f .vac/capabilities/vac-init-doctor-release.yaml
test -f .vac/workflows/maintenance.vac-init-doctor-release.yaml
test -f docs/validation/VAC_INIT_DOCTOR_RELEASE_GATE.md
printf 'vac-init doctor release contract: PASS\n'
