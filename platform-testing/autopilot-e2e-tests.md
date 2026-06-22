> **VAC v1.9 status note:** This report is historical platform evidence. Current installation and runtime instructions are in `GETTING-STARTED.md`; current source authority is `.vac/registry/compiled` JSON. URLs inside this report that point to example endpoints are fixture data, not current hosted documentation.

# VAC autopilot E2E testing state

Current sandbox validation is source/static only. Real device/PTY/service tests remain TV-Pending.

## SV tests available now

```bash
python3 scripts/vac-runtime-agent-e2e-sv.py
python3 scripts/check-docs-current-state.py
bash scripts/vac-static-gate.sh
```

## TV-Pending local tests

- cargo metadata/fmt/check/clippy/test.
- PTY visual QA for TUI screens.
- Real one-shot/cron/filewatch job execution.
- Optional broker-mediated L2 runtime validation.

Do not mark platform tests green until those commands and device/service checks are actually run.
