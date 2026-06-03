# VAC-Init TUI Real-Data Integration

Status: implemented in sandbox production RC batch.

## Scope

F1-F3 requires TUI dashboard, approval popup, and autopilot monitor to consume real reports/stores instead of sample snapshots.

## Code-backed contract

```text
vac-rs/core/src/control_plane/vac_init_tui_real_data.rs
```

## Gate

```bash
bash scripts/check-vac-init-tui-real-data-contract.sh
```

The gate uses targeted `rustc --test` and YAML/static checks. Full workspace build is not a required sandbox gate.
