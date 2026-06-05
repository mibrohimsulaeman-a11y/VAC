// Width-keyed desired-height cache for the chat surface.
//
// O5/O6 performance closeout keeps this cache across draws. It records whole
// surface height plus per-cell height slots that can be filled by windowed
// render paths. The current render path can still fall back to the aggregate
// `desired_height(width)` computation, but every lookup records hit/miss and a
// stable prefix-index API is present for the transcript renderer to skip cells
// outside the visible window.

use std::collections::BTreeMap;
use std::collections::VecDeque;

use ratatui::text::Line;

pub(crate) const HEIGHT_CACHE_MAX_ENTRIES: usize = 4096;
pub(crate) const HEIGHT_CACHE_MAX_REVISIONS_PER_CELL: usize = 3;
pub(crate) const RENDERED_LINES_CACHE_MAX_ENTRIES: usize = 2048;
pub(crate) const RENDERED_LINES_CACHE_MAX_REVISIONS_PER_CELL: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CellHeightKey {
    pub(crate) cell_id: u64,
    pub(crate) width: u16,
    pub(crate) content_revision: u64,
    pub(crate) style_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeightPrefixBuildKey {
    pub(crate) width: u16,
    pub(crate) transcript_revision: u64,
    pub(crate) style_revision: u64,
    pub(crate) cell_count: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HeightCacheMetrics {
    pub(crate) hits: u64,
    pub(crate) misses: u64,
    pub(crate) relayouts: u64,
    pub(crate) visible_cells_rendered: u64,
    pub(crate) full_transcript_cells_skipped: u64,
    pub(crate) cell_height_entries: u64,
    pub(crate) cell_height_evictions: u64,
    pub(crate) stale_revision_pruned: u64,
    pub(crate) prefix_rebuilds: u64,
    pub(crate) prefix_rebuild_skipped: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RenderedLinesKey {
    pub(crate) cell_id: u64,
    pub(crate) width: u16,
    pub(crate) style_revision: u64,
    pub(crate) content_revision: u64,
    pub(crate) active_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderedLinesValue {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) height: u16,
    pub(crate) bytes_estimate: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RenderedLinesCacheMetrics {
    pub(crate) hits: u64,
    pub(crate) misses: u64,
    pub(crate) evictions: u64,
    pub(crate) entries: u64,
    pub(crate) bytes_estimate: u64,
    pub(crate) highlight_recomputes_avoided: u64,
    pub(crate) stale_revision_pruned: u64,
}

#[derive(Debug, Default)]
pub(crate) struct RenderedLinesCache {
    lines: BTreeMap<RenderedLinesKey, RenderedLinesValue>,
    order: VecDeque<RenderedLinesKey>,
    metrics: RenderedLinesCacheMetrics,
}

impl RenderedLinesCache {
    pub(crate) fn get_or_compute(
        &mut self,
        key: RenderedLinesKey,
        compute: impl FnOnce() -> Vec<Line<'static>>,
    ) -> RenderedLinesValue {
        if let Some(value) = self.lines.get(&key).cloned() {
            self.touch(key);
            self.metrics.hits = self.metrics.hits.saturating_add(1);
            self.metrics.highlight_recomputes_avoided =
                self.metrics.highlight_recomputes_avoided.saturating_add(1);
            return value;
        }

        self.metrics.misses = self.metrics.misses.saturating_add(1);
        let lines = compute();
        let value = RenderedLinesValue {
            height: u16::try_from(lines.len()).unwrap_or(u16::MAX),
            bytes_estimate: estimate_lines_bytes(&lines),
            lines,
        };
        self.lines.insert(key, value.clone());
        self.order.push_back(key);
        self.prune_stale_revisions_for(key);
        self.enforce_entry_cap();
        self.refresh_entry_metrics();
        value
    }

    pub(crate) fn metrics(&self) -> RenderedLinesCacheMetrics {
        let mut metrics = self.metrics;
        metrics.entries = u64::try_from(self.lines.len()).unwrap_or(u64::MAX);
        metrics.bytes_estimate = self
            .lines
            .values()
            .map(|value| u64::try_from(value.bytes_estimate).unwrap_or(u64::MAX))
            .sum();
        metrics
    }

    pub(crate) fn clear(&mut self) {
        self.lines.clear();
        self.order.clear();
        self.metrics.entries = 0;
        self.metrics.bytes_estimate = 0;
    }

    fn prune_stale_revisions_for(&mut self, inserted_key: RenderedLinesKey) {
        let mut revisions = self
            .lines
            .keys()
            .copied()
            .filter(|key| key.cell_id == inserted_key.cell_id && key.width == inserted_key.width)
            .collect::<Vec<_>>();
        if revisions.len() <= RENDERED_LINES_CACHE_MAX_REVISIONS_PER_CELL {
            return;
        }
        revisions.sort_by_key(|key| {
            (
                key.content_revision,
                key.active_revision,
                key.style_revision,
            )
        });
        while revisions.len() > RENDERED_LINES_CACHE_MAX_REVISIONS_PER_CELL {
            let stale = revisions.remove(0);
            if self.lines.remove(&stale).is_some() {
                self.metrics.stale_revision_pruned =
                    self.metrics.stale_revision_pruned.saturating_add(1);
            }
        }
        self.retain_live_order_keys();
    }

    fn enforce_entry_cap(&mut self) {
        while self.lines.len() > RENDERED_LINES_CACHE_MAX_ENTRIES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if self.lines.remove(&oldest).is_some() {
                self.metrics.evictions = self.metrics.evictions.saturating_add(1);
            }
        }
    }

    fn retain_live_order_keys(&mut self) {
        self.order.retain(|key| self.lines.contains_key(key));
    }

    fn touch(&mut self, key: RenderedLinesKey) {
        if let Some(position) = self.order.iter().position(|existing| *existing == key) {
            self.order.remove(position);
        }
        self.order.push_back(key);
    }

    fn refresh_entry_metrics(&mut self) {
        self.metrics.entries = u64::try_from(self.lines.len()).unwrap_or(u64::MAX);
        self.metrics.bytes_estimate = self
            .lines
            .values()
            .map(|value| u64::try_from(value.bytes_estimate).unwrap_or(u64::MAX))
            .sum();
    }
}

fn estimate_lines_bytes(lines: &[Line<'static>]) -> usize {
    lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.len())
        .sum()
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HeightPrefixIndex {
    heights: Vec<u16>,
    prefix: Vec<u32>,
}

impl HeightPrefixIndex {
    pub(crate) fn rebuild(&mut self, heights: impl IntoIterator<Item = u16>) {
        self.heights.clear();
        self.prefix.clear();
        self.prefix.push(0);
        for height in heights {
            self.heights.push(height);
            let next = self
                .prefix
                .last()
                .copied()
                .unwrap_or_default()
                .saturating_add(u32::from(height));
            self.prefix.push(next);
        }
    }

    pub(crate) fn visible_range(
        &self,
        scroll_top: u32,
        viewport_height: u16,
        overscan: usize,
    ) -> std::ops::Range<usize> {
        if self.heights.is_empty() || viewport_height == 0 {
            return 0..0;
        }
        let scroll_bottom = scroll_top.saturating_add(u32::from(viewport_height));
        let mut start = 0usize;
        while start + 1 < self.prefix.len() && self.prefix[start + 1] <= scroll_top {
            start += 1;
        }
        let mut end = start;
        while end < self.heights.len() && self.prefix[end] < scroll_bottom {
            end += 1;
        }
        start.saturating_sub(overscan)..(end + overscan).min(self.heights.len())
    }

    pub(crate) fn cell_top(&self, index: usize) -> u32 {
        self.prefix.get(index).copied().unwrap_or_default()
    }
}

#[derive(Debug, Default)]
pub(crate) struct DesiredHeightCache {
    width: Option<u16>,
    revision: u64,
    height: u16,
    cell_heights: BTreeMap<CellHeightKey, u16>,
    cell_height_order: VecDeque<CellHeightKey>,
    prefix: HeightPrefixIndex,
    prefix_build_key: Option<HeightPrefixBuildKey>,
    metrics: HeightCacheMetrics,
}

impl DesiredHeightCache {
    pub(crate) fn get_or_compute(
        &mut self,
        width: u16,
        revision: u64,
        compute: impl FnOnce() -> u16,
    ) -> u16 {
        if self.width == Some(width) && self.revision == revision {
            self.metrics.hits = self.metrics.hits.saturating_add(1);
            return self.height;
        }
        self.metrics.misses = self.metrics.misses.saturating_add(1);
        self.metrics.relayouts = self.metrics.relayouts.saturating_add(1);
        let height = compute();
        self.width = Some(width);
        self.revision = revision;
        self.height = height;
        height
    }

    pub(crate) fn get_cell_or_compute(
        &mut self,
        key: CellHeightKey,
        compute: impl FnOnce() -> u16,
    ) -> u16 {
        if let Some(height) = self.cell_heights.get(&key).copied() {
            self.metrics.hits = self.metrics.hits.saturating_add(1);
            return height;
        }
        self.metrics.misses = self.metrics.misses.saturating_add(1);
        self.metrics.relayouts = self.metrics.relayouts.saturating_add(1);
        let height = compute();
        self.cell_heights.insert(key, height);
        self.cell_height_order.push_back(key);
        self.prune_stale_revisions_for(key);
        self.enforce_entry_cap();
        self.metrics.cell_height_entries =
            u64::try_from(self.cell_heights.len()).unwrap_or(u64::MAX);
        height
    }

    pub(crate) fn rebuild_prefix_if_changed(
        &mut self,
        key: HeightPrefixBuildKey,
        heights: impl IntoIterator<Item = u16>,
    ) -> bool {
        if self.prefix_build_key == Some(key) {
            self.metrics.prefix_rebuild_skipped =
                self.metrics.prefix_rebuild_skipped.saturating_add(1);
            return false;
        }
        self.prefix.rebuild(heights);
        self.prefix_build_key = Some(key);
        self.metrics.prefix_rebuilds = self.metrics.prefix_rebuilds.saturating_add(1);
        true
    }

    pub(crate) fn visible_range(
        &self,
        scroll_top: u32,
        viewport_height: u16,
        overscan: usize,
    ) -> std::ops::Range<usize> {
        self.prefix
            .visible_range(scroll_top, viewport_height, overscan)
    }

    pub(crate) fn cell_top(&self, index: usize) -> u32 {
        self.prefix.cell_top(index)
    }

    pub(crate) fn record_windowed_render(&mut self, visible: usize, skipped: usize) {
        self.metrics.visible_cells_rendered = self
            .metrics
            .visible_cells_rendered
            .saturating_add(u64::try_from(visible).unwrap_or(u64::MAX));
        self.metrics.full_transcript_cells_skipped = self
            .metrics
            .full_transcript_cells_skipped
            .saturating_add(u64::try_from(skipped).unwrap_or(u64::MAX));
    }

    pub(crate) fn metrics(&self) -> HeightCacheMetrics {
        let mut metrics = self.metrics;
        metrics.cell_height_entries = u64::try_from(self.cell_heights.len()).unwrap_or(u64::MAX);
        metrics
    }

    pub(crate) fn clear(&mut self) {
        self.width = None;
        self.height = 0;
        self.cell_heights.clear();
        self.cell_height_order.clear();
        self.prefix.rebuild(std::iter::empty());
        self.prefix_build_key = None;
        self.metrics.cell_height_entries = 0;
    }

    fn prune_stale_revisions_for(&mut self, inserted_key: CellHeightKey) {
        let mut revisions = self
            .cell_heights
            .keys()
            .copied()
            .filter(|key| key.cell_id == inserted_key.cell_id && key.width == inserted_key.width)
            .collect::<Vec<_>>();
        if revisions.len() <= HEIGHT_CACHE_MAX_REVISIONS_PER_CELL {
            return;
        }
        revisions.sort_by_key(|key| (key.content_revision, key.style_revision));
        while revisions.len() > HEIGHT_CACHE_MAX_REVISIONS_PER_CELL {
            let stale = revisions.remove(0);
            if self.cell_heights.remove(&stale).is_some() {
                self.metrics.stale_revision_pruned =
                    self.metrics.stale_revision_pruned.saturating_add(1);
            }
        }
        self.retain_live_order_keys();
    }

    fn enforce_entry_cap(&mut self) {
        while self.cell_heights.len() > HEIGHT_CACHE_MAX_ENTRIES {
            let Some(oldest) = self.cell_height_order.pop_front() else {
                break;
            };
            if self.cell_heights.remove(&oldest).is_some() {
                self.metrics.cell_height_evictions =
                    self.metrics.cell_height_evictions.saturating_add(1);
            }
        }
    }

    fn retain_live_order_keys(&mut self) {
        self.cell_height_order
            .retain(|key| self.cell_heights.contains_key(key));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowedTranscriptRenderPlan {
    pub(crate) visible: std::ops::Range<usize>,
    pub(crate) skipped_before: usize,
    pub(crate) skipped_after: usize,
    pub(crate) scroll_top: u32,
    pub(crate) first_visible_top: u32,
}

impl DesiredHeightCache {
    pub(crate) fn windowed_render_plan(
        &mut self,
        total_cells: usize,
        scroll_top: u32,
        viewport_height: u16,
        overscan: usize,
    ) -> WindowedTranscriptRenderPlan {
        let visible = self.visible_range(scroll_top, viewport_height, overscan);
        let skipped_before = visible.start.min(total_cells);
        let skipped_after = total_cells.saturating_sub(visible.end.min(total_cells));
        let first_visible_top = self.cell_top(visible.start);
        self.record_windowed_render(visible.len(), skipped_before.saturating_add(skipped_after));
        WindowedTranscriptRenderPlan {
            visible,
            skipped_before,
            skipped_after,
            scroll_top,
            first_visible_top,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_style_revision_is_part_of_cell_key() {
        let first = CellHeightKey {
            cell_id: 1,
            width: 80,
            content_revision: 7,
            style_revision: 1,
        };
        let second = CellHeightKey {
            style_revision: 2,
            ..first
        };
        assert_ne!(first, second);
    }

    #[test]
    fn rendered_lines_key_tracks_style_and_active_revision() {
        let first = RenderedLinesKey {
            cell_id: 1,
            width: 80,
            style_revision: 1,
            content_revision: 7,
            active_revision: 7,
        };
        assert_ne!(
            first,
            RenderedLinesKey {
                style_revision: 2,
                ..first
            }
        );
        assert_ne!(
            first,
            RenderedLinesKey {
                active_revision: 8,
                ..first
            }
        );
    }

    #[test]
    fn rendered_lines_cache_hits_do_not_recompute_static_cells() {
        let mut cache = RenderedLinesCache::default();
        let key = RenderedLinesKey {
            cell_id: 1,
            width: 80,
            style_revision: 1,
            content_revision: 7,
            active_revision: 7,
        };
        let mut computes = 0usize;
        let first = cache.get_or_compute(key, || {
            computes = computes.saturating_add(1);
            vec![Line::from("first"), Line::from("second")]
        });
        let second = cache.get_or_compute(key, || {
            computes = computes.saturating_add(1);
            vec![Line::from("replacement")]
        });
        assert_eq!(computes, 1);
        assert_eq!(first.lines, second.lines);
        assert_eq!(second.height, 2);
        assert!(second.bytes_estimate >= "firstsecond".len());
        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.highlight_recomputes_avoided, 1);
    }

    #[test]
    fn rendered_lines_cache_eviction_keeps_recent_hits() {
        let mut cache = RenderedLinesCache::default();
        for cell_id in 0..RENDERED_LINES_CACHE_MAX_ENTRIES {
            let key = RenderedLinesKey {
                cell_id: u64::try_from(cell_id).unwrap_or(u64::MAX),
                width: 80,
                style_revision: 1,
                content_revision: 1,
                active_revision: 1,
            };
            cache.get_or_compute(key, || vec![Line::from("cached")]);
        }
        let first_key = RenderedLinesKey {
            cell_id: 0,
            width: 80,
            style_revision: 1,
            content_revision: 1,
            active_revision: 1,
        };
        let second_key = RenderedLinesKey {
            cell_id: 1,
            ..first_key
        };
        cache.get_or_compute(first_key, || vec![Line::from("recent")]);
        let overflow_key = RenderedLinesKey {
            cell_id: u64::try_from(RENDERED_LINES_CACHE_MAX_ENTRIES).unwrap_or(u64::MAX),
            ..first_key
        };
        cache.get_or_compute(overflow_key, || vec![Line::from("overflow")]);

        assert!(cache.lines.contains_key(&first_key));
        assert!(!cache.lines.contains_key(&second_key));
        assert_eq!(cache.metrics().evictions, 1);
        assert_eq!(
            cache.metrics().entries,
            u64::try_from(RENDERED_LINES_CACHE_MAX_ENTRIES).unwrap_or(u64::MAX)
        );
    }

    #[test]
    fn cell_height_cache_enforces_revision_cap() {
        let mut cache = DesiredHeightCache::default();
        for revision in 0..8 {
            let key = CellHeightKey {
                cell_id: 1,
                width: 80,
                content_revision: revision,
                style_revision: 1,
            };
            assert_eq!(cache.get_cell_or_compute(key, || 1), 1);
        }
        assert!(cache.cell_heights.len() <= HEIGHT_CACHE_MAX_REVISIONS_PER_CELL);
        assert!(cache.metrics().stale_revision_pruned > 0);
    }

    #[test]
    fn identical_prefix_build_key_skips_rebuild() {
        let mut cache = DesiredHeightCache::default();
        let key = HeightPrefixBuildKey {
            width: 80,
            transcript_revision: 1,
            style_revision: 1,
            cell_count: 3,
        };
        assert!(cache.rebuild_prefix_if_changed(key, [1, 2, 3]));
        assert!(!cache.rebuild_prefix_if_changed(key, [1, 2, 3]));
        assert_eq!(cache.metrics().prefix_rebuild_skipped, 1);
    }

    #[test]
    fn non_zero_scroll_changes_visible_window() {
        let mut cache = DesiredHeightCache::default();
        let key = HeightPrefixBuildKey {
            width: 80,
            transcript_revision: 1,
            style_revision: 1,
            cell_count: 5,
        };
        cache.rebuild_prefix_if_changed(key, [5, 5, 5, 5, 5]);
        let top = cache.windowed_render_plan(5, 0, 5, 0);
        let scrolled = cache.windowed_render_plan(5, 10, 5, 0);
        assert_ne!(top.visible, scrolled.visible);
        assert_eq!(scrolled.scroll_top, 10);
    }
}
