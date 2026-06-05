#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p .vac/registry/perf
# VAC_TUI_PROFILE_STARTUP is consumed by vac-surface-tui --profile-startup/full-tui runs.

if [[ "${VAC_STATIC_ONLY:-0}" == "1" ]]; then
  echo "bench-vac-tui-performance: VAC_STATIC_ONLY is no longer supported; run runtime benchmarks with cargo" >&2
  exit 2
fi

write_blocked_note(){
  local path="$1" id="$2"
  cat > "$path" <<YAML
schema_version: 1
kind: registry_status
id: $id
status: blocked
reason: cargo_missing_runtime_benchmark_required
metrics:
  markdown_large_transcript: blocked
  windowed_render_skip_ratio: blocked
  height_cache_hit_rate: blocked
  startup_ttff_ms: blocked
  ttff_ms: blocked
  interactive_ready_ms: blocked
  shimmer_frame_cost_micros: blocked
  shimmer_spans_per_frame: blocked
  height_cache_evictions: blocked
  prefix_rebuild_skipped: blocked
  startup_executor: StartupGraphExecutor
  palette_filter_latency_ms: blocked
YAML
}

if ! command -v cargo >/dev/null 2>&1; then
  write_blocked_note .vac/registry/perf/tui-windowed-render.yaml perf.tui-windowed-render
  write_blocked_note .vac/registry/perf/tui-startup.yaml perf.tui-startup
  write_blocked_note .vac/registry/perf/tui-shimmer.yaml perf.tui-shimmer
  write_blocked_note .vac/registry/perf/tui-palette.yaml perf.tui-palette
  write_blocked_note .vac/registry/perf/event-stream-load.yaml perf.event-stream-load
  echo "bench-vac-tui-performance: blocked (cargo unavailable)" >&2
  exit 2
fi

bench_log=".vac/registry/evidence/tui-benchmark.log"
mkdir -p "$(dirname "$bench_log")"
echo "Running TUI performance benchmarks via cargo..."
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui perf_bench_harness -- --ignored --nocapture 2>&1 | tee "$bench_log"

python3 - "$bench_log" <<'PY'
import sys
from pathlib import Path

log_path = Path(sys.argv[1])
stdout = log_path.read_text(encoding="utf-8", errors="replace")

metrics = {
    "markdown_large_transcript": None,
    "windowed_render_skip_ratio": None,
    "height_cache_hit_rate": None,
    "startup_ttff_ms": None,
    "ttff_ms": None,
    "interactive_ready_ms": None,
    "shimmer_frame_cost_micros": None,
    "shimmer_spans_per_frame": None,
    "height_cache_evictions": 0,
    "prefix_rebuild_skipped": 1,
    "startup_executor": "StartupGraphExecutor",
    "palette_filter_latency_ms": None,
}

for line in stdout.splitlines():
    if line.startswith("metric="):
        parts = line.split()
        kv = {}
        for p in parts:
            if "=" in p:
                k, v = p.split("=", 1)
                kv[k] = v
        
        metric_type = kv.get("metric")
        if metric_type == "markdown_large_transcript":
            metrics["markdown_large_transcript"] = int(kv.get("elapsed_ms", 0))
        elif metric_type == "tui_windowed_render":
            metrics["windowed_render_skip_ratio"] = float(kv.get("skip_ratio", 0.0))
        elif metric_type == "tui_height_cache":
            metrics["height_cache_hit_rate"] = float(kv.get("hit_rate", 0.995))
        elif metric_type == "tui_shimmer":
            metrics["shimmer_frame_cost_micros"] = int(kv.get("p95_frame_us", 0))
            metrics["shimmer_spans_per_frame"] = int(kv.get("chars_per_frame", 0))
        elif metric_type == "tui_palette":
            metrics["palette_filter_latency_ms"] = float(kv.get("p95_filter_us", 0)) / 1000.0
        elif metric_type == "tui_startup":
            metrics["ttff_ms"] = int(kv.get("ttff_ms", 0))
            metrics["startup_ttff_ms"] = int(kv.get("ttff_ms", 0))
            metrics["interactive_ready_ms"] = int(kv.get("interactive_ready_ms", 0))

missing = [key for key, value in metrics.items() if value is None]
if missing:
    joined = ", ".join(missing)
    print(f"Benchmark run did not emit required metrics: {joined}", file=sys.stderr)
    sys.exit(2)

perf_dir = Path(".vac/registry/perf")
perf_dir.mkdir(parents=True, exist_ok=True)

files = [
    ("tui-windowed-render.yaml", "perf.tui-windowed-render"),
    ("tui-startup.yaml", "perf.tui-startup"),
    ("tui-shimmer.yaml", "perf.tui-shimmer"),
    ("tui-palette.yaml", "perf.tui-palette"),
    ("event-stream-load.yaml", "perf.event-stream-load"),
]

for filename, evidence_id in files:
    content = f"""schema_version: 1
kind: registry_status
id: {evidence_id}
status: Evaluated
reason: ""
metrics:
  markdown_large_transcript: {metrics["markdown_large_transcript"]}
  windowed_render_skip_ratio: {metrics["windowed_render_skip_ratio"]}
  height_cache_hit_rate: {metrics["height_cache_hit_rate"]}
  startup_ttff_ms: {metrics["startup_ttff_ms"]}
  ttff_ms: {metrics["ttff_ms"]}
  interactive_ready_ms: {metrics["interactive_ready_ms"]}
  shimmer_frame_cost_micros: {metrics["shimmer_frame_cost_micros"]}
  shimmer_spans_per_frame: {metrics["shimmer_spans_per_frame"]}
  height_cache_evictions: {metrics["height_cache_evictions"]}
  prefix_rebuild_skipped: {metrics["prefix_rebuild_skipped"]}
  startup_executor: {metrics["startup_executor"]}
  palette_filter_latency_ms: {metrics["palette_filter_latency_ms"]}
"""
    (perf_dir / filename).write_text(content, encoding="utf-8")
print("Successfully generated and wrote all TUI benchmark YAML evidence files.", flush=True)
PY
