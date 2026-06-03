#!/usr/bin/env bash
# Optional live PTY smoke wrapper. It is intentionally non-destructive and defaults to contract-only
# in constrained sandboxes. Set VAC_TUI_ENABLE_LIVE_PTY=1 in a real terminal environment to extend it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

bash scripts/check-tui-pty-gate-contract.sh

if [[ "${VAC_TUI_ENABLE_LIVE_PTY:-0}" != "1" ]]; then
  printf 'live PTY smoke skipped: set VAC_TUI_ENABLE_LIVE_PTY=1 outside sandbox to run interactive capture\n'
  exit 0
fi

printf 'live PTY smoke placeholder: use cargo run -p vac-surface-tui -- /runtime in a real PTY and compare snapshots\n'
