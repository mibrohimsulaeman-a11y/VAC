use std::fs;
use std::path::Path;
use std::path::PathBuf;

use ratatui::style::Stylize;
use ratatui::text::Line;

use crate::history_cell::PlainHistoryCell;
use crate::legacy_core::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionStatusSurface {
    Session,
    Spec,
    Tasks,
    Todo,
    Evidence,
}

pub(crate) fn new_session_status_output(
    config: &Config,
    surface: SessionStatusSurface,
) -> PlainHistoryCell {
    PlainHistoryCell::new(render_session_status_lines(config.cwd.as_path(), surface))
}

fn render_session_status_lines(cwd: &Path, surface: SessionStatusSurface) -> Vec<Line<'static>> {
    match surface {
        SessionStatusSurface::Session => render_session_lines(cwd),
        SessionStatusSurface::Spec => render_artifact_lines(cwd, "spec.yaml", "spec artifacts"),
        SessionStatusSurface::Tasks => render_artifact_lines(cwd, "task.yaml", "task artifacts"),
        SessionStatusSurface::Todo => render_artifact_lines(cwd, "todo.yaml", "todo artifacts"),
        SessionStatusSurface::Evidence => render_evidence_lines(cwd),
    }
}

fn render_session_lines(cwd: &Path) -> Vec<Line<'static>> {
    let report = vac_core::control_plane::load_session_doctor_report(cwd);
    let mut lines = vec!["/session".bold().into(), "".into()];
    lines.extend(
        report
            .render_text()
            .lines()
            .map(|line| line.to_string().into()),
    );
    lines
}

fn render_artifact_lines(cwd: &Path, artifact_name: &str, label: &str) -> Vec<Line<'static>> {
    let mut lines = vec![
        format!("/{}", artifact_name.trim_end_matches(".yaml"))
            .bold()
            .into(),
        format!("workspace: {}", cwd.display()).dim().into(),
        "".into(),
        Line::from(label.to_string()),
    ];

    let artifacts = find_session_artifacts(cwd, artifact_name);
    if artifacts.is_empty() {
        lines.push("  no session artifacts registered yet".dim().into());
        lines.push(
            "  run `vac session start <session-id>` to create them"
                .dim()
                .into(),
        );
        return lines;
    }

    for artifact in artifacts {
        match fs::read_to_string(&artifact) {
            Ok(source) => lines.extend(render_artifact_summary(&artifact, artifact_name, &source)),
            Err(err) => lines.push(
                format!("  {}: failed to read: {err}", artifact.display())
                    .red()
                    .into(),
            ),
        }
    }
    lines
}

fn render_artifact_summary(path: &Path, artifact_name: &str, source: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let session = yaml_field(source, "session").unwrap_or_else(|| "unknown".to_string());
    let id = yaml_field(source, "id").unwrap_or_else(|| path.display().to_string());
    let state = yaml_field(source, "state").unwrap_or_else(|| "unknown".to_string());
    lines.push(format!("- {session} · {id} · state={state}").into());

    match artifact_name {
        "task.yaml" => {
            let criteria_total = source.matches("met:").count();
            let criteria_met = source.matches("met: true").count();
            let open_questions = list_count_after_key(source, "open_questions");
            lines.push(
                format!(
                    "  acceptance: {criteria_met}/{criteria_total} met · open_questions={open_questions}"
                )
                .dim()
                .into(),
            );
        }
        "spec.yaml" => {
            if let Some(problem) = yaml_field(source, "problem") {
                lines.push(format!("  problem: {problem}").dim().into());
            }
            let capabilities = list_count_after_key(source, "touched_capabilities");
            let memory_refs = list_count_after_key(source, "memory_refs");
            lines.push(
                format!("  touched_capabilities={capabilities} · memory_refs={memory_refs}")
                    .dim()
                    .into(),
            );
        }
        "todo.yaml" => {
            let total = source.matches("checked:").count();
            let checked = source.matches("checked: true").count();
            let blocking = source.matches("blocking: true").count();
            lines.push(
                format!("  todo: {checked}/{total} checked · blocking={blocking}")
                    .dim()
                    .into(),
            );
        }
        _ => {}
    }
    lines.push(format!("  source: {}", path.display()).dim().into());
    lines
}

fn render_evidence_lines(cwd: &Path) -> Vec<Line<'static>> {
    let mut lines = vec![
        "/evidence".bold().into(),
        format!("workspace: {}", cwd.display()).dim().into(),
        "".into(),
    ];
    let v1 = yaml_files(cwd.join(".vac/registry/evidence"));
    let v2 = yaml_files(cwd.join(".vac/registry/evidence-v2"));
    lines.push(format!("evidence v1 records: {}", v1.len()).into());
    lines.push(format!("evidence v2 records: {}", v2.len()).into());

    let mut recent = v2
        .iter()
        .chain(v1.iter())
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    recent.sort();
    if recent.is_empty() {
        lines.push("  no evidence records registered yet".dim().into());
        lines.push(
            "  run `vac doctor evidence` after a validated slice"
                .dim()
                .into(),
        );
    } else {
        lines.push("".into());
        lines.push("recent records:".bold().into());
        for path in recent {
            lines.push(
                format!("  {}", display_workspace_path(cwd, &path))
                    .dim()
                    .into(),
            );
        }
    }
    lines
}

fn find_session_artifacts(cwd: &Path, artifact_name: &str) -> Vec<PathBuf> {
    let sessions_root = cwd.join(".vac/registry/sessions");
    let Ok(entries) = fs::read_dir(sessions_root) else {
        return Vec::new();
    };
    let mut artifacts = entries
        .flatten()
        .map(|entry| entry.path().join(artifact_name))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    artifacts.sort();
    artifacts
}

fn yaml_files(root: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_yaml_files(&root, &mut out);
    out.sort();
    out
}

fn collect_yaml_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "yaml" || ext == "yml")
        {
            out.push(path);
        }
    }
}

fn yaml_field(source: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    source.lines().find_map(|line| {
        let trimmed = line.trim();
        let value = trimmed.strip_prefix(&prefix)?.trim();
        (!value.is_empty()).then(|| trim_yaml_scalar(value))
    })
}

fn trim_yaml_scalar(value: &str) -> String {
    value
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn list_count_after_key(source: &str, key: &str) -> usize {
    let mut in_list = false;
    let mut count = 0usize;
    for line in source.lines() {
        if line.trim() == format!("{key}:") {
            in_list = true;
            continue;
        }
        if !in_list {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("- ") {
            count += 1;
        } else if !trimmed.is_empty() && !line.starts_with(' ') {
            break;
        }
    }
    count
}

fn display_workspace_path(cwd: &Path, path: &Path) -> String {
    path.strip_prefix(cwd).unwrap_or(path).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_empty_session_guidance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let lines = render_session_status_lines(temp.path(), SessionStatusSurface::Session)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(lines.contains("vac doctor sessions: PASS"));
        assert!(lines.contains("run `vac session start <SESSION_ID>`"));
    }

    #[test]
    fn renders_task_spec_todo_artifact_summaries() {
        let temp = tempfile::tempdir().expect("tempdir");
        let session_dir = temp.path().join(".vac/registry/sessions/session-test");
        fs::create_dir_all(&session_dir).expect("session dir");
        fs::write(
            session_dir.join("task.yaml"),
            "id: task.session-test.main\nsession: session-test\nstate: done\nacceptance_criteria:\n  - id: ac.1\n    met: true\nopen_questions: []\n",
        )
        .expect("task");
        fs::write(
            session_dir.join("spec.yaml"),
            "id: spec.session-test.main\nsession: session-test\nstate: finalized\nproblem: ship session surface\ntouched_capabilities:\n  - vac.sessions\nmemory_refs: []\n",
        )
        .expect("spec");
        fs::write(
            session_dir.join("todo.yaml"),
            "id: todo.session-test.main\nsession: session-test\nstate: all_checked\nitems:\n  - id: t.1\n    checked: true\n    blocking: false\n",
        )
        .expect("todo");

        let task = render_session_status_lines(temp.path(), SessionStatusSurface::Tasks)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(task.contains("acceptance: 1/1 met"));

        let spec = render_session_status_lines(temp.path(), SessionStatusSurface::Spec)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(spec.contains("problem: ship session surface"));

        let todo = render_session_status_lines(temp.path(), SessionStatusSurface::Todo)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(todo.contains("todo: 1/1 checked"));
    }
}
