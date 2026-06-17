//! Conversion between unified types and Anthropic types

use super::types::{
    AnthropicAuth, AnthropicCacheControl, AnthropicConfig, AnthropicContent, AnthropicMessage,
    AnthropicMessageContent, AnthropicRequest, AnthropicResponse, AnthropicSource,
    AnthropicSystemBlock, AnthropicSystemContent, AnthropicThinkingConfig as AnthropicThinking,
    CLAUDE_CODE_SYSTEM_PREFIX, infer_max_tokens,
};
use crate::error::{Error, Result};
use crate::types::{
    CacheContext, CacheControlValidator, CacheWarning, CacheWarningType, ContentPart, FinishReason,
    FinishReasonKind, GenerateRequest, GenerateResponse, InputTokenDetails, Message,
    OutputTokenDetails, ResponseContent, Role, Usage,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// Check whether the target model belongs to the Opus 4.7 (or later) family.
///
/// Opus 4.7 dropped `temperature`, `top_p`, `top_k`, and `thinking.budget_tokens` from
/// the Messages API. This helper centralizes detection so the conversion layer can shape
/// requests to the subset of parameters those models still accept. Case-insensitive prefix
/// match, mirroring `is_reasoning_model` in `providers/openai/convert.rs`.
///
/// See: https://platform.claude.com/docs/en/about-claude/models/whats-new-claude-4-7
fn is_opus_4_7_or_later(model_id: &str) -> bool {
    let id = model_id.to_lowercase();
    id.starts_with("claude-opus-4-7")
}

/// Result of converting a request to Anthropic format
pub struct AnthropicConversionResult {
    /// The converted request
    pub request: AnthropicRequest,
    /// Warnings generated during conversion (e.g., cache validation)
    pub warnings: Vec<CacheWarning>,
    /// Whether any cache control was used (to determine if beta header is needed)
    pub has_cache_control: bool,
}

/// Convert unified request to Anthropic request with smart caching
///
/// This function applies the caching strategy from the request options,
/// falling back to the provider's default strategy if not specified.
pub fn to_anthropic_request(
    req: &GenerateRequest,
    config: &AnthropicConfig,
    stream: bool,
) -> Result<AnthropicConversionResult> {
    let mut validator = CacheControlValidator::new();

    // Determine the effective caching strategy:
    // 1. Request-level strategy takes precedence
    // 2. Fall back to provider default
    let cache_strategy = req
        .options
        .cache_strategy
        .clone()
        .unwrap_or_else(|| config.default_cache_strategy.clone());

    let cache_config = cache_strategy.to_anthropic_config();

    // Check if we have tools (for cache budget calculation)
    let has_tools = req.options.tools.as_ref().is_some_and(|t| !t.is_empty());

    // Build tools with smart caching (cache last tool)
    let tools = build_tools_with_caching(
        &req.options.tools,
        &mut validator,
        cache_config
            .as_ref()
            .is_some_and(|c| c.cache_tools && has_tools),
    )?;

    // Extract and convert system messages with smart caching
    let system = build_system_content_with_caching(
        &req.messages,
        &config.auth,
        &mut validator,
        cache_config.as_ref().is_some_and(|c| c.cache_system),
    )?;

    // Calculate remaining budget for tail messages
    let tail_budget = cache_config.as_ref().map_or(0, |c| {
        let used = validator.breakpoint_count();
        let max = 4usize; // Anthropic limit
        let remaining = max.saturating_sub(used);
        c.tail_message_count.min(remaining)
    });

    // Convert non-system messages with smart tail caching
    let messages = build_messages_with_caching(&req.messages, &mut validator, tail_budget)?;

    // Determine max_tokens (required by Anthropic!)
    let max_tokens = req
        .options
        .max_tokens
        .unwrap_or_else(|| infer_max_tokens(&req.model.id));

    // Convert tool_choice to Anthropic format
    let tool_choice = req.options.tool_choice.as_ref().map(|choice| match choice {
        crate::types::ToolChoice::Auto => json!({"type": "auto"}),
        crate::types::ToolChoice::None => json!({"type": "none"}),
        crate::types::ToolChoice::Required { name } => json!({
            "type": "tool",
            "name": name
        }),
    });

    let is_opus_47 = is_opus_4_7_or_later(&req.model.id);

    let thinking = req.provider_options.as_ref().and_then(|opts| {
        if let crate::types::ProviderOptions::Anthropic(anthropic) = opts {
            anthropic.thinking.as_ref().map(|t| {
                if is_opus_47 {
                    AnthropicThinking {
                        type_: "adaptive".to_string(),
                        budget_tokens: None,
                    }
                } else {
                    AnthropicThinking {
                        type_: "enabled".to_string(),
                        budget_tokens: Some(t.budget_tokens.max(1024)),
                    }
                }
            })
        } else {
            None
        }
    });

    let has_cache_control = validator.breakpoint_count() > 0;
    let mut warnings = validator.take_warnings();

    // top_k is already None at the struct level; only cover temperature/top_p on input.
    let (temperature, top_p) = if is_opus_47 {
        if req.options.temperature.is_some() {
            warnings.push(opus_47_strip_warning("temperature"));
        }
        if req.options.top_p.is_some() {
            warnings.push(opus_47_strip_warning("top_p"));
        }
        (None, None)
    } else {
        (req.options.temperature, req.options.top_p)
    };

    if is_opus_47 && thinking.is_some() {
        warnings.push(opus_47_thinking_rewrite_warning());
    }

    Ok(AnthropicConversionResult {
        request: AnthropicRequest {
            model: req.model.id.clone(),
            messages,
            max_tokens,
            system,
            temperature,
            top_p,
            top_k: None,
            metadata: None,
            stop_sequences: req.options.stop_sequences.clone(),
            stream: if stream { Some(true) } else { None },
            thinking,
            tools,
            tool_choice,
        },
        warnings,
        has_cache_control,
    })
}

fn opus_47_strip_warning(param: &str) -> CacheWarning {
    CacheWarning::new(
        CacheWarningType::UnsupportedContext,
        format!(
            "Claude Opus 4.7 removed the `{}` sampling parameter; it was dropped from the outgoing request.",
            param
        ),
    )
}

fn opus_47_thinking_rewrite_warning() -> CacheWarning {
    CacheWarning::new(
        CacheWarningType::UnsupportedContext,
        "Claude Opus 4.7 removed `thinking.budget_tokens`; request rewritten to `thinking: {type: \"adaptive\"}`."
            .to_string(),
    )
}

/// Build system content with smart caching and OAuth handling
///
/// When `auto_cache_last` is true, the last system block gets a cache breakpoint.
/// This caches ALL system messages (Anthropic caches the full prefix up to the breakpoint).
fn build_system_content_with_caching(
    messages: &[Message],
    auth: &AnthropicAuth,
    validator: &mut CacheControlValidator,
    auto_cache_last: bool,
) -> Result<Option<AnthropicSystemContent>> {
    let system_messages: Vec<&Message> =
        messages.iter().filter(|m| m.role == Role::System).collect();

    // For OAuth, we need the Claude Code prefix
    let is_oauth = matches!(auth, AnthropicAuth::OAuth { .. });

    if system_messages.is_empty() && !is_oauth {
        return Ok(None);
    }

    // Check if any system message has explicit cache control
    let has_explicit_cache = system_messages.iter().any(|m| m.cache_control().is_some());

    // Determine if we should use blocks format
    let use_blocks = is_oauth || has_explicit_cache || auto_cache_last;

    // For OAuth, always use blocks format with Claude Code prefix
    if is_oauth {
        let mut blocks = vec![];

        // Add Claude Code prefix with 1-hour cache
        blocks.push(AnthropicSystemBlock {
            type_: "text".to_string(),
            text: CLAUDE_CODE_SYSTEM_PREFIX.to_string(),
            cache_control: Some(AnthropicCacheControl::ephemeral_with_ttl("1h")),
        });
        // Count this as a cache breakpoint
        validator.validate(
            Some(&crate::types::CacheControl::ephemeral_with_ttl("1h")),
            CacheContext::system_message(),
        );

        // Add user system messages
        let msg_count = system_messages.len();
        for (i, msg) in system_messages.iter().enumerate() {
            if let Some(text) = msg.text() {
                let is_last = i == msg_count - 1;

                // Use explicit cache or auto-cache last with 1-hour TTL
                let cache_control = msg.cache_control().cloned().or_else(|| {
                    if is_last && auto_cache_last {
                        Some(crate::types::CacheControl::ephemeral_with_ttl("1h"))
                    } else {
                        None
                    }
                });

                let validated_cache =
                    validator.validate(cache_control.as_ref(), CacheContext::system_message());

                blocks.push(AnthropicSystemBlock {
                    type_: "text".to_string(),
                    text,
                    cache_control: validated_cache.map(|c| AnthropicCacheControl::from(&c)),
                });
            }
        }

        return Ok(Some(AnthropicSystemContent::Blocks(blocks)));
    }

    // For API key auth without any caching, use simple string format
    if !use_blocks {
        let combined = system_messages
            .iter()
            .filter_map(|m| m.text())
            .collect::<Vec<_>>()
            .join("\n\n");
        return Ok(Some(AnthropicSystemContent::String(combined)));
    }

    // Complex case: caching needed, use blocks format
    let msg_count = system_messages.len();
    let blocks: Vec<AnthropicSystemBlock> = system_messages
        .iter()
        .enumerate()
        .filter_map(|(i, msg)| {
            let text = msg.text()?;
            let is_last = i == msg_count - 1;

            // Use explicit cache or auto-cache last with 1-hour TTL
            let cache_control = msg.cache_control().cloned().or_else(|| {
                if is_last && auto_cache_last {
                    Some(crate::types::CacheControl::ephemeral_with_ttl("1h"))
                } else {
                    None
                }
            });

            let validated_cache =
                validator.validate(cache_control.as_ref(), CacheContext::system_message());

            Some(AnthropicSystemBlock {
                type_: "text".to_string(),
                text,
                cache_control: validated_cache.map(|c| AnthropicCacheControl::from(&c)),
            })
        })
        .collect();

    if blocks.is_empty() {
        Ok(None)
    } else {
        Ok(Some(AnthropicSystemContent::Blocks(blocks)))
    }
}

/// Build tools with smart caching on the last tool
///
/// When `auto_cache_last` is true, the last tool gets a cache breakpoint.
/// This caches ALL tools as a group (Anthropic caches the full prefix).
fn build_tools_with_caching(
    tools: &Option<Vec<crate::types::Tool>>,
    validator: &mut CacheControlValidator,
    auto_cache_last: bool,
) -> Result<Option<Vec<serde_json::Value>>> {
    let tools = match tools {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };

    let len = tools.len();
    let converted: Vec<serde_json::Value> = tools
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            let is_last = i == len - 1;

            // Use explicit cache_control if set, otherwise auto-cache last tool with 1h TTL
            let cache_control = tool.cache_control().cloned().or_else(|| {
                if is_last && auto_cache_last {
                    Some(crate::types::CacheControl::ephemeral_with_ttl("1h"))
                } else {
                    None
                }
            });

            let validated_cache =
                validator.validate(cache_control.as_ref(), CacheContext::tool_definition());

            let mut tool_json = json!({
                "name": tool.function.name,
                "description": tool.function.description,
                "input_schema": tool.function.parameters,
            });

            if let Some(cache) = validated_cache {
                tool_json["cache_control"] = json!(AnthropicCacheControl::from(&cache));
            }

            tool_json
        })
        .collect();

    Ok(Some(converted))
}

/// Build messages with smart tail caching
///
/// Caches the last N non-system messages to maximize cache hits
/// on subsequent requests in a conversation.
///
/// Tail caching runs **last** — after all structural mutations (merging,
/// per-message sanitization, and sequence-level sanitization) are complete.
/// This guarantees cache breakpoints land on the final stable message
/// boundaries, preventing stale breakpoints from messages that get
/// inserted, removed, or re-merged by sanitization phases.
fn build_messages_with_caching(
    messages: &[Message],
    validator: &mut CacheControlValidator,
    tail_count: usize,
) -> Result<Vec<AnthropicMessage>> {
    let non_system: Vec<&Message> = messages.iter().filter(|m| m.role != Role::System).collect();

    // Phase 1: Convert each message individually (no auto-caching yet)
    let converted: Vec<AnthropicMessage> = non_system
        .iter()
        .map(|msg| to_anthropic_message_with_caching(msg, validator, false))
        .collect::<Result<Vec<_>>>()?;

    // Phase 2: Merge consecutive same-role messages
    let mut merged = merge_consecutive_messages(converted);

    // Phase 3: Sanitize individual messages to enforce per-message constraints.
    // Runs before sequence sanitization so that empty text blocks are removed
    // before tool-pairing logic inspects message content.
    for msg in &mut merged {
        sanitize_anthropic_message(msg);
    }

    // Phase 4: Enforce message-sequence-level Anthropic constraints.
    // This handles structural invariants that span multiple messages:
    // - Every tool_use must have a matching tool_result in the next user message
    // - Orphan tool_results without matching tool_use are removed
    // - Conversation must start with a user message
    // - Conversation must not end with an assistant message (unless prefill-safe)
    //
    // This phase can insert, remove, and re-merge messages, so caching
    // must run after it to avoid stale breakpoint placement.
    sanitize_message_sequence(&mut merged);

    // Phase 5: Apply tail caching to the last N messages of the *final* array.
    // Running after all mutations ensures breakpoints land on stable positions
    // and won't be shifted by later inserts/removes/re-merges.
    if tail_count > 0 {
        let len = merged.len();
        let cache_start = len.saturating_sub(tail_count);
        for msg in &mut merged[cache_start..] {
            if !is_empty_content_message(msg) {
                apply_tail_cache_to_message(msg, validator);
            }
        }
    }

    Ok(merged)
}

/// Apply ephemeral cache control to the last content block of a message.
///
/// Used for tail-caching after message merging to ensure cache breakpoints
/// land on the actual last block of each merged message.
fn apply_tail_cache_to_message(msg: &mut AnthropicMessage, validator: &mut CacheControlValidator) {
    let cache = crate::types::CacheControl::ephemeral();
    let context = if msg.role == "assistant" {
        CacheContext::assistant_message_part()
    } else {
        CacheContext::user_message_part()
    };

    let Some(validated_cache) = validator.validate(Some(&cache), context) else {
        return; // Breakpoint limit exceeded
    };

    let anthropic_cc = AnthropicCacheControl::from(&validated_cache);
    match &mut msg.content {
        AnthropicMessageContent::Blocks(blocks) => {
            if let Some(last) = blocks.last_mut() {
                set_block_cache_control(last, Some(anthropic_cc));
            }
        }
        AnthropicMessageContent::String(s) => {
            // Convert to blocks format to attach cache control
            msg.content = AnthropicMessageContent::Blocks(vec![AnthropicContent::Text {
                text: std::mem::take(s),
                cache_control: Some(anthropic_cc),
            }]);
        }
    }
}

/// Returns true if the message contains only empty text content (no cacheable substance).
///
/// Used to skip tail-caching on messages that would waste a cache breakpoint,
/// since Phase 4 would strip the `cache_control` from empty text blocks anyway.
fn is_empty_content_message(msg: &AnthropicMessage) -> bool {
    match &msg.content {
        AnthropicMessageContent::String(s) => s.is_empty(),
        AnthropicMessageContent::Blocks(blocks) => blocks
            .iter()
            .all(|b| matches!(b, AnthropicContent::Text { text, .. } if text.is_empty())),
    }
}

/// Sanitize an Anthropic message to enforce per-message API constraints.
///
/// This is the **single boundary** that fixes structural issues before the
/// message is sent to the API. All Anthropic-specific content invariants
/// are enforced here, rather than scattering guards across conversion,
/// merging, and caching phases.
///
/// Rules (validated against live API + informed by Vercel AI SDK / OpenCode):
/// - Strip empty text blocks from blocks content
///   (prevents "all messages must have non-empty content" when only empty text remains)
/// - Strip `cache_control` from any remaining empty text blocks
///   (Anthropic rejects: "cache_control cannot be set for empty text blocks")
fn sanitize_anthropic_message(msg: &mut AnthropicMessage) {
    match &mut msg.content {
        AnthropicMessageContent::Blocks(blocks) => {
            // Remove empty text blocks entirely (OpenCode pattern: filter empty text/reasoning).
            // Keep non-text blocks (tool_result, tool_use, image) and non-empty text.
            blocks.retain(
                |block| !matches!(block, AnthropicContent::Text { text, .. } if text.is_empty()),
            );

            // Safety: strip cache_control from any remaining empty text blocks
            // (e.g., if a block somehow slipped through)
            for block in blocks.iter_mut() {
                if let AnthropicContent::Text {
                    text,
                    cache_control,
                } = block
                    && text.is_empty()
                    && cache_control.is_some()
                {
                    *cache_control = None;
                }
            }
        }
        AnthropicMessageContent::String(_) => {
            // String content has no cache_control field; nothing to sanitize.
        }
    }
}

/// Enforce Anthropic message-sequence-level constraints on the complete array.
///
/// This runs as the final phase after conversion, merging, and caching.
/// It handles structural invariants that span multiple messages.
///
/// Constraints enforced (validated against live Anthropic API 2025-02):
///
/// 1. Every `tool_use` must have exactly one `tool_result` in the immediately
///    following user message (adds placeholders for missing ones)
/// 2. Orphan `tool_result` blocks (not referencing any `tool_use` in the
///    immediately preceding assistant message) are removed
/// 3. No duplicate `tool_result` blocks for the same `tool_use_id`
///    (Anthropic rejects: "each tool_use must have a single result")
/// 4. No empty-content messages — empty string or empty blocks array
///    (Anthropic rejects: "all messages must have non-empty content")
/// 5. Conversation must start with role="user"
/// 6. Conversation must not end with role="assistant" (no prefill — some
///    models reject it; defensive for cross-model compatibility)
/// 7. Re-merges consecutive same-role messages after mutations
/// 8. Tool IDs must match Anthropic-family provider validation
///    (`^[a-zA-Z0-9_-]+$`)
fn sanitize_message_sequence(messages: &mut Vec<AnthropicMessage>) {
    if messages.is_empty() {
        return;
    }

    // Step 1: Ensure every tool_use has a matching tool_result.
    patch_tool_result_coverage(messages);

    // Step 2: Remove orphan tool_results that don't match any tool_use
    // in the immediately preceding assistant message.
    remove_orphan_tool_results(messages);

    // Step 3: Deduplicate tool_results — keep only the last result per tool_use_id.
    // Anthropic rejects: "each tool_use must have a single result. Found multiple
    // `tool_result` blocks with id: <id>"
    dedup_tool_results(messages);

    // Step 4: Remove messages with empty content (empty string or empty blocks).
    // Anthropic rejects: "all messages must have non-empty content except for
    // the optional final assistant message"
    remove_empty_content_messages(messages);

    // Step 5: Re-merge consecutive same-role messages that may have been
    // introduced by insertions/removals in steps 1-4.
    let re_merged = merge_consecutive_messages(std::mem::take(messages));
    *messages = re_merged;

    // Step 6: Ensure the first message is role="user".
    if messages.first().is_some_and(|m| m.role != "user") {
        messages.insert(
            0,
            AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicMessageContent::String(".".to_string()),
            },
        );
    }

    // Step 7: Ensure the conversation does not end with an assistant message.
    ensure_not_trailing_assistant(messages);

    // Step 8: Rewrite invalid tool_use/tool_result IDs after all structural
    // fixes, so injected placeholder results are covered too.
    sanitize_tool_use_ids(messages);
}

/// Ensure every `tool_use` in assistant messages has a matching `tool_result`
/// in the immediately following user message.
///
/// If the next message is a user message with missing tool_results, placeholder
/// results are injected. If no user message follows, a new one is inserted.
fn patch_tool_result_coverage(messages: &mut Vec<AnthropicMessage>) {
    let mut i = 0;
    while i < messages.len() {
        if messages[i].role != "assistant" {
            i += 1;
            continue;
        }

        let tool_use_ids = extract_tool_use_ids(&messages[i]);
        if tool_use_ids.is_empty() {
            i += 1;
            continue;
        }

        let next_is_user = messages.get(i + 1).is_some_and(|m| m.role == "user");
        if next_is_user {
            // Check which tool_use IDs are already covered
            let covered_ids = extract_tool_result_ids(&messages[i + 1]);
            let missing: Vec<String> = tool_use_ids
                .into_iter()
                .filter(|id| !covered_ids.contains(id))
                .collect();

            if !missing.is_empty() {
                inject_placeholder_tool_results(&mut messages[i + 1], &missing);
            }
        } else {
            // No user message follows — insert one with all tool_results
            let tool_results: Vec<AnthropicContent> = tool_use_ids
                .into_iter()
                .map(|id| AnthropicContent::ToolResult {
                    tool_use_id: id,
                    content: Some(AnthropicMessageContent::String(
                        "[Tool call not executed]".to_string(),
                    )),
                    is_error: Some(true),
                    cache_control: None,
                })
                .collect();
            messages.insert(
                i + 1,
                AnthropicMessage {
                    role: "user".to_string(),
                    content: AnthropicMessageContent::Blocks(tool_results),
                },
            );
        }

        // Skip over the assistant + user pair
        i += 2;
    }
}

/// Remove `tool_result` blocks from user messages that don't match any
/// `tool_use` in the immediately preceding assistant message.
///
/// Also removes user messages that become empty after orphan removal.
fn remove_orphan_tool_results(messages: &mut Vec<AnthropicMessage>) {
    let mut i = 0;
    while i < messages.len() {
        if messages[i].role != "user" {
            i += 1;
            continue;
        }

        // Collect valid tool_use IDs from the immediately preceding assistant message
        let valid_ids: HashSet<String> = if i > 0 && messages[i - 1].role == "assistant" {
            extract_tool_use_ids(&messages[i - 1]).into_iter().collect()
        } else {
            HashSet::new()
        };

        if let AnthropicMessageContent::Blocks(blocks) = &mut messages[i].content {
            let had_tool_results = blocks
                .iter()
                .any(|b| matches!(b, AnthropicContent::ToolResult { .. }));

            if had_tool_results {
                blocks.retain(|block| match block {
                    AnthropicContent::ToolResult { tool_use_id, .. } => {
                        valid_ids.contains(tool_use_id)
                    }
                    _ => true,
                });
            }

            // If all blocks were removed, drop the message entirely
            if blocks.is_empty() {
                messages.remove(i);
                continue; // Don't increment — next message shifted into position i
            }
        }

        i += 1;
    }
}

/// Deduplicate `tool_result` blocks within user messages.
///
/// Anthropic rejects: "each tool_use must have a single result. Found multiple
/// `tool_result` blocks with id: <id>". When duplicates exist (e.g., from
/// retry flows or checkpoint corruption), keep only the **last** result per
/// `tool_use_id`.
fn dedup_tool_results(messages: &mut [AnthropicMessage]) {
    for msg in messages.iter_mut() {
        if msg.role != "user" {
            continue;
        }

        if let AnthropicMessageContent::Blocks(blocks) = &mut msg.content {
            let has_tool_results = blocks
                .iter()
                .any(|b| matches!(b, AnthropicContent::ToolResult { .. }));

            if !has_tool_results {
                continue;
            }

            // Find the last occurrence index for each tool_use_id
            let mut last_index: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for (i, block) in blocks.iter().enumerate() {
                if let AnthropicContent::ToolResult { tool_use_id, .. } = block {
                    last_index.insert(tool_use_id.clone(), i);
                }
            }

            // Retain non-tool-result blocks and only the last tool_result per ID
            let mut i = 0;
            blocks.retain(|block| {
                let keep = match block {
                    AnthropicContent::ToolResult { tool_use_id, .. } => {
                        last_index.get(tool_use_id) == Some(&i)
                    }
                    _ => true,
                };
                i += 1;
                keep
            });
        }
    }
}

/// Remove messages with empty content.
///
/// Anthropic rejects: "all messages must have non-empty content except for
/// the optional final assistant message". This covers:
/// - Empty string content (`""`)
/// - Empty blocks array (`[]`)
fn remove_empty_content_messages(messages: &mut Vec<AnthropicMessage>) {
    messages.retain(|msg| match &msg.content {
        AnthropicMessageContent::String(s) => !s.is_empty(),
        AnthropicMessageContent::Blocks(blocks) => !blocks.is_empty(),
    });
}

/// Ensure the conversation does not end with an assistant message that would
/// cause API errors.
///
/// Handling by case:
/// - **tool_use blocks present**: append a user message with placeholder
///   `tool_result` blocks (API requires every tool_use to have a result).
/// - **Empty or whitespace-only text**: remove the trailing assistant
///   (Anthropic rejects trailing whitespace-only assistant content, and
///   empty responses indicate incomplete/dangling state).
/// - **Substantive text content**: preserve it as-is. The Anthropic API
///   accepts trailing assistant messages as "prefill" for continuation on
///   models that support it (Claude Sonnet 4, Opus 4, etc.). Removing
///   valid context would lose information from checkpoints and context
///   managers that legitimately produce this state.
fn ensure_not_trailing_assistant(messages: &mut Vec<AnthropicMessage>) {
    // Loop in case removing an assistant reveals another trailing assistant.
    while messages.last().is_some_and(|m| m.role == "assistant") {
        let last = messages.last().expect("checked above");
        let tool_use_ids = extract_tool_use_ids(last);

        if !tool_use_ids.is_empty() {
            // Has tool_use — add user message with placeholder tool_results
            let tool_results: Vec<AnthropicContent> = tool_use_ids
                .into_iter()
                .map(|id| AnthropicContent::ToolResult {
                    tool_use_id: id,
                    content: Some(AnthropicMessageContent::String(
                        "[Tool call interrupted]".to_string(),
                    )),
                    is_error: Some(true),
                    cache_control: None,
                })
                .collect();
            messages.push(AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicMessageContent::Blocks(tool_results),
            });
            break; // We added a user message, so we're done
        }

        // No tool_use — check if content is substantive (worth keeping as prefill)
        let is_substantive = match &last.content {
            AnthropicMessageContent::String(s) => !s.trim().is_empty(),
            AnthropicMessageContent::Blocks(blocks) => blocks.iter().any(|b| match b {
                AnthropicContent::Text { text, .. } => !text.trim().is_empty(),
                // Non-text blocks (images, thinking) count as substantive
                AnthropicContent::Image { .. }
                | AnthropicContent::Thinking { .. }
                | AnthropicContent::RedactedThinking { .. } => true,
                // tool_use handled above; tool_result in assistant is unusual
                _ => false,
            }),
        };

        if is_substantive {
            // Preserve trailing assistant with real content (API accepts prefill)
            break;
        }

        // Empty/whitespace-only — discard (dangling/incomplete response)
        messages.pop();
    }
}

/// Extract all `tool_use` IDs from an Anthropic message's content blocks.
fn extract_tool_use_ids(msg: &AnthropicMessage) -> Vec<String> {
    match &msg.content {
        AnthropicMessageContent::Blocks(blocks) => blocks
            .iter()
            .filter_map(|b| {
                if let AnthropicContent::ToolUse { id, .. } = b {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    }
}

/// Extract all `tool_result` tool_use_ids from an Anthropic message's content blocks.
fn extract_tool_result_ids(msg: &AnthropicMessage) -> HashSet<String> {
    match &msg.content {
        AnthropicMessageContent::Blocks(blocks) => blocks
            .iter()
            .filter_map(|b| {
                if let AnthropicContent::ToolResult { tool_use_id, .. } = b {
                    Some(tool_use_id.clone())
                } else {
                    None
                }
            })
            .collect(),
        _ => HashSet::new(),
    }
}

/// Inject placeholder `tool_result` blocks for missing tool_use IDs into a user message.
fn inject_placeholder_tool_results(msg: &mut AnthropicMessage, missing_ids: &[String]) {
    let new_blocks: Vec<AnthropicContent> = missing_ids
        .iter()
        .map(|id| AnthropicContent::ToolResult {
            tool_use_id: id.clone(),
            content: Some(AnthropicMessageContent::String(
                "[Tool call not executed]".to_string(),
            )),
            is_error: Some(true),
            cache_control: None,
        })
        .collect();

    match &mut msg.content {
        AnthropicMessageContent::Blocks(blocks) => {
            blocks.extend(new_blocks);
        }
        AnthropicMessageContent::String(s) => {
            // Convert String content to Blocks. Skip creating an empty text block
            // from String("") — this avoids reintroducing empty text blocks that
            // per-message sanitization already stripped.
            let mut blocks = Vec::new();
            if !s.is_empty() {
                blocks.push(AnthropicContent::Text {
                    text: std::mem::take(s),
                    cache_control: None,
                });
            }
            blocks.extend(new_blocks);
            msg.content = AnthropicMessageContent::Blocks(blocks);
        }
    }
}

/// Rewrite invalid `tool_use.id` and matching `tool_result.tool_use_id` values
/// in the outgoing request. This preserves local conversation history while
/// satisfying Anthropic-family validators that require `^[a-zA-Z0-9_-]+$`.
///
/// Uses a two-pass approach to avoid clobbering originally-valid IDs:
///
/// 1. **Reserve pass**: collect every already-valid ID (matches
///    `^[a-zA-Z0-9_-]+$`) into `used_ids`/`id_map` (mapping it to itself).
///    These IDs are never rewritten, even if a later invalid ID would
///    normalize to the same value.
/// 2. **Rewrite pass**: visit every block again and only rename invalid or
///    empty IDs. Collision resolution in `safe_anthropic_tool_id` then suffixes
///    the *sanitized* ID (e.g. `a_b_2`) instead of stealing a reserved valid
///    original (`a_b`).
fn sanitize_tool_use_ids(messages: &mut [AnthropicMessage]) {
    let mut id_map = HashMap::new();
    let mut used_ids = HashSet::new();

    // Pass 1: reserve all already-valid IDs so they cannot be clobbered by
    // a later invalid ID that normalizes to the same value.
    for message in messages.iter() {
        let AnthropicMessageContent::Blocks(blocks) = &message.content else {
            continue;
        };

        for block in blocks {
            let id = match block {
                AnthropicContent::ToolUse { id, .. } => id,
                AnthropicContent::ToolResult { tool_use_id, .. } => tool_use_id,
                _ => continue,
            };

            if is_anthropic_tool_id(id) && !used_ids.contains(id) {
                used_ids.insert(id.clone());
                id_map.insert(id.clone(), id.clone());
            }
        }
    }

    // Pass 2: rewrite only invalid/empty IDs. Already-valid IDs were
    // registered above and are short-circuited by the lookup in
    // `safe_anthropic_tool_id`.
    for message in messages {
        let AnthropicMessageContent::Blocks(blocks) = &mut message.content else {
            continue;
        };

        for block in blocks {
            match block {
                AnthropicContent::ToolUse { id, .. } => {
                    rewrite_tool_id(id, &mut id_map, &mut used_ids);
                }
                AnthropicContent::ToolResult { tool_use_id, .. } => {
                    rewrite_tool_id(tool_use_id, &mut id_map, &mut used_ids);
                }
                _ => {}
            }
        }
    }
}

fn rewrite_tool_id(
    id: &mut String,
    id_map: &mut HashMap<String, String>,
    used_ids: &mut HashSet<String>,
) {
    // Already-valid IDs were reserved in pass 1 of `sanitize_tool_use_ids`
    // and are therefore present in `id_map` (mapped to themselves). Nothing
    // to do here.
    if is_anthropic_tool_id(id) && id_map.get(id).is_some_and(|mapped| mapped == id) {
        return;
    }

    let safe_id = safe_anthropic_tool_id(id, id_map, used_ids);
    if safe_id != *id {
        *id = safe_id;
    }
}

fn safe_anthropic_tool_id(
    original_id: &str,
    id_map: &mut HashMap<String, String>,
    used_ids: &mut HashSet<String>,
) -> String {
    if let Some(mapped) = id_map.get(original_id) {
        return mapped.clone();
    }

    let base = normalize_anthropic_tool_id(original_id);
    let mut candidate = if base.is_empty() {
        "toolu".to_string()
    } else {
        base
    };

    if used_ids.contains(&candidate) {
        let base = candidate;
        let mut suffix = 2usize;
        loop {
            candidate = format!("{base}_{suffix}");
            if !used_ids.contains(&candidate) {
                break;
            }
            suffix += 1;
        }
    }

    used_ids.insert(candidate.clone());
    id_map.insert(original_id.to_string(), candidate.clone());
    candidate
}

fn normalize_anthropic_tool_id(id: &str) -> String {
    id.chars()
        .map(|c| if is_anthropic_tool_id_char(c) { c } else { '_' })
        .collect()
}

fn is_anthropic_tool_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(is_anthropic_tool_id_char)
}

fn is_anthropic_tool_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Set cache_control on an AnthropicContent block.
fn set_block_cache_control(block: &mut AnthropicContent, cc: Option<AnthropicCacheControl>) {
    match block {
        AnthropicContent::Text { cache_control, .. }
        | AnthropicContent::ToolUse { cache_control, .. }
        | AnthropicContent::ToolResult { cache_control, .. }
        | AnthropicContent::Image { cache_control, .. } => *cache_control = cc,
        AnthropicContent::Thinking { .. } | AnthropicContent::RedactedThinking { .. } => {
            // Thinking blocks don't support cache_control
        }
    }
}

/// Merge consecutive messages with the same role into single messages.
///
/// Anthropic requires that tool_result blocks appear in a single user message
/// immediately after the assistant message containing the matching tool_use blocks.
/// When multiple tool results are converted individually, each becomes a separate
/// "user" message. This function combines them (and any other consecutive same-role
/// messages) into one.
fn merge_consecutive_messages(messages: Vec<AnthropicMessage>) -> Vec<AnthropicMessage> {
    if messages.is_empty() {
        return messages;
    }

    let mut result: Vec<AnthropicMessage> = Vec::with_capacity(messages.len());

    for msg in messages {
        let should_merge = result.last().is_some_and(|last| last.role == msg.role);

        if should_merge {
            let Some(last) = result.last_mut() else {
                // unreachable: guarded by is_some_and check above
                result.push(msg);
                continue;
            };
            let prev = std::mem::take(&mut last.content);
            last.content = merge_content(prev, msg.content);
        } else {
            result.push(msg);
        }
    }

    result
}

/// Convert AnthropicMessageContent to a Vec<AnthropicContent> blocks.
fn content_to_blocks(content: AnthropicMessageContent) -> Vec<AnthropicContent> {
    match content {
        AnthropicMessageContent::Blocks(blocks) => blocks,
        AnthropicMessageContent::String(s) => {
            vec![AnthropicContent::Text {
                text: s,
                cache_control: None,
            }]
        }
    }
}

/// Merge two AnthropicMessageContent values into one Blocks variant.
fn merge_content(
    a: AnthropicMessageContent,
    b: AnthropicMessageContent,
) -> AnthropicMessageContent {
    let mut blocks = content_to_blocks(a);
    blocks.extend(content_to_blocks(b));
    AnthropicMessageContent::Blocks(blocks)
}

/// Convert unified message to Anthropic message with optional auto-caching
fn to_anthropic_message_with_caching(
    msg: &Message,
    validator: &mut CacheControlValidator,
    auto_cache: bool,
) -> Result<AnthropicMessage> {
    // Determine the Anthropic role - Tool messages become "user" with tool_result content
    // (Anthropic doesn't support role="tool" like OpenAI)
    let role = match msg.role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "user", // Anthropic expects tool results as user messages
        Role::System => {
            return Err(Error::invalid_response(
                "System messages should be filtered out",
            ));
        }
    };

    // Get the message-level cache control, or use auto-cache
    let msg_cache_control = msg.cache_control().cloned().or_else(|| {
        if auto_cache {
            Some(crate::types::CacheControl::ephemeral())
        } else {
            None
        }
    });

    // Convert content parts
    let parts = msg.parts();

    // Check if any part has cache control, or if message has cache control
    let has_cache_control =
        msg_cache_control.is_some() || parts.iter().any(|p| p.cache_control().is_some());

    // Tool messages always use blocks format (tool_result content blocks)
    let force_blocks = msg.role == Role::Tool;

    let content = if parts.len() == 1 && !has_cache_control && !force_blocks {
        // Single content without cache control - try to use simple string format if text
        match &parts[0] {
            ContentPart::Text { text, .. } => AnthropicMessageContent::String(text.clone()),
            _ => AnthropicMessageContent::Blocks(vec![to_anthropic_content_part(
                &parts[0], None, validator, true,
            )?]),
        }
    } else {
        // Multiple content parts, has cache control, or tool message - use array format
        let num_parts = parts.len();
        let content_parts = parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                let is_last = i == num_parts - 1;
                // For the last part, include message-level cache control as fallback
                let fallback_cache = if is_last {
                    msg_cache_control.as_ref()
                } else {
                    None
                };
                to_anthropic_content_part(part, fallback_cache, validator, is_last)
            })
            .collect::<Result<Vec<_>>>()?;

        AnthropicMessageContent::Blocks(content_parts)
    };

    Ok(AnthropicMessage {
        role: role.to_string(),
        content,
    })
}

/// Convert a single message to Anthropic format (test helper, no auto-caching)
#[cfg(test)]
fn to_anthropic_message(
    msg: &Message,
    validator: &mut CacheControlValidator,
) -> Result<AnthropicMessage> {
    to_anthropic_message_with_caching(msg, validator, false)
}

/// Convert a content part to Anthropic format with cache control
fn to_anthropic_content_part(
    part: &ContentPart,
    fallback_cache: Option<&crate::types::CacheControl>,
    validator: &mut CacheControlValidator,
    is_last_part: bool,
) -> Result<AnthropicContent> {
    // Get the part-level cache control, with fallback to message-level for last part
    let part_cache = part.cache_control();
    let effective_cache = if part_cache.is_some() {
        part_cache
    } else if is_last_part {
        fallback_cache
    } else {
        None
    };

    match part {
        ContentPart::Text { text, .. } => {
            let context = CacheContext::user_message_part();
            let validated_cache = validator.validate(effective_cache, context);

            Ok(AnthropicContent::Text {
                text: text.clone(),
                cache_control: validated_cache.map(|c| AnthropicCacheControl::from(&c)),
            })
        }
        ContentPart::Image { url, .. } => {
            let context = CacheContext::image_content();
            let validated_cache = validator.validate(effective_cache, context);

            Ok(AnthropicContent::Image {
                source: parse_image_source(url)?,
                cache_control: validated_cache.map(|c| AnthropicCacheControl::from(&c)),
            })
        }
        ContentPart::ToolCall {
            id,
            name,
            arguments,
            ..
        } => {
            let context = CacheContext::assistant_message_part();
            let validated_cache = validator.validate(effective_cache, context);

            Ok(AnthropicContent::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: arguments.clone(),
                cache_control: validated_cache.map(|c| AnthropicCacheControl::from(&c)),
            })
        }
        ContentPart::ToolResult {
            tool_call_id,
            content,
            ..
        } => {
            let context = CacheContext::tool_result();
            let validated_cache = validator.validate(effective_cache, context);

            Ok(AnthropicContent::ToolResult {
                tool_use_id: tool_call_id.clone(),
                content: Some(AnthropicMessageContent::String(content.to_string())),
                is_error: None,
                cache_control: validated_cache.map(|c| AnthropicCacheControl::from(&c)),
            })
        }
    }
}

/// Parse image URL to Anthropic image source format
fn parse_image_source(url: &str) -> Result<AnthropicSource> {
    if url.starts_with("data:") {
        // Data URL format: data:image/png;base64,iVBORw0KG...
        let parts: Vec<&str> = url.splitn(2, ',').collect();
        if parts.len() != 2 {
            return Err(Error::invalid_response("Invalid data URL format"));
        }

        let media_type = parts[0]
            .strip_prefix("data:")
            .and_then(|s| s.strip_suffix(";base64"))
            .ok_or_else(|| Error::invalid_response("Invalid data URL media type"))?;

        Ok(AnthropicSource {
            type_: "base64".to_string(),
            media_type: media_type.to_string(),
            data: parts[1].to_string(),
        })
    } else {
        // URL format (Anthropic doesn't support direct URLs, would need to fetch)
        Err(Error::invalid_response(
            "Anthropic requires base64-encoded images, not URLs",
        ))
    }
}

/// Convert Anthropic response to unified response with warnings from conversion
pub fn from_anthropic_response_with_warnings(
    resp: AnthropicResponse,
    warnings: Vec<CacheWarning>,
) -> Result<GenerateResponse> {
    use crate::types::{ResponseWarning, ToolCall};

    let content: Vec<ResponseContent> = resp
        .content
        .iter()
        .filter_map(|c| match c {
            AnthropicContent::Text { text, .. } => {
                Some(ResponseContent::Text { text: text.clone() })
            }
            AnthropicContent::Thinking { thinking, .. } => Some(ResponseContent::Reasoning {
                reasoning: thinking.clone(),
            }),
            AnthropicContent::ToolUse {
                id, name, input, ..
            } => Some(ResponseContent::ToolCall(ToolCall {
                id: id.clone(),
                name: name.clone(),
                arguments: input.clone(),
                metadata: None,
            })),
            _ => None,
        })
        .collect();

    if content.is_empty() {
        return Err(Error::invalid_response("No content in response"));
    }

    // Determine finish reason - tool_use should be ToolCalls
    let finish_reason = if content
        .iter()
        .any(|c| matches!(c, ResponseContent::ToolCall(_)))
    {
        FinishReason::with_raw(FinishReasonKind::ToolCalls, "tool_use")
    } else {
        parse_stop_reason(&resp.stop_reason)
    };

    // Calculate cache tokens
    // Anthropic token breakdown (per official API docs):
    // - input_tokens: tokens NOT read from or written to cache (non-cached input)
    // - cache_creation_input_tokens: tokens written to cache (cache miss, creating entry)
    // - cache_read_input_tokens: tokens read from cache (cache hit)
    // Total input = non-cached + cache-write + cache-read
    let cache_creation = resp.usage.cache_creation_input_tokens.unwrap_or(0);
    let cache_read = resp.usage.cache_read_input_tokens.unwrap_or(0);
    let input_tokens = resp.usage.input_tokens;
    let output_tokens = resp.usage.output_tokens;

    let total_input = input_tokens + cache_creation + cache_read;

    let usage = Usage::with_details(
        InputTokenDetails {
            total: Some(total_input),
            no_cache: Some(input_tokens),
            cache_read: if cache_read > 0 {
                Some(cache_read)
            } else {
                None
            },
            cache_write: if cache_creation > 0 {
                Some(cache_creation)
            } else {
                None
            },
        },
        OutputTokenDetails {
            total: Some(output_tokens),
            text: None,      // Anthropic doesn't break down output tokens
            reasoning: None, // Will be populated if extended thinking is used
        },
        Some(serde_json::to_value(&resp.usage).unwrap_or_default()),
    );

    // Convert cache warnings to response warnings
    let response_warnings: Option<Vec<ResponseWarning>> = if warnings.is_empty() {
        None
    } else {
        Some(warnings.into_iter().map(ResponseWarning::from).collect())
    };

    Ok(GenerateResponse {
        content,
        usage,
        finish_reason,
        metadata: Some(json!({
            "id": resp.id,
            "model": resp.model,
        })),
        warnings: response_warnings,
    })
}

/// Parse Anthropic stop reason to unified finish reason
fn parse_stop_reason(reason: &Option<String>) -> FinishReason {
    match reason.as_deref() {
        Some("end_turn") => FinishReason::with_raw(FinishReasonKind::Stop, "end_turn"),
        Some("max_tokens") => FinishReason::with_raw(FinishReasonKind::Length, "max_tokens"),
        Some("stop_sequence") => FinishReason::with_raw(FinishReasonKind::Stop, "stop_sequence"),
        Some("tool_use") => FinishReason::with_raw(FinishReasonKind::ToolCalls, "tool_use"),
        Some(raw) => FinishReason::with_raw(FinishReasonKind::Other, raw),
        None => FinishReason::other(),
    }
}

#[cfg(test)]
mod tests;
