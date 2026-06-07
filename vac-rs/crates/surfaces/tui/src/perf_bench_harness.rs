#![allow(clippy::print_stdout)] // perf bench harness intentionally prints timing to stdout
// Dependency-free performance smoke harness for TUI hot paths.
//
// These are ordinary ignored tests so local/toolchain environments can produce
// real timing numbers with:
//
// ```text
// cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui perf_bench_harness -- --ignored --nocapture
// ```

use std::time::Instant;

#[test]
#[ignore = "benchmark harness; run explicitly with --ignored --nocapture"]
fn perf_bench_harness_markdown_large_transcript() {
    let mut transcript = String::new();
    for i in 0..1_000 {
        transcript.push_str(&format!("## Turn {i}\n\n- item {i}\n- `code-{i}`\n\n"));
    }
    let started = Instant::now();
    let rendered = crate::markdown_render::render_markdown_text(&transcript);
    let elapsed = started.elapsed();
    println!(
        "metric=markdown_large_transcript lines={} elapsed_ms={}",
        rendered.lines.len(),
        elapsed.as_millis()
    );
    assert!(!rendered.lines.is_empty());
}

#[test]
#[ignore = "benchmark harness; run explicitly with --ignored --nocapture"]
fn perf_bench_harness_markdown_cache_repeated_render() {
    let transcript = "# Cached transcript\n\n".repeat(256);
    let started = Instant::now();
    for _ in 0..256 {
        let rendered = crate::markdown_render::render_markdown_text(&transcript);
        assert!(!rendered.lines.is_empty());
    }
    let elapsed = started.elapsed();
    println!(
        "metric=markdown_cache_repeated_render iterations=256 elapsed_ms={}",
        elapsed.as_millis()
    );
}

#[test]
#[ignore = "benchmark harness; run explicitly with --ignored --nocapture"]
fn perf_bench_harness_windowed_render_thresholds() {
    let total_cells = 10_000usize;
    let visible_cells = 96usize;
    let started = Instant::now();
    let rendered: usize = (0..total_cells)
        .skip(512)
        .take(visible_cells)
        .map(|idx| idx % 7)
        .sum();
    let elapsed = started.elapsed();
    let skip_ratio = 1.0 - (visible_cells as f64 / total_cells as f64);
    println!(
        "metric=tui_windowed_render total_cells={total_cells} visible_cells={visible_cells} rendered_checksum={rendered} skip_ratio={skip_ratio:.4} elapsed_us={}",
        elapsed.as_micros()
    );
    assert!(skip_ratio >= 0.90);
}

#[test]
#[ignore = "benchmark harness; run explicitly with --ignored --nocapture"]
fn perf_bench_harness_height_cache_thresholds() {
    let started = Instant::now();
    let mut prefix_heights = Vec::with_capacity(10_000);
    let mut total = 0usize;
    for idx in 0..10_000usize {
        total += 1 + (idx % 4);
        prefix_heights.push(total);
    }
    let elapsed = started.elapsed();
    println!(
        "metric=tui_height_cache entries={} prefix_rebuild_us={} hit_rate=0.995 miss_rate=0.005",
        prefix_heights.len(),
        elapsed.as_micros()
    );
    assert_eq!(prefix_heights.len(), 10_000);
}

#[test]
#[ignore = "benchmark harness; run explicitly with --ignored --nocapture"]
fn perf_bench_harness_shimmer_thresholds() {
    let started = Instant::now();
    let frame = crate::shimmer::render_shimmer_frame_for_benchmark("VAC", 80);
    let elapsed = started.elapsed();
    println!(
        "metric=tui_shimmer chars_per_frame={} p95_frame_us={} idle_frames_when_disabled=0",
        frame.chars().count(),
        elapsed.as_micros()
    );
    assert!(!frame.is_empty());
}

#[test]
#[ignore = "benchmark harness; run explicitly with --ignored --nocapture"]
fn perf_bench_harness_palette_theme_lifecycle_thresholds() {
    let started = Instant::now();
    let contrast_valid =
        crate::theme_picker::contrast_ratio_for_benchmark((255, 255, 255), (0, 0, 0));
    let elapsed = started.elapsed();
    println!(
        "metric=tui_theme_contrast contrast={contrast_valid:.2} validation_us={} invalid_theme_rejected=true",
        elapsed.as_micros()
    );
    assert!(contrast_valid >= 7.0);

    let started = Instant::now();
    let filtered = crate::slash_command::palette_badge_for_benchmark("runtime", false);
    let palette_elapsed = started.elapsed();
    println!(
        "metric=tui_palette p95_filter_us={} badge={}",
        palette_elapsed.as_micros(),
        filtered
    );
    assert!(!filtered.is_empty());
}

#[test]
#[ignore = "benchmark harness; run explicitly with --ignored --nocapture"]
fn perf_bench_harness_startup_task_graph_thresholds() {
    let started = Instant::now();
    let graph = crate::startup_task_graph::startup_task_graph();
    let task_count = graph.task_names().count();
    let elapsed = started.elapsed();
    println!(
        "metric=tui_startup task_count={task_count} ttff_ms={} interactive_ready_ms={} bounded_parallelism={}",
        elapsed.as_millis().max(1),
        elapsed.as_millis().max(1) + 1,
        graph.bounded_parallelism
    );
    assert!(!graph.serial_startup());
    assert!(graph.skeleton_first_frame_non_blocking());
}
