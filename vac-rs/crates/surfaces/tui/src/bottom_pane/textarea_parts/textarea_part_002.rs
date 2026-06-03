impl WidgetRef for &TextArea {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let lines = self.wrapped_lines(area.width);
        self.render_lines(area, buf, &lines, 0..lines.len(), Style::default(), &[]);
    }
}

impl StatefulWidgetRef for &TextArea {
    type State = TextAreaState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area.height, &lines, state.scroll);
        state.scroll = scroll;

        let start = scroll as usize;
        let end = (scroll + area.height).min(lines.len() as u16) as usize;
        self.render_lines(area, buf, &lines, start..end, Style::default(), &[]);
    }
}

impl TextArea {
    pub(crate) fn render_ref_masked(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut TextAreaState,
        mask_char: char,
        base_style: Style,
    ) {
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area.height, &lines, state.scroll);
        state.scroll = scroll;

        let start = scroll as usize;
        let end = (scroll + area.height).min(lines.len() as u16) as usize;
        self.render_lines_masked(area, buf, &lines, start..end, mask_char, base_style);
    }

    /// Render the textarea with an explicit `base_style` applied to every cell,
    /// used by the Zellij code path to override inherited terminal styles.
    pub(crate) fn render_ref_styled(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut TextAreaState,
        base_style: Style,
    ) {
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area.height, &lines, state.scroll);
        state.scroll = scroll;

        let start = scroll as usize;
        let end = (scroll + area.height).min(lines.len() as u16) as usize;
        self.render_lines(area, buf, &lines, start..end, base_style, &[]);
    }

    /// Render the textarea with `base_style` plus additional render-only highlight ranges.
    ///
    /// Highlight ranges are byte ranges in `self.text`. They affect only the buffer rendering and
    /// do not mutate the editable text, cursor, element metadata, or wrapping cache.
    pub(crate) fn render_ref_styled_with_highlights(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut TextAreaState,
        base_style: Style,
        highlights: &[(Range<usize>, Style)],
    ) {
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area.height, &lines, state.scroll);
        state.scroll = scroll;

        let start = scroll as usize;
        let end = (scroll + area.height).min(lines.len() as u16) as usize;
        self.render_lines(area, buf, &lines, start..end, base_style, highlights);
    }

    fn render_lines(
        &self,
        area: Rect,
        buf: &mut Buffer,
        lines: &[Range<usize>],
        range: std::ops::Range<usize>,
        base_style: Style,
        highlights: &[(Range<usize>, Style)],
    ) {
        for (row, idx) in range.enumerate() {
            let r = &lines[idx];
            let y = area.y + row as u16;
            let line_range = r.start..r.end - 1;
            buf.set_style(Rect::new(area.x, y, area.width, 1), base_style);
            // Draw base line with the provided style.
            buf.set_string(area.x, y, &self.text[line_range.clone()], base_style);

            // Overlay styled segments for elements that intersect this line.
            for elem in &self.elements {
                // Compute overlap with displayed slice.
                let overlap_start = elem.range.start.max(line_range.start);
                let overlap_end = elem.range.end.min(line_range.end);
                if overlap_start >= overlap_end {
                    continue;
                }
                let styled = &self.text[overlap_start..overlap_end];
                let x_off = self.text[line_range.start..overlap_start].width() as u16;
                let style = base_style.fg(ratatui::style::Color::Cyan);
                buf.set_string(area.x + x_off, y, styled, style);
            }

            // Overlay render-only highlight ranges last so transient search highlighting remains
            // visible even when it intersects attachment placeholders or other styled elements.
            for (highlight_range, style) in highlights {
                let overlap_start = highlight_range.start.max(line_range.start);
                let overlap_end = highlight_range.end.min(line_range.end);
                if overlap_start >= overlap_end {
                    continue;
                }
                let highlighted = &self.text[overlap_start..overlap_end];
                let x_off = self.text[line_range.start..overlap_start].width() as u16;
                buf.set_string(area.x + x_off, y, highlighted, *style);
            }
        }
    }

    fn render_lines_masked(
        &self,
        area: Rect,
        buf: &mut Buffer,
        lines: &[Range<usize>],
        range: std::ops::Range<usize>,
        mask_char: char,
        base_style: Style,
    ) {
        for (row, idx) in range.enumerate() {
            let r = &lines[idx];
            let y = area.y + row as u16;
            let line_range = r.start..r.end - 1;
            buf.set_style(Rect::new(area.x, y, area.width, 1), base_style);
            let masked = self.text[line_range.clone()]
                .chars()
                .map(|_| mask_char)
                .collect::<String>();
            buf.set_string(area.x, y, &masked, base_style);
        }
    }
}

