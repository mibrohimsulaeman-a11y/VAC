#!/usr/bin/env python3
"""Check that VAC v1.9 clippy deny-debt is explicit rather than silently broken."""
from __future__ import annotations
import pathlib, re, sys
ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
errors=[]
cargo=(ROOT/'vac-rs/Cargo.toml').read_text()
policy=ROOT/'.vac/policies/clippy-debt.yaml'
if not policy.is_file():
    errors.append('missing .vac/policies/clippy-debt.yaml')
for lint in ['unwrap_used','expect_used','string_slice']:
    m=re.search(rf'^{lint}\s*=\s*"([^"]+)"', cargo, re.M)
    if not m:
        errors.append(f'missing workspace lint {lint}')
    elif m.group(1) == 'deny':
        errors.append(f'{lint}=deny while debt baseline is still unresolved; this creates known false TV failure')
if 'cargo clippy --manifest-path vac-rs/Cargo.toml' not in (ROOT/'.github/workflows/ci.yml').read_text():
    errors.append('CI clippy command must target vac-rs/Cargo.toml')
if errors:
    print('VAC clippy debt strategy: FAIL')
    for e in errors: print('-', e)
    raise SystemExit(1)
print('VAC clippy debt strategy: PASS')
print('deny_lints=deferred_with_policy_debt_baseline')
