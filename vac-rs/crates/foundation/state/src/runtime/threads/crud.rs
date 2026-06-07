use super::*;

impl StateRuntime {
    /// Find a rollout path by thread id using the underlying database.
    pub async fn find_rollout_path_by_id(
        &self,
        id: ThreadId,
        archived_only: Option<bool>,
    ) -> anyhow::Result<Option<PathBuf>> {
        let mut builder =
            QueryBuilder::<Sqlite>::new("SELECT rollout_path FROM threads WHERE id = ");
        builder.push_bind(id.to_string());
        match archived_only {
            Some(true) => {
                builder.push(" AND archived = 1");
            }
            Some(false) => {
                builder.push(" AND archived = 0");
            }
            None => {}
        }
        let row = builder.build().fetch_optional(self.pool.as_ref()).await?;
        Ok(row
            .and_then(|r| r.try_get::<String, _>("rollout_path").ok())
            .map(PathBuf::from))
    }

    /// Find the newest thread whose user-facing title exactly matches `title`.
    #[allow(clippy::too_many_arguments)]
    pub async fn find_thread_by_exact_title(
        &self,
        title: &str,
        allowed_sources: &[String],
        model_providers: Option<&[String]>,
        archived_only: bool,
        cwd: Option<&Path>,
    ) -> anyhow::Result<Option<crate::ThreadMetadata>> {
        let mut builder = QueryBuilder::<Sqlite>::new("");
        push_thread_select_columns(&mut builder);
        builder.push(" FROM threads");
        push_thread_filters(
            &mut builder,
            ThreadFilterOptions {
                archived_only,
                allowed_sources,
                model_providers,
                cwd_filters: None,
                anchor: None,
                sort_key: crate::SortKey::UpdatedAt,
                sort_direction: SortDirection::Desc,
                search_term: None,
            },
        );
        builder.push(" AND threads.title = ");
        builder.push_bind(title);
        if let Some(cwd) = cwd {
            builder.push(" AND threads.cwd = ");
            builder.push_bind(cwd.display().to_string());
        }
        push_thread_order_and_limit(
            &mut builder,
            crate::SortKey::UpdatedAt,
            SortDirection::Desc,
            /*limit*/ 1,
        );

        let row = builder.build().fetch_optional(self.pool.as_ref()).await?;
        row.map(|row| ThreadRow::try_from_row(&row).and_then(crate::ThreadMetadata::try_from))
            .transpose()
    }

    /// List threads using the underlying database.
    pub async fn list_threads(
        &self,
        page_size: usize,
        filters: ThreadFilterOptions<'_>,
    ) -> anyhow::Result<crate::ThreadsPage> {
        let limit = page_size.saturating_add(1);
        let sort_key = filters.sort_key;
        let sort_direction = filters.sort_direction;

        let mut builder = QueryBuilder::<Sqlite>::new("");
        push_thread_select_columns(&mut builder);
        builder.push(" FROM threads");
        push_thread_filters(&mut builder, filters);
        push_thread_order_and_limit(&mut builder, sort_key, sort_direction, limit);

        let rows = builder.build().fetch_all(self.pool.as_ref()).await?;
        let mut items = rows
            .into_iter()
            .map(|row| ThreadRow::try_from_row(&row).and_then(ThreadMetadata::try_from))
            .collect::<Result<Vec<_>, _>>()?;
        let num_scanned_rows = items.len();
        let next_anchor = if items.len() > page_size {
            items.pop();
            items
                .last()
                .and_then(|item| anchor_from_item(item, sort_key))
        } else {
            None
        };
        Ok(ThreadsPage {
            items,
            next_anchor,
            num_scanned_rows,
        })
    }

    /// List thread ids using the underlying database (no rollout scanning).
    pub async fn list_thread_ids(
        &self,
        limit: usize,
        anchor: Option<&crate::Anchor>,
        sort_key: crate::SortKey,
        allowed_sources: &[String],
        model_providers: Option<&[String]>,
        archived_only: bool,
    ) -> anyhow::Result<Vec<ThreadId>> {
        let mut builder = QueryBuilder::<Sqlite>::new("SELECT threads.id FROM threads");
        push_thread_filters(
            &mut builder,
            ThreadFilterOptions {
                archived_only,
                allowed_sources,
                model_providers,
                cwd_filters: None,
                anchor,
                sort_key,
                sort_direction: SortDirection::Desc,
                search_term: None,
            },
        );
        push_thread_order_and_limit(&mut builder, sort_key, SortDirection::Desc, limit);

        let rows = builder.build().fetch_all(self.pool.as_ref()).await?;
        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                Ok(ThreadId::try_from(id)?)
            })
            .collect()
    }

    /// Insert or replace thread metadata directly.
    pub async fn upsert_thread(&self, metadata: &crate::ThreadMetadata) -> anyhow::Result<()> {
        self.upsert_thread_with_creation_memory_mode(metadata, /*creation_memory_mode*/ None)
            .await
    }

    pub async fn insert_thread_if_absent(
        &self,
        metadata: &crate::ThreadMetadata,
    ) -> anyhow::Result<bool> {
        let updated_at = self.allocate_thread_updated_at(metadata.updated_at)?;
        let result = sqlx::query(
            r#"
INSERT INTO threads (
    id,
    rollout_path,
    created_at,
    updated_at,
    created_at_ms,
    updated_at_ms,
    source,
    agent_nickname,
    agent_role,
    agent_path,
    model_provider,
    model,
    reasoning_effort,
    cwd,
    cli_version,
    title,
    sandbox_policy,
    approval_mode,
    tokens_used,
    first_user_message,
    archived,
    archived_at,
    git_sha,
    git_branch,
    git_origin_url,
    memory_mode
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(id) DO NOTHING
            "#,
        )
        .bind(metadata.id.to_string())
        .bind(metadata.rollout_path.display().to_string())
        .bind(datetime_to_epoch_seconds(metadata.created_at))
        .bind(datetime_to_epoch_seconds(updated_at))
        .bind(datetime_to_epoch_millis(metadata.created_at))
        .bind(datetime_to_epoch_millis(updated_at))
        .bind(metadata.source.as_str())
        .bind(metadata.agent_nickname.as_deref())
        .bind(metadata.agent_role.as_deref())
        .bind(metadata.agent_path.as_deref())
        .bind(metadata.model_provider.as_str())
        .bind(metadata.model.as_deref())
        .bind(
            metadata
                .reasoning_effort
                .as_ref()
                .map(crate::extract::enum_to_string),
        )
        .bind(metadata.cwd.display().to_string())
        .bind(metadata.cli_version.as_str())
        .bind(metadata.title.as_str())
        .bind(metadata.sandbox_policy.as_str())
        .bind(metadata.approval_mode.as_str())
        .bind(metadata.tokens_used)
        .bind(metadata.first_user_message.as_deref().unwrap_or_default())
        .bind(metadata.archived_at.is_some())
        .bind(metadata.archived_at.map(datetime_to_epoch_seconds))
        .bind(metadata.git_sha.as_deref())
        .bind(metadata.git_branch.as_deref())
        .bind(metadata.git_origin_url.as_deref())
        .bind("enabled")
        .execute(self.pool.as_ref())
        .await?;
        self.insert_thread_spawn_edge_from_source_if_absent(metadata.id, metadata.source.as_str())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn set_thread_memory_mode(
        &self,
        thread_id: ThreadId,
        memory_mode: &str,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query("UPDATE threads SET memory_mode = ? WHERE id = ?")
            .bind(memory_mode)
            .bind(thread_id.to_string())
            .execute(self.pool.as_ref())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_thread_title(
        &self,
        thread_id: ThreadId,
        title: &str,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query("UPDATE threads SET title = ? WHERE id = ?")
            .bind(title)
            .bind(thread_id.to_string())
            .execute(self.pool.as_ref())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn touch_thread_updated_at(
        &self,
        thread_id: ThreadId,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        let updated_at = self.allocate_thread_updated_at(updated_at)?;
        let result =
            sqlx::query("UPDATE threads SET updated_at = ?, updated_at_ms = ? WHERE id = ?")
                .bind(datetime_to_epoch_seconds(updated_at))
                .bind(datetime_to_epoch_millis(updated_at))
                .bind(thread_id.to_string())
                .execute(self.pool.as_ref())
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Allocate a persisted `updated_at` value for thread-list cursor ordering.
    ///
    /// We keep a process-local high-water mark so hot rollout writes can get unique,
    /// monotonic millisecond timestamps without querying SQLite on every update. Older
    /// backfill/repair timestamps are allowed through unchanged so historical ordering
    /// remains tied to the rollout file mtimes.
    pub(super) fn allocate_thread_updated_at(
        &self,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<DateTime<Utc>> {
        let candidate = datetime_to_epoch_millis(updated_at);
        let allocated = loop {
            let current = self.thread_updated_at_millis.load(Ordering::Relaxed);

            // New wall-clock time: advance the process-local high-water mark and use it as-is.
            if candidate > current {
                if self
                    .thread_updated_at_millis
                    .compare_exchange(current, candidate, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    break candidate;
                }
                continue;
            }

            // Older timestamps come from backfill/repair paths that preserve rollout mtimes.
            // Do not drag historical rows forward just because this process has seen newer writes.
            if candidate.saturating_add(1000) <= current {
                break candidate;
            }

            // Same hot one-second bucket as the current high-water mark. Allocate the next
            // millisecond so updated_at remains unique and cursor-orderable inside the process.
            let bumped = current.saturating_add(1);
            if self
                .thread_updated_at_millis
                .compare_exchange(current, bumped, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break bumped;
            }
        };
        epoch_millis_to_datetime(allocated)
    }
}
