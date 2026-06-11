#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelSnapshot {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageSnapshot {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnforcementSnapshot {
    pub level: String,
    pub guarantee: String,
}

impl EnforcementSnapshot {
    pub fn l1() -> Self {
        Self {
            level: "L1".into(),
            guarantee: "advisory discipline + audit".into(),
        }
    }
}

pub mod runtime_journal;
pub use runtime_journal::{
    RUNTIME_DB_REQUIRED_PRAGMAS, RUNTIME_DB_REQUIRED_TABLES, RuntimeJournalAppendDecision,
    RuntimeJournalEventDraft, RuntimeJournalOpenRequest, RuntimeJournalRecordEnvelope,
    RuntimeJournalWritePlan, RuntimeManifestBinding, RuntimeTrustClaim,
    evaluate_runtime_event_append, runtime_db_migration_has_required_pragmas,
    runtime_journal_write_plan,
};
