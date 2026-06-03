// Approval-control-flow helpers and process-wide ApprovalId correlation.

#![allow(unused_imports)]

use vac_core::local_runtime::ApprovalAction;
use vac_core::local_runtime::ApprovalDecision;
use vac_core::local_runtime::ApprovalId;
use vac_core::local_runtime::ApprovalResolved;
use vac_core::local_runtime::RuntimeCommand;
use vac_core::local_runtime::RuntimeEvent;
use vac_core::local_runtime::SessionId;
use vac_core::local_runtime::TaskId;

use crate::session_protocol::CommandExecutionApprovalDecision;
use crate::session_protocol::FileChangeApprovalDecision;

/// Record an approval decision originating from the existing root approval
/// surface and return the corresponding [`RuntimeCommand`]. This is the
/// single point that maps the existing approve/reject UI into the contract.
/// Correlation status of a contract-side approval resolution.
///
/// `Correlated` means the resolve fn found a matching `ApprovalId` recorded
/// by the originating projection (`ApprovalRequested.id` == `ApprovalResolved.id`).
/// `Fallback` means no such mapping existed at resolve time, so a fresh
/// `ApprovalId` was minted and a `tracing::warn!` was logged. Callers should
/// label fallback evidence as uncorrelated rather than treating it as a
/// trusted correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalCorrelation {
    Correlated,
    Fallback,
}

/// Internal: resolve an approval entry by registry key. Consumes the
/// `key -> ApprovalId` mapping recorded by the originating projection so the
/// returned [`RuntimeCommand`] and [`RuntimeEvent::ApprovalResolved`] carry
/// the **same** `ApprovalId`. When no mapping exists, mints a fresh
/// `ApprovalId` and logs a `tracing::warn!`; the returned
/// [`ApprovalCorrelation::Fallback`] flags the result as uncorrelated.
fn approval_resolved_for_decision(
    key: &str,
    decision: ApprovalDecision,
) -> (RuntimeCommand, RuntimeEvent, ApprovalCorrelation) {
    let (approval_id, correlation) = match approval_registry::take(key) {
        Some(id) => (id, ApprovalCorrelation::Correlated),
        None => {
            tracing::warn!(
                target: "vac::local_runtime::approval",
                key = %key,
                "approval resolution arrived without matching ApprovalRequested entry; emitting uncorrelated fallback"
            );
            (ApprovalId::new(), ApprovalCorrelation::Fallback)
        }
    };
    let command = approval_decision_command(approval_id, decision);
    let event = RuntimeEvent::ApprovalResolved(ApprovalResolved::new(approval_id, decision, None));
    (command, event, correlation)
}

pub(crate) fn approval_decision_command(
    approval_id: ApprovalId,
    decision: ApprovalDecision,
) -> RuntimeCommand {
    match decision {
        ApprovalDecision::Approved => RuntimeCommand::Approve { approval_id },
        ApprovalDecision::Rejected | ApprovalDecision::Cancelled | ApprovalDecision::Timeout => {
            RuntimeCommand::Reject { approval_id }
        }
    }
}

/// Cancel command for the active task. Used by the existing Ctrl-C / cancel
/// affordances so they cancel the runtime task in addition to interrupting
/// the legacy thread submission.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn cancel_task_command(task_id: TaskId) -> RuntimeCommand {
    RuntimeCommand::CancelTask { task_id }
}

/// Resume command for an existing local session.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn resume_session_command(session_id: SessionId) -> RuntimeCommand {
    RuntimeCommand::ResumeSession { session_id }
}

/// Process-wide pending-approval correlation map.
///
/// `ApprovalRequested` and `ApprovalResolved` need a shared `ApprovalId` even
/// though the request originates inside [`RuntimeBridge`] (chatwidget seam)
/// while the resolve fires from the approval overlay (`bottom_pane`), which
/// has no direct handle to the bridge. We bridge those two surfaces with a
/// process-wide map keyed by the protocol `call_id`. Both sides of the
/// roundtrip consult this map so the correlated id flows end-to-end.
pub(in crate::runtime_adapter) mod approval_registry {
    use super::ApprovalId;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::OnceLock;

    static PENDING: OnceLock<Mutex<HashMap<String, ApprovalId>>> = OnceLock::new();

    fn map() -> &'static Mutex<HashMap<String, ApprovalId>> {
        PENDING.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub(in crate::runtime_adapter) fn record(call_id: &str, id: ApprovalId) {
        if let Ok(mut m) = map().lock() {
            m.insert(call_id.to_string(), id);
        }
    }

    pub(in crate::runtime_adapter) fn take(call_id: &str) -> Option<ApprovalId> {
        map().lock().ok().and_then(|mut m| m.remove(call_id))
    }

    pub(in crate::runtime_adapter) fn drop_all(keys: &[String]) {
        if keys.is_empty() {
            return;
        }
        if let Ok(mut m) = map().lock() {
            for k in keys {
                m.remove(k);
            }
        }
    }

    #[cfg(test)]
    pub(in crate::runtime_adapter) fn contains(call_id: &str) -> bool {
        map().lock().ok().is_some_and(|m| m.contains_key(call_id))
    }
}

/// Map a TUI-local `CommandExecutionApprovalDecision` into the contract
/// `(RuntimeCommand, RuntimeEvent)` pair without requiring a `RuntimeBridge`
/// reference. Used by the existing approval overlay so each operator
/// approve/reject becomes visible runtime evidence even when the bridge
/// state lives elsewhere.
///
/// Looks up `call_id` in `approval_registry` and consumes the entry. When a
/// pending mapping exists, the returned `RuntimeCommand` and `ApprovalResolved`
/// carry the **same** `ApprovalId` minted by the originating projection — the
/// happy-path correlation. When no mapping exists (e.g. resolution arrives
/// after the task already drained), a fresh `ApprovalId` is minted as a
/// paranoia fallback; this path is **not** considered correlated and
/// downstream evidence should be labeled accordingly.
pub(crate) fn approval_resolved_for_exec_decision(
    call_id: &str,
    decision: &CommandExecutionApprovalDecision,
) -> (RuntimeCommand, RuntimeEvent, ApprovalCorrelation) {
    let resolved_decision = match decision {
        CommandExecutionApprovalDecision::Accept
        | CommandExecutionApprovalDecision::AcceptForSession => ApprovalDecision::Approved,
        CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment { .. } => {
            ApprovalDecision::Approved
        }
        CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment { .. } => {
            // Operator chose to apply an amendment; treat as approval. A future
            // slice may distinguish Allow vs Deny when the contract grows a
            // dedicated decision variant.
            ApprovalDecision::Approved
        }
        _ => ApprovalDecision::Rejected,
    };
    approval_resolved_for_decision(call_id, resolved_decision)
}

/// Map a TUI-local `FileChangeApprovalDecision` into the contract
/// `(RuntimeCommand, RuntimeEvent)` pair. See
/// [`approval_resolved_for_exec_decision`] for the correlation note.
pub(crate) fn approval_resolved_for_patch_decision(
    call_id: &str,
    decision: &FileChangeApprovalDecision,
) -> (RuntimeCommand, RuntimeEvent, ApprovalCorrelation) {
    let resolved_decision = match decision {
        FileChangeApprovalDecision::Accept | FileChangeApprovalDecision::AcceptForSession => {
            ApprovalDecision::Approved
        }
        _ => ApprovalDecision::Rejected,
    };
    approval_resolved_for_decision(call_id, resolved_decision)
}

/// Permissions approval resolution: maps the operator decision to a
/// `(RuntimeCommand, RuntimeEvent, ApprovalCorrelation)` tuple. Caller
/// translates the TUI-local `PermissionsDecision` into the contract
/// `ApprovalDecision` (typically `Approved` for any grant variant and
/// `Rejected` for `Deny`). See [`approval_resolved_for_decision`] for
/// correlation semantics.
pub(crate) fn approval_resolved_for_permissions_decision(
    call_id: &str,
    decision: ApprovalDecision,
) -> (RuntimeCommand, RuntimeEvent, ApprovalCorrelation) {
    approval_resolved_for_decision(call_id, decision)
}

/// MCP server elicitation resolution: keyed on the protocol `request_id`
/// recorded by the originating projection. See
/// [`approval_resolved_for_decision`].
pub(crate) fn approval_resolved_for_elicitation_decision(
    request_id: &str,
    decision: ApprovalDecision,
) -> (RuntimeCommand, RuntimeEvent, ApprovalCorrelation) {
    approval_resolved_for_decision(request_id, decision)
}
