# Plan 33F Evidence — Direct TUI Dependency Removal

Date: 2026-05-27
Environment: ChatGPT sandbox

## Result

The default TUI Cargo path is no longer connected to direct `vac-app-server` or `vac-app-server-protocol` dependencies.

## Cargo evidence

`vac-rs/tui/Cargo.toml` now has:

```toml
[features]
default = []
legacy-app-server-compat = [
    "dep:vac-app-server-client",
]

vac-app-server-client = { workspace = true, optional = true }
vac-runtime-protocol = { workspace = true }
```

## Delete/defer interpretation

- Direct TUI protocol/server dependency removal is complete.
- The optional client transport remains non-default and quarantined for rollback/defer evidence.
- Workspace-wide deletion of `vac-app-server*` crates is still deferred because non-TUI app-server crates/tests own historical compatibility surfaces.
