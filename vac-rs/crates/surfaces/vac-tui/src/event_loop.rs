//! Event Loop Module
//!
//! Contains the main TUI event loop and related helper functions.

use crate::app::{AppState, AppStateOptions, InputEvent, OutputEvent};
use crate::services::banner::BannerMessage;
use crate::services::detect_term::ThemeColors;
use crate::services::handlers::tool::{
    clear_streaming_tool_results, handle_tool_result, update_session_tool_calls_queue,
};
use crate::services::message::Message;
use crate::view::view;
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::{execute, terminal::EnterAlternateScreen};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::interval;
use vac_foundation::models::integrations::openai::ToolCallResultStatus;
use vac_foundation::task_manager::TaskManagerHandle;
use vac_foundation::utils::strip_tool_name;
use vac_provider_core::Model;

use crate::app::{ApprovalSettingsPersistenceTrigger, ToolCallStatus};
use crate::terminal::TerminalGuard;

// Rulebook config struct (re-defined here to avoid circular dependency)
#[derive(Clone, Debug)]
pub struct RulebookConfig {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub include_tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_tui(
    mut input_rx: Receiver<InputEvent>,
    output_tx: Sender<OutputEvent>,
    cancel_tx: Option<tokio::sync::broadcast::Sender<()>>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    latest_version: Option<String>,
    redact_secrets: bool,
    privacy_mode: bool,
    is_git_repo: bool,
    auto_approve_tools: Option<&Vec<String>>,
    allowed_tools: Option<&Vec<String>>,
    current_profile_name: String,
    rulebook_config: Option<RulebookConfig>,
    model: Model,
    editor_command: Option<String>,
    auth_display_info: (Option<String>, Option<String>, Option<String>),
    init_prompt_content: Option<String>,
    send_init_prompt_on_start: bool,
    recent_models: Vec<String>,
    banner_message: Option<BannerMessage>,
    task_manager_handle: Arc<TaskManagerHandle>,
) -> io::Result<()> {
    let _guard = TerminalGuard;

    crossterm::terminal::enable_raw_mode()?;

    // Detect terminal for adaptive colors (but always enable mouse capture)
    #[cfg(unix)]
    {
        let _terminal_info = crate::services::detect_term::detect_terminal();
    }

    execute!(
        std::io::stdout(),
        EnterAlternateScreen,
        EnableBracketedPaste,
        EnableMouseCapture
    )?;

    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let term_size = terminal.size()?;

    // Create internal channel for event handling (needed for error reporting during initialization)
    let (internal_tx, mut internal_rx) = tokio::sync::mpsc::channel::<InputEvent>(100);

    // Get board_agent_id from environment variable
    let board_agent_id = std::env::var("AGENT_BOARD_AGENT_ID").ok();

    let mut state = AppState::new(AppStateOptions {
        latest_version,
        redact_secrets,
        privacy_mode,
        is_git_repo,
        auto_approve_tools,
        allowed_tools,
        input_tx: Some(internal_tx.clone()),
        model,
        editor_command,
        auth_display_info,
        board_agent_id,
        init_prompt_content,
        recent_models,
        task_manager_handle: Some(task_manager_handle),
    });

    state.banner_state.message = banner_message;

    // Mouse capture is always enabled
    state.terminal_ui_state.mouse_capture_enabled = true;

    // Set initial terminal size
    state.terminal_ui_state.terminal_size = ratatui::layout::Size {
        width: term_size.width,
        height: term_size.height,
    };

    // Pre-initialize the gitleaks config for secret redaction
    // This compiles all regex patterns upfront so first paste is fast
    tokio::spawn(async move {
        vac_foundation::secrets::initialize_gitleaks_config(privacy_mode);
    });

    // Set the current profile name and rulebook config
    state.profile_switcher_state.current_profile_name = current_profile_name;
    state.rulebook_switcher_state.rulebook_config = rulebook_config;

    // Add welcome messages after state is created
    let welcome_msg = crate::services::helper_block::welcome_messages(
        state.configuration_state.latest_version.clone(),
        &state,
    );
    state.messages_scrolling_state.messages.extend(welcome_msg);

    // Trigger initial board tasks refresh if agent ID is configured
    if state.side_panel_state.board_agent_id.is_some() {
        let _ = internal_tx.try_send(InputEvent::RefreshBoardTasks);
    }

    // When started via `vac init`, add init prompt as user message and send to backend
    if send_init_prompt_on_start
        && let Some(prompt) = state.configuration_state.init_prompt_content.clone()
        && !prompt.trim().is_empty()
    {
        state
            .messages_scrolling_state
            .messages
            .push(Message::user(prompt.clone(), None));
        crate::services::message::invalidate_message_lines_cache(&mut state);
        let _ = output_tx.try_send(OutputEvent::UserMessage(prompt, None, Vec::new(), None));
    }

    let internal_tx_thread = internal_tx.clone();
    // Create atomic pause/stop flags for input thread. The stop flag is required
    // because a native Rust thread keeps the process alive after the TUI future
    // returns; relying on a failed channel send is insufficient when no further
    // terminal input arrives.
    let input_paused = Arc::new(AtomicBool::new(false));
    let input_paused_thread = input_paused.clone();
    let input_should_stop = Arc::new(AtomicBool::new(false));
    let input_should_stop_thread = input_should_stop.clone();

    // Spawn input handling thread
    // This thread reads from crossterm and converts to internal events
    // It must be pausable when we yield terminal control to external programs (like nano/vim)
    std::thread::spawn(move || {
        loop {
            if input_should_stop_thread.load(Ordering::Relaxed) {
                break;
            }

            // Check if we should pause input reading
            if input_paused_thread.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }

            // Use poll with timeout instead of blocking read to allow checking pause flag
            if let Ok(true) = crossterm::event::poll(Duration::from_millis(50))
                && let Ok(event) = crossterm::event::read()
                && let Some(event) = crate::event::map_crossterm_event_to_input_event(event)
                && internal_tx_thread.blocking_send(event).is_err()
            {
                break;
            }
        }
    });

    let shell_event_tx = internal_tx.clone();

    let mut spinner_interval = interval(Duration::from_millis(100));

    // Main async update/view loop
    terminal.draw(|f| view(f, &mut state))?;
    let mut should_quit = false;
    let mut prefer_backend_after_internal = false;

    loop {
        expire_quit_intent(&mut state);

        if prefer_backend_after_internal {
            tokio::select! {
                biased;

                event = input_rx.recv() => {
                    prefer_backend_after_internal = false;
                    if dispatch_inbound_backend_event(
                        &mut terminal,
                        &mut state,
                        event,
                        &internal_tx,
                        &output_tx,
                        cancel_tx.clone(),
                        &shell_event_tx,
                        input_paused.as_ref(),
                    ).await? {
                        should_quit = true;
                    }
                }
                event = internal_rx.recv() => {
                    prefer_backend_after_internal = true;
                    if dispatch_inbound_internal_input_event(
                        &mut terminal,
                        &mut state,
                        event,
                        &mut internal_rx,
                        &internal_tx,
                        &output_tx,
                        cancel_tx.clone(),
                        &shell_event_tx,
                        input_paused.as_ref(),
                    )? {
                        should_quit = true;
                    }
                }
                _ = spinner_interval.tick() => {
                    handle_spinner_tick(&mut state);
                    terminal.draw(|f| view(f, &mut state))?;
                }
            }
        } else {
            tokio::select! {
                biased;

                event = internal_rx.recv() => {
                    prefer_backend_after_internal = true;
                    if dispatch_inbound_internal_input_event(
                        &mut terminal,
                        &mut state,
                        event,
                        &mut internal_rx,
                        &internal_tx,
                        &output_tx,
                        cancel_tx.clone(),
                        &shell_event_tx,
                        input_paused.as_ref(),
                    )? {
                        should_quit = true;
                    }
                }
                event = input_rx.recv() => {
                    if dispatch_inbound_backend_event(
                        &mut terminal,
                        &mut state,
                        event,
                        &internal_tx,
                        &output_tx,
                        cancel_tx.clone(),
                        &shell_event_tx,
                        input_paused.as_ref(),
                    ).await? {
                        should_quit = true;
                    }
                }
                _ = spinner_interval.tick() => {
                    handle_spinner_tick(&mut state);
                    terminal.draw(|f| view(f, &mut state))?;
                }
            }
        }

        if should_quit {
            break;
        }
        if state.shell_popup_state.needs_terminal_clear {
            state.shell_popup_state.needs_terminal_clear = false;
            emergency_clear_and_redraw(&mut terminal, &mut state)?;
        }
        state.poll_file_search_results();
        state.update_session_empty_status();
        terminal.draw(|f| view(f, &mut state))?;
    }

    input_should_stop.store(true, Ordering::Relaxed);
    let _ = shutdown_tx.send(());
    crossterm::terminal::disable_raw_mode()?;
    execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        DisableBracketedPaste,
        DisableMouseCapture
    )?;
    Ok(())
}

#[derive(Clone, Copy)]
struct MessageAreaDimensions {
    width: usize,
    height: usize,
    term_size: ratatui::layout::Size,
}

const MAX_INTERNAL_INPUT_DRAIN_PER_FRAME: usize = 64;

fn expire_quit_intent(state: &mut AppState) {
    if state.quit_intent_state.ctrl_c_pressed_once
        && let Some(timer) = state.quit_intent_state.ctrl_c_timer
        && std::time::Instant::now() > timer
    {
        state.quit_intent_state.ctrl_c_pressed_once = false;
        state.quit_intent_state.ctrl_c_timer = None;
    }
}

fn is_shell_event(event: &InputEvent) -> bool {
    matches!(
        event,
        InputEvent::ShellOutput(_)
            | InputEvent::ShellError(_)
            | InputEvent::ShellWaitingForInput
            | InputEvent::ShellCompleted(_)
            | InputEvent::ShellClear
    )
}

fn handle_quit_event(state: &mut AppState) -> bool {
    if state
        .configuration_state
        .auto_approve_manager
        .has_unsaved_changes()
        && !state.approval_settings_persistence_state.is_visible
    {
        state.approval_settings_persistence_state.is_visible = true;
        state.approval_settings_persistence_state.selected = 0;
        state.approval_settings_persistence_state.trigger =
            ApprovalSettingsPersistenceTrigger::Quit;
        false
    } else {
        true
    }
}

fn compute_main_term_rect(
    state: &AppState,
    term_size: ratatui::layout::Size,
) -> ratatui::layout::Rect {
    let main_area_width = if state.side_panel_state.is_shown {
        term_size.width.saturating_sub(32 + 1)
    } else {
        term_size.width
    };
    ratatui::layout::Rect::new(0, 0, main_area_width, term_size.height)
}

fn compute_message_area(
    state: &AppState,
    term_size: ratatui::layout::Size,
) -> MessageAreaDimensions {
    let term_rect = compute_main_term_rect(state, term_size);
    let input_height: u16 = 3;
    let margin_height: u16 = 2;
    let dropdown_showing = state.input_state.show_helper_dropdown
        && ((!state.input_state.filtered_helpers.is_empty() && state.input().starts_with('/'))
            || !state.input_state.filtered_files.is_empty());
    let dropdown_height = if dropdown_showing {
        state.input_state.filtered_helpers.len() as u16
    } else {
        0
    };
    let hint_height = if dropdown_showing { 0 } else { margin_height };
    let banner_h = crate::services::banner::banner_height(state);
    let outer_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(banner_h),
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(input_height),
            ratatui::layout::Constraint::Length(dropdown_height),
            ratatui::layout::Constraint::Length(hint_height),
        ])
        .split(term_rect);

    MessageAreaDimensions {
        width: outer_chunks[1].width.saturating_sub(2) as usize,
        height: outer_chunks[1].height as usize,
        term_size,
    }
}

fn compute_tool_confirmation_message_area(
    state: &AppState,
    term_size: ratatui::layout::Size,
) -> MessageAreaDimensions {
    let term_rect = compute_main_term_rect(state, term_size);
    let margin_height: u16 = 2;
    let dropdown_showing = state.input_state.show_helper_dropdown
        && ((!state.input_state.filtered_helpers.is_empty() && state.input().starts_with('/'))
            || !state.input_state.filtered_files.is_empty());
    let hint_height = if dropdown_showing { 0 } else { margin_height };
    let approval_bar_height = state
        .dialog_approval_state
        .approval_bar
        .calculate_height(term_rect.width)
        .max(7);
    let banner_h = crate::services::banner::banner_height(state);
    let outer_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(banner_h),
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(0),
            ratatui::layout::Constraint::Length(approval_bar_height),
            ratatui::layout::Constraint::Length(0),
            ratatui::layout::Constraint::Length(0),
            ratatui::layout::Constraint::Length(hint_height),
        ])
        .split(term_rect);

    MessageAreaDimensions {
        width: outer_chunks[1].width.saturating_sub(2) as usize,
        height: outer_chunks[1].height as usize,
        term_size,
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_update_event(
    state: &mut AppState,
    event: InputEvent,
    area: MessageAreaDimensions,
    internal_tx: &Sender<InputEvent>,
    output_tx: &Sender<OutputEvent>,
    cancel_tx: Option<tokio::sync::broadcast::Sender<()>>,
    shell_event_tx: &Sender<InputEvent>,
) {
    crate::services::update::update(
        state,
        event,
        area.height,
        area.width,
        internal_tx,
        output_tx,
        cancel_tx,
        shell_event_tx,
        area.term_size,
    );
}

#[allow(clippy::too_many_arguments)]
fn dispatch_internal_input_event<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    event: InputEvent,
    internal_rx: &mut Receiver<InputEvent>,
    internal_tx: &Sender<InputEvent>,
    output_tx: &Sender<OutputEvent>,
    cancel_tx: Option<tokio::sync::broadcast::Sender<()>>,
    shell_event_tx: &Sender<InputEvent>,
    input_paused: &AtomicBool,
) -> io::Result<bool> {
    match event {
        InputEvent::ToggleMouseCapture => {
            #[cfg(unix)]
            toggle_mouse_capture_with_redraw(terminal, state)?;
            return Ok(false);
        }
        InputEvent::EmergencyClearTerminal => {
            emergency_clear_and_redraw(terminal, state)?;
            return Ok(false);
        }
        InputEvent::Quit => return Ok(handle_quit_event(state)),
        other => {
            let term_size = terminal.size()?;
            let area = compute_message_area(state, term_size);
            if matches!(&other, InputEvent::ScrollUp | InputEvent::ScrollDown) {
                if let Some(deferred_event) = dispatch_scroll_batch(
                    state,
                    other,
                    area,
                    internal_rx,
                    internal_tx,
                    output_tx,
                    cancel_tx.clone(),
                    shell_event_tx,
                ) && dispatch_internal_input_event(
                    terminal,
                    state,
                    deferred_event,
                    internal_rx,
                    internal_tx,
                    output_tx,
                    cancel_tx,
                    shell_event_tx,
                    input_paused,
                )? {
                    return Ok(true);
                }
            } else {
                dispatch_update_event(
                    state,
                    other,
                    area,
                    internal_tx,
                    output_tx,
                    cancel_tx,
                    shell_event_tx,
                );
            }
        }
    }

    state.poll_file_search_results();
    handle_pending_editor_open(terminal, state, input_paused)?;
    state.update_session_empty_status();
    Ok(false)
}

#[allow(clippy::too_many_arguments)]
fn dispatch_inbound_internal_input_event<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    event: Option<InputEvent>,
    internal_rx: &mut Receiver<InputEvent>,
    internal_tx: &Sender<InputEvent>,
    output_tx: &Sender<OutputEvent>,
    cancel_tx: Option<tokio::sync::broadcast::Sender<()>>,
    shell_event_tx: &Sender<InputEvent>,
    input_paused: &AtomicBool,
) -> io::Result<bool> {
    let Some(event) = event else {
        return Ok(true);
    };

    dispatch_internal_input_event(
        terminal,
        state,
        event,
        internal_rx,
        internal_tx,
        output_tx,
        cancel_tx,
        shell_event_tx,
        input_paused,
    )
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_inbound_backend_event<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    event: Option<InputEvent>,
    internal_tx: &Sender<InputEvent>,
    output_tx: &Sender<OutputEvent>,
    cancel_tx: Option<tokio::sync::broadcast::Sender<()>>,
    shell_event_tx: &Sender<InputEvent>,
    input_paused: &AtomicBool,
) -> io::Result<bool> {
    let Some(event) = event else {
        return Ok(true);
    };

    if is_shell_event(&event) {
        let _ = shell_event_tx.send(event).await;
        return Ok(false);
    }

    dispatch_backend_event(
        terminal,
        state,
        event,
        internal_tx,
        output_tx,
        cancel_tx,
        shell_event_tx,
        input_paused,
    )
}

#[allow(clippy::too_many_arguments)]
fn dispatch_scroll_batch(
    state: &mut AppState,
    initial_event: InputEvent,
    area: MessageAreaDimensions,
    internal_rx: &mut Receiver<InputEvent>,
    internal_tx: &Sender<InputEvent>,
    output_tx: &Sender<OutputEvent>,
    cancel_tx: Option<tokio::sync::broadcast::Sender<()>>,
    shell_event_tx: &Sender<InputEvent>,
) -> Option<InputEvent> {
    let mut pending_scroll_up: i32 = 0;
    let mut pending_scroll_down: i32 = 0;

    match &initial_event {
        InputEvent::ScrollUp => pending_scroll_up += 1,
        InputEvent::ScrollDown => pending_scroll_down += 1,
        _ => return Some(initial_event),
    }

    let mut deferred_event = None;
    for _ in 0..MAX_INTERNAL_INPUT_DRAIN_PER_FRAME.saturating_sub(1) {
        match internal_rx.try_recv() {
            Ok(InputEvent::ScrollUp) => pending_scroll_up += 1,
            Ok(InputEvent::ScrollDown) => pending_scroll_down += 1,
            Ok(other) => {
                deferred_event = Some(other);
                break;
            }
            Err(_) => break,
        }
    }

    let net_scroll = pending_scroll_down - pending_scroll_up;
    for _ in 0..net_scroll.unsigned_abs() {
        let event = if net_scroll > 0 {
            InputEvent::ScrollDown
        } else {
            InputEvent::ScrollUp
        };
        dispatch_update_event(
            state,
            event,
            area,
            internal_tx,
            output_tx,
            cancel_tx.clone(),
            shell_event_tx,
        );
    }

    deferred_event
}

#[allow(clippy::too_many_arguments)]
fn dispatch_backend_event<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    event: InputEvent,
    internal_tx: &Sender<InputEvent>,
    output_tx: &Sender<OutputEvent>,
    cancel_tx: Option<tokio::sync::broadcast::Sender<()>>,
    shell_event_tx: &Sender<InputEvent>,
    input_paused: &AtomicBool,
) -> io::Result<bool> {
    if matches!(&event, InputEvent::EmergencyClearTerminal) {
        emergency_clear_and_redraw(terminal, state)?;
        return Ok(false);
    }

    if let InputEvent::RunToolCall(tool_call) = &event {
        let term_size = terminal.size()?;
        let area = compute_tool_confirmation_message_area(state, term_size);
        dispatch_update_event(
            state,
            InputEvent::ShowConfirmationDialog(tool_call.clone()),
            area,
            internal_tx,
            output_tx,
            cancel_tx,
            shell_event_tx,
        );
        state.poll_file_search_results();
        terminal.draw(|f| view(f, state))?;
        return Ok(false);
    }

    if let InputEvent::ToolResult(tool_call_result) = &event {
        handle_backend_tool_result(state, tool_call_result, internal_tx);
    }

    if matches!(&event, InputEvent::ToggleMouseCapture) {
        #[cfg(unix)]
        toggle_mouse_capture_with_redraw(terminal, state)?;
        return Ok(false);
    }

    if matches!(&event, InputEvent::Quit) {
        return Ok(handle_quit_event(state));
    }

    let term_size = terminal.size()?;
    let area = compute_message_area(state, term_size);
    dispatch_update_event(
        state,
        event,
        area,
        internal_tx,
        output_tx,
        cancel_tx,
        shell_event_tx,
    );
    state.poll_file_search_results();
    handle_pending_editor_open(terminal, state, input_paused)?;
    state.update_session_empty_status();
    Ok(false)
}

fn handle_backend_tool_result(
    state: &mut AppState,
    tool_call_result: &vac_foundation::models::integrations::openai::ToolCallResult,
    internal_tx: &Sender<InputEvent>,
) {
    clear_streaming_tool_results(state);

    state.tool_call_state.cancel_requested = false;

    if let Ok(tool_call_uuid) = uuid::Uuid::parse_str(&tool_call_result.call.id) {
        state
            .messages_scrolling_state
            .messages
            .retain(|m| m.id != tool_call_uuid);
    }

    state
        .session_tool_calls_state
        .session_tool_calls_queue
        .insert(tool_call_result.call.id.clone(), ToolCallStatus::Executed);
    update_session_tool_calls_queue(state, tool_call_result);
    let tool_name = strip_tool_name(&tool_call_result.call.function.name);

    let is_fg_cmd = matches!(tool_name, "run_command" | "run_remote_command");
    if tool_call_result.status == ToolCallResultStatus::Cancelled && is_fg_cmd {
        state.tool_call_state.latest_tool_call = Some(tool_call_result.call.clone());
    }

    let is_cancelled = tool_call_result.status == ToolCallResultStatus::Cancelled;
    let is_error = tool_call_result.status == ToolCallResultStatus::Error;

    if (is_cancelled || is_error) && !is_fg_cmd {
        state
            .messages_scrolling_state
            .messages
            .push(Message::render_result_border_block(
                tool_call_result.clone(),
            ));
        state
            .messages_scrolling_state
            .messages
            .push(Message::render_full_content_message(
                tool_call_result.clone(),
            ));
    } else {
        match tool_name {
            "str_replace" | "create" => {
                state
                    .messages_scrolling_state
                    .messages
                    .push(Message::render_result_border_block(
                        tool_call_result.clone(),
                    ));
                state
                    .messages_scrolling_state
                    .messages
                    .push(Message::render_full_content_message(
                        tool_call_result.clone(),
                    ));
            }
            "run_command_task" | "run_remote_command_task" => {
                state
                    .messages_scrolling_state
                    .messages
                    .push(Message::render_result_border_block(
                        tool_call_result.clone(),
                    ));
                state
                    .messages_scrolling_state
                    .messages
                    .push(Message::render_full_content_message(
                        tool_call_result.clone(),
                    ));
            }
            "run_command" | "run_remote_command" => {
                let command = crate::services::handlers::shell::extract_command_from_tool_call(
                    &tool_call_result.call,
                )
                .unwrap_or_else(|_| "command".to_string());
                let run_state = if is_error {
                    crate::services::bash_block::RunCommandState::Error
                } else if is_cancelled {
                    crate::services::bash_block::RunCommandState::Cancelled
                } else {
                    crate::services::bash_block::RunCommandState::Completed
                };

                let run_cmd_msg = Message::render_run_command_block(
                    command,
                    Some(tool_call_result.result.clone()),
                    run_state,
                    None,
                );
                let popup_msg = Message::render_full_content_message(tool_call_result.clone());

                if is_cancelled && state.shell_popup_state.is_visible {
                    if let Some(shell_msg_id) =
                        state.shell_session_state.interactive_shell_message_id
                    {
                        if let Some(pos) = state
                            .messages_scrolling_state
                            .messages
                            .iter()
                            .position(|m| m.id == shell_msg_id)
                        {
                            state
                                .messages_scrolling_state
                                .messages
                                .insert(pos, popup_msg);
                            state
                                .messages_scrolling_state
                                .messages
                                .insert(pos, run_cmd_msg);
                        } else {
                            state.messages_scrolling_state.messages.push(run_cmd_msg);
                            state.messages_scrolling_state.messages.push(popup_msg);
                        }
                    } else {
                        state.messages_scrolling_state.messages.push(run_cmd_msg);
                        state.messages_scrolling_state.messages.push(popup_msg);
                    }
                } else {
                    state.messages_scrolling_state.messages.push(run_cmd_msg);
                    state.messages_scrolling_state.messages.push(popup_msg);
                }
            }
            "read" | "view" | "read_file" => {
                let (file_path, grep, glob) =
                    crate::services::handlers::tool::extract_view_params_from_tool_call(
                        &tool_call_result.call,
                    );
                let file_path = file_path.unwrap_or_else(|| "file".to_string());
                let total_lines = tool_call_result.result.lines().count();
                state
                    .messages_scrolling_state
                    .messages
                    .push(Message::render_view_file_block(
                        file_path.clone(),
                        total_lines,
                        grep.clone(),
                        glob.clone(),
                    ));
                state.messages_scrolling_state.messages.push(
                    Message::render_view_file_block_popup(file_path, total_lines, grep, glob),
                );
            }
            _ => {
                state.messages_scrolling_state.messages.push(
                    Message::render_collapsed_command_message(tool_call_result.clone()),
                );
                state
                    .messages_scrolling_state
                    .messages
                    .push(Message::render_full_content_message(
                        tool_call_result.clone(),
                    ));
            }
        }

        if !is_cancelled && !is_error {
            handle_tool_result(state, tool_call_result.clone());
        }
    }

    crate::services::message::invalidate_message_lines_cache(state);
    state.messages_scrolling_state.stay_at_bottom = true;

    let _ = internal_tx.try_send(InputEvent::RefreshBoardTasks);
}

fn handle_spinner_tick(state: &mut AppState) {
    expire_quit_intent(state);
    state.loading_state.spinner_frame = state.loading_state.spinner_frame.wrapping_add(1);
    crate::services::shell_popup::update_cursor_blink(state);
    state.poll_file_search_results();

    if let Some((old_status, new_status)) = state.poll_plan_file() {
        use crate::services::plan::PlanStatus;
        match new_status {
            PlanStatus::PendingReview => {
                if !state.plan_mode_state.review_auto_opened {
                    state.plan_mode_state.review_auto_opened = true;
                    crate::services::plan_review::open_plan_review(state);
                    crate::services::helper_block::push_styled_message(
                        state,
                        " Plan ready for review. Opening reviewer... (ctrl+p to toggle)",
                        ThemeColors::cyan(),
                        ">> ",
                        ThemeColors::cyan(),
                    );
                }
            }
            PlanStatus::Approved => {
                let _ = old_status;
            }
            PlanStatus::Drafting => {
                state.plan_mode_state.review_auto_opened = false;
            }
        }
    }

    crate::services::handlers::tick_selection_auto_scroll(state);
}

fn handle_pending_editor_open<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    input_paused: &AtomicBool,
) -> io::Result<()> {
    let Some(file_path) = state.side_panel_state.pending_editor_open.take() else {
        return Ok(());
    };

    input_paused.store(true, Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(10));

    let was_mouse_capture_enabled = state.terminal_ui_state.mouse_capture_enabled;
    if was_mouse_capture_enabled {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        state.terminal_ui_state.mouse_capture_enabled = false;
    }

    if let Err(error) = crate::services::editor::open_in_editor(
        terminal,
        &state.side_panel_state.editor_command,
        &file_path,
        None,
    ) {
        state.messages_scrolling_state.messages.push(Message::info(
            format!("Failed to open editor: {}", error),
            Some(ratatui::style::Style::default().fg(ThemeColors::red())),
        ));
    }

    if was_mouse_capture_enabled {
        let _ = execute!(std::io::stdout(), EnableMouseCapture);
        state.terminal_ui_state.mouse_capture_enabled = true;
    }

    input_paused.store(false, Ordering::Relaxed);
    Ok(())
}

pub fn emergency_clear_and_redraw<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
) -> io::Result<()> {
    use crossterm::{
        cursor::MoveTo,
        execute,
        terminal::{Clear, ClearType},
    };

    // Nuclear option - clear everything including scrollback
    execute!(
        std::io::stdout(),
        Clear(ClearType::All),
        Clear(ClearType::Purge),
        MoveTo(0, 0)
    )?;

    // Force a complete redraw of the TUI
    terminal.clear()?;
    terminal.draw(|f| view(f, state))?;

    Ok(())
}

fn toggle_mouse_capture_with_redraw<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
) -> io::Result<()> {
    crate::toggle_mouse_capture(state)?;
    emergency_clear_and_redraw(terminal, state)?;
    Ok(())
}
