use crate::operator::{OperatorSnapshot, chrome};

pub fn render_lines(snapshot: &OperatorSnapshot) -> Vec<String> {
    let approval = match &snapshot.approval {
        Some(a) => a,
        None => return vec!["no approval pending".into()],
    };
    let mut lines = vec![
        chrome::top_bar(snapshot, "destructive bash"),
        "approval required".into(),
        format!("{} the agent wants to run a shell command", approval.kind),
        format!("$ {}", approval.command),
        format!(
            "cwd {} · sandbox {} · network {} · writes {}",
            approval.cwd, approval.sandbox, approval.network, approval.writes
        ),
        format!("risk {}", approval.risk),
        format!("policy {}", approval.policy),
        "y approve once · a approve+remember · n reject · r reject with reason".into(),
    ];
    if let Some((i, n)) = approval.batch_position {
        lines.push(format!("batch {i}/{n}"));
    }
    lines.push(chrome::footer(snapshot));
    lines
}
