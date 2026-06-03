#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "audit remainder static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() { ! grep -qE "$1" "$2" || fail "forbidden pattern in $2: $1"; }

# R-03: production core code must not hide reachable fallible behavior behind `.expect(...)`.
python3 - <<'PY_EXPECT'
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
        if stripped.startswith('#[cfg(test)]'):
            pending_cfg_test = True
            continue
        if stripped.startswith('#[cfg_attr(test'):
            continue
        yield i, line

violations = []
for path in sorted(Path('vac-rs/core/src').rglob('*.rs')):
    rel = path.as_posix()
    if '/tests/' in rel or rel.endswith('_tests.rs') or rel.endswith('/tests.rs') or rel.endswith('_test.rs') or 'test_support' in rel:
        continue
    for line_no, line in production_lines(path):
        if '.expect(' in line:
            violations.append(f'{rel}:{line_no}: {line.strip()}')
if violations:
    print('production .expect(...) violations:', file=__import__('sys').stderr)
    for violation in violations:
        print(violation, file=__import__('sys').stderr)
    raise SystemExit(1)
print('production core expect surface: PASS')
PY_EXPECT

# R-06/S-10: production core+tui channels must be bounded; test-only fixtures may keep unbounded channels.
python3 - <<'PY_CHANNEL'
from pathlib import Path
TERMS = ('unbounded_channel', 'async_channel::unbounded', 'mpsc::unbounded', 'UnboundedReceiver', 'UnboundedSender')

def production_lines(path: Path):
    lines = path.read_text(errors='ignore').splitlines()
    pending_cfg_test = False
    skip_block = False
    depth = 0
    line_cfg_test = False
    for i, line in enumerate(lines, 1):
        stripped = line.strip()
        if skip_block:
            depth += line.count('{') - line.count('}')
            if depth <= 0:
                skip_block = False
                depth = 0
            continue
        if pending_cfg_test:
            if not stripped:
                continue
            if stripped.startswith('#'):
                line_cfg_test = line_cfg_test or stripped.startswith('#[cfg(test)]')
                continue
            if '{' in line:
                skip_block = True
                depth = line.count('{') - line.count('}')
                if depth <= 0:
                    skip_block = False
                    depth = 0
                pending_cfg_test = False
                line_cfg_test = False
                continue
            # test-only single item/field/use line; skip it.
            pending_cfg_test = False
            line_cfg_test = False
            continue
        if stripped.startswith('#[cfg(test)]'):
            pending_cfg_test = True
            line_cfg_test = True
            continue
        if stripped.startswith('#[cfg_attr(test'):
            continue
        yield i, line

violations = []
for root in (Path('vac-rs/core/src'), Path('vac-rs/crates/capabilities'), Path('vac-rs/crates/surfaces/tui/src')):
    for path in sorted(root.rglob('*.rs')):
        rel = path.as_posix()
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
PY_CHANNEL

# R-05/S-08: semantic split files are dispatchers; large domain shard work is tracked separately.
python3 - <<'PY_SPLIT'
from pathlib import Path
import re
violations = []
for manifest in sorted(Path('vac-rs').rglob('split_manifest.yaml')):
    text = manifest.read_text()
    if 'kind: o5_godfile_semantic_split' not in text:
        continue
    dispatcher = manifest.parent / 'semantic_split.rs'
    if not dispatcher.exists():
        violations.append(f'{dispatcher}: missing dispatcher')
        continue
    lines = dispatcher.read_text().splitlines()
    code_lines = [line.strip() for line in lines if line.strip() and not line.strip().startswith('//')]
    if len(lines) > 80:
        violations.append(f'{dispatcher}: dispatcher too large ({len(lines)} lines)')
    for line in code_lines:
        if not re.fullmatch(r'include!\("[^"]+\.rs"\);', line):
            violations.append(f'{dispatcher}: non-dispatcher code line `{line}`')
if violations:
    for violation in violations:
        print(violation, file=__import__('sys').stderr)
    raise SystemExit(1)
print('semantic split dispatcher surface: PASS')
PY_SPLIT

# R-04/S-05: legacy ChatGPT remote catalog/skill paths are fail-closed by default.
require_file vac-rs/crates/capabilities/core-plugins/src/remote.rs
require_file vac-rs/crates/capabilities/core-skills/src/remote.rs
require_grep 'VAC_ENABLE_LEGACY_CHATGPT_CLOUD_PLUGINS' vac-rs/crates/capabilities/core-plugins/src/remote.rs
require_grep 'LegacyCloudDisabled' vac-rs/crates/capabilities/core-plugins/src/remote.rs
require_grep 'return Err\(RemotePluginCatalogError::LegacyCloudDisabled\)' vac-rs/crates/capabilities/core-plugins/src/remote.rs
require_grep 'VAC_ENABLE_LEGACY_CHATGPT_REMOTE_SKILLS' vac-rs/crates/capabilities/core-skills/src/remote.rs
require_grep 'legacy ChatGPT remote skill scopes are disabled' vac-rs/crates/capabilities/core-skills/src/remote.rs

# R-07/S-07: topology target is encoded as a machine-checkable layer map pending cargo toolchain relocation.
require_file .vac/registry/architecture-layer-map.yaml
require_grep 'layered_workspace_target' .vac/registry/architecture-layer-map.yaml
require_grep 'crates/control-plane' .vac/registry/architecture-layer-map.yaml

printf 'audit remainder static gate: PASS\n'
