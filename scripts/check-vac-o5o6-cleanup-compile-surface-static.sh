#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "cleanup compile-surface static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_dir() { [[ -d "$1" ]] || fail "missing dir: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() {
  local pattern="$1"; shift
  local paths=()
  for path in "$@"; do
    [[ -e "$path" ]] && paths+=("$path")
  done
  [[ ${#paths[@]} -eq 0 ]] || ! grep -R -qE "$pattern" "${paths[@]}" || fail "forbidden pattern found: $pattern"
}

python3 - <<'PY'
from pathlib import Path
import sys
import tomllib

root = Path('vac-rs')
manifest = root / 'Cargo.toml'
try:
    data = tomllib.loads(manifest.read_text())
except Exception as exc:
    print(f'FAIL: cannot parse {manifest}: {exc}', file=sys.stderr)
    raise SystemExit(1)
errors: list[str] = []
workspace = data.get('workspace', {})
workspace_deps = workspace.get('dependencies', {})

for member in workspace.get('members', []):
    if '*' in member:
        matches = sorted(root.glob(member))
        # Cargo member globs are allowed to be empty while layer directories are
        # introduced before physical crate moves. Non-empty matches must be crates.
        for match in matches:
            if not (match / 'Cargo.toml').is_file():
                # Directories such as README-only layer placeholders are not workspace crates.
                continue
        continue
    cargo = root / member / 'Cargo.toml'
    if not cargo.is_file():
        errors.append(f'missing workspace member manifest: {member}/Cargo.toml')

for dep_name, spec in workspace_deps.items():
    if isinstance(spec, dict) and 'path' in spec:
        dep_path = root / spec['path']
        if not dep_path.exists():
            errors.append(f'missing workspace dependency path for {dep_name}: {spec["path"]}')

for cargo in sorted(root.rglob('Cargo.toml')):
    try:
        package = tomllib.loads(cargo.read_text())
    except Exception as exc:
        errors.append(f'cannot parse {cargo}: {exc}')
        continue
    for section_name in ('dependencies', 'dev-dependencies', 'build-dependencies'):
        section = package.get(section_name, {})
        for dep_name, spec in section.items():
            if isinstance(spec, dict):
                if spec.get('workspace') is True and dep_name not in workspace_deps:
                    errors.append(f'{cargo}: {section_name}.{dep_name} inherits missing workspace dependency')
                if 'path' in spec and not (cargo.parent / spec['path']).exists():
                    errors.append(f'{cargo}: {section_name}.{dep_name} missing path {spec["path"]}')

if errors:
    print('cleanup compile-surface static gate: FAIL', file=sys.stderr)
    for err in errors:
        print(f'ERROR: {err}', file=sys.stderr)
    raise SystemExit(1)

print('workspace manifest path surface: PASS')
PY

# Locked local-agent surfaces must remain available after cleanup.
for d in \
  vac-rs/crates/providers/login \
  vac-rs/crates/providers/model-provider \
  vac-rs/crates/providers/models-manager \
  vac-rs/crates/providers/device-key \
  vac-rs/crates/providers/aws-auth \
  vac-rs/crates/providers/vac-api \
  vac-rs/crates/providers/provider-http \
  vac-rs/crates/providers/vac-client \
  vac-rs/crates/integrations/otel \
  vac-rs/crates/integrations/connectors \
  vac-rs/crates/integrations/vac-mcp \
  vac-rs/crates/integrations/rmcp-client \
  vac-rs/crates/capabilities/external-agent-migration \
  vac-rs/crates/capabilities/external-agent-sessions; do
  require_dir "$d"
done


[[ ! -d vac-rs/chatgpt ]] || fail "retired cloud-task vac-chatgpt crate returned"
[[ ! -d vac-rs/backend-client ]] || fail "retired backend-client crate returned"
[[ ! -d vac-rs/vac-backend-openapi-models ]] || fail "retired backend OpenAPI models crate returned"
reject_grep 'vac-chatgpt = \{ workspace = true \}' vac-rs/crates/surfaces/cli/Cargo.toml vac-rs/crates/surfaces/tui/Cargo.toml
reject_grep 'vac-backend-client = \{ workspace = true \}' vac-rs/crates/capabilities/memories/write/Cargo.toml
require_grep 'cloud task retrieval was removed' vac-rs/crates/surfaces/cli/src/apply_cli.rs
require_grep 'without backend-client rate-limit fetch' vac-rs/crates/capabilities/memories/write/src/guard.rs
require_grep 'ChatGPT account sign-in was removed' vac-rs/crates/surfaces/tui/src/onboarding/auth.rs

# TUI local cleanup stubs must remain source-reachable without their removed implementation trees.
require_file vac-rs/crates/surfaces/tui/src/lib.rs
require_file vac-rs/crates/surfaces/tui/src/chatwidget/realtime.rs
require_file vac-rs/crates/surfaces/tui/src/ide_context.rs
require_file vac-rs/crates/surfaces/tui/src/chatwidget/ide_context.rs
require_file vac-rs/crates/surfaces/tui/src/multi_agents.rs
require_grep '^mod ide_context;' vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs
require_grep '^mod multi_agents;' vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs
require_grep 'mod audio_device' vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs
require_grep 'Realtime voice/WebRTC was removed' vac-rs/crates/surfaces/tui/src/chatwidget/realtime.rs
require_grep 'IDE / VSCode context integration was removed' vac-rs/crates/surfaces/tui/src/ide_context.rs
require_grep 'IDE context integration was removed' vac-rs/crates/surfaces/tui/src/chatwidget/ide_context.rs
require_grep 'removed multi-agent collaboration UI' vac-rs/crates/surfaces/tui/src/multi_agents.rs
reject_grep 'use vac_realtime_webrtc' vac-rs/crates/surfaces/tui/src/chatwidget/realtime.rs
[[ ! -d vac-rs/crates/surfaces/tui/src/ide_context ]] || fail "removed IDE IPC implementation directory returned"
[[ ! -f vac-rs/crates/surfaces/tui/src/audio_device.rs ]] || fail "removed audio implementation returned as source file"
[[ ! -f vac-rs/crates/surfaces/tui/src/voice.rs ]] || fail "removed voice implementation returned as source file"

# Connectors Path A is local MCP mention metadata only; cloud directory references must stay absent.
require_file vac-rs/crates/integrations/connectors/src/accessible.rs
require_file vac-rs/crates/integrations/connectors/src/merge.rs
require_file vac-rs/crates/integrations/connectors/src/metadata.rs
require_grep 'ChatGPT Apps cloud directory surface was removed' vac-rs/crates/integrations/connectors/src/lib.rs
require_grep 'vac://mcp-connectors' vac-rs/crates/integrations/connectors/src/lib.rs
reject_grep '/connectors/directory/list' vac-rs/core/src vac-rs/crates/capabilities/docs/src/core_migrated vac-rs/crates/capabilities/ownership/src/core_migrated vac-rs/chatgpt/src vac-rs/crates/integrations/connectors/src
reject_grep 'chatgpt.com/apps' vac-rs/core/src vac-rs/crates/capabilities/docs/src/core_migrated vac-rs/crates/capabilities/ownership/src/core_migrated vac-rs/chatgpt/src vac-rs/crates/integrations/connectors/src vac-rs/crates/surfaces/tui/src

# Remote thread-store RPC stays fail-closed through a local shim.
require_file vac-rs/crates/foundation/thread-store/src/lib.rs
require_file vac-rs/crates/foundation/thread-store/src/remote_disabled.rs
require_grep '^mod remote_disabled;' vac-rs/crates/foundation/thread-store/src/lib.rs
require_grep 'pub use remote_disabled::RemoteThreadStore;' vac-rs/crates/foundation/thread-store/src/lib.rs
require_grep 'removed remote thread-store RPC client' vac-rs/crates/foundation/thread-store/src/remote_disabled.rs
[[ ! -d vac-rs/crates/foundation/thread-store/src/remote ]] || fail "removed remote thread-store RPC directory returned"

# Compile-sensitive docs/evidence for this slice must be present and explicit about TV-Pending.
require_file docs/monolith-quality/O5O6_CLEANUP_COMPILE_SURFACE_STATIC_REPORT.md
require_file .vac/registry/evidence/evidence.2026-06-01-o5o6-cleanup-compile-surface.yaml
require_file .vac/registry/trajectory/o5o6-cleanup-compile-surface.yaml
require_grep 'cargo_status: TV-Pending' .vac/registry/evidence/evidence.2026-06-01-o5o6-cleanup-compile-surface.yaml
require_grep 'cleanup_compile_surface_2026_06_01:' .vac/registry/o5-o6-completion-state.yaml
require_grep 'cleanup_compile_surface_2026_06_01:' .vac/registry/o5-o6-monolith-quality-state.yaml

printf 'cleanup compile-surface static gate: PASS\n'
