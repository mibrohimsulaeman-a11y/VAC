use crate::approval_boundary::{VacBoundApproval, require_vac_bound_approval};
use rmcp::model::{CallToolResult, Content};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use vac_foundation::remote_connection::PathLocation;

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VacReadPlanTicket {
    pub id: String,
    #[serde(default)]
    pub span_id: Option<String>,
    #[serde(default)]
    pub path_sha256: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

pub(crate) fn require_vac_view_governance(
    approval: &Option<VacBoundApproval>,
    ticket: &Option<VacReadPlanTicket>,
    path: &str,
    password: &Option<String>,
    private_key_path: &Option<String>,
    actual_arguments: &Value,
) -> Result<(), CallToolResult> {
    let credential_material_present = password.as_ref().is_some_and(|value| !value.is_empty())
        || private_key_path
            .as_ref()
            .is_some_and(|value| !value.is_empty());
    let remote = is_remote_path(path);

    if credential_material_present || remote {
        require_vac_bound_approval(
            approval,
            if credential_material_present {
                "credential_read"
            } else {
                "network_access"
            },
            path,
            actual_arguments,
        )?;
        return Ok(());
    }

    if let Some(approval) = approval {
        require_vac_bound_approval(
            &Some(approval.clone()),
            "filesystem_read",
            path,
            actual_arguments,
        )?;
        return Ok(());
    }

    let Some(ticket) = ticket else {
        return Err(CallToolResult::error(vec![
            Content::text("VAC_READ_GOVERNANCE_REQUIRED"),
            Content::text(format!(
                "VAC v1.9 blocked view on {path}: local file/grep/glob reads require a deterministic read_plan_ticket or vac_bound_approval stamped by the bound runtime."
            )),
        ]));
    };
    verify_vac_read_plan_ticket(ticket, path)?;
    Ok(())
}

fn is_remote_path(path: &str) -> bool {
    PathLocation::parse(path)
        .map(|loc| loc.is_remote())
        .unwrap_or(false)
}

fn verify_vac_read_plan_ticket(
    ticket: &VacReadPlanTicket,
    path: &str,
) -> Result<(), CallToolResult> {
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    verify_vac_read_plan_ticket_at_root(ticket, path, &root)
}

fn verify_vac_read_plan_ticket_at_root(
    ticket: &VacReadPlanTicket,
    path: &str,
    root: &Path,
) -> Result<(), CallToolResult> {
    if ticket.id.trim().is_empty() {
        return Err(CallToolResult::error(vec![
            Content::text("VAC_READ_PLAN_TICKET_INVALID"),
            Content::text(
                "VAC read_plan_ticket.id must be non-empty and generated from .vac/index/read_plans.jsonl",
            ),
        ]));
    }
    let read_plans = root.join(".vac/index/read_plans.jsonl");
    let Ok(contents) = fs::read_to_string(&read_plans) else {
        return Err(CallToolResult::error(vec![
            Content::text("VAC_READ_PLAN_TICKET_INDEX_MISSING"),
            Content::text(
                "VAC read_plan_ticket cannot be verified because .vac/index/read_plans.jsonl is unavailable",
            ),
        ]));
    };
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let id_match = row
            .get("ticket_id")
            .or_else(|| row.get("id"))
            .and_then(serde_json::Value::as_str)
            == Some(ticket.id.as_str());
        let path_match = row.get("path").and_then(serde_json::Value::as_str) == Some(path);
        if id_match && path_match {
            return Ok(());
        }
    }
    Err(CallToolResult::error(vec![
        Content::text("VAC_READ_PLAN_TICKET_BINDING_MISMATCH"),
        Content::text(format!(
            "VAC read_plan_ticket {} does not bind to requested path {} in .vac/index/read_plans.jsonl",
            ticket.id, path
        )),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn rendered_error(result: CallToolResult) -> String {
        format!("{result:?}")
    }

    fn ticket(id: &str) -> VacReadPlanTicket {
        VacReadPlanTicket {
            id: id.to_string(),
            span_id: None,
            path_sha256: None,
            mode: None,
        }
    }

    fn write_read_plans(root: &Path, contents: &str) {
        let index_dir = root.join(".vac/index");
        std::fs::create_dir_all(&index_dir).expect("read plan index dir should be created");
        let mut file = std::fs::File::create(index_dir.join("read_plans.jsonl"))
            .expect("read plan index should be created");
        file.write_all(contents.as_bytes())
            .expect("read plan index should be written");
    }

    #[test]
    fn read_plan_ticket_rejects_empty_id() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let err = verify_vac_read_plan_ticket_at_root(&ticket("   "), "src/lib.rs", temp.path())
            .err()
            .map(rendered_error)
            .unwrap_or_default();

        assert!(err.contains("VAC_READ_PLAN_TICKET_INVALID"));
    }

    #[test]
    fn read_plan_ticket_accepts_matching_ticket_and_path() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        write_read_plans(
            temp.path(),
            &format!("{}\n", json!({"ticket_id":"read.1","path":"src/lib.rs"})),
        );

        assert!(
            verify_vac_read_plan_ticket_at_root(&ticket("read.1"), "src/lib.rs", temp.path())
                .is_ok()
        );
    }

    #[test]
    fn read_plan_ticket_rejects_path_mismatch() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        write_read_plans(
            temp.path(),
            &format!("{}\n", json!({"ticket_id":"read.1","path":"src/lib.rs"})),
        );

        let err =
            verify_vac_read_plan_ticket_at_root(&ticket("read.1"), "src/main.rs", temp.path())
                .err()
                .map(rendered_error)
                .unwrap_or_default();

        assert!(err.contains("VAC_READ_PLAN_TICKET_BINDING_MISMATCH"));
    }

    #[test]
    fn view_governance_rejects_local_read_without_ticket_or_approval() {
        let err = require_vac_view_governance(
            &None,
            &None,
            "src/lib.rs",
            &None,
            &None,
            &json!({"path":"src/lib.rs"}),
        )
        .err()
        .map(rendered_error)
        .unwrap_or_default();

        assert!(err.contains("VAC_READ_GOVERNANCE_REQUIRED"));
    }
}
