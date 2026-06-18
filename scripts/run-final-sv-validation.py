#!/usr/bin/env python3
from __future__ import annotations
import os, pathlib, shlex, subprocess, sys, tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]

def resolve_log() -> pathlib.Path:
    if len(sys.argv) > 1:
        candidate = pathlib.Path(sys.argv[1])
        return candidate if candidate.is_absolute() else pathlib.Path('/tmp') / candidate.name
    return pathlib.Path(tempfile.gettempdir()) / 'vac-final-sv-validation.log'

LOG = resolve_log()
commands = [
    ('cargo-tv', ['python3', 'scripts/check-cargo-tv.py', '.', '--summary-only'] if os.environ.get('VAC_CARGO_TV_CONSUME_PROOF') == '1' else ['python3', 'scripts/check-cargo-tv.py', '.'], 7200),
    ('deterministic-index', ['python3', 'scripts/generate-deterministic-index-sv.py', '.'], 300),
    ('compile-registry', ['python3', 'scripts/compile-vac-registry-sv.py', '.'], 300),
    ('assessment-report', ['python3', 'scripts/generate-assessment-report-sv.py', '.'], 300),
    ('assessment-freshness', ['python3', 'scripts/check-assessment-freshness.py', '.'], 300),
    ('external-provider-remote-process-io', ['python3', 'scripts/check-external-provider-remote-process-io-e2e.py', '.'], 300),
    ('scoped-validation-proof', ['python3', 'scripts/check-ci-scoped-validation-proof.py', '.'], 300),
    ('l2-broker-status', ['python3', 'scripts/check-l2-broker-status.py', '.'], 300),
    ('confirmed-intent-coverage', ['python3', 'scripts/check-confirmed-intent-coverage.py', '.'], 300),
    ('confirmed-intent-negative-fixtures', ['python3', 'scripts/check-confirmed-intent-negative-fixtures.py', '.'], 300),
    ('confirmed-intent-status', ['python3', 'scripts/refresh-confirmed-intent-status.py', '.'], 300),
    ('rust-ast-index-status', ['python3', 'scripts/refresh-rust-ast-index-status.py', '.'], 300),
    ('sv-static', ['python3', 'scripts/sv_static_validate.py', '.'], 300),
    ('sv-deep', ['python3', 'scripts/vac-sv-deep-validate.py', str(ROOT)], 300),
    ('runtime-state5-operational', ['python3', 'scripts/vac-runtime-state5-operational-sv.py', '.'], 300),
    ('runtime-state6-semantics', ['python3', 'scripts/vac-runtime-state6-semantics-sv.py', '.'], 300),
    ('runtime-state7-merged-audit', ['python3', 'scripts/vac-runtime-state7-merged-audit-sv.py', '.'], 300),
    ('refresh-evidence-logs', ['python3', 'scripts/refresh-evidence-logs-sv.py', '.'], 300),
    ('evidence-log-freshness', ['python3', 'scripts/check-evidence-log-freshness.py', '.'], 300),
]

LOG.parent.mkdir(parents=True, exist_ok=True)
with LOG.open('w') as log:
    log.write('VAC final SV validation v1.9/state7\n')
    log.write(f'log_path={LOG}\n')
    log.write('log_hygiene=outside_indexed_root_before_index_generation\n')
    for name, cmd, timeout_seconds in commands:
        log.write(f"\n== {name} ==\n$ {' '.join(shlex.quote(x) for x in cmd)}\n")
        log.flush()
        try:
            p = subprocess.run(cmd, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout_seconds)
        except subprocess.TimeoutExpired as e:
            if isinstance(e.stdout, str):
                log.write(e.stdout)
            log.write(f"\nTIMEOUT: {name}\n")
            log.flush()
            print(f"SV validation timeout at {name}")
            sys.exit(124)
        log.write(p.stdout or '')
        log.write(f"exit={p.returncode}\n")
        log.flush()
        if p.returncode != 0:
            print(f"SV validation failed at {name}")
            sys.exit(p.returncode)
    log.write('\nVAC v1.9 final SV replay: PASS\n')
    log.write('final_gate_order_replayable=true\n')
    log.write('final_gate_idempotence_checked_by=scripts/vac-final-idempotence-sv.py\n')
    log.write('assessment_regenerated_after_index=true\n')
    p = subprocess.run(['python3', 'scripts/check-cargo-tv.py', '.', '--summary-only'], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=300)
    log.write(p.stdout or '')
    if p.returncode != 0:
        print('SV validation failed at cargo-tv-summary')
        sys.exit(p.returncode)
    log.write('l2_broker=NotImplemented\n')
print(f"wrote {LOG}")
