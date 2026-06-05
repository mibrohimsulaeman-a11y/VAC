use anyhow::Result;
use sqlx::SqlitePool;

use crate::episodic::count_repeated_failures;

pub const REPEATED_FAILURE_THRESHOLD: i64 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiLoopDecision {
    pub blocked: bool,
    pub repeated_failures: i64,
    pub requires_new_hypothesis: bool,
}

pub async fn evaluate_repeated_failure(
    pool: &SqlitePool,
    session_id: &str,
    action_taken: &str,
) -> Result<AntiLoopDecision> {
    let repeated_failures = count_repeated_failures(pool, session_id, action_taken).await?;
    Ok(AntiLoopDecision {
        blocked: repeated_failures >= REPEATED_FAILURE_THRESHOLD,
        repeated_failures,
        requires_new_hypothesis: repeated_failures >= REPEATED_FAILURE_THRESHOLD,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic::EpisodicTrace;
    use crate::episodic::ExecutionSession;
    use crate::episodic::open_episodic_db;
    use crate::episodic::upsert_execution_session;
    use crate::episodic::write_episodic_trace;

    #[tokio::test]
    async fn anti_loop_blocks_fourth_same_failed_action() {
        let temp = tempfile::tempdir().unwrap();
        let pool = open_episodic_db(temp.path().join("episodic.db"))
            .await
            .unwrap();
        upsert_execution_session(
            &pool,
            &ExecutionSession {
                session_id: "session.test".to_string(),
                status: "open".to_string(),
                current_task: "fix tests".to_string(),
            },
        )
        .await
        .unwrap();
        for index in 0..3 {
            write_episodic_trace(
                &pool,
                &EpisodicTrace {
                    trace_id: format!("trace.{index}"),
                    session_id: "session.test".to_string(),
                    phase_name: "validation".to_string(),
                    action_taken: "same fix".to_string(),
                    outcome_status: "failure".to_string(),
                    output_summary: "failed".to_string(),
                    recovery_hypothesis: Some("same hypothesis".to_string()),
                },
            )
            .await
            .unwrap();
        }
        let decision = evaluate_repeated_failure(&pool, "session.test", "same fix")
            .await
            .unwrap();
        assert!(decision.blocked);
        assert!(decision.requires_new_hypothesis);
    }
}
