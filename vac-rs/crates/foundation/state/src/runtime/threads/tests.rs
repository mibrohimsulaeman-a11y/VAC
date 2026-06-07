    use super::*;
    use crate::Anchor;
    use crate::DirectionalThreadSpawnEdgeStatus;
    use crate::runtime::test_support::test_thread_metadata;
    use crate::runtime::test_support::unique_temp_dir;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;
    use vac_protocol::protocol::EventMsg;
    use vac_protocol::protocol::GitInfo;
    use vac_protocol::protocol::SessionMeta;
    use vac_protocol::protocol::SessionMetaLine;
    use vac_protocol::protocol::SessionSource;

    #[tokio::test]
    async fn upsert_thread_keeps_creation_memory_mode_for_existing_rows() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000123").expect("valid thread id");
        let mut metadata = test_thread_metadata(&vac_home, thread_id, vac_home.clone());

        runtime
            .upsert_thread_with_creation_memory_mode(&metadata, Some("disabled"))
            .await
            .expect("initial insert should succeed");

        let memory_mode: String =
            sqlx::query_scalar("SELECT memory_mode FROM threads WHERE id = ?")
                .bind(thread_id.to_string())
                .fetch_one(runtime.pool.as_ref())
                .await
                .expect("memory mode should be readable");
        assert_eq!(memory_mode, "disabled");

        metadata.title = "updated title".to_string();
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("upsert should succeed");

        let memory_mode: String =
            sqlx::query_scalar("SELECT memory_mode FROM threads WHERE id = ?")
                .bind(thread_id.to_string())
                .fetch_one(runtime.pool.as_ref())
                .await
                .expect("memory mode should remain readable");
        assert_eq!(memory_mode, "disabled");
    }

    #[tokio::test]
    async fn list_threads_updated_after_returns_oldest_changes_first() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let older_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000001").expect("valid thread id");
        let middle_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000002").expect("valid thread id");
        let newer_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000003").expect("valid thread id");
        let older_updated_at =
            DateTime::<Utc>::from_timestamp(1_700_000_100, 0).expect("valid older timestamp");
        let newer_updated_at =
            DateTime::<Utc>::from_timestamp(1_700_000_200, 0).expect("valid newer timestamp");

        for (thread_id, updated_at) in [
            (older_id, older_updated_at),
            (newer_id, newer_updated_at),
            (middle_id, newer_updated_at),
        ] {
            let mut metadata = test_thread_metadata(&vac_home, thread_id, vac_home.clone());
            metadata.updated_at = updated_at;
            metadata.first_user_message = Some("hello".to_string());
            runtime
                .upsert_thread(&metadata)
                .await
                .expect("thread insert should succeed");
        }

        let anchor = Anchor {
            ts: older_updated_at,
        };
        let model_providers = ["test-provider".to_string()];
        let page = runtime
            .list_threads(
                /*page_size*/ 1,
                ThreadFilterOptions {
                    archived_only: false,
                    allowed_sources: &[],
                    model_providers: Some(&model_providers),
                    cwd_filters: None,
                    anchor: Some(&anchor),
                    sort_key: SortKey::UpdatedAt,
                    sort_direction: SortDirection::Asc,
                    search_term: None,
                },
            )
            .await
            .expect("list should succeed");

        let ids = page.items.iter().map(|item| item.id).collect::<Vec<_>>();
        assert_eq!(ids, vec![newer_id]);
        assert_eq!(
            page.next_anchor,
            Some(Anchor {
                ts: DateTime::<Utc>::from_timestamp_millis(1_700_000_200_000)
                    .expect("valid timestamp"),
            })
        );

        let page = runtime
            .list_threads(
                /*page_size*/ 1,
                ThreadFilterOptions {
                    archived_only: false,
                    allowed_sources: &[],
                    model_providers: Some(&model_providers),
                    cwd_filters: None,
                    anchor: page.next_anchor.as_ref(),
                    sort_key: SortKey::UpdatedAt,
                    sort_direction: SortDirection::Asc,
                    search_term: None,
                },
            )
            .await
            .expect("second page should succeed");

        let ids = page.items.iter().map(|item| item.id).collect::<Vec<_>>();
        assert_eq!(ids, vec![middle_id]);
        assert_eq!(page.next_anchor, None);
    }

    #[tokio::test]
    async fn list_threads_filters_by_cwd() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let first_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000101").expect("valid thread id");
        let second_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000102").expect("valid thread id");
        let other_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000103").expect("valid thread id");
        let first_cwd = vac_home.join("first");
        let second_cwd = vac_home.join("second");
        let other_cwd = vac_home.join("other");

        for (thread_id, cwd, updated_at) in [
            (first_id, first_cwd.clone(), 1_700_000_100),
            (second_id, second_cwd.clone(), 1_700_000_300),
            (other_id, other_cwd, 1_700_000_500),
        ] {
            let mut metadata = test_thread_metadata(&vac_home, thread_id, cwd);
            metadata.updated_at =
                DateTime::<Utc>::from_timestamp(updated_at, 0).expect("valid timestamp");
            runtime
                .upsert_thread(&metadata)
                .await
                .expect("thread insert should succeed");
        }

        let cwd_filters = vec![first_cwd, second_cwd];
        let page = runtime
            .list_threads(
                /*page_size*/ 10,
                ThreadFilterOptions {
                    archived_only: false,
                    allowed_sources: &[],
                    model_providers: None,
                    cwd_filters: Some(cwd_filters.as_slice()),
                    anchor: None,
                    sort_key: SortKey::UpdatedAt,
                    sort_direction: SortDirection::Desc,
                    search_term: None,
                },
            )
            .await
            .expect("list should succeed");

        let ids = page.items.iter().map(|item| item.id).collect::<Vec<_>>();
        assert_eq!(ids, vec![second_id, first_id]);

        let page = runtime
            .list_threads(
                /*page_size*/ 10,
                ThreadFilterOptions {
                    archived_only: false,
                    allowed_sources: &[],
                    model_providers: None,
                    cwd_filters: Some(&[]),
                    anchor: None,
                    sort_key: SortKey::UpdatedAt,
                    sort_direction: SortDirection::Desc,
                    search_term: None,
                },
            )
            .await
            .expect("list with empty cwd filters should succeed");

        assert_eq!(page.items, Vec::new());
    }

    #[tokio::test]
    async fn apply_rollout_items_restores_memory_mode_from_session_meta() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000456").expect("valid thread id");
        let metadata = test_thread_metadata(&vac_home, thread_id, vac_home.clone());

        runtime
            .upsert_thread(&metadata)
            .await
            .expect("initial upsert should succeed");

        let builder = ThreadMetadataBuilder::new(
            thread_id,
            metadata.rollout_path.clone(),
            metadata.created_at,
            SessionSource::Cli,
        );
        let items = vec![RolloutItem::SessionMeta(SessionMetaLine {
            meta: SessionMeta {
                id: thread_id,
                branched_from_id: None,
                timestamp: metadata.created_at.to_rfc3339(),
                cwd: PathBuf::new(),
                originator: String::new(),
                cli_version: String::new(),
                source: SessionSource::Cli,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
                model_provider: None,
                base_instructions: None,
                dynamic_tools: None,
                memory_mode: Some("polluted".to_string()),
            },
            git: None,
        })];

        runtime
            .apply_rollout_items(
                &builder, &items, /*new_thread_memory_mode*/ None,
                /*updated_at_override*/ None,
            )
            .await
            .expect("apply_rollout_items should succeed");

        let memory_mode = runtime
            .get_thread_memory_mode(thread_id)
            .await
            .expect("memory mode should load");
        assert_eq!(memory_mode.as_deref(), Some("polluted"));
    }

    #[tokio::test]
    async fn apply_rollout_items_preserves_existing_git_branch_and_fills_missing_git_fields() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000457").expect("valid thread id");
        let mut metadata = test_thread_metadata(&vac_home, thread_id, vac_home.clone());
        metadata.git_branch = Some("sqlite-branch".to_string());

        runtime
            .upsert_thread(&metadata)
            .await
            .expect("initial upsert should succeed");

        let created_at = metadata.created_at.to_rfc3339();
        let builder = ThreadMetadataBuilder::new(
            thread_id,
            metadata.rollout_path.clone(),
            metadata.created_at,
            SessionSource::Cli,
        );
        let items = vec![RolloutItem::SessionMeta(SessionMetaLine {
            meta: SessionMeta {
                id: thread_id,
                branched_from_id: None,
                timestamp: created_at,
                cwd: PathBuf::new(),
                originator: String::new(),
                cli_version: String::new(),
                source: SessionSource::Cli,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
                model_provider: None,
                base_instructions: None,
                dynamic_tools: None,
                memory_mode: None,
            },
            git: Some(GitInfo {
                commit_hash: Some(vac_git_utils::GitSha::new("rollout-sha")),
                branch: Some("rollout-branch".to_string()),
                repository_url: Some("git@example.com:vastar/vac.git".to_string()),
            }),
        })];

        runtime
            .apply_rollout_items(
                &builder, &items, /*new_thread_memory_mode*/ None,
                /*updated_at_override*/ None,
            )
            .await
            .expect("apply_rollout_items should succeed");

        let persisted = runtime
            .get_thread(thread_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(persisted.git_sha.as_deref(), Some("rollout-sha"));
        assert_eq!(persisted.git_branch.as_deref(), Some("sqlite-branch"));
        assert_eq!(
            persisted.git_origin_url.as_deref(),
            Some("git@example.com:vastar/vac.git")
        );
    }

    #[tokio::test]
    async fn update_thread_git_info_preserves_newer_non_git_metadata() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000789").expect("valid thread id");
        let metadata = test_thread_metadata(&vac_home, thread_id, vac_home.clone());

        runtime
            .upsert_thread(&metadata)
            .await
            .expect("initial upsert should succeed");

        let updated_at = datetime_to_epoch_millis(
            DateTime::<Utc>::from_timestamp(1_700_000_100, 0).expect("timestamp"),
        );
        sqlx::query(
            "UPDATE threads SET updated_at = ?, updated_at_ms = ?, tokens_used = ?, first_user_message = ? WHERE id = ?",
        )
        .bind(updated_at / 1000)
        .bind(updated_at)
        .bind(123_i64)
        .bind("newer preview")
        .bind(thread_id.to_string())
        .execute(runtime.pool.as_ref())
        .await
        .expect("concurrent metadata write should succeed");

        let updated = runtime
            .update_thread_git_info(
                thread_id,
                Some(Some("abc123")),
                Some(Some("feature/branch")),
                Some(Some("git@example.com:vastar/vac.git")),
            )
            .await
            .expect("git info update should succeed");
        assert!(updated, "git info update should touch the thread row");

        let persisted = runtime
            .get_thread(thread_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(persisted.tokens_used, 123);
        assert_eq!(
            persisted.first_user_message.as_deref(),
            Some("newer preview")
        );
        assert_eq!(datetime_to_epoch_millis(persisted.updated_at), updated_at);
        assert_eq!(persisted.git_sha.as_deref(), Some("abc123"));
        assert_eq!(persisted.git_branch.as_deref(), Some("feature/branch"));
        assert_eq!(
            persisted.git_origin_url.as_deref(),
            Some("git@example.com:vastar/vac.git")
        );
    }

    #[tokio::test]
    async fn insert_thread_if_absent_preserves_existing_metadata() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000791").expect("valid thread id");

        let mut existing = test_thread_metadata(&vac_home, thread_id, vac_home.clone());
        existing.tokens_used = 123;
        existing.first_user_message = Some("newer preview".to_string());
        existing.updated_at = DateTime::<Utc>::from_timestamp(1_700_000_100, 0).expect("timestamp");
        runtime
            .upsert_thread(&existing)
            .await
            .expect("initial upsert should succeed");

        let mut fallback = test_thread_metadata(&vac_home, thread_id, vac_home.clone());
        fallback.tokens_used = 0;
        fallback.first_user_message = None;
        fallback.updated_at = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("timestamp");

        let inserted = runtime
            .insert_thread_if_absent(&fallback)
            .await
            .expect("insert should succeed");
        assert!(!inserted, "existing rows should not be overwritten");

        let persisted = runtime
            .get_thread(thread_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(persisted.tokens_used, 123);
        assert_eq!(
            persisted.first_user_message.as_deref(),
            Some("newer preview")
        );
        assert_eq!(
            datetime_to_epoch_millis(persisted.updated_at),
            datetime_to_epoch_millis(existing.updated_at)
        );
    }

    #[tokio::test]
    async fn update_thread_git_info_can_clear_fields() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000790").expect("valid thread id");
        let mut metadata = test_thread_metadata(&vac_home, thread_id, vac_home.clone());
        metadata.git_sha = Some("abc123".to_string());
        metadata.git_branch = Some("feature/branch".to_string());
        metadata.git_origin_url = Some("git@example.com:vastar/vac.git".to_string());

        runtime
            .upsert_thread(&metadata)
            .await
            .expect("initial upsert should succeed");

        let updated = runtime
            .update_thread_git_info(thread_id, Some(None), Some(None), Some(None))
            .await
            .expect("git info clear should succeed");
        assert!(updated, "git info clear should touch the thread row");

        let persisted = runtime
            .get_thread(thread_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(persisted.git_sha, None);
        assert_eq!(persisted.git_branch, None);
        assert_eq!(persisted.git_origin_url, None);
    }

    #[tokio::test]
    async fn touch_thread_updated_at_updates_only_updated_at() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000791").expect("valid thread id");
        let mut metadata = test_thread_metadata(&vac_home, thread_id, vac_home.clone());
        metadata.title = "original title".to_string();
        metadata.first_user_message = Some("first-user-message".to_string());

        runtime
            .upsert_thread(&metadata)
            .await
            .expect("initial upsert should succeed");

        let touched_at = DateTime::<Utc>::from_timestamp(1_700_001_111, 0).expect("timestamp");
        let touched = runtime
            .touch_thread_updated_at(thread_id, touched_at)
            .await
            .expect("touch should succeed");
        assert!(touched);

        let persisted = runtime
            .get_thread(thread_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(persisted.updated_at, touched_at);
        assert_eq!(persisted.title, "original title");
        assert_eq!(
            persisted.first_user_message.as_deref(),
            Some("first-user-message")
        );
    }

    #[tokio::test]
    async fn thread_updated_at_uses_unique_epoch_millis_and_reads_legacy_seconds() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let first_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000901").expect("valid thread id");
        let second_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000902").expect("valid thread id");
        let older_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000903").expect("valid thread id");
        let updated_at =
            DateTime::<Utc>::from_timestamp_millis(1_700_001_111_123).expect("timestamp millis");
        let mut first = test_thread_metadata(&vac_home, first_id, vac_home.clone());
        first.updated_at = updated_at;
        let mut second = test_thread_metadata(&vac_home, second_id, vac_home.clone());
        second.updated_at = updated_at;

        runtime
            .upsert_thread(&first)
            .await
            .expect("first upsert should succeed");
        runtime
            .upsert_thread(&second)
            .await
            .expect("second upsert should succeed");

        let first = runtime
            .get_thread(first_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        let second = runtime
            .get_thread(second_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(
            datetime_to_epoch_millis(first.updated_at),
            1_700_001_111_123
        );
        assert_eq!(
            datetime_to_epoch_millis(second.updated_at),
            1_700_001_111_124
        );
        let second_row: (i64, i64, Option<i64>, Option<i64>) = sqlx::query_as(
            "SELECT created_at, updated_at, created_at_ms, updated_at_ms FROM threads WHERE id = ?",
        )
        .bind(second_id.to_string())
        .fetch_one(runtime.pool.as_ref())
        .await
        .expect("thread timestamp row should load");
        assert_eq!(
            second_row,
            (
                datetime_to_epoch_seconds(second.created_at),
                1_700_001_111,
                Some(datetime_to_epoch_millis(second.created_at)),
                Some(1_700_001_111_124)
            )
        );

        let older_updated_at =
            DateTime::<Utc>::from_timestamp_millis(1_700_001_100_123).expect("timestamp millis");
        let mut older = test_thread_metadata(&vac_home, older_id, vac_home.clone());
        older.updated_at = older_updated_at;
        runtime
            .upsert_thread(&older)
            .await
            .expect("older upsert should succeed");
        let older = runtime
            .get_thread(older_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(
            datetime_to_epoch_millis(older.updated_at),
            1_700_001_100_123
        );

        sqlx::query("UPDATE threads SET updated_at = ? WHERE id = ?")
            .bind(1_700_001_112_i64)
            .bind(first_id.to_string())
            .execute(runtime.pool.as_ref())
            .await
            .expect("legacy timestamp write should succeed");
        let legacy = runtime
            .get_thread(first_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(
            datetime_to_epoch_millis(legacy.updated_at),
            1_700_001_112_000
        );
    }

    #[tokio::test]
    async fn apply_rollout_items_uses_override_updated_at_when_provided() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home.clone(), "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000792").expect("valid thread id");
        let metadata = test_thread_metadata(&vac_home, thread_id, vac_home.clone());

        runtime
            .upsert_thread(&metadata)
            .await
            .expect("initial upsert should succeed");

        let builder = ThreadMetadataBuilder::new(
            thread_id,
            metadata.rollout_path.clone(),
            metadata.created_at,
            SessionSource::Cli,
        );
        let items = vec![RolloutItem::EventMsg(EventMsg::TokenCount(
            vac_protocol::protocol::TokenCountEvent {
                info: Some(vac_protocol::protocol::TokenUsageInfo {
                    total_token_usage: vac_protocol::protocol::TokenUsage {
                        input_tokens: 0,
                        cached_input_tokens: 0,
                        output_tokens: 0,
                        reasoning_output_tokens: 0,
                        total_tokens: 321,
                    },
                    last_token_usage: vac_protocol::protocol::TokenUsage::default(),
                    model_context_window: None,
                }),
                rate_limits: None,
            },
        ))];
        let override_updated_at =
            DateTime::<Utc>::from_timestamp(1_700_001_234, 0).expect("timestamp");

        runtime
            .apply_rollout_items(
                &builder,
                &items,
                /*new_thread_memory_mode*/ None,
                Some(override_updated_at),
            )
            .await
            .expect("apply_rollout_items should succeed");

        let persisted = runtime
            .get_thread(thread_id)
            .await
            .expect("thread should load")
            .expect("thread should exist");
        assert_eq!(persisted.tokens_used, 321);
        assert_eq!(persisted.updated_at, override_updated_at);
    }

    #[tokio::test]
    async fn thread_spawn_edges_track_directional_status() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home, "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let parent_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000900").expect("valid thread id");
        let child_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000901").expect("valid thread id");
        let grandchild_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000902").expect("valid thread id");

        runtime
            .upsert_thread_spawn_edge(
                parent_thread_id,
                child_thread_id,
                DirectionalThreadSpawnEdgeStatus::Open,
            )
            .await
            .expect("child edge insert should succeed");
        runtime
            .upsert_thread_spawn_edge(
                child_thread_id,
                grandchild_thread_id,
                DirectionalThreadSpawnEdgeStatus::Open,
            )
            .await
            .expect("grandchild edge insert should succeed");

        let children = runtime
            .list_thread_spawn_children_with_status(
                parent_thread_id,
                DirectionalThreadSpawnEdgeStatus::Open,
            )
            .await
            .expect("open child list should load");
        assert_eq!(children, vec![child_thread_id]);

        let descendants = runtime
            .list_thread_spawn_descendants_with_status(
                parent_thread_id,
                DirectionalThreadSpawnEdgeStatus::Open,
            )
            .await
            .expect("open descendants should load");
        assert_eq!(descendants, vec![child_thread_id, grandchild_thread_id]);

        runtime
            .set_thread_spawn_edge_status(child_thread_id, DirectionalThreadSpawnEdgeStatus::Closed)
            .await
            .expect("edge close should succeed");

        let open_children = runtime
            .list_thread_spawn_children_with_status(
                parent_thread_id,
                DirectionalThreadSpawnEdgeStatus::Open,
            )
            .await
            .expect("open child list should load");
        assert_eq!(open_children, Vec::<ThreadId>::new());

        let closed_children = runtime
            .list_thread_spawn_children_with_status(
                parent_thread_id,
                DirectionalThreadSpawnEdgeStatus::Closed,
            )
            .await
            .expect("closed child list should load");
        assert_eq!(closed_children, vec![child_thread_id]);

        let closed_descendants = runtime
            .list_thread_spawn_descendants_with_status(
                parent_thread_id,
                DirectionalThreadSpawnEdgeStatus::Closed,
            )
            .await
            .expect("closed descendants should load");
        assert_eq!(closed_descendants, vec![child_thread_id]);

        let open_descendants_from_child = runtime
            .list_thread_spawn_descendants_with_status(
                child_thread_id,
                DirectionalThreadSpawnEdgeStatus::Open,
            )
            .await
            .expect("open descendants from child should load");
        assert_eq!(open_descendants_from_child, vec![grandchild_thread_id]);

        let all_descendants = runtime
            .list_thread_spawn_descendants(parent_thread_id)
            .await
            .expect("all descendants should load");
        assert_eq!(all_descendants, vec![child_thread_id, grandchild_thread_id]);
    }

    #[tokio::test]
    async fn thread_spawn_children_without_status_filter_lists_all_statuses() {
        let vac_home = unique_temp_dir();
        let runtime = StateRuntime::init(vac_home, "test-provider".to_string())
            .await
            .expect("state db should initialize");
        let parent_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000910").expect("valid thread id");
        let open_child_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000911").expect("valid thread id");
        let closed_child_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000912").expect("valid thread id");
        let future_child_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000913").expect("valid thread id");

        runtime
            .upsert_thread_spawn_edge(
                parent_thread_id,
                open_child_thread_id,
                DirectionalThreadSpawnEdgeStatus::Open,
            )
            .await
            .expect("open child edge insert should succeed");
        runtime
            .upsert_thread_spawn_edge(
                parent_thread_id,
                closed_child_thread_id,
                DirectionalThreadSpawnEdgeStatus::Closed,
            )
            .await
            .expect("closed child edge insert should succeed");
        sqlx::query(
            r#"
INSERT INTO thread_spawn_edges (
    parent_thread_id,
    child_thread_id,
    status
) VALUES (?, ?, ?)
            "#,
        )
        .bind(parent_thread_id.to_string())
        .bind(future_child_thread_id.to_string())
        .bind("future")
        .execute(runtime.pool.as_ref())
        .await
        .expect("future-status child edge insert should succeed");

        let children = runtime
            .list_thread_spawn_children(parent_thread_id)
            .await
            .expect("all children should load");
        assert_eq!(
            children,
            vec![
                open_child_thread_id,
                closed_child_thread_id,
                future_child_thread_id,
            ]
        );
    }
