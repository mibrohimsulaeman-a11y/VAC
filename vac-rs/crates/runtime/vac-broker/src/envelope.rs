use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const BROKER_ENVELOPE_SCHEMA_VERSION: &str = "vac.broker.envelope.v1";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BrokerEnvelopeError {
    #[error("broker envelope JSON error: {0}")]
    Json(String),
    #[error("tool payload cannot supply broker authority field: {0}")]
    ToolSuppliedAuthorityField(String),
    #[error("tool payload cannot request broker-controlled execution mode: {0}")]
    ToolSuppliedExecutionMode(String),
    #[error("broker envelope field is invalid or empty: {0}")]
    InvalidRequiredField(String),
    #[error("mediated execution requires policy_snapshot_hash")]
    MissingPolicySnapshot,
    #[error("mediated execution requires approval_ref or read_plan_ref")]
    MissingApprovalOrReadPlanBinding,
    #[error("broker decision does not match intent field: {0}")]
    DecisionIntentMismatch(String),
    #[error("broker decision must allow the execution request")]
    DeniedByBrokerDecision,
    #[error("broker decision must select mediated_l2 execution")]
    DecisionNotMediated,
    #[error("mediated_l2 evidence requires broker_record_hash")]
    MissingBrokerRecordHash,
    #[error("broker_attested custody requires broker_signature_hash")]
    MissingBrokerSignatureHash,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrokerSubjectKind {
    Agent,
    Broker,
    Operator,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerSubject {
    pub subject_kind: BrokerSubjectKind,
    pub subject_id: String,
}

impl BrokerSubject {
    pub fn validate(&self) -> Result<(), BrokerEnvelopeError> {
        validate_nonempty("actor.subject_id", &self.subject_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrokerOperationKind {
    FilesystemRead,
    FilesystemCreate,
    FilesystemStrReplace,
    FilesystemRemove,
    ProcessSpawn,
    NetworkHttpRequest,
    RemoteProcessIo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrokerResourceKind {
    Filesystem,
    Process,
    Network,
    CredentialBearingRemoteIo,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BrokerExecutionMode {
    #[default]
    ObservedL1,
    MediatedL2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BrokerCustody {
    #[default]
    LocalOnly,
    SelfPromoted,
    CiAttested,
    BrokerAttested,
    ExternalAttested,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrokerDecisionVerdict {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerPolicySnapshotRef {
    pub policy_snapshot_hash: String,
}

impl BrokerPolicySnapshotRef {
    pub fn validate(&self) -> Result<(), BrokerEnvelopeError> {
        validate_nonempty("policy_snapshot_hash", &self.policy_snapshot_hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerApprovalBinding {
    pub approval_ref: Option<String>,
    pub read_plan_ref: Option<String>,
    pub preimage_ref: Option<String>,
}

impl BrokerApprovalBinding {
    pub fn has_approval_or_read_plan(&self) -> bool {
        option_is_nonempty(&self.approval_ref) || option_is_nonempty(&self.read_plan_ref)
    }

    pub fn validate(&self) -> Result<(), BrokerEnvelopeError> {
        validate_optional_nonempty("approval_ref", &self.approval_ref)?;
        validate_optional_nonempty("read_plan_ref", &self.read_plan_ref)?;
        validate_optional_nonempty("preimage_ref", &self.preimage_ref)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerIntent {
    pub schema_version: String,
    pub intent_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub actor: BrokerSubject,
    pub capability_id: String,
    pub tool_name: String,
    pub operation_kind: BrokerOperationKind,
    pub resource_kind: BrokerResourceKind,
    pub resource_ref: String,
    pub structured_args_hash: String,
    pub policy_snapshot_hash: Option<String>,
    pub approval_ref: Option<String>,
    pub read_plan_ref: Option<String>,
    pub preimage_ref: Option<String>,
    #[serde(default)]
    pub execution_mode: BrokerExecutionMode,
    pub created_at: String,
}

impl BrokerIntent {
    pub fn from_tool_payload(payload: Value) -> Result<Self, BrokerEnvelopeError> {
        if let Some(mode) = find_tool_supplied_mediated_mode(&payload) {
            return Err(BrokerEnvelopeError::ToolSuppliedExecutionMode(mode));
        }
        if let Some(field) = find_tool_supplied_authority_field(&payload) {
            return Err(BrokerEnvelopeError::ToolSuppliedAuthorityField(field));
        }
        let intent: Self = serde_json::from_value(payload)
            .map_err(|err| BrokerEnvelopeError::Json(err.to_string()))?;
        if intent.execution_mode != BrokerExecutionMode::ObservedL1 {
            return Err(BrokerEnvelopeError::ToolSuppliedExecutionMode(format!(
                "{:?}",
                intent.execution_mode
            )));
        }
        intent.validate_tool_intent()?;
        Ok(intent)
    }

    pub fn canonical_hash(&self) -> Result<String, BrokerEnvelopeError> {
        canonical_sha256(self)
    }

    pub fn policy_snapshot_ref(&self) -> Result<BrokerPolicySnapshotRef, BrokerEnvelopeError> {
        let policy_snapshot_hash = self
            .policy_snapshot_hash
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .ok_or(BrokerEnvelopeError::MissingPolicySnapshot)?;
        Ok(BrokerPolicySnapshotRef {
            policy_snapshot_hash,
        })
    }

    pub fn approval_binding(&self) -> BrokerApprovalBinding {
        BrokerApprovalBinding {
            approval_ref: self.approval_ref.clone(),
            read_plan_ref: self.read_plan_ref.clone(),
            preimage_ref: self.preimage_ref.clone(),
        }
    }

    fn validate_tool_intent(&self) -> Result<(), BrokerEnvelopeError> {
        validate_schema_version(&self.schema_version)?;
        validate_nonempty("intent_id", &self.intent_id)?;
        validate_nonempty("session_id", &self.session_id)?;
        validate_nonempty("turn_id", &self.turn_id)?;
        self.actor.validate()?;
        validate_nonempty("capability_id", &self.capability_id)?;
        validate_nonempty("tool_name", &self.tool_name)?;
        validate_nonempty("resource_ref", &self.resource_ref)?;
        validate_nonempty("structured_args_hash", &self.structured_args_hash)?;
        validate_optional_nonempty("policy_snapshot_hash", &self.policy_snapshot_hash)?;
        self.approval_binding().validate()?;
        validate_nonempty("created_at", &self.created_at)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerDecision {
    pub schema_version: String,
    pub decision_id: String,
    pub intent_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub actor: BrokerSubject,
    pub capability_id: String,
    pub tool_name: String,
    pub operation_kind: BrokerOperationKind,
    pub resource_kind: BrokerResourceKind,
    pub resource_ref: String,
    pub structured_args_hash: String,
    pub policy_snapshot_hash: String,
    pub approval_ref: Option<String>,
    pub read_plan_ref: Option<String>,
    pub preimage_ref: Option<String>,
    pub execution_mode: BrokerExecutionMode,
    pub verdict: BrokerDecisionVerdict,
    pub reason_summary: String,
    pub decided_at: String,
}

impl BrokerDecision {
    pub fn canonical_hash(&self) -> Result<String, BrokerEnvelopeError> {
        canonical_sha256(self)
    }

    pub fn validate_for_intent(&self, intent: &BrokerIntent) -> Result<(), BrokerEnvelopeError> {
        validate_schema_version(&self.schema_version)?;
        validate_nonempty("decision_id", &self.decision_id)?;
        validate_nonempty("policy_snapshot_hash", &self.policy_snapshot_hash)?;
        validate_optional_nonempty("approval_ref", &self.approval_ref)?;
        validate_optional_nonempty("read_plan_ref", &self.read_plan_ref)?;
        validate_optional_nonempty("preimage_ref", &self.preimage_ref)?;
        validate_nonempty("reason_summary", &self.reason_summary)?;
        validate_nonempty("decided_at", &self.decided_at)?;
        self.actor.validate()?;
        require_match("intent_id", &self.intent_id, &intent.intent_id)?;
        require_match("session_id", &self.session_id, &intent.session_id)?;
        require_match("turn_id", &self.turn_id, &intent.turn_id)?;
        require_match("capability_id", &self.capability_id, &intent.capability_id)?;
        require_match("tool_name", &self.tool_name, &intent.tool_name)?;
        if self.operation_kind != intent.operation_kind {
            return Err(BrokerEnvelopeError::DecisionIntentMismatch(
                "operation_kind".to_string(),
            ));
        }
        if self.resource_kind != intent.resource_kind {
            return Err(BrokerEnvelopeError::DecisionIntentMismatch(
                "resource_kind".to_string(),
            ));
        }
        require_match("resource_ref", &self.resource_ref, &intent.resource_ref)?;
        require_match(
            "structured_args_hash",
            &self.structured_args_hash,
            &intent.structured_args_hash,
        )?;
        let policy_snapshot = intent.policy_snapshot_ref()?;
        require_match(
            "policy_snapshot_hash",
            &self.policy_snapshot_hash,
            &policy_snapshot.policy_snapshot_hash,
        )?;
        require_optional_match("approval_ref", &self.approval_ref, &intent.approval_ref)?;
        require_optional_match("read_plan_ref", &self.read_plan_ref, &intent.read_plan_ref)?;
        require_optional_match("preimage_ref", &self.preimage_ref, &intent.preimage_ref)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerExecutionRequest {
    pub schema_version: String,
    pub intent: BrokerIntent,
    pub decision: BrokerDecision,
    pub execution_mode: BrokerExecutionMode,
    pub created_at: String,
}

impl BrokerExecutionRequest {
    pub fn mediated(
        intent: BrokerIntent,
        decision: BrokerDecision,
        created_at: impl Into<String>,
    ) -> Result<Self, BrokerEnvelopeError> {
        decision.validate_for_intent(&intent)?;
        if decision.execution_mode != BrokerExecutionMode::MediatedL2 {
            return Err(BrokerEnvelopeError::DecisionNotMediated);
        }
        if decision.verdict != BrokerDecisionVerdict::Allow {
            return Err(BrokerEnvelopeError::DeniedByBrokerDecision);
        }
        if !intent.approval_binding().has_approval_or_read_plan() {
            return Err(BrokerEnvelopeError::MissingApprovalOrReadPlanBinding);
        }
        Ok(Self {
            schema_version: BROKER_ENVELOPE_SCHEMA_VERSION.to_string(),
            intent,
            decision,
            execution_mode: BrokerExecutionMode::MediatedL2,
            created_at: created_at.into(),
        })
    }

    pub fn canonical_hash(&self) -> Result<String, BrokerEnvelopeError> {
        canonical_sha256(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerExecutionResult {
    pub schema_version: String,
    pub intent_id: String,
    pub session_id: String,
    pub execution_mode: BrokerExecutionMode,
    pub result_hash: Option<String>,
    pub stdout_hash: Option<String>,
    pub stderr_hash: Option<String>,
    pub exit_status: Option<i32>,
    pub redaction_summary_hash: String,
    pub created_at: String,
}

impl BrokerExecutionResult {
    pub fn canonical_hash(&self) -> Result<String, BrokerEnvelopeError> {
        canonical_sha256(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerEvidenceRecord {
    pub schema_version: String,
    pub intent_id: String,
    pub session_id: String,
    pub execution_mode: BrokerExecutionMode,
    pub custody: BrokerCustody,
    pub policy_snapshot_hash: String,
    pub approval_ref: Option<String>,
    pub read_plan_ref: Option<String>,
    pub preimage_ref: Option<String>,
    pub broker_record_hash: Option<String>,
    pub result_hash: Option<String>,
    pub stdout_hash: Option<String>,
    pub stderr_hash: Option<String>,
    pub exit_status: Option<i32>,
    pub redaction_summary_hash: String,
    pub broker_signature_hash: Option<String>,
    pub created_at: String,
}

impl BrokerEvidenceRecord {
    pub fn validate_claim_boundary(&self) -> Result<(), BrokerEnvelopeError> {
        validate_schema_version(&self.schema_version)?;
        validate_nonempty("intent_id", &self.intent_id)?;
        validate_nonempty("session_id", &self.session_id)?;
        validate_nonempty("policy_snapshot_hash", &self.policy_snapshot_hash)?;
        validate_optional_nonempty("approval_ref", &self.approval_ref)?;
        validate_optional_nonempty("read_plan_ref", &self.read_plan_ref)?;
        validate_optional_nonempty("preimage_ref", &self.preimage_ref)?;
        validate_optional_nonempty("broker_record_hash", &self.broker_record_hash)?;
        validate_optional_nonempty("result_hash", &self.result_hash)?;
        validate_optional_nonempty("stdout_hash", &self.stdout_hash)?;
        validate_optional_nonempty("stderr_hash", &self.stderr_hash)?;
        validate_nonempty("redaction_summary_hash", &self.redaction_summary_hash)?;
        validate_optional_nonempty("broker_signature_hash", &self.broker_signature_hash)?;
        validate_nonempty("created_at", &self.created_at)?;
        if self.execution_mode == BrokerExecutionMode::MediatedL2
            && !option_is_nonempty(&self.broker_record_hash)
        {
            return Err(BrokerEnvelopeError::MissingBrokerRecordHash);
        }
        if self.custody == BrokerCustody::BrokerAttested
            && !option_is_nonempty(&self.broker_signature_hash)
        {
            return Err(BrokerEnvelopeError::MissingBrokerSignatureHash);
        }
        Ok(())
    }

    pub fn canonical_hash(&self) -> Result<String, BrokerEnvelopeError> {
        canonical_sha256(self)
    }
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, BrokerEnvelopeError> {
    let value =
        serde_json::to_value(value).map_err(|err| BrokerEnvelopeError::Json(err.to_string()))?;
    let canonical = canonical_json(&value)?;
    Ok(sha256_prefixed(canonical.as_bytes()))
}

pub fn canonical_json(value: &Value) -> Result<String, BrokerEnvelopeError> {
    let mut out = String::new();
    write_canonical_json(value, &mut out)?;
    Ok(out)
}

fn write_canonical_json(value: &Value, out: &mut String) -> Result<(), BrokerEnvelopeError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => out.push_str(&value.to_string()),
        Value::String(value) => out.push_str(
            &serde_json::to_string(value)
                .map_err(|err| BrokerEnvelopeError::Json(err.to_string()))?,
        ),
        Value::Array(values) => {
            out.push('[');
            for (idx, item) in values.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            for (idx, key) in keys.into_iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(
                    &serde_json::to_string(key)
                        .map_err(|err| BrokerEnvelopeError::Json(err.to_string()))?,
                );
                out.push(':');
                if let Some(item) = map.get(key) {
                    write_canonical_json(item, out)?;
                }
            }
            out.push('}');
        }
    }
    Ok(())
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity("sha256:".len() + digest.len() * 2);
    out.push_str("sha256:");
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn find_tool_supplied_authority_field(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                if is_forbidden_tool_authority_key(key) {
                    return Some(key.clone());
                }
                if let Some(found) = find_tool_supplied_authority_field(item) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(values) => values.iter().find_map(find_tool_supplied_authority_field),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn find_tool_supplied_mediated_mode(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                if key == "execution_mode" && item.as_str() == Some("mediated_l2") {
                    return Some("mediated_l2".to_string());
                }
                if let Some(found) = find_tool_supplied_mediated_mode(item) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(values) => values.iter().find_map(find_tool_supplied_mediated_mode),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn is_forbidden_tool_authority_key(key: &str) -> bool {
    matches!(
        key,
        "allow"
            | "broker_record_hash"
            | "broker_signature_hash"
            | "broker_attested"
            | "custody"
            | "decision"
            | "decision_id"
            | "decision_verdict"
            | "deny"
            | "policy_decision"
            | "verdict"
    )
}

fn validate_schema_version(value: &str) -> Result<(), BrokerEnvelopeError> {
    if value == BROKER_ENVELOPE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(BrokerEnvelopeError::InvalidRequiredField(
            "schema_version".to_string(),
        ))
    }
}

fn validate_nonempty(field: &str, value: &str) -> Result<(), BrokerEnvelopeError> {
    if value.trim().is_empty() {
        Err(BrokerEnvelopeError::InvalidRequiredField(field.to_string()))
    } else {
        Ok(())
    }
}

fn validate_optional_nonempty(
    field: &str,
    value: &Option<String>,
) -> Result<(), BrokerEnvelopeError> {
    if value.as_ref().is_some_and(|value| value.trim().is_empty()) {
        Err(BrokerEnvelopeError::InvalidRequiredField(field.to_string()))
    } else {
        Ok(())
    }
}

fn require_match(field: &str, left: &str, right: &str) -> Result<(), BrokerEnvelopeError> {
    if left == right {
        Ok(())
    } else {
        Err(BrokerEnvelopeError::DecisionIntentMismatch(
            field.to_string(),
        ))
    }
}

fn require_optional_match(
    field: &str,
    left: &Option<String>,
    right: &Option<String>,
) -> Result<(), BrokerEnvelopeError> {
    if left.as_deref() == right.as_deref() {
        Ok(())
    } else {
        Err(BrokerEnvelopeError::DecisionIntentMismatch(
            field.to_string(),
        ))
    }
}

fn option_is_nonempty(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn actor() -> BrokerSubject {
        BrokerSubject {
            subject_kind: BrokerSubjectKind::Agent,
            subject_id: "agent.session".to_string(),
        }
    }

    fn base_payload() -> Value {
        json!({
            "schema_version": BROKER_ENVELOPE_SCHEMA_VERSION,
            "intent_id": "intent.1",
            "session_id": "session.1",
            "turn_id": "turn.1",
            "actor": {
                "subject_kind": "agent",
                "subject_id": "agent.session"
            },
            "capability_id": "vac.runtime.broker",
            "tool_name": "view",
            "operation_kind": "filesystem_read",
            "resource_kind": "filesystem",
            "resource_ref": "README.md",
            "structured_args_hash": "sha256:args",
            "policy_snapshot_hash": "sha256:policy",
            "approval_ref": null,
            "read_plan_ref": "read-plan.1",
            "preimage_ref": null,
            "created_at": "2026-06-18T00:00:00Z"
        })
    }

    fn intent() -> BrokerIntent {
        BrokerIntent::from_tool_payload(base_payload()).expect("valid intent")
    }

    fn decision_for(intent: &BrokerIntent) -> BrokerDecision {
        BrokerDecision {
            schema_version: BROKER_ENVELOPE_SCHEMA_VERSION.to_string(),
            decision_id: "decision.1".to_string(),
            intent_id: intent.intent_id.clone(),
            session_id: intent.session_id.clone(),
            turn_id: intent.turn_id.clone(),
            actor: actor(),
            capability_id: intent.capability_id.clone(),
            tool_name: intent.tool_name.clone(),
            operation_kind: intent.operation_kind.clone(),
            resource_kind: intent.resource_kind.clone(),
            resource_ref: intent.resource_ref.clone(),
            structured_args_hash: intent.structured_args_hash.clone(),
            policy_snapshot_hash: intent
                .policy_snapshot_hash
                .clone()
                .expect("policy snapshot"),
            approval_ref: intent.approval_ref.clone(),
            read_plan_ref: intent.read_plan_ref.clone(),
            preimage_ref: intent.preimage_ref.clone(),
            execution_mode: BrokerExecutionMode::MediatedL2,
            verdict: BrokerDecisionVerdict::Allow,
            reason_summary: "policy snapshot permits bounded read".to_string(),
            decided_at: "2026-06-18T00:00:01Z".to_string(),
        }
    }

    #[test]
    fn structured_intent_canonical_hash_is_stable() {
        let left = json!({"b": [2, 1], "a": {"z": true, "m": null}});
        let right = json!({"a": {"m": null, "z": true}, "b": [2, 1]});

        assert_eq!(
            canonical_json(&left).expect("left canonical json"),
            canonical_json(&right).expect("right canonical json")
        );
        assert_eq!(
            canonical_sha256(&left).expect("left hash"),
            canonical_sha256(&right).expect("right hash")
        );
    }

    #[test]
    fn tool_cannot_supply_policy_decision() {
        let mut payload = base_payload();
        payload["policy_decision"] = json!("allow");

        assert!(matches!(
            BrokerIntent::from_tool_payload(payload),
            Err(BrokerEnvelopeError::ToolSuppliedAuthorityField(field)) if field == "policy_decision"
        ));
    }

    #[test]
    fn tool_cannot_mark_itself_mediated_l2() {
        let mut payload = base_payload();
        payload["execution_mode"] = json!("mediated_l2");

        assert!(matches!(
            BrokerIntent::from_tool_payload(payload),
            Err(BrokerEnvelopeError::ToolSuppliedExecutionMode(mode)) if mode == "mediated_l2"
        ));
    }

    #[test]
    fn tool_cannot_self_assign_broker_attested_custody() {
        let mut payload = base_payload();
        payload["custody"] = json!("broker_attested");

        assert!(matches!(
            BrokerIntent::from_tool_payload(payload),
            Err(BrokerEnvelopeError::ToolSuppliedAuthorityField(field)) if field == "custody"
        ));
    }

    #[test]
    fn missing_policy_snapshot_blocks_mediation() {
        let mut intent = intent();
        intent.policy_snapshot_hash = None;
        let decision = decision_for(&BrokerIntent {
            policy_snapshot_hash: Some("sha256:policy".to_string()),
            ..intent.clone()
        });

        assert!(matches!(
            BrokerExecutionRequest::mediated(intent, decision, "2026-06-18T00:00:02Z"),
            Err(BrokerEnvelopeError::MissingPolicySnapshot)
        ));
    }

    #[test]
    fn missing_approval_or_read_plan_blocks_mediation() {
        let mut intent = intent();
        intent.approval_ref = None;
        intent.read_plan_ref = None;
        let mut decision = decision_for(&BrokerIntent {
            approval_ref: Some("approval.1".to_string()),
            read_plan_ref: None,
            ..intent.clone()
        });
        decision.approval_ref = None;
        decision.read_plan_ref = None;

        assert!(matches!(
            BrokerExecutionRequest::mediated(intent, decision, "2026-06-18T00:00:02Z"),
            Err(BrokerEnvelopeError::MissingApprovalOrReadPlanBinding)
        ));
    }

    #[test]
    fn mediated_l2_evidence_requires_broker_record_hash() {
        let evidence = BrokerEvidenceRecord {
            schema_version: BROKER_ENVELOPE_SCHEMA_VERSION.to_string(),
            intent_id: "intent.1".to_string(),
            session_id: "session.1".to_string(),
            execution_mode: BrokerExecutionMode::MediatedL2,
            custody: BrokerCustody::LocalOnly,
            policy_snapshot_hash: "sha256:policy".to_string(),
            approval_ref: None,
            read_plan_ref: Some("read-plan.1".to_string()),
            preimage_ref: None,
            broker_record_hash: None,
            result_hash: Some("sha256:result".to_string()),
            stdout_hash: Some("sha256:stdout".to_string()),
            stderr_hash: Some("sha256:stderr".to_string()),
            exit_status: Some(0),
            redaction_summary_hash: "sha256:redaction".to_string(),
            broker_signature_hash: None,
            created_at: "2026-06-18T00:00:03Z".to_string(),
        };

        assert!(matches!(
            evidence.validate_claim_boundary(),
            Err(BrokerEnvelopeError::MissingBrokerRecordHash)
        ));
    }

    #[test]
    fn broker_attested_evidence_requires_signature_hash() {
        let evidence = BrokerEvidenceRecord {
            schema_version: BROKER_ENVELOPE_SCHEMA_VERSION.to_string(),
            intent_id: "intent.1".to_string(),
            session_id: "session.1".to_string(),
            execution_mode: BrokerExecutionMode::MediatedL2,
            custody: BrokerCustody::BrokerAttested,
            policy_snapshot_hash: "sha256:policy".to_string(),
            approval_ref: None,
            read_plan_ref: Some("read-plan.1".to_string()),
            preimage_ref: None,
            broker_record_hash: Some("sha256:broker-record".to_string()),
            result_hash: Some("sha256:result".to_string()),
            stdout_hash: Some("sha256:stdout".to_string()),
            stderr_hash: Some("sha256:stderr".to_string()),
            exit_status: Some(0),
            redaction_summary_hash: "sha256:redaction".to_string(),
            broker_signature_hash: None,
            created_at: "2026-06-18T00:00:03Z".to_string(),
        };

        assert!(matches!(
            evidence.validate_claim_boundary(),
            Err(BrokerEnvelopeError::MissingBrokerSignatureHash)
        ));
    }
}
