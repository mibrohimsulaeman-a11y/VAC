#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum HookEventName {
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    SessionStart,
    UserPromptSubmit,
    Stop,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum HookHandlerType {
    Command,
    Prompt,
    Agent,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum HookExecutionMode {
    Sync,
    Async,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum HookScope {
    Thread,
    Turn,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum HookSource {
    System,
    User,
    Project,
    Mdm,
    SessionFlags,
    Plugin,
    LegacyManagedConfigFile,
    LegacyManagedConfigMdm,
    #[default]
    Unknown,
}

impl HookSource {
    /// Returns whether hooks from this source are managed and therefore not
    /// user-configurable.
    pub fn is_managed(self) -> bool {
        matches!(
            self,
            Self::System | Self::Mdm | Self::LegacyManagedConfigFile | Self::LegacyManagedConfigMdm
        )
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum HookRunStatus {
    Running,
    Completed,
    Failed,
    Blocked,
    Stopped,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum HookOutputEntryKind {
    Warning,
    Stop,
    Feedback,
    Context,
    Error,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub struct HookOutputEntry {
    pub kind: HookOutputEntryKind,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub struct HookRunSummary {
    pub id: String,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub execution_mode: HookExecutionMode,
    pub scope: HookScope,
    pub source_path: AbsolutePathBuf,
    #[serde(default)]
    pub source: HookSource,
    pub display_order: i64,
    pub status: HookRunStatus,
    pub status_message: Option<String>,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number | null")]
    pub completed_at: Option<i64>,
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
    pub entries: Vec<HookOutputEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub struct HookStartedEvent {
    pub turn_id: Option<String>,
    pub run: HookRunSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub struct HookCompletedEvent {
    pub turn_id: Option<String>,
    pub run: HookRunSummary,
}

impl From<CollabAgentSpawnBeginEvent> for EventMsg {
    fn from(event: CollabAgentSpawnBeginEvent) -> Self {
        EventMsg::CollabAgentSpawnBegin(event)
    }
}

impl From<CollabAgentSpawnEndEvent> for EventMsg {
    fn from(event: CollabAgentSpawnEndEvent) -> Self {
        EventMsg::CollabAgentSpawnEnd(event)
    }
}

impl From<CollabAgentInteractionBeginEvent> for EventMsg {
    fn from(event: CollabAgentInteractionBeginEvent) -> Self {
        EventMsg::CollabAgentInteractionBegin(event)
    }
}

impl From<CollabAgentInteractionEndEvent> for EventMsg {
    fn from(event: CollabAgentInteractionEndEvent) -> Self {
        EventMsg::CollabAgentInteractionEnd(event)
    }
}

impl From<CollabWaitingBeginEvent> for EventMsg {
    fn from(event: CollabWaitingBeginEvent) -> Self {
        EventMsg::CollabWaitingBegin(event)
    }
}

impl From<CollabWaitingEndEvent> for EventMsg {
    fn from(event: CollabWaitingEndEvent) -> Self {
        EventMsg::CollabWaitingEnd(event)
    }
}

impl From<CollabCloseBeginEvent> for EventMsg {
    fn from(event: CollabCloseBeginEvent) -> Self {
        EventMsg::CollabCloseBegin(event)
    }
}

impl From<CollabCloseEndEvent> for EventMsg {
    fn from(event: CollabCloseEndEvent) -> Self {
        EventMsg::CollabCloseEnd(event)
    }
}

impl From<CollabResumeBeginEvent> for EventMsg {
    fn from(event: CollabResumeBeginEvent) -> Self {
        EventMsg::CollabResumeBegin(event)
    }
}

impl From<CollabResumeEndEvent> for EventMsg {
    fn from(event: CollabResumeEndEvent) -> Self {
        EventMsg::CollabResumeEnd(event)
    }
}

/// Agent lifecycle status, derived from emitted events.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is waiting for initialization.
    #[default]
    PendingInit,
    /// Agent is currently running.
    Running,
    /// Agent's current turn was interrupted and it may receive more input.
    Interrupted,
    /// Agent is done. Contains the final assistant message.
    Completed(Option<String>),
    /// Agent encountered an error.
    Errored(String),
    /// Agent has been shutdown.
    Shutdown,
    /// Agent is not found.
    NotFound,
}

/// Turn kinds that reject same-turn steering.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum NonSteerableTurnKind {
    Review,
    Compact,
}

/// VAC errors that we expose to clients.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum VACErrorInfo {
    ContextWindowExceeded,
    UsageLimitExceeded,
    ServerOverloaded,
    CyberPolicy,
    HttpConnectionFailed {
        http_status_code: Option<u16>,
    },
    /// Failed to connect to the response SSE stream.
    ResponseStreamConnectionFailed {
        http_status_code: Option<u16>,
    },
    InternalServerError,
    Unauthorized,
    BadRequest,
    SandboxError,
    /// The response SSE stream disconnected in the middle of a turnbefore completion.
    ResponseStreamDisconnected {
        http_status_code: Option<u16>,
    },
    /// Reached the retry limit for responses.
    ResponseTooManyFailedAttempts {
        http_status_code: Option<u16>,
    },
    /// Returned when `turn/start` or `turn/steer` is submitted while the current active turn
    /// cannot accept same-turn steering, for example `/review` or manual `/compact`.
    ActiveTurnNotSteerable {
        turn_kind: NonSteerableTurnKind,
    },
    ThreadRollbackFailed,
    Other,
}

impl VACErrorInfo {
    /// Whether this error should mark the current turn as failed when replaying history.
    pub fn affects_turn_status(&self) -> bool {
        match self {
            Self::ThreadRollbackFailed | Self::ActiveTurnNotSteerable { .. } => false,
            Self::ContextWindowExceeded
            | Self::UsageLimitExceeded
            | Self::ServerOverloaded
            | Self::CyberPolicy
            | Self::HttpConnectionFailed { .. }
            | Self::ResponseStreamConnectionFailed { .. }
            | Self::InternalServerError
            | Self::Unauthorized
            | Self::BadRequest
            | Self::SandboxError
            | Self::ResponseStreamDisconnected { .. }
            | Self::ResponseTooManyFailedAttempts { .. }
            | Self::Other => true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS, JsonSchema)]
pub struct RawResponseItemEvent {
    pub item: ResponseItem,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS, JsonSchema)]
pub struct ItemStartedEvent {
    pub thread_id: ThreadId,
    pub turn_id: String,
    pub item: TurnItem,
}

impl HasLegacyEvent for ItemStartedEvent {
    fn as_legacy_events(&self, _: bool) -> Vec<EventMsg> {
        match &self.item {
            TurnItem::WebSearch(item) => vec![EventMsg::WebSearchBegin(WebSearchBeginEvent {
                call_id: item.id.clone(),
            })],
            TurnItem::ImageView(_) => Vec::new(),
            TurnItem::ImageGeneration(item) => {
                vec![EventMsg::ImageGenerationBegin(ImageGenerationBeginEvent {
                    call_id: item.id.clone(),
                })]
            }
            TurnItem::FileChange(item) => vec![item.as_legacy_begin_event(self.turn_id.clone())],
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS, JsonSchema)]
pub struct ItemCompletedEvent {
    pub thread_id: ThreadId,
    pub turn_id: String,
    pub item: TurnItem,
}

pub trait HasLegacyEvent {
    fn as_legacy_events(&self, show_raw_agent_reasoning: bool) -> Vec<EventMsg>;
}

impl HasLegacyEvent for ItemCompletedEvent {
    fn as_legacy_events(&self, show_raw_agent_reasoning: bool) -> Vec<EventMsg> {
        match &self.item {
            TurnItem::FileChange(item) => item
                .as_legacy_end_event(self.turn_id.clone())
                .into_iter()
                .collect(),
            _ => self.item.as_legacy_events(show_raw_agent_reasoning),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS, JsonSchema)]
pub struct AgentMessageContentDeltaEvent {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

impl HasLegacyEvent for AgentMessageContentDeltaEvent {
    fn as_legacy_events(&self, _: bool) -> Vec<EventMsg> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS, JsonSchema)]
pub struct PlanDeltaEvent {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS, JsonSchema)]
pub struct ReasoningContentDeltaEvent {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
    // load with default value so it's backward compatible with the old format.
    #[serde(default)]
    pub summary_index: i64,
}

impl HasLegacyEvent for ReasoningContentDeltaEvent {
    fn as_legacy_events(&self, _: bool) -> Vec<EventMsg> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS, JsonSchema)]
pub struct ReasoningRawContentDeltaEvent {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
    // load with default value so it's backward compatible with the old format.
    #[serde(default)]
    pub content_index: i64,
}

impl HasLegacyEvent for ReasoningRawContentDeltaEvent {
    fn as_legacy_events(&self, _: bool) -> Vec<EventMsg> {
        Vec::new()
    }
}

impl HasLegacyEvent for EventMsg {
    fn as_legacy_events(&self, show_raw_agent_reasoning: bool) -> Vec<EventMsg> {
        match self {
            EventMsg::ItemStarted(event) => event.as_legacy_events(show_raw_agent_reasoning),
            EventMsg::ItemCompleted(event) => event.as_legacy_events(show_raw_agent_reasoning),
            EventMsg::AgentMessageContentDelta(event) => {
                event.as_legacy_events(show_raw_agent_reasoning)
            }
            EventMsg::ReasoningContentDelta(event) => {
                event.as_legacy_events(show_raw_agent_reasoning)
            }
            EventMsg::ReasoningRawContentDelta(event) => {
                event.as_legacy_events(show_raw_agent_reasoning)
            }
            _ => Vec::new(),
        }
    }
}

