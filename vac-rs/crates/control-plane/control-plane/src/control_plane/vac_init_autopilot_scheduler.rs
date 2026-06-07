//! Runtime autopilot scheduler/executor for VAC control-plane surfaces.
//!
//! This module is intentionally host-local and dependency-light. It is the
//! producer behind `.vac/registry/autopilot/status.yaml`, but it is no longer a
//! fixture/status façade: each scheduler tick loads scheduled workflow manifests
//! and drives them through the control-plane `workflow_runner` state machine.
//! The TUI `/runtime` surface therefore observes real workflow outcomes
//! (`running -> ok|failed|waiting_approval`) instead of synthetic `queued` rows.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::registry::load_control_plane_registry_report;
use super::workflow_manifest::load_workflow_manifest;
use super::workflow_runner::evaluate_workflow_approval_policy;
use super::workflow_runner::execute_workflow_manifest;
use super::workflow_runner::execute_workflow_manifest_with_approval_policy;
use super::workflow_runner::format_workflow_execution_event;

pub const AUTOPILOT_STATUS_PATH: &str = ".vac/registry/autopilot/status.yaml";
pub const AUTOPILOT_ACTION_LOG_PATH: &str = ".vac/registry/autopilot/actions.log.yaml";
pub const AUTOPILOT_RUN_LOG_PATH: &str = ".vac/registry/autopilot/runs.log.yaml";
pub const AUTOPILOT_RUN_STATE_PATH: &str = ".vac/registry/autopilot/run-state.yaml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutopilotJobRecord {
    pub state: String,
    pub kind: String,
    pub id: String,
    pub trigger: String,
    pub age: String,
    pub next_run: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutopilotSchedulerRuntimeReport {
    pub status: String,
    pub pid: u32,
    pub mode: String,
    pub env: String,
    pub queued: usize,
    pub running: usize,
    pub jobs: Vec<AutopilotJobRecord>,
    pub status_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutopilotActionReport {
    pub action: String,
    pub job_id: String,
    pub applied: bool,
    pub reason: String,
    pub status_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiscoveredAutopilotWorkflow {
    path: PathBuf,
    record: AutopilotJobRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AutopilotRunState {
    last_run_at: u64,
    next_run_at: Option<u64>,
    state: String,
}

/// Execute one host-local scheduler tick and persist the live status report.
///
/// This is deliberately synchronous: the TUI calls it on open/refresh/key events
/// and sees a deterministic status file. Long-running shell execution still
/// remains policy-gated in the normal workflow/command layers; this scheduler
/// only drives the existing control-plane workflow state machine.
pub fn refresh_autopilot_scheduler_status(
    workspace_root: impl AsRef<Path>,
) -> Result<AutopilotSchedulerRuntimeReport, String> {
    let root = workspace_root.as_ref();
    refresh_autopilot_scheduler_status_with_options(root, false)
}

fn refresh_autopilot_scheduler_status_with_options(
    root: &Path,
    force_due: bool,
) -> Result<AutopilotSchedulerRuntimeReport, String> {
    let discovered = discover_autopilot_workflows(root)?;
    let existing = read_status_jobs(root);
    let mut run_states = read_run_states(root);
    let now = unix_ts();
    let mut jobs = Vec::new();

    for workflow in discovered {
        if let Some(cancelled) = existing
            .iter()
            .find(|job| job.id == workflow.record.id && job.state == "cancelled")
        {
            run_states.insert(
                workflow.record.id.clone(),
                AutopilotRunState {
                    last_run_at: now,
                    next_run_at: None,
                    state: "cancelled".to_string(),
                },
            );
            jobs.push(cancelled.clone());
            continue;
        }

        let prior_state = run_states.get(&workflow.record.id).cloned();
        if !force_due && !workflow_is_due(&workflow, prior_state.as_ref(), now) {
            jobs.push(record_from_cached_state(
                &workflow.record,
                prior_state.as_ref(),
                now,
            ));
            continue;
        }

        let mut executed = execute_autopilot_workflow(root, workflow, &jobs)?;
        let interval = schedule_interval_secs(&executed.trigger);
        let next_run_at = interval.map(|seconds| now.saturating_add(seconds));
        executed.next_run = format_next_run(next_run_at, now);
        run_states.insert(
            executed.id.clone(),
            AutopilotRunState {
                last_run_at: now,
                next_run_at,
                state: executed.state.clone(),
            },
        );
        jobs.push(executed);
    }

    // Preserve terminal records for workflows that no longer exist in the
    // workspace, so cancellation/evidence history is not silently hidden.
    for job in existing {
        if matches!(
            job.state.as_str(),
            "cancelled" | "failed" | "ok" | "waiting_approval"
        ) && !jobs.iter().any(|candidate| candidate.id == job.id)
        {
            jobs.push(job);
        }
    }

    jobs.sort_by(|a, b| a.id.cmp(&b.id));
    write_run_states(root, &run_states)?;
    let report = build_report(root, jobs);
    write_autopilot_status(root, &report)?;
    Ok(report)
}

/// Execute an operator action against a real workflow job.
///
/// `retry` reruns the workflow through `workflow_runner`; `cancel` persists a
/// terminal cancellation marker; `inspect`/`attach` produce evidence and preserve
/// the current runtime status without pretending to attach to a process that does
/// not exist.
pub fn execute_autopilot_action(
    workspace_root: impl AsRef<Path>,
    action: &str,
    job_id: &str,
) -> Result<AutopilotActionReport, String> {
    let root = workspace_root.as_ref();
    let mut report = refresh_autopilot_scheduler_status_with_options(root, false)?;
    let mut run_states = read_run_states(root);
    let mut applied = false;
    let mut reason = String::new();

    match action {
        "cancel" => {
            for job in &mut report.jobs {
                if job.id == job_id {
                    job.state = "cancelled".to_string();
                    job.age = "operator-action".to_string();
                    job.next_run = "cancelled by operator; future ticks skip this job".to_string();
                    run_states.insert(
                        job.id.clone(),
                        AutopilotRunState {
                            last_run_at: unix_ts(),
                            next_run_at: None,
                            state: "cancelled".to_string(),
                        },
                    );
                    applied = true;
                    reason =
                        "cancel marker persisted; cooperative scheduler will not spawn future runs"
                            .to_string();
                    break;
                }
            }
        }
        "retry" => {
            let discovered = discover_autopilot_workflows(root)?;
            if let Some(workflow) = discovered
                .into_iter()
                .find(|entry| entry.record.id == job_id)
            {
                report.jobs.retain(|job| job.id != job_id);
                let now = unix_ts();
                let mut executed = execute_autopilot_workflow(root, workflow, &report.jobs)?;
                let interval = schedule_interval_secs(&executed.trigger);
                let next_run_at = interval.map(|seconds| now.saturating_add(seconds));
                executed.next_run = format_next_run(next_run_at, now);
                run_states.insert(
                    executed.id.clone(),
                    AutopilotRunState {
                        last_run_at: now,
                        next_run_at,
                        state: executed.state.clone(),
                    },
                );
                reason = format!(
                    "retry executed via workflow_runner; state={}",
                    executed.state
                );
                report.jobs.push(executed);
                applied = true;
            }
        }
        "open" | "inspect" => {
            if report.jobs.iter().any(|job| job.id == job_id) {
                reason = format!("inspection opened for {job_id}; see {AUTOPILOT_RUN_LOG_PATH}");
                applied = true;
            }
        }
        "attach" => {
            if report.jobs.iter().any(|job| job.id == job_id) {
                reason = format!(
                    "attached to persisted workflow run log for {job_id}; no detached process is claimed"
                );
                applied = true;
            }
        }
        other => {
            reason = format!("unknown autopilot action '{other}'");
        }
    }

    if !applied && reason.is_empty() {
        reason = format!("job '{job_id}' not found");
    }

    write_run_states(root, &run_states)?;
    report = build_report(root, report.jobs);
    write_autopilot_status(root, &report)?;
    append_autopilot_action_log(root, action, job_id, applied, &reason)?;
    Ok(AutopilotActionReport {
        action: action.to_string(),
        job_id: job_id.to_string(),
        applied,
        reason,
        status_path: root.join(AUTOPILOT_STATUS_PATH),
    })
}

fn execute_autopilot_workflow(
    root: &Path,
    workflow: DiscoveredAutopilotWorkflow,
    prefix_jobs: &[AutopilotJobRecord],
) -> Result<AutopilotJobRecord, String> {
    let mut running = workflow.record.clone();
    running.state = "running".to_string();
    running.age = "executing".to_string();
    running.next_run = "workflow_runner executing".to_string();

    let mut running_jobs = prefix_jobs.to_vec();
    running_jobs.push(running.clone());
    write_autopilot_status(root, &build_report(root, running_jobs))?;

    let manifest = match load_workflow_manifest(&workflow.path) {
        Ok(manifest) => manifest,
        Err(err) => {
            let mut failed = running;
            failed.state = "failed".to_string();
            failed.age = "0s".to_string();
            failed.next_run = format!("manifest load failed: {err}");
            append_autopilot_run_log(root, &failed, &[failed.next_run.clone()])?;
            return Ok(failed);
        }
    };

    let registry_report = load_control_plane_registry_report(root);
    let execution = if let Some(registry) = registry_report.registry() {
        let approval_policy = evaluate_workflow_approval_policy(&manifest, registry);
        execute_workflow_manifest_with_approval_policy(&manifest, &approval_policy)
    } else {
        // Unit-test and isolated temp workspaces may not have a full `.vac`
        // registry. Real workspaces still bind approval policy via the branch
        // above; this fallback keeps the scheduler testable without inventing
        // a fake registry fixture here.
        execute_workflow_manifest(&manifest)
    };
    let mut completed = running;
    completed.age = format!(
        "{} started/{} completed",
        execution.started_step_count(),
        execution.completed_step_count()
    );
    completed.state = if execution.is_waiting_approval() {
        "waiting_approval".to_string()
    } else if execution.is_failure() {
        "failed".to_string()
    } else {
        "ok".to_string()
    };
    completed.next_run = match completed.state.as_str() {
        "ok" => "completed by workflow_runner".to_string(),
        "waiting_approval" => "paused for approval".to_string(),
        "failed" => "failed; inspect run log".to_string(),
        _ => "completed".to_string(),
    };
    let event_lines = execution
        .events()
        .iter()
        .map(format_workflow_execution_event)
        .collect::<Vec<_>>();
    append_autopilot_run_log(root, &completed, &event_lines)?;
    Ok(completed)
}

fn discover_autopilot_workflows(root: &Path) -> Result<Vec<DiscoveredAutopilotWorkflow>, String> {
    let workflow_dir = root.join(".vac/workflows");
    let mut jobs = Vec::new();
    if !workflow_dir.exists() {
        return Ok(jobs);
    }
    for entry in fs::read_dir(&workflow_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if !matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yaml" | "yml")
        ) {
            continue;
        }
        let source = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        let id = yaml_scalar_by_key(&source, "id").unwrap_or_else(|| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("workflow")
                .to_string()
        });
        let is_scheduled = source.contains("autopilot")
            || source.contains("scheduler")
            || source
                .lines()
                .any(|line| line.trim_start().starts_with("when:"));
        if !is_scheduled {
            continue;
        }
        let trigger = yaml_scalar_by_key(&source, "when")
            .or_else(|| yaml_scalar_by_key(&source, "schedule"))
            .unwrap_or_else(|| "manual/on-demand".to_string());
        let kind = if trigger.contains("cron") || trigger.contains('@') {
            "cron"
        } else if trigger.contains("file") || trigger.contains("watch") {
            "filewatch"
        } else {
            "workflow"
        };
        jobs.push(DiscoveredAutopilotWorkflow {
            path,
            record: AutopilotJobRecord {
                state: "queued".to_string(),
                kind: kind.to_string(),
                id,
                trigger,
                age: "0s".to_string(),
                next_run: "due now".to_string(),
            },
        });
    }
    Ok(jobs)
}

fn workflow_is_due(
    _workflow: &DiscoveredAutopilotWorkflow,
    prior_state: Option<&AutopilotRunState>,
    now: u64,
) -> bool {
    let Some(prior) = prior_state else {
        return true;
    };
    if prior.state == "cancelled" || prior.state == "waiting_approval" {
        return false;
    }
    match prior.next_run_at {
        Some(next) => now >= next,
        None => !matches!(prior.state.as_str(), "ok" | "failed"),
    }
}

fn record_from_cached_state(
    base: &AutopilotJobRecord,
    prior_state: Option<&AutopilotRunState>,
    now: u64,
) -> AutopilotJobRecord {
    let mut record = base.clone();
    if let Some(prior) = prior_state {
        record.state = prior.state.clone();
        record.age = format_age(now.saturating_sub(prior.last_run_at));
        record.next_run = format_next_run(prior.next_run_at, now);
    } else {
        record.state = "queued".to_string();
        record.age = "new".to_string();
        record.next_run = "due now".to_string();
    }
    record
}

fn read_run_states(root: &Path) -> BTreeMap<String, AutopilotRunState> {
    let Ok(source) = fs::read_to_string(root.join(AUTOPILOT_RUN_STATE_PATH)) else {
        return BTreeMap::new();
    };
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("run_state:")?;
            let parts = rest.split('|').map(str::trim).collect::<Vec<_>>();
            if parts.len() < 4 {
                return None;
            }
            let id = parts[0].to_string();
            let last_run_at = parts[1].parse::<u64>().ok()?;
            let next_run_at = if parts[2] == "none" {
                None
            } else {
                parts[2].parse::<u64>().ok()
            };
            Some((
                id,
                AutopilotRunState {
                    last_run_at,
                    next_run_at,
                    state: parts[3].to_string(),
                },
            ))
        })
        .collect()
}

fn write_run_states(
    root: &Path,
    run_states: &BTreeMap<String, AutopilotRunState>,
) -> Result<(), String> {
    let path = root.join(AUTOPILOT_RUN_STATE_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut out = String::new();
    out.push_str("schema_version: 1\nkind: autopilot_run_state\nid: runtime.autopilot.run-state\n");
    for (id, state) in run_states {
        let next = state
            .next_run_at
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string());
        out.push_str(&format!(
            "run_state: {} | {} | {} | {}\n",
            yaml_inline(id),
            state.last_run_at,
            next,
            yaml_inline(&state.state)
        ));
    }
    fs::write(path, out).map_err(|err| err.to_string())
}

fn schedule_interval_secs(trigger: &str) -> Option<u64> {
    let normalized = trigger.trim().to_ascii_lowercase();
    if normalized.contains("@hourly") || normalized.contains("cron.hourly") {
        return Some(60 * 60);
    }
    if normalized.contains("@daily") || normalized.contains("cron.daily") {
        return Some(24 * 60 * 60);
    }
    if normalized.contains("@weekly") || normalized.contains("cron.weekly") {
        return Some(7 * 24 * 60 * 60);
    }
    parse_every_interval_secs(&normalized)
}

fn parse_every_interval_secs(value: &str) -> Option<u64> {
    let marker = "every";
    let idx = value.find(marker)? + marker.len();
    let rest =
        value[idx..].trim_start_matches(|ch: char| ch == ':' || ch == '=' || ch.is_whitespace());
    let digits = rest
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    let amount = digits.parse::<u64>().ok()?;
    let unit = rest[digits.len()..].trim_start();
    // Fail-closed unit handling: an empty unit (or an explicit seconds unit)
    // means seconds, but an *unrecognized* unit must not silently fall through
    // to seconds -- e.g. "every 1d" would otherwise be read as "every 1 second"
    // and hammer the runtime. Returning None makes the workflow stop
    // auto-rescheduling instead. NOTE: day/week units are intentionally not
    // parsed here yet (the only multi-day vocabulary is the @daily/@weekly cron
    // aliases handled in schedule_interval_secs); add them to both places
    // together if the spec adopts "every Nd"/"every Nw".
    if unit.is_empty() || unit.starts_with('s') {
        Some(amount)
    } else if unit.starts_with('m') {
        Some(amount.saturating_mul(60))
    } else if unit.starts_with('h') {
        Some(amount.saturating_mul(60 * 60))
    } else {
        None
    }
}

fn format_age(seconds: u64) -> String {
    format!("{} ago", format_duration(seconds))
}

fn format_duration(seconds: u64) -> String {
    if seconds >= 60 * 60 {
        format!("{}h", seconds / (60 * 60))
    } else if seconds >= 60 {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

fn format_next_run(next_run_at: Option<u64>, now: u64) -> String {
    match next_run_at {
        Some(next) if next > now => format!("next run in {}", format_duration(next - now)),
        Some(_) => "due now".to_string(),
        None => "manual retry required".to_string(),
    }
}

fn read_status_jobs(root: &Path) -> Vec<AutopilotJobRecord> {
    let Ok(source) = fs::read_to_string(root.join(AUTOPILOT_STATUS_PATH)) else {
        return Vec::new();
    };
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("job:")?;
            let parts = rest.split('|').map(str::trim).collect::<Vec<_>>();
            (parts.len() >= 6).then(|| AutopilotJobRecord {
                state: parts[0].to_string(),
                kind: parts[1].to_string(),
                id: parts[2].to_string(),
                trigger: parts[3].to_string(),
                age: parts[4].to_string(),
                next_run: parts[5].to_string(),
            })
        })
        .collect()
}

fn build_report(root: &Path, mut jobs: Vec<AutopilotJobRecord>) -> AutopilotSchedulerRuntimeReport {
    jobs.sort_by(|a, b| a.id.cmp(&b.id));
    let queued = jobs.iter().filter(|job| job.state == "queued").count();
    let running = jobs.iter().filter(|job| job.state == "running").count();
    let status = if running > 0 {
        "running"
    } else if queued > 0 {
        "queued"
    } else if jobs.iter().any(|job| job.state == "waiting_approval") {
        "waiting_approval"
    } else if jobs.iter().any(|job| job.state == "failed") {
        "failed"
    } else if jobs.is_empty() {
        "idle"
    } else {
        "ok"
    };
    AutopilotSchedulerRuntimeReport {
        status: status.to_string(),
        pid: std::process::id(),
        mode: "scheduled-workflow-runner".to_string(),
        env: "host".to_string(),
        queued,
        running,
        jobs,
        status_path: root.join(AUTOPILOT_STATUS_PATH),
    }
}

fn write_autopilot_status(
    root: &Path,
    report: &AutopilotSchedulerRuntimeReport,
) -> Result<(), String> {
    let path = root.join(AUTOPILOT_STATUS_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, render_autopilot_status_yaml(report)).map_err(|err| err.to_string())
}

fn append_autopilot_action_log(
    root: &Path,
    action: &str,
    job_id: &str,
    applied: bool,
    reason: &str,
) -> Result<(), String> {
    let path = root.join(AUTOPILOT_ACTION_LOG_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut existing = fs::read_to_string(&path).unwrap_or_else(|_| {
        "schema_version: 1\nkind: evidence\nid: autopilot.actions\nrecords:\n".to_string()
    });
    existing.push_str(&format!(
        "  - at: {}\n    action: {}\n    job_id: {}\n    applied: {}\n    reason: {}\n",
        unix_ts(),
        yaml_scalar(action),
        yaml_scalar(job_id),
        applied,
        yaml_scalar(reason)
    ));
    fs::write(path, existing).map_err(|err| err.to_string())
}

fn append_autopilot_run_log(
    root: &Path,
    job: &AutopilotJobRecord,
    events: &[String],
) -> Result<(), String> {
    let path = root.join(AUTOPILOT_RUN_LOG_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut existing = fs::read_to_string(&path).unwrap_or_else(|_| {
        "schema_version: 1\nkind: evidence\nid: autopilot.runs\nrecords:\n".to_string()
    });
    existing.push_str(&format!(
        "  - at: {}\n    workflow_id: {}\n    state: {}\n    trigger: {}\n    summary: {}\n    events:\n",
        unix_ts(),
        yaml_scalar(&job.id),
        yaml_scalar(&job.state),
        yaml_scalar(&job.trigger),
        yaml_scalar(&job.next_run)
    ));
    if events.is_empty() {
        existing.push_str("      - <no events emitted>\n");
    } else {
        for event in events {
            existing.push_str(&format!("      - {}\n", yaml_scalar(event)));
        }
    }
    fs::write(path, existing).map_err(|err| err.to_string())
}

pub fn render_autopilot_status_yaml(report: &AutopilotSchedulerRuntimeReport) -> String {
    let mut out = String::new();
    out.push_str("schema_version: 1\nkind: autopilot_status\nid: runtime.autopilot.status\n");
    out.push_str(&format!("status: {}\n", yaml_scalar(&report.status)));
    out.push_str(&format!("pid: {}\n", report.pid));
    out.push_str("uptime: live\n");
    out.push_str(&format!("mode: {}\n", yaml_scalar(&report.mode)));
    out.push_str(&format!("env: {}\n", yaml_scalar(&report.env)));
    out.push_str(&format!("queued: {}\n", report.queued));
    out.push_str(&format!("running: {}\n", report.running));
    out.push_str("task_graph: '[workflow→gate→execute_workflow_manifest→evidence]'\n");
    out.push_str(&format!(
        "node_progress: 'node {}/{} · {}%'\n",
        report.running,
        report.jobs.len().max(1),
        if report.jobs.is_empty() {
            0
        } else {
            100 * report
                .jobs
                .iter()
                .filter(|job| {
                    matches!(
                        job.state.as_str(),
                        "ok" | "failed" | "waiting_approval" | "cancelled"
                    )
                })
                .count()
                / report.jobs.len().max(1)
        }
    ));
    out.push_str(
        "tokens: 'runtime-owned'\nspend: '$0.000'\nretry: 'policy-gated workflow_runner retry'\n",
    );
    for job in &report.jobs {
        out.push_str(&format!(
            "job: {} | {} | {} | {} | {} | {}\n",
            yaml_inline(&job.state),
            yaml_inline(&job.kind),
            yaml_inline(&job.id),
            yaml_inline(&job.trigger),
            yaml_inline(&job.age),
            yaml_inline(&job.next_run)
        ));
    }
    out
}

fn yaml_scalar_by_key(source: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    source.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed.strip_prefix(&prefix).map(|value| {
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        })
    })
}

fn yaml_inline(value: &str) -> String {
    value.replace('|', "/").replace('\n', " ")
}

fn yaml_scalar(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/' | ':' | '@'))
        && !value.is_empty()
    {
        value.to_string()
    } else {
        format!("{value:?}")
    }
}

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_every_interval_rejects_unknown_units_fail_closed() {
        // Recognized cron aliases and "every N<unit>" forms.
        assert_eq!(schedule_interval_secs("@hourly"), Some(60 * 60));
        assert_eq!(schedule_interval_secs("@daily"), Some(24 * 60 * 60));
        assert_eq!(schedule_interval_secs("@weekly"), Some(7 * 24 * 60 * 60));
        assert_eq!(parse_every_interval_secs("every 30"), Some(30));
        assert_eq!(parse_every_interval_secs("every 30s"), Some(30));
        assert_eq!(parse_every_interval_secs("every 5m"), Some(5 * 60));
        assert_eq!(parse_every_interval_secs("every 2h"), Some(2 * 60 * 60));

        // Fail-closed: an unrecognized unit must NOT silently degrade to seconds
        // (previously "every 1d" scheduled every 1 second). Returning None stops
        // auto-rescheduling instead of hammering the runtime.
        assert_eq!(parse_every_interval_secs("every 1d"), None);
        assert_eq!(parse_every_interval_secs("every 2w"), None);
        assert_eq!(parse_every_interval_secs("every 5y"), None);

        // No "every" marker -> None.
        assert_eq!(parse_every_interval_secs("@monthly"), None);
    }

    fn temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-autopilot-scheduler-{unique}"))
    }

    fn scheduled_workflow_yaml(id: &str) -> String {
        format!(
            "schema_version: 1\nkind: workflow\nid: {id}\ntitle: |\n  Autopilot maintenance\n  when: '@hourly'\nstatus: ready\ninputs: {{}}\nsteps:\n  - id: emit\n    uses: capability.activity.emit\nui:\n  surface: /runtime\n  progress_panel: true\n  activity_log: true\npolicy:\n  default_risk: safe_read\nvalidation:\n  gates:\n    - emit\n"
        )
    }

    fn capability_workflow_yaml() -> String {
        "schema_version: 1\nkind: capability\nid: vac.workflow\ntitle: Workflow\nstatus: ready\nowner:\n  crate: vac-control-plane\n  module: control_plane\ndescription: Stub\nreason: stub\ndepends_on: []\npolicy:\n  risk: safe_read\n  mutates_files: false\n  network: false\n  redaction: false\n  approval_required_for: []\nvalidation:\n  commands: []\n".to_string()
    }

    #[test]
    fn refresh_executes_scheduled_workflow_and_writes_status() {
        let root = temp_root();
        fs::create_dir_all(root.join(".vac/workflows")).unwrap();
        fs::write(
            root.join(".vac/workflows/maintenance.autopilot.yaml"),
            scheduled_workflow_yaml("maintenance.autopilot"),
        )
        .unwrap();
        fs::create_dir_all(root.join(".vac/capabilities")).unwrap();
        fs::write(
            root.join(".vac/capabilities/workflow.yaml"),
            capability_workflow_yaml(),
        )
        .unwrap();
        let report = refresh_autopilot_scheduler_status(&root).unwrap();
        assert!(report.jobs.iter().any(|job| job.state == "ok"));
        let status = fs::read_to_string(root.join(AUTOPILOT_STATUS_PATH)).unwrap();
        assert!(status.contains("execute_workflow_manifest"));
        assert!(status.contains("job: ok | cron | maintenance.autopilot"));
        let run_log = fs::read_to_string(root.join(AUTOPILOT_RUN_LOG_PATH)).unwrap();
        assert!(run_log.contains("workflow_id: maintenance.autopilot"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refresh_does_not_rerun_terminal_job_until_schedule_due() {
        let root = temp_root();
        fs::create_dir_all(root.join(".vac/workflows")).unwrap();
        fs::write(
            root.join(".vac/workflows/maintenance.autopilot.yaml"),
            scheduled_workflow_yaml("maintenance.autopilot"),
        )
        .unwrap();
        fs::create_dir_all(root.join(".vac/capabilities")).unwrap();
        fs::write(
            root.join(".vac/capabilities/workflow.yaml"),
            capability_workflow_yaml(),
        )
        .unwrap();
        let first = refresh_autopilot_scheduler_status(&root).unwrap();
        assert!(first.jobs.iter().any(|job| job.state == "ok"));
        let run_log_before = fs::read_to_string(root.join(AUTOPILOT_RUN_LOG_PATH)).unwrap();
        let second = refresh_autopilot_scheduler_status(&root).unwrap();
        let run_log_after = fs::read_to_string(root.join(AUTOPILOT_RUN_LOG_PATH)).unwrap();
        assert_eq!(
            run_log_before, run_log_after,
            "refresh tick must not rerun already-ok job before next schedule"
        );
        assert!(
            second
                .jobs
                .iter()
                .any(|job| job.next_run.contains("next run in"))
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn run_state_records_next_schedule() {
        let root = temp_root();
        fs::create_dir_all(root.join(".vac/workflows")).unwrap();
        fs::write(
            root.join(".vac/workflows/maintenance.autopilot.yaml"),
            scheduled_workflow_yaml("maintenance.autopilot"),
        )
        .unwrap();
        fs::create_dir_all(root.join(".vac/capabilities")).unwrap();
        fs::write(
            root.join(".vac/capabilities/workflow.yaml"),
            capability_workflow_yaml(),
        )
        .unwrap();
        let _ = refresh_autopilot_scheduler_status(&root).unwrap();
        let run_state = fs::read_to_string(root.join(AUTOPILOT_RUN_STATE_PATH)).unwrap();
        assert!(run_state.contains("run_state: maintenance.autopilot"));
        assert!(run_state.contains("| ok"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn retry_reruns_workflow_and_cancel_persists_terminal_state() {
        let root = temp_root();
        fs::create_dir_all(root.join(".vac/workflows")).unwrap();
        fs::write(
            root.join(".vac/workflows/maintenance.autopilot.yaml"),
            scheduled_workflow_yaml("maintenance.autopilot"),
        )
        .unwrap();
        fs::create_dir_all(root.join(".vac/capabilities")).unwrap();
        fs::write(
            root.join(".vac/capabilities/workflow.yaml"),
            capability_workflow_yaml(),
        )
        .unwrap();
        let retry = execute_autopilot_action(&root, "retry", "maintenance.autopilot").unwrap();
        assert!(retry.applied);
        assert!(retry.reason.contains("workflow_runner"));
        let cancel = execute_autopilot_action(&root, "cancel", "maintenance.autopilot").unwrap();
        assert!(cancel.applied);
        let status = fs::read_to_string(root.join(AUTOPILOT_STATUS_PATH)).unwrap();
        assert!(status.contains("job: cancelled | cron | maintenance.autopilot"));
        let log = fs::read_to_string(root.join(AUTOPILOT_ACTION_LOG_PATH)).unwrap();
        assert!(log.contains("action: retry"));
        assert!(log.contains("action: cancel"));
        let _ = fs::remove_dir_all(root);
    }
}
