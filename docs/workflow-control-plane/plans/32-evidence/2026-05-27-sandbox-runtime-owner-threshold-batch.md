# Plan 32 Sandbox Runtime-Owner Threshold Batch — 2026-05-27

Status: `PARTIAL-IMPLEMENTED`

This sandbox slice extends the `vac doctor runtime-owner-gates` threshold surface beyond the Plan 32F missing-field hardening. It keeps migration-dependent checks warning-level where Plan 30/31/33 preconditions are still open, and keeps `message_processor_copy` as a hard error because copying the donor processor into active owner paths would be a direct architecture regression.

## Implemented in this batch

- `missing_field` remains a hard error for top-level required manifest fields and is now also a hard error for malformed `ownership.targets[]` entries that omit `crate_name`.
- `app_server_dependency_present` warns when watched runtime-owner Cargo manifests still carry default app-server crate dependencies.
- `pty_false_green` warns on `BLOCKED-OPERATOR` PTY evidence and errors only if the same evidence also claims pass/green.
- `message_processor_copy` errors when active runtime-owner scan paths copy or recreate `MessageProcessor` patterns.
- `unsupported_control_default_defer` warns when owner/default paths still expose explicit unsupported-control or background-completion sentinels.

## Sandbox audit snapshot

| Signal | Count | Interpretation |
|---|---:|---|
| active TUI/app-server source or manifest matches | 44 | Plan 31/33 not complete; warnings must remain non-green evidence. |
| watched Cargo app-server dependencies | 4 | Default dependency retirement still blocked by Plan 31/33. |
| active owner MessageProcessor copy patterns | 0 | Should remain zero; nonzero is a hard error. |
| blocked operator PTY evidence markers | 22 | PTY is honest blocked evidence, not release pass evidence. |
| unsupported-control/default-defer markers | 7 | Plan 30G/31 still need explicit owner-native closeout or non-default defer. |

## Validation intent

Targeted validation for this slice is limited to `runtime_owner_gates` formatting plus focused tests/harnesses. Full workspace build remains unreliable in sandbox and is intentionally not used as the slice gate.
