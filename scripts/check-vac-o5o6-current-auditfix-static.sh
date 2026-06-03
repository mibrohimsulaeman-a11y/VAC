#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "current auditfix static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() { ! grep -qE "$1" "$2" || fail "forbidden pattern in $2: $1"; }

# C-05: the two explicitly flagged giant files must now be dispatchers backed by bounded shards.
require_file vac-rs/crates/foundation/runtime-protocol/src/protocol/v2.rs
require_grep 'include!\("v2_parts/v2_part_000\.rs"\)' vac-rs/crates/foundation/runtime-protocol/src/protocol/v2.rs
python3 - <<'PY_V2'
from pathlib import Path
v2 = Path('vac-rs/crates/foundation/runtime-protocol/src/protocol/v2.rs')
if len(v2.read_text().splitlines()) > 80:
    raise SystemExit(f'{v2}: dispatcher too large')
parts = sorted(Path('vac-rs/crates/foundation/runtime-protocol/src/protocol/v2_parts').glob('v2_part_*.rs'))
if len(parts) < 2:
    raise SystemExit('runtime-protocol v2_parts missing split shards')
violations = [(p.as_posix(), sum(1 for _ in p.open(errors='ignore'))) for p in parts if sum(1 for _ in p.open(errors='ignore')) > 1500]
if violations:
    for path, lines in violations:
        print(f'{path}: {lines} lines > 1500', file=__import__('sys').stderr)
    raise SystemExit(1)
print(f'runtime-protocol v2 dispatcher: PASS shards={len(parts)}')
PY_V2

require_file vac-rs/crates/surfaces/tui/src/bottom_pane/chat_composer/split_002_footerflash.rs
require_file vac-rs/crates/surfaces/tui/src/bottom_pane/chat_composer/split_002_footerflash_tests.rs
require_grep 'include!\("footer_flash_parts/footer_flash_000\.rs"\)' vac-rs/crates/surfaces/tui/src/bottom_pane/chat_composer/split_002_footerflash.rs
require_file vac-rs/crates/surfaces/tui/src/bottom_pane/chat_composer/split_002_footerflash_tests.rs
require_grep 'include!\("footer_flash_group_000_preamble\.rs"\)' vac-rs/crates/surfaces/tui/src/bottom_pane/chat_composer/footer_flash_parts/footer_flash_000.rs
python3 - <<'PY_FOOTER'
from pathlib import Path
root = Path('vac-rs/crates/surfaces/tui/src/bottom_pane/chat_composer/footer_flash_parts')
dispatcher = root / 'footer_flash_000.rs'
if not dispatcher.exists():
    raise SystemExit('footer flash dispatcher missing')
lines = dispatcher.read_text(errors='ignore').splitlines()
code_lines = [line.strip() for line in lines if line.strip() and not line.strip().startswith('//')]
if len(lines) > 80:
    raise SystemExit(f'{dispatcher}: dispatcher too large ({len(lines)} lines)')
missing=[]
for line in code_lines:
    if not line.startswith('include!("') or not line.endswith('.rs");'):
        raise SystemExit(f'{dispatcher}: non-dispatcher code line `{line}`')
    name = line.split('"', 2)[1]
    if not (root / name).exists():
        missing.append(name)
if missing:
    raise SystemExit(f'footer flash dispatcher references missing groups: {missing}')
groups = sorted(root.glob('footer_flash_group_*.rs'))
if len(groups) < 8:
    raise SystemExit(f'footer flash balanced group shards incomplete: {len(groups)}')
print(f'footer flash compile-balanced dispatcher: PASS groups={len(groups)}')
PY_FOOTER

# C-07: production core+tui must not use unbounded channels. Inline cfg(test) fixtures are excluded.
python3 - <<'PY_CHANNELS'
from pathlib import Path
TERMS = ('unbounded_channel', 'async_channel::unbounded', 'mpsc::unbounded', 'UnboundedReceiver', 'UnboundedSender')

def production_lines(path: Path):
    lines = path.read_text(errors='ignore').splitlines()
    pending_cfg_test = False
    skip_block = False
    depth = 0
    for i, line in enumerate(lines, 1):
        stripped = line.strip()
        if skip_block:
            depth += line.count('{') - line.count('}')
            if depth <= 0:
                skip_block = False
                depth = 0
            continue
        if pending_cfg_test:
            if not stripped or stripped.startswith('#'):
                continue
            if '{' in line:
                skip_block = True
                depth = line.count('{') - line.count('}')
                if depth <= 0:
                    skip_block = False
                    depth = 0
                pending_cfg_test = False
                continue
            pending_cfg_test = False
            continue
        if stripped.startswith('#[cfg(test)]'):
            pending_cfg_test = True
            continue
        if stripped.startswith('#[cfg_attr(test'):
            continue
        yield i, line

violations=[]
for root in (Path('vac-rs/core/src'), Path('vac-rs/crates/capabilities'), Path('vac-rs/crates/surfaces/tui/src')):
    for path in sorted(root.rglob('*.rs')):
        rel=path.as_posix()
        if '/tests/' in rel or rel.endswith('_tests.rs') or rel.endswith('/tests.rs') or rel.endswith('_test.rs') or 'test_support' in rel:
            continue
        for line_no, line in production_lines(path):
            if any(term in line for term in TERMS):
                violations.append(f'{rel}:{line_no}: {line.strip()}')
if violations:
    print('production unbounded channel violations:', file=__import__('sys').stderr)
    for violation in violations:
        print(violation, file=__import__('sys').stderr)
    raise SystemExit(1)
print('production bounded channel surface: PASS')
PY_CHANNELS

# C-08: production TUI must not panic through unwrap/expect; remaining panic! sites are tracked as invariants.
python3 - <<'PY_PANIC'
from pathlib import Path

def production_lines(path: Path):
    lines = path.read_text(errors='ignore').splitlines()
    pending_cfg_test = False
    skip_block = False
    depth = 0
    for i, line in enumerate(lines, 1):
        stripped = line.strip()
        if skip_block:
            depth += line.count('{') - line.count('}')
            if depth <= 0:
                skip_block = False
                depth = 0
            continue
        if pending_cfg_test:
            if not stripped or stripped.startswith('#'):
                continue
            if '{' in line:
                skip_block = True
                depth = line.count('{') - line.count('}')
                if depth <= 0:
                    skip_block = False
                    depth = 0
                pending_cfg_test = False
                continue
            pending_cfg_test = False
            continue
        if stripped.startswith('#[cfg(test)]'):
            pending_cfg_test = True
            continue
        if stripped.startswith('#[cfg_attr(test'):
            continue
        yield i, line

forbidden = ('.unwrap(', '.expect(')
violations=[]
for root in (Path('vac-rs/core/src'), Path('vac-rs/crates/capabilities'), Path('vac-rs/crates/surfaces/tui/src')):
    for path in sorted(root.rglob('*.rs')):
        rel=path.as_posix()
        if '/tests/' in rel or rel.endswith('_tests.rs') or rel.endswith('/tests.rs') or rel.endswith('_test.rs') or 'test_support' in rel:
            continue
        for line_no, line in production_lines(path):
            if any(term in line for term in forbidden):
                violations.append(f'{rel}:{line_no}: {line.strip()}')
if violations:
    print('production unwrap/expect violations:', file=__import__('sys').stderr)
    for violation in violations:
        print(violation, file=__import__('sys').stderr)
    raise SystemExit(1)
print('production unwrap/expect surface: PASS')
PY_PANIC

# C-09: legacy ChatGPT account/cloud paths must be explicitly opt-in/fail-closed.
require_file vac-rs/crates/providers/model-provider-info/src/lib.rs
require_file vac-rs/crates/providers/login/src/server.rs
require_file vac-rs/crates/capabilities/core-plugins/src/startup_sync.rs
require_file vac-rs/crates/capabilities/core-plugins/src/remote.rs
require_file vac-rs/crates/capabilities/core-skills/src/remote.rs
require_grep 'VAC_ENABLE_LEGACY_CHATGPT_ACCOUNT_BACKEND' vac-rs/crates/providers/model-provider-info/src/lib.rs
require_grep 'VAC_ENABLE_LEGACY_CHATGPT_ACCOUNT_LOGIN' vac-rs/crates/providers/login/src/server.rs
require_grep 'legacy ChatGPT account login is disabled' vac-rs/crates/providers/login/src/server.rs
require_grep 'VAC_ENABLE_LEGACY_CHATGPT_CURATED_PLUGINS_BACKUP' vac-rs/crates/capabilities/core-plugins/src/startup_sync.rs
require_grep 'legacy ChatGPT curated plugin backup archive is disabled' vac-rs/crates/capabilities/core-plugins/src/startup_sync.rs
require_grep 'VAC_ENABLE_LEGACY_CHATGPT_CLOUD_PLUGINS' vac-rs/crates/capabilities/core-plugins/src/remote.rs
require_grep 'VAC_ENABLE_LEGACY_CHATGPT_REMOTE_SKILLS' vac-rs/crates/capabilities/core-skills/src/remote.rs

# C-03/C-04: full cargo-sensitive relocation remains represented as a machine-checkable target.
require_file .vac/registry/architecture-layer-map.yaml
require_grep 'layered_workspace_target' .vac/registry/architecture-layer-map.yaml
require_grep 'crates/foundation/runtime-protocol' .vac/registry/architecture-layer-map.yaml
require_grep 'capability_extraction_target' .vac/registry/architecture-layer-map.yaml

printf 'current auditfix static gate: PASS\n'
