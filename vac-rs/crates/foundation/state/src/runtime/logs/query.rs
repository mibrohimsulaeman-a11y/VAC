use super::*;

impl StateRuntime {
    pub(crate) async fn delete_logs_before(&self, cutoff_ts: i64) -> anyhow::Result<u64> {
        let result = sqlx::query("DELETE FROM logs WHERE ts < ?")
            .bind(cutoff_ts)
            .execute(self.logs_pool.as_ref())
            .await?;
        Ok(result.rows_affected())
    }

    pub(crate) async fn run_logs_startup_maintenance(&self) -> anyhow::Result<()> {
        let Some(cutoff) =
            Utc::now().checked_sub_signed(chrono::Duration::days(LOG_RETENTION_DAYS))
        else {
            return Ok(());
        };
        self.delete_logs_before(cutoff.timestamp()).await?;
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(self.logs_pool.as_ref())
            .await?;
        sqlx::query("PRAGMA incremental_vacuum")
            .execute(self.logs_pool.as_ref())
            .await?;
        Ok(())
    }

    /// Query logs with optional filters.
    pub async fn query_logs(&self, query: &LogQuery) -> anyhow::Result<Vec<LogRow>> {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT id, ts, ts_nanos, level, target, feedback_log_body AS message, thread_id, process_uuid, file, line FROM logs WHERE 1 = 1",
        );
        push_log_filters(&mut builder, query);
        if query.descending {
            builder.push(" ORDER BY id DESC");
        } else {
            builder.push(" ORDER BY id ASC");
        }
        if let Some(limit) = query.limit {
            builder.push(" LIMIT ").push_bind(limit as i64);
        }

        let rows = builder
            .build_query_as::<LogRow>()
            .fetch_all(self.logs_pool.as_ref())
            .await?;
        Ok(rows)
    }

    /// Query feedback logs for a set of threads, capped to the SQLite retention budget.
    pub async fn query_feedback_logs_for_threads(
        &self,
        thread_ids: &[&str],
    ) -> anyhow::Result<Vec<u8>> {
        if thread_ids.is_empty() {
            return Ok(Vec::new());
        }

        let max_bytes = usize::try_from(LOG_PARTITION_SIZE_LIMIT_BYTES).unwrap_or(usize::MAX);
        // Bound the fetched rows in SQL first so over-retained partitions do not have to load
        // every row into memory, then apply the exact whole-line byte cap after formatting.
        let requested_threads = vec!["(?)"; thread_ids.len()].join(", ");
        let query = format!(
            r#"
WITH requested_threads(thread_id) AS (
    VALUES {requested_threads}
),
latest_processes AS (
    SELECT (
        SELECT process_uuid
        FROM logs
        WHERE logs.thread_id = requested_threads.thread_id AND process_uuid IS NOT NULL
        ORDER BY ts DESC, ts_nanos DESC, id DESC
        LIMIT 1
    ) AS process_uuid
    FROM requested_threads
),
feedback_logs AS (
    SELECT ts, ts_nanos, level, feedback_log_body, estimated_bytes, id
    FROM logs
    WHERE feedback_log_body IS NOT NULL AND (
        thread_id IN (SELECT thread_id FROM requested_threads)
        OR (
            thread_id IS NULL
            AND process_uuid IN (
                SELECT process_uuid
                FROM latest_processes
                WHERE process_uuid IS NOT NULL
            )
        )
    )
),
bounded_feedback_logs AS (
    SELECT
        ts,
        ts_nanos,
        level,
        feedback_log_body,
        id,
        SUM(estimated_bytes) OVER (
            ORDER BY ts DESC, ts_nanos DESC, id DESC
        ) AS cumulative_estimated_bytes
    FROM feedback_logs
)
SELECT ts, ts_nanos, level, feedback_log_body
FROM bounded_feedback_logs
WHERE cumulative_estimated_bytes <= ?
ORDER BY ts DESC, ts_nanos DESC, id DESC
"#
        );
        let mut sql = sqlx::query_as::<_, FeedbackLogRow>(query.as_str());
        for thread_id in thread_ids {
            sql = sql.bind(thread_id);
        }
        let rows = sql
            .bind(LOG_PARTITION_SIZE_LIMIT_BYTES)
            .fetch_all(self.logs_pool.as_ref())
            .await?;

        let mut lines = Vec::new();
        let mut total_bytes = 0usize;
        for row in rows {
            let line =
                format_feedback_log_line(row.ts, row.ts_nanos, &row.level, &row.feedback_log_body);
            if total_bytes.saturating_add(line.len()) > max_bytes {
                break;
            }
            total_bytes += line.len();
            lines.push(line);
        }

        let mut ordered_bytes = Vec::with_capacity(total_bytes);
        for line in lines.into_iter().rev() {
            ordered_bytes.extend_from_slice(line.as_bytes());
        }

        Ok(ordered_bytes)
    }

    /// Query per-thread feedback logs, capped to the per-thread SQLite retention budget.
    pub async fn query_feedback_logs(&self, thread_id: &str) -> anyhow::Result<Vec<u8>> {
        self.query_feedback_logs_for_threads(&[thread_id]).await
    }

    /// Return the max log id matching optional filters.
    pub async fn max_log_id(&self, query: &LogQuery) -> anyhow::Result<i64> {
        let mut builder =
            QueryBuilder::<Sqlite>::new("SELECT MAX(id) AS max_id FROM logs WHERE 1 = 1");
        push_log_filters(&mut builder, query);
        let row = builder.build().fetch_one(self.logs_pool.as_ref()).await?;
        let max_id: Option<i64> = row.try_get("max_id")?;
        Ok(max_id.unwrap_or(0))
    }
}
