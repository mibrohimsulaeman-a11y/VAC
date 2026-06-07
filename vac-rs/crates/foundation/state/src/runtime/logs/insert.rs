use super::*;

impl StateRuntime {
    pub async fn insert_log(&self, entry: &LogEntry) -> anyhow::Result<()> {
        self.insert_logs(std::slice::from_ref(entry)).await
    }

    /// Insert a batch of log entries into the logs table.
    pub async fn insert_logs(&self, entries: &[LogEntry]) -> anyhow::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut tx = self.logs_pool.begin().await?;
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO logs (ts, ts_nanos, level, target, feedback_log_body, thread_id, process_uuid, module_path, file, line, estimated_bytes) ",
        );
        builder.push_values(entries, |mut row, entry| {
            let feedback_log_body = entry.feedback_log_body.as_ref().or(entry.message.as_ref());
            // Keep about 10 MiB of reader-visible log content per partition.
            // Both `query_logs` and `/feedback` read the persisted
            // `feedback_log_body`, while `LogEntry.message` is only a write-time
            // fallback for callers that still populate the old field.
            let estimated_bytes = feedback_log_body.map_or(0, String::len) as i64
                + entry.level.len() as i64
                + entry.target.len() as i64
                + entry.module_path.as_ref().map_or(0, String::len) as i64
                + entry.file.as_ref().map_or(0, String::len) as i64;
            row.push_bind(entry.ts)
                .push_bind(entry.ts_nanos)
                .push_bind(&entry.level)
                .push_bind(&entry.target)
                .push_bind(feedback_log_body)
                .push_bind(&entry.thread_id)
                .push_bind(&entry.process_uuid)
                .push_bind(&entry.module_path)
                .push_bind(&entry.file)
                .push_bind(entry.line)
                .push_bind(estimated_bytes);
        });
        builder.build().execute(&mut *tx).await?;
        self.prune_logs_after_insert(entries, &mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Enforce per-partition retained-log-content caps after a successful batch insert.
    ///
    /// We maintain two independent budgets:
    /// - Thread logs: rows with `thread_id IS NOT NULL`, capped per `thread_id`.
    /// - Threadless process logs: rows with `thread_id IS NULL` ("threadless"),
    ///   capped per `process_uuid` (including `process_uuid IS NULL` as its own
    ///   threadless partition).
    ///
    /// "Threadless" means the log row is not associated with any conversation
    /// thread, so retention is keyed by process identity instead.
    ///
    /// This runs inside the same transaction as the insert so callers never
    /// observe "inserted but not yet pruned" rows.
    pub(super) async fn prune_logs_after_insert(
        &self,
        entries: &[LogEntry],
        tx: &mut SqliteConnection,
    ) -> anyhow::Result<()> {
        let thread_ids: BTreeSet<&str> = entries
            .iter()
            .filter_map(|entry| entry.thread_id.as_deref())
            .collect();
        if !thread_ids.is_empty() {
            // Cheap precheck: only run the heavier window-function prune for
            // threads that are currently above the cap.
            let mut over_limit_threads_query =
                QueryBuilder::<Sqlite>::new("SELECT thread_id FROM logs WHERE thread_id IN (");
            {
                let mut separated = over_limit_threads_query.separated(", ");
                for thread_id in &thread_ids {
                    separated.push_bind(*thread_id);
                }
            }
            over_limit_threads_query.push(") GROUP BY thread_id HAVING SUM(");
            over_limit_threads_query.push("estimated_bytes");
            over_limit_threads_query.push(") > ");
            over_limit_threads_query.push_bind(LOG_PARTITION_SIZE_LIMIT_BYTES);
            over_limit_threads_query.push(" OR COUNT(*) > ");
            over_limit_threads_query.push_bind(LOG_PARTITION_ROW_LIMIT);
            let over_limit_thread_ids: Vec<String> = over_limit_threads_query
                .build()
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .map(|row| row.try_get("thread_id"))
                .collect::<Result<_, _>>()?;
            if !over_limit_thread_ids.is_empty() {
                // Enforce a strict per-thread cap by deleting every row whose
                // newest-first cumulative bytes exceed the partition budget.
                let mut prune_threads = QueryBuilder::<Sqlite>::new(
                    r#"
DELETE FROM logs
WHERE id IN (
    SELECT id
    FROM (
        SELECT
            id,
            SUM(
"#,
                );
                prune_threads.push("estimated_bytes");
                prune_threads.push(
                    r#"
            ) OVER (
                PARTITION BY thread_id
                ORDER BY ts DESC, ts_nanos DESC, id DESC
            ) AS cumulative_bytes,
            ROW_NUMBER() OVER (
                PARTITION BY thread_id
                ORDER BY ts DESC, ts_nanos DESC, id DESC
            ) AS row_number
        FROM logs
        WHERE thread_id IN (
"#,
                );
                {
                    let mut separated = prune_threads.separated(", ");
                    for thread_id in &over_limit_thread_ids {
                        separated.push_bind(thread_id);
                    }
                }
                prune_threads.push(
                    r#"
        )
    )
    WHERE cumulative_bytes >
"#,
                );
                prune_threads.push_bind(LOG_PARTITION_SIZE_LIMIT_BYTES);
                prune_threads.push(" OR row_number > ");
                prune_threads.push_bind(LOG_PARTITION_ROW_LIMIT);
                prune_threads.push("\n)");
                prune_threads.build().execute(&mut *tx).await?;
            }
        }

        let threadless_process_uuids: BTreeSet<&str> = entries
            .iter()
            .filter(|entry| entry.thread_id.is_none())
            .filter_map(|entry| entry.process_uuid.as_deref())
            .collect();
        let has_threadless_null_process_uuid = entries
            .iter()
            .any(|entry| entry.thread_id.is_none() && entry.process_uuid.is_none());
        if !threadless_process_uuids.is_empty() {
            // Threadless logs are budgeted separately per process UUID.
            let mut over_limit_processes_query = QueryBuilder::<Sqlite>::new(
                "SELECT process_uuid FROM logs WHERE thread_id IS NULL AND process_uuid IN (",
            );
            {
                let mut separated = over_limit_processes_query.separated(", ");
                for process_uuid in &threadless_process_uuids {
                    separated.push_bind(*process_uuid);
                }
            }
            over_limit_processes_query.push(") GROUP BY process_uuid HAVING SUM(");
            over_limit_processes_query.push("estimated_bytes");
            over_limit_processes_query.push(") > ");
            over_limit_processes_query.push_bind(LOG_PARTITION_SIZE_LIMIT_BYTES);
            over_limit_processes_query.push(" OR COUNT(*) > ");
            over_limit_processes_query.push_bind(LOG_PARTITION_ROW_LIMIT);
            let over_limit_process_uuids: Vec<String> = over_limit_processes_query
                .build()
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .map(|row| row.try_get("process_uuid"))
                .collect::<Result<_, _>>()?;
            if !over_limit_process_uuids.is_empty() {
                // Same strict cap policy as thread pruning, but only for
                // threadless rows in the affected process UUIDs.
                let mut prune_threadless_process_logs = QueryBuilder::<Sqlite>::new(
                    r#"
DELETE FROM logs
WHERE id IN (
    SELECT id
    FROM (
        SELECT
            id,
            SUM(
"#,
                );
                prune_threadless_process_logs.push("estimated_bytes");
                prune_threadless_process_logs.push(
                    r#"
            ) OVER (
                PARTITION BY process_uuid
                ORDER BY ts DESC, ts_nanos DESC, id DESC
            ) AS cumulative_bytes,
            ROW_NUMBER() OVER (
                PARTITION BY process_uuid
                ORDER BY ts DESC, ts_nanos DESC, id DESC
            ) AS row_number
        FROM logs
        WHERE thread_id IS NULL
          AND process_uuid IN (
"#,
                );
                {
                    let mut separated = prune_threadless_process_logs.separated(", ");
                    for process_uuid in &over_limit_process_uuids {
                        separated.push_bind(process_uuid);
                    }
                }
                prune_threadless_process_logs.push(
                    r#"
          )
    )
    WHERE cumulative_bytes >
"#,
                );
                prune_threadless_process_logs.push_bind(LOG_PARTITION_SIZE_LIMIT_BYTES);
                prune_threadless_process_logs.push(" OR row_number > ");
                prune_threadless_process_logs.push_bind(LOG_PARTITION_ROW_LIMIT);
                prune_threadless_process_logs.push("\n)");
                prune_threadless_process_logs
                    .build()
                    .execute(&mut *tx)
                    .await?;
            }
        }
        if has_threadless_null_process_uuid {
            // Rows without a process UUID still need a cap; treat NULL as its
            // own threadless partition.
            let mut null_process_usage_query = QueryBuilder::<Sqlite>::new("SELECT SUM(");
            null_process_usage_query.push("estimated_bytes");
            null_process_usage_query.push(
                ") AS total_bytes, COUNT(*) AS row_count FROM logs WHERE thread_id IS NULL AND process_uuid IS NULL",
            );
            let null_process_usage = null_process_usage_query.build().fetch_one(&mut *tx).await?;
            let total_null_process_bytes: Option<i64> =
                null_process_usage.try_get("total_bytes")?;
            let null_process_row_count: i64 = null_process_usage.try_get("row_count")?;

            if total_null_process_bytes.unwrap_or(0) > LOG_PARTITION_SIZE_LIMIT_BYTES
                || null_process_row_count > LOG_PARTITION_ROW_LIMIT
            {
                let mut prune_threadless_null_process_logs = QueryBuilder::<Sqlite>::new(
                    r#"
DELETE FROM logs
WHERE id IN (
    SELECT id
    FROM (
        SELECT
            id,
            SUM(
"#,
                );
                prune_threadless_null_process_logs.push("estimated_bytes");
                prune_threadless_null_process_logs.push(
                    r#"
            ) OVER (
                PARTITION BY process_uuid
                ORDER BY ts DESC, ts_nanos DESC, id DESC
            ) AS cumulative_bytes,
            ROW_NUMBER() OVER (
                PARTITION BY process_uuid
                ORDER BY ts DESC, ts_nanos DESC, id DESC
            ) AS row_number
        FROM logs
        WHERE thread_id IS NULL
          AND process_uuid IS NULL
    )
    WHERE cumulative_bytes >
"#,
                );
                prune_threadless_null_process_logs.push_bind(LOG_PARTITION_SIZE_LIMIT_BYTES);
                prune_threadless_null_process_logs.push(" OR row_number > ");
                prune_threadless_null_process_logs.push_bind(LOG_PARTITION_ROW_LIMIT);
                prune_threadless_null_process_logs.push("\n)");
                prune_threadless_null_process_logs
                    .build()
                    .execute(&mut *tx)
                    .await?;
            }
        }
        Ok(())
    }
}
