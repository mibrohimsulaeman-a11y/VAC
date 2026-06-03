# TUI Operator ANSI Style Gate

The ANSI style gate validates the semantic style layer added in Batch 3.

Commands:

```bash
rustc --edition 2024 --test vac-rs/tui/src/operator_style.rs
rustc --edition 2024 --test vac-rs/tui/tools/operator_style_snapshot_harness.rs
bash scripts/render-tui-operator-ansi-snapshots.sh docs/validation/tui-operator-ansi-snapshots
bash scripts/check-tui-operator-ansi-contract.sh
bash scripts/check-tui-operator-live-adapter.sh
bash scripts/check-tui-pty-gate-contract.sh
```

Acceptance criteria:

- all semantic roles exist: `chrome`, `muted`, `accent`, `success`, `warning`, `danger`, `user`, `agent`, `status`;
- generated `.ansi.txt` snapshots contain ANSI escape sequences;
- stripping ANSI returns the exact plain snapshot text;
- `/runtime` and `/capabilities` use the live ratatui adapter;
- live PTY proof remains explicit and is not falsely claimed in sandbox.
