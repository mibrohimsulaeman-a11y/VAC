#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail(){ echo "provider prune gate: $*" >&2; exit 1; }
require_grep(){ grep -qE "$1" "$2" || fail "missing pattern $1 in $2"; }
vc="vac-rs/crates/providers/vac-client"
ph="vac-rs/crates/providers/provider-http"
require_grep '^default = \[\]' "$vc/Cargo.toml"
require_grep '^provider-chatgpt = ' "$vc/Cargo.toml"
require_grep '^provider-realtime = ' "$vc/Cargo.toml"
require_grep '^provider-cloud-tests = ' "$vc/Cargo.toml"
require_grep 'required-features = \["provider-cloud-tests"\]' "$vc/Cargo.toml"
require_grep 'Legacy compatibility crate for provider HTTP transport' "$vc/src/lib.rs"
require_grep 'pub use vac_provider_http::\*;' "$vc/src/lib.rs"
require_grep '^default = \[\]' "$ph/Cargo.toml"
require_grep '^provider-chatgpt = \[\]' "$ph/Cargo.toml"
require_grep '^provider-realtime = \[' "$ph/Cargo.toml"
require_grep '^provider-cloud-tests = \["provider-chatgpt", "provider-realtime"\]' "$ph/Cargo.toml"
require_grep '^telemetry = \[\]' "$ph/Cargo.toml"
require_grep '^feedback = \[\]' "$ph/Cargo.toml"
require_grep '^telemetry-upload = \[\]' vac-rs/crates/providers/analytics/Cargo.toml
require_grep '^feedback-upload = \[\]' vac-rs/crates/providers/feedback/Cargo.toml
require_grep '^debug-cloud = \[\]' vac-rs/crates/capabilities/response-debug-context/Cargo.toml
python3 - <<'PY2'
from pathlib import Path
import re, sys
forbidden_default = re.compile(r'CYBER_VERIFY_URL|chatgpt\.com|backend-api|chatgpt-account-id|local_chatgpt_auth')
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
        violations.append(f'{path}: forbidden default provider endpoint')
    if legacy_auth.search(text) and not (is_test or path in allowed_legacy_paths):
        violations.append(f'{path}: legacy auth enum in default path')
if violations:
    print('ungated provider-default residuals:', file=sys.stderr)
    print('\n'.join(violations[:120]), file=sys.stderr)
    raise SystemExit(1)
PY2
echo "provider prune gate: PASS"
