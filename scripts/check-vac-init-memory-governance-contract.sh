#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

test -f vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_memory_governance.rs
grep -q "pub enum MemoryTier" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_memory_governance.rs
grep -q "CredentialLikeContent" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_memory_governance.rs
grep -q "MissingTeamApproval" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_memory_governance.rs
grep -q "contains_credential_like_content" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_memory_governance.rs
grep -q "may_write_memory" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_memory_governance.rs
grep -q "vac_init_memory_governance" vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
test -f .vac/capabilities/vac-init-memory-governance.yaml
test -f .vac/workflows/maintenance.vac-init-memory-governance.yaml
printf 'vac-init memory governance contract: PASS\n'
