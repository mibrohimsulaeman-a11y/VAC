use crate::key_hint::KeyBinding;

/// Runtime keymap used by TUI input handlers.
///
/// Resolution precedence is:
///
/// 1. Context-specific binding (`tui.keymap.<context>`).
/// 2. `tui.keymap.global` for actions that support global fallback.
/// 3. Built-in defaults.
///
/// This is the only shape UI code should use for dispatch. It represents a
/// fully resolved snapshot with parsing, fallback, explicit unbinding, and
/// duplicate-key validation already applied. If a caller keeps using an older
/// snapshot after config changes, visible hints and active handlers can drift.
#[derive(Clone, Debug)]
pub(crate) struct RuntimeKeymap {
    pub(crate) app: AppKeymap,
    pub(crate) chat: ChatKeymap,
    pub(crate) composer: ComposerKeymap,
    pub(crate) editor: EditorKeymap,
    pub(crate) vim_normal: VimNormalKeymap,
    pub(crate) vim_operator: VimOperatorKeymap,
    pub(crate) pager: PagerKeymap,
    pub(crate) list: ListKeymap,
    pub(crate) approval: ApprovalKeymap,
}

#[derive(Clone, Debug)]
pub(crate) struct AppKeymap {
    /// Open transcript overlay.
    pub(crate) open_transcript: Vec<KeyBinding>,
    /// Open external editor for the current draft.
    pub(crate) open_external_editor: Vec<KeyBinding>,
    /// Copy the last agent response to the clipboard.
    pub(crate) copy: Vec<KeyBinding>,
    /// Clear the terminal UI.
    pub(crate) clear_terminal: Vec<KeyBinding>,
    /// Toggle Vim mode for the composer input.
    pub(crate) toggle_vim_mode: Vec<KeyBinding>,
}

/// Chat-level keybindings evaluated at the app event layer.
///
/// These participate in the first app-scope conflict validation pass alongside
/// `AppKeymap` actions because both are checked before input reaches the
/// composer. Dispatch gating (empty-composer guard for backtrack) happens in
/// handler code, not here.
#[derive(Clone, Debug)]
pub(crate) struct ChatKeymap {
    /// Decrease the active reasoning effort.
    pub(crate) decrease_reasoning_effort: Vec<KeyBinding>,
    /// Increase the active reasoning effort.
    pub(crate) increase_reasoning_effort: Vec<KeyBinding>,
    /// Edit the most recently queued message.
    pub(crate) edit_queued_message: Vec<KeyBinding>,
}

/// Composer-level keybindings validated in the second app-scope conflict pass.
///
/// App-level handlers execute before the composer receives input, so any key
/// bound here that also appears in `AppKeymap` would be silently intercepted.
/// The conflict validator prevents this by checking app + composer uniqueness.
#[derive(Clone, Debug)]
pub(crate) struct ComposerKeymap {
    /// Submit current draft.
    pub(crate) submit: Vec<KeyBinding>,
    /// Queue current draft while a task is running.
    pub(crate) queue: Vec<KeyBinding>,
    /// Toggle composer shortcut overlay.
    pub(crate) toggle_shortcuts: Vec<KeyBinding>,
    /// Open reverse history search or move to the previous match.
    pub(crate) history_search_previous: Vec<KeyBinding>,
    /// Move to the next match in reverse history search.
    pub(crate) history_search_next: Vec<KeyBinding>,
}

/// Editor-specific keybindings used by the composer textarea.
///
/// These bindings are interpreted only by text-editing widgets and do not
/// participate in global/chat fallback resolution.
#[derive(Clone, Debug)]
pub(crate) struct EditorKeymap {
    pub(crate) insert_newline: Vec<KeyBinding>,
    pub(crate) move_left: Vec<KeyBinding>,
    pub(crate) move_right: Vec<KeyBinding>,
    pub(crate) move_up: Vec<KeyBinding>,
    pub(crate) move_down: Vec<KeyBinding>,
    pub(crate) move_word_left: Vec<KeyBinding>,
    pub(crate) move_word_right: Vec<KeyBinding>,
    pub(crate) move_line_start: Vec<KeyBinding>,
    pub(crate) move_line_end: Vec<KeyBinding>,
    pub(crate) delete_backward: Vec<KeyBinding>,
    pub(crate) delete_forward: Vec<KeyBinding>,
    pub(crate) delete_backward_word: Vec<KeyBinding>,
    pub(crate) delete_forward_word: Vec<KeyBinding>,
    pub(crate) kill_line_start: Vec<KeyBinding>,
    pub(crate) kill_line_end: Vec<KeyBinding>,
    pub(crate) yank: Vec<KeyBinding>,
}

/// Vim normal-mode keybindings for modal editing in the composer textarea.
///
/// Normal mode is the resting state when Vim is enabled. Pressing a movement
/// or editing key here either moves the cursor, triggers an operator-pending
/// state (via `start_delete_operator` / `start_yank_operator`), or transitions
/// to insert mode. Default bindings include both `shift(letter)` and
/// `plain(UPPERCASE)` variants for uppercase commands like `A`, `I`, `O` to
/// handle cross-terminal shift-reporting inconsistencies.
#[derive(Clone, Debug, Default)]
pub(crate) struct VimNormalKeymap {
    pub(crate) enter_insert: Vec<KeyBinding>,
    pub(crate) append_after_cursor: Vec<KeyBinding>,
    pub(crate) append_line_end: Vec<KeyBinding>,
    pub(crate) insert_line_start: Vec<KeyBinding>,
    pub(crate) open_line_below: Vec<KeyBinding>,
    pub(crate) open_line_above: Vec<KeyBinding>,
    pub(crate) move_left: Vec<KeyBinding>,
    pub(crate) move_right: Vec<KeyBinding>,
    pub(crate) move_up: Vec<KeyBinding>,
    pub(crate) move_down: Vec<KeyBinding>,
    pub(crate) move_word_forward: Vec<KeyBinding>,
    pub(crate) move_word_backward: Vec<KeyBinding>,
    pub(crate) move_word_end: Vec<KeyBinding>,
    pub(crate) move_line_start: Vec<KeyBinding>,
    pub(crate) move_line_end: Vec<KeyBinding>,
    pub(crate) delete_char: Vec<KeyBinding>,
    pub(crate) delete_to_line_end: Vec<KeyBinding>,
    pub(crate) yank_line: Vec<KeyBinding>,
    pub(crate) paste_after: Vec<KeyBinding>,
    pub(crate) start_delete_operator: Vec<KeyBinding>,
    pub(crate) start_yank_operator: Vec<KeyBinding>,
    pub(crate) cancel_operator: Vec<KeyBinding>,
}

/// Vim operator-pending keybindings active after `d` or `y` in normal mode.
///
/// When an operator (`start_delete_operator` or `start_yank_operator`) is
/// pressed, the next keypress is matched against this context to determine the
/// motion range. Repeating the operator key (`dd`, `yy`) acts on the whole
/// line. `Esc` cancels the pending operator and returns to normal mode.
#[derive(Clone, Debug, Default)]
pub(crate) struct VimOperatorKeymap {
    pub(crate) delete_line: Vec<KeyBinding>,
    pub(crate) yank_line: Vec<KeyBinding>,
    pub(crate) motion_left: Vec<KeyBinding>,
    pub(crate) motion_right: Vec<KeyBinding>,
    pub(crate) motion_up: Vec<KeyBinding>,
    pub(crate) motion_down: Vec<KeyBinding>,
    pub(crate) motion_word_forward: Vec<KeyBinding>,
    pub(crate) motion_word_backward: Vec<KeyBinding>,
    pub(crate) motion_word_end: Vec<KeyBinding>,
    pub(crate) motion_line_start: Vec<KeyBinding>,
    pub(crate) motion_line_end: Vec<KeyBinding>,
    pub(crate) cancel: Vec<KeyBinding>,
}

/// Pager/overlay keybindings for transcript and static help views.
#[derive(Clone, Debug)]
pub(crate) struct PagerKeymap {
    pub(crate) scroll_up: Vec<KeyBinding>,
    pub(crate) scroll_down: Vec<KeyBinding>,
    pub(crate) page_up: Vec<KeyBinding>,
    pub(crate) page_down: Vec<KeyBinding>,
    pub(crate) half_page_up: Vec<KeyBinding>,
    pub(crate) half_page_down: Vec<KeyBinding>,
    pub(crate) jump_top: Vec<KeyBinding>,
    pub(crate) jump_bottom: Vec<KeyBinding>,
    pub(crate) close: Vec<KeyBinding>,
    pub(crate) close_transcript: Vec<KeyBinding>,
}

/// Generic list picker keybindings shared across popup list views.
#[derive(Clone, Debug)]
pub(crate) struct ListKeymap {
    pub(crate) move_up: Vec<KeyBinding>,
    pub(crate) move_down: Vec<KeyBinding>,
    pub(crate) accept: Vec<KeyBinding>,
    pub(crate) cancel: Vec<KeyBinding>,
}

/// Approval modal keybindings.
///
/// This covers both selection actions and the "open details fullscreen" escape
/// hatch for large approval payloads.
#[derive(Clone, Debug)]
pub(crate) struct ApprovalKeymap {
    pub(crate) open_fullscreen: Vec<KeyBinding>,
    pub(crate) open_thread: Vec<KeyBinding>,
    pub(crate) approve: Vec<KeyBinding>,
    pub(crate) approve_for_session: Vec<KeyBinding>,
    pub(crate) approve_for_prefix: Vec<KeyBinding>,
    pub(crate) deny: Vec<KeyBinding>,
    pub(crate) decline: Vec<KeyBinding>,
    pub(crate) cancel: Vec<KeyBinding>,
}

/// Returns the first binding, used as the primary UI hint for an action.
///
/// Rendering code should prefer this for concise hints while preserving all
/// bindings for actual input matching.
pub(crate) fn primary_binding(bindings: &[KeyBinding]) -> Option<KeyBinding> {
    bindings.first().copied()
}

