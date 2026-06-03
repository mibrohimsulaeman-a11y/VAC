# Plan 30G blocker #3 closure — L24 TUI fallback scope

Date: 2026-05-26
Slice: L24 — Plan 30G blocker #3 closure (TUI fallback)
Status: **PARTIAL** — allowed `background_requests.rs` scope is clear; retained/direct fallback removal remains in forbidden `app_server_session.rs`.

## Pre-flight overlap check

Requested pre-flight commands:

```bash
git log --oneline -5 -- vac-rs/tui/src/app/background_requests.rs
git status --short -- vac-rs/tui/src/app/background_requests.rs
```

Observed output:

```text
4b52cec refactor(tui): expand exact-owner session dto boundary
8b0b0c8 refactor(tui): route runtime consumers through local session facade
a6aaeb1 00D: wrap TUI session background request handle
b85931c 00D: introduce TUI session protocol facade
4ae59e3 VAC fork initial: VAC product tree + P1.x batch (2026-05-07 session)
```

`git status --short -- vac-rs/tui/src/app/background_requests.rs` returned no output. The file was clean before this slice. Although the last path-specific commit was older than `680c69b`, that confirms L1-continuation did not modify this file while closing blocker #2 owner-side plugin import completion.

## Allowed-scope fallback audit

Allowed code scope for this slice:

- `vac-rs/tui/src/app/background_requests.rs`
- `vac-rs/local-runtime-owner/src/external_agent_config.rs` read-only only

Forbidden fallback owner file, not edited:

- `vac-rs/tui/src/app_server_session.rs`

Focused scans in the allowed file found:

```bash
rg -n "external|External|agent|Agent|detect|Detect|import|Import|fallback|Fallback|vac_app_server|app-server|owner" vac-rs/tui/src/app/background_requests.rs
rg -n "vac_app_server" vac-rs/tui/src/app/background_requests.rs
```

Results:

- No direct `vac_app_server` references in `background_requests.rs`.
- No external-agent detect/import fallback branch in `background_requests.rs`.
- The only `external` / `fallback` / `app-server` hits were module comments or unrelated text.
- Plugin helper branches in `background_requests.rs` call typed `ClientRequest` operations through the `LocalRuntimeRequestHandle` abstraction; they are not external-agent detect/import fallback arms.

## External-agent owner completeness check

Read-only inspection of `vac-rs/local-runtime-owner/src/external_agent_config.rs` confirms blocker #2 was narrowed, not fully eliminated for sessions:

- Plugin imports now complete without leaving pending background work.
- Session imports still remain explicit pending background work via `pending_session_imports`.
- `has_pending_background_work()` is true when selected session imports remain.
- The local tests include `session_imports_remain_explicit_pending_background_work`, preserving the stop condition rather than silently claiming completion.

Therefore no app-server fallback can be deleted from `background_requests.rs`, because the relevant retained/direct external-agent fallback arms are not in that file. Any code change that removes or gates those arms requires touching forbidden `vac-rs/tui/src/app_server_session.rs`.

## Fallback marker / warning decision

No structured warning marker was added in this slice because there is no external-agent detect/import fallback branch in the allowed `background_requests.rs` file. Adding the marker at the actual retained/direct fallback site would require editing forbidden `vac-rs/tui/src/app_server_session.rs`, so this lane stops at documentation.

## `vac_app_server` count

Command:

```bash
rg -o "vac_app_server" vac-rs/tui/src | wc -l
```

Before: `41`
After: `41`

The count is unchanged because this slice made no TUI code edits and found no allowed fallback branch to remove.

## Validation

Because no Rust source changed, the cargo validation commands are no-op confidence checks for the current tree rather than proof of a code delta.

```bash
cd vac-rs && cargo +1.93.0 build -p vac-surface-tui
cargo +1.93.0 nextest run -p vac-surface-tui background_requests
cargo +1.93.0 clippy -p vac-surface-tui -- -D warnings
```

Observed results:

- `cargo +1.93.0 build -p vac-surface-tui`: **PASS**. The build completed with existing warnings in forbidden `tui/src/app_server_session.rs` / `tui/src/session_protocol.rs`.
- `cargo +1.93.0 nextest run -p vac-surface-tui background_requests`: **PASS**, 5 passed / 2199 skipped.
- `cargo +1.93.0 clippy -p vac-surface-tui -- -D warnings`: **FAIL — out-of-scope pre-existing lint debt**. The first hard errors are in `vac-rs/core/src/session/mod.rs`, `vac-rs/core/src/control_plane/*`, `vac-rs/core/src/local_runtime/projection.rs`, and `vac-rs/core/src/tools/sandboxing.rs`. This slice did not edit those files, and the forbidden TUI fallback file also had existing warnings surfaced by build/nextest.

## Conclusion

Blocker #3 status: **PARTIAL**.

Closed for the allowed L24 `background_requests.rs` scope:

- File was clean before launch.
- L1-continuation did not touch it.
- No external-agent detect/import fallback arm exists there.
- No direct `vac_app_server` reference exists there.

Remaining gap:

- Retained/direct external-agent detect/import fallback arms still live in forbidden `vac-rs/tui/src/app_server_session.rs`.
- Session import completion is still explicit pending background work in the owner provider.
- A follow-up lane with ownership of `app_server_session.rs` must remove/gate those fallback arms once session import semantics are owner-complete, or add a structured warning marker there if policy keeps the fallback.
