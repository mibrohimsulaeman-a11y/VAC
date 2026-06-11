#!/usr/bin/env python3
from __future__ import annotations
import hashlib, json, re, sys
try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 local TV gate compatibility.
    import tomli as tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
fail: list[str] = []
warn: list[str] = []

def check(cond: bool, msg: str) -> None:
    if not cond: fail.append(msg)

def read_json(path: Path):
    try: return json.loads((ROOT/path).read_text())
    except Exception as exc:
        fail.append(f"invalid json {path}: {exc}"); return None

def sha256_file(path: Path) -> str:
    h=hashlib.sha256()
    with path.open('rb') as fh:
        for chunk in iter(lambda: fh.read(1024*1024), b''):
            h.update(chunk)
    return 'sha256:'+h.hexdigest()

required_dirs = [
    '.vac/capabilities','.vac/policies','.vac/surfaces','.vac/workflows','.vac/specs',
    '.vac/index','.vac/assessment','.vac/cache/compiled','.vac/registry/runtime',
    'vac-rs/crates/surfaces/vac-tui','vac-rs/crates/runtime/vac-runtime-jobs',
    'vac-rs/crates/control-plane/vac-readiness','vac-rs/crates/control-plane/vac-registry-compiler',
    'vac-rs/crates/foundation/vac-brand','vac-rs/crates/foundation/vac-paths','vac-cli/bin'
]
for d in required_dirs:
    check((ROOT/d).is_dir(), f"missing required dir: {d}")

for legacy_root in ['Cargo.toml','cli','tui','libs']:
    check(not (ROOT/legacy_root).exists(), f"legacy root artifact still present: {legacy_root}")

cargo_path=ROOT/'vac-rs/Cargo.toml'
try:
    cargo=tomllib.loads(cargo_path.read_text())
except Exception as exc:
    fail.append(f"vac-rs/Cargo.toml is invalid TOML: {exc}"); cargo={}
workspace = cargo.get('workspace', {}) if isinstance(cargo, dict) else {}
for member in workspace.get('members', []):
    check((ROOT/'vac-rs'/member/'Cargo.toml').is_file(), f"workspace member missing Cargo.toml: {member}")
for dep,spec in workspace.get('dependencies', {}).items():
    if dep.startswith('vac-') and isinstance(spec, dict) and 'path' in spec:
        check((ROOT/'vac-rs'/spec['path']/'Cargo.toml').is_file(), f"workspace dependency path missing: {dep} -> {spec['path']}")

status = read_json(Path('.vac/registry/status.json')) or {}
check(status.get('product')=='VAC','status product must be VAC')
check(status.get('control_plane_version') in {'1.9','1.5'},'control plane version must be 1.9 or legacy-compatible 1.5')
check(status.get('control_plane',{}).get('runtime')=='compiled_json','runtime authority must be compiled_json')
check(status.get('control_plane',{}).get('compiled_snapshots_current') is True,'compiled snapshots must be current')
check(status.get('runtime',{}).get('server_default') is False,'server must not be default runtime')
check(status.get('runtime',{}).get('gateway_default') is False,'gateway must not be default runtime')

caps = read_json(Path('.vac/cache/compiled/capabilities.json')) or read_json(Path('.vac/registry/compiled/capabilities.json')) or {}
order={'deprecated':0,'planned':1,'partial':2,'ready':3}
for cap in caps.get('capabilities', []):
    r=cap.get('readiness', {})
    check('declared' in r and 'computed' in r and 'effective' in r, f"readiness triplet missing: {cap.get('id')}")
    if order.get(r.get('effective','planned'),1) > order.get(r.get('computed','planned'),1):
        fail.append(f"effective readiness exceeds computed: {cap.get('id')}")
    src=cap.get('source', {}) if isinstance(cap, dict) else {}
    if src:
        p=ROOT/src.get('path','')
        check(p.is_file(), f"compiled capability source missing: {src.get('path')}")
        if p.is_file():
            check(sha256_file(p)==src.get('sha256'), f"compiled capability source hash mismatch: {src.get('path')}")

compiled_ids={cap.get('id') for cap in caps.get('capabilities', []) if isinstance(cap, dict)}
authoring_ids=set()
for cap_yaml in sorted((ROOT/'.vac/capabilities').glob('*.yaml')):
    m=re.search(r'^id:\s*(\S+)', cap_yaml.read_text(errors='ignore'), re.M)
    if m:
        authoring_ids.add(m.group(1))
check(compiled_ids == authoring_ids, f"compiled capability ids must match authoring ids: compiled={len(compiled_ids)} authoring={len(authoring_ids)}")

compiled = read_json(Path('.vac/cache/compiled/workspace.json')) or read_json(Path('.vac/registry/compiled/workspace.json')) or {}
source_hashes = compiled.get('payload',{}).get('source_hashes') or compiled.get('source_hashes') or []
check(bool(source_hashes), 'compiled workspace snapshot must record source_hashes')
for src in source_hashes:
    p=ROOT/src.get('path','')
    check(p.is_file(), f"compiled source missing: {src.get('path')}")
    if p.is_file():
        check(sha256_file(p)==src.get('sha256'), f"compiled source hash mismatch: {src.get('path')}")

jobs=read_json(Path('.vac/registry/runtime/jobs.json')) or {}
check(isinstance(jobs.get('jobs'), list),'runtime jobs must be a list')

for name in ['files.jsonl','symbols.jsonl','spans.jsonl','risks.jsonl','read_plans.jsonl','relations.jsonl','repo_manifest.jsonl']:
    path=ROOT/'.vac/index'/name
    check(path.is_file(), f"missing index file: {name}")
    if path.is_file():
        for lineno,line in enumerate(path.read_text().splitlines(),1):
            if not line.strip(): continue
            try: json.loads(line)
            except Exception as exc:
                fail.append(f"invalid JSONL {name}:{lineno}: {exc}"); break

# TUI must not contain screenshot/mock tokens in production renderer.
tui_root=ROOT/'vac-rs/crates/surfaces/vac-tui/src'
for path in tui_root.rglob('*.rs'):
    text=path.read_text(errors='ignore')
    for pat in ['claude-sonnet','VIL-native','rulebook vil.core','refactor handlers/input.rs','mutation gate']:
        if pat in text:
            fail.append(f"hardcoded mock token {pat!r} in {path.relative_to(ROOT)}")

# Upstream brand/provider labels must not remain in product Rust after rebrand.
legacy = re.compile('|'.join(re.escape(p) for p in ['stak'+'pak', 'stack'+'pak', 'stak'+'ai', '.'+'stak'+'pak']), re.IGNORECASE)
for path in (ROOT/'vac-rs/crates').rglob('*.rs'):
    text=path.read_text(errors='ignore')
    if legacy.search(text):
        fail.append(f"legacy upstream brand/provider path in product Rust: {path.relative_to(ROOT)}")

# Guard against known mechanical crate-rename artifacts without flagging ordinary prose.
mechanical_bad_tokens = [
    'vac-provider-core_adapter',
    'vac-provider-core::',
    'vac-foundation::',
    'vac-runtime-jobs::',
]
for path in (ROOT/'vac-rs/crates').rglob('*.rs'):
    text=path.read_text(errors='ignore')
    for tok in mechanical_bad_tokens:
        if tok in text:
            fail.append(f"mechanical hyphenated Rust artifact {tok!r}: {path.relative_to(ROOT)}")

models_mod=ROOT/'vac-rs/crates/foundation/vac-foundation/src/models/mod.rs'
if models_mod.is_file():
    mod_text=models_mod.read_text(errors='ignore')
    check('provider_core_adapter' in mod_text, 'provider_core_adapter module missing in vac-foundation models/mod.rs')
    check('vac-provider-core_adapter' not in mod_text, 'hyphenated module name remains in vac-foundation models/mod.rs')

cmd=ROOT/'vac-rs/crates/surfaces/vac-tui/src/services/commands.rs'
if cmd.exists():
    text=cmd.read_text(errors='ignore')
    for route in ['/runtime','/capabilities','/assessment','/spec-sync','/first-launch','/idle','/chat']:
        check(route in text, f"missing TUI command route {route}")
else:
    fail.append('missing TUI commands.rs')

if fail:
    print('SV validation: FAIL')
    for item in fail: print(' -', item)
    if warn:
        print('Warnings:')
        for item in warn: print(' -', item)
    sys.exit(1)
print('SV validation: PASS')
print(f"checked root: {ROOT}")
print(f"workspace members: {len(workspace.get('members', []))}")
print(f"capabilities: {len(caps.get('capabilities', []))}")
print(f"runtime jobs: {len(jobs.get('jobs', []))}")
print(f"source hashes checked: {len(source_hashes)}")
