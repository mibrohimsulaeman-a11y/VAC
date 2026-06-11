# VAC autopilot channel setup

This document is current-state only. Autopilot is an optional VAC runtime monitor that records jobs under `.vac/registry/runtime/jobs.json`; it is not the default product runtime and it does not require a server/gateway stack.

## Current contract

- Runtime jobs are one-shot, cron, or filewatch records.
- TUI `/runtime` reads `.vac/registry/runtime/jobs.json`.
- Empty registry is a valid honest empty state.
- Channel gateways live under `vac-rs/crates/integrations/vac-messaging-gateway` and are optional.

## Sandbox validation

```bash
python3 scripts/vac-runtime-agent-e2e-sv.py
bash scripts/vac-static-gate.sh
```

Real remote channel setup and service installation remain TV-Pending.
