pub fn format_workflow_execution_state(state: &WorkflowExecutionState) -> String {
    match state {
        WorkflowExecutionState::Ready {
            workflow_id,
            title,
            step_count,
        } => format!("execution: ready workflow={workflow_id} title={title} steps={step_count}"),
        WorkflowExecutionState::Step {
            index,
            step_count,
            resolution,
            lifecycle,
            started_step_count,
            completed_step_count,
            waiting_approval_step_count,
            blocked_step_count,
        } => format!(
            "execution: step {index}/{step_count} {} lifecycle={} started={} completed={} waiting_approval={} blocked={}",
            resolution.id,
            lifecycle,
            started_step_count,
            completed_step_count,
            waiting_approval_step_count,
            blocked_step_count
        ),
        WorkflowExecutionState::Finished {
            workflow_id,
            title,
            started_step_count,
            completed_step_count,
            waiting_approval_step_count,
            blocked_step_count,
        } => format!(
            "execution: finished workflow={workflow_id} title={title} started={started_step_count} completed={completed_step_count} waiting_approval={waiting_approval_step_count} blocked={blocked_step_count}"
        ),
        WorkflowExecutionState::Cancelled {
            workflow_id,
            title,
            reason,
            started_step_count,
            completed_step_count,
            waiting_approval_step_count,
            blocked_step_count,
        } => format!(
            "execution: cancelled workflow={workflow_id} title={title} reason={reason} started={started_step_count} completed={completed_step_count} waiting_approval={waiting_approval_step_count} blocked={blocked_step_count}"
        ),
        WorkflowExecutionState::Failed {
            workflow_id,
            title,
            reason,
            started_step_count,
            completed_step_count,
            waiting_approval_step_count,
            blocked_step_count,
        } => format!(
            "execution: failed workflow={workflow_id} title={title} reason={reason} started={started_step_count} completed={completed_step_count} waiting_approval={waiting_approval_step_count} blocked={blocked_step_count}"
        ),
    }
}

pub fn format_workflow_execution_event(event: &WorkflowExecutionEvent) -> String {
    match event {
        WorkflowExecutionEvent::Started {
            workflow_id,
            title,
            step_count,
        } => format!("started workflow={workflow_id} title={title} steps={step_count}"),
        WorkflowExecutionEvent::StepStarted { index, resolution } => {
            format!("step {index}. {} -> running", resolution.id)
        }
        WorkflowExecutionEvent::StepWaitingApproval {
            index,
            resolution,
            approval_request_id,
        } => format!(
            "step {index}. {} -> waiting approval approval_id={approval_request_id}",
            resolution.id
        ),
        WorkflowExecutionEvent::ApprovalResolved {
            approval_request_id,
            status,
            reason,
        } => format!("approval {approval_request_id} -> {status}: {reason}"),
        WorkflowExecutionEvent::StepSucceeded { index, resolution } => {
            format!("step {index}. {} -> succeeded", resolution.id)
        }
        WorkflowExecutionEvent::StepFailed {
            index,
            resolution,
            reason,
        } => format!("step {index}. {} -> failed: {reason}", resolution.id),
        WorkflowExecutionEvent::Cancelled { reason } => format!("cancelled reason={reason}"),
        WorkflowExecutionEvent::Finished {
            started_step_count,
            completed_step_count,
            waiting_approval_step_count,
            blocked_step_count,
        } => format!(
            "finished started={started_step_count} completed={completed_step_count} waiting_approval={waiting_approval_step_count} blocked={blocked_step_count}"
        ),
    }
}

pub fn format_workflow_dry_run_state(state: &WorkflowDryRunState) -> String {
    match state {
        WorkflowDryRunState::Ready {
            workflow_id,
            title,
            step_count,
        } => format!("dry-run: ready workflow={workflow_id} title={title} steps={step_count}"),
        WorkflowDryRunState::Started {
            workflow_id,
            title,
            step_count,
        } => format!("dry-run: started workflow={workflow_id} title={title} steps={step_count}"),
        WorkflowDryRunState::Step {
            index,
            step_count,
            resolution,
            supported_step_count,
            blocked_step_count,
        } => format!(
            "dry-run: step {index}/{step_count} {} -> {} (supported={} blocked={})",
            resolution.id,
            format_workflow_step_resolution(resolution),
            supported_step_count,
            blocked_step_count
        ),
        WorkflowDryRunState::Finished {
            workflow_id,
            title,
            supported_step_count,
            blocked_step_count,
        } => format!(
            "dry-run: finished workflow={workflow_id} title={title} supported={supported_step_count} blocked={blocked_step_count}"
        ),
    }
}

fn format_workflow_run_preview(preview: &WorkflowRunPreview) -> String {
    if preview.supported {
        return "supported by initial safe runner".to_string();
    }

    let blocked_steps = preview
        .blocked_steps
        .iter()
        .map(|step| format!("{}={}", step.id, step.uses))
        .collect::<Vec<_>>()
        .join(", ");
    format!("blocked: unsupported steps=[{blocked_steps}]")
}

