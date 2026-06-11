use crate::operator::{OperatorSnapshot, chrome};

pub fn render_lines(snapshot: &OperatorSnapshot) -> Vec<String> {
    let mut lines = vec![
        chrome::top_bar(snapshot, "runtime jobs"),
        chrome::tab_bar(snapshot),
        format!(
            "autopilot {} · queue {} · running {}",
            snapshot.control_plane.status,
            snapshot.runtime_jobs.queued,
            snapshot.runtime_jobs.running
        ),
        "state kind id trigger age next-run".into(),
    ];
    if snapshot.runtime_jobs.records.is_empty() {
        lines.push("no runtime jobs registered".into());
    } else {
        for r in &snapshot.runtime_jobs.records {
            lines.push(format!(
                "{} {} {} {} {} {}",
                r.state,
                r.kind,
                r.id,
                r.trigger,
                r.age,
                r.next_run.as_deref().unwrap_or("—")
            ));
        }
    }
    lines.push(chrome::footer(snapshot));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{mode::OperatorMode, snapshot::OperatorSnapshot};
    #[test]
    fn runtime_jobs_empty_state_has_no_mock_rows() {
        let s = OperatorSnapshot::from_workspace("/missing", OperatorMode::RuntimeJobs);
        let joined = render_lines(&s).join("\n");
        assert!(joined.contains("no runtime jobs"));
        assert!(!joined.contains(concat!("refactor ", "handlers/input.rs")));
    }
}
