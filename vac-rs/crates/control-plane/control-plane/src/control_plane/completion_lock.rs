//! Completion lock evaluation for Part V session closeout.

use serde::{Deserialize, Serialize};

use super::session_artifacts::{
    SessionArtifactState, SpecArtifact, SpecArtifactState, TaskArtifact, TaskArtifactState,
    TodoArtifact, TodoArtifactState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionLockSummary {
    pub task_ok: bool,
    pub spec_ok: bool,
    pub todo_ok: bool,
    pub open_blocking_todos: usize,
    pub unmet_acceptance_criteria: usize,
    pub missing_evidence_refs: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionLockOutcome {
    Done,
    NeedsDiscussion,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionLockDecision {
    pub outcome: CompletionLockOutcome,
    pub summary: CompletionLockSummary,
}

impl CompletionLockSummary {
    pub fn can_close(&self) -> bool {
        self.task_ok
            && self.spec_ok
            && self.todo_ok
            && self.open_blocking_todos == 0
            && self.unmet_acceptance_criteria == 0
            && self.missing_evidence_refs == 0
    }
}

pub fn evaluate_completion_lock(
    task: &TaskArtifact,
    spec: &SpecArtifact,
    todo: &TodoArtifact,
) -> CompletionLockDecision {
    let task_ok = matches!(
        task.state,
        TaskArtifactState::Done | TaskArtifactState::NeedsDiscussion
    );
    let spec_ok = matches!(
        spec.state,
        SpecArtifactState::Finalized | SpecArtifactState::NeedsDiscussion
    );
    let todo_ok = matches!(
        todo.state,
        TodoArtifactState::AllChecked | TodoArtifactState::NeedsDiscussion
    );
    let open_blocking_todos = todo
        .items
        .iter()
        .filter(|item| !item.checked && item.blocking)
        .count();
    let unmet_acceptance_criteria = task
        .acceptance_criteria
        .iter()
        .filter(|criterion| !criterion.met)
        .count();
    let missing_evidence_refs = task
        .acceptance_criteria
        .iter()
        .filter(|criterion| criterion.met && criterion.evidence.is_none())
        .count();
    let summary = CompletionLockSummary {
        task_ok,
        spec_ok,
        todo_ok,
        open_blocking_todos,
        unmet_acceptance_criteria,
        missing_evidence_refs,
    };
    let outcome = if summary.can_close() {
        CompletionLockOutcome::Done
    } else if summary.unmet_acceptance_criteria > 0 || summary.missing_evidence_refs > 0 {
        CompletionLockOutcome::NeedsDiscussion
    } else {
        CompletionLockOutcome::Open
    };
    CompletionLockDecision { outcome, summary }
}

pub fn session_close_state(decision: &CompletionLockDecision) -> SessionArtifactState {
    if decision.summary.can_close() {
        SessionArtifactState::Done
    } else if matches!(decision.outcome, CompletionLockOutcome::NeedsDiscussion) {
        SessionArtifactState::PausedForDiscussion
    } else {
        SessionArtifactState::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::session_artifacts::{
        AcceptanceCriterion, ArtifactContract, SessionArtifactBundle, TodoItem,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn completion_lock_passes_only_when_everything_is_closed() {
        let mut bundle = SessionArtifactBundle::new(
            "session-001",
            "task",
            TaskArtifactState::Done,
            SpecArtifactState::Finalized,
            TodoArtifactState::AllChecked,
            "problem",
        );
        bundle.task.acceptance_criteria.push(AcceptanceCriterion {
            id: "ac.1".to_string(),
            text: "done".to_string(),
            met: true,
            evidence: Some("evidence-1".to_string()),
        });
        bundle.spec.contract = ArtifactContract {
            inputs: vec!["input".to_string()],
            outputs: vec!["output".to_string()],
            invariants: vec!["invariant".to_string()],
            out_of_scope: vec!["scope".to_string()],
        };
        bundle.todo.items.push(TodoItem {
            id: "t.1".to_string(),
            text: "checked".to_string(),
            kind: "test".to_string(),
            checked: true,
            blocking: true,
        });

        let decision = evaluate_completion_lock(&bundle.task, &bundle.spec, &bundle.todo);
        assert_eq!(decision.outcome, CompletionLockOutcome::Done);
        assert!(decision.summary.can_close());
        assert_eq!(session_close_state(&decision), SessionArtifactState::Done);
    }

    #[test]
    fn completion_lock_requires_discussion_for_missing_evidence() {
        let mut bundle = SessionArtifactBundle::new(
            "session-001",
            "task",
            TaskArtifactState::Open,
            SpecArtifactState::Draft,
            TodoArtifactState::Open,
            "problem",
        );
        bundle.task.acceptance_criteria.push(AcceptanceCriterion {
            id: "ac.1".to_string(),
            text: "needs evidence".to_string(),
            met: true,
            evidence: None,
        });
        bundle.todo.items.push(TodoItem {
            id: "t.1".to_string(),
            text: "blocked".to_string(),
            kind: "implement".to_string(),
            checked: false,
            blocking: true,
        });

        let decision = evaluate_completion_lock(&bundle.task, &bundle.spec, &bundle.todo);
        assert_eq!(decision.outcome, CompletionLockOutcome::NeedsDiscussion);
        assert_eq!(decision.summary.open_blocking_todos, 1);
        assert_eq!(decision.summary.missing_evidence_refs, 1);
        assert_eq!(
            session_close_state(&decision),
            SessionArtifactState::PausedForDiscussion
        );
    }
}
