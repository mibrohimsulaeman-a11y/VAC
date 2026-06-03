# O5/O6 Continuation Truth Ledger

This ledger corrects readiness claims that were marker-only or computed-but-discarded in the prior continuation artifact.

## Downgraded readiness

| Item | Prior claim | Corrected status | Follow-up slice | Reason |
| --- | --- | --- | --- | --- |
| P-2 windowed render | SV-Done | Partial/Dead-code | tui_runtime_effective_windowing | `windowed_render_plan` was computed but not applied to runtime render selection. |
| V-1 shimmer metric | SV-Done | Marker-only | tui_runtime_effective_shimmer | Comment marker existed without metrics storage/emission. |
| V-2 contrast validator | SV-Done | Marker-only | tui_runtime_effective_theme_contrast | Comment marker existed without contrast ratio logic. |
| L-1 explicit auth state | SV-Done | Open | tui_runtime_effective_auth_state | Auth skip could still be logged only. |
| F-2 experimental badge | Fixed | Partial | tui_runtime_effective_palette_badges | Descriptions mentioned experimental status, but palette availability was not typed. |

## Readiness rules

- comment marker != implementation
- computed plan discarded != runtime behavior
- description text != palette badge/disabled-state
- SV-Done requires runtime-effective source path plus a static gate that checks the behavior hook, not only strings.

Cargo, clippy, benchmark timings, and live TUI smoke remain TV-Pending until actually executed.
