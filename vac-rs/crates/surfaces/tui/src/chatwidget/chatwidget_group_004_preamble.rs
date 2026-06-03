fn user_message_preview_text(
    message: &UserMessage,
    history_record: Option<&UserMessageHistoryRecord>,
) -> String {
    match history_record {
        Some(UserMessageHistoryRecord::Override(history)) if !history.text.is_empty() => {
            history.text.clone()
        }
        Some(UserMessageHistoryRecord::Override(_))
        | Some(UserMessageHistoryRecord::UserMessageText)
        | None => message.text.clone(),
    }
}

fn user_message_display_for_history(
    message: UserMessage,
    history_record: &UserMessageHistoryRecord,
) -> UserMessageDisplay {
    let message = user_message_for_restore(message, history_record);
    ChatWidget::user_message_display_from_parts(
        message.text,
        message.text_elements,
        message
            .local_images
            .into_iter()
            .map(|image| image.path)
            .collect(),
        message.remote_image_urls,
    )
}

fn merge_user_messages_with_history_record(
    messages: Vec<(UserMessage, UserMessageHistoryRecord)>,
) -> (UserMessage, UserMessageHistoryRecord) {
    let messages = remap_user_messages_with_history_records(messages);
    let history_record = if messages
        .iter()
        .all(|(_, record)| *record == UserMessageHistoryRecord::UserMessageText)
    {
        UserMessageHistoryRecord::UserMessageText
    } else {
        let mut history_text = String::new();
        let mut history_text_elements = Vec::new();
        let mut history_segment_count = 0usize;
        let mut append_history_segment = |text: &str, text_elements: Vec<TextElement>| {
            if history_segment_count > 0 {
                history_text.push('\n');
            }
            append_text_with_rebased_elements(
                &mut history_text,
                &mut history_text_elements,
                text,
                text_elements,
            );
            history_segment_count += 1;
        };
        for (message, record) in &messages {
            match record {
                UserMessageHistoryRecord::Override(history) if !history.text.is_empty() => {
                    append_history_segment(&history.text, history.text_elements.clone());
                }
                UserMessageHistoryRecord::Override(_) if message.text.is_empty() => {}
                UserMessageHistoryRecord::Override(_)
                | UserMessageHistoryRecord::UserMessageText => {
                    append_history_segment(&message.text, message.text_elements.clone());
                }
            }
        }
        UserMessageHistoryRecord::Override(UserMessageHistoryOverride {
            text: history_text,
            text_elements: history_text_elements,
        })
    };
    (
        merge_remapped_user_messages(messages.into_iter().map(|(message, _)| message)),
        history_record,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReplayKind {
    ResumeInitialMessages,
    ThreadSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionConfiguredDisplay {
    Normal,
    /// Apply session state without emitting the session info cell.
    Quiet,
    SideConversation,
}

/// Scope used to keep Plan-mode nudge dismissal local to one conversation context.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PlanModeNudgeScope {
    /// Drafts entered before the server has assigned a thread id.
    NewThread,
    /// Drafts associated with one configured thread.
    Thread(ThreadId),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TurnAbortReason {
    Interrupted,
    BudgetLimited,
}

/// Returns whether `text` contains the standalone word `plan`.
///
/// This intentionally mirrors the App suggestion heuristic instead of trying to infer broader
/// planning intent from substrings such as `planning`. Slash and shell drafts still match here so
/// callers can keep lexical matching separate from presentation policy.
fn contains_plan_keyword(text: &str) -> bool {
    text.split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .any(|word| word.eq_ignore_ascii_case("plan"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThreadItemRenderSource {
    Live,
    Replay(ReplayKind),
}

impl ThreadItemRenderSource {
    fn is_replay(self) -> bool {
        matches!(self, Self::Replay(_))
    }

    fn replay_kind(self) -> Option<ReplayKind> {
        match self {
            Self::Live => None,
            Self::Replay(replay_kind) => Some(replay_kind),
        }
    }
}

fn exec_approval_request_from_params(
    params: CommandExecutionRequestApprovalParams,
    fallback_cwd: &AbsolutePathBuf,
) -> ExecApprovalRequestEvent {
    let additional_permissions: Option<crate::session_protocol::AdditionalPermissionProfile> =
        params.additional_permissions.map(Into::into);
    ExecApprovalRequestEvent {
        call_id: params.item_id,
        command: params
            .command
            .as_deref()
            .map(split_command_string)
            .unwrap_or_default(),
        cwd: params.cwd.unwrap_or_else(|| fallback_cwd.clone()),
        reason: params.reason,
        network_approval_context: params
            .network_approval_context
            .map(crate::app_server_approval_conversions::network_approval_context_from_app_server),
        additional_permissions,
        turn_id: params.turn_id,
        approval_id: params.approval_id,
        proposed_execpolicy_amendment: params.proposed_execpolicy_amendment,
        proposed_network_policy_amendments: params.proposed_network_policy_amendments,
        available_decisions: params.available_decisions,
    }
}

fn patch_approval_request_from_params(
    params: FileChangeRequestApprovalParams,
) -> ApplyPatchApprovalRequestEvent {
    ApplyPatchApprovalRequestEvent {
        call_id: params.item_id,
        turn_id: params.turn_id,
        changes: HashMap::new(),
        reason: params.reason,
        grant_root: params.grant_root,
    }
}

fn request_permissions_from_params(
    params: crate::session_protocol::PermissionsRequestApprovalParams,
) -> RequestPermissionsEvent {
    RequestPermissionsEvent {
        turn_id: params.turn_id,
        call_id: params.item_id,
        reason: params.reason,
        permissions: params.permissions.into(),
        cwd: Some(params.cwd),
    }
}

fn token_usage_info_from_app_server(token_usage: ThreadTokenUsage) -> TokenUsageInfo {
    TokenUsageInfo {
        total_token_usage: TokenUsage {
            total_tokens: token_usage.total.total_tokens,
            input_tokens: token_usage.total.input_tokens,
            cached_input_tokens: token_usage.total.cached_input_tokens,
            output_tokens: token_usage.total.output_tokens,
            reasoning_output_tokens: token_usage.total.reasoning_output_tokens,
        },
        last_token_usage: TokenUsage {
            total_tokens: token_usage.last.total_tokens,
            input_tokens: token_usage.last.input_tokens,
            cached_input_tokens: token_usage.last.cached_input_tokens,
            output_tokens: token_usage.last.output_tokens,
            reasoning_output_tokens: token_usage.last.reasoning_output_tokens,
        },
        model_context_window: token_usage.model_context_window,
    }
}

async fn fetch_kilo_model_presets(
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<ModelPreset>, String> {
    let models_url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut request = client.get(models_url.clone());
    if let Some(api_key) = api_key.as_ref() {
        request = request.bearer_auth(api_key);
    }

    let response = request
        .send()
        .await
        .map_err(|err| format!("failed to request {models_url}: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{models_url} returned HTTP {status}"));
    }
    let payload: KiloModelsResponse = response
        .json()
        .await
        .map_err(|err| format!("failed to parse Kilo model list: {err}"))?;

    let mut models: Vec<ModelPreset> = payload
        .data
        .into_iter()
        .filter_map(|entry| {
            let id = entry.id.trim().to_string();
            (!id.is_empty()).then(|| kilo_model_preset(id, "Kilo Gateway model"))
        })
        .collect();
    models.sort_by(|a, b| a.model.cmp(&b.model));
    models.dedup_by(|a, b| a.model == b.model);
    Ok(models)
}

fn kilo_fallback_model_presets() -> Vec<ModelPreset> {
    [
        "anthropic/claude-sonnet-4.5",
        "anthropic/claude-opus-4.1",
        "openai/gpt-5.1",
        "openai/gpt-5.1-mini",
        "google/gemini-2.5-pro",
        "google/gemini-2.5-flash",
    ]
    .into_iter()
    .map(|model| kilo_model_preset(model, "Common Kilo Gateway model"))
    .collect()
}

fn kilo_model_preset(model: impl Into<String>, description: impl Into<String>) -> ModelPreset {
    let model = model.into();
    ModelPreset {
        id: model.clone(),
        model: model.clone(),
        display_name: model,
        description: description.into(),
        default_reasoning_effort: ReasoningEffortConfig::None,
        supported_reasoning_efforts: vec![ReasoningEffortPreset {
            effort: ReasoningEffortConfig::None,
            description: "Provider default reasoning".to_string(),
        }],
        supports_personality: false,
        additional_speed_tiers: Vec::new(),
        is_default: false,
        upgrade: None,
        show_in_picker: true,
        availability_nux: None,
        supported_in_api: true,
        input_modalities: vec![InputModality::Text, InputModality::Image],
    }
}
