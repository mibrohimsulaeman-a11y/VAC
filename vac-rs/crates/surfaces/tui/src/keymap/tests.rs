use super::*;
use super::resolve::parse_keybinding;
use crate::key_hint;
use crate::key_hint::KeyBinding;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use vac_config::types::KeybindingSpec;
use vac_config::types::KeybindingsSpec;
use vac_config::types::TuiKeymap;


    fn one(spec: &str) -> KeybindingsSpec {
        KeybindingsSpec::One(KeybindingSpec(spec.to_string()))
    }

    fn expect_conflict(keymap: &TuiKeymap, first: &str, second: &str) {
        let err = RuntimeKeymap::from_config(keymap).expect_err("expected conflict");
        assert!(err.contains(first));
        assert!(err.contains(second));
    }

    #[test]
    fn parses_canonical_binding() {
        let binding = parse_keybinding("ctrl-alt-shift-a").expect("binding should parse");
        assert_eq!(binding.parts().0, KeyCode::Char('a'));
        assert_eq!(
            binding.parts().1,
            KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
        );
    }

    #[test]
    fn rejects_shadowing_composer_binding_in_app_scope() {
        let mut keymap = TuiKeymap::default();
        keymap.global.open_transcript = Some(one("ctrl-t"));
        keymap.composer.submit = Some(one("ctrl-t"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected shadowing conflict");
        assert!(err.contains("composer.submit"));
        assert!(err.contains("open_transcript"));
    }

    #[test]
    fn rejects_shadowing_composer_queue_in_app_scope() {
        let mut keymap = TuiKeymap::default();
        keymap.global.open_external_editor = Some(one("ctrl-g"));
        keymap.composer.queue = Some(one("ctrl-g"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected shadowing conflict");
        assert!(err.contains("composer.queue"));
        assert!(err.contains("open_external_editor"));
    }

    #[test]
    fn rejects_shadowing_composer_toggle_shortcuts_in_app_scope() {
        let mut keymap = TuiKeymap::default();
        keymap.global.open_transcript = Some(one("ctrl-k"));
        keymap.composer.toggle_shortcuts = Some(one("ctrl-k"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected shadowing conflict");
        assert!(err.contains("composer.toggle_shortcuts"));
        assert!(err.contains("open_transcript"));
    }

    #[test]
    fn rejects_shadowing_editor_binding_in_main_scope() {
        let mut keymap = TuiKeymap::default();
        keymap.composer.submit = Some(one("ctrl-j"));
        keymap.editor.insert_newline = Some(one("ctrl-j"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected shadowing conflict");
        assert!(err.contains("composer.submit"));
        assert!(err.contains("editor.insert_newline"));
    }

    #[test]
    fn rejects_shadowing_editor_binding_from_outer_main_handler() {
        let mut keymap = TuiKeymap::default();
        keymap.global.copy = Some(one("ctrl-y"));
        keymap.editor.yank = Some(one("ctrl-y"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected shadowing conflict");
        assert!(err.contains("copy"));
        assert!(err.contains("editor.yank"));
    }

    #[test]
    fn rejects_shadowing_approval_binding_in_app_scope() {
        let mut keymap = TuiKeymap::default();
        keymap.global.open_transcript = Some(one("y"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected shadowing conflict");
        assert!(err.contains("approval.approve"));
        assert!(err.contains("open_transcript"));
    }

    #[test]
    fn rejects_shadowing_list_binding_in_app_scope() {
        let mut keymap = TuiKeymap::default();
        keymap.global.copy = Some(one("down"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected shadowing conflict");
        assert!(err.contains("list.move_down"));
        assert!(err.contains("copy"));
    }

    #[test]
    fn supports_string_or_array_bindings() {
        let mut keymap = TuiKeymap::default();
        keymap.composer.submit = Some(KeybindingsSpec::Many(vec![
            KeybindingSpec("ctrl-enter".to_string()),
            KeybindingSpec("meta-enter".to_string()),
        ]));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("meta is not a valid modifier");
        assert!(err.contains("tui.keymap.composer.submit"));

        keymap.composer.submit = Some(KeybindingsSpec::Many(vec![
            KeybindingSpec("ctrl-enter".to_string()),
            KeybindingSpec("ctrl-shift-enter".to_string()),
        ]));

        let runtime = RuntimeKeymap::from_config(&keymap).expect("valid multi-binding");
        assert_eq!(runtime.composer.submit.len(), 2);
    }

    #[test]
    fn deduplicates_repeated_bindings_while_preserving_first_seen_order() {
        let mut keymap = TuiKeymap::default();
        keymap.composer.submit = Some(KeybindingsSpec::Many(vec![
            KeybindingSpec("ctrl-enter".to_string()),
            KeybindingSpec("ctrl-enter".to_string()),
            KeybindingSpec("ctrl-shift-enter".to_string()),
        ]));

        let runtime = RuntimeKeymap::from_config(&keymap).expect("valid multi-binding");
        assert_eq!(
            runtime.composer.submit,
            vec![
                key_hint::ctrl(KeyCode::Enter),
                KeyBinding::new(KeyCode::Enter, KeyModifiers::CONTROL | KeyModifiers::SHIFT)
            ]
        );
    }

    #[test]
    fn falls_back_to_global_binding_when_context_override_is_not_set() {
        let mut keymap = TuiKeymap::default();
        keymap.global.queue = Some(one("ctrl-q"));

        let runtime = RuntimeKeymap::from_config(&keymap).expect("config should parse");
        assert_eq!(
            runtime.composer.queue,
            vec![key_hint::ctrl(KeyCode::Char('q'))]
        );
    }

    #[test]
    fn invalid_global_open_transcript_binding_reports_global_path() {
        let mut keymap = TuiKeymap::default();
        keymap.global.open_transcript = Some(one("meta-t"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected parse error");
        assert!(err.contains("tui.keymap.global.open_transcript"));
    }

    #[test]
    fn invalid_global_open_external_editor_binding_reports_global_path() {
        let mut keymap = TuiKeymap::default();
        keymap.global.open_external_editor = Some(one("meta-g"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected parse error");
        assert!(err.contains("tui.keymap.global.open_external_editor"));
    }

    #[test]
    fn default_copy_binding_is_ctrl_o() {
        let runtime = RuntimeKeymap::defaults();
        assert_eq!(runtime.app.copy, vec![key_hint::ctrl(KeyCode::Char('o'))]);
    }

    #[test]
    fn defaults_include_reassignable_main_surface_actions() {
        let runtime = RuntimeKeymap::defaults();

        assert_eq!(
            runtime.app.clear_terminal,
            vec![key_hint::ctrl(KeyCode::Char('l'))]
        );
        assert_eq!(
            runtime.chat.decrease_reasoning_effort,
            vec![key_hint::alt(KeyCode::Char(','))]
        );
        assert_eq!(
            runtime.chat.increase_reasoning_effort,
            vec![key_hint::alt(KeyCode::Char('.'))]
        );
        assert_eq!(
            runtime.chat.edit_queued_message,
            vec![key_hint::alt(KeyCode::Up), key_hint::shift(KeyCode::Left)]
        );
        assert_eq!(
            runtime.composer.history_search_previous,
            vec![key_hint::ctrl(KeyCode::Char('r'))]
        );
        assert_eq!(
            runtime.composer.history_search_next,
            vec![key_hint::ctrl(KeyCode::Char('s'))]
        );
    }

    #[test]
    fn vim_normal_defaults_include_insert_and_arrow_aliases() {
        let runtime = RuntimeKeymap::defaults();

        assert_eq!(
            runtime.vim_normal.enter_insert,
            vec![
                key_hint::plain(KeyCode::Char('i')),
                key_hint::plain(KeyCode::Insert)
            ]
        );
        assert_eq!(
            runtime.vim_normal.move_left,
            vec![
                key_hint::plain(KeyCode::Char('h')),
                key_hint::plain(KeyCode::Left)
            ]
        );
        assert_eq!(
            runtime.vim_normal.move_right,
            vec![
                key_hint::plain(KeyCode::Char('l')),
                key_hint::plain(KeyCode::Right)
            ]
        );
        assert_eq!(
            runtime.vim_normal.move_up,
            vec![
                key_hint::plain(KeyCode::Char('k')),
                key_hint::plain(KeyCode::Up)
            ]
        );
        assert_eq!(
            runtime.vim_normal.move_down,
            vec![
                key_hint::plain(KeyCode::Char('j')),
                key_hint::plain(KeyCode::Down)
            ]
        );
    }

    #[test]
    fn invalid_global_copy_binding_reports_global_path() {
        let mut keymap = TuiKeymap::default();
        keymap.global.copy = Some(one("meta-o"));

        let err = RuntimeKeymap::from_config(&keymap).expect_err("expected parse error");
        assert!(err.contains("tui.keymap.global.copy"));
    }

    #[test]
    fn rejects_conflicting_editor_bindings() {
        let mut keymap = TuiKeymap::default();
        keymap.editor.move_left = Some(one("ctrl-h"));
        keymap.editor.move_right = Some(one("ctrl-h"));

        expect_conflict(&keymap, "move_left", "move_right");
    }

    #[test]
    fn rejects_conflicting_pager_bindings() {
        let mut keymap = TuiKeymap::default();
        keymap.pager.scroll_up = Some(one("ctrl-u"));
        keymap.pager.scroll_down = Some(one("ctrl-u"));

        expect_conflict(&keymap, "scroll_up", "scroll_down");
    }

    #[test]
    fn rejects_conflicting_list_bindings() {
        let mut keymap = TuiKeymap::default();
        keymap.list.move_up = Some(one("up"));
        keymap.list.move_down = Some(one("up"));

        expect_conflict(&keymap, "move_up", "move_down");
    }

    #[test]
    fn rejects_conflicting_approval_bindings() {
        let mut keymap = TuiKeymap::default();
        keymap.approval.approve = Some(one("y"));
        keymap.approval.decline = Some(one("y"));

        expect_conflict(&keymap, "approve", "decline");
    }

    #[test]
    fn rejects_conflicting_approval_deny_binding() {
        let mut keymap = TuiKeymap::default();
        keymap.approval.approve = Some(one("y"));
        keymap.approval.deny = Some(one("y"));

        expect_conflict(&keymap, "approve", "deny");
    }

    #[test]
    fn rejects_conflicting_approval_overlay_accept_binding() {
        let mut keymap = TuiKeymap::default();
        keymap.list.accept = Some(one("y"));

        expect_conflict(&keymap, "list.accept", "approval.approve");
    }

    #[test]
    fn rejects_conflicting_approval_overlay_cancel_binding() {
        let mut keymap = TuiKeymap::default();
        keymap.list.cancel = Some(one("c"));

        expect_conflict(&keymap, "list.cancel", "approval.cancel");
    }

    #[test]
    fn reassignable_fixed_shortcuts_conflict_until_original_action_is_unbound() {
        let mut keymap = TuiKeymap::default();
        keymap.global.copy = Some(one("alt-."));

        expect_conflict(&keymap, "copy", "chat.increase_reasoning_effort");

        keymap.chat.increase_reasoning_effort = Some(KeybindingsSpec::Many(vec![]));
        let runtime = RuntimeKeymap::from_config(&keymap).expect("remapped key should be free");
        assert_eq!(runtime.app.copy, vec![key_hint::alt(KeyCode::Char('.'))]);
    }

    #[test]
    fn rejects_main_bindings_that_collide_with_remaining_fixed_shortcuts() {
        let mut keymap = TuiKeymap::default();
        keymap.composer.submit = Some(one("ctrl-v"));

        expect_conflict(&keymap, "composer.submit", "fixed.paste_image");
    }

    #[test]
    fn rejects_pager_bindings_that_collide_with_transcript_backtrack_keys() {
        let mut keymap = TuiKeymap::default();
        keymap.pager.close = Some(one("left"));

        expect_conflict(&keymap, "close", "fixed.transcript_edit_previous");
    }

    #[test]
    fn parses_function_keys_and_rejects_out_of_range_function_keys() {
        assert_eq!(
            parse_keybinding("f1").map(|binding| binding.parts()),
            Some((KeyCode::F(1), KeyModifiers::NONE))
        );
        assert_eq!(parse_keybinding("f13"), None);
    }

    #[test]
    fn parses_all_named_non_character_keys() {
        let cases = [
            ("tab", KeyCode::Tab),
            ("backspace", KeyCode::Backspace),
            ("esc", KeyCode::Esc),
            ("delete", KeyCode::Delete),
            ("up", KeyCode::Up),
            ("down", KeyCode::Down),
            ("left", KeyCode::Left),
            ("right", KeyCode::Right),
            ("home", KeyCode::Home),
            ("end", KeyCode::End),
            ("page-up", KeyCode::PageUp),
            ("page-down", KeyCode::PageDown),
            ("space", KeyCode::Char(' ')),
        ];

        for (spec, expected_key) in cases {
            assert_eq!(
                parse_keybinding(spec).map(|binding| binding.parts()),
                Some((expected_key, KeyModifiers::NONE)),
                "failed to parse {spec}"
            );
        }
    }

    #[test]
    fn rejects_modifier_only_and_nonnumeric_function_key_specs() {
        assert_eq!(parse_keybinding("ctrl"), None);
        assert_eq!(parse_keybinding("ff"), None);
    }

    #[test]
    fn explicit_empty_array_unbinds_action() {
        let mut keymap = TuiKeymap::default();
        keymap.composer.toggle_shortcuts = Some(KeybindingsSpec::Many(vec![]));
        let runtime = RuntimeKeymap::from_config(&keymap).expect("config should parse");
        assert!(runtime.composer.toggle_shortcuts.is_empty());
    }

    #[test]
    fn default_editor_insert_newline_includes_current_aliases() {
        let runtime = RuntimeKeymap::defaults();
        assert_eq!(
            runtime.editor.insert_newline,
            vec![
                key_hint::ctrl(KeyCode::Char('j')),
                key_hint::plain(KeyCode::Enter),
                key_hint::shift(KeyCode::Enter),
                key_hint::alt(KeyCode::Enter),
            ]
        );
    }

    #[test]
    fn default_composer_submit_accepts_enter_and_terminal_cr() {
        let runtime = RuntimeKeymap::defaults();
        assert_eq!(
            runtime.composer.submit,
            vec![
                key_hint::plain(KeyCode::Enter),
                key_hint::ctrl(KeyCode::Char('m')),
            ]
        );
    }

    #[test]
    fn default_editor_delete_forward_word_includes_alt_d() {
        let runtime = RuntimeKeymap::defaults();
        assert!(
            runtime
                .editor
                .delete_forward_word
                .contains(&key_hint::alt(KeyCode::Char('d')))
        );
    }

    #[test]
    fn default_composer_toggle_shortcuts_includes_shift_question_mark() {
        let runtime = RuntimeKeymap::defaults();
        assert!(
            runtime
                .composer
                .toggle_shortcuts
                .contains(&key_hint::shift(KeyCode::Char('?')))
        );
    }

    #[test]
    fn default_approval_open_fullscreen_includes_ctrl_shift_a() {
        let runtime = RuntimeKeymap::defaults();
        assert!(runtime.approval.open_fullscreen.contains(&KeyBinding::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        )));
    }

    #[test]
    fn primary_binding_returns_first_or_none() {
        let bindings = vec![
            key_hint::ctrl(KeyCode::Char('a')),
            key_hint::shift(KeyCode::Char('b')),
        ];
        assert_eq!(
            primary_binding(&bindings),
            Some(key_hint::ctrl(KeyCode::Char('a')))
        );
        assert_eq!(primary_binding(&[]), None);
    }

    #[test]
    fn defaults_pass_conflict_validation() {
        RuntimeKeymap::defaults()
            .validate_conflicts()
            .expect("default keymap should be conflict free");
    }
