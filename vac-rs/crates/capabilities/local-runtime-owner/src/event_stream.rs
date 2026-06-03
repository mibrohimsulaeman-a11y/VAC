//! Event-stream ownership primitives for the local runtime owner.
//!
//! The stream is intentionally app-server-protocol-free. It converts
//! `vac_protocol::Event` into owner-owned envelopes, reuses
//! `vac_core::local_runtime::LocalRuntimeBridge` for semantic events, and
//! keeps lifecycle/TUI compatibility payloads outside the semantic contract
//! while protocol retirement is pending.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use vac_core::local_runtime::BridgeOutput;
use vac_core::local_runtime::LocalRuntimeBridge;
use vac_core::local_runtime::RuntimeEvent;
use vac_protocol::protocol::Event;
use vac_protocol::protocol::EventMsg;

/// Delivery policy for an owner-stream item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeEventDelivery {
    /// Transcript-critical or terminal event. The owner must enqueue it or fail
    /// loudly instead of silently dropping it.
    Lossless,
    /// UI-progress event. The owner may drop it under backpressure, but must
    /// surface lag to subscribers.
    BestEffort,
}

/// Owner-level event classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeEventKind {
    AssistantDelta,
    ReasoningDelta,
    PlanDelta,
    ToolStarted,
    ToolFinished,
    ExecOutputDelta,
    TerminalInteraction,
    PatchProgress,
    ValidationStarted,
    ValidationFinished,
    ApprovalRequested,
    UserInputRequested,
    ThreadMetadataUpdated,
    RealtimeStarted,
    RealtimeClosed,
    TurnStarted,
    TurnCompleted,
    TurnInterrupted,
    TurnFailed,
    ReviewMode,
    Lagged,
    Shutdown,
    Unsupported,
}

/// Payload owned by the local runtime owner stream.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeOwnerEventPayload {
    Runtime(RuntimeEvent),
    ExecOutputDelta {
        call_id: String,
        stream: String,
        chunk: Vec<u8>,
    },
    TerminalInteraction {
        call_id: String,
        process_id: String,
        stdin: String,
    },
    PatchProgress {
        call_id: String,
        changed_files: usize,
    },
    UserInputRequested {
        turn_id: String,
        call_id: String,
        question_count: usize,
    },
    ThreadNameUpdated {
        thread_id: String,
        thread_name: Option<String>,
    },
    ThreadGoalUpdated {
        thread_id: String,
        turn_id: Option<String>,
    },
    RealtimeStarted {
        realtime_session_id: Option<String>,
        version: String,
    },
    RealtimeClosed {
        reason: Option<String>,
    },
    Failure {
        message: String,
    },
    Shutdown,
    Unsupported {
        event_type: String,
        reason: String,
    },
}

/// Temporary compatibility classification for existing TUI surfaces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeTuiCompatibilityEvent {
    NativeOwnerEvent {
        kind: RuntimeEventKind,
        delivery: RuntimeEventDelivery,
    },
    LegacyAdapterRequired {
        kind: RuntimeEventKind,
        reason: String,
    },
    Lagged {
        dropped: u64,
        next_sequence: u64,
    },
}

/// Event envelope owned by the local runtime owner stream.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeEventEnvelope {
    pub sequence: u64,
    pub kind: RuntimeEventKind,
    pub delivery: RuntimeEventDelivery,
    pub payload: RuntimeOwnerEventPayload,
}

impl RuntimeEventEnvelope {
    #[must_use]
    pub fn new(
        sequence: u64,
        kind: RuntimeEventKind,
        delivery: RuntimeEventDelivery,
        payload: RuntimeOwnerEventPayload,
    ) -> Self {
        Self {
            sequence,
            kind,
            delivery,
            payload,
        }
    }

    #[must_use]
    pub fn to_tui_compatibility_event(&self) -> RuntimeTuiCompatibilityEvent {
        match &self.payload {
            RuntimeOwnerEventPayload::Unsupported { reason, .. } => {
                RuntimeTuiCompatibilityEvent::LegacyAdapterRequired {
                    kind: self.kind,
                    reason: reason.clone(),
                }
            }
            RuntimeOwnerEventPayload::Runtime(_)
            | RuntimeOwnerEventPayload::ExecOutputDelta { .. }
            | RuntimeOwnerEventPayload::TerminalInteraction { .. }
            | RuntimeOwnerEventPayload::PatchProgress { .. }
            | RuntimeOwnerEventPayload::UserInputRequested { .. }
            | RuntimeOwnerEventPayload::ThreadNameUpdated { .. }
            | RuntimeOwnerEventPayload::ThreadGoalUpdated { .. }
            | RuntimeOwnerEventPayload::RealtimeStarted { .. }
            | RuntimeOwnerEventPayload::RealtimeClosed { .. }
            | RuntimeOwnerEventPayload::Failure { .. }
            | RuntimeOwnerEventPayload::Shutdown => {
                RuntimeTuiCompatibilityEvent::NativeOwnerEvent {
                    kind: self.kind,
                    delivery: self.delivery,
                }
            }
        }
    }
}

/// Item visible to a subscriber.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeEventStreamItem {
    Event(RuntimeEventEnvelope),
    /// One or more best-effort events were dropped before the next delivered
    /// event. Lossless events are never represented by this marker.
    Lagged {
        dropped: u64,
        next_sequence: u64,
    },
}

impl RuntimeEventStreamItem {
    #[must_use]
    pub fn to_tui_compatibility_event(&self) -> RuntimeTuiCompatibilityEvent {
        match self {
            Self::Event(envelope) => envelope.to_tui_compatibility_event(),
            Self::Lagged {
                dropped,
                next_sequence,
            } => RuntimeTuiCompatibilityEvent::Lagged {
                dropped: *dropped,
                next_sequence: *next_sequence,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEventStreamError {
    ZeroCapacity,
    LosslessBackpressure {
        capacity: usize,
        sequence: u64,
        kind: RuntimeEventKind,
    },
}

impl std::fmt::Display for RuntimeEventStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroCapacity => {
                write!(f, "runtime event stream capacity must be greater than zero")
            }
            Self::LosslessBackpressure {
                capacity,
                sequence,
                kind,
            } => write!(
                f,
                "runtime event stream backpressure blocked lossless event {kind:?} at sequence {sequence} with capacity {capacity}"
            ),
        }
    }
}

impl std::error::Error for RuntimeEventStreamError {}

/// Bounded local-owner event stream.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeEventStream {
    capacity: NonZeroUsize,
    next_sequence: u64,
    dropped_best_effort: u64,
    queue: VecDeque<RuntimeEventEnvelope>,
}

impl Default for RuntimeEventStream {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeEventStream {
    pub const DEFAULT_CAPACITY: usize = 1024;

    #[must_use]
    pub fn new() -> Self {
        match Self::with_capacity(Self::DEFAULT_CAPACITY) {
            Ok(stream) => stream,
            Err(_) => unreachable!("default event stream capacity is non-zero"),
        }
    }

    pub fn with_capacity(capacity: usize) -> Result<Self, RuntimeEventStreamError> {
        let capacity = NonZeroUsize::new(capacity).ok_or(RuntimeEventStreamError::ZeroCapacity)?;
        Ok(Self {
            capacity,
            next_sequence: 0,
            dropped_best_effort: 0,
            queue: VecDeque::with_capacity(capacity.get()),
        })
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    #[must_use]
    pub fn dropped_best_effort(&self) -> u64 {
        self.dropped_best_effort
    }

    pub fn publish(
        &mut self,
        kind: RuntimeEventKind,
        delivery: RuntimeEventDelivery,
        payload: RuntimeOwnerEventPayload,
    ) -> Result<u64, RuntimeEventStreamError> {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);

        if self.queue.len() == self.capacity() && !self.make_room_for(delivery, sequence, kind)? {
            return Ok(sequence);
        }

        self.queue.push_back(RuntimeEventEnvelope::new(
            sequence,
            sequence_kind(kind, &payload),
            delivery,
            payload,
        ));
        Ok(sequence)
    }

    pub fn publish_runtime(&mut self, event: RuntimeEvent) -> Result<u64, RuntimeEventStreamError> {
        let classification = classify_runtime_event(&event);
        self.publish(
            classification.kind,
            classification.delivery,
            RuntimeOwnerEventPayload::Runtime(event),
        )
    }

    pub fn publish_lossless(
        &mut self,
        kind: RuntimeEventKind,
        payload: RuntimeOwnerEventPayload,
    ) -> Result<u64, RuntimeEventStreamError> {
        self.publish(kind, RuntimeEventDelivery::Lossless, payload)
    }

    pub fn publish_best_effort(
        &mut self,
        kind: RuntimeEventKind,
        payload: RuntimeOwnerEventPayload,
    ) -> Result<u64, RuntimeEventStreamError> {
        self.publish(kind, RuntimeEventDelivery::BestEffort, payload)
    }

    #[must_use]
    pub fn subscribe(&self) -> RuntimeEventSubscriber {
        RuntimeEventSubscriber::default()
    }

    pub fn drain_for(
        &mut self,
        subscriber: &mut RuntimeEventSubscriber,
    ) -> Vec<RuntimeEventStreamItem> {
        let mut items = Vec::new();
        if self.dropped_best_effort > subscriber.observed_dropped_best_effort {
            let dropped = self
                .dropped_best_effort
                .saturating_sub(subscriber.observed_dropped_best_effort);
            subscriber.observed_dropped_best_effort = self.dropped_best_effort;
            let next_sequence = self
                .queue
                .front()
                .map_or(self.next_sequence, |event| event.sequence);
            items.push(RuntimeEventStreamItem::Lagged {
                dropped,
                next_sequence,
            });
        }

        while let Some(envelope) = self.queue.pop_front() {
            subscriber.next_sequence = envelope.sequence.saturating_add(1);
            items.push(RuntimeEventStreamItem::Event(envelope));
        }
        items
    }

    fn make_room_for(
        &mut self,
        delivery: RuntimeEventDelivery,
        sequence: u64,
        kind: RuntimeEventKind,
    ) -> Result<bool, RuntimeEventStreamError> {
        if let Some(drop_index) = self
            .queue
            .iter()
            .position(|event| event.delivery == RuntimeEventDelivery::BestEffort)
        {
            self.queue.remove(drop_index);
            self.dropped_best_effort = self.dropped_best_effort.saturating_add(1);
            return Ok(true);
        }

        if delivery == RuntimeEventDelivery::BestEffort {
            self.dropped_best_effort = self.dropped_best_effort.saturating_add(1);
            return Ok(false);
        }

        Err(RuntimeEventStreamError::LosslessBackpressure {
            capacity: self.capacity(),
            sequence,
            kind,
        })
    }
}

/// Subscriber cursor for the bounded owner stream.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEventSubscriber {
    next_sequence: u64,
    observed_dropped_best_effort: u64,
}

impl RuntimeEventSubscriber {
    #[must_use]
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnerEventClassification {
    pub kind: RuntimeEventKind,
    pub delivery: RuntimeEventDelivery,
}

impl OwnerEventClassification {
    const fn lossless(kind: RuntimeEventKind) -> Self {
        Self {
            kind,
            delivery: RuntimeEventDelivery::Lossless,
        }
    }

    const fn best_effort(kind: RuntimeEventKind) -> Self {
        Self {
            kind,
            delivery: RuntimeEventDelivery::BestEffort,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolProjection {
    LocalRuntimeBridge,
    OwnerPayload,
    Unsupported,
}

/// Matrix entry from `vac_protocol::EventMsg` to owner event and temporary
/// TUI compatibility expectations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolEventMapping {
    pub event_type: &'static str,
    pub owner: OwnerEventClassification,
    pub projection: ProtocolProjection,
    pub tui_visible: bool,
    pub retirement_blocker: Option<&'static str>,
}

#[must_use]
pub fn classify_protocol_event(msg: &EventMsg) -> ProtocolEventMapping {
    use EventMsg::*;
    match msg {
        TurnStarted(_) => mapping(
            "TurnStarted",
            OwnerEventClassification::lossless(RuntimeEventKind::TurnStarted),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        TurnComplete(_) => mapping(
            "TurnComplete",
            OwnerEventClassification::lossless(RuntimeEventKind::TurnCompleted),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        TurnAborted(_) => mapping(
            "TurnAborted",
            OwnerEventClassification::lossless(RuntimeEventKind::TurnInterrupted),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        Error(_) | StreamError(_) => mapping(
            "Error/StreamError",
            OwnerEventClassification::lossless(RuntimeEventKind::TurnFailed),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        AgentMessageContentDelta(_) | AgentMessage(_) => mapping(
            "AgentMessage",
            OwnerEventClassification::lossless(RuntimeEventKind::AssistantDelta),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        PlanDelta(_) | PlanUpdate(_) => mapping(
            "PlanDelta/PlanUpdate",
            OwnerEventClassification::lossless(RuntimeEventKind::PlanDelta),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        ReasoningContentDelta(_)
        | ReasoningRawContentDelta(_)
        | AgentReasoning(_)
        | AgentReasoningRawContent(_)
        | AgentReasoningSectionBreak(_) => mapping(
            "Reasoning",
            OwnerEventClassification::lossless(RuntimeEventKind::ReasoningDelta),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        ExecCommandBegin(_)
        | McpToolCallBegin(_)
        | WebSearchBegin(_)
        | ImageGenerationBegin(_)
        | PatchApplyBegin(_)
        | HookStarted(_) => mapping(
            "ToolStarted",
            OwnerEventClassification::lossless(RuntimeEventKind::ToolStarted),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        ExecCommandEnd(_)
        | McpToolCallEnd(_)
        | WebSearchEnd(_)
        | ImageGenerationEnd(_)
        | PatchApplyEnd(_)
        | HookCompleted(_) => mapping(
            "ToolFinished",
            OwnerEventClassification::lossless(RuntimeEventKind::ToolFinished),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        ExecCommandOutputDelta(_) => mapping(
            "ExecCommandOutputDelta",
            OwnerEventClassification::lossless(RuntimeEventKind::ExecOutputDelta),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        TerminalInteraction(_) => mapping(
            "TerminalInteraction",
            OwnerEventClassification::lossless(RuntimeEventKind::TerminalInteraction),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        PatchApplyUpdated(_) | TurnDiff(_) => mapping(
            "PatchProgress",
            OwnerEventClassification::best_effort(RuntimeEventKind::PatchProgress),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        ExecApprovalRequest(_)
        | ApplyPatchApprovalRequest(_)
        | RequestPermissions(_)
        | ElicitationRequest(_) => mapping(
            "ApprovalRequested",
            OwnerEventClassification::lossless(RuntimeEventKind::ApprovalRequested),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        RequestUserInput(_) => mapping(
            "RequestUserInput",
            OwnerEventClassification::lossless(RuntimeEventKind::UserInputRequested),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        ThreadNameUpdated(_) | ThreadGoalUpdated(_) => mapping(
            "ThreadMetadataUpdated",
            OwnerEventClassification::lossless(RuntimeEventKind::ThreadMetadataUpdated),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        RealtimeConversationStarted(_) => mapping(
            "RealtimeConversationStarted",
            OwnerEventClassification::lossless(RuntimeEventKind::RealtimeStarted),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        RealtimeConversationClosed(_) => mapping(
            "RealtimeConversationClosed",
            OwnerEventClassification::lossless(RuntimeEventKind::RealtimeClosed),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        ShutdownComplete => mapping(
            "ShutdownComplete",
            OwnerEventClassification::lossless(RuntimeEventKind::Shutdown),
            ProtocolProjection::OwnerPayload,
            true,
            None,
        ),
        EnteredReviewMode(_) | ExitedReviewMode(_) => mapping(
            "ReviewMode",
            OwnerEventClassification::lossless(RuntimeEventKind::ReviewMode),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        ItemStarted(_)
        | ItemCompleted(_)
        | RawResponseItem(_)
        | DynamicToolCallResponse(_)
        | CollabAgentSpawnBegin(_)
        | CollabAgentSpawnEnd(_)
        | CollabAgentInteractionBegin(_)
        | CollabAgentInteractionEnd(_)
        | CollabWaitingBegin(_)
        | CollabWaitingEnd(_)
        | CollabCloseBegin(_)
        | CollabCloseEnd(_)
        | CollabResumeBegin(_)
        | CollabResumeEnd(_) => mapping(
            "ItemLifecycle",
            OwnerEventClassification::lossless(RuntimeEventKind::ToolStarted),
            ProtocolProjection::LocalRuntimeBridge,
            true,
            None,
        ),
        Warning(_)
        | GuardianWarning(_)
        | GuardianAssessment(_)
        | ModelReroute(_)
        | ModelVerification(_)
        | TokenCount(_)
        | McpStartupUpdate(_)
        | McpStartupComplete(_)
        | DeprecationNotice(_)
        | SkillsUpdateAvailable => mapping(
            "StatusWarningOrTelemetry",
            OwnerEventClassification::best_effort(RuntimeEventKind::Unsupported),
            ProtocolProjection::Unsupported,
            true,
            Some("owner-specific status/warning payloads are deferred to protocol retirement"),
        ),
        RealtimeConversationRealtime(_)
        | RealtimeConversationSdp(_)
        | RealtimeConversationListVoicesResponse(_) => mapping(
            "RealtimePayload",
            OwnerEventClassification::best_effort(RuntimeEventKind::Unsupported),
            ProtocolProjection::Unsupported,
            true,
            Some("realtime payload DTOs remain behind the temporary TUI adapter"),
        ),
        ContextCompacted(_)
        | ThreadRolledBack(_)
        | SessionConfigured(_)
        | UserMessage(_)
        | ViewImageToolCall(_)
        | DynamicToolCallRequest(_)
        | GetHistoryEntryResponse(_)
        | McpListToolsResponse(_)
        | ListSkillsResponse(_) => mapping(
            "UnsupportedOrNonStreaming",
            OwnerEventClassification::best_effort(RuntimeEventKind::Unsupported),
            ProtocolProjection::Unsupported,
            false,
            Some(
                "non-streaming/control response; no final owner stream payload needed for Plan 29",
            ),
        ),
    }
}

const fn mapping(
    event_type: &'static str,
    owner: OwnerEventClassification,
    projection: ProtocolProjection,
    tui_visible: bool,
    retirement_blocker: Option<&'static str>,
) -> ProtocolEventMapping {
    ProtocolEventMapping {
        event_type,
        owner,
        projection,
        tui_visible,
        retirement_blocker,
    }
}

#[must_use]
pub fn classify_runtime_event(event: &RuntimeEvent) -> OwnerEventClassification {
    match event {
        RuntimeEvent::SessionStarted(_) | RuntimeEvent::TaskStarted(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::TurnStarted)
        }
        RuntimeEvent::SessionEnded(_) | RuntimeEvent::TaskCompleted(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::TurnCompleted)
        }
        RuntimeEvent::AssistantDelta(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::AssistantDelta)
        }
        RuntimeEvent::ToolCallStarted(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::ToolStarted)
        }
        RuntimeEvent::ToolCallFinished(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::ToolFinished)
        }
        RuntimeEvent::ApprovalRequested(_) | RuntimeEvent::ApprovalResolved(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::ApprovalRequested)
        }
        RuntimeEvent::ValidationStarted(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::ValidationStarted)
        }
        RuntimeEvent::ValidationFinished(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::ValidationFinished)
        }
        RuntimeEvent::TaskFailed(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::TurnFailed)
        }
        RuntimeEvent::TaskCancelled(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::TurnInterrupted)
        }
        RuntimeEvent::EnteredReviewMode(_) | RuntimeEvent::ExitedReviewMode(_) => {
            OwnerEventClassification::lossless(RuntimeEventKind::ReviewMode)
        }
    }
}

/// Stateful projector that owns the Plan 29 protocol-to-owner stream path.
pub struct RuntimeOwnerEventProjector {
    bridge: LocalRuntimeBridge,
    stream: RuntimeEventStream,
}

impl RuntimeOwnerEventProjector {
    #[must_use]
    pub fn new(bridge: LocalRuntimeBridge, stream: RuntimeEventStream) -> Self {
        Self { bridge, stream }
    }

    #[must_use]
    pub fn stream(&self) -> &RuntimeEventStream {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut RuntimeEventStream {
        &mut self.stream
    }

    pub fn accept_core_event(
        &mut self,
        event: Event,
    ) -> Result<BridgeOutput, RuntimeEventStreamError> {
        let mapping = classify_protocol_event(&event.msg);
        if let Some(payload) = owner_payload_for_protocol_event(&event.msg) {
            self.stream
                .publish(mapping.owner.kind, mapping.owner.delivery, payload)?;
            return Ok(BridgeOutput {
                events: Vec::new(),
                error_seen: matches!(mapping.owner.kind, RuntimeEventKind::TurnFailed),
                terminate: matches!(
                    mapping.owner.kind,
                    RuntimeEventKind::TurnCompleted
                        | RuntimeEventKind::TurnInterrupted
                        | RuntimeEventKind::TurnFailed
                        | RuntimeEventKind::Shutdown
                ),
            });
        }

        let output = self.bridge.map_core_event(event);
        for runtime_event in output.events.iter().cloned() {
            self.stream.publish_runtime(runtime_event)?;
        }
        Ok(output)
    }
}

fn owner_payload_for_protocol_event(msg: &EventMsg) -> Option<RuntimeOwnerEventPayload> {
    match msg {
        EventMsg::ExecCommandOutputDelta(event) => {
            Some(RuntimeOwnerEventPayload::ExecOutputDelta {
                call_id: event.call_id.clone(),
                stream: format!("{:?}", event.stream),
                chunk: event.chunk.clone(),
            })
        }
        EventMsg::TerminalInteraction(event) => {
            Some(RuntimeOwnerEventPayload::TerminalInteraction {
                call_id: event.call_id.clone(),
                process_id: event.process_id.clone(),
                stdin: event.stdin.clone(),
            })
        }
        EventMsg::PatchApplyUpdated(event) => Some(RuntimeOwnerEventPayload::PatchProgress {
            call_id: event.call_id.clone(),
            changed_files: event.changes.len(),
        }),
        EventMsg::TurnDiff(event) => Some(RuntimeOwnerEventPayload::PatchProgress {
            call_id: "turn_diff".to_string(),
            changed_files: usize::from(!event.unified_diff.trim().is_empty()),
        }),
        EventMsg::RequestUserInput(event) => Some(RuntimeOwnerEventPayload::UserInputRequested {
            turn_id: event.turn_id.clone(),
            call_id: event.call_id.clone(),
            question_count: event.questions.len(),
        }),
        EventMsg::ThreadNameUpdated(event) => Some(RuntimeOwnerEventPayload::ThreadNameUpdated {
            thread_id: event.thread_id.to_string(),
            thread_name: event.thread_name.clone(),
        }),
        EventMsg::ThreadGoalUpdated(event) => Some(RuntimeOwnerEventPayload::ThreadGoalUpdated {
            thread_id: event.thread_id.to_string(),
            turn_id: event.turn_id.clone(),
        }),
        EventMsg::RealtimeConversationStarted(event) => {
            Some(RuntimeOwnerEventPayload::RealtimeStarted {
                realtime_session_id: event.realtime_session_id.clone(),
                version: format!("{:?}", event.version),
            })
        }
        EventMsg::RealtimeConversationClosed(event) => {
            Some(RuntimeOwnerEventPayload::RealtimeClosed {
                reason: event.reason.clone(),
            })
        }
        EventMsg::Error(event) if event.affects_turn_status() => {
            Some(RuntimeOwnerEventPayload::Failure {
                message: event.message.clone(),
            })
        }
        EventMsg::StreamError(event)
            if event
                .vac_error_info
                .as_ref()
                .is_none_or(vac_protocol::protocol::VACErrorInfo::affects_turn_status) =>
        {
            Some(RuntimeOwnerEventPayload::Failure {
                message: event.message.clone(),
            })
        }
        EventMsg::ShutdownComplete => Some(RuntimeOwnerEventPayload::Shutdown),
        _ => None,
    }
}

fn sequence_kind(kind: RuntimeEventKind, payload: &RuntimeOwnerEventPayload) -> RuntimeEventKind {
    match payload {
        RuntimeOwnerEventPayload::Runtime(event) => classify_runtime_event(event).kind,
        _ => kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use vac_core::local_runtime::AssistantDelta;
    use vac_core::local_runtime::SessionId;
    use vac_core::local_runtime::TaskCompleted;
    use vac_core::local_runtime::TaskId;
    use vac_protocol::config_types::ModeKind;
    use vac_protocol::protocol::AgentMessageContentDeltaEvent;
    use vac_protocol::protocol::ErrorEvent;
    use vac_protocol::protocol::ExecCommandOutputDeltaEvent;
    use vac_protocol::protocol::ExecOutputStream;
    use vac_protocol::protocol::TerminalInteractionEvent;
    use vac_protocol::protocol::TurnCompleteEvent;
    use vac_protocol::protocol::TurnStartedEvent;

    fn task_id() -> TaskId {
        TaskId::from_string("22222222-2222-2222-2222-222222222222").expect("valid task id")
    }

    fn session_id() -> SessionId {
        SessionId::from_string("11111111-1111-1111-1111-111111111111").expect("valid session id")
    }

    fn delta(text: &str) -> RuntimeOwnerEventPayload {
        RuntimeOwnerEventPayload::Runtime(RuntimeEvent::AssistantDelta(AssistantDelta::new(
            task_id(),
            text,
        )))
    }

    fn completed() -> RuntimeOwnerEventPayload {
        RuntimeOwnerEventPayload::Runtime(RuntimeEvent::TaskCompleted(TaskCompleted::new(
            task_id(),
            Some("done".to_string()),
            Vec::new(),
        )))
    }

    fn core_event(msg: EventMsg) -> Event {
        Event {
            id: "turn-1".to_string(),
            msg,
        }
    }

    fn projector(capacity: usize) -> RuntimeOwnerEventProjector {
        RuntimeOwnerEventProjector::new(
            LocalRuntimeBridge::new(session_id(), task_id(), "do work".to_string()),
            RuntimeEventStream::with_capacity(capacity).expect("capacity is valid"),
        )
    }

    #[test]
    fn best_effort_events_drop_with_lag_marker_under_backpressure() {
        let mut stream = RuntimeEventStream::with_capacity(2).expect("capacity is valid");
        stream
            .publish_best_effort(RuntimeEventKind::AssistantDelta, delta("one"))
            .expect("best effort event can enqueue");
        stream
            .publish_best_effort(RuntimeEventKind::PlanDelta, delta("two"))
            .expect("best effort event can enqueue");
        stream
            .publish_best_effort(RuntimeEventKind::ReasoningDelta, delta("three"))
            .expect("best effort event can replace older best effort event");

        let mut subscriber = stream.subscribe();
        let items = stream.drain_for(&mut subscriber);

        assert_eq!(stream.dropped_best_effort(), 1);
        assert!(matches!(
            items.as_slice(),
            [
                RuntimeEventStreamItem::Lagged {
                    dropped: 1,
                    next_sequence: 1
                },
                RuntimeEventStreamItem::Event(RuntimeEventEnvelope { sequence: 1, .. }),
                RuntimeEventStreamItem::Event(RuntimeEventEnvelope { sequence: 2, .. }),
            ]
        ));
        assert_eq!(subscriber.next_sequence(), 3);
    }

    #[test]
    fn lossless_events_are_not_silently_dropped() {
        let mut stream = RuntimeEventStream::with_capacity(1).expect("capacity is valid");
        stream
            .publish_lossless(RuntimeEventKind::TurnCompleted, completed())
            .expect("first lossless event can enqueue");

        let error = stream
            .publish_lossless(RuntimeEventKind::TurnFailed, completed())
            .expect_err("second lossless event must fail loudly under backpressure");

        assert_eq!(stream.len(), 1);
        assert!(matches!(
            error,
            RuntimeEventStreamError::LosslessBackpressure {
                capacity: 1,
                sequence: 1,
                kind: RuntimeEventKind::TurnFailed,
            }
        ));
    }

    #[test]
    fn incoming_best_effort_event_drops_when_queue_is_full_of_lossless_events() {
        let mut stream = RuntimeEventStream::with_capacity(1).expect("capacity is valid");
        stream
            .publish_lossless(RuntimeEventKind::TurnCompleted, completed())
            .expect("lossless event can enqueue");
        stream
            .publish_best_effort(RuntimeEventKind::AssistantDelta, delta("transient"))
            .expect("best effort event can drop itself instead of overflowing");

        let mut subscriber = stream.subscribe();
        let items = stream.drain_for(&mut subscriber);

        assert_eq!(stream.dropped_best_effort(), 1);
        assert!(matches!(
            items.as_slice(),
            [
                RuntimeEventStreamItem::Lagged {
                    dropped: 1,
                    next_sequence: 0
                },
                RuntimeEventStreamItem::Event(RuntimeEventEnvelope {
                    sequence: 0,
                    kind: RuntimeEventKind::TurnCompleted,
                    delivery: RuntimeEventDelivery::Lossless,
                    ..
                }),
            ]
        ));
    }

    #[test]
    fn incoming_lossless_event_may_displace_best_effort_event() {
        let mut stream = RuntimeEventStream::with_capacity(1).expect("capacity is valid");
        stream
            .publish_best_effort(RuntimeEventKind::AssistantDelta, delta("transient"))
            .expect("best effort event can enqueue");
        stream
            .publish_lossless(RuntimeEventKind::TurnCompleted, completed())
            .expect("lossless event can displace best effort event");

        let mut subscriber = stream.subscribe();
        let items = stream.drain_for(&mut subscriber);

        assert!(matches!(
            items.as_slice(),
            [
                RuntimeEventStreamItem::Lagged {
                    dropped: 1,
                    next_sequence: 1
                },
                RuntimeEventStreamItem::Event(RuntimeEventEnvelope {
                    sequence: 1,
                    kind: RuntimeEventKind::TurnCompleted,
                    delivery: RuntimeEventDelivery::Lossless,
                    ..
                }),
            ]
        ));
    }

    #[test]
    fn protocol_matrix_marks_required_events_lossless_and_visible() {
        let required = [
            EventMsg::TurnStarted(TurnStartedEvent {
                turn_id: "turn-1".to_string(),
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: ModeKind::default(),
            }),
            EventMsg::AgentMessageContentDelta(AgentMessageContentDeltaEvent {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "item-1".to_string(),
                delta: "hello".to_string(),
            }),
            EventMsg::ExecCommandOutputDelta(ExecCommandOutputDeltaEvent {
                call_id: "call-1".to_string(),
                stream: ExecOutputStream::Stdout,
                chunk: b"output".to_vec(),
            }),
            EventMsg::TerminalInteraction(TerminalInteractionEvent {
                call_id: "call-1".to_string(),
                process_id: "pty-1".to_string(),
                stdin: "q".to_string(),
            }),
            EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id: "turn-1".to_string(),
                last_agent_message: Some("done".to_string()),
                completed_at: None,
                duration_ms: Some(Duration::from_millis(7).as_millis() as i64),
                time_to_first_token_ms: None,
            }),
        ];

        for msg in required {
            let mapping = classify_protocol_event(&msg);
            assert_eq!(mapping.owner.delivery, RuntimeEventDelivery::Lossless);
            assert!(
                mapping.tui_visible,
                "{} should be TUI-visible",
                mapping.event_type
            );
        }
    }

    #[test]
    fn projector_reuses_bridge_and_preserves_terminal_payloads() {
        let mut projector = projector(8);
        projector
            .accept_core_event(core_event(EventMsg::TurnStarted(TurnStartedEvent {
                turn_id: "turn-1".to_string(),
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: ModeKind::default(),
            })))
            .expect("turn start projects");
        projector
            .accept_core_event(core_event(EventMsg::AgentMessageContentDelta(
                AgentMessageContentDeltaEvent {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "item-1".to_string(),
                    delta: "hello".to_string(),
                },
            )))
            .expect("assistant delta projects");
        projector
            .accept_core_event(core_event(EventMsg::ExecCommandOutputDelta(
                ExecCommandOutputDeltaEvent {
                    call_id: "call-1".to_string(),
                    stream: ExecOutputStream::Stdout,
                    chunk: b"output".to_vec(),
                },
            )))
            .expect("terminal output projects");

        let mut subscriber = projector.stream().subscribe();
        let items = projector.stream_mut().drain_for(&mut subscriber);

        assert!(items.iter().any(|item| matches!(
            item,
            RuntimeEventStreamItem::Event(RuntimeEventEnvelope {
                kind: RuntimeEventKind::AssistantDelta,
                payload: RuntimeOwnerEventPayload::Runtime(RuntimeEvent::AssistantDelta(_)),
                ..
            })
        )));
        assert!(items.iter().any(|item| matches!(
            item,
            RuntimeEventStreamItem::Event(RuntimeEventEnvelope {
                kind: RuntimeEventKind::ExecOutputDelta,
                payload: RuntimeOwnerEventPayload::ExecOutputDelta { call_id, chunk, .. },
                ..
            }) if call_id == "call-1" && chunk == b"output"
        )));
    }

    #[test]
    fn projector_fails_loudly_when_lossless_terminal_event_cannot_enqueue() {
        let mut projector = projector(1);
        projector
            .accept_core_event(core_event(EventMsg::TurnStarted(TurnStartedEvent {
                turn_id: "turn-1".to_string(),
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: ModeKind::default(),
            })))
            .expect("turn start enqueues lossless event");

        let result = projector.accept_core_event(core_event(EventMsg::ExecCommandOutputDelta(
            ExecCommandOutputDeltaEvent {
                call_id: "call-1".to_string(),
                stream: ExecOutputStream::Stdout,
                chunk: b"output".to_vec(),
            },
        )));

        assert!(matches!(
            result,
            Err(RuntimeEventStreamError::LosslessBackpressure {
                capacity: 1,
                kind: RuntimeEventKind::ExecOutputDelta,
                ..
            })
        ));
    }

    #[test]
    fn compatibility_adapter_marks_native_and_legacy_paths() {
        let native = RuntimeEventEnvelope::new(
            7,
            RuntimeEventKind::ExecOutputDelta,
            RuntimeEventDelivery::Lossless,
            RuntimeOwnerEventPayload::ExecOutputDelta {
                call_id: "call-1".to_string(),
                stream: "Stdout".to_string(),
                chunk: b"output".to_vec(),
            },
        );
        assert_eq!(
            native.to_tui_compatibility_event(),
            RuntimeTuiCompatibilityEvent::NativeOwnerEvent {
                kind: RuntimeEventKind::ExecOutputDelta,
                delivery: RuntimeEventDelivery::Lossless,
            }
        );

        let legacy = RuntimeEventEnvelope::new(
            8,
            RuntimeEventKind::Unsupported,
            RuntimeEventDelivery::BestEffort,
            RuntimeOwnerEventPayload::Unsupported {
                event_type: "RealtimePayload".to_string(),
                reason: "temporary adapter".to_string(),
            },
        );
        assert_eq!(
            legacy.to_tui_compatibility_event(),
            RuntimeTuiCompatibilityEvent::LegacyAdapterRequired {
                kind: RuntimeEventKind::Unsupported,
                reason: "temporary adapter".to_string(),
            }
        );
    }

    #[test]
    fn projector_surfaces_error_as_terminal_failure_payload() {
        let mut projector = projector(4);
        let output = projector
            .accept_core_event(core_event(EventMsg::Error(ErrorEvent {
                message: "boom".to_string(),
                vac_error_info: None,
            })))
            .expect("error payload enqueues");

        assert!(output.error_seen);
        assert!(output.terminate);
        let mut subscriber = projector.stream().subscribe();
        let items = projector.stream_mut().drain_for(&mut subscriber);
        assert!(matches!(
            items.as_slice(),
            [RuntimeEventStreamItem::Event(RuntimeEventEnvelope {
                kind: RuntimeEventKind::TurnFailed,
                delivery: RuntimeEventDelivery::Lossless,
                payload: RuntimeOwnerEventPayload::Failure { message },
                ..
            })] if message == "boom"
        ));
    }
}
