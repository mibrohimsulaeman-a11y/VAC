# Local Runtime Gate

## Goal

Verify local prompt execution no longer depends on old runtime client/protocol stack.

## Commands

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
```

## Expected result

- build passes,
- old runtime client/server crates are absent from default local path or explicitly deferred,
- `vac exec` and the root `vac` TUI path use Local Runtime Contract.

## Cannot claim if failed

```text
local-first runtime
safe to start .vac registry
safe to delete old runtime crates
```
