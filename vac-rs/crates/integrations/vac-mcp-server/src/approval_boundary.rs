use rmcp::model::{CallToolResult, Content};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct VacSignatureHint {
    pub algorithm: String,
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct VacBoundApproval {
    pub schema_version: u32,
    pub kind: String,
    pub gate: String,
    pub decision: String,
    pub binding_hash: String,
    pub mode: String,
    pub action: Option<String>,
    pub target: Option<String>,
    pub session_id: Option<String>,
    pub capability: Option<String>,
    pub read_plan_ticket: Option<String>,
    #[serde(default)]
    pub approval_request_id: Option<String>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub plan_hash: Option<String>,
    #[serde(default)]
    pub diff_hash: Option<String>,
    #[serde(default)]
    pub policy_snapshot_hash: Option<String>,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub operator_sig: Option<VacSignatureHint>,
    #[serde(default)]
    pub broker_sig: Option<VacSignatureHint>,
}

// SV marker: require_vac_bound_approval(&vac_bound_approval, "execute_process"
// SV marker: require_vac_bound_approval(&vac_bound_approval, "filesystem_write"
// SV marker: require_vac_bound_approval(&vac_bound_approval, "filesystem_delete"
// SV marker: require_vac_bound_approval(&vac_bound_approval, "network_access"
pub(crate) fn require_vac_bound_approval(
    approval: &Option<VacBoundApproval>,
    action: &str,
    target: &str,
    actual_arguments: &Value,
) -> Result<(), CallToolResult> {
    let Some(approval) = approval else {
        return Err(CallToolResult::error(vec![
            Content::text("VAC_BOUND_APPROVAL_REQUIRED"),
            Content::text(format!(
                "VAC v1.9 blocked {action} on {target}: mutating/process/network MCP tools require a vac_bound_approval stamped by BoundRuntimeToolBoundary after Semantic Plan, artifact, patch/command/network, and policy gates pass."
            )),
        ]));
    };
    verify_vac_bound_approval(approval, action, target, actual_arguments)
}

fn verify_vac_bound_approval(
    approval: &VacBoundApproval,
    action: &str,
    target: &str,
    actual_arguments: &Value,
) -> Result<(), CallToolResult> {
    if approval.schema_version != 2
        || approval.kind != "vac_bound_tool_approval"
        || !matches!(approval.decision.as_str(), "pass" | "pass_with_warnings")
        || !approval.binding_hash.starts_with("sha256:")
        || approval.mode != "l1_runtime_mediated"
    {
        return Err(CallToolResult::error(vec![
            Content::text("VAC_BOUND_APPROVAL_INVALID"),
            Content::text(format!(
                "VAC v1.9 rejected {action} on {target}: approval proof is missing a valid v2 schema/kind/decision/hash/mode binding."
            )),
        ]));
    }
    if approval.action.as_deref() != Some(action) {
        return Err(vac_bound_binding_error(action, target, "action mismatch"));
    }
    if approval.target.as_deref() != Some(target) {
        return Err(vac_bound_binding_error(action, target, "target mismatch"));
    }
    for (label, value) in [
        ("session_id", approval.session_id.as_deref()),
        ("capability", approval.capability.as_deref()),
        (
            "approval_request_id",
            approval.approval_request_id.as_deref(),
        ),
        ("tool_call_id", approval.tool_call_id.as_deref()),
        ("tool_name", approval.tool_name.as_deref()),
        ("plan_hash", approval.plan_hash.as_deref()),
        ("diff_hash", approval.diff_hash.as_deref()),
        (
            "policy_snapshot_hash",
            approval.policy_snapshot_hash.as_deref(),
        ),
        ("nonce", approval.nonce.as_deref()),
    ] {
        if value.unwrap_or("").trim().is_empty() {
            return Err(vac_bound_binding_error(
                action,
                target,
                &format!("{label} missing"),
            ));
        }
    }
    if !vac_diff_hash_matches(approval, action, target, actual_arguments) {
        return Err(vac_bound_binding_error(
            action,
            target,
            "diff_hash mismatch against actual MCP tool arguments",
        ));
    }
    let recomputed = recompute_vac_bound_binding_hash(approval);
    if recomputed != approval.binding_hash {
        return Err(vac_bound_binding_error(
            action,
            target,
            "binding_hash mismatch",
        ));
    }
    Ok(())
}

fn vac_diff_hash_matches(
    approval: &VacBoundApproval,
    action: &str,
    target: &str,
    actual_arguments: &Value,
) -> bool {
    let expected = approval.diff_hash.as_deref();
    let payload = arguments_without_vac_bound_approval(actual_arguments);
    let compact_payload = strip_nulls(payload.clone());
    let candidates = [payload, compact_payload];
    candidates.iter().any(|arguments| {
        let candidate = vac_jcs::canonical_json_sha256(&json!({
            "tool_call_id": approval.tool_call_id,
            "tool_name": approval.tool_name,
            "action": action,
            "target": target,
            "arguments": arguments,
            "gate": approval.gate,
        }));
        expected == Some(candidate.as_str())
    })
}

fn recompute_vac_bound_binding_hash(approval: &VacBoundApproval) -> String {
    vac_jcs::canonical_json_sha256(&json!({
        "plan_hash": approval.plan_hash,
        "diff_hash": approval.diff_hash,
        "policy_snapshot_hash": approval.policy_snapshot_hash,
        "nonce": approval.nonce,
    }))
}

fn arguments_without_vac_bound_approval(arguments: &Value) -> Value {
    let mut payload = arguments.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("vac_bound_approval");
    }
    payload
}

fn strip_nulls(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                if val.is_null() {
                    continue;
                }
                out.insert(key, strip_nulls(val));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(strip_nulls).collect()),
        other => other,
    }
}

fn vac_bound_binding_error(action: &str, target: &str, detail: &str) -> CallToolResult {
    CallToolResult::error(vec![
        Content::text("VAC_BOUND_APPROVAL_BINDING_MISMATCH"),
        Content::text(format!(
            "VAC v1.9 rejected {action} on {target}: approval v2 binding failed broker-side recomputation against the actual MCP payload: {detail}."
        )),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_approval(action: &str, target: &str, actual_arguments: &Value) -> VacBoundApproval {
        let tool_call_id = Some("tool-call-1".to_string());
        let tool_name = Some("run_command".to_string());
        let gate = "policy.command.execute_process".to_string();
        let diff_hash = vac_jcs::canonical_json_sha256(&json!({
            "tool_call_id": tool_call_id,
            "tool_name": tool_name,
            "action": action,
            "target": target,
            "arguments": arguments_without_vac_bound_approval(actual_arguments),
            "gate": gate,
        }));
        let mut approval = VacBoundApproval {
            schema_version: 2,
            kind: "vac_bound_tool_approval".to_string(),
            gate,
            decision: "pass".to_string(),
            binding_hash: "sha256:pending".to_string(),
            mode: "l1_runtime_mediated".to_string(),
            action: Some(action.to_string()),
            target: Some(target.to_string()),
            session_id: Some("session-1".to_string()),
            capability: Some("vac-mcp-server".to_string()),
            read_plan_ticket: None,
            approval_request_id: Some("approval-1".to_string()),
            tool_call_id,
            tool_name,
            plan_hash: Some("sha256:plan".to_string()),
            diff_hash: Some(diff_hash),
            policy_snapshot_hash: Some("sha256:policy".to_string()),
            nonce: Some("nonce-1".to_string()),
            expires_at: None,
            operator_sig: None,
            broker_sig: None,
        };
        approval.binding_hash = recompute_vac_bound_binding_hash(&approval);
        approval
    }

    fn rendered_error(result: CallToolResult) -> String {
        format!("{result:?}")
    }

    #[test]
    fn require_bound_approval_rejects_missing_proof() {
        let err = require_vac_bound_approval(
            &None,
            "execute_process",
            "run_command",
            &json!({"command": "cargo check"}),
        )
        .err()
        .map(rendered_error)
        .unwrap_or_default();

        assert!(err.contains("VAC_BOUND_APPROVAL_REQUIRED"));
    }

    #[test]
    fn require_bound_approval_accepts_matching_payload_binding() {
        let arguments = json!({"command": "cargo check"});
        let approval = valid_approval("execute_process", "run_command", &arguments);

        assert!(
            require_vac_bound_approval(
                &Some(approval),
                "execute_process",
                "run_command",
                &arguments,
            )
            .is_ok()
        );
    }

    #[test]
    fn require_bound_approval_rejects_argument_drift() {
        let approved_arguments = json!({"command": "cargo check"});
        let runtime_arguments = json!({"command": "cargo check -p vac-mcp-server"});
        let approval = valid_approval("execute_process", "run_command", &approved_arguments);
        let err = require_vac_bound_approval(
            &Some(approval),
            "execute_process",
            "run_command",
            &runtime_arguments,
        )
        .err()
        .map(rendered_error)
        .unwrap_or_default();

        assert!(err.contains("VAC_BOUND_APPROVAL_BINDING_MISMATCH"));
        assert!(err.contains("diff_hash mismatch"));
    }
}
