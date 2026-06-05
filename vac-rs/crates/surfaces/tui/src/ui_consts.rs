// Shared UI constants for layout and alignment within the TUI.

/// Width (in terminal columns) reserved for the left gutter/prefix used by
/// live cells and aligned widgets.
///
/// Semantics:
/// - Chat composer reserves this many columns for the left border + padding.
/// - Status indicator lines begin with this many spaces for alignment.
/// - User history lines account for this many columns (e.g., "▌ ") when wrapping.
pub(crate) const LIVE_PREFIX_COLS: u16 = 2;
pub(crate) const FOOTER_INDENT_COLS: usize = LIVE_PREFIX_COLS as usize;

pub(crate) const APP_BG_RGB: (u8, u8, u8) = (7, 13, 20);
pub(crate) const APP_BG: ratatui::style::Color =
    ratatui::style::Color::Rgb(APP_BG_RGB.0, APP_BG_RGB.1, APP_BG_RGB.2);
