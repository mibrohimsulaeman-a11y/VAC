#!/usr/bin/env python3
from __future__ import annotations
import pathlib, shlex, subprocess, sys, tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]

def resolve_log() -> pathlib.Path:
    if len(sys.argv) > 1:
        candidate = pathlib.Path(sys.argv[1])
        return candidate if candidate.is_absolute() else pathlib.Path('/tmp') / candidate.name
    return pathlib.Path(tempfile.gettempdir()) / 'vac-final-sv-validation.log'

LOG = resolve_log()
commands = [
    ('deterministic-index', ['python3', 'scripts/generate-deterministic-index-sv.py', '.']),
    ('compile-registry', ['python3', 'scripts/compile-vac-registry-sv.py', '.']),
    ('assessment-report', ['python3', 'scripts/generate-assessment-report-sv.py', '.']),
    ('assessment-freshness', ['python3', 'scripts/check-assessment-freshness.py', '.']),
    ('sv-static', ['python3', 'scripts/sv_static_validate.py', '.']),
    ('sv-deep', ['python3', 'scripts/vac-sv-deep-validate.py', str(ROOT)]),
    ('runtime-state5-operational', ['python3', 'scripts/vac-runtime-state5-operational-sv.py', '.']),
    ('runtime-state6-semantics', ['python3', 'scripts/vac-runtime-state6-semantics-sv.py', '.']),
    ('runtime-state7-merged-audit', ['python3', 'scripts/vac-runtime-state7-merged-audit-sv.py', '.']),
    ('refresh-evidence-logs', ['python3', 'scripts/refresh-evidence-logs-sv.py', '.']),
    ('evidence-log-freshness', ['python3', 'scripts/check-evidence-log-freshness.py', '.']),
]

LOG.parent.mkdir(parents=True, exist_ok=True)
with LOG.open('w') as log:
    log.write('VAC final SV validation v1.5/state7\n')
    log.write(f'log_path={LOG}\n')
    log.write('log_hygiene=outside_indexed_root_before_index_generation\n')
    for name, cmd in commands:
        log.write(f"\n== {name} ==\n$ {' '.join(shlex.quote(x) for x in cmd)}\n")
        log.flush()
        try:
            p = subprocess.run(cmd, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=300)
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
    log.write('\nVAC re-audit final SV gate: PASS\n')
    log.write('final_gate_order_replayable=true\n')
    log.write('final_gate_idempotence_checked_by=scripts/vac-final-idempotence-sv.py\n')
    log.write('assessment_regenerated_after_index=true\n')
    log.write('cargo_tv=NotEvaluated\n')
    log.write('l2_broker=NotImplemented\n')
print(f"wrote {LOG}")
