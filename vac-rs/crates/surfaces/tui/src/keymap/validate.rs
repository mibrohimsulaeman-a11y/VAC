use super::*;
use crate::key_hint;
use crate::key_hint::KeyBinding;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use std::collections::HashMap;

impl RuntimeKeymap {
    /// Reject ambiguous bindings in scopes that are evaluated together.
    ///
    /// We validate in multiple passes because runtime handling has mixed
    /// precedence:
    ///
    /// 1. `app` actions can shadow composer actions because app checks run
    ///    before forwarding to the composer.
    /// 2. Contexts with hard-coded sequence behavior, such as edit-previous
    ///    backtracking, intentionally stay outside this configurable keymap.
    pub(super) fn validate_conflicts(&self) -> Result<(), String> {
        validate_unique(
            "app",
            [
                ("open_transcript", self.app.open_transcript.as_slice()),
                (
                    "open_external_editor",
                    self.app.open_external_editor.as_slice(),
                ),
                ("copy", self.app.copy.as_slice()),
                ("clear_terminal", self.app.clear_terminal.as_slice()),
                ("toggle_vim_mode", self.app.toggle_vim_mode.as_slice()),
                (
                    "chat.decrease_reasoning_effort",
                    self.chat.decrease_reasoning_effort.as_slice(),
                ),
                (
                    "chat.increase_reasoning_effort",
                    self.chat.increase_reasoning_effort.as_slice(),
                ),
                (
                    "chat.edit_queued_message",
                    self.chat.edit_queued_message.as_slice(),
                ),
                ("composer.submit", self.composer.submit.as_slice()),
                ("composer.queue", self.composer.queue.as_slice()),
                (
                    "composer.toggle_shortcuts",
                    self.composer.toggle_shortcuts.as_slice(),
                ),
                (
                    "composer.history_search_previous",
                    self.composer.history_search_previous.as_slice(),
                ),
                (
                    "composer.history_search_next",
                    self.composer.history_search_next.as_slice(),
                ),
            ],
        )?;

        validate_no_reserved(
            "main",
            [
                ("open_transcript", self.app.open_transcript.as_slice()),
                (
                    "open_external_editor",
                    self.app.open_external_editor.as_slice(),
                ),
                ("copy", self.app.copy.as_slice()),
                ("clear_terminal", self.app.clear_terminal.as_slice()),
                ("toggle_vim_mode", self.app.toggle_vim_mode.as_slice()),
                (
                    "chat.decrease_reasoning_effort",
                    self.chat.decrease_reasoning_effort.as_slice(),
                ),
                (
                    "chat.increase_reasoning_effort",
                    self.chat.increase_reasoning_effort.as_slice(),
                ),
                (
                    "chat.edit_queued_message",
                    self.chat.edit_queued_message.as_slice(),
                ),
                ("composer.submit", self.composer.submit.as_slice()),
                ("composer.queue", self.composer.queue.as_slice()),
                (
                    "composer.toggle_shortcuts",
                    self.composer.toggle_shortcuts.as_slice(),
                ),
                (
                    "composer.history_search_previous",
                    self.composer.history_search_previous.as_slice(),
                ),
                (
                    "composer.history_search_next",
                    self.composer.history_search_next.as_slice(),
                ),
            ],
            MAIN_RESERVED_BINDINGS,
        )?;

        validate_no_shadow(
            "app",
            [
                ("open_transcript", self.app.open_transcript.as_slice()),
                (
                    "open_external_editor",
                    self.app.open_external_editor.as_slice(),
                ),
                ("copy", self.app.copy.as_slice()),
                ("clear_terminal", self.app.clear_terminal.as_slice()),
                ("toggle_vim_mode", self.app.toggle_vim_mode.as_slice()),
            ],
            [
                ("list.move_up", self.list.move_up.as_slice()),
                ("list.move_down", self.list.move_down.as_slice()),
                ("list.accept", self.list.accept.as_slice()),
                ("list.cancel", self.list.cancel.as_slice()),
                (
                    "approval.open_fullscreen",
                    self.approval.open_fullscreen.as_slice(),
                ),
                ("approval.open_thread", self.approval.open_thread.as_slice()),
                ("approval.approve", self.approval.approve.as_slice()),
                (
                    "approval.approve_for_session",
                    self.approval.approve_for_session.as_slice(),
                ),
                (
                    "approval.approve_for_prefix",
                    self.approval.approve_for_prefix.as_slice(),
                ),
                ("approval.deny", self.approval.deny.as_slice()),
                ("approval.decline", self.approval.decline.as_slice()),
                ("approval.cancel", self.approval.cancel.as_slice()),
            ],
        )?;

        // While the composer is focused, these main-surface handlers always
        // consume matching keys before the event reaches the textarea editor.
        validate_no_shadow_with_allowed_overlaps(
            "main",
            [
                ("open_transcript", self.app.open_transcript.as_slice()),
                (
                    "open_external_editor",
                    self.app.open_external_editor.as_slice(),
                ),
                ("copy", self.app.copy.as_slice()),
                ("clear_terminal", self.app.clear_terminal.as_slice()),
                (
                    "chat.decrease_reasoning_effort",
                    self.chat.decrease_reasoning_effort.as_slice(),
                ),
                (
                    "chat.increase_reasoning_effort",
                    self.chat.increase_reasoning_effort.as_slice(),
                ),
                ("composer.submit", self.composer.submit.as_slice()),
                ("toggle_vim_mode", self.app.toggle_vim_mode.as_slice()),
                (
                    "composer.history_search_previous",
                    self.composer.history_search_previous.as_slice(),
                ),
            ],
            [
                (
                    "editor.insert_newline",
                    self.editor.insert_newline.as_slice(),
                ),
                ("editor.move_left", self.editor.move_left.as_slice()),
                ("editor.move_right", self.editor.move_right.as_slice()),
                ("editor.move_up", self.editor.move_up.as_slice()),
                ("editor.move_down", self.editor.move_down.as_slice()),
                (
                    "editor.move_word_left",
                    self.editor.move_word_left.as_slice(),
                ),
                (
                    "editor.move_word_right",
                    self.editor.move_word_right.as_slice(),
                ),
                (
                    "editor.move_line_start",
                    self.editor.move_line_start.as_slice(),
                ),
                ("editor.move_line_end", self.editor.move_line_end.as_slice()),
                (
                    "editor.delete_backward",
                    self.editor.delete_backward.as_slice(),
                ),
                (
                    "editor.delete_forward",
                    self.editor.delete_forward.as_slice(),
                ),
                (
                    "editor.delete_backward_word",
                    self.editor.delete_backward_word.as_slice(),
                ),
                (
                    "editor.delete_forward_word",
                    self.editor.delete_forward_word.as_slice(),
                ),
                (
                    "editor.kill_line_start",
                    self.editor.kill_line_start.as_slice(),
                ),
                ("editor.kill_line_end", self.editor.kill_line_end.as_slice()),
                ("editor.yank", self.editor.yank.as_slice()),
            ],
            [(
                "composer.submit",
                "editor.insert_newline",
                key_hint::plain(KeyCode::Enter),
            )],
        )?;

        validate_unique(
            "editor",
            [
                ("insert_newline", self.editor.insert_newline.as_slice()),
                ("move_left", self.editor.move_left.as_slice()),
                ("move_right", self.editor.move_right.as_slice()),
                ("move_up", self.editor.move_up.as_slice()),
                ("move_down", self.editor.move_down.as_slice()),
                ("move_word_left", self.editor.move_word_left.as_slice()),
                ("move_word_right", self.editor.move_word_right.as_slice()),
                ("move_line_start", self.editor.move_line_start.as_slice()),
                ("move_line_end", self.editor.move_line_end.as_slice()),
                ("delete_backward", self.editor.delete_backward.as_slice()),
                ("delete_forward", self.editor.delete_forward.as_slice()),
                (
                    "delete_backward_word",
                    self.editor.delete_backward_word.as_slice(),
                ),
                (
                    "delete_forward_word",
                    self.editor.delete_forward_word.as_slice(),
                ),
                ("kill_line_start", self.editor.kill_line_start.as_slice()),
                ("kill_line_end", self.editor.kill_line_end.as_slice()),
                ("yank", self.editor.yank.as_slice()),
            ],
        )?;

        validate_unique(
            "vim_normal",
            [
                ("enter_insert", self.vim_normal.enter_insert.as_slice()),
                (
                    "append_after_cursor",
                    self.vim_normal.append_after_cursor.as_slice(),
                ),
                (
                    "append_line_end",
                    self.vim_normal.append_line_end.as_slice(),
                ),
                (
                    "insert_line_start",
                    self.vim_normal.insert_line_start.as_slice(),
                ),
                (
                    "open_line_below",
                    self.vim_normal.open_line_below.as_slice(),
                ),
                (
                    "open_line_above",
                    self.vim_normal.open_line_above.as_slice(),
                ),
                ("move_left", self.vim_normal.move_left.as_slice()),
                ("move_right", self.vim_normal.move_right.as_slice()),
                ("move_up", self.vim_normal.move_up.as_slice()),
                ("move_down", self.vim_normal.move_down.as_slice()),
                (
                    "move_word_forward",
                    self.vim_normal.move_word_forward.as_slice(),
                ),
                (
                    "move_word_backward",
                    self.vim_normal.move_word_backward.as_slice(),
                ),
                ("move_word_end", self.vim_normal.move_word_end.as_slice()),
                (
                    "move_line_start",
                    self.vim_normal.move_line_start.as_slice(),
                ),
                ("move_line_end", self.vim_normal.move_line_end.as_slice()),
                ("delete_char", self.vim_normal.delete_char.as_slice()),
                (
                    "delete_to_line_end",
                    self.vim_normal.delete_to_line_end.as_slice(),
                ),
                ("yank_line", self.vim_normal.yank_line.as_slice()),
                ("paste_after", self.vim_normal.paste_after.as_slice()),
                (
                    "start_delete_operator",
                    self.vim_normal.start_delete_operator.as_slice(),
                ),
                (
                    "start_yank_operator",
                    self.vim_normal.start_yank_operator.as_slice(),
                ),
                (
                    "cancel_operator",
                    self.vim_normal.cancel_operator.as_slice(),
                ),
            ],
        )?;

        validate_unique(
            "vim_operator",
            [
                ("delete_line", self.vim_operator.delete_line.as_slice()),
                ("yank_line", self.vim_operator.yank_line.as_slice()),
                ("motion_left", self.vim_operator.motion_left.as_slice()),
                ("motion_right", self.vim_operator.motion_right.as_slice()),
                ("motion_up", self.vim_operator.motion_up.as_slice()),
                ("motion_down", self.vim_operator.motion_down.as_slice()),
                (
                    "motion_word_forward",
                    self.vim_operator.motion_word_forward.as_slice(),
                ),
                (
                    "motion_word_backward",
                    self.vim_operator.motion_word_backward.as_slice(),
                ),
                (
                    "motion_word_end",
                    self.vim_operator.motion_word_end.as_slice(),
                ),
                (
                    "motion_line_start",
                    self.vim_operator.motion_line_start.as_slice(),
                ),
                (
                    "motion_line_end",
                    self.vim_operator.motion_line_end.as_slice(),
                ),
                ("cancel", self.vim_operator.cancel.as_slice()),
            ],
        )?;

        validate_unique(
            "pager",
            [
                ("scroll_up", self.pager.scroll_up.as_slice()),
                ("scroll_down", self.pager.scroll_down.as_slice()),
                ("page_up", self.pager.page_up.as_slice()),
                ("page_down", self.pager.page_down.as_slice()),
                ("half_page_up", self.pager.half_page_up.as_slice()),
                ("half_page_down", self.pager.half_page_down.as_slice()),
                ("jump_top", self.pager.jump_top.as_slice()),
                ("jump_bottom", self.pager.jump_bottom.as_slice()),
                ("close", self.pager.close.as_slice()),
                ("close_transcript", self.pager.close_transcript.as_slice()),
            ],
        )?;

        validate_no_reserved(
            "pager",
            [
                ("scroll_up", self.pager.scroll_up.as_slice()),
                ("scroll_down", self.pager.scroll_down.as_slice()),
                ("page_up", self.pager.page_up.as_slice()),
                ("page_down", self.pager.page_down.as_slice()),
                ("half_page_up", self.pager.half_page_up.as_slice()),
                ("half_page_down", self.pager.half_page_down.as_slice()),
                ("jump_top", self.pager.jump_top.as_slice()),
                ("jump_bottom", self.pager.jump_bottom.as_slice()),
                ("close", self.pager.close.as_slice()),
                ("close_transcript", self.pager.close_transcript.as_slice()),
            ],
            TRANSCRIPT_BACKTRACK_RESERVED_BINDINGS,
        )?;

        validate_unique(
            "list",
            [
                ("move_up", self.list.move_up.as_slice()),
                ("move_down", self.list.move_down.as_slice()),
                ("accept", self.list.accept.as_slice()),
                ("cancel", self.list.cancel.as_slice()),
            ],
        )?;

        validate_unique(
            "approval",
            [
                ("open_fullscreen", self.approval.open_fullscreen.as_slice()),
                ("open_thread", self.approval.open_thread.as_slice()),
                ("approve", self.approval.approve.as_slice()),
                (
                    "approve_for_session",
                    self.approval.approve_for_session.as_slice(),
                ),
                (
                    "approve_for_prefix",
                    self.approval.approve_for_prefix.as_slice(),
                ),
                ("deny", self.approval.deny.as_slice()),
                ("decline", self.approval.decline.as_slice()),
                ("cancel", self.approval.cancel.as_slice()),
            ],
        )?;

        let mut seen: HashMap<(KeyCode, KeyModifiers), &'static str> = HashMap::new();
        for (action, bindings) in [
            ("list.move_up", self.list.move_up.as_slice()),
            ("list.move_down", self.list.move_down.as_slice()),
            ("list.accept", self.list.accept.as_slice()),
            ("list.cancel", self.list.cancel.as_slice()),
            (
                "approval.open_fullscreen",
                self.approval.open_fullscreen.as_slice(),
            ),
            ("approval.open_thread", self.approval.open_thread.as_slice()),
            ("approval.approve", self.approval.approve.as_slice()),
            (
                "approval.approve_for_session",
                self.approval.approve_for_session.as_slice(),
            ),
            (
                "approval.approve_for_prefix",
                self.approval.approve_for_prefix.as_slice(),
            ),
            ("approval.deny", self.approval.deny.as_slice()),
            ("approval.decline", self.approval.decline.as_slice()),
            ("approval.cancel", self.approval.cancel.as_slice()),
        ] {
            for binding in bindings {
                let key = binding.parts();
                if let Some(previous) = seen.insert(key, action) {
                    // Approval overlays intentionally reserve Esc as a stable
                    // cancellation path even though decline options may also
                    // display it in contexts where that is safe.
                    if previous == "list.cancel"
                        && action == "approval.decline"
                        && key == (KeyCode::Esc, KeyModifiers::NONE)
                    {
                        continue;
                    }
                    return Err(format!(
                        "Ambiguous approval overlay keymap bindings: `{previous}` and `{action}` use the same key. \
Set unique keys in `~/.vac/config.toml` and retry. \
See the VAC keymap documentation for supported actions and examples."
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Reject duplicate keys inside one effective context map.
///
/// This intentionally allows the same key across different contexts; handlers
/// only evaluate one context at a time.
fn validate_unique<const N: usize>(
    context: &str,
    pairs: [(&'static str, &[KeyBinding]); N],
) -> Result<(), String> {
    let mut seen: HashMap<(KeyCode, KeyModifiers), &'static str> = HashMap::new();
    for (action, bindings) in pairs {
        for binding in bindings {
            let key = binding.parts();
            if let Some(previous) = seen.insert(key, action) {
                return Err(format!(
                    "Ambiguous `tui.keymap.{context}` bindings: `{previous}` and `{action}` use the same key. \
Set unique keys in `~/.vac/config.toml` and retry. \
See the VAC keymap documentation for supported actions and examples."
                ));
            }
        }
    }
    Ok(())
}

fn validate_no_shadow<const N: usize, const M: usize>(
    context: &str,
    primary: [(&'static str, &[KeyBinding]); N],
    shadowed: [(&'static str, &[KeyBinding]); M],
) -> Result<(), String> {
    validate_no_shadow_with_allowed_overlaps(context, primary, shadowed, [])
}

fn validate_no_shadow_with_allowed_overlaps<const N: usize, const M: usize, const A: usize>(
    context: &str,
    primary: [(&'static str, &[KeyBinding]); N],
    shadowed: [(&'static str, &[KeyBinding]); M],
    allowed_overlaps: [(&'static str, &'static str, KeyBinding); A],
) -> Result<(), String> {
    let mut seen: HashMap<(KeyCode, KeyModifiers), &'static str> = HashMap::new();
    for (action, bindings) in primary {
        for binding in bindings {
            seen.insert(binding.parts(), action);
        }
    }
    for (action, bindings) in shadowed {
        for binding in bindings {
            let key = binding.parts();
            if let Some(previous) = seen.get(&key) {
                if allowed_overlaps.iter().any(
                    |(allowed_primary, allowed_shadowed, allowed_binding)| {
                        *allowed_primary == *previous
                            && *allowed_shadowed == action
                            && allowed_binding.parts() == key
                    },
                ) {
                    continue;
                }
                return Err(format!(
                    "Ambiguous `tui.keymap.{context}` bindings: `{previous}` shadows `{action}` with the same key. \
Set unique keys in `~/.vac/config.toml` and retry. \
See the VAC keymap documentation for supported actions and examples."
                ));
            }
        }
    }
    Ok(())
}

fn validate_no_reserved<const N: usize>(
    context: &str,
    pairs: [(&'static str, &[KeyBinding]); N],
    reserved: &[(&'static str, KeyBinding)],
) -> Result<(), String> {
    for (action, bindings) in pairs {
        for binding in bindings {
            let key = binding.parts();
            if let Some((reserved_action, _)) = reserved
                .iter()
                .find(|(_, reserved_binding)| reserved_binding.parts() == key)
            {
                return Err(format!(
                    "Ambiguous `tui.keymap.{context}` bindings: `{action}` uses a key reserved by `{reserved_action}`. \
Set a different key in `~/.vac/config.toml` and retry. \
See the VAC keymap documentation for supported actions and examples."
                ));
            }
        }
    }
    Ok(())
}

const MAIN_RESERVED_BINDINGS: &[(&str, KeyBinding)] = &[
    (
        "fixed.interrupt_or_quit",
        key_hint::ctrl(KeyCode::Char('c')),
    ),
    ("fixed.quit", key_hint::ctrl(KeyCode::Char('d'))),
    ("fixed.paste_image", key_hint::ctrl(KeyCode::Char('v'))),
    ("fixed.paste_image", key_hint::ctrl_alt(KeyCode::Char('v'))),
    (
        "fixed.cycle_collaboration_mode",
        key_hint::shift(KeyCode::Tab),
    ),
    (
        "fixed.return_from_side_or_backtrack",
        key_hint::plain(KeyCode::Esc),
    ),
    ("fixed.previous_agent", key_hint::alt(KeyCode::Left)),
    ("fixed.next_agent", key_hint::alt(KeyCode::Right)),
    ("fixed.slash_command", key_hint::plain(KeyCode::Char('/'))),
    ("fixed.shell_command", key_hint::plain(KeyCode::Char('!'))),
    ("fixed.file_paths", key_hint::plain(KeyCode::Char('@'))),
    (
        "fixed.connector_mentions",
        key_hint::plain(KeyCode::Char('$')),
    ),
];

const TRANSCRIPT_BACKTRACK_RESERVED_BINDINGS: &[(&str, KeyBinding)] = &[
    (
        "fixed.transcript_edit_previous",
        key_hint::plain(KeyCode::Esc),
    ),
    (
        "fixed.transcript_edit_previous",
        key_hint::plain(KeyCode::Left),
    ),
    (
        "fixed.transcript_edit_next",
        key_hint::plain(KeyCode::Right),
    ),
    (
        "fixed.transcript_confirm_edit",
        key_hint::plain(KeyCode::Enter),
    ),
];
