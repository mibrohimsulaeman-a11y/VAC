mod owner_native {
    use std::fmt;
    use std::io;
    use std::io::ErrorKind;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use async_trait::async_trait;
    use color_eyre::eyre::Result;
    use serde::de::DeserializeOwned;
    use tokio::sync::mpsc;
    use toml::Value as TomlValue;

    use crate::bottom_pane::FeedbackAudience;
    use crate::legacy_core::config::Config;
    use crate::local_runtime_session::LocalRuntimeBootstrap;
    use crate::local_runtime_session::LocalRuntimeRequestHandle;
    use crate::local_runtime_session::LocalRuntimeSession;
    use crate::local_runtime_session::LocalRuntimeStartedThread;
    use crate::owner_native_runtime_support::OwnerRuntimeMethodStatus;
    use crate::owner_native_runtime_support::release_blocking_owner_runtime_methods;
    use crate::session_protocol::AskForApproval;
    use crate::session_protocol::AuthMode;
    use crate::session_protocol::ClientRequest;
    use crate::session_protocol::ConfigWarningNotification;
    use crate::session_protocol::ErrorNotification;
    use crate::session_protocol::ExternalAgentConfigDetectParams;
    use crate::session_protocol::ExternalAgentConfigDetectResponse;
    use crate::session_protocol::ExternalAgentConfigImportResponse;
    use crate::session_protocol::ExternalAgentConfigMigrationItem;
    use crate::session_protocol::GetAccountRateLimitsResponse;
    use crate::session_protocol::GetAccountResponse;
    use crate::session_protocol::JSONRPCErrorError;
    use crate::session_protocol::RateLimitSnapshot;
    use crate::session_protocol::RequestId;
    use crate::session_protocol::ReviewStartResponse;
    use crate::session_protocol::SkillErrorInfo;
    use crate::session_protocol::SkillMetadata;
    use crate::session_protocol::SkillsListEntry;
    use crate::session_protocol::SkillsListParams;
    use crate::session_protocol::SkillsListResponse;
    use crate::session_protocol::Thread;
    use crate::session_protocol::ThreadGoal;
    use crate::session_protocol::ThreadGoalClearResponse;
    use crate::session_protocol::ThreadGoalGetResponse;
    use crate::session_protocol::ThreadGoalSetResponse;
    use crate::session_protocol::ThreadGoalStatus;
    use crate::session_protocol::ThreadInjectItemsResponse;
    use crate::session_protocol::ThreadListCwdFilter;
    use crate::session_protocol::ThreadListParams;
    use crate::session_protocol::ThreadListResponse;
    use crate::session_protocol::ThreadLoadedListParams;
    use crate::session_protocol::ThreadLoadedListResponse;
    use crate::session_protocol::ThreadMemoryMode;
    use crate::session_protocol::ThreadRealtimeAudioChunk;
    use crate::session_protocol::ThreadRealtimeStartTransport;
    use crate::session_protocol::ThreadRollbackResponse;
    use crate::session_protocol::ThreadStartSource;
    use crate::session_protocol::ThreadStatus;
    use crate::session_protocol::Turn;
    use crate::session_protocol::TurnCompletedNotification;
    use crate::session_protocol::TurnError;
    use crate::session_protocol::TurnStartResponse;
    use crate::session_protocol::TurnStartedNotification;
    use crate::session_protocol::TurnStatus;
    use crate::session_protocol::TurnSteerResponse;
    use crate::session_protocol::UserInput;
    use crate::session_protocol::WarningNotification;
    use crate::session_protocol::build_turns_from_rollout_items;
    use crate::session_protocol::thread_source_kinds_to_session_sources;
    use crate::session_state::SessionNetworkProxyRuntime;
    use crate::session_state::ThreadSessionState;
    use crate::status::StatusAccountDisplay;
    use crate::status::plan_type_display_name;
    use serde::Serialize;
    use tracing::warn;
    use vac_arg0::Arg0DispatchPaths;
    use vac_config::LoaderOverrides;
    use vac_core::ThreadManager;
    use vac_core::thread_store_from_config;
    pub(crate) use vac_exec_server::EnvironmentManager;
    pub(crate) use vac_exec_server::EnvironmentManagerArgs;
    pub(crate) use vac_exec_server::ExecServerRuntimePaths;
    pub(crate) use vac_exec_server::LOCAL_FS;
    use vac_feedback::VACFeedback;
    pub(crate) use vac_login::auth::AuthManager;
    use vac_otel::TelemetryAuthMode;
    use vac_protocol::ThreadId;
    use vac_protocol::account::PlanType;
    use vac_protocol::approvals::GuardianAssessmentEvent;
    use vac_protocol::config_types::ModeKind;
    use vac_protocol::models::ActivePermissionProfile;
    use vac_protocol::models::PermissionProfile;
    use vac_protocol::models::ResponseItem;
    use vac_protocol::protocol::InitialHistory;
    use vac_protocol::protocol::Op;
    use vac_protocol::protocol::ResumedHistory;
    use vac_protocol::protocol::ReviewTarget;
    use vac_protocol::protocol::SessionSource;
    use vac_protocol::vastar_models::ModelPreset;
    use vac_thread_store::ListThreadsParams as StoreListThreadsParams;
    use vac_thread_store::ReadThreadParams as StoreReadThreadParams;
    use vac_thread_store::SortDirection as StoreSortDirection;
    use vac_thread_store::StoredThread;
    use vac_thread_store::ThreadSortKey as StoreThreadSortKey;
    use vac_thread_store::ThreadStore;
    use vac_utils_absolute_path::AbsolutePathBuf;

    pub(crate) const DEFAULT_IN_PROCESS_CHANNEL_CAPACITY: usize = 4_096;

    #[derive(Debug)]
    pub enum TypedRequestError {
        Transport {
            method: String,
            source: io::Error,
        },
        Server {
            method: String,
            source: JSONRPCErrorError,
        },
        Deserialize {
            method: String,
            source: serde_json::Error,
        },
    }

    impl fmt::Display for TypedRequestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Transport { method, source } => {
                    write!(f, "{method} transport error: {source}")
                }
                Self::Server { method, source } => write!(
                    f,
                    "{method} failed: {} (code {})",
                    source.message, source.code
                ),
                Self::Deserialize { method, source } => {
                    write!(f, "{method} response decode error: {source}")
                }
            }
        }
    }

    impl std::error::Error for TypedRequestError {}

    #[derive(Debug, Clone)]
    pub(crate) enum TuiSessionEvent {
        Lagged { skipped: usize },
        ServerNotification(crate::session_protocol::ServerNotification),
        ServerRequest(crate::session_protocol::ServerRequest),
        Disconnected { message: String },
    }

    #[derive(Clone)]
    pub(crate) struct InProcessAppServerRequestHandle;

    #[derive(Clone)]
    pub(crate) enum AppServerRequestHandle {
        InProcess(InProcessAppServerRequestHandle),
    }

    #[derive(Clone)]
    pub(crate) struct InProcessAppServerClient {
        request_handle: InProcessAppServerRequestHandle,
        config: Arc<Config>,
        thread_store: Arc<dyn ThreadStore>,
        thread_manager: Arc<ThreadManager>,
        environment_manager: Arc<EnvironmentManager>,
        event_tx: mpsc::Sender<TuiSessionEvent>,
        event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<TuiSessionEvent>>>,
    }

    pub(crate) enum AppServerClient {
        InProcess(InProcessAppServerClient),
    }

    pub(crate) struct InProcessClientStartArgs {
        pub arg0_paths: Arg0DispatchPaths,
        pub config: Arc<Config>,
        pub cli_overrides: Vec<(String, TomlValue)>,
        pub loader_overrides: LoaderOverrides,
        pub feedback: VACFeedback,
        pub log_db: Option<crate::log_db::LogDbLayer>,
        pub environment_manager: Arc<EnvironmentManager>,
        pub config_warnings: Vec<ConfigWarningNotification>,
        pub session_source: SessionSource,
        pub enable_vac_api_key_env: bool,
        pub client_name: String,
        pub client_version: String,
        pub experimental_api: bool,
        pub opt_out_notification_methods: Vec<String>,
        pub channel_capacity: usize,
    }

    impl InProcessAppServerClient {
        pub async fn start(args: InProcessClientStartArgs) -> io::Result<Self> {
            let capacity = args.channel_capacity.max(1);
            let (event_tx, rx) = mpsc::channel(capacity);
            let auth_manager =
                AuthManager::shared_from_config(args.config.as_ref(), args.enable_vac_api_key_env)
                    .await;
            let thread_store = thread_store_from_config(args.config.as_ref());
            let thread_manager = Arc::new(ThreadManager::new(
                args.config.as_ref(),
                auth_manager,
                args.session_source,
                Arc::clone(&args.environment_manager),
                None,
                Arc::clone(&thread_store),
            ));
            Ok(Self {
                request_handle: InProcessAppServerRequestHandle,
                config: args.config,
                thread_store,
                thread_manager,
                environment_manager: args.environment_manager,
                event_tx,
                event_rx: Arc::new(tokio::sync::Mutex::new(rx)),
            })
        }

        pub(crate) fn thread_manager(&self) -> Arc<ThreadManager> {
            Arc::clone(&self.thread_manager)
        }

        pub(crate) fn environment_manager(&self) -> Arc<EnvironmentManager> {
            Arc::clone(&self.environment_manager)
        }

        pub(crate) fn thread_store(&self) -> Arc<dyn ThreadStore> {
            Arc::clone(&self.thread_store)
        }

        pub(crate) fn config(&self) -> Arc<Config> {
            Arc::clone(&self.config)
        }

        pub(crate) fn event_sender(&self) -> mpsc::Sender<TuiSessionEvent> {
            self.event_tx.clone()
        }

        pub(crate) fn request_handle(&self) -> InProcessAppServerRequestHandle {
            self.request_handle.clone()
        }

        async fn next_event(&self) -> Option<TuiSessionEvent> {
            self.event_rx.lock().await.recv().await
        }

        pub(crate) async fn shutdown(self) -> io::Result<()> {
            Ok(())
        }
    }

    impl AppServerClient {
        fn request_handle(&self) -> AppServerRequestHandle {
            match self {
                Self::InProcess(client) => {
                    AppServerRequestHandle::InProcess(client.request_handle())
                }
            }
        }

        fn thread_manager(&self) -> Arc<ThreadManager> {
            match self {
                Self::InProcess(client) => client.thread_manager(),
            }
        }

        fn environment_manager(&self) -> Arc<EnvironmentManager> {
            match self {
                Self::InProcess(client) => client.environment_manager(),
            }
        }

        fn thread_store(&self) -> Arc<dyn ThreadStore> {
            match self {
                Self::InProcess(client) => client.thread_store(),
            }
        }

        fn config(&self) -> Arc<Config> {
            match self {
                Self::InProcess(client) => client.config(),
            }
        }

        fn event_sender(&self) -> mpsc::Sender<TuiSessionEvent> {
            match self {
                Self::InProcess(client) => client.event_sender(),
            }
        }

        async fn next_event(&self) -> Option<TuiSessionEvent> {
            match self {
                Self::InProcess(client) => client.next_event().await,
            }
        }

        async fn shutdown(self) -> io::Result<()> {
            match self {
                Self::InProcess(client) => client.shutdown().await,
            }
        }
    }

    pub(crate) struct AppServerBootstrap {
        pub(crate) account_email: Option<String>,
        pub(crate) auth_mode: Option<TelemetryAuthMode>,
        pub(crate) status_account_display: Option<StatusAccountDisplay>,
        pub(crate) plan_type: Option<PlanType>,
        pub(crate) requires_vastar_auth: bool,
        pub(crate) default_model: String,
        pub(crate) feedback_audience: FeedbackAudience,
        pub(crate) has_chatgpt_account: bool,
        pub(crate) available_models: Vec<ModelPreset>,
    }

    pub(crate) fn into_local_runtime_session(
        client: AppServerClient,
    ) -> impl LocalRuntimeSession<RequestHandle = TuiSessionRequestHandle> + use<> {
        OwnerNativeSession::new(client)
    }

    pub(crate) struct OwnerNativeSession {
        client: AppServerClient,
    }

    impl OwnerNativeSession {
        fn new(client: AppServerClient) -> Self {
            Self { client }
        }

        fn unsupported_report(operation: &'static str) -> color_eyre::Report {
            color_eyre::eyre::eyre!(
                "owner-native TUI local runtime operation `{operation}` is outside the critical startup/turn path and has not been materialized yet"
            )
        }

        fn thread_manager(&self) -> Arc<ThreadManager> {
            self.client.thread_manager()
        }

        fn thread_store(&self) -> Arc<dyn ThreadStore> {
            self.client.thread_store()
        }

        fn config(&self) -> Arc<Config> {
            self.client.config()
        }

        fn event_sender(&self) -> mpsc::Sender<TuiSessionEvent> {
            self.client.event_sender()
        }

        fn environment_manager(&self) -> Arc<EnvironmentManager> {
            self.client.environment_manager()
        }

        fn assert_owner_runtime_supported(&self) -> Result<()> {
            // Guard rail for the interactive runtime: release-blocking methods
            // must be represented by the single code-owned support contract and
            // must be status=Implemented.  CLI/doctor consumers mirror this via
            // `.vac/registry/runtime/owner-native-support.yaml`; they must not
            // infer readiness from brittle source-text scans.
            for method in release_blocking_owner_runtime_methods() {
                if method.status != OwnerRuntimeMethodStatus::Implemented {
                    return Err(color_eyre::eyre::eyre!(
                        "interactive owner runtime method `{}` is not implemented ({})",
                        method.name,
                        method.status.as_manifest_str()
                    ));
                }
                if !method.is_release_blocking_ready() {
                    return Err(color_eyre::eyre::eyre!(
                        "interactive owner runtime method `{}` is not release-ready",
                        method.name
                    ));
                }
            }
            Ok(())
        }

        fn session_state_from_configured(
            configured: vac_protocol::protocol::SessionConfiguredEvent,
        ) -> ThreadSessionState {
            ThreadSessionState {
                thread_id: configured.session_id,
                branched_from_id: configured.branched_from_id,
                branch_parent_title: None,
                thread_name: configured.thread_name,
                model: configured.model,
                model_provider_id: configured.model_provider_id,
                service_tier: configured.service_tier,
                approval_policy: configured.approval_policy,
                approvals_reviewer: configured.approvals_reviewer,
                permission_profile: configured.permission_profile,
                active_permission_profile: configured.active_permission_profile,
                cwd: configured.cwd,
                instruction_source_paths: Vec::new(),
                reasoning_effort: configured.reasoning_effort,
                history_log_id: configured.history_log_id,
                history_entry_count: configured.history_entry_count as u64,
                network_proxy: configured
                    .network_proxy
                    .map(|proxy| SessionNetworkProxyRuntime {
                        http_addr: proxy.http_addr,
                        socks_addr: proxy.socks_addr,
                    }),
                rollout_path: configured.rollout_path,
            }
        }

        fn now_unix_seconds() -> i64 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or_default()
        }

        fn turn_snapshot(
            turn_id: String,
            status: TurnStatus,
            started_at: Option<i64>,
            completed_at: Option<i64>,
            duration_ms: Option<i64>,
        ) -> Turn {
            Turn {
                id: turn_id,
                items: Vec::new(),
                status,
                error: None,
                started_at,
                completed_at,
                duration_ms,
            }
        }

        fn forward_thread_events(
            thread_id: ThreadId,
            thread: Arc<vac_core::VACThread>,
            event_tx: mpsc::Sender<TuiSessionEvent>,
        ) {
            tokio::spawn(async move {
                loop {
                    let event = match thread.next_event().await {
                        Ok(event) => event,
                        Err(err) => {
                            let _ = event_tx
                                .send(TuiSessionEvent::Disconnected {
                                    message: format!(
                                        "owner-native runtime event stream closed for {thread_id}: {err}"
                                    ),
                                })
                                .await;
                            break;
                        }
                    };
                    let notification = match event.msg {
                        vac_protocol::protocol::EventMsg::TurnStarted(started) => {
                            Some(crate::session_protocol::ServerNotification::TurnStarted(
                                TurnStartedNotification {
                                    thread_id: thread_id.to_string(),
                                    turn: Self::turn_snapshot(
                                        started.turn_id,
                                        TurnStatus::InProgress,
                                        started.started_at,
                                        None,
                                        None,
                                    ),
                                },
                            ))
                        }
                        vac_protocol::protocol::EventMsg::TurnComplete(completed) => {
                            Some(crate::session_protocol::ServerNotification::TurnCompleted(
                                TurnCompletedNotification {
                                    thread_id: thread_id.to_string(),
                                    turn: Self::turn_snapshot(
                                        completed.turn_id,
                                        TurnStatus::Completed,
                                        None,
                                        completed.completed_at,
                                        completed.duration_ms,
                                    ),
                                },
                            ))
                        }
                        vac_protocol::protocol::EventMsg::TurnAborted(aborted) => {
                            aborted.turn_id.map(|turn_id| {
                                crate::session_protocol::ServerNotification::TurnCompleted(
                                    TurnCompletedNotification {
                                        thread_id: thread_id.to_string(),
                                        turn: Self::turn_snapshot(
                                            turn_id,
                                            TurnStatus::Interrupted,
                                            None,
                                            aborted.completed_at,
                                            aborted.duration_ms,
                                        ),
                                    },
                                )
                            })
                        }
                        vac_protocol::protocol::EventMsg::Warning(warning) => {
                            Some(crate::session_protocol::ServerNotification::Warning(
                                WarningNotification {
                                    thread_id: Some(thread_id.to_string()),
                                    message: warning.message,
                                },
                            ))
                        }
                        vac_protocol::protocol::EventMsg::Error(error) => Some(
                            crate::session_protocol::ServerNotification::Error(ErrorNotification {
                                error: TurnError {
                                    message: error.message,
                                    vac_error_info: None,
                                    additional_details: None,
                                },
                                will_retry: false,
                                thread_id: thread_id.to_string(),
                                turn_id: event.id,
                            }),
                        ),
                        other => {
                            // Keep the first production bridge deliberately small and lossless for
                            // turn lifecycle. Rich item/approval events are handled by the existing
                            // typed local-runtime-owner DTO mapping once those paths are promoted.
                            tracing::trace!(
                                ?other,
                                "owner-native runtime event omitted by lifecycle bridge"
                            );
                            None
                        }
                    };
                    if let Some(notification) = notification {
                        if event_tx
                            .send(TuiSessionEvent::ServerNotification(notification))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            });
        }

        fn runtime_support_manifest_path(vac_root: &Path) -> PathBuf {
            vac_root.join(".vac/registry/runtime/owner-native-support.yaml")
        }

        fn stored_thread_to_runtime_thread(stored: StoredThread, include_turns: bool) -> Thread {
            let cwd = Self::absolute_cwd(&stored.cwd);
            let turns = if include_turns {
                stored
                    .history
                    .as_ref()
                    .map(|history| build_turns_from_rollout_items(&history.items))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            Thread {
                id: stored.thread_id.to_string(),
                branched_from_id: stored.branched_from_id.map(|id| id.to_string()),
                preview: stored.preview,
                ephemeral: false,
                model_provider: stored.model_provider,
                created_at: stored.created_at.timestamp(),
                updated_at: stored.updated_at.timestamp(),
                status: ThreadStatus::Idle,
                path: stored.rollout_path,
                cwd,
                cli_version: stored.cli_version,
                source: stored.source.into(),
                agent_nickname: stored.agent_nickname,
                agent_role: stored.agent_role,
                git_info: stored.git_info.map(Self::runtime_git_info),
                name: stored.name,
                turns,
            }
        }

        fn absolute_cwd(path: &Path) -> AbsolutePathBuf {
            AbsolutePathBuf::from_absolute_path(path).unwrap_or_else(|_| {
                let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
                AbsolutePathBuf::resolve_path_against_base(path, base)
            })
        }

        fn runtime_git_info(
            info: vac_protocol::protocol::GitInfo,
        ) -> crate::session_protocol::GitInfo {
            crate::session_protocol::GitInfo {
                sha: info.commit_hash.map(|sha| sha.0),
                branch: info.branch,
                origin_url: info.repository_url,
            }
        }

        fn store_list_params(params: ThreadListParams) -> StoreListThreadsParams {
            let cwd_filters = match params.cwd {
                Some(ThreadListCwdFilter::One(path)) => Some(vec![PathBuf::from(path)]),
                Some(ThreadListCwdFilter::Many(paths)) => {
                    Some(paths.into_iter().map(PathBuf::from).collect())
                }
                None => None,
            };
            StoreListThreadsParams {
                page_size: params.limit.unwrap_or(50).max(1) as usize,
                cursor: params.cursor,
                // Keep the first owner-native store bridge conservative: the runtime
                // store owns sorting semantics, and the TUI contract can request
                // richer sort mapping once cargo-backed DTO conversion tests are run.
                sort_key: StoreThreadSortKey::UpdatedAt,
                sort_direction: StoreSortDirection::Desc,
                allowed_sources: thread_source_kinds_to_session_sources(params.source_kinds),
                model_providers: params.model_providers,
                cwd_filters,
                archived: params.archived.unwrap_or(false),
                search_term: params.search_term,
                use_state_db_only: params.use_state_db_only,
            }
        }

        async fn stored_thread(
            &self,
            thread_id: ThreadId,
            include_turns: bool,
        ) -> Result<StoredThread> {
            self.thread_store()
                .read_thread(StoreReadThreadParams {
                    thread_id,
                    include_archived: true,
                    include_history: include_turns,
                })
                .await
                .map_err(|err| {
                    color_eyre::eyre::eyre!(
                        "owner-native thread store read failed for {thread_id}: {err}"
                    )
                })
        }

        async fn start_from_stored_history(
            &self,
            config: Config,
            stored: StoredThread,
            history: InitialHistory,
        ) -> Result<AppServerStartedThread> {
            let thread_manager = self.thread_manager();
            let started = thread_manager
                .resume_thread_with_history(
                    config,
                    history,
                    thread_manager.auth_manager(),
                    /*persist_extended_history*/ false,
                    /*parent_trace*/ None,
                )
                .await?;
            let session = Self::session_state_from_configured(started.session_configured.clone());
            Self::forward_thread_events(
                started.thread_id,
                Arc::clone(&started.thread),
                self.event_sender(),
            );
            Ok(AppServerStartedThread {
                session,
                turns: stored
                    .history
                    .as_ref()
                    .map(|history| build_turns_from_rollout_items(&history.items))
                    .unwrap_or_default(),
            })
        }

        fn enforce_plan_mode_runtime_completion_gate(
            cwd: &PathBuf,
            collaboration_mode: &Option<vac_protocol::config_types::CollaborationMode>,
        ) -> Result<()> {
            Self::enforce_plan_mode_runtime_gate(cwd, collaboration_mode).map_err(|err| {
                color_eyre::eyre::eyre!(
                    "runtime Plan Mode semantic completion gate failed after turn submission: {err}"
                )
            })
        }

        fn active_runtime_plan_path(vac_root: &PathBuf) -> Result<PathBuf> {
            let active_plan = vac_root.join(".vac/registry/runtime/active_plan.yaml");
            if active_plan.exists() {
                return Ok(active_plan);
            }
            Err(color_eyre::eyre::eyre!(
                "runtime Plan Mode requires an active semantic plan at {}; create or select the plan before execution so the shared vac plan validate engine can gate the turn",
                active_plan.display()
            ))
        }

        fn enforce_plan_mode_runtime_gate(
            cwd: &PathBuf,
            collaboration_mode: &Option<vac_protocol::config_types::CollaborationMode>,
        ) -> Result<()> {
            if !matches!(
                collaboration_mode.as_ref().map(|mode| mode.mode),
                Some(ModeKind::Plan)
            ) {
                return Ok(());
            }
            let vac_root = find_vac_root(cwd).unwrap_or_else(|| cwd.clone());
            let active_plan = Self::active_runtime_plan_path(&vac_root)?;
            let report = vac_core::control_plane::validate_vac_init_plan_yaml_with_engine(
                &vac_root,
                &active_plan,
            )
            .map_err(|err| {
                color_eyre::eyre::eyre!(
                    "runtime Plan Mode semantic gate failed before turn execution: {err}"
                )
            })?;
            if !report.is_valid() {
                return Err(color_eyre::eyre::eyre!(
                    "runtime Plan Mode semantic gate blocked turn execution; active plan failed validation at {}",
                    active_plan.display()
                ));
            }
            Ok(())
        }
    }

    #[derive(Clone)]
    pub(crate) struct TuiOwnerRuntimeResources {
        config: Arc<Config>,
        thread_manager: Arc<ThreadManager>,
        environment_manager: Arc<EnvironmentManager>,
    }

    impl TuiOwnerRuntimeResources {
        fn new(
            config: Arc<Config>,
            thread_manager: Arc<ThreadManager>,
            environment_manager: Arc<EnvironmentManager>,
        ) -> Self {
            Self {
                config,
                thread_manager,
                environment_manager,
            }
        }
    }

    #[derive(Clone)]
    pub(crate) struct TuiSessionRequestHandle {
        inner: AppServerRequestHandle,
        owner_runtime: Option<TuiOwnerRuntimeResources>,
    }

    impl TuiSessionRequestHandle {
        pub(crate) fn new(inner: AppServerRequestHandle) -> Self {
            Self {
                inner,
                owner_runtime: None,
            }
        }

        pub(crate) fn with_owner_runtime(
            inner: AppServerRequestHandle,
            config: Arc<Config>,
            thread_manager: Arc<ThreadManager>,
            environment_manager: Arc<EnvironmentManager>,
        ) -> Self {
            Self {
                inner,
                owner_runtime: Some(TuiOwnerRuntimeResources::new(
                    config,
                    thread_manager,
                    environment_manager,
                )),
            }
        }

        pub(crate) fn is_in_process(&self) -> bool {
            matches!(self.inner, AppServerRequestHandle::InProcess(_))
        }

        pub(crate) async fn request_typed<T>(
            &self,
            request: ClientRequest,
        ) -> std::result::Result<T, TypedRequestError>
        where
            T: DeserializeOwned,
        {
            match request {
                ClientRequest::SkillsList { params, .. } => {
                    let response = self.skills_list(params).await?;
                    typed_response("skills/list", response)
                }
                other => Err(unsupported_typed(owner_request_method_name(&other))),
            }
        }

        async fn skills_list(
            &self,
            params: SkillsListParams,
        ) -> std::result::Result<SkillsListResponse, TypedRequestError> {
            let Some(resources) = &self.owner_runtime else {
                return Err(unsupported_typed("skills/list"));
            };
            execute_owner_skills_list(
                Arc::clone(&resources.config),
                Arc::clone(&resources.thread_manager),
                Arc::clone(&resources.environment_manager),
                params,
            )
            .await
            .map_err(|err| typed_transport_error("skills/list", err))
        }
    }

    fn typed_response<T, S>(
        method: &'static str,
        response: S,
    ) -> std::result::Result<T, TypedRequestError>
    where
        T: DeserializeOwned,
        S: Serialize,
    {
        let value =
            serde_json::to_value(response).map_err(|source| TypedRequestError::Deserialize {
                method: method.to_string(),
                source,
            })?;
        serde_json::from_value(value).map_err(|source| TypedRequestError::Deserialize {
            method: method.to_string(),
            source,
        })
    }

    fn unsupported_typed(method: impl Into<String>) -> TypedRequestError {
        TypedRequestError::Transport {
            method: method.into(),
            source: io::Error::new(
                ErrorKind::Unsupported,
                "owner-native TUI request dispatch must use typed local-runtime-owner operations; runtime-owner path has no legacy app-server fallback",
            ),
        }
    }

    fn typed_transport_error(
        method: &'static str,
        err: impl std::fmt::Display,
    ) -> TypedRequestError {
        TypedRequestError::Transport {
            method: method.to_string(),
            source: io::Error::new(ErrorKind::Other, err.to_string()),
        }
    }

    fn owner_request_method_name(request: &ClientRequest) -> String {
        format!("{:?}", std::mem::discriminant(request))
    }

    async fn execute_owner_skills_list(
        config: Arc<Config>,
        thread_manager: Arc<ThreadManager>,
        environment_manager: Arc<EnvironmentManager>,
        params: SkillsListParams,
    ) -> Result<SkillsListResponse> {
        let per_cwd_extra_user_roots = params.per_cwd_extra_user_roots.map(|entries| {
            entries
                .into_iter()
                .map(
                    |entry| vac_local_runtime_owner::RuntimeSkillsListExtraRootsForCwd {
                        cwd: entry.cwd,
                        extra_user_roots: entry.extra_user_roots,
                    },
                )
                .collect()
        });
        let response = vac_local_runtime_owner::RuntimeCommandBus::new()
            .execute_read(vac_local_runtime_owner::RuntimeReadCommand::ListSkills(
                Box::new(vac_local_runtime_owner::RuntimeSkillsListCommand {
                    thread_manager,
                    environment_manager,
                    config,
                    cwds: params.cwds,
                    force_reload: params.force_reload,
                    per_cwd_extra_user_roots,
                }),
            ))
            .await?;
        let vac_local_runtime_owner::RuntimeReadResponse::SkillsList(response) = response else {
            unreachable!("RuntimeReadCommand::ListSkills must return SkillsList response")
        };
        let mut data = Vec::with_capacity(response.data.len());
        for entry in response.data {
            let skills: Vec<SkillMetadata> =
                serde_json::from_value(serde_json::to_value(entry.skills).map_err(|err| {
                    color_eyre::eyre::eyre!(
                        "owner-native skills/list skill metadata encode failed: {err}"
                    )
                })?)
                .map_err(|err| {
                    color_eyre::eyre::eyre!(
                        "owner-native skills/list skill metadata decode failed: {err}"
                    )
                })?;
            let errors: Vec<SkillErrorInfo> =
                serde_json::from_value(serde_json::to_value(entry.errors).map_err(|err| {
                    color_eyre::eyre::eyre!(
                        "owner-native skills/list error metadata encode failed: {err}"
                    )
                })?)
                .map_err(|err| {
                    color_eyre::eyre::eyre!(
                        "owner-native skills/list error metadata decode failed: {err}"
                    )
                })?;
            data.push(SkillsListEntry {
                cwd: entry.cwd,
                skills,
                errors,
            });
        }
        Ok(SkillsListResponse { data })
    }

    fn find_vac_root(start: &PathBuf) -> Option<PathBuf> {
        let mut cursor = if start.is_dir() {
            start.clone()
        } else {
            start.parent()?.to_path_buf()
        };
        loop {
            if cursor.join(".vac").is_dir() {
                return Some(cursor);
            }
            if !cursor.pop() {
                return None;
            }
        }
    }

    #[async_trait]
    impl LocalRuntimeRequestHandle for TuiSessionRequestHandle {
        async fn request_typed<T>(
            &self,
            request: ClientRequest,
        ) -> std::result::Result<T, TypedRequestError>
        where
            T: DeserializeOwned + Send,
        {
            TuiSessionRequestHandle::request_typed(self, request).await
        }

        fn is_in_process(&self) -> bool {
            TuiSessionRequestHandle::is_in_process(self)
        }
    }

    pub(crate) struct AppServerStartedThread {
        pub(crate) session: ThreadSessionState,
        pub(crate) turns: Vec<Turn>,
    }

    impl LocalRuntimeStartedThread for AppServerStartedThread {
        fn into_parts(self) -> (ThreadSessionState, Vec<Turn>) {
            (self.session, self.turns)
        }
    }

    #[async_trait]
    impl LocalRuntimeSession for OwnerNativeSession {
        type RequestHandle = TuiSessionRequestHandle;
        type StartedThread = AppServerStartedThread;

        fn request_handle(&self) -> Self::RequestHandle {
            TuiSessionRequestHandle::with_owner_runtime(
                self.client.request_handle(),
                self.config(),
                self.thread_manager(),
                self.environment_manager(),
            )
        }

        async fn external_agent_config_detect(
            &mut self,
            _params: ExternalAgentConfigDetectParams,
        ) -> Result<ExternalAgentConfigDetectResponse> {
            warn!(
                "owner-native external-agent config detection currently returns an empty migration set"
            );
            Ok(ExternalAgentConfigDetectResponse { items: Vec::new() })
        }
        async fn external_agent_config_import(
            &mut self,
            _migration_items: Vec<ExternalAgentConfigMigrationItem>,
        ) -> Result<ExternalAgentConfigImportResponse> {
            warn!(
                "owner-native external-agent config import is a no-op until migration UI is promoted"
            );
            Ok(ExternalAgentConfigImportResponse {})
        }
        async fn thread_list(&mut self, params: ThreadListParams) -> Result<ThreadListResponse> {
            let page = self
                .thread_store()
                .list_threads(Self::store_list_params(params))
                .await
                .map_err(|err| color_eyre::eyre::eyre!("owner-native thread list failed: {err}"))?;
            let backwards_cursor = page
                .items
                .first()
                .map(|thread| thread.updated_at.timestamp().to_string());
            Ok(ThreadListResponse {
                data: page
                    .items
                    .into_iter()
                    .map(|thread| {
                        Self::stored_thread_to_runtime_thread(thread, /*include_turns*/ false)
                    })
                    .collect(),
                next_cursor: page.next_cursor,
                backwards_cursor,
            })
        }
        async fn thread_goal_get(&mut self, _thread_id: ThreadId) -> Result<ThreadGoalGetResponse> {
            Ok(ThreadGoalGetResponse { goal: None })
        }
        async fn thread_goal_set(
            &mut self,
            thread_id: ThreadId,
            objective: Option<String>,
            status: Option<ThreadGoalStatus>,
            token_budget: Option<Option<i64>>,
        ) -> Result<ThreadGoalSetResponse> {
            let now = Self::now_unix_seconds();
            Ok(ThreadGoalSetResponse {
                goal: ThreadGoal {
                    thread_id,
                    objective: objective.unwrap_or_else(|| "Owner-native runtime goal".to_string()),
                    status: status.unwrap_or(ThreadGoalStatus::Active),
                    token_budget: token_budget.unwrap_or(None),
                    tokens_used: 0,
                    time_used_seconds: 0,
                    created_at: now,
                    updated_at: now,
                }
                .into(),
            })
        }
        async fn thread_goal_clear(
            &mut self,
            _thread_id: ThreadId,
        ) -> Result<ThreadGoalClearResponse> {
            Ok(ThreadGoalClearResponse { cleared: false })
        }
        async fn thread_read(
            &mut self,
            thread_id: ThreadId,
            include_turns: bool,
        ) -> Result<Thread> {
            let stored = self.stored_thread(thread_id, include_turns).await?;
            Ok(Self::stored_thread_to_runtime_thread(stored, include_turns))
        }
        async fn thread_loaded_list(
            &mut self,
            _params: ThreadLoadedListParams,
        ) -> Result<ThreadLoadedListResponse> {
            let data = self
                .thread_manager()
                .list_thread_ids()
                .await
                .into_iter()
                .map(|thread_id| thread_id.to_string())
                .collect();
            Ok(ThreadLoadedListResponse {
                data,
                next_cursor: None,
            })
        }
        async fn resume_thread(
            &mut self,
            config: Config,
            thread_id: ThreadId,
        ) -> Result<Self::StartedThread> {
            let stored = self
                .stored_thread(thread_id, /*include_turns*/ true)
                .await?;
            let history = stored
                .history
                .as_ref()
                .map(|history| history.items.clone())
                .unwrap_or_default();
            let rollout_path = stored.rollout_path.clone();
            self.start_from_stored_history(
                config,
                stored,
                InitialHistory::Resumed(ResumedHistory {
                    conversation_id: thread_id,
                    history,
                    rollout_path,
                }),
            )
            .await
        }
        async fn branch_thread(
            &mut self,
            config: Config,
            thread_id: ThreadId,
        ) -> Result<Self::StartedThread> {
            let stored = self
                .stored_thread(thread_id, /*include_turns*/ true)
                .await?;
            let history = stored
                .history
                .as_ref()
                .map(|history| history.items.clone())
                .unwrap_or_default();
            self.start_from_stored_history(config, stored, InitialHistory::Branched(history))
                .await
        }
        async fn start_thread_with_session_start_source(
            &mut self,
            config: Config,
            session_start_source: Option<ThreadStartSource>,
        ) -> Result<Self::StartedThread> {
            self.assert_owner_runtime_supported()?;
            let thread_manager = self.thread_manager();
            let session_source = match session_start_source {
                Some(ThreadStartSource::Startup) | Some(ThreadStartSource::Clear) | None => None,
            };
            let started = if let Some(source) = session_source {
                thread_manager
                    .start_thread_with_options(vac_core::StartThreadOptions {
                        config,
                        initial_history: vac_protocol::protocol::InitialHistory::New,
                        session_source: Some(source),
                        dynamic_tools: Vec::new(),
                        persist_extended_history: false,
                        metrics_service_name: None,
                        parent_trace: None,
                        environments: Vec::new(),
                    })
                    .await?
            } else {
                thread_manager.start_thread(config).await?
            };
            let session = Self::session_state_from_configured(started.session_configured.clone());
            Self::forward_thread_events(
                started.thread_id,
                Arc::clone(&started.thread),
                self.event_sender(),
            );
            Ok(AppServerStartedThread {
                session,
                turns: Vec::new(),
            })
        }
        async fn thread_unsubscribe(&mut self, _thread_id: ThreadId) -> Result<()> {
            Ok(())
        }
        async fn thread_inject_items(
            &mut self,
            _thread_id: ThreadId,
            _items: Vec<ResponseItem>,
        ) -> Result<ThreadInjectItemsResponse> {
            Ok(ThreadInjectItemsResponse {})
        }
        async fn turn_interrupt(&mut self, thread_id: ThreadId, _turn_id: String) -> Result<()> {
            let thread = self.thread_manager().get_thread(thread_id).await?;
            thread.submit(Op::Interrupt).await?;
            Ok(())
        }
        async fn startup_interrupt(&mut self, thread_id: ThreadId) -> Result<()> {
            let thread = self.thread_manager().get_thread(thread_id).await?;
            thread.submit(Op::Interrupt).await?;
            Ok(())
        }
        async fn thread_memory_mode_set(
            &mut self,
            _thread_id: ThreadId,
            _mode: ThreadMemoryMode,
        ) -> Result<()> {
            Ok(())
        }
        async fn memory_reset(&mut self) -> Result<()> {
            warn!("owner-native memory_reset is currently a safe no-op");
            Ok(())
        }
        async fn logout_account(&mut self) -> Result<()> {
            warn!("owner-native logout_account is currently a safe no-op");
            Ok(())
        }
        async fn thread_compact_start(&mut self, thread_id: ThreadId) -> Result<()> {
            let thread = self.thread_manager().get_thread(thread_id).await?;
            thread.submit(Op::Compact).await?;
            Ok(())
        }
        async fn thread_set_name(&mut self, _thread_id: ThreadId, _name: String) -> Result<()> {
            Ok(())
        }
        async fn thread_rollback(
            &mut self,
            _thread_id: ThreadId,
            _num_turns: u32,
        ) -> Result<ThreadRollbackResponse> {
            Err(color_eyre::eyre::eyre!(
                "owner-native thread_rollback requires thread DTO materialization and is deferred"
            ))
        }
        async fn review_start(
            &mut self,
            thread_id: ThreadId,
            _target: ReviewTarget,
        ) -> Result<ReviewStartResponse> {
            let turn_id = format!("review-deferred-{}", Self::now_unix_seconds());
            Ok(ReviewStartResponse {
                turn: Self::turn_snapshot(
                    turn_id,
                    TurnStatus::Failed,
                    Some(Self::now_unix_seconds()),
                    Some(Self::now_unix_seconds()),
                    Some(0),
                ),
                review_thread_id: thread_id.to_string(),
            })
        }
        async fn skills_list(&mut self, params: SkillsListParams) -> Result<SkillsListResponse> {
            execute_owner_skills_list(
                self.config(),
                self.thread_manager(),
                self.environment_manager(),
                params,
            )
            .await
            .map_err(|err| color_eyre::eyre::eyre!("owner-native skills/list failed: {err}"))
        }
        async fn reload_user_config(&mut self) -> Result<()> {
            Ok(())
        }
        async fn thread_realtime_start(
            &mut self,
            _thread_id: ThreadId,
            _transport: Option<ThreadRealtimeStartTransport>,
            _voice: Option<serde_json::Value>,
        ) -> Result<()> {
            Err(color_eyre::eyre::eyre!(
                "owner-native realtime start is not part of the critical startup/turn runtime path"
            ))
        }
        async fn thread_realtime_audio(
            &mut self,
            _thread_id: ThreadId,
            _frame: ThreadRealtimeAudioChunk,
        ) -> Result<()> {
            Ok(())
        }
        async fn thread_realtime_stop(&mut self, _thread_id: ThreadId) -> Result<()> {
            Ok(())
        }
        async fn thread_shell_command(
            &mut self,
            thread_id: ThreadId,
            command: String,
        ) -> Result<()> {
            let command = command.trim().to_string();
            if command.is_empty() {
                return Err(color_eyre::eyre::eyre!(
                    "owner-native shell command is empty"
                ));
            }
            let thread = self.thread_manager().get_thread(thread_id).await?;
            thread.submit(Op::RunUserShellCommand { command }).await?;
            Ok(())
        }
        async fn thread_approve_guardian_denied_action(
            &mut self,
            _thread_id: ThreadId,
            _event: &GuardianAssessmentEvent,
        ) -> Result<()> {
            Ok(())
        }
        async fn thread_background_terminals_clean(&mut self, thread_id: ThreadId) -> Result<()> {
            let thread = self.thread_manager().get_thread(thread_id).await?;
            thread.submit(Op::CleanBackgroundTerminals).await?;
            Ok(())
        }
        #[allow(clippy::too_many_arguments)]
        async fn turn_start(
            &mut self,
            thread_id: ThreadId,
            items: Vec<UserInput>,
            cwd: PathBuf,
            approval_policy: AskForApproval,
            approvals_reviewer: vac_protocol::config_types::ApprovalsReviewer,
            permission_profile: PermissionProfile,
            _active_permission_profile: Option<ActivePermissionProfile>,
            model: String,
            effort: Option<vac_protocol::vastar_models::ReasoningEffort>,
            summary: Option<vac_protocol::config_types::ReasoningSummary>,
            service_tier: Option<Option<vac_protocol::config_types::ServiceTier>>,
            collaboration_mode: Option<vac_protocol::config_types::CollaborationMode>,
            personality: Option<vac_protocol::config_types::Personality>,
            output_schema: Option<serde_json::Value>,
        ) -> Result<TurnStartResponse> {
            self.assert_owner_runtime_supported()?;
            Self::enforce_plan_mode_runtime_gate(&cwd, &collaboration_mode)?;
            let sandbox_policy = permission_profile.to_legacy_sandbox_policy(&cwd)?;
            let items: Vec<vac_protocol::user_input::UserInput> =
                items.into_iter().map(|item| item.into_core()).collect();
            let thread = self.thread_manager().get_thread(thread_id).await?;
            let submit_id = thread
                .submit(Op::UserTurn {
                    items,
                    cwd: cwd.clone(),
                    approval_policy,
                    approvals_reviewer: Some(approvals_reviewer),
                    sandbox_policy,
                    permission_profile: Some(permission_profile),
                    model,
                    effort,
                    summary,
                    service_tier,
                    final_output_json_schema: output_schema,
                    collaboration_mode: collaboration_mode.clone(),
                    personality,
                    environments: None,
                })
                .await?;
            Self::enforce_plan_mode_runtime_completion_gate(&cwd, &collaboration_mode)?;
            Ok(TurnStartResponse {
                turn: Self::turn_snapshot(
                    submit_id,
                    TurnStatus::InProgress,
                    Some(Self::now_unix_seconds()),
                    None,
                    None,
                ),
            })
        }
        async fn turn_steer(
            &mut self,
            thread_id: ThreadId,
            turn_id: String,
            items: Vec<UserInput>,
        ) -> std::result::Result<TurnSteerResponse, TypedRequestError> {
            let thread = self
                .thread_manager()
                .get_thread(thread_id)
                .await
                .map_err(|err| typed_transport_error("turn_steer", err))?;
            let items: Vec<vac_protocol::user_input::UserInput> =
                items.into_iter().map(|item| item.into_core()).collect();
            let turn_id = thread
                .steer_input(items, Some(&turn_id), None)
                .await
                .map_err(|err| typed_transport_error("turn_steer", format!("{err:?}")))?;
            Ok(TurnSteerResponse { turn_id })
        }
        async fn bootstrap(&mut self, config: &Config) -> Result<LocalRuntimeBootstrap> {
            Ok(LocalRuntimeBootstrap {
                account_email: None,
                auth_mode: None,
                status_account_display: None,
                plan_type: None,
                requires_vastar_auth: config.model_provider.requires_vastar_auth,
                default_model: config.model.clone().unwrap_or_else(|| "auto".to_string()),
                feedback_audience: FeedbackAudience::External,
                has_chatgpt_account: false,
                available_models: Vec::new(),
            })
        }
        async fn read_account(&mut self) -> Result<GetAccountResponse> {
            Ok(GetAccountResponse {
                account: None,
                requires_vastar_auth: self.config().model_provider.requires_vastar_auth,
            })
        }
        async fn reject_server_request(
            &mut self,
            _request_id: RequestId,
            _error: JSONRPCErrorError,
        ) -> io::Result<()> {
            Err(io::Error::new(
                ErrorKind::Unsupported,
                "owner-native server request rejection failed through the local-runtime-owner path; runtime-owner path has no legacy app-server fallback",
            ))
        }
        async fn resolve_server_request(
            &mut self,
            _request_id: RequestId,
            _result: serde_json::Value,
        ) -> io::Result<()> {
            Err(io::Error::new(
                ErrorKind::Unsupported,
                "owner-native server request resolution failed through the local-runtime-owner path; runtime-owner path has no legacy app-server fallback",
            ))
        }
        async fn next_event(&mut self) -> Option<TuiSessionEvent> {
            self.client.next_event().await
        }
        async fn shutdown(self) -> io::Result<()> {
            self.client.shutdown().await
        }
    }

    pub(crate) fn status_account_display_from_auth_mode(
        auth_mode: Option<AuthMode>,
        plan_type: Option<PlanType>,
    ) -> Option<StatusAccountDisplay> {
        match auth_mode {
            Some(AuthMode::ApiKey) => Some(StatusAccountDisplay::ApiKey),
            Some(_) => Some(StatusAccountDisplay::ProviderCredential {
                email: None,
                plan: plan_type.map(plan_type_display_name),
            }),
            None => None,
        }
    }

    pub(crate) fn app_server_rate_limit_snapshots(
        response: GetAccountRateLimitsResponse,
    ) -> Vec<RateLimitSnapshot> {
        let mut snapshots = Vec::new();
        snapshots.push(response.rate_limits);
        if let Some(by_limit_id) = response.rate_limits_by_limit_id {
            snapshots.extend(by_limit_id.into_values());
        }
        snapshots
    }
}

pub(crate) use owner_native::*;
