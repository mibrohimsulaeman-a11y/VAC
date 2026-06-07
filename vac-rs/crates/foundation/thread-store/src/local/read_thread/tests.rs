    use std::io::Write;
    use std::path::PathBuf;

    use chrono::Utc;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;
    use uuid::Uuid;
    use vac_protocol::ThreadId;
    use vac_protocol::protocol::SessionSource;
    use vac_state::ThreadMetadataBuilder;

    use super::*;
    use crate::ThreadStore;
    use crate::local::LocalThreadStore;
    use crate::local::test_support::test_config;
    use crate::local::test_support::write_archived_session_file;
    use crate::local::test_support::write_session_file;
    use crate::local::test_support::write_session_file_with_branch;

    #[tokio::test]
    async fn read_thread_returns_active_rollout_summary() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalThreadStore::new(test_config(home.path()));
        let uuid = Uuid::from_u128(205);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let active_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(active_path));
        assert_eq!(thread.archived_at, None);
        assert_eq!(thread.preview, "Hello from user");
        assert_eq!(
            thread.history.expect("history should load").thread_id,
            thread_id
        );
    }

    #[tokio::test]
    async fn read_thread_returns_rollout_path_summary() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalThreadStore::new(test_config(home.path()));
        let uuid = Uuid::from_u128(211);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let active_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let relative_path = active_path
            .strip_prefix(home.path())
            .expect("path should be under vac home")
            .to_path_buf();

        let thread = store
            .read_thread_by_rollout_path(
                relative_path,
                /*include_archived*/ false,
                /*include_history*/ false,
            )
            .await
            .expect("read thread by rollout path");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(
            thread.rollout_path,
            Some(std::fs::canonicalize(active_path).expect("canonical path"))
        );
        assert_eq!(thread.preview, "Hello from user");
    }

    #[tokio::test]
    async fn read_thread_by_rollout_path_prefers_sqlite_git_info() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(223);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let active_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder = ThreadMetadataBuilder::new(
            thread_id,
            active_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.model_provider = Some(config.default_model_provider_id.clone());
        builder.git_branch = Some("sqlite-branch".to_string());
        runtime
            .upsert_thread(&builder.build(config.default_model_provider_id.as_str()))
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_thread_by_rollout_path(
                active_path,
                /*include_archived*/ false,
                /*include_history*/ false,
            )
            .await
            .expect("read thread by rollout path");

        let git_info = thread.git_info.expect("git info should be present");
        assert_eq!(git_info.branch.as_deref(), Some("sqlite-branch"));
        assert_eq!(
            git_info.commit_hash.as_ref().map(|sha| sha.0.as_str()),
            Some("abcdef")
        );
        assert_eq!(
            git_info.repository_url.as_deref(),
            Some("https://example.com/repo.git")
        );
    }

    #[tokio::test]
    async fn read_thread_returns_archived_rollout_when_requested() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalThreadStore::new(test_config(home.path()));
        let uuid = Uuid::from_u128(207);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let archived_path = write_archived_session_file(home.path(), "2025-01-03T12-00-00", uuid)
            .expect("archived session file");

        let active_only_err = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect_err("active-only read should fail for archived rollout");
        let ThreadStoreError::InvalidRequest { message } = active_only_err else {
            panic!("expected invalid request error");
        };
        assert_eq!(
            message,
            format!("no rollout found for thread id {thread_id}")
        );

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: false,
            })
            .await
            .expect("read archived thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(archived_path));
        assert!(thread.archived_at.is_some());
        assert_eq!(thread.preview, "Archived user message");
        assert!(thread.history.is_none());
    }

    #[tokio::test]
    async fn read_thread_prefers_active_rollout_over_archived() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalThreadStore::new(test_config(home.path()));
        let uuid = Uuid::from_u128(208);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let active_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        write_archived_session_file(home.path(), "2025-01-03T12-00-00", uuid)
            .expect("archived session file");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.rollout_path, Some(active_path));
        assert_eq!(thread.archived_at, None);
        assert_eq!(thread.preview, "Hello from user");
    }

    #[tokio::test]
    async fn read_thread_returns_branched_from_id() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalThreadStore::new(test_config(home.path()));
        let uuid = Uuid::from_u128(209);
        let parent_uuid = Uuid::from_u128(210);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let parent_thread_id =
            ThreadId::from_string(&parent_uuid.to_string()).expect("valid parent thread id");
        write_session_file_with_branch(
            home.path(),
            home.path().join("sessions/2025/01/03"),
            "2025-01-03T12-00-00",
            uuid,
            "Branched user message",
            Some("test-provider"),
            Some(parent_uuid),
        )
        .expect("branched session file");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.branched_from_id, Some(parent_thread_id));
    }

    #[tokio::test]
    async fn read_thread_applies_sqlite_thread_name() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(212);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder =
            ThreadMetadataBuilder::new(thread_id, rollout_path, Utc::now(), SessionSource::Cli);
        builder.model_provider = Some(config.default_model_provider_id.clone());
        builder.cwd = home.path().to_path_buf();
        builder.cli_version = Some("test_version".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.title = "Saved title".to_string();
        metadata.first_user_message = Some("Hello from user".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.name, Some("Saved title".to_string()));
    }

    #[tokio::test]
    async fn read_thread_preserves_rollout_cwd_when_sqlite_metadata_exists() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(224);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let day_dir = home.path().join("sessions/2025/01/03");
        std::fs::create_dir_all(&day_dir).expect("sessions dir");
        let rollout_path = day_dir.join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let mut file = std::fs::File::create(&rollout_path).expect("session file");
        let rollout_cwd = PathBuf::from("/");
        let meta = serde_json::json!({
            "timestamp": "2025-01-03T12:00:00Z",
            "type": "session_meta",
            "payload": {
                "id": uuid,
                "timestamp": "2025-01-03T12:00:00Z",
                "cwd": rollout_cwd,
                "originator": "test_originator",
                "cli_version": "test_version",
                "source": "cli",
                "model_provider": "rollout-provider"
            },
        });
        writeln!(file, "{meta}").expect("write session meta");
        let user_event = serde_json::json!({
            "timestamp": "2025-01-03T12:00:00Z",
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": "Hello from rollout",
                "kind": "plain",
            },
        });
        writeln!(file, "{user_event}").expect("write user event");

        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder = ThreadMetadataBuilder::new(
            thread_id,
            rollout_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.model_provider = Some(config.default_model_provider_id.clone());
        builder.cwd = home.path().join("sqlite-workspace");
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.title = "Saved title".to_string();
        metadata.first_user_message = Some("Hello from sqlite".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "Hello from rollout");
        assert_eq!(thread.name, Some("Saved title".to_string()));
        assert_eq!(thread.model_provider, "rollout-provider");
        assert_eq!(thread.cwd, rollout_cwd);
    }

    #[tokio::test]
    async fn read_thread_uses_legacy_thread_name_when_sqlite_title_is_missing() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalThreadStore::new(test_config(home.path()));
        let uuid = Uuid::from_u128(213);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        vac_rollout::append_thread_name(home.path(), thread_id, "Legacy title")
            .await
            .expect("append legacy thread name");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.name, Some("Legacy title".to_string()));
    }

    #[tokio::test]
    async fn read_thread_uses_sqlite_metadata_for_rollout_without_user_preview() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(217);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let day_dir = home.path().join("sessions/2025/01/03");
        std::fs::create_dir_all(&day_dir).expect("sessions dir");
        let rollout_path = day_dir.join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let mut file = std::fs::File::create(&rollout_path).expect("session file");
        let meta = serde_json::json!({
            "timestamp": "2025-01-03T12-00-00",
            "type": "session_meta",
            "payload": {
                "id": uuid,
                "timestamp": "2025-01-03T12-00-00",
                "cwd": home.path(),
                "originator": "test_originator",
                "cli_version": "test_version",
                "source": "cli",
                "model_provider": "rollout-provider"
            },
        });
        writeln!(file, "{meta}").expect("write session meta");

        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder = ThreadMetadataBuilder::new(
            thread_id,
            rollout_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.model_provider = Some("sqlite-provider".to_string());
        builder.cwd = home.path().join("workspace");
        builder.cli_version = Some("sqlite-cli".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.title = "Command-only thread".to_string();
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "");
        assert_eq!(thread.name.as_deref(), Some("Command-only thread"));
        assert_eq!(thread.model_provider, "sqlite-provider");
        assert_eq!(thread.cwd, home.path().join("workspace"));
        assert_eq!(thread.cli_version, "sqlite-cli");
        let history = thread.history.expect("history should load");
        assert_eq!(history.thread_id, thread_id);
        assert_eq!(history.items.len(), 1);
    }

    #[tokio::test]
    async fn read_thread_falls_back_to_rollout_search_when_sqlite_path_is_stale() {
        let home = TempDir::new().expect("temp dir");
        let external = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(220);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let stale_path = external.path().join("missing-rollout.jsonl");
        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder = ThreadMetadataBuilder::new(
            thread_id,
            stale_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.model_provider = Some("stale-sqlite-provider".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.first_user_message = Some("stale sqlite preview".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "Hello from user");
        assert_eq!(thread.model_provider, config.default_model_provider_id);
        let history = thread.history.expect("history should load");
        assert_eq!(history.thread_id, thread_id);
        assert_eq!(history.items.len(), 2);
    }

    #[tokio::test]
    async fn read_thread_falls_back_when_sqlite_path_points_to_another_thread() {
        let home = TempDir::new().expect("temp dir");
        let external = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(221);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let other_uuid = Uuid::from_u128(222);
        let stale_path = write_session_file(external.path(), "2025-01-04T12-00-00", other_uuid)
            .expect("other session file");
        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder =
            ThreadMetadataBuilder::new(thread_id, stale_path, Utc::now(), SessionSource::Cli);
        builder.model_provider = Some("wrong-sqlite-provider".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.first_user_message = Some("wrong sqlite preview".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "Hello from user");
        assert_eq!(thread.model_provider, config.default_model_provider_id);
        let history = thread.history.expect("history should load");
        assert_eq!(history.thread_id, thread_id);
        assert_eq!(history.items.len(), 2);
    }

    #[tokio::test]
    async fn read_thread_uses_session_meta_for_rollout_without_user_preview_or_sqlite_metadata() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalThreadStore::new(test_config(home.path()));
        let uuid = Uuid::from_u128(218);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let day_dir = home.path().join("sessions/2025/01/03");
        std::fs::create_dir_all(&day_dir).expect("sessions dir");
        let rollout_path = day_dir.join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let mut file = std::fs::File::create(&rollout_path).expect("session file");
        let meta = serde_json::json!({
            "timestamp": "2025-01-03T12:00:00Z",
            "type": "session_meta",
            "payload": {
                "id": uuid,
                "timestamp": "2025-01-03T12:00:00Z",
                "cwd": home.path(),
                "originator": "test_originator",
                "cli_version": "test_version",
                "source": "cli",
                "model_provider": "rollout-provider"
            },
        });
        writeln!(file, "{meta}").expect("write session meta");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "");
        assert_eq!(thread.name, None);
        assert_eq!(thread.model_provider, "rollout-provider");
        assert_eq!(
            thread.created_at,
            parse_rfc3339_non_optional("2025-01-03T12:00:00Z").unwrap()
        );
        assert!(thread.updated_at >= thread.created_at);
        assert_eq!(thread.archived_at, None);
        assert_eq!(thread.cwd, home.path());
        assert_eq!(thread.cli_version, "test_version");
        assert_eq!(thread.source, SessionSource::Cli);
        let history = thread.history.expect("history should load");
        assert_eq!(history.thread_id, thread_id);
        assert_eq!(history.items.len(), 1);
    }

    #[tokio::test]
    async fn read_thread_falls_back_to_sqlite_summary() {
        let home = TempDir::new().expect("temp dir");
        let external = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(214);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path = external
            .path()
            .join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder = ThreadMetadataBuilder::new(
            thread_id,
            rollout_path.clone(),
            Utc::now(),
            SessionSource::Exec,
        );
        builder.model_provider = Some("sqlite-provider".to_string());
        builder.cwd = external.path().join("workspace");
        builder.cli_version = Some("sqlite-cli".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.title = "SQLite title".to_string();
        metadata.first_user_message = Some("SQLite preview".to_string());
        metadata.model = Some("sqlite-model".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "SQLite preview");
        assert_eq!(thread.first_user_message.as_deref(), Some("SQLite preview"));
        assert_eq!(thread.name.as_deref(), Some("SQLite title"));
        assert_eq!(thread.model_provider, "sqlite-provider");
        assert_eq!(thread.model.as_deref(), Some("sqlite-model"));
        assert_eq!(thread.cwd, external.path().join("workspace"));
        assert_eq!(thread.cli_version, "sqlite-cli");
        assert_eq!(thread.source, SessionSource::Exec);
        assert_eq!(thread.archived_at, None);
        assert!(thread.history.is_none());
    }

    #[tokio::test]
    async fn read_thread_sqlite_fallback_respects_include_archived() {
        let home = TempDir::new().expect("temp dir");
        let external = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(216);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path = external
            .path()
            .join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder =
            ThreadMetadataBuilder::new(thread_id, rollout_path, Utc::now(), SessionSource::Cli);
        builder.archived_at = Some(Utc::now());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.first_user_message = Some("Archived SQLite preview".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let active_only_err = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect_err("active-only read should fail for archived metadata");
        let ThreadStoreError::InvalidRequest { message } = active_only_err else {
            panic!("expected invalid request error");
        };
        assert_eq!(
            message,
            format!("no rollout found for thread id {thread_id}")
        );

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: false,
            })
            .await
            .expect("read archived thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.preview, "Archived SQLite preview");
        assert!(thread.archived_at.is_some());
    }

    #[tokio::test]
    async fn read_thread_sqlite_fallback_loads_archived_history() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let store = LocalThreadStore::new(config.clone());
        let uuid = Uuid::from_u128(219);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");
        let archived_path = write_archived_session_file(home.path(), "2025-01-03T12-00-00", uuid)
            .expect("archived session file");
        let runtime = vac_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder = ThreadMetadataBuilder::new(
            thread_id,
            archived_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.archived_at = Some(Utc::now());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.first_user_message = Some("Archived SQLite preview".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await
            .expect("read archived thread");

        assert_eq!(thread.thread_id, thread_id);
        assert_eq!(thread.rollout_path, Some(archived_path));
        assert_eq!(thread.preview, "Archived SQLite preview");
        assert!(thread.archived_at.is_some());
        let history = thread.history.expect("history should load");
        assert_eq!(history.thread_id, thread_id);
        assert_eq!(history.items.len(), 2);
    }

    #[tokio::test]
    async fn read_thread_fails_without_rollout() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalThreadStore::new(test_config(home.path()));
        let uuid = Uuid::from_u128(206);
        let thread_id = ThreadId::from_string(&uuid.to_string()).expect("valid thread id");

        let err = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect_err("read should fail without rollout");

        let ThreadStoreError::InvalidRequest { message } = err else {
            panic!("expected invalid request error");
        };
        assert_eq!(
            message,
            format!("no rollout found for thread id {thread_id}")
        );
    }
