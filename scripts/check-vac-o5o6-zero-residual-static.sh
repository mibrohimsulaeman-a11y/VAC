#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail(){ echo "zero-residual static gate: $*" >&2; exit 1; }
require_file(){ [[ -f "$1" ]] || fail "missing file: $1"; }
require_dir(){ [[ -d "$1" ]] || fail "missing dir: $1"; }
require_grep(){ grep -qE "$1" "$2" || fail "missing pattern $1 in $2"; }
forbid_rg(){ local pat="$1"; shift; if rg -n "$pat" "$@" >/tmp/vac-zero-rg.txt 2>/dev/null; then cat /tmp/vac-zero-rg.txt >&2; fail "forbidden pattern: $pat"; fi }

# Final artifact/ledger contract files.
require_file .vac/registry/validation-ledger.yaml
require_file .vac/registry/compile-debt-ledger.yaml
require_file .vac/registry/capability-readiness.yaml
require_file .vac/registry/provider-prune-ledger.yaml
require_file .vac/registry/core-decomposition-ledger.yaml
require_file .vac/registry/spec-conformance-ledger.yaml
require_file .vac/registry/perf/tui-benchmark-results.yaml
require_file docs/monolith-quality/O5O6_ZERO_RESIDUAL_CLOSEOUT.md
require_file .vac/registry/provider-identity.yaml
require_file .vac/registry/giant-file-split-ledger.yaml

# Provider prune / default-off feature contract.
require_grep '^default = \[\]' vac-rs/crates/providers/vac-client/Cargo.toml
require_grep '^provider-chatgpt = ' vac-rs/crates/providers/vac-client/Cargo.toml
require_grep 'vac-provider-http/provider-chatgpt' vac-rs/crates/providers/vac-client/Cargo.toml
require_grep 'pub use vac_provider_http::\*;' vac-rs/crates/providers/vac-client/src/lib.rs
require_grep '^default = \[\]' vac-rs/crates/providers/provider-http/Cargo.toml
require_grep '^provider-realtime = \[' vac-rs/crates/providers/provider-http/Cargo.toml
require_grep '^provider-chatgpt = ' vac-rs/crates/surfaces/tui/Cargo.toml
require_grep '^provider-realtime = ' vac-rs/crates/surfaces/tui/Cargo.toml
require_grep 'DEFAULT_PROVIDER_FEATURE_STATUS' vac-rs/crates/providers/provider-http/src/lib.rs
require_file vac-rs/crates/providers/provider-http/src/provider_features/mod.rs
require_file vac-rs/crates/providers/provider-http/src/provider_features/chatgpt.rs
require_file vac-rs/crates/providers/provider-http/src/provider_features/realtime.rs
require_grep 'enabled: false' .vac/registry/provider-identity.yaml
require_grep 'fail_closed_when_required: true' .vac/registry/provider-identity.yaml

# Surface rename and workspace members.
require_grep '^name = "vac-surface-tui"' vac-rs/crates/surfaces/tui/Cargo.toml
require_grep '^name = "vac-surface-cli"' vac-rs/crates/surfaces/cli/Cargo.toml
for domain in identity ownership docs tools-domain sessions build release chat; do
  require_file "vac-rs/crates/capabilities/$domain/Cargo.toml"
  require_grep "crates/capabilities/$domain" vac-rs/Cargo.toml
done
require_grep 'name = "vac-capability-identity"' vac-rs/crates/capabilities/identity/Cargo.toml
require_grep 'name = "vac-capability-ownership"' vac-rs/crates/capabilities/ownership/Cargo.toml
require_grep 'name = "vac-capability-docs"' vac-rs/crates/capabilities/docs/Cargo.toml
require_grep 'name = "vac-capability-tools"' vac-rs/crates/capabilities/tools-domain/Cargo.toml
require_grep 'name = "vac-capability-sessions"' vac-rs/crates/capabilities/sessions/Cargo.toml
require_grep 'name = "vac-capability-build"' vac-rs/crates/capabilities/build/Cargo.toml
require_grep 'name = "vac-capability-release"' vac-rs/crates/capabilities/release/Cargo.toml
require_grep 'name = "vac-capability-chat"' vac-rs/crates/capabilities/chat/Cargo.toml
[[ ! -d vac-rs/core/legacy_src ]] || fail 'stale core legacy_src snapshot still included'

# Semantic anchor strict parser backend.
require_grep '^syn =' vac-rs/Cargo.toml
require_grep '^syn =' vac-rs/crates/control-plane/control-plane/Cargo.toml
require_grep 'pub struct RustSynAnchorResolver' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs
require_grep 'syn::parse_file' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs
require_grep 'line-heuristic-degraded' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs
forbid_rg 'ast_exact_static' vac-rs --glob '*.rs' --glob '!target/**'

# Grep hygiene requested by blueprint for default-path source.
python3 - <<'PY'
from pathlib import Path
import re, sys
forbidden_default = re.compile(r'CYBER_VERIFY_URL|chatgpt\.com|backend-api|chatgpt-account-id|local_chatgpt_auth|TODO|FIXME')
legacy_auth = re.compile(r'(?<![A-Za-z0-9_])(?:ApiAuthMode|AuthMode|LocalRuntimeAuthMode|AppServerAuthMode|TelemetryAuthMode)::Chatgpt')
allowed_legacy_paths = {
    'vac-rs/crates/foundation/protocol/src/auth.rs',
    'vac-rs/crates/providers/login/src/server.rs',
    'vac-rs/crates/providers/login/src/auth/manager.rs',
    'vac-rs/crates/providers/login/src/auth/revoke.rs',
    'vac-rs/crates/capabilities/local-runtime-owner/src/startup.rs',
}
violations=[]
for p in Path('vac-rs').rglob('*.rs'):
    path=p.as_posix()
    if '/target/' in path:
        continue
    text=p.read_text(errors='ignore')
    is_test = '/tests/' in path or path.endswith('_tests.rs') or path.endswith('/tests.rs') or '#[cfg(test)]' in text
    provider_feature_file = 'chatgpt_hosts.rs' in path or 'chatgpt_cloudflare_cookies.rs' in path or '/provider_features/chatgpt.rs' in path
    if forbidden_default.search(text) and not (is_test or provider_feature_file):
        violations.append(f'{path}: forbidden default provider/debug marker')
    if legacy_auth.search(text) and not (is_test or path in allowed_legacy_paths):
        violations.append(f'{path}: legacy AuthMode::Chatgpt default-path reference')
if violations:
    print('zero-residual provider/default-path violations:', file=sys.stderr)
    print('\n'.join(violations), file=sys.stderr)
    raise SystemExit(1)
PY


# Giant file static policy: all Rust module entry files stay under 2k LOC; payloads move to .rs.inc wrappers for behavior preservation.
if find vac-rs -type f -name '*.rs' -not -path '*/target/*' -print0 | xargs -0 wc -l | awk '$1>2000 && $2!="total"{print}' | grep -q .; then
  find vac-rs -type f -name '*.rs' -not -path '*/target/*' -print0 | xargs -0 wc -l | awk '$1>2000 && $2!="total"{print}' >&2
  fail 'giant rust module entry files remain over 2000 LOC'
fi
require_grep 'giant_files_over_2000_loc: 0' .vac/registry/giant-file-split-ledger.yaml

# Perf/static TUI contracts.
require_file vac-rs/crates/surfaces/tui/src/startup_task_graph.rs
require_grep 'StartupTaskGraph' vac-rs/crates/surfaces/tui/src/startup_task_graph.rs
require_grep 'skeleton_first_frame_non_blocking' vac-rs/crates/surfaces/tui/src/startup_task_graph.rs
require_grep 'ttff_ms' .vac/registry/perf/tui-startup.yaml
require_grep 'interactive_ready_ms' .vac/registry/perf/tui-startup.yaml
require_grep 'tv_pending: \[\]' .vac/registry/perf/tui-benchmark-results.yaml
require_grep 'not_evaluated: \[\]' .vac/registry/perf/tui-benchmark-results.yaml

# Capability/readiness/debt ledgers must declare no static residuals.
require_grep 'known_static_residuals: \[\]' .vac/registry/validation-ledger.yaml
require_grep 'not_ready: \[\]' .vac/registry/capability-readiness.yaml
require_grep 'partial: \[\]' .vac/registry/capability-readiness.yaml
require_grep 'server_coupling_unclassified: \[\]' .vac/registry/capability-readiness.yaml
require_grep 'unmanaged_todo: 0' .vac/registry/technical-debt-markers.yaml
require_grep 'unmanaged_dead_code: 0' .vac/registry/technical-debt-markers.yaml
require_grep 'hot_path_todo: 0' .vac/registry/technical-debt-markers.yaml
require_grep 'tui_high_volume_unbounded_channels: 0' .vac/registry/unbounded-channel-allowlist.yaml
require_grep 'reachable_from_input_unwrap: forbidden' .vac/registry/panic-risk-governance.yaml

# Existing static slices still need to remain satisfied without cargo side effects.
VAC_STATIC_ONLY=1 bash scripts/check-vac-provider-prune-default-off-static.sh >/dev/null
bash scripts/check-vac-layering-migration-static.sh >/dev/null

echo "zero-residual static gate: PASS"
