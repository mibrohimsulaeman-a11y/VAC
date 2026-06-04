#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

test -f vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_safe_rationale.rs
grep -q "pub struct WhyQuery" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_safe_rationale.rs
grep -q "pub struct TrajectoryIndex" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_safe_rationale.rs
grep -q "raw_chain_of_thought_excluded" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_safe_rationale.rs
grep -q "contains_raw_chain_of_thought" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_safe_rationale.rs
grep -q "lookup_safe_rationale" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_safe_rationale.rs
grep -q "vac_init_safe_rationale" vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
test -f .vac/capabilities/vac-init-safe-rationale.yaml
test -f .vac/workflows/maintenance.vac-init-safe-rationale.yaml
printf 'vac-init safe rationale contract: PASS\n'
