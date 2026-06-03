#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "architecture extraction static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_dir() { [[ -d "$1" ]] || fail "missing dir: $1"; }
require_absent() { [[ ! -e "$1" ]] || fail "retired/relocated path still exists: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() { ! grep -R -qE "$1" "${@:2}" || fail "forbidden pattern found: $1"; }

# S-01: control-plane must be a physical source-of-record crate, not a #[path] façade into vac-core.
require_dir vac-rs/crates/control-plane/control-plane/src/control_plane
require_dir vac-rs/crates/control-plane/control-plane/src/local_runtime
require_file vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
require_file vac-rs/crates/control-plane/control-plane/src/local_runtime/mod.rs
require_absent vac-rs/core/src/control_plane
require_absent vac-rs/core/src/local_runtime
require_grep '^pub mod control_plane;' vac-rs/crates/control-plane/control-plane/src/lib.rs
require_grep '^pub mod local_runtime;' vac-rs/crates/control-plane/control-plane/src/lib.rs
reject_grep '#\[path = "\.\./\.\./core/src/(control_plane|local_runtime)' vac-rs/crates/control-plane/control-plane/src/lib.rs
require_grep 'pub use vac_control_plane::control_plane;' vac-rs/core/src/lib.rs
require_grep 'pub use vac_control_plane::local_runtime;' vac-rs/core/src/lib.rs

# S-02: provider-http must own provider-generic HTTP transport source, while vac-api is compatibility-only.
require_dir vac-rs/crates/providers/provider-http/src/endpoint
require_dir vac-rs/crates/providers/provider-http/src/requests
require_dir vac-rs/crates/providers/provider-http/src/sse
require_file vac-rs/crates/providers/provider-http/src/auth.rs
require_file vac-rs/crates/providers/provider-http/src/provider.rs
require_file vac-rs/crates/providers/provider-http/src/files.rs
require_file vac-rs/crates/providers/provider-http/src/endpoint/responses.rs
require_file vac-rs/crates/providers/provider-http/src/requests/headers.rs
require_grep '^pub\(crate\) mod endpoint;' vac-rs/crates/providers/provider-http/src/lib.rs
require_grep '^pub use crate::endpoint::ResponsesClient;' vac-rs/crates/providers/provider-http/src/lib.rs
require_grep '^name = "vac-provider-http"' vac-rs/crates/providers/provider-http/Cargo.toml
require_grep 'vac-provider-http' vac-rs/crates/providers/vac-api/Cargo.toml
require_grep '^pub use vac_provider_http::\*;' vac-rs/crates/providers/vac-api/src/lib.rs
reject_grep 'pub use vac_api::\*;' vac-rs/crates/providers/provider-http/src/lib.rs

# vac-api should no longer duplicate the transport implementation.
python3 - <<'PY'
from pathlib import Path
files=[p for p in Path('vac-rs/crates/providers/vac-api/src').rglob('*.rs')]
if [p.name for p in files] != ['lib.rs']:
    raise SystemExit(f'vac-api still has implementation files: {[str(p) for p in files]}')
print('vac-api compatibility surface: PASS')
PY

# Cargo graph/manifests must reflect the physical ownership without requiring cargo execution.
python3 - <<'PY'
from pathlib import Path
import tomllib
manifest=tomllib.loads(Path('vac-rs/Cargo.toml').read_text())
workspace_deps=manifest.get('workspace', {}).get('dependencies', {})
errors=[]
for dep in ('vac-control-plane','vac-provider-http','vac-api','vac-client'):
    if dep not in workspace_deps:
        errors.append(f'missing workspace dependency: {dep}')
provider=tomllib.loads(Path('vac-rs/crates/providers/provider-http/Cargo.toml').read_text())
api=tomllib.loads(Path('vac-rs/crates/providers/vac-api/Cargo.toml').read_text())
client=tomllib.loads(Path('vac-rs/crates/providers/vac-client/Cargo.toml').read_text())
if provider['package']['name'] != 'vac-provider-http':
    errors.append('provider-http package name mismatch')
if 'vac-api' in provider.get('dependencies', {}):
    errors.append('provider-http must not depend on vac-api')
if 'vac-provider-http' not in api.get('dependencies', {}):
    errors.append('vac-api must be compatibility re-export over vac-provider-http')
if 'vac-provider-http' not in client.get('dependencies', {}):
    errors.append('vac-client must consume vac-provider-http source-of-truth')
api_files=[p.name for p in Path('vac-rs/crates/providers/vac-api/src').rglob('*.rs')]
if api_files != ['lib.rs']:
    errors.append(f'vac-api still has implementation files: {api_files}')
if errors:
    for err in errors:
        print(f'ERROR: {err}', file=__import__('sys').stderr)
    raise SystemExit(1)
print('Cargo manifest physical extraction: PASS')
PY

printf 'architecture extraction static gate: PASS\n'
