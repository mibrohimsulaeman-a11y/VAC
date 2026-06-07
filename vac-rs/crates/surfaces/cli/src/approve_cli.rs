use clap::Parser;
use serde_yaml::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Respond to a VAC approval request from the local control plane.
#[derive(Debug, Parser)]
pub struct ApproveCommand {
    /// Approval request id, for example `approval.workflow.step`.
    #[arg(value_name = "APPROVAL_ID")]
    approval_id: String,

    /// Decision to persist for the approval request.
    #[arg(long, value_enum, default_value_t = ApprovalDecisionArg::Approved)]
    decision: ApprovalDecisionArg,

    /// Human-readable reason stored with the response.
    #[arg(long, default_value = "operator approved via vac approve")]
    reason: String,

    /// Workspace root used for `.vac/registry/approvals`.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,

    /// Explicit approval request binding record. Defaults to `.vac/registry/approvals/<id>.yaml`.
    #[arg(long, value_name = "FILE")]
    request: Option<PathBuf>,

    /// Expected plan hash for binding verification. Defaults to the request binding hash.
    #[arg(long, value_name = "SHA256")]
    plan_hash: Option<String>,

    /// Expected diff hash for binding verification. Defaults to the request binding hash.
    #[arg(long, value_name = "SHA256")]
    diff_hash: Option<String>,

    /// Expected policy snapshot hash for binding verification. Defaults to the request binding hash.
    #[arg(long, value_name = "SHA256")]
    policy_snapshot_hash: Option<String>,

    /// Require an Ed25519 signed approval response before accepting it.
    #[arg(long, default_value_t = false)]
    require_signature: bool,

    /// Do not write the response file; print the target path and payload only.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ApprovalDecisionArg {
    Approved,
    Denied,
}

impl ApproveCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let id = sanitize_id(&self.approval_id);
        let request_path = self.request.clone().unwrap_or_else(|| {
            workspace
                .join(".vac/registry/approvals")
                .join(format!("{id}.yaml"))
        });
        let request_record = load_approval_request_record(&request_path, self.decision)?;
        let snapshot =
            vac_core::control_plane::vac_init_approval_binding::ApprovalBindingSnapshot {
                plan_hash: self
                    .plan_hash
                    .clone()
                    .unwrap_or_else(|| request_record.binding.plan_hash.clone()),
                diff_hash: self
                    .diff_hash
                    .clone()
                    .unwrap_or_else(|| request_record.binding.diff_hash.clone()),
                policy_snapshot_hash: self
                    .policy_snapshot_hash
                    .clone()
                    .unwrap_or_else(|| request_record.binding.policy_snapshot_hash.clone()),
                now: utc_timestamp_string(),
            };
        let replay_path = workspace
            .join(".vac/registry/approvals")
            .join("replay-store.yaml");
        let mut replay_nonces = load_replay_nonces(&replay_path)?;
        let mut replay_store = vac_core::control_plane::VacInitApprovalReplayStore::default();
        for nonce in &replay_nonces {
            replay_store.mark_nonce(nonce.clone());
        }
        let signature_policy = if self.require_signature {
            vac_core::control_plane::VacInitApprovalSignaturePolicy::RequireEd25519
        } else {
            vac_core::control_plane::VacInitApprovalSignaturePolicy::AllowUnsigned
        };
        vac_core::control_plane::vac_init_approval_binding::validate_approval_binding_with_signature_policy(
            &request_record,
            &snapshot,
            &replay_store,
            signature_policy,
        )
        .map_err(|err| anyhow::anyhow!("approval binding verification failed: {err:?}"))?;

        let response_nonce = request_record.binding.nonce;
        let response_path = workspace
            .join(".vac/registry/approvals")
            .join(format!("{id}.response.yaml"));
        let response_hash = approval_response_hash(
            &self.approval_id,
            self.decision,
            &self.reason,
            &response_nonce,
            &snapshot,
        );
        let payload = render_approval_response(
            &self.approval_id,
            self.decision,
            &self.reason,
            &request_path,
            &snapshot,
            &response_nonce,
            &response_hash,
        );
        if self.dry_run {
            println!("vac approve: DRY-RUN");
            println!("request: {}", request_path.display());
            println!("path: {}", response_path.display());
            print!("{payload}");
            return Ok(());
        }
        let parent = response_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("approval response path has no parent"))?;
        fs::create_dir_all(parent)?;
        fs::write(&response_path, payload)?;
        replay_nonces.insert(response_nonce.clone());
        write_replay_nonces(&replay_path, &replay_nonces)?;
        println!("vac approve: PASS");
        println!("approval_id: {}", self.approval_id);
        println!("decision: {}", self.decision.as_str());
        println!("request: {}", request_path.display());
        println!("response: {}", response_path.display());
        println!("replay_nonce_consumed: {response_nonce}");
        Ok(())
    }
}

fn load_approval_request_record(
    request_path: &Path,
    decision: ApprovalDecisionArg,
) -> anyhow::Result<vac_core::control_plane::VacInitApprovalRequestRecord> {
    if !request_path.exists() {
        return Err(anyhow::anyhow!(
            "missing approval request binding record: {}",
            request_path.display()
        ));
    }
    let value: Value = serde_yaml::from_str(&fs::read_to_string(request_path)?)?;
    let id = scalar(&value, "id").ok_or_else(|| anyhow::anyhow!("approval request missing id"))?;
    let timestamp = scalar(&value, "timestamp").unwrap_or_else(utc_timestamp_string);
    let binding = vac_core::control_plane::vac_init_approval_binding::ApprovalBinding {
        plan_hash: nested_scalar(&value, &["binding", "plan_hash"])
            .ok_or_else(|| anyhow::anyhow!("approval request missing binding.plan_hash"))?,
        diff_hash: nested_scalar(&value, &["binding", "diff_hash"])
            .ok_or_else(|| anyhow::anyhow!("approval request missing binding.diff_hash"))?,
        policy_snapshot_hash: nested_scalar(&value, &["binding", "policy_snapshot_hash"])
            .ok_or_else(|| {
                anyhow::anyhow!("approval request missing binding.policy_snapshot_hash")
            })?,
        nonce: nested_scalar(&value, &["binding", "nonce"])
            .ok_or_else(|| anyhow::anyhow!("approval request missing binding.nonce"))?,
        expires_at: nested_scalar(&value, &["binding", "expires_at"])
            .ok_or_else(|| anyhow::anyhow!("approval request missing binding.expires_at"))?,
    };
    let response = build_response_for_validation(&value, decision);
    Ok(vac_core::control_plane::VacInitApprovalRequestRecord {
        id,
        timestamp,
        status: vac_core::control_plane::vac_init_approval_binding::ApprovalStatus::Approved,
        request: vac_core::control_plane::vac_init_approval_binding::ApprovalRequestPayload {
            action: nested_scalar(&value, &["request", "action"])
                .unwrap_or_else(|| "unspecified".to_string()),
            risk_level: nested_scalar(&value, &["request", "risk_level"])
                .unwrap_or_else(|| "medium".to_string()),
            capability: nested_scalar(&value, &["request", "capability"])
                .unwrap_or_else(|| "vac.unknown".to_string()),
            plan_id: nested_scalar(&value, &["request", "plan_id"])
                .unwrap_or_else(|| "plan.unknown".to_string()),
            rationale: nested_scalar(&value, &["request", "rationale"])
                .unwrap_or_else(|| "approval request loaded by vac approve".to_string()),
            scope: vac_core::control_plane::vac_init_approval_binding::ApprovalScope {
                file: nested_scalar(&value, &["request", "scope", "file"]),
                command: nested_scalar(&value, &["request", "scope", "command"]),
                network_host: nested_scalar(&value, &["request", "scope", "network_host"]),
                network_protocol: nested_scalar(&value, &["request", "scope", "network_protocol"]),
            },
        },
        binding,
        response: Some(response),
    })
}

fn build_response_for_validation(
    value: &Value,
    decision: ApprovalDecisionArg,
) -> vac_core::control_plane::vac_init_approval_binding::ApprovalResponse {
    let signature = value
        .as_mapping()
        .and_then(|_| nested_scalar(value, &["response", "signature", "algorithm"]))
        .and_then(|algorithm| {
            (algorithm == "ed25519").then(|| {
                vac_core::control_plane::VacInitApprovalResponseSignature {
                    algorithm,
                    public_key_base64: nested_scalar(
                        value,
                        &["response", "signature", "public_key_base64"],
                    )
                    .unwrap_or_default(),
                    signature_base64: nested_scalar(value, &["response", "signature", "value"])
                        .or_else(|| {
                            nested_scalar(value, &["response", "signature", "signature_base64"])
                        })
                        .unwrap_or_default(),
                }
            })
        });
    vac_core::control_plane::vac_init_approval_binding::ApprovalResponse {
        decided_by: nested_scalar(value, &["response", "decided_by"])
            .unwrap_or_else(|| "operator".to_string()),
        decided_at: nested_scalar(value, &["response", "decided_at"]),
        decision: match decision {
            ApprovalDecisionArg::Approved => {
                vac_core::control_plane::vac_init_approval_binding::ApprovalDecision::Approved
            }
            ApprovalDecisionArg::Denied => {
                vac_core::control_plane::vac_init_approval_binding::ApprovalDecision::Denied
            }
        },
        comment: nested_scalar(value, &["response", "comment"]),
        content_hash: nested_scalar(value, &["response", "content_hash"]),
        signature_algorithm: signature
            .as_ref()
            .map(|signature| signature.algorithm.clone())
            .unwrap_or_else(|| "unsigned".to_string()),
        signature,
    }
}

fn render_approval_response(
    id: &str,
    decision: ApprovalDecisionArg,
    reason: &str,
    request_path: &Path,
    snapshot: &vac_core::control_plane::vac_init_approval_binding::ApprovalBindingSnapshot,
    nonce: &str,
    content_hash: &str,
) -> String {
    format!(
        "schema_version: 1\nkind: approval.response\nid: {}\napproval_id: {}\ndecision: {}\nreason: {}\nsource: vac approve\nrequest: {}\nbinding_snapshot:\n  plan_hash: {}\n  diff_hash: {}\n  policy_snapshot_hash: {}\n  verified_at: {}\nresponse:\n  content_hash: {}\nreplay_protection:\n  nonce: {}\n  consumed: true\n",
        yaml_scalar(&format!("approval.response.{}", sanitize_id(id))),
        yaml_scalar(id),
        decision.as_str(),
        yaml_scalar(reason),
        yaml_scalar(&request_path.display().to_string()),
        yaml_scalar(&snapshot.plan_hash),
        yaml_scalar(&snapshot.diff_hash),
        yaml_scalar(&snapshot.policy_snapshot_hash),
        yaml_scalar(&snapshot.now),
        yaml_scalar(content_hash),
        yaml_scalar(nonce)
    )
}

fn approval_response_hash(
    id: &str,
    decision: ApprovalDecisionArg,
    reason: &str,
    nonce: &str,
    snapshot: &vac_core::control_plane::vac_init_approval_binding::ApprovalBindingSnapshot,
) -> String {
    let payload = format!(
        "approval_id={id}\ndecision={}\nreason={reason}\nnonce={nonce}\nplan_hash={}\ndiff_hash={}\npolicy_snapshot_hash={}\n",
        decision.as_str(),
        snapshot.plan_hash,
        snapshot.diff_hash,
        snapshot.policy_snapshot_hash
    );
    vac_core::control_plane::vac_init_evidence_chain::sha256_hex(payload.as_bytes())
}

impl ApprovalDecisionArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
        }
    }
}

fn normalize_root(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path.to_path_buf())
    }
}

fn load_replay_nonces(path: &Path) -> anyhow::Result<BTreeSet<String>> {
    let mut nonces = BTreeSet::new();
    if !path.exists() {
        return Ok(nonces);
    }
    let value: Value = serde_yaml::from_str(&fs::read_to_string(path)?)?;
    if let Some(Value::Sequence(items)) = mapping_get(&value, "consumed_nonces") {
        for item in items {
            if let Some(nonce) = item.as_str() {
                nonces.insert(nonce.to_string());
            }
        }
    }
    Ok(nonces)
}

fn write_replay_nonces(path: &Path, nonces: &BTreeSet<String>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut yaml = "schema_version: 1\nkind: approval.replay_store\nid: approval.replay_store\nconsumed_nonces:\n".to_string();
    for nonce in nonces {
        yaml.push_str(&format!("  - {}\n", yaml_scalar(nonce)));
    }
    fs::write(path, yaml)?;
    Ok(())
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn scalar(value: &Value, key: &str) -> Option<String> {
    nested_scalar(value, &[key])
}

fn nested_scalar(value: &Value, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = mapping_get(current, key)?;
    }
    match current {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn mapping_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String(key.to_string())))
}

fn yaml_scalar(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/' | ':' | '@'))
    {
        value.to_string()
    } else {
        format!("{value:?}")
    }
}

fn utc_timestamp_string() -> String {
    std::env::var("VAC_APPROVAL_NOW").unwrap_or_else(|_| "2026-06-02T00:00:00Z".to_string())
}
