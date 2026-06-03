#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "cloud coupling isolation static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() { ! grep -qE "$1" "$2" || fail "forbidden pattern in $2: $1"; }

require_file vac-rs/crates/capabilities/identity/src/core_migrated/cloud_account_disabled.rs
require_grep 'legacy ChatGPT-account backend integration is disabled' vac-rs/crates/capabilities/identity/src/core_migrated/cloud_account_disabled.rs

# Runtime production paths must not synthesize cloud/backend URLs from chatgpt_base_url.
reject_grep 'chatgpt_base_url\.trim_end_matches' vac-rs/crates/capabilities/sessions/src/core_migrated/session/session.rs
reject_grep 'chatgpt_base_url\.trim_end_matches' vac-rs/crates/capabilities/sessions/src/core_migrated/arc_monitor.rs
reject_grep 'chatgpt_base_url\.trim_end_matches' vac-rs/crates/capabilities/docs/src/core_migrated/mcp_vastar_file.rs
reject_grep 'upload_local_file' vac-rs/crates/capabilities/docs/src/core_migrated/mcp_vastar_file.rs
python3 - <<'PY_PROD'
from pathlib import Path
text = Path('vac-rs/crates/capabilities/docs/src/core_migrated/mcp_vastar_file.rs').read_text().split('#[cfg(test)]', 1)[0]
for forbidden in ('chatgpt-account-id', 'backend-api/files', 'upload_local_file', 'chatgpt_base_url.trim_end_matches'):
    if forbidden in text:
        raise SystemExit(f'production mcp_vastar_file still has legacy cloud upload coupling: {forbidden}')
print('mcp_vastar_file production cloud upload coupling: PASS')
PY_PROD

# Local builds keep explicit local override safety monitoring only; no implicit backend fallback.
require_grep 'VAC_ARC_MONITOR_ENDPOINT_OVERRIDE' vac-rs/crates/capabilities/sessions/src/core_migrated/arc_monitor.rs
require_grep 'return ArcMonitorOutcome::Ok' vac-rs/crates/capabilities/sessions/src/core_migrated/arc_monitor.rs
require_grep 'local/owned service' vac-rs/crates/capabilities/sessions/src/core_migrated/arc_monitor.rs

# Session analytics defaults fail closed instead of opening a legacy backend client.
require_grep 'analytics_events_client.unwrap_or_else\(AnalyticsEventsClient::disabled\)' vac-rs/crates/capabilities/sessions/src/core_migrated/session/session.rs
require_grep 'must not create an implicit legacy' vac-rs/crates/capabilities/sessions/src/core_migrated/session/session.rs

# Cloud VAC Apps file upload is explicitly disabled in local agent runtime.
require_grep 'disabled_feature_message' vac-rs/crates/capabilities/docs/src/core_migrated/mcp_vastar_file.rs
require_grep 'VAC Apps cloud file upload' vac-rs/crates/capabilities/docs/src/core_migrated/mcp_vastar_file.rs

# User-visible install/app links should point to Vastar-owned docs, not chatgpt.com app surfaces.
reject_grep 'chatgpt\.com/vac' vac-rs/crates/surfaces/tui/src/update_action.rs
reject_grep 'chatgpt\.com/vac' vac-rs/crates/surfaces/tui/src/tooltips.rs
require_grep 'developers\.vastar\.com/vac' vac-rs/crates/surfaces/tui/src/update_action.rs
require_grep 'developers\.vastar\.com/vac' vac-rs/crates/surfaces/tui/src/tooltips.rs

printf 'cloud coupling isolation static gate: PASS\n'
