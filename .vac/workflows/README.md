# Workflows (authored)

Ordered, typed gate definitions. acceptance is BINARY:

- `acceptance.mode: binary` — only `pass` or `fail`.
- `acceptance.unknown_is: fail` — a gate/command that did not run is failing (fail-closed).
- No `pass_or_recorded_pending`. To defer a blocking finding, add an expiring Waiver in `ledger/waivers.yaml`.
- Evidence is `runner_emitted` — the runner records real exit codes + artifact hashes; the agent never writes evidence.
