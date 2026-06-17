pub mod agent;
pub mod approval;
pub mod bound_runtime;
pub mod bound_tool;
pub mod budget_context;
pub mod checkpoint;
pub mod compaction;
pub mod context;
pub mod error;
pub mod hooks;
pub mod retry;
pub mod runtime_e2e;
pub mod session_bootstrap;
pub mod stream;
pub mod tools;
pub mod types;

pub use agent::run_agent;
pub use approval::{ApprovalError, ApprovalStateMachine, ResolvedToolCall};
pub use bound_tool::{BoundRuntimeToolBoundary, BoundToolGate};
pub use budget_context::BudgetAwareContextReducer;
pub use checkpoint::{
    CHECKPOINT_FORMAT_V1, CHECKPOINT_VERSION_V1, CheckpointEnvelopeV1, CheckpointError,
    deserialize_checkpoint, serialize_checkpoint,
};
pub use compaction::{CompactionEngine, CompactionResult, PassthroughCompactionEngine};
pub use context::{
    ContextReducer, DefaultContextReducer, dedup_tool_results, merge_consecutive_same_role,
    reduce_context, remove_orphaned_tool_results, truncate_old_assistant_messages,
    truncate_old_tool_results,
};
pub use error::AgentError;
pub use hooks::AgentHook;
pub use retry::{
    RetryDelay, RetryDelaySource, exponential_backoff_ms, parse_retry_delay_from_headers,
    resolve_retry_delay_ms,
};
pub use stream::{
    IndexedStreamEvent, OrderedContentPart, StreamAssemblyError, assemble_ordered_content,
};
pub use tools::{ToolExecutionResult, ToolExecutor};
pub use types::{
    AgentCommand, AgentConfig, AgentEvent, AgentLoopResult, AgentRunContext, CompactionConfig,
    ContextConfig, ProposedToolCall, RetryConfig, SAFE_AUTOPILOT_TOOLS, StopReason, TokenUsage,
    ToolApprovalAction, ToolApprovalPolicy, ToolDecision, TurnFinishReason, strip_tool_prefix,
};

pub use bound_runtime::{
    AcceptanceCriterion, AssessmentState, BoundRuntimeConfig, BoundRuntimeController,
    BoundRuntimeE2eInput, BoundRuntimeE2eResult, BoundRuntimePhase, BoundRuntimeTraceEvent,
    CapabilityRuntimeRecord, CloseoutState, CommandApproval, CommandRisk, CompletionDisposition,
    CompletionLockResult, EvidenceState, FileOperation, GateDecision, GateOutcome, LineRange,
    ManifestSyncCloseoutState, OwnershipState, PatchAttempt, PatchBudget, PlanApproval,
    PlanFileScope, PlanStatus, PolicyDecision, ReadinessLevel, ReadinessState, ReadinessTriplet,
    RuntimeAuthority, RuntimeConformanceLevel, RuntimeGate, RuntimeJournalCloseoutState,
    RuntimeRegistrySnapshot, SemanticAnchor, SemanticPlan, SessionArtifacts, SessionState,
    SignatureAlgorithm, SignatureMode, SourceHash, SpecArtifact, SpecArtifactState, SpecSyncState,
    StructuredCommand, TaskArtifact, TaskArtifactState, TodoArtifact, TodoArtifactState, TodoItem,
    ValidationState, approval_binding_hash, canonical_json_sha256, completed_artifacts,
    evaluate_completion_lock_v1_5, successful_closeout,
};

pub use runtime_e2e::{BoundAgentE2EInput, BoundAgentE2EReport, run_bound_agent_e2e};
pub use session_bootstrap::{
    VacRuntimeBootstrapReport, VacRuntimeMetadataBootstrap, VacRuntimeMetadataBundle,
};
