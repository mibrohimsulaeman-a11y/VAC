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

pub(crate) const HEIGHT_CACHE_MAX_ENTRIES: usize = 4096;
pub(crate) const HEIGHT_CACHE_MAX_REVISIONS_PER_CELL: usize = 3;

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
                .saturating_add(height as u32);
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
        let scroll_bottom = scroll_top.saturating_add(viewport_height as u32);
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
        self.metrics.cell_height_entries = self.cell_heights.len() as u64;
        height
    }

    pub(crate) fn rebuild_prefix_if_changed(
        &mut self,
        key: HeightPrefixBuildKey,
        heights: impl IntoIterator<Item = u16>,
    ) -> bool {
        if self.prefix_build_key == Some(key) {
            self.metrics.prefix_rebuild_skipped = self.metrics.prefix_rebuild_skipped.saturating_add(1);
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
        self.prefix.visible_range(scroll_top, viewport_height, overscan)
    }

    pub(crate) fn cell_top(&self, index: usize) -> u32 {
        self.prefix.cell_top(index)
    }

    pub(crate) fn record_windowed_render(&mut self, visible: usize, skipped: usize) {
        self.metrics.visible_cells_rendered = self
            .metrics
            .visible_cells_rendered
            .saturating_add(visible as u64);
        self.metrics.full_transcript_cells_skipped = self
            .metrics
            .full_transcript_cells_skipped
            .saturating_add(skipped as u64);
    }

    pub(crate) fn metrics(&self) -> HeightCacheMetrics {
        let mut metrics = self.metrics;
        metrics.cell_height_entries = self.cell_heights.len() as u64;
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
                self.metrics.stale_revision_pruned = self.metrics.stale_revision_pruned.saturating_add(1);
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
                self.metrics.cell_height_evictions = self.metrics.cell_height_evictions.saturating_add(1);
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
        let first = CellHeightKey { cell_id: 1, width: 80, content_revision: 7, style_revision: 1 };
        let second = CellHeightKey { style_revision: 2, ..first };
        assert_ne!(first, second);
    }

    #[test]
    fn cell_height_cache_enforces_revision_cap() {
        let mut cache = DesiredHeightCache::default();
        for revision in 0..8 {
            let key = CellHeightKey { cell_id: 1, width: 80, content_revision: revision, style_revision: 1 };
            assert_eq!(cache.get_cell_or_compute(key, || 1), 1);
        }
        assert!(cache.cell_heights.len() <= HEIGHT_CACHE_MAX_REVISIONS_PER_CELL);
        assert!(cache.metrics().stale_revision_pruned > 0);
    }

    #[test]
    fn identical_prefix_build_key_skips_rebuild() {
        let mut cache = DesiredHeightCache::default();
        let key = HeightPrefixBuildKey { width: 80, transcript_revision: 1, style_revision: 1, cell_count: 3 };
        assert!(cache.rebuild_prefix_if_changed(key, [1, 2, 3]));
        assert!(!cache.rebuild_prefix_if_changed(key, [1, 2, 3]));
        assert_eq!(cache.metrics().prefix_rebuild_skipped, 1);
    }

    #[test]
    fn non_zero_scroll_changes_visible_window() {
        let mut cache = DesiredHeightCache::default();
        let key = HeightPrefixBuildKey { width: 80, transcript_revision: 1, style_revision: 1, cell_count: 5 };
        cache.rebuild_prefix_if_changed(key, [5, 5, 5, 5, 5]);
        let top = cache.windowed_render_plan(5, 0, 5, 0);
        let scrolled = cache.windowed_render_plan(5, 10, 5, 0);
        assert_ne!(top.visible, scrolled.visible);
        assert_eq!(scrolled.scroll_top, 10);
    }
}
