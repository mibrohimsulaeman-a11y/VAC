#!/usr/bin/env bash
set -euo pipefail
fail() { echo "tui local-tool cleanup static gate: $*" >&2; exit 1; }
reject_grep_existing() {
  local pattern="$1"; shift
  local paths=()
  for path in "$@"; do
    [[ -e "$path" ]] && paths+=("$path")
  done
  [[ ${#paths[@]} -eq 0 ]] || ! grep -R -q "$pattern" "${paths[@]}" || fail "forbidden pattern still referenced: $pattern"
}

# Scope decision: keep login/auth/providers, capability dashboard, external-agent import, docs/scripts,
# and MCP @mention helpers. Remove realtime/voice, VSCode IDE IPC, cloud ChatGPT Apps directory,
# dangling npm workspace entries, and remote thread-store RPC surface.

[[ ! -d vac-rs/realtime-webrtc ]] || fail "realtime-webrtc crate must be removed from local coding build"
! grep -q '"realtime-webrtc"' vac-rs/Cargo.toml || fail "realtime-webrtc workspace member still present"
! grep -q 'vac-realtime-webrtc' vac-rs/Cargo.toml vac-rs/crates/surfaces/tui/Cargo.toml || fail "vac-realtime-webrtc dependency still present"
! grep -q '^cpal =' vac-rs/Cargo.toml vac-rs/crates/surfaces/tui/Cargo.toml || fail "cpal voice dependency still present"
[[ ! -f vac-rs/crates/surfaces/tui/src/audio_device.rs ]] || fail "audio device implementation file must be removed"
[[ ! -f vac-rs/crates/surfaces/tui/src/voice.rs ]] || fail "voice implementation file must be removed"
! grep -R -q 'use vac_realtime_webrtc' vac-rs/crates/surfaces/tui/src || fail "TUI still imports realtime WebRTC crate"
grep -q 'voice input was removed from the local coding tool build' vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs || fail "TUI must expose explicit voice removed stub"
grep -q 'Realtime voice/WebRTC was removed' vac-rs/crates/surfaces/tui/src/chatwidget/realtime.rs || fail "chatwidget realtime path must degrade as removed"

[[ ! -d vac-rs/crates/surfaces/tui/src/ide_context ]] || fail "IDE IPC subdirectory must be removed"
[[ -f vac-rs/crates/surfaces/tui/src/ide_context.rs ]] || fail "IDE compatibility stub missing"
grep -q 'IDE / VSCode context integration was removed' vac-rs/crates/surfaces/tui/src/ide_context.rs || fail "IDE context must be local-tool disabled stub"
grep -q 'IDE context integration was removed' vac-rs/crates/surfaces/tui/src/chatwidget/ide_context.rs || fail "chatwidget IDE command must degrade locally"

[[ -f vac-rs/crates/surfaces/tui/src/multi_agents.rs ]] || fail "multi-agent compatibility stub missing"
grep -q 'removed multi-agent collaboration UI' vac-rs/crates/surfaces/tui/src/multi_agents.rs || fail "multi-agent module must be a local-tool disabled stub"
! find vac-rs/crates/surfaces/tui/src -path '*snapshots*' -type f | grep -Eq '(^|[/_])(multi_agents|realtime|ide_context)([._/]|$)' || fail "stale multi-agent/realtime/ide snapshots remain"

! grep -q 'sdk/typescript' pnpm-workspace.yaml pnpm-lock.yaml || fail "dangling sdk/typescript npm workspace entry remains"
! grep -q 'responses-api-proxy/npm' pnpm-workspace.yaml pnpm-lock.yaml || fail "dangling responses-api-proxy npm workspace entry remains"

# Path A for connectors: keep MCP @mention helpers, remove cloud directory/orchestration.
[[ -f vac-rs/crates/integrations/connectors/src/accessible.rs ]] || fail "MCP accessible connector helper removed unexpectedly"
[[ -f vac-rs/crates/integrations/connectors/src/merge.rs ]] || fail "MCP connector merge helper removed unexpectedly"
[[ -f vac-rs/crates/integrations/connectors/src/metadata.rs ]] || fail "MCP connector metadata helper removed unexpectedly"
grep -q 'ChatGPT Apps cloud directory surface was removed' vac-rs/crates/integrations/connectors/src/lib.rs || fail "connectors crate must document local-only helper scope"
grep -q 'vac://mcp-connectors' vac-rs/crates/integrations/connectors/src/lib.rs || fail "connectors install URL must be local, not chatgpt.com/apps"
reject_grep_existing '/connectors/directory/list' vac-rs/core/src vac-rs/crates/capabilities/docs/src/core_migrated vac-rs/crates/capabilities/ownership/src/core_migrated vac-rs/chatgpt/src vac-rs/crates/integrations/connectors/src
reject_grep_existing 'chatgpt.com/apps' vac-rs/core/src vac-rs/crates/capabilities/docs/src/core_migrated vac-rs/crates/capabilities/ownership/src/core_migrated vac-rs/chatgpt/src vac-rs/crates/integrations/connectors/src vac-rs/crates/surfaces/tui/src
! grep -R -q 'with_vac_apps_mcp(mcp_servers' vac-rs/core/src vac-rs/crates/capabilities || fail "core session still injects cloud Apps MCP server"
grep -q 'do not inject the cloud ChatGPT Apps MCP directory server' vac-rs/crates/integrations/vac-mcp/src/mcp/mod.rs || fail "vac-mcp compatibility helper must be explicit no-op"

[[ ! -d vac-rs/crates/foundation/thread-store/src/remote ]] || fail "remote thread-store RPC directory must be removed"
[[ -f vac-rs/crates/foundation/thread-store/src/remote_disabled.rs ]] || fail "remote thread-store fail-closed shim missing"
grep -q 'removed remote thread-store RPC client' vac-rs/crates/foundation/thread-store/src/remote_disabled.rs || fail "remote thread-store shim must explain local-only removal"

# Safety color consistency.
grep -q '"DESTRUCTIVE".red().bold()' vac-rs/crates/surfaces/tui/src/bottom_pane/approval_overlay.rs.inc || fail "DESTRUCTIVE approval label must use red Danger styling"
! grep -q '"DESTRUCTIVE".yellow().bold()' vac-rs/crates/surfaces/tui/src/bottom_pane/approval_overlay.rs.inc || fail "DESTRUCTIVE approval label still yellow"

# Keep explicitly locked areas.
[[ -d vac-rs/crates/providers/login && -d vac-rs/crates/providers/model-provider && -d vac-rs/crates/providers/models-manager ]] || fail "locked auth/provider crates were removed"
[[ ! -d vac-rs/chatgpt ]] || fail "retired cloud-task vac-chatgpt crate returned"
[[ -f vac-rs/crates/surfaces/tui/src/capability_dashboard.rs && -f vac-rs/crates/surfaces/tui/src/operator_console.rs ]] || fail "locked capability dashboard/operator console surface missing"
[[ -d vac-rs/crates/capabilities/external-agent-migration && -d vac-rs/crates/capabilities/external-agent-sessions ]] || fail "locked external-agent import crates missing"
[[ -d docs && -d scripts ]] || fail "locked docs/scripts must remain"

echo "tui local-tool cleanup static gate: PASS"
