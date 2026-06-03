# TUI Status Output Gate

Status: active  
Capability: `vac.tui.status-output`  
Workflow: `maintenance.tui-status-output`

## Intent

This gate prevents `/status` regressions after the operator-console visual hardening work. `/status` must stay focused on runtime/provider/model/token/context diagnostics and must not reintroduce legacy rate-limit or credit account display rows.

## Required Checks

```bash
bash scripts/check-tui-status-output-contract.sh
bash scripts/check-tui-status-deep-hardening-contract.sh
rustc --edition 2024 --test vac-rs/tui/src/status/output_contract.rs -o /tmp/vac-status-output-contract-test
/tmp/vac-status-output-contract-test
```

## Required Rows

- `Model`
- `Model provider`, always visible
- `Model providers` when multiple providers are registered
- `Directory`
- `Permissions`
- `Token usage`
- `Context window`

## Forbidden Runtime Display Fragments

- `Visit ... rate limits`
- `rate limits and credits`
- `Credits:`
- `Credit balance`
- `Limits:`

## Runtime Refresh Policy

The gate also checks that `/status` is local-only:

- `SlashCommand::Status` must not send `AppEvent::RefreshRateLimits`.
- `add_status_output` must pass an explicit empty account-limit snapshot set.
- default `vastar` provider display must not be suppressed.
- multi-provider registry support must be present in the status card and contract.
- status command regression tests must assert that ChatGPT-authenticated sessions do not request a rate-limit refresh for `/status`.

## Notes

Historical tests may still construct protocol snapshots containing credit or rate-limit data to prove `/status` ignores those display rows. The gate is concerned with runtime card output and maintained status snapshots, not protocol type names.
