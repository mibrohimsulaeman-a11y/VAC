# VAC-Init Deterministic Hardening P0-P4

Status: implemented.

## Scope

P0-P4 removes non-deterministic readiness claims and makes the next layer of production hardening measurable:

- P0 removes hardcoded readiness scoreboard and hardcoded release doctor pass aggregation.
- P1 migrates workflow manifests to typed `steps[].uses` and top-level structured `validation`.
- P2 makes release aggregation consume real doctor reports rather than static pass values.
- P3 wires runtime gate call-sites into patch and command execution paths.
- P4 adds live file-backed store helpers and wires `vac init` atomic writes through them.

## Non-goals

This slice does not require full workspace build or full production E2E. Those remain minimal/explicitly evaluated in later gates.
