#[cfg_attr(not(test), allow(dead_code))]
enum VACOpTarget {
    #[cfg(test)]
    Direct(UnboundedSender<AppCommand>),
    AppEvent,
}

/// Snapshot of active-cell state that affects transcript overlay rendering.
///
/// The overlay keeps a cached "live tail" for the in-flight cell; this key lets
/// it cheaply decide when to recompute that tail as the active cell evolves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActiveCellTranscriptKey {
    /// Cache-busting revision for in-place updates.
    ///
    /// Many active cells are updated incrementally while streaming (for example when exec groups
    /// add output or change status), and the transcript overlay caches its live tail, so this
    /// revision gives a cheap way to say "same active cell, but its transcript output is different
    /// now". Callers bump it on any mutation that can affect `HistoryCell::transcript_lines`.
    pub(crate) revision: u64,
    /// Whether the active cell continues the prior stream, which affects
    /// spacing between transcript blocks.
    pub(crate) is_stream_continuation: bool,
    /// Optional animation tick for time-dependent transcript output.
    ///
    /// When this changes, the overlay recomputes the cached tail even if the revision and width
    /// are unchanged, which is how shimmer/spinner visuals can animate in the overlay without any
    /// underlying data change.
    pub(crate) animation_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UserMessage {
    text: String,
    local_images: Vec<LocalImageAttachment>,
    /// Remote image attachments represented as URLs (for example data URLs)
    /// provided by app-server clients.
    ///
    /// Unlike `local_images`, these are not created by TUI image attach/paste
    /// flows. The TUI can restore and remove them while editing/backtracking.
    remote_image_urls: Vec<String>,
    text_elements: Vec<TextElement>,
    mention_bindings: Vec<MentionBinding>,
}

#[derive(Clone, Debug, PartialEq)]
enum UserMessageHistoryRecord {
    UserMessageText,
    Override(UserMessageHistoryOverride),
}

#[derive(Clone, Debug, PartialEq)]
struct UserMessageHistoryOverride {
    text: String,
    text_elements: Vec<TextElement>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShellEscapePolicy {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, PartialEq)]
struct QueuedUserMessage {
    user_message: UserMessage,
    action: QueuedInputAction,
}

impl QueuedUserMessage {
    fn new(user_message: UserMessage, action: QueuedInputAction) -> Self {
        Self {
            user_message,
            action,
        }
    }

    fn into_user_message(self) -> UserMessage {
        self.user_message
    }
}

impl From<UserMessage> for QueuedUserMessage {
    fn from(user_message: UserMessage) -> Self {
        Self::new(user_message, QueuedInputAction::Plain)
    }
}

impl Deref for QueuedUserMessage {
    type Target = UserMessage;

    fn deref(&self) -> &Self::Target {
        &self.user_message
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueueDrain {
    Continue,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct ThreadComposerState {
    text: String,
    local_images: Vec<LocalImageAttachment>,
    remote_image_urls: Vec<String>,
    text_elements: Vec<TextElement>,
    mention_bindings: Vec<MentionBinding>,
    pending_pastes: Vec<(String, String)>,
}

impl ThreadComposerState {
    fn has_content(&self) -> bool {
        !self.text.is_empty()
            || !self.local_images.is_empty()
            || !self.remote_image_urls.is_empty()
            || !self.text_elements.is_empty()
            || !self.mention_bindings.is_empty()
            || !self.pending_pastes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThreadInputState {
    composer: Option<ThreadComposerState>,
    pending_steers: VecDeque<UserMessage>,
    pending_steer_history_records: VecDeque<UserMessageHistoryRecord>,
    pending_steer_compare_keys: VecDeque<PendingSteerCompareKey>,
    rejected_steers_queue: VecDeque<UserMessage>,
    rejected_steer_history_records: VecDeque<UserMessageHistoryRecord>,
    queued_user_messages: VecDeque<QueuedUserMessage>,
    queued_user_message_history_records: VecDeque<UserMessageHistoryRecord>,
    user_turn_pending_start: bool,
    current_collaboration_mode: CollaborationMode,
    active_collaboration_mask: Option<CollaborationModeMask>,
    task_running: bool,
    agent_turn_running: bool,
}

impl From<String> for UserMessage {
    fn from(text: String) -> Self {
        Self {
            text,
            local_images: Vec::new(),
            remote_image_urls: Vec::new(),
            // Plain text conversion has no UI element ranges.
            text_elements: Vec::new(),
            mention_bindings: Vec::new(),
        }
    }
}

impl From<&str> for UserMessage {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_string(),
            local_images: Vec::new(),
            remote_image_urls: Vec::new(),
            // Plain text conversion has no UI element ranges.
            text_elements: Vec::new(),
            mention_bindings: Vec::new(),
        }
    }
}

struct PendingSteer {
    user_message: UserMessage,
    history_record: UserMessageHistoryRecord,
    compare_key: PendingSteerCompareKey,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum InterruptedTurnNoticeMode {
    #[default]
    Default,
    Suppress,
}

pub(crate) fn create_initial_user_message(
    text: Option<String>,
    local_image_paths: Vec<PathBuf>,
    text_elements: Vec<TextElement>,
) -> Option<UserMessage> {
    let text = text.unwrap_or_default();
    if text.is_empty() && local_image_paths.is_empty() {
        None
    } else {
        let local_images = local_image_paths
            .into_iter()
            .enumerate()
            .map(|(idx, path)| LocalImageAttachment {
                placeholder: local_image_label_text(idx + 1),
                path,
            })
            .collect();
        Some(UserMessage {
            text,
            local_images,
            remote_image_urls: Vec::new(),
            text_elements,
            mention_bindings: Vec::new(),
        })
    }
}

fn append_text_with_rebased_elements(
    target_text: &mut String,
    target_text_elements: &mut Vec<TextElement>,
    text: &str,
    text_elements: impl IntoIterator<Item = TextElement>,
) {
    let offset = target_text.len();
    target_text.push_str(text);
    target_text_elements.extend(text_elements.into_iter().map(|mut element| {
        element.byte_range.start += offset;
        element.byte_range.end += offset;
        element
    }));
}

fn app_server_text_elements(elements: &[TextElement]) -> Vec<AppServerTextElement> {
    elements.iter().cloned().map(Into::into).collect()
}

fn build_placeholder_mapping(
    local_images: Vec<LocalImageAttachment>,
    next_label: &mut usize,
) -> (HashMap<String, String>, Vec<LocalImageAttachment>) {
    let mut mapping: HashMap<String, String> = HashMap::new();
    let mut remapped_images = Vec::new();
    for attachment in local_images {
        let new_placeholder = local_image_label_text(*next_label);
        *next_label += 1;
        mapping.insert(attachment.placeholder.clone(), new_placeholder.clone());
        remapped_images.push(LocalImageAttachment {
            placeholder: new_placeholder,
            path: attachment.path,
        });
    }
    (mapping, remapped_images)
}

fn remap_placeholders_in_text(
    text: String,
    text_elements: Vec<TextElement>,
    mapping: &HashMap<String, String>,
) -> (String, Vec<TextElement>) {
    if mapping.is_empty() {
        return (text, text_elements);
    }

    let mut elements = text_elements;
    elements.sort_by_key(|elem| elem.byte_range.start);

    let mut cursor = 0usize;
    let mut rebuilt = String::new();
    let mut rebuilt_elements = Vec::new();
    for mut elem in elements {
        let start = elem.byte_range.start.min(text.len());
        let end = elem.byte_range.end.min(text.len());
        if let Some(segment) = text.get(cursor..start) {
            rebuilt.push_str(segment);
        }

        let original = text.get(start..end).unwrap_or("");
        let placeholder = elem.placeholder(&text);
        let replacement = placeholder
            .and_then(|ph| mapping.get(ph))
            .map(String::as_str)
            .unwrap_or(original);

        let elem_start = rebuilt.len();
        rebuilt.push_str(replacement);
        let elem_end = rebuilt.len();

        if let Some(remapped) = placeholder.and_then(|ph| mapping.get(ph)) {
            elem.set_placeholder(Some(remapped.clone()));
        }
        elem.byte_range = (elem_start..elem_end).into();
        rebuilt_elements.push(elem);
        cursor = end;
    }
    if let Some(segment) = text.get(cursor..) {
        rebuilt.push_str(segment);
    }

    (rebuilt, rebuilt_elements)
}

// When merging multiple queued drafts (e.g., after interrupt), each draft starts numbering
// its attachments at [Image #1]. Reassign placeholder labels based on the attachment list so
// the combined local_image_paths order matches the labels, even if placeholders were moved
// in the text (e.g., [Image #2] appearing before [Image #1]). Apply the same remapping to
// history overrides so restored drafts and rendered transcript entries agree.
fn remap_placeholders_for_message_and_history_record(
    message: UserMessage,
    history_record: UserMessageHistoryRecord,
    next_label: &mut usize,
) -> (UserMessage, UserMessageHistoryRecord) {
    let UserMessage {
        text,
        text_elements,
        local_images,
        remote_image_urls,
        mention_bindings,
    } = message;
    let (mapping, remapped_images) = build_placeholder_mapping(local_images, next_label);
    let (text, text_elements) = remap_placeholders_in_text(text, text_elements, &mapping);
    let history_record = match history_record {
        UserMessageHistoryRecord::Override(history) if !history.text.is_empty() => {
            let (text, text_elements) =
                remap_placeholders_in_text(history.text, history.text_elements, &mapping);
            UserMessageHistoryRecord::Override(UserMessageHistoryOverride {
                text,
                text_elements,
            })
        }
        record => record,
    };

    (
        UserMessage {
            text,
            local_images: remapped_images,
            remote_image_urls,
            text_elements,
            mention_bindings,
        },
        history_record,
    )
}

#[cfg(test)]
fn remap_placeholders_for_message(message: UserMessage, next_label: &mut usize) -> UserMessage {
    remap_placeholders_for_message_and_history_record(
        message,
        UserMessageHistoryRecord::UserMessageText,
        next_label,
    )
    .0
}

fn remap_user_messages_with_history_records(
    messages: Vec<(UserMessage, UserMessageHistoryRecord)>,
) -> Vec<(UserMessage, UserMessageHistoryRecord)> {
    let total_remote_images = messages
        .iter()
        .map(|(message, _)| message.remote_image_urls.len())
        .sum::<usize>();
    let mut next_image_label = total_remote_images + 1;
    messages
        .into_iter()
        .map(|(message, history_record)| {
            remap_placeholders_for_message_and_history_record(
                message,
                history_record,
                &mut next_image_label,
            )
        })
        .collect()
}

fn merge_user_messages(messages: Vec<UserMessage>) -> UserMessage {
    let messages = remap_user_messages_with_history_records(
        messages
            .into_iter()
            .map(|message| (message, UserMessageHistoryRecord::UserMessageText))
            .collect(),
    );
    merge_remapped_user_messages(messages.into_iter().map(|(message, _)| message))
}

fn merge_remapped_user_messages(messages: impl IntoIterator<Item = UserMessage>) -> UserMessage {
    let mut combined = UserMessage {
        text: String::new(),
        text_elements: Vec::new(),
        local_images: Vec::new(),
        remote_image_urls: Vec::new(),
        mention_bindings: Vec::new(),
    };

    for (idx, message) in messages.into_iter().enumerate() {
        if idx > 0 {
            combined.text.push('\n');
        }
        let UserMessage {
            text,
            text_elements,
            local_images,
            remote_image_urls,
            mention_bindings,
        } = message;
        append_text_with_rebased_elements(
            &mut combined.text,
            &mut combined.text_elements,
            &text,
            text_elements,
        );
        combined.local_images.extend(local_images);
        combined.remote_image_urls.extend(remote_image_urls);
        combined.mention_bindings.extend(mention_bindings);
    }

    combined
}

fn user_message_for_restore(
    message: UserMessage,
    history_record: &UserMessageHistoryRecord,
) -> UserMessage {
    match history_record {
        UserMessageHistoryRecord::Override(history) if !history.text.is_empty() => UserMessage {
            text: history.text.clone(),
            text_elements: history.text_elements.clone(),
            ..message
        },
        UserMessageHistoryRecord::Override(_) | UserMessageHistoryRecord::UserMessageText => {
            message
        }
    }
}

