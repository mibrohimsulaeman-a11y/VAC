# TUI Operator UI Hardening Gate

This gate covers Hardening 1-2 for the Batch 1-3 operator-console implementation.

## Hardening 1: Regression Lock

Run:

```bash
bash scripts/check-tui-hardening-regression-lock.sh
```

The lock chains the status output gate, operator UI contract, visual contract, snapshot contract, ANSI contract, live adapter contract, renderer semantic contract, PTY contract, and autopilot scheduler contract.

## Hardening 2: Semantic Renderer Contract

Run:

```bash
bash scripts/check-tui-renderer-semantic-contract.sh
```

The renderer contract requires:

- `OperatorSpanSpec`
- `OperatorLineSpec`
- semantic plain text rendering
- semantic ANSI rendering
- `render_operator_snapshot_specs`
- `render_operator_snapshot_ansi_text`
- semantic live adapter via `style_operator_lines_from_specs`

## Status Output Gate

Run:

```bash
bash scripts/check-tui-status-output-contract.sh
```

The `/status` surface must retain model/provider/token/context/directory/permissions information while omitting legacy rate-limit and credit display rows. Hardening 3 additionally requires local-only slash-command behavior: no `RefreshRateLimits` event is emitted by `/status`, and cached account-limit snapshots are not passed into the status card.
