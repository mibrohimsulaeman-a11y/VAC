#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
errors: list[str] = []


def read(rel: str) -> str:
    path = root / rel
    if not path.exists():
        errors.append(f"missing {rel}")
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def require(name: str, ok: bool) -> None:
    if not ok:
        errors.append(name)


def function_body(source: str, signature: str) -> str:
    start = source.find(signature)
    if start < 0:
        return ""
    brace = source.find("{", start)
    if brace < 0:
        return ""
    depth = 0
    for index in range(brace, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[start : index + 1]
    return source[start:]


operator = read("vac-rs/crates/surfaces/vac-tui/src/services/vac_operator.rs")
handlers = read("vac-rs/crates/surfaces/vac-tui/src/services/handlers/mod.rs")
message = read("vac-rs/crates/surfaces/vac-tui/src/services/handlers/message.rs")
popup = read("vac-rs/crates/surfaces/vac-tui/src/services/handlers/popup.rs")
event = read("vac-rs/crates/surfaces/vac-tui/src/event.rs")
commands = read("vac-rs/crates/surfaces/vac-tui/src/services/commands.rs")
event_loop = read("vac-rs/crates/surfaces/vac-tui/src/event_loop.rs")
plan = read("vac-rs/crates/surfaces/vac-tui/src/services/plan.rs")
plan_review = read("vac-rs/crates/surfaces/vac-tui/src/services/plan_review.rs")
app = read("vac-rs/crates/surfaces/vac-tui/src/app.rs")
wrapping = read("vac-rs/crates/surfaces/vac-tui/src/services/wrapping.rs")
secret_manager = read("vac-rs/crates/foundation/vac-foundation/src/secret_manager.rs")
pty_smoke = read("scripts/pty-tui-lifecycle-smoke.py")

handle_toggle = function_body(handlers, "fn handle_toggle_plan_review")
close_overlays = function_body(handlers, "fn close_composer_locking_overlays")
push_lifecycle = function_body(handlers, "fn push_plan_lifecycle_message")
plan_toggle_lifecycle = handle_toggle + close_overlays + push_lifecycle
render_idle = function_body(operator, "fn render_idle")
render_feed = function_body(operator, "fn render_conversation_stream")
plan_start = commands.find('"/plan" => {')
plan_end = commands.find('"/init" => {', plan_start) if plan_start >= 0 else -1
plan_branch = commands[plan_start:plan_end] if plan_start >= 0 and plan_end > plan_start else ""


# Production maturity guards added after the lifecycle audit.
require(
    "secret_manager_no_global_keyword_fast_path",
    "content_has_redaction_candidate" not in secret_manager
    and "CANDIDATE_MARKERS" not in secret_manager
    and "if !self.redact_secrets" in secret_manager
    and "redact_secrets(content, path" in secret_manager,
)
require(
    "canonical_plan_session_dir_declared",
    "pub const PLAN_SESSION_DIR: &str = \".vac/session\"" in plan
    and "pub fn current_plan_session_dir()" in plan,
)
require(
    "plan_paths_use_canonical_session_dir",
    ".vac/registry/sessions/current" not in commands
    and ".vac/registry/sessions/current" not in app
    and "current_plan_session_dir()" in commands
    and "current_plan_session_dir()" in app
    and "current_plan_session_dir()" in plan_review
    and "current_plan_session_dir()" in handlers,
)
require(
    "event_loop_lifecycle_dispatch_helpers",
    all(
        token in event_loop
        for token in [
            "fn compute_message_area(",
            "fn handle_quit_event(",
            "fn handle_pending_editor_open",
            "fn dispatch_internal_input_event",
            "fn dispatch_backend_event",
            "MAX_INTERNAL_INPUT_DRAIN_PER_FRAME",
            "prefer_backend_after_internal",
            "if prefer_backend_after_internal",
            "fn dispatch_inbound_backend_event",
        ]
    ),
)
require(
    "wrapping_no_unsafe_offset_or_owned_panic",
    "offset_from" not in wrapping
    and "unexpected owned string" not in wrapping
    and "locate_wrapped_range" in wrapping,
)
require(
    "official_pty_smoke_gate_present",
    "VAC TUI PTY lifecycle smoke" in pty_smoke
    and "entered_alt_screen" in pty_smoke
    and "shift_tab_or_plan_visible" in pty_smoke
    and "plain_text_echo_visible" in pty_smoke,
)

# Shift+Tab must be a real source path, not only a doc/static key legend.
require(
    "shift_tab_keymap",
    "KeyCode::BackTab => Some(InputEvent::TogglePlanReview)" in event,
)
require(
    "shift_tab_handler_present",
    "fn handle_toggle_plan_review" in handlers
    and "InputEvent::TogglePlanReview =>" in handlers
    and "handle_toggle_plan_review(state, output_tx)" in handlers,
)
require(
    "shift_tab_closes_locking_overlays",
    all(
        token in plan_toggle_lifecycle
        for token in [
            "show_helper_dropdown = false",
            "command_palette_state.is_visible = false",
            "model_switcher_state.is_visible = false",
            "profile_switcher_state.show_profile_switcher = false",
            "rulebook_switcher_state.show_rulebook_switcher = false",
            "shortcuts_panel_state.is_visible = false",
        ]
    ),
)
require(
    "shift_tab_routes_to_review",
    "vac_operator_state.route" in handle_toggle and "VacOperatorRoute::Review" in handle_toggle,
)
require(
    "shift_tab_enables_plan_mode",
    "plan_mode_state.is_active = true" in handle_toggle
    and "OutputEvent::PlanModeActivated(None)" in handle_toggle,
)
require(
    "shift_tab_toggles_visible_lifecycle",
    "plan_review_state.is_visible = !state.plan_review_state.is_visible" in handle_toggle
    and "Plan review opened. Press Shift+Tab again to return to the feed." in handle_toggle
    and "Plan review closed. Plan mode remains active." in handle_toggle,
)
require(
    "shift_tab_pushes_visible_feed_message",
    "Message::info" in plan_toggle_lifecycle
    and "Plan mode enabled. Draft the approach first; execution remains gated by VAC runtime policy." in handle_toggle
    and "invalidate_message_lines_cache" in plan_toggle_lifecycle
    and "scroll_to_bottom = true" in plan_toggle_lifecycle,
)
bad_esc_toggle_group = """InputEvent::HandleEsc
                | InputEvent::PlanReviewClose
                | InputEvent::TogglePlanReview"""
require(
    "esc_close_does_not_toggle_plan_mode",
    "InputEvent::HandleEsc | InputEvent::PlanReviewClose" in handlers
    and bad_esc_toggle_group not in handlers,
)

# Idle/feed must render a persistent visual plan-mode indicator.
require(
    "operator_plan_indicator_literal",
    "plan mode active — draft/review before execution" in operator
    and "fn plan_mode_indicator_line()" in operator,
)
require(
    "operator_idle_plan_indicator",
    "state.plan_mode_state.is_active" in render_idle
    and "plan_mode_indicator_line()" in render_idle,
)
require(
    "operator_feed_plan_indicator",
    "state.plan_mode_state.is_active" in render_feed
    and "plan_mode_indicator_line()" in render_feed,
)

# /plan must visibly affect the feed, not only emit an OutputEvent.
require("plan_command_found", bool(plan_branch))
require(
    "plan_command_visible_feed_indicator",
    "Message::info" in plan_branch
    and "Plan mode enabled. Draft/review before execution; Shift+Tab toggles plan review." in plan_branch
    and "Plan mode already active — draft/review before execution; Shift+Tab toggles plan review." in plan_branch
    and "invalidate_message_lines_cache" in plan_branch
    and "scroll_to_bottom = true" in plan_branch,
)
require(
    "plan_command_sets_review_route",
    "plan_mode_state.is_active = true" in plan_branch and "VacOperatorRoute::Review" in plan_branch,
)

# Regression guards for prior lifecycle findings.
require(
    "first_launch_timer",
    "startup_hydrating" in operator and "as_secs_f32" in operator and "< 3.0" in operator,
)
require(
    "right_context_rail_present",
    "fn render_right_context_rail" in operator and "ctx" in operator and "tls" in operator,
)
require(
    "right_context_panel_present",
    "fn render_right_context_panel" in operator
    and "context window" in operator
    and "tool timeline" in operator,
)
require(
    "canonical_overlays_present",
    "fn render_operator_overlays" in operator
    and "render_model_switcher_popup" in operator
    and "render_profile_switcher_popup" in operator,
)
require(
    "submit_forces_feed",
    "state.vac_operator_state.route = VacOperatorRoute::Chat;" in message,
)
require(
    "model_switcher_unlock_guard",
    "No selectable models are available yet" in popup
    and "No model matches the current filter." in popup,
)
require(
    "context_tool_commands_present",
    '"/context" | "/timeline" | "/tools"' in commands,
)
require(
    "mock_tabs_absent",
    "fn render_tab_bar" not in operator
    and " workbench " not in operator.lower()
    and " mcp " not in operator.lower(),
)

if errors:
    print("VAC TUI E2E static lifecycle gate: FAIL")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)
print("VAC TUI E2E static lifecycle gate: PASS")
