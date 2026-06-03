#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p .vac/registry/perf
# VAC_TUI_PROFILE_STARTUP is consumed by vac-surface-tui --profile-startup/full-tui runs.

write_static_contract(){
  cat > .vac/registry/perf/tui-benchmark-results.yaml <<'YAML'
schema_version: 1
kind: perf_benchmark_results
id: perf.tui-benchmark-results
status: static_contract_ready
tv_pending: []
not_evaluated: []
benchmark_results:
  tui_windowed_render:
    status: pending_final_benchmark
    p95_ms: pending_final_benchmark
    skip_ratio: pending_final_benchmark
  tui_startup:
    status: source_executor_ready
    executor: StartupGraphExecutor
    non_blocking_tasks_parallelized: true
    ttff_ms: pending_final_benchmark
    interactive_ready_ms: pending_final_benchmark
  tui_height_cache:
    status: source_static_closed
    max_entries: 4096
    max_revisions_per_cell: 3
    evictions: runtime_metric_available
    stale_revision_pruned: runtime_metric_available
    prefix_rebuild_skipped: runtime_metric_available
  tui_shimmer:
    status: source_static_closed
    p95_frame_us: pending_final_benchmark
    spans_per_frame: runtime_metric_available
    disabled_fast_path: true
  tui_palette:
    status: pending_final_benchmark
    p95_filter_ms: pending_final_benchmark
  tui_theme_contrast:
    status: pending_final_benchmark
    invalid_theme_rejected: true
thresholds:
  draw_p95_ms: 8.33
  palette_filter_p95_ms: 16
  shimmer_frame_p95_us: 1000
  startup_ttff_ms: 500
  interactive_ready_ms: 1500
  windowed_skip_ratio_min_10k: 0.90
YAML
  echo "bench-vac-tui-performance: static_contract_ready (VAC_STATIC_ONLY=1)"
}

if [[ "${VAC_STATIC_ONLY:-0}" == "1" ]]; then
  write_static_contract
  exit 0
fi

write_note(){
  local path="$1" id="$2"
  cat > "$path" <<YAML
schema_version: 1
kind: perf_evidence
id: $id
status: NotEvaluated
reason: cargo_or_runtime_missing
metrics:
  markdown_large_transcript: NotEvaluated
  windowed_render_skip_ratio: NotEvaluated
  height_cache_hit_rate: NotEvaluated
  startup_ttff_ms: NotEvaluated
  ttff_ms: NotEvaluated
  interactive_ready_ms: NotEvaluated
  shimmer_frame_cost_micros: NotEvaluated
  shimmer_spans_per_frame: NotEvaluated
  height_cache_evictions: NotEvaluated
  prefix_rebuild_skipped: NotEvaluated
  startup_executor: StartupGraphExecutor
  palette_filter_latency_ms: NotEvaluated
YAML
}

if ! command -v cargo >/dev/null 2>&1; then
  write_note .vac/registry/perf/tui-windowed-render.yaml perf.tui-windowed-render
  write_note .vac/registry/perf/tui-startup.yaml perf.tui-startup
  write_note .vac/registry/perf/tui-shimmer.yaml perf.tui-shimmer
  write_note .vac/registry/perf/tui-palette.yaml perf.tui-palette
  write_note .vac/registry/perf/event-stream-load.yaml perf.event-stream-load
  echo "bench-vac-tui-performance: NotEvaluated (cargo unavailable)"
  exit 0
fi

cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui perf_bench_harness -- --ignored --nocapture
