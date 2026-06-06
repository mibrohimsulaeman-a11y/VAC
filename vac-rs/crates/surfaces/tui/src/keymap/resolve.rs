use super::*;
use crate::key_hint::KeyBinding;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use vac_config::types::KeybindingsSpec;
use vac_config::types::TuiKeymap;

impl RuntimeKeymap {
    /// Return built-in defaults.
    ///
    /// This is a convenience for tests and bootstrapping UI state before user
    /// config has been loaded. It should not be used as a fallback after
    /// parsing `TuiKeymap`, because doing so would ignore explicit user
    /// unbindings and conflict diagnostics.
    pub(crate) fn defaults() -> Self {
        Self::built_in_defaults()
    }

    /// Resolve a runtime keymap from config, applying precedence and validation.
    ///
    /// Returns an error when:
    ///
    /// 1. A keybinding spec cannot be parsed.
    /// 2. A context has ambiguous bindings (same key assigned to multiple actions).
    ///
    /// The error text includes the relevant config path and a concrete next step.
    /// Calling code should not merge bindings across unrelated contexts before
    /// dispatch, or conflict guarantees from this resolver no longer hold.
    pub(crate) fn from_config(keymap: &TuiKeymap) -> Result<Self, String> {
        let defaults = Self::built_in_defaults();

        let app = AppKeymap {
            open_transcript: resolve_bindings(
                keymap.global.open_transcript.as_ref(),
                &defaults.app.open_transcript,
                "tui.keymap.global.open_transcript",
            )?,
            open_external_editor: resolve_bindings(
                keymap.global.open_external_editor.as_ref(),
                &defaults.app.open_external_editor,
                "tui.keymap.global.open_external_editor",
            )?,
            copy: resolve_bindings(
                keymap.global.copy.as_ref(),
                &defaults.app.copy,
                "tui.keymap.global.copy",
            )?,
            clear_terminal: resolve_bindings(
                keymap.global.clear_terminal.as_ref(),
                &defaults.app.clear_terminal,
                "tui.keymap.global.clear_terminal",
            )?,
            toggle_vim_mode: resolve_bindings(
                keymap.global.toggle_vim_mode.as_ref(),
                &defaults.app.toggle_vim_mode,
                "tui.keymap.global.toggle_vim_mode",
            )?,
        };

        let chat = ChatKeymap {
            decrease_reasoning_effort: resolve_bindings(
                keymap.chat.decrease_reasoning_effort.as_ref(),
                &defaults.chat.decrease_reasoning_effort,
                "tui.keymap.chat.decrease_reasoning_effort",
            )?,
            increase_reasoning_effort: resolve_bindings(
                keymap.chat.increase_reasoning_effort.as_ref(),
                &defaults.chat.increase_reasoning_effort,
                "tui.keymap.chat.increase_reasoning_effort",
            )?,
            edit_queued_message: resolve_bindings(
                keymap.chat.edit_queued_message.as_ref(),
                &defaults.chat.edit_queued_message,
                "tui.keymap.chat.edit_queued_message",
            )?,
        };

        let composer = ComposerKeymap {
            submit: resolve_with_global!(keymap, defaults, composer, submit),
            queue: resolve_with_global!(keymap, defaults, composer, queue),
            toggle_shortcuts: resolve_with_global!(keymap, defaults, composer, toggle_shortcuts),
            history_search_previous: resolve_local!(
                keymap,
                defaults,
                composer,
                history_search_previous
            ),
            history_search_next: resolve_local!(keymap, defaults, composer, history_search_next),
        };

        let editor = EditorKeymap {
            insert_newline: resolve_local!(keymap, defaults, editor, insert_newline),
            move_left: resolve_local!(keymap, defaults, editor, move_left),
            move_right: resolve_local!(keymap, defaults, editor, move_right),
            move_up: resolve_local!(keymap, defaults, editor, move_up),
            move_down: resolve_local!(keymap, defaults, editor, move_down),
            move_word_left: resolve_local!(keymap, defaults, editor, move_word_left),
            move_word_right: resolve_local!(keymap, defaults, editor, move_word_right),
            move_line_start: resolve_local!(keymap, defaults, editor, move_line_start),
            move_line_end: resolve_local!(keymap, defaults, editor, move_line_end),
            delete_backward: resolve_local!(keymap, defaults, editor, delete_backward),
            delete_forward: resolve_local!(keymap, defaults, editor, delete_forward),
            delete_backward_word: resolve_local!(keymap, defaults, editor, delete_backward_word),
            delete_forward_word: resolve_local!(keymap, defaults, editor, delete_forward_word),
            kill_line_start: resolve_local!(keymap, defaults, editor, kill_line_start),
            kill_line_end: resolve_local!(keymap, defaults, editor, kill_line_end),
            yank: resolve_local!(keymap, defaults, editor, yank),
        };

        let vim_normal = VimNormalKeymap {
            enter_insert: resolve_local!(keymap, defaults, vim_normal, enter_insert),
            append_after_cursor: resolve_local!(keymap, defaults, vim_normal, append_after_cursor),
            append_line_end: resolve_local!(keymap, defaults, vim_normal, append_line_end),
            insert_line_start: resolve_local!(keymap, defaults, vim_normal, insert_line_start),
            open_line_below: resolve_local!(keymap, defaults, vim_normal, open_line_below),
            open_line_above: resolve_local!(keymap, defaults, vim_normal, open_line_above),
            move_left: resolve_local!(keymap, defaults, vim_normal, move_left),
            move_right: resolve_local!(keymap, defaults, vim_normal, move_right),
            move_up: resolve_local!(keymap, defaults, vim_normal, move_up),
            move_down: resolve_local!(keymap, defaults, vim_normal, move_down),
            move_word_forward: resolve_local!(keymap, defaults, vim_normal, move_word_forward),
            move_word_backward: resolve_local!(keymap, defaults, vim_normal, move_word_backward),
            move_word_end: resolve_local!(keymap, defaults, vim_normal, move_word_end),
            move_line_start: resolve_local!(keymap, defaults, vim_normal, move_line_start),
            move_line_end: resolve_local!(keymap, defaults, vim_normal, move_line_end),
            delete_char: resolve_local!(keymap, defaults, vim_normal, delete_char),
            delete_to_line_end: resolve_local!(keymap, defaults, vim_normal, delete_to_line_end),
            yank_line: resolve_local!(keymap, defaults, vim_normal, yank_line),
            paste_after: resolve_local!(keymap, defaults, vim_normal, paste_after),
            start_delete_operator: resolve_local!(
                keymap,
                defaults,
                vim_normal,
                start_delete_operator
            ),
            start_yank_operator: resolve_local!(keymap, defaults, vim_normal, start_yank_operator),
            cancel_operator: resolve_local!(keymap, defaults, vim_normal, cancel_operator),
        };

        let vim_operator = VimOperatorKeymap {
            delete_line: resolve_local!(keymap, defaults, vim_operator, delete_line),
            yank_line: resolve_local!(keymap, defaults, vim_operator, yank_line),
            motion_left: resolve_local!(keymap, defaults, vim_operator, motion_left),
            motion_right: resolve_local!(keymap, defaults, vim_operator, motion_right),
            motion_up: resolve_local!(keymap, defaults, vim_operator, motion_up),
            motion_down: resolve_local!(keymap, defaults, vim_operator, motion_down),
            motion_word_forward: resolve_local!(
                keymap,
                defaults,
                vim_operator,
                motion_word_forward
            ),
            motion_word_backward: resolve_local!(
                keymap,
                defaults,
                vim_operator,
                motion_word_backward
            ),
            motion_word_end: resolve_local!(keymap, defaults, vim_operator, motion_word_end),
            motion_line_start: resolve_local!(keymap, defaults, vim_operator, motion_line_start),
            motion_line_end: resolve_local!(keymap, defaults, vim_operator, motion_line_end),
            cancel: resolve_local!(keymap, defaults, vim_operator, cancel),
        };

        let pager = PagerKeymap {
            scroll_up: resolve_local!(keymap, defaults, pager, scroll_up),
            scroll_down: resolve_local!(keymap, defaults, pager, scroll_down),
            page_up: resolve_local!(keymap, defaults, pager, page_up),
            page_down: resolve_local!(keymap, defaults, pager, page_down),
            half_page_up: resolve_local!(keymap, defaults, pager, half_page_up),
            half_page_down: resolve_local!(keymap, defaults, pager, half_page_down),
            jump_top: resolve_local!(keymap, defaults, pager, jump_top),
            jump_bottom: resolve_local!(keymap, defaults, pager, jump_bottom),
            close: resolve_local!(keymap, defaults, pager, close),
            close_transcript: resolve_local!(keymap, defaults, pager, close_transcript),
        };

        let list = ListKeymap {
            move_up: resolve_local!(keymap, defaults, list, move_up),
            move_down: resolve_local!(keymap, defaults, list, move_down),
            accept: resolve_local!(keymap, defaults, list, accept),
            cancel: resolve_local!(keymap, defaults, list, cancel),
        };

        let approval = ApprovalKeymap {
            open_fullscreen: resolve_local!(keymap, defaults, approval, open_fullscreen),
            open_thread: resolve_local!(keymap, defaults, approval, open_thread),
            approve: resolve_local!(keymap, defaults, approval, approve),
            approve_for_session: resolve_local!(keymap, defaults, approval, approve_for_session),
            approve_for_prefix: resolve_local!(keymap, defaults, approval, approve_for_prefix),
            deny: resolve_local!(keymap, defaults, approval, deny),
            decline: resolve_local!(keymap, defaults, approval, decline),
            cancel: resolve_local!(keymap, defaults, approval, cancel),
        };

        let resolved = Self {
            app,
            chat,
            composer,
            editor,
            vim_normal,
            vim_operator,
            pager,
            list,
            approval,
        };

        resolved.validate_conflicts()?;
        Ok(resolved)
    }
}

/// Resolve one action with context -> global -> default precedence.
///
/// `path` should be the context-specific config path so parser errors point
/// users at the override they attempted to set.
///
/// A configured empty list is authoritative: it returns an empty binding set
/// and does not continue to the global or built-in fallback. This is what makes
/// explicit unbinding work for globally reusable actions like composer submit.
fn resolve_bindings_with_global_fallback(
    configured: Option<&KeybindingsSpec>,
    global: Option<&KeybindingsSpec>,
    fallback: &[KeyBinding],
    path: &str,
) -> Result<Vec<KeyBinding>, String> {
    if let Some(configured) = configured {
        return parse_bindings(configured, path);
    }
    if let Some(global) = global {
        return parse_bindings(global, path);
    }
    Ok(fallback.to_vec())
}

/// Resolve one action binding in a context without global fallback.
///
/// Missing values inherit from the built-in fallback; configured values, including
/// empty lists, replace that fallback for the action.
fn resolve_bindings(
    configured: Option<&KeybindingsSpec>,
    fallback: &[KeyBinding],
    path: &str,
) -> Result<Vec<KeyBinding>, String> {
    let Some(spec) = configured else {
        return Ok(fallback.to_vec());
    };
    parse_bindings(spec, path)
}

/// Parse one keybinding value (`string` or `list[string]`) into concrete bindings.
///
/// Duplicate entries are de-duplicated while preserving first-seen order so the
/// first key can remain the primary UI hint.
fn parse_bindings(spec: &KeybindingsSpec, path: &str) -> Result<Vec<KeyBinding>, String> {
    let mut parsed = Vec::new();
    for raw in spec.specs() {
        let binding = parse_keybinding(raw.as_str()).ok_or_else(|| {
            format!(
                "Invalid `{path}` = `{}`. Use values like `ctrl-a`, `shift-enter`, or `page-down`. \
See the VAC keymap documentation for supported actions and examples.",
                raw.as_str()
            )
        })?;

        if !parsed.contains(&binding) {
            parsed.push(binding);
        }
    }
    Ok(parsed)
}

/// Parse one normalized keybinding spec such as `ctrl-a` or `shift-enter`.
///
/// Specs are expected to be normalized by config deserialization, but this
/// parser remains strict to keep runtime error messages precise.
pub(super) fn parse_keybinding(spec: &str) -> Option<KeyBinding> {
    let mut parts = spec.split('-');
    let mut modifiers = KeyModifiers::NONE;
    let mut key_name = None;

    for part in parts.by_ref() {
        match part {
            "ctrl" => modifiers |= KeyModifiers::CONTROL,
            "alt" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            other => {
                key_name = Some(other.to_string());
                break;
            }
        }
    }

    let mut key_name = key_name?;
    for trailing in parts {
        key_name.push('-');
        key_name.push_str(trailing);
    }

    let key = match key_name.as_str() {
        "enter" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "esc" => KeyCode::Esc,
        "delete" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "page-up" => KeyCode::PageUp,
        "page-down" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        other if other.len() == 1 => KeyCode::Char(char::from(other.as_bytes()[0])),
        other if other.starts_with('f') => {
            let number = other[1..].parse::<u8>().ok()?;
            if (1..=12).contains(&number) {
                KeyCode::F(number)
            } else {
                return None;
            }
        }
        _ => return None,
    };

    Some(KeyBinding::new(key, modifiers))
}
