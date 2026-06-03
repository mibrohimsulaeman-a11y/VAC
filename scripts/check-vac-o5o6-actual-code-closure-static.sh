#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail(){ echo "actual-code closure gate: $*" >&2; exit 1; }
require_file(){ [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep(){ grep -qE "$1" "$2" || fail "missing pattern $1 in $2"; }
forbid_grep(){ if grep -qE "$1" "$2"; then fail "forbidden pattern $1 in $2"; fi }
require_file docs/monolith-quality/O5O6_CONTINUATION_TRUTH_LEDGER.md
require_grep 'computed plan discarded != runtime behavior' docs/monolith-quality/O5O6_CONTINUATION_TRUTH_LEDGER.md
forbid_grep 'ast_exact_static' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs
require_grep 'RiskDetectionMethod::AstExact.as_str' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs
require_grep 'pub fn validate\(value: &str\)' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_risk_policy.rs
require_grep 'pub struct RustSynAnchorResolver' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs
require_grep 'syn::parse_file' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs
require_grep 'legacy_line_heuristic_item_candidates' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs
require_grep 'rust_syn_item_candidates' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs
forbid_grep 'parse_rust_item_declaration' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs
require_grep 'surface\.planned_visible' vac-rs/crates/control-plane/control-plane/src/control_plane/surface_manifest.rs
require_grep 'visibility_contract' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
require_grep 'fn render_windowed_transcript_cells' vac-rs/crates/surfaces/tui/src/chatwidget/split_024_thread_id.rs
forbid_grep 'let _window =' vac-rs/crates/surfaces/tui/src/chatwidget/split_024_thread_id.rs
require_grep 'transcript_cell_count_for_windowing' vac-rs/crates/surfaces/tui/src/chatwidget/split_024_thread_id.rs
require_file vac-rs/crates/surfaces/tui/src/auth_state_banner.rs
require_grep 'AuthStateBanner::AuthUnavailable' vac-rs/crates/surfaces/tui/src/onboarding/onboarding_screen.rs
require_grep 'startup-exit.yaml' vac-rs/crates/surfaces/tui/src/app.rs
require_grep 'struct ShimmerFrameMetrics' vac-rs/crates/surfaces/tui/src/shimmer.rs
require_grep 'record_shimmer_frame' vac-rs/crates/surfaces/tui/src/shimmer.rs
forbid_grep 'hardening marker: shimmer frame metric' vac-rs/crates/surfaces/tui/src/shimmer.rs
require_grep 'fn contrast_ratio' vac-rs/crates/surfaces/tui/src/theme_picker.rs
require_grep 'enforce_contrast_for_custom_theme' vac-rs/crates/surfaces/tui/src/theme_picker.rs
forbid_grep 'contrast validation fallback' vac-rs/crates/surfaces/tui/src/theme_picker.rs
require_grep 'pub enum CommandAvailability' vac-rs/crates/surfaces/tui/src/slash_command.rs
require_grep 'palette_badge' vac-rs/crates/surfaces/tui/src/slash_command.rs
require_grep 'CommandAvailability::FeatureOff' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_file .vac/registry/perf/tui-windowed-render.yaml
require_file .vac/registry/perf/tui-startup.yaml
require_file .vac/registry/perf/tui-shimmer.yaml
require_file .vac/registry/perf/tui-palette.yaml
require_file vac-rs/crates/providers/agent-identity/Cargo.toml
require_file vac-rs/crates/integrations/otel/Cargo.toml
[[ ! -d vac-rs/memories ]] || fail 'memories README-only stub still present'
require_grep 'crates/providers/agent-identity' vac-rs/Cargo.toml
require_grep 'crates/integrations/otel' vac-rs/Cargo.toml
require_file .vac/registry/capability-readiness-matrix.yaml
require_file .vac/registry/panic-risk-governance.yaml
require_file .vac/registry/unbounded-channel-allowlist.yaml
require_file docs/monolith-quality/O5O6_ACTUAL_CODE_CLOSURE_REPORT.md
VAC_STATIC_ONLY=1 bash scripts/bench-vac-tui-performance.sh >/dev/null
echo "actual-code closure gate: PASS"
