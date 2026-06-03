# Plan 33 validation matrix — sandbox closeout 2026-05-28

| Field | Value |
|---|---|
| Capture run directory | `/mnt/data/vac-next-impl/vastar-agentic-cli` |
| Capture timestamp | 2026-05-28 sandbox run |
| Git commit | sandbox source artifact checkpoint, not a local git commit |
| Disk free before validation | Checked by sandbox batch; full workspace build not used as gate due known sandbox reset/link instability |
| Cargo/sccache notes | Targeted Rust/file/YAML validation used; full Cargo remains local/operator evidence |
| Plan 32 status reference | Complete for runtime-owner hard gates |

## Validation rows

| Check | Command / method | Result | Notes |
|---|---|---|---|
| Source grep | `rg ... vac-rs/tui/Cargo.toml vac-rs/tui/src` | PASS with classified optional/comment matches | See `source-grep-evidence.md`. |
| Default TUI feature check | Static parse of `vac-rs/tui/Cargo.toml` | PASS | `default = []`; app-server client optional only. |
| Runtime-owner gate guard retention | Static grep for guard code/tests | PASS | App-server regression checks remain in `doctor/runtime_owner_gates.rs`. |
| Docs closeout | Updated evidence docs and Plan 33 main doc | PASS | No placeholder evidence remains in the closeout files. |
| Full local Cargo validation | `cargo check/tree/...` | OPERATOR-GATED | Required for physical deletion, not for default-path defer closeout. |

## Stop-condition review

| Stop condition | Result | Action |
|---|---|---|
| Inverse tree still reaches app-server through default `vac-cli` path | Not proven by sandbox Cargo tree; static default-feature evidence says no default TUI path | Keep physical deletion deferred; do not overclaim. |
| Source grep shows active TUI app-server imports | Optional feature only | Allowed as non-default compatibility. |
| Validation fails after dependency removal | Not observed in targeted validation | Full Cargo remains local/operator evidence. |
| Workspace-wide consumers are unclear | Classified | Keep app-server workspace crates deferred. |
| Deletion would remove a still-owned non-default capability | Yes | Do not delete in this slice. |

## Closeout answers

| Question | Answer |
|---|---|
| Is the validation matrix complete for default-path closeout? | Yes. |
| Are failures explained with blockers, not hidden? | Yes. Full physical deletion remains explicitly deferred. |
| Can Plan 33 proceed to delete/defer decision? | Yes: default path retired, workspace crate deletion deferred. |
