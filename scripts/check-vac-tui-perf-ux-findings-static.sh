#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "tui perf/ux findings gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() { ! grep -qE "$1" "$2" || fail "forbidden pattern in $2: $1"; }
require_file scripts/bench-vac-tui-performance.sh
require_grep 'VAC_TUI_PROFILE_STARTUP' scripts/bench-vac-tui-performance.sh
require_grep 'MARKDOWN_RENDER_CACHE_CAPACITY: usize = 1024' vac-rs/crates/surfaces/tui/src/markdown_render.rs
require_grep 'HashMap<u64, Text' vac-rs/crates/surfaces/tui/src/markdown_render.rs
require_grep 'VecDeque<u64>' vac-rs/crates/surfaces/tui/src/markdown_render.rs
require_grep 'fn has_runtime_status_files' vac-rs/crates/surfaces/tui/src/operator_console.rs
require_grep 'Duration::from_secs\(5\)' vac-rs/crates/surfaces/tui/src/operator_console.rs
require_grep 'return None;' vac-rs/crates/surfaces/tui/src/operator_console.rs
reject_grep 'DO NOT USE' vac-rs/crates/surfaces/tui/src/slash_command.rs
require_grep 'SlashCommand::MemoryDrop \| SlashCommand::MemoryUpdate => false' vac-rs/crates/surfaces/tui/src/slash_command.rs
require_grep 'Use /memories for supported local memory settings' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_grep 'Realtime audio settings are unavailable' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_grep 'actions: /workflow run <id>' vac-rs/crates/surfaces/tui/src/workflow_browser.rs
require_grep 'Try enabling OSC 52' vac-rs/crates/surfaces/tui/src/chatwidget/split_011_handle_key_event.rs
require_grep 'VAC_TUI_PROFILE_STARTUP' vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs
require_file docs/tui/PERFORMANCE_BENCHMARK_HARNESS.md

# Snapshot-8 audit closures: CL-1..CL-5 and P-6 must be source-effective, not marker-only.
require_grep 'style_epoch: std::cell::Cell<u64>' vac-rs/crates/surfaces/tui/src/chatwidget/split_002_new.rs
require_grep 'fn bump_style_epoch' vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_014_impl.rs
require_grep 'style_revision,?\s*$' vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_025_epilogue.rs
reject_grep 'style_revision: self\.last_rendered_width' vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_025_epilogue.rs
require_grep 'HEIGHT_CACHE_MAX_ENTRIES: usize = 4096' vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs
require_grep 'HEIGHT_CACHE_MAX_REVISIONS_PER_CELL: usize = 3' vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs
require_grep 'cell_height_evictions' vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs
require_grep 'HeightPrefixBuildKey' vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs
require_grep 'prefix_rebuild_skipped' vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs
require_grep 'rebuild_prefix_if_changed' vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_025_epilogue.rs
reject_grep 'windowed_render_plan\(total_cells, 0' vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_025_epilogue.rs
require_grep 'current_transcript_scroll_top' vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_025_epilogue.rs
require_grep 'spans_emitted' vac-rs/crates/surfaces/tui/src/shimmer.rs
require_grep 'Span::raw\(text\.to_owned\(\)\)' vac-rs/crates/surfaces/tui/src/shimmer.rs
reject_grep 'Span::raw\(ch\.to_string\(\)\)' vac-rs/crates/surfaces/tui/src/shimmer.rs
reject_grep 'chars\(\)\.collect::<Vec' vac-rs/crates/surfaces/tui/src/shimmer.rs
require_grep 'struct StartupGraphExecutor' vac-rs/crates/surfaces/tui/src/startup_task_graph.rs
require_grep 'thread::scope' vac-rs/crates/surfaces/tui/src/startup_task_graph.rs
require_grep 'non_blocking_tasks_parallelized: true' vac-rs/crates/surfaces/tui/src/startup_task_graph.rs
printf 'tui perf/ux findings gate: PASS
'
