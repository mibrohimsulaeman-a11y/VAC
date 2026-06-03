// Submit primitives for the root TUI runtime adapter.
//
// Mints `RuntimeCommand::StartTask`, opens session/task pairs, and exposes
// `RuntimeSubmitPlan`. Also includes `default_autonomy_mode` and the legacy
// compat transport marker.

#![allow(unused_imports)]

use std::path::PathBuf;
use vac_core::local_runtime::AutonomyMode;
use vac_core::local_runtime::RuntimeCommand;
use vac_core::local_runtime::RuntimeEntrypoint;
use vac_core::local_runtime::RuntimeSession;
use vac_core::local_runtime::RuntimeTask;
use vac_core::local_runtime::RuntimeTaskKind;
use vac_core::local_runtime::StartTask;

/// Build a [`RuntimeCommand::StartTask`] for a fresh local prompt submission.
///
/// Used by the bottom-input Enter path in `chatwidget` so that every prompt
/// is also expressed as a Local Runtime Contract command, not only as the
/// legacy `AppCommand::UserTurn` submission. The returned command is the
/// canonical product-level intent for downstream activity/progress/
/// approval/validation projections.
pub(crate) fn mint_start_task(
    prompt: impl Into<String>,
    cwd: impl Into<PathBuf>,
    autonomy_mode: AutonomyMode,
) -> RuntimeCommand {
    RuntimeCommand::start_task(prompt, autonomy_mode, RuntimeEntrypoint::Tui, cwd)
}

/// Materialize a fresh [`RuntimeSession`] / [`RuntimeTask`] pair for the
/// supplied [`StartTask`]. The TUI does not yet persist these, but emitting
/// them on submit gives the rest of the runtime a single, stable id to attach
/// activity, approval and validation events to.
pub(crate) fn open_session_and_task(start: &StartTask) -> (RuntimeSession, RuntimeTask) {
    let session = RuntimeSession::new(start.cwd.clone(), start.entrypoint, start.autonomy_mode);
    let task = RuntimeTask::new(
        session.id,
        RuntimeTaskKind::SemanticCoding,
        start.prompt.clone(),
    );
    (session, task)
}

/// Marker for the legacy transport carrying a runtime-first submission.
///
/// Step 00D-5 makes [`RuntimeCommand::StartTask`] the canonical product-level
/// intent for the root `vac` TUI bottom-input Enter path. The legacy
/// `AppCommand::UserTurn` op that the chatwidget submit path eventually
/// constructs is no longer the source-of-truth — it is a *transport* that
/// carries the runtime-first submission across the in-process app-server
/// boundary so the existing execution machinery keeps working without a
/// wholesale backend rewrite.
///
/// Step 00E retires this transport by routing
/// [`RuntimeCommand::StartTask`] directly into the local runtime driver
/// (`vac-exec`) so the `UserTurn` reachability disappears entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyCompatTransport {
    /// Carry the runtime-first submission over the legacy
    /// `AppCommand::UserTurn` op as a temporary compat transport.
    UserTurn,
}

/// Runtime-first submission plan for the root `vac` TUI bottom-input Enter.
///
/// **Canonical input is [`RuntimeCommand::StartTask`].** The legacy
/// `AppCommand::UserTurn` op the chatwidget submit path eventually builds is
/// documented as a temporary compatibility transport behind
/// [`LegacyCompatTransport`]. Step 00E retires the legacy transport.
///
/// The plan is intentionally thin: it owns the canonical [`StartTask`], the
/// derived [`RuntimeSession`]/[`RuntimeTask`] pair (so activity, approval,
/// validation and evidence projections share stable ids), and the compat
/// transport marker. Naming and control-flow at every caller MUST keep the
/// runtime command as the source-of-truth: the legacy op is *derived from*
/// the plan, never the other way around.
#[derive(Debug, Clone)]
pub(crate) struct RuntimeSubmitPlan {
    pub(crate) start_task: StartTask,
    pub(crate) session: RuntimeSession,
    pub(crate) task: RuntimeTask,
    pub(crate) legacy_compat_transport: LegacyCompatTransport,
}

impl RuntimeSubmitPlan {
    /// Build a runtime-first submission plan from the canonical
    /// [`RuntimeCommand::StartTask`]. Returns `None` if the supplied command
    /// is any other variant — that would mean a caller bypassed
    /// [`mint_start_task`] and tried to submit a non-canonical input, and
    /// the chatwidget submit path treats that as an error rather than
    /// silently falling back to the legacy transport.
    pub(crate) fn from_runtime_command(command: RuntimeCommand) -> Option<Self> {
        match command {
            RuntimeCommand::StartTask(start_task) => Some(Self::from_start_task(start_task)),
            _ => None,
        }
    }

    /// Build the plan directly from a [`StartTask`]. Equivalent to wrapping
    /// the value in [`RuntimeCommand::StartTask`] and calling
    /// [`RuntimeSubmitPlan::from_runtime_command`].
    pub(crate) fn from_start_task(start_task: StartTask) -> Self {
        let (session, task) = open_session_and_task(&start_task);
        Self {
            start_task,
            session,
            task,
            legacy_compat_transport: LegacyCompatTransport::UserTurn,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn prompt(&self) -> &str {
        &self.start_task.prompt
    }

    #[allow(dead_code)]
    pub(crate) fn cwd(&self) -> &std::path::Path {
        &self.start_task.cwd
    }

    /// One-line `tracing::info` payload describing the runtime-first
    /// submission. Logged from the chatwidget submit path so PTY/dogfood
    /// logs prove the canonical contract input — not the legacy transport —
    /// fired for a real operator prompt.
    pub(crate) fn trace(&self) -> String {
        start_trace(&self.session, &self.task, &self.start_task.prompt)
    }
}

/// A short, stable trace string for the local runtime start. Logged at
/// `tracing::info` from the chatwidget submit path so PTY/dogfood logs prove
/// the contract was engaged for a real operator prompt.
pub(crate) fn start_trace(session: &RuntimeSession, task: &RuntimeTask, prompt: &str) -> String {
    format!(
        "local-runtime: session={} task={} entrypoint={} autonomy={} prompt_len={}",
        session.id,
        task.id,
        session.entrypoint,
        session.autonomy_mode,
        prompt.chars().count(),
    )
}

/// Resolve an autonomy mode for the current TUI submission. The TUI does not
/// yet expose an explicit autonomy switch, so we conservatively return
/// [`AutonomyMode::Assist`] (operator-in-the-loop) which matches the existing
/// approval-surface defaults.
pub(crate) fn default_autonomy_mode() -> AutonomyMode {
    AutonomyMode::Assist
}
