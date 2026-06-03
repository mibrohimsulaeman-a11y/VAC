#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail(){ echo "layering migration gate: $*" >&2; exit 1; }
require(){ [[ -e "$1" ]] || fail "missing $1"; }
require .vac/registry/layering-migration-ledger.yaml
require .vac/registry/layering-status.yaml
require vac-rs/crates/layer-map.yaml
for layer in control-plane providers foundation runtime surfaces integrations capabilities; do
  require "vac-rs/crates/$layer"
  grep -q "  $layer:" vac-rs/crates/layer-map.yaml || fail "layer $layer missing from layer-map"
done
require vac-rs/crates/control-plane/control-plane/Cargo.toml
require vac-rs/crates/providers/provider-http/Cargo.toml
require vac-rs/crates/providers/vac-client/Cargo.toml
require vac-rs/crates/foundation/protocol/Cargo.toml
require vac-rs/crates/foundation/runtime-protocol/Cargo.toml
require vac-rs/crates/runtime/exec/exec/Cargo.toml
require vac-rs/crates/runtime/sandbox/sandboxing/Cargo.toml
require vac-rs/crates/surfaces/tui/Cargo.toml
require vac-rs/crates/surfaces/cli/Cargo.toml
require vac-rs/crates/integrations/vac-mcp/Cargo.toml
require vac-rs/crates/capabilities/core-plugins/Cargo.toml
[[ ! -e vac-rs/control-plane/Cargo.toml ]] || fail "control-plane still flat"
[[ ! -e vac-rs/provider-http/Cargo.toml ]] || fail "provider-http still flat"
[[ ! -e vac-rs/tui/Cargo.toml ]] || fail "tui still flat"
python3 - <<'PY'
from pathlib import Path
import re, sys
bad=[]
for p in Path('vac-rs').rglob('Cargo.toml'):
    for line in p.read_text(errors='ignore').splitlines():
        if line.strip().startswith('#'):
            continue
        for m in re.finditer(r'path\s*=\s*"([^"]+)"', line):
            target=(p.parent/m.group(1)).resolve()
            if not target.exists():
                bad.append(f'{p}:{m.group(1)} -> {target}')
if bad:
    print('missing Cargo path dependencies:', file=sys.stderr)
    print('\n'.join(bad), file=sys.stderr)
    raise SystemExit(1)
PY
echo "layering migration gate: PASS"
