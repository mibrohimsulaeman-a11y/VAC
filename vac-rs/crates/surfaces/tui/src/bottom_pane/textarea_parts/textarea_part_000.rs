// The textarea owns editable composer text, placeholder elements, cursor/wrap state, and a
// single-entry kill buffer.
//
// Whole-buffer replacement APIs intentionally rebuild only the visible draft state. They clear
// element ranges and derived cursor/wrapping caches, but they keep the kill buffer intact so a
// caller can clear or rewrite the draft and still allow `Ctrl+Y` to restore the user's most
// recent `Ctrl+K`. This is the contract higher-level composer flows rely on after submit,
// slash-command dispatch, and other synthetic clears.
//
// This module does not implement an Emacs-style multi-entry kill ring. It keeps only the most
// recent killed span.

use crate::key_hint::KeyBindingListExt;
use crate::key_hint::is_altgr;
use crate::keymap::EditorKeymap;
use crate::keymap::RuntimeKeymap;
use crate::keymap::VimNormalKeymap;
use crate::keymap::VimOperatorKeymap;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::StatefulWidgetRef;
use ratatui::widgets::WidgetRef;
use std::cell::RefCell;
use std::ops::Range;
use textwrap::Options;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use vac_protocol::user_input::ByteRange;
use vac_protocol::user_input::TextElement as UserTextElement;

const WORD_SEPARATORS: &str = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";

fn is_word_separator(ch: char) -> bool {
    WORD_SEPARATORS.contains(ch)
}

fn split_word_pieces(run: &str) -> Vec<(usize, &str)> {
    let mut pieces = Vec::new();
    for (segment_start, segment) in run.split_word_bound_indices() {
        let mut piece_start = 0;
        let mut chars = segment.char_indices();
        let Some((_, first_char)) = chars.next() else {
            continue;
        };
        let mut in_separator = is_word_separator(first_char);

        for (idx, ch) in chars {
            let is_separator = is_word_separator(ch);
            if is_separator == in_separator {
                continue;
            }
            pieces.push((segment_start + piece_start, &segment[piece_start..idx]));
            piece_start = idx;
            in_separator = is_separator;
        }

        pieces.push((segment_start + piece_start, &segment[piece_start..]));
    }

    pieces
}

#[derive(Debug, Clone)]
struct TextElement {
    id: u64,
    range: Range<usize>,
    name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextElementSnapshot {
    pub(crate) id: u64,
    pub(crate) range: Range<usize>,
    pub(crate) text: String,
}

/// `TextArea` is the editable buffer behind the TUI composer.
///
/// It owns the raw UTF-8 text, placeholder-like text elements that must move atomically with
/// edits, cursor/wrapping state for rendering, and a single-entry kill buffer for `Ctrl+K` /
/// `Ctrl+Y` style editing. Callers may replace the entire visible buffer through
/// [`Self::set_text_clearing_elements`] or [`Self::set_text_with_elements`] without disturbing the
/// kill buffer; if they incorrectly assume those methods fully reset editing state, a later yank
/// will appear to restore stale text from the user's perspective.
#[derive(Debug)]
pub(crate) struct TextArea {
    text: String,
    cursor_pos: usize,
    wrap_cache: RefCell<Option<WrapCache>>,
    preferred_col: Option<usize>,
    elements: Vec<TextElement>,
    next_element_id: u64,
    kill_buffer: String,
    kill_buffer_kind: KillBufferKind,
    vim_enabled: bool,
    vim_mode: VimMode,
    vim_operator: Option<VimOperator>,
    editor_keymap: EditorKeymap,
    vim_normal_keymap: VimNormalKeymap,
    vim_operator_keymap: VimOperatorKeymap,
}

#[derive(Debug, Clone)]
struct WrapCache {
    width: u16,
    lines: Vec<Range<usize>>,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TextAreaState {
    /// Index into wrapped lines of the first visible line.
    scroll: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VimMode {
    /// Normal mode routes printable keys to movement, operators, and mode transitions.
    Normal,
    /// Insert mode routes input through the regular editor keymap until Escape is pressed.
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillBufferKind {
    /// Characterwise kills and yanks paste at the cursor.
    Characterwise,
    /// Linewise kills and yanks paste as whole lines below the cursor line.
    Linewise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VimOperator {
    /// Delete the range selected by the next motion or repeated operator key.
    Delete,
    /// Copy the range selected by the next motion or repeated operator key.
    Yank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VimMotion {
    /// Move one atomic boundary to the left.
    Left,
    /// Move one atomic boundary to the right.
    Right,
    /// Move one visual row up, preserving preferred display column.
    Up,
    /// Move one visual row down, preserving preferred display column.
    Down,
    /// Move to the start of the next word-like run.
    WordForward,
    /// Move to the start of the previous word-like run.
    WordBackward,
    /// Move to the end of the current or next word-like run.
    WordEnd,
    /// Move to the start of the current line.
    LineStart,
    /// Move to the end of the current line.
    LineEnd,
}

