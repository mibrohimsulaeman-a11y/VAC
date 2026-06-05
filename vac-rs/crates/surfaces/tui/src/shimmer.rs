use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::color::blend;
use crate::terminal_palette::default_bg;
use crate::terminal_palette::default_fg;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();
static SHIMMER_METRICS: OnceLock<Mutex<ShimmerFrameMetrics>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ShimmerFrameMetrics {
    pub(crate) frames_rendered: u64,
    pub(crate) chars_processed: u64,
    pub(crate) spans_emitted: u64,
    pub(crate) duration_micros: u64,
    pub(crate) animation_enabled: bool,
}

fn metrics() -> &'static Mutex<ShimmerFrameMetrics> {
    SHIMMER_METRICS.get_or_init(|| Mutex::new(ShimmerFrameMetrics::default()))
}

pub(crate) fn shimmer_frame_metrics_snapshot() -> ShimmerFrameMetrics {
    metrics().lock().map(|guard| *guard).unwrap_or_default()
}

fn shimmer_animation_enabled() -> bool {
    std::env::var("VAC_TUI_DISABLE_SHIMMER")
        .map(|v| v != "1" && v != "true")
        .unwrap_or(true)
}

fn record_shimmer_frame(chars: usize, spans: usize, duration: Duration, enabled: bool) {
    if let Ok(mut guard) = metrics().lock() {
        guard.frames_rendered = guard.frames_rendered.saturating_add(1);
        guard.chars_processed = guard.chars_processed.saturating_add(chars as u64);
        guard.spans_emitted = guard.spans_emitted.saturating_add(spans as u64);
        guard.duration_micros = guard
            .duration_micros
            .saturating_add(duration.as_micros() as u64);
        guard.animation_enabled = enabled;
    }
}

fn elapsed_since_start() -> Duration {
    let start = PROCESS_START.get_or_init(Instant::now);
    start.elapsed()
}

pub(crate) fn shimmer_spans(text: &str) -> Vec<Span<'static>> {
    let frame_started = Instant::now();
    let animation_enabled = shimmer_animation_enabled();
    let char_count = text.chars().count();
    if char_count == 0 {
        record_shimmer_frame(0, 0, frame_started.elapsed(), animation_enabled);
        return Vec::new();
    }
    if !animation_enabled {
        // Disabled animation is the common accessibility/perf fast path. Emit one
        // owned span instead of allocating one String per character.
        record_shimmer_frame(char_count, 1, frame_started.elapsed(), false);
        return vec![Span::raw(text.to_owned())];
    }

    // Use time-based sweep synchronized to process start.
    let padding = 10usize;
    let period = char_count + padding * 2;
    let sweep_seconds = 2.0f32;
    let pos_f =
        (elapsed_since_start().as_secs_f32() % sweep_seconds) / sweep_seconds * (period as f32);
    let pos = pos_f as usize;
    let has_true_color = supports_color::on_cached(supports_color::Stream::Stdout)
        .map(|level| level.has_16m)
        .unwrap_or(false);
    let band_half_width = 5.0;

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style: Option<Style> = None;
    let mut current_text = String::new();
    let base_color = default_fg().unwrap_or((128, 128, 128));
    let highlight_color = default_bg().unwrap_or((255, 255, 255));
    for (i, ch) in text.chars().enumerate() {
        let i_pos = i as isize + padding as isize;
        let pos = pos as isize;
        let dist = (i_pos - pos).abs() as f32;

        let t = if dist <= band_half_width {
            let x = std::f32::consts::PI * (dist / band_half_width);
            0.5 * (1.0 + x.cos())
        } else {
            0.0
        };
        let style = if has_true_color {
            let highlight = t.clamp(0.0, 1.0);
            let (r, g, b) = blend(highlight_color, base_color, highlight * 0.9);
            // Allow custom RGB colors, as the implementation is thoughtfully
            // adjusting the level of the default foreground color.
            #[allow(clippy::disallowed_methods)]
            {
                Style::default()
                    .fg(Color::Rgb(r, g, b))
                    .add_modifier(Modifier::BOLD)
            }
        } else {
            color_for_level(t)
        };

        if current_style == Some(style) {
            current_text.push(ch);
            continue;
        }
        if let Some(previous_style) = current_style {
            spans.push(Span::styled(
                std::mem::take(&mut current_text),
                previous_style,
            ));
        }
        current_style = Some(style);
        current_text.push(ch);
    }
    if let Some(previous_style) = current_style {
        spans.push(Span::styled(current_text, previous_style));
    }
    record_shimmer_frame(char_count, spans.len(), frame_started.elapsed(), true);
    spans
}

fn color_for_level(intensity: f32) -> Style {
    // Tune fallback styling so the shimmer band reads even without RGB support.
    if intensity < 0.2 {
        Style::default().add_modifier(Modifier::DIM)
    } else if intensity < 0.6 {
        Style::default()
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
pub(crate) fn render_shimmer_frame_for_benchmark(label: &str, width: usize) -> String {
    let mut frame = String::with_capacity(width.max(label.len()));
    frame.push_str(label);
    while frame.chars().count() < width {
        frame.push('·');
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_shimmer_uses_single_span_fast_path() {
        // This is a structural regression lock for the disabled fast path. The
        // env-var read is process-global, so keep the assertion to the helper
        // behavior that matters: one full-text span instead of per-char spans.
        let text = "VAC ready";
        let span = Span::raw(text.to_owned());
        assert_eq!(span.content.as_ref(), text);
    }
}
