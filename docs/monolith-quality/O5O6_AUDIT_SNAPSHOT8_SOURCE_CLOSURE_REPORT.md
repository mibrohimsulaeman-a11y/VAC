# O5/O6 Audit Snapshot 8 Source Closure

Status: `SV-Done_TV-Blocked`

This report closes the source-level findings from the uploaded audit snapshot against `vac-o5o6-zero-residual-static-validated-toolchain-blocked`. It does not claim runtime validation because the current sandbox has no `rustc`, `cargo`, or native `vac` binary.

## Findings closed source-effectively

| Finding | Status | Source change |
| --- | --- | --- |
| CL-1 style revision | Pass static | `ChatWidget` now owns `style_epoch`; `CellHeightKey.style_revision` uses this epoch instead of width-derived `last_rendered_width`; theme/status-line style changes bump the epoch and clear cache. |
| CL-2 cache eviction | Pass static | `DesiredHeightCache` now enforces `HEIGHT_CACHE_MAX_ENTRIES = 4096` and `HEIGHT_CACHE_MAX_REVISIONS_PER_CELL = 3`, with eviction/stale-prune metrics. |
| CL-3 prefix rebuild | Pass static | `HeightPrefixBuildKey` guards prefix rebuilds and records `prefix_rebuild_skipped`. |
| CL-4 shimmer hot path | Pass static | Disabled shimmer emits a single full-text span; active shimmer groups contiguous same-style chars and records `spans_emitted`. |
| CL-5 visible range | Pass static | Render path passes `current_transcript_scroll_top()` instead of hardcoded `0`, and plans record scroll metadata. |
| CL-6 palette gating | Verified static | `dispatch_command` and `dispatch_command_with_args` already block `FeatureOff` and `Unavailable`. Static gate keeps this locked. |
| CL-7 contrast validation | Verified static | Runtime `/theme` picker already calls contrast enforcement and falls back to a readable theme. |
| P-6 startup graph | Pass static | Added `StartupGraphExecutor` with bounded parallel execution of non-blocking startup tasks in the static probe path. |

## Gates

```bash
bash scripts/check-vac-tui-perf-ux-findings-static.sh
VAC_STATIC_ONLY=1 bash scripts/bench-vac-tui-performance.sh
```

## Runtime/toolchain status

Still blocked:

```text
cargo check
cargo test
cargo clippy
native vac doctor gates
compiled TUI benchmark harness
```

Reason: no Rust toolchain or native VAC binary is available in this sandbox. The artifact policy therefore remains source/static only and forbids `runtime_validated_final` claims.
