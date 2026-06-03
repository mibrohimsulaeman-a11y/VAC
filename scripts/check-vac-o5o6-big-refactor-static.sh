#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "big-refactor static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_dir() { [[ -d "$1" ]] || fail "missing dir: $1"; }
require_absent() { [[ ! -e "$1" ]] || fail "retired path still exists: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() {
  local pattern="$1"; shift
  local paths=()
  for path in "$@"; do
    [[ -e "$path" ]] && paths+=("$path")
  done
  [[ ${#paths[@]} -eq 0 ]] || ! grep -R -qE "$pattern" "${paths[@]}" || fail "forbidden pattern found: $pattern"
}

# F-021/F-022: cloud task and backend OpenAPI islands are retired from the active workspace graph.
require_absent vac-rs/chatgpt
require_absent vac-rs/backend-client
require_absent vac-rs/vac-backend-openapi-models
reject_grep 'vac-chatgpt|vac_chatgpt|vac-backend-client|vac_backend_client|vac-backend-openapi-models|vac_backend_openapi' \
  vac-rs/Cargo.toml \
  vac-rs/crates/surfaces/cli/Cargo.toml \
  vac-rs/crates/surfaces/tui/Cargo.toml \
  vac-rs/crates/capabilities/memories/write/Cargo.toml \
  vac-rs/crates/surfaces/cli/src \
  vac-rs/crates/surfaces/tui/src/chatwidget \
  vac-rs/crates/capabilities/memories/write/src \
  .vac/capabilities

# S-01/S-02 follow-up: physical control-plane/provider-http extraction must be real, not façade.
bash scripts/check-vac-o5o6-architecture-extraction-static.sh

bash scripts/check-vac-o5o6-cloud-coupling-isolation-static.sh
bash scripts/check-vac-o5o6-bounded-hotpath-static.sh

# F-021 local CLI compatibility: `vac apply` stays parse-compatible but fails closed.
require_file vac-rs/crates/surfaces/cli/src/apply_cli.rs
require_grep 'cloud task retrieval was removed' vac-rs/crates/surfaces/cli/src/apply_cli.rs
require_grep 'mod apply_cli;' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'crate::apply_cli' vac-rs/crates/surfaces/cli/src/main.rs

# F-022 memories/write is local-startup only, not backend OpenAPI rate-limit fetch.
require_file vac-rs/crates/capabilities/memories/write/src/guard.rs
require_grep 'without backend-client rate-limit fetch' vac-rs/crates/capabilities/memories/write/src/guard.rs
reject_grep 'BackendClient|vac_backend_client|vac-backend-client' vac-rs/crates/capabilities/memories/write/src vac-rs/crates/capabilities/memories/write/Cargo.toml

# F-024 account login is removed from active CLI/TUI selection paths.
require_grep 'ChatGPT account sign-in was removed' vac-rs/crates/surfaces/tui/src/onboarding/auth.rs
require_grep 'fn is_chatgpt_login_allowed\(&self\) -> bool' vac-rs/crates/surfaces/tui/src/onboarding/auth.rs
require_grep 'false' vac-rs/crates/surfaces/tui/src/onboarding/auth.rs
require_grep 'let highlighted_mode = SignInOption::ApiKey' vac-rs/crates/surfaces/tui/src/onboarding/onboarding_screen.rs
require_grep 'cfg_attr\(test, allow\(clippy::unwrap_used\)\)' vac-rs/crates/surfaces/tui/src/onboarding/auth.rs
python3 - <<'PY_UNWRAP'
from pathlib import Path
text = Path('vac-rs/crates/surfaces/tui/src/onboarding/auth.rs').read_text()
prod = text.split('#[cfg(test)]', 1)[0]
if '.unwrap()' in prod or '.expect(' in prod:
    raise SystemExit('production onboarding auth still contains unwrap/expect')
print('onboarding auth production unwrap surface: PASS')
PY_UNWRAP
reject_grep 'Chatgpt|DeviceCode' vac-rs/crates/surfaces/cli/src/auth_cli.rs

# Connector helper moved out of retired `chatgpt` crate while preserving local MCP @mention surface.
require_grep 'pub async fn list_all_connectors_with_options' vac-rs/crates/capabilities/ownership/src/core_migrated/connectors.rs
require_grep 'fn merge_connectors_with_accessible' vac-rs/crates/capabilities/ownership/src/core_migrated/connectors.rs
require_grep 'use vac_core::connectors;' vac-rs/crates/surfaces/tui/src/chatwidget/split_001_btreemap.rs

# F-007: apply-patch rejects relative parent traversal before filesystem writes.
require_grep 'resolve_relative_patch_path_within_cwd' vac-rs/crates/runtime/shell/apply-patch/src/lib.rs
require_grep 'patch path escapes working directory' vac-rs/crates/runtime/shell/apply-patch/src/lib.rs
require_grep 'test_apply_patch_rejects_relative_parent_escape' vac-rs/crates/runtime/shell/apply-patch/src/lib.rs

# F-009 targeted hot-path discipline: frame scheduler redraw channel is bounded/coalescing.
require_file vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs
require_grep 'FRAME_SCHEDULE_QUEUE_CAPACITY' vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs
require_grep 'mpsc::channel\(FRAME_SCHEDULE_QUEUE_CAPACITY\)' vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs
require_grep 'try_send' vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs
! grep -qE 'unbounded_channel|UnboundedSender|UnboundedReceiver' vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs || fail 'frame_requester still uses unbounded channel'

# Control-plane refactor is represented in VAC registry/contracts, not only Cargo files.
require_file .vac/registry/architecture-layer-map.yaml
require_grep 'vac-control-plane' .vac/registry/architecture-layer-map.yaml
require_grep 'vac-provider-http' .vac/registry/architecture-layer-map.yaml
require_file docs/architecture/VAC_CONTROL_PLANE_REFACTOR_REPORT.md
require_file docs/monolith-quality/O5O6_BIG_REFACTOR_STATIC_REPORT.md
require_file docs/monolith-quality/O5O6_STANDARD_CARGO_HYGIENE_GATE.md
require_file docs/monolith-quality/O5O6_TECHNICAL_DEBT_MARKER_TRACKING.md
require_file .vac/registry/technical-debt-markers.yaml
require_file .vac/registry/plans/plan.o5o6.big-refactor.yaml
require_file .vac/registry/evidence/evidence.2026-06-01-o5o6-big-refactor.yaml
require_file .vac/registry/trajectory/o5o6-big-refactor.yaml
require_grep 'big_refactor_2026_06_01:' .vac/registry/o5-o6-completion-state.yaml
require_grep 'big_refactor_2026_06_01:' .vac/registry/o5-o6-monolith-quality-state.yaml
require_grep 'TV-Pending' .vac/registry/evidence/evidence.2026-06-01-o5o6-big-refactor.yaml

printf 'big-refactor static gate: PASS\n'
