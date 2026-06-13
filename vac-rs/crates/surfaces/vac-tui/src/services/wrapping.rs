use ratatui::text::Line;
use ratatui::text::Span;
use std::ops::Range;
use textwrap::Options;

/// Clone a borrowed ratatui `Line` into an owned `'static` line.
pub fn line_to_static(line: &Line<'_>) -> Line<'static> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .iter()
            .map(|s| Span {
                style: s.style,
                content: std::borrow::Cow::Owned(s.content.to_string()),
            })
            .collect(),
    }
}

/// Push a borrowed `Line` to an owned `Vec<Line<'static>>`.
pub fn push_owned_lines(src: &[Line<'_>], out: &mut Vec<Line<'static>>) {
    for l in src {
        out.push(line_to_static(l));
    }
}

fn previous_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn advance_by_chars(text: &str, start: usize, char_count: usize) -> usize {
    let start = previous_char_boundary(text, start);
    text[start..]
        .char_indices()
        .nth(char_count)
        .map(|(offset, _)| start + offset)
        .unwrap_or(text.len())
}

fn locate_wrapped_range(text: &str, rendered: &str, cursor: usize) -> Range<usize> {
    let cursor = previous_char_boundary(text, cursor);
    if rendered.is_empty() {
        return cursor..cursor;
    }

    if let Some(tail) = text.get(cursor..)
        && let Some(offset) = tail.find(rendered)
    {
        let start = cursor + offset;
        return start..start + rendered.len();
    }

    if let Some(start) = text.find(rendered) {
        return start..start + rendered.len();
    }

    // `textwrap` may return an owned string when options inject indentation or
    // otherwise transform the display line. In that case there is no exact
    // borrowed slice to point at. Return a safe, char-boundary fallback range
    // over the same number of displayed characters instead of panicking in the
    // renderer path.
    let end = advance_by_chars(text, cursor, rendered.chars().count());
    cursor..end
}

fn content_end_with_trailing_spaces(text: &str, range: &Range<usize>) -> usize {
    let mut end = previous_char_boundary(text, range.end);
    while end < text.len() {
        let Some(ch) = text[end..].chars().next() else {
            break;
        };
        if ch != ' ' {
            break;
        }
        end += ch.len_utf8();
    }
    end
}

fn range_with_trailing_space_sentinel(text: &str, range: Range<usize>) -> Range<usize> {
    let start = previous_char_boundary(text, range.start);
    let end = content_end_with_trailing_spaces(text, &range);

    // TextArea cursor math historically expects a sentinel after each wrapped
    // row. Keep the sentinel for compatibility, but make it land on a valid
    // char boundary for Unicode input instead of a raw byte offset.
    let sentinel_end = if end < text.len() {
        advance_by_chars(text, end, 1)
    } else {
        text.len().saturating_add(1)
    };
    start..sentinel_end.min(text.len().saturating_add(1))
}

pub(crate) fn wrap_ranges<'a, O>(text: &str, width_or_options: O) -> Vec<Range<usize>>
where
    O: Into<Options<'a>>,
{
    let opts = width_or_options.into();
    let mut cursor = 0usize;
    let mut lines: Vec<Range<usize>> = Vec::new();
    for line in textwrap::wrap(text, opts).iter() {
        let range = locate_wrapped_range(text, line.as_ref(), cursor);
        cursor = content_end_with_trailing_spaces(text, &range);
        let range = range_with_trailing_space_sentinel(text, range);
        lines.push(range);
    }
    lines
}

/// Like `wrap_ranges` but returns ranges without trailing whitespace and
/// without the sentinel extra byte. Suitable for general wrapping where
/// trailing spaces should not be preserved.
pub(crate) fn wrap_ranges_trim<'a, O>(text: &str, width_or_options: O) -> Vec<Range<usize>>
where
    O: Into<Options<'a>>,
{
    let opts = width_or_options.into();
    let mut cursor = 0usize;
    let mut lines: Vec<Range<usize>> = Vec::new();
    for line in textwrap::wrap(text, opts).iter() {
        let range = locate_wrapped_range(text, line.as_ref(), cursor);
        cursor = previous_char_boundary(text, range.end.min(text.len()));
        lines.push(range);
    }
    lines
}

#[derive(Debug, Clone)]
pub struct RtOptions<'a> {
    /// The width in columns at which the text will be wrapped.
    pub width: usize,
    /// Line ending used for breaking lines.
    pub line_ending: textwrap::LineEnding,
    /// Indentation used for the first line of output. See the
    /// [`Options::initial_indent`] method.
    pub initial_indent: Line<'a>,
    /// Indentation used for subsequent lines of output. See the
    /// [`Options::subsequent_indent`] method.
    pub subsequent_indent: Line<'a>,
    /// Allow long words to be broken if they cannot fit on a line.
    /// When set to `false`, some lines may be longer than
    /// `self.width`. See the [`Options::break_words`] method.
    pub break_words: bool,
    /// Wrapping algorithm to use, see the implementations of the
    /// [`WrapAlgorithm`] trait for details.
    pub wrap_algorithm: textwrap::WrapAlgorithm,
    /// The line breaking algorithm to use, see the [`WordSeparator`]
    /// trait for an overview and possible implementations.
    pub word_separator: textwrap::WordSeparator,
    /// The method for splitting words. This can be used to prohibit
    /// splitting words on hyphens, or it can be used to implement
    /// language-aware machine hyphenation.
    pub word_splitter: textwrap::WordSplitter,
}
impl From<usize> for RtOptions<'_> {
    fn from(width: usize) -> Self {
        RtOptions::new(width)
    }
}

#[allow(dead_code)]
impl<'a> RtOptions<'a> {
    pub fn new(width: usize) -> Self {
        RtOptions {
            width,
            line_ending: textwrap::LineEnding::LF,
            initial_indent: Line::default(),
            subsequent_indent: Line::default(),
            break_words: true,
            word_separator: textwrap::WordSeparator::new(),
            wrap_algorithm: textwrap::WrapAlgorithm::new(),
            word_splitter: textwrap::WordSplitter::HyphenSplitter,
        }
    }

    pub fn line_ending(self, line_ending: textwrap::LineEnding) -> Self {
        RtOptions {
            line_ending,
            ..self
        }
    }

    pub fn width(self, width: usize) -> Self {
        RtOptions { width, ..self }
    }

    pub fn initial_indent(self, initial_indent: Line<'a>) -> Self {
        RtOptions {
            initial_indent,
            ..self
        }
    }

    pub fn subsequent_indent(self, subsequent_indent: Line<'a>) -> Self {
        RtOptions {
            subsequent_indent,
            ..self
        }
    }

    pub fn break_words(self, break_words: bool) -> Self {
        RtOptions {
            break_words,
            ..self
        }
    }

    pub fn word_separator(self, word_separator: textwrap::WordSeparator) -> RtOptions<'a> {
        RtOptions {
            word_separator,
            ..self
        }
    }

    pub fn wrap_algorithm(self, wrap_algorithm: textwrap::WrapAlgorithm) -> RtOptions<'a> {
        RtOptions {
            wrap_algorithm,
            ..self
        }
    }

    pub fn word_splitter(self, word_splitter: textwrap::WordSplitter) -> RtOptions<'a> {
        RtOptions {
            word_splitter,
            ..self
        }
    }
}

pub(crate) fn word_wrap_line<'a, O>(line: &'a Line<'a>, width_or_options: O) -> Vec<Line<'a>>
where
    O: Into<RtOptions<'a>>,
{
    // Flatten the line and record span byte ranges.
    let mut flat = String::new();
    let mut span_bounds = Vec::new();
    let mut acc = 0usize;
    for s in &line.spans {
        let text = s.content.as_ref();
        let start = acc;
        flat.push_str(text);
        acc += text.len();
        span_bounds.push((start..acc, s.style));
    }

    let rt_opts: RtOptions<'a> = width_or_options.into();
    let opts = Options::new(rt_opts.width)
        .line_ending(rt_opts.line_ending)
        .break_words(rt_opts.break_words)
        .wrap_algorithm(rt_opts.wrap_algorithm)
        .word_separator(rt_opts.word_separator)
        .word_splitter(rt_opts.word_splitter);

    let mut out: Vec<Line<'a>> = Vec::new();

    // Compute first line range with reduced width due to initial indent.
    let initial_width_available = opts
        .width
        .saturating_sub(rt_opts.initial_indent.width())
        .max(1);
    let initial_wrapped = wrap_ranges_trim(&flat, opts.clone().width(initial_width_available));
    let Some(first_line_range) = initial_wrapped.first() else {
        return vec![rt_opts.initial_indent.clone()];
    };

    // Build first wrapped line with initial indent.
    let mut first_line = rt_opts.initial_indent.clone();
    {
        let mut sliced = slice_line_spans(line, &span_bounds, first_line_range);
        let mut spans = first_line.spans;
        spans.append(&mut sliced.spans);
        first_line.spans = spans;
        out.push(first_line);
    }

    // Wrap the remainder using subsequent indent width and map back to original indices.
    let base = first_line_range.end;
    let skip_leading_spaces = flat[base..].chars().take_while(|c| *c == ' ').count();
    let base = base + skip_leading_spaces;
    let subsequent_width_available = opts
        .width
        .saturating_sub(rt_opts.subsequent_indent.width())
        .max(1);
    let remaining_wrapped = wrap_ranges_trim(&flat[base..], opts.width(subsequent_width_available));
    for r in &remaining_wrapped {
        if r.is_empty() {
            continue;
        }
        let mut subsequent_line = rt_opts.subsequent_indent.clone();
        let offset_range = (r.start + base)..(r.end + base);
        let mut sliced = slice_line_spans(line, &span_bounds, &offset_range);
        let mut spans = subsequent_line.spans;
        spans.append(&mut sliced.spans);
        subsequent_line.spans = spans;
        out.push(subsequent_line);
    }

    out
}

/// Wrap a sequence of lines, applying the initial indent only to the very first
/// output line, and using the subsequent indent for all later wrapped pieces.
#[allow(dead_code)]
pub(crate) fn word_wrap_lines<'a, I, O>(lines: I, width_or_options: O) -> Vec<Line<'static>>
where
    I: IntoIterator<Item = &'a Line<'a>>,
    O: Into<RtOptions<'a>>,
{
    let base_opts: RtOptions<'a> = width_or_options.into();
    let mut out: Vec<Line<'static>> = Vec::new();

    for (idx, line) in lines.into_iter().enumerate() {
        let opts = if idx == 0 {
            base_opts.clone()
        } else {
            let mut o = base_opts.clone();
            let sub = o.subsequent_indent.clone();
            o = o.initial_indent(sub);
            o
        };
        let wrapped = word_wrap_line(line, opts);
        push_owned_lines(&wrapped, &mut out);
    }

    out
}

#[allow(dead_code)]
pub(crate) fn word_wrap_lines_borrowed<'a, I, O>(lines: I, width_or_options: O) -> Vec<Line<'a>>
where
    I: IntoIterator<Item = &'a Line<'a>>,
    O: Into<RtOptions<'a>>,
{
    let base_opts: RtOptions<'a> = width_or_options.into();
    let mut out: Vec<Line<'a>> = Vec::new();
    let mut first = true;
    for line in lines.into_iter() {
        let opts = if first {
            base_opts.clone()
        } else {
            base_opts
                .clone()
                .initial_indent(base_opts.subsequent_indent.clone())
        };
        out.extend(word_wrap_line(line, opts));
        first = false;
    }
    out
}

fn slice_line_spans<'a>(
    original: &'a Line<'a>,
    span_bounds: &[(Range<usize>, ratatui::style::Style)],
    range: &Range<usize>,
) -> Line<'a> {
    let start_byte = range.start;
    let end_byte = range.end;
    let mut acc: Vec<Span<'a>> = Vec::new();
    for (i, (range, style)) in span_bounds.iter().enumerate() {
        let s = range.start;
        let e = range.end;
        if e <= start_byte {
            continue;
        }
        if s >= end_byte {
            break;
        }
        let seg_start = start_byte.max(s);
        let seg_end = end_byte.min(e);
        if seg_end > seg_start {
            let local_start = seg_start - s;
            let local_end = seg_end - s;
            let content = original.spans[i].content.as_ref();
            let slice = &content[local_start..local_end];
            acc.push(Span {
                style: *style,
                content: std::borrow::Cow::Borrowed(slice),
            });
        }
        if e >= end_byte {
            break;
        }
    }
    Line {
        style: original.style,
        alignment: original.alignment,
        spans: acc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_ranges_trim_handles_owned_textwrap_lines() {
        let text = "abcdef ghij";
        let ranges = wrap_ranges_trim(
            text,
            Options::new(5).initial_indent("> ").subsequent_indent("> "),
        );

        assert!(!ranges.is_empty());
        assert!(ranges.iter().all(|range| range.start <= range.end));
        assert!(ranges.iter().all(|range| range.end <= text.len()));
    }

    #[test]
    fn wrap_ranges_handles_unicode_without_mid_scalar_bounds() {
        let text = "αβ γδ εζ";
        let ranges = wrap_ranges(text, Options::new(4));

        assert!(!ranges.is_empty());
        for range in ranges {
            assert!(range.start <= range.end);
            assert!(range.end <= text.len() + 1);
            assert!(text.is_char_boundary(range.start));
            assert!(range.end == text.len() + 1 || text.is_char_boundary(range.end));
        }
    }
}
