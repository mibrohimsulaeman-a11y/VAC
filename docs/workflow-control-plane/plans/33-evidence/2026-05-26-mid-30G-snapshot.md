# Plan 33 mid-30G snapshot — 2026-05-26

- Lane: L18 — Plan 33 evidence sweep refresh
- Snapshot local time: `2026-05-26T19:50:00+07:00`
- Branch: `main`
- HEAD at capture: `d2e57cf` (`docs: add Plan 23 PTY operator evidence runbook`)
- Scope: docs-only evidence refresh. No source files, other plans, or `30-evidence` files were edited.
- Context: repeated L6-style evidence after `fd61ed8` and later 30G/L1-continuation work landed on `main`.

## Status conclusion

Plan 33 remains **BLOCKED / final proof gate**.

- Active TUI source still contains `vac_app_server` references in 3 files.
- Cargo manifests still contain `vac-app-server` dependencies in 19 top-level crate manifests at this capture point.
- `vac doctor registry ..` is still not accepted by the current CLI surface.
- 30G is improved but not terminal: blocker #1 is closed, blocker #2 is partial, blocker #3 is escalated.

No delete/defer closeout is claimed by this snapshot.

## 1. `grep -rln vac_app_server vac-rs/ | grep -v target`

The target-pruned source snapshot relevant to the active TUI path reports 3 files under `vac-rs/tui/`:

```text
vac-rs/tui/src/app_server_session.rs
vac-rs/tui/src/legacy_core.rs
vac-rs/tui/src/session_protocol.rs
```

## 2. `grep -c vac_app_server` counts for the 3 TUI files

```text
vac-rs/tui/src/app_server_session.rs: 15
vac-rs/tui/src/legacy_core.rs: 1
vac-rs/tui/src/session_protocol.rs: 25
```

## 3. `grep -rln "vac-app-server" vac-rs/*/Cargo.toml`

Current dependent crate manifest list:

```text
vac-rs/analytics/Cargo.toml
vac-rs/app-server/Cargo.toml
vac-rs/app-server-client/Cargo.toml
vac-rs/app-server-protocol/Cargo.toml
vac-rs/app-server-transport/Cargo.toml
vac-rs/chatgpt/Cargo.toml
vac-rs/config/Cargo.toml
vac-rs/connectors/Cargo.toml
vac-rs/core/Cargo.toml
vac-rs/core-plugins/Cargo.toml
vac-rs/core-skills/Cargo.toml
vac-rs/exec-server/Cargo.toml
vac-rs/external-agent-sessions/Cargo.toml
vac-rs/login/Cargo.toml
vac-rs/model-provider-info/Cargo.toml
vac-rs/models-manager/Cargo.toml
vac-rs/otel/Cargo.toml
vac-rs/tools/Cargo.toml
vac-rs/tui/Cargo.toml
```

Count at capture: `19`.

Note: the lane request expected 22 dependent crate manifests, but the point-in-time command above returns 19 after the already-landed mainline changes. This file records the actual command result rather than preserving a stale expected count.

## 4. `vac doctor registry ..` output

Command:

```sh
vac doctor registry ..
```

Result:

```text
error: unexpected argument 'registry' found

Usage: vac doctor [OPTIONS]

For more information, try '--help'.
exit_code=2
```

## 5. 30G blocker progress

Current Plan 30 status says 30G is **partial / not complete** and must not be marked complete while provider gaps and compatibility fallbacks remain.

### Blocker #1 — owner external-agent provider path

Status: **CLOSED**.

- Closed by `fd61ed8` / owner provider work.
- External-agent config detect/import has an owner-native provider path for normal imports.
- TUI retained/direct paths route detect/import through `RuntimeCommandBus` for the normal path.

### Blocker #2 — external-agent background completion semantics

Status: **PARTIAL**.

- L1-continuation closed plugin import completion by making `ExternalAgentConfigService::import` complete plugin migration synchronously in the owner path.
- Remaining selected session import completion is still explicit pending background work.
- Session completion is escalated because the relevant fallback/session wiring lives in forbidden Lane L7-owned `vac-rs/tui/src/app_server_session.rs`.

### Blocker #3 — retained/direct fallback removal

Status: **ESCALATED**.

- Retained/direct external-agent fallback removal is still blocked by Lane L7-owned `vac-rs/tui/src/app_server_session.rs`.
- The allowed background request path did not contain the fallback branch needed for this lane to remove.
- This keeps Plan 30G and therefore Plan 33 non-terminal.

## 6. Plan 32 gate state relevant to Plan 33

Plan 32 remains **in progress**.

- The runtime-owner-gates scaffold subcommand has landed.
- Targeted validation previously reported `./target/debug/vac doctor runtime-owner-gates ..` as green for the scaffold gate.
- Full Plan 32 completion is still pending threshold/final gate work and broader out-of-scope fixture cleanup.

## Hygiene

- Existing unrelated dirty source/config work was observed and left untouched.
- This snapshot intentionally edits only this new evidence file plus the Plan 33 status index.
- No source files, `.vac` files, other plan files, scheduled-audit files, or `30-evidence` files are part of this L18 commit.
