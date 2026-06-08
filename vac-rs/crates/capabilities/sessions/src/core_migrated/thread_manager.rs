use crate::SkillsManager;
use crate::agent::AgentControl;
use crate::config::Config;
use crate::config::ThreadStoreConfig;
use crate::environment_selection::default_thread_environment_selections;
use crate::environment_selection::resolve_environment_selections;
use crate::file_watcher::FileWatcher;
use crate::mcp::McpManager;
use crate::rollout::RolloutRecorder;
use crate::rollout::truncation;
use crate::session::INITIAL_SUBMIT_ID;
use crate::session::VAC;
use crate::session::VACSpawnArgs;
use crate::session::VACSpawnOk;
use crate::shell_snapshot::ShellSnapshot;
use crate::skills_watcher::SkillsWatcher;
use crate::skills_watcher::SkillsWatcherEvent;
use crate::tasks::InterruptedTurnHistoryMarker;
use crate::tasks::interrupted_turn_history_marker;
use crate::vac_thread::VACThread;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::runtime::RuntimeFlavor;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tracing::warn;
use vac_analytics::AnalyticsEventsClient;
use vac_core_plugins::PluginsManager;
use vac_exec_server::EnvironmentManager;
use vac_login::AuthManager;
use vac_login::VACAuth;
use vac_model_provider::create_model_provider;
use vac_model_provider_info::ModelProviderInfo;
use vac_model_provider_info::VASTAR_PROVIDER_ID;
use vac_models_manager::manager::RefreshStrategy;
use vac_models_manager::manager::SharedModelsManager;
use vac_protocol::ThreadId;
use vac_protocol::config_types::CollaborationModeMask;
use vac_protocol::error::Result as VACResult;
use vac_protocol::error::VACErr;
#[cfg(test)]
use vac_protocol::models::ResponseItem;
use vac_protocol::protocol::Event;
use vac_protocol::protocol::EventMsg;
use vac_protocol::protocol::InitialHistory;
use vac_protocol::protocol::McpServerRefreshConfig;
use vac_protocol::protocol::Op;
use vac_protocol::protocol::RolloutItem;
use vac_protocol::protocol::SessionConfiguredEvent;
use vac_protocol::protocol::SessionSource;
use vac_protocol::protocol::SubAgentSource;
use vac_protocol::protocol::TurnAbortReason;
use vac_protocol::protocol::TurnAbortedEvent;
use vac_protocol::protocol::TurnEnvironmentSelection;
use vac_protocol::protocol::W3cTraceContext;
use vac_protocol::vastar_models::ModelPreset;
use vac_runtime_protocol::ThreadHistoryBuilder;
use vac_runtime_protocol::TurnStatus;
use vac_state::DirectionalThreadSpawnEdgeStatus;
use vac_thread_store::InMemoryThreadStore;
use vac_thread_store::LocalThreadStore;
use vac_thread_store::LocalThreadStoreConfig;
use vac_thread_store::ReadThreadParams;
use vac_thread_store::RemoteThreadStore;
use vac_thread_store::StoredThread;
use vac_thread_store::ThreadStore;
use vac_thread_store::ThreadStoreError;
use vac_utils_absolute_path::AbsolutePathBuf;

const THREAD_CREATED_CHANNEL_CAPACITY: usize = 1024;
/// Test-only override for enabling thread-manager behaviors used by integration
/// tests.
///
/// In production builds this value should remain at its default (`false`) and
/// must not be toggled.
static FORCE_TEST_THREAD_MANAGER_BEHAVIOR: AtomicBool = AtomicBool::new(false);

type CapturedOps = Vec<(ThreadId, Op)>;
type SharedCapturedOps = Arc<std::sync::Mutex<CapturedOps>>;

pub(crate) fn set_thread_manager_test_mode_for_tests(enabled: bool) {
    FORCE_TEST_THREAD_MANAGER_BEHAVIOR.store(enabled, Ordering::Relaxed);
}

fn should_use_test_thread_manager_behavior() -> bool {
    FORCE_TEST_THREAD_MANAGER_BEHAVIOR.load(Ordering::Relaxed)
}

struct TempVACHomeGuard {
    path: PathBuf,
}

impl Drop for TempVACHomeGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn build_skills_watcher(skills_manager: Arc<SkillsManager>) -> Arc<SkillsWatcher> {
    if should_use_test_thread_manager_behavior()
        && let Ok(handle) = Handle::try_current()
        && handle.runtime_flavor() == RuntimeFlavor::CurrentThread
    {
        // The real watcher spins background tasks that can starve the
        // current-thread test runtime and cause event waits to time out.
        warn!("using noop skills watcher under current-thread test runtime");
        return Arc::new(SkillsWatcher::noop());
    }

    let file_watcher = match FileWatcher::new() {
        Ok(file_watcher) => Arc::new(file_watcher),
        Err(err) => {
            warn!("failed to initialize file watcher: {err}");
            Arc::new(FileWatcher::noop())
        }
    };
    let skills_watcher = Arc::new(SkillsWatcher::new(&file_watcher));

    let mut rx = skills_watcher.subscribe();
    let skills_manager = Arc::clone(&skills_manager);
    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(SkillsWatcherEvent::SkillsChanged { .. }) => {
                        skills_manager.clear_cache();
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        });
    } else {
        warn!("skills watcher listener skipped: no Tokio runtime available");
    }

    skills_watcher
}

/// Represents a newly created VAC thread (formerly called a conversation), including the first event
/// (which is [`EventMsg::SessionConfigured`]).
pub struct NewThread {
    pub thread_id: ThreadId,
    pub thread: Arc<VACThread>,
    pub session_configured: SessionConfiguredEvent,
}

// VAC-DEBT(owner=vac-o5o6-zero-residual,target=post-cargo-hardening)(ccunningham): Add an explicit non-interrupting live-turn snapshot once
// core can represent sampling boundaries directly instead of relying on
// whichever items happened to be persisted mid-turn.
//
// Two likely future variants:
// - `TruncateToLastSamplingBoundary` for callers that want a coherent branch from
//   the last stable model boundary without synthesizing an interrupt.
// - `WaitUntilNextSamplingBoundary` (or similar) for callers that prefer to
//   branch after the next sampling boundary rather than interrupting immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchSnapshot {
    /// Branch a committed prefix ending strictly before the nth user message.
    ///
    /// When `n` is within range, this cuts before that 0-based user-message
    /// boundary. When `n` is out of range and the source thread is currently
    /// mid-turn, this instead cuts before the active turn's opening boundary
    /// so the branch drops the unfinished turn suffix. When `n` is out of range
    /// and the source thread is already at a turn boundary, this returns the
    /// full committed history unchanged.
    TruncateBeforeNthUserMessage(usize),

    /// Branch the current persisted history as if the source thread had been
    /// interrupted now.
    ///
    /// If the persisted snapshot ends mid-turn, this appends the same
    /// `<turn_aborted>` marker produced by a real interrupt. If the snapshot is
    /// already at a turn boundary, this returns the current persisted history
    /// unchanged.
    Interrupted,
}

/// Preserve legacy `branch_thread(usize, ...)` callsites by mapping them to the
/// existing truncate-before-nth-user-message snapshot mode.
impl From<usize> for BranchSnapshot {
    fn from(value: usize) -> Self {
        Self::TruncateBeforeNthUserMessage(value)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ThreadShutdownReport {
    pub completed: Vec<ThreadId>,
    pub submit_failed: Vec<ThreadId>,
    pub timed_out: Vec<ThreadId>,
}

enum ShutdownOutcome {
    Complete,
    SubmitFailed,
    TimedOut,
}

/// [`ThreadManager`] is responsible for creating threads and maintaining
/// them in memory.
pub struct ThreadManager {
    state: Arc<ThreadManagerState>,
    _test_vac_home_guard: Option<TempVACHomeGuard>,
}

pub struct StartThreadOptions {
    pub config: Config,
    pub initial_history: InitialHistory,
    pub session_source: Option<SessionSource>,
    pub dynamic_tools: Vec<vac_protocol::dynamic_tools::DynamicToolSpec>,
    pub persist_extended_history: bool,
    pub metrics_service_name: Option<String>,
    pub parent_trace: Option<W3cTraceContext>,
    pub environments: Vec<TurnEnvironmentSelection>,
}

pub(crate) struct ResumeThreadWithHistoryOptions {
    pub(crate) config: Config,
    pub(crate) initial_history: InitialHistory,
    pub(crate) agent_control: AgentControl,
    pub(crate) session_source: SessionSource,
    pub(crate) inherited_shell_snapshot: Option<Arc<ShellSnapshot>>,
    pub(crate) inherited_exec_policy: Option<Arc<crate::exec_policy::ExecPolicyManager>>,
}

/// Shared, `Arc`-owned state for [`ThreadManager`]. This `Arc` is required to have a single
/// `Arc` reference that can be downgraded to by `AgentControl` while preventing every single
/// function to require an `Arc<&Self>`.
pub(crate) struct ThreadManagerState {
    threads: Arc<RwLock<HashMap<ThreadId, Arc<VACThread>>>>,
    thread_created_tx: broadcast::Sender<ThreadId>,
    auth_manager: Arc<AuthManager>,
    models_manager: SharedModelsManager,
    environment_manager: Arc<EnvironmentManager>,
    skills_manager: Arc<SkillsManager>,
    plugins_manager: Arc<PluginsManager>,
    mcp_manager: Arc<McpManager>,
    skills_watcher: Arc<SkillsWatcher>,
    thread_store: Arc<dyn ThreadStore>,
    session_source: SessionSource,
    analytics_events_client: Option<AnalyticsEventsClient>,
    // Captures submitted ops for testing purpose when test mode is enabled.
    ops_log: Option<SharedCapturedOps>,
}

pub fn build_models_manager(
    config: &Config,
    auth_manager: Arc<AuthManager>,
) -> SharedModelsManager {
    let provider = create_model_provider(config.model_provider.clone(), Some(auth_manager));
    provider.models_manager(config.vac_home.to_path_buf(), config.model_catalog.clone())
}

pub fn thread_store_from_config(config: &Config) -> Arc<dyn ThreadStore> {
    match &config.experimental_thread_store {
        ThreadStoreConfig::Local => Arc::new(LocalThreadStore::new(
            LocalThreadStoreConfig::from_config(config),
        )),
        ThreadStoreConfig::Remote { endpoint: _ } => Arc::new(RemoteThreadStore::unavailable()),
        ThreadStoreConfig::InMemory { id } => InMemoryThreadStore::for_id(id),
    }
}

impl ThreadManager {
    pub fn new(
        config: &Config,
        auth_manager: Arc<AuthManager>,
        session_source: SessionSource,
        environment_manager: Arc<EnvironmentManager>,
        analytics_events_client: Option<AnalyticsEventsClient>,
        thread_store: Arc<dyn ThreadStore>,
    ) -> Self {
        let vac_home = config.vac_home.clone();
        let restriction_product = session_source.restriction_product();
        let (thread_created_tx, _) = broadcast::channel(THREAD_CREATED_CHANNEL_CAPACITY);
        let plugins_manager = Arc::new(PluginsManager::new_with_restriction_product(
            vac_home.to_path_buf(),
            restriction_product,
        ));
        let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
        let skills_manager = Arc::new(SkillsManager::new_with_restriction_product(
            vac_home,
            config.bundled_skills_enabled(),
            restriction_product,
        ));
        let skills_watcher = build_skills_watcher(Arc::clone(&skills_manager));
        Self {
            state: Arc::new(ThreadManagerState {
                threads: Arc::new(RwLock::new(HashMap::new())),
                thread_created_tx,
                models_manager: build_models_manager(config, auth_manager.clone()),
                environment_manager,
                skills_manager,
                plugins_manager,
                mcp_manager,
                skills_watcher,
                thread_store,
                auth_manager,
                session_source,
                analytics_events_client,
                ops_log: should_use_test_thread_manager_behavior()
                    .then(|| Arc::new(std::sync::Mutex::new(Vec::new()))),
            }),
            _test_vac_home_guard: None,
        }
    }

    /// Construct with a dummy AuthManager containing the provided VACAuth.
    /// Used for integration tests: should not be used by ordinary business logic.
    pub(crate) fn with_models_provider_for_tests(
        auth: VACAuth,
        provider: ModelProviderInfo,
    ) -> Self {
        set_thread_manager_test_mode_for_tests(/*enabled*/ true);
        let vac_home =
            std::env::temp_dir().join(format!("vac-thread-manager-test-{}", uuid::Uuid::new_v4()));
        if let Err(err) = std::fs::create_dir_all(&vac_home) {
            tracing::warn!("failed to create temp vac home dir for test thread manager: {err}");
        }
        let mut manager = Self::with_models_provider_and_home_for_tests(
            auth,
            provider,
            vac_home.clone(),
            Arc::new(EnvironmentManager::default_for_tests()),
        );
        manager._test_vac_home_guard = Some(TempVACHomeGuard { path: vac_home });
        manager
    }

    /// Construct with a dummy AuthManager containing the provided VACAuth and vac home.
    /// Used for integration tests: should not be used by ordinary business logic.
    pub(crate) fn with_models_provider_and_home_for_tests(
        auth: VACAuth,
        provider: ModelProviderInfo,
        vac_home: PathBuf,
        environment_manager: Arc<EnvironmentManager>,
    ) -> Self {
        set_thread_manager_test_mode_for_tests(/*enabled*/ true);
        let auth_manager = AuthManager::from_auth_for_testing(auth);
        let skills_vac_home = match AbsolutePathBuf::from_absolute_path_checked(&vac_home) {
            Ok(vac_home) => vac_home,
            Err(err) => {
                tracing::warn!(
                    "test vac_home is not absolute; resolving it against process temp dir: {err}"
                );
                AbsolutePathBuf::resolve_path_against_base(&vac_home, std::env::temp_dir())
            }
        };
        let (thread_created_tx, _) = broadcast::channel(THREAD_CREATED_CHANNEL_CAPACITY);
        let restriction_product = SessionSource::Exec.restriction_product();
        let plugins_manager = Arc::new(PluginsManager::new_with_restriction_product(
            vac_home.clone(),
            restriction_product,
        ));
        let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
        let skills_manager = Arc::new(SkillsManager::new_with_restriction_product(
            skills_vac_home,
            /*bundled_skills_enabled*/ true,
            restriction_product,
        ));
        let skills_watcher = build_skills_watcher(Arc::clone(&skills_manager));
        // This test constructor has no Config input. Tests that need a non-local
        // process store should construct ThreadManager::new with an explicit store.
        let thread_store: Arc<dyn ThreadStore> =
            Arc::new(LocalThreadStore::new(LocalThreadStoreConfig {
                vac_home: vac_home.clone(),
                sqlite_home: vac_home.clone(),
                default_model_provider_id: VASTAR_PROVIDER_ID.to_string(),
            }));
        Self {
            state: Arc::new(ThreadManagerState {
                threads: Arc::new(RwLock::new(HashMap::new())),
                thread_created_tx,
                models_manager: create_model_provider(provider, Some(auth_manager.clone()))
                    .models_manager(vac_home, /*config_model_catalog*/ None),
                environment_manager,
                skills_manager,
                plugins_manager,
                mcp_manager,
                skills_watcher,
                thread_store,
                auth_manager,
                session_source: SessionSource::Exec,
                analytics_events_client: None,
                ops_log: should_use_test_thread_manager_behavior()
                    .then(|| Arc::new(std::sync::Mutex::new(Vec::new()))),
            }),
            _test_vac_home_guard: None,
        }
    }

    pub fn session_source(&self) -> SessionSource {
        self.state.session_source.clone()
    }

    pub fn auth_manager(&self) -> Arc<AuthManager> {
        self.state.auth_manager.clone()
    }

    pub fn skills_manager(&self) -> Arc<SkillsManager> {
        self.state.skills_manager.clone()
    }

    pub fn plugins_manager(&self) -> Arc<PluginsManager> {
        self.state.plugins_manager.clone()
    }

    pub fn mcp_manager(&self) -> Arc<McpManager> {
        self.state.mcp_manager.clone()
    }

    pub fn environment_manager(&self) -> Arc<EnvironmentManager> {
        self.state.environment_manager.clone()
    }

    pub fn default_environment_selections(
        &self,
        cwd: &AbsolutePathBuf,
    ) -> Vec<TurnEnvironmentSelection> {
        default_thread_environment_selections(self.state.environment_manager.as_ref(), cwd)
    }

    pub fn validate_environment_selections(
        &self,
        environments: &[TurnEnvironmentSelection],
    ) -> VACResult<()> {
        resolve_environment_selections(self.state.environment_manager.as_ref(), environments)
            .map(|_| ())
    }

    pub fn get_models_manager(&self) -> SharedModelsManager {
        self.state.models_manager.clone()
    }

    pub async fn list_models(&self, refresh_strategy: RefreshStrategy) -> Vec<ModelPreset> {
        self.state
            .models_manager
            .list_models(refresh_strategy)
            .await
    }

    pub fn list_collaboration_modes(&self) -> Vec<CollaborationModeMask> {
        self.state.models_manager.list_collaboration_modes()
    }

    pub async fn list_thread_ids(&self) -> Vec<ThreadId> {
        self.state.list_thread_ids().await
    }

    pub async fn refresh_mcp_servers(&self, refresh_config: McpServerRefreshConfig) {
        let threads = self
            .state
            .threads
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for thread in threads {
            if let Err(err) = thread
                .submit(Op::RefreshMcpServers {
                    config: refresh_config.clone(),
                })
                .await
            {
                warn!("failed to request MCP server refresh: {err}");
            }
        }
    }

    pub fn subscribe_thread_created(&self) -> broadcast::Receiver<ThreadId> {
        self.state.thread_created_tx.subscribe()
    }

    pub async fn get_thread(&self, thread_id: ThreadId) -> VACResult<Arc<VACThread>> {
        self.state.get_thread(thread_id).await
    }

    /// List `thread_id` plus all known descendants in its spawn subtree.
    pub async fn list_agent_subtree_thread_ids(
        &self,
        thread_id: ThreadId,
    ) -> VACResult<Vec<ThreadId>> {
        let thread = self.state.get_thread(thread_id).await?;

        let mut subtree_thread_ids = Vec::new();
        let mut seen_thread_ids = HashSet::new();
        subtree_thread_ids.push(thread_id);
        seen_thread_ids.insert(thread_id);

        if let Some(state_db_ctx) = thread.state_db() {
            for status in [
                DirectionalThreadSpawnEdgeStatus::Open,
                DirectionalThreadSpawnEdgeStatus::Closed,
            ] {
                for descendant_id in state_db_ctx
                    .list_thread_spawn_descendants_with_status(thread_id, status)
                    .await
                    .map_err(|err| {
                        VACErr::Fatal(format!("failed to load thread-spawn descendants: {err}"))
                    })?
                {
                    if seen_thread_ids.insert(descendant_id) {
                        subtree_thread_ids.push(descendant_id);
                    }
                }
            }
        }

        for descendant_id in thread
            .vac
            .session
            .services
            .agent_control
            .list_live_agent_subtree_thread_ids(thread_id)
            .await?
        {
            if seen_thread_ids.insert(descendant_id) {
                subtree_thread_ids.push(descendant_id);
            }
        }

        Ok(subtree_thread_ids)
    }

    pub async fn start_thread(&self, config: Config) -> VACResult<NewThread> {
        // Box delegated thread-spawn futures so these convenience wrappers do
        // not inline the full spawn path into every caller's async state.
        Box::pin(self.start_thread_with_tools(
            config,
            Vec::new(),
            /*persist_extended_history*/ false,
        ))
        .await
    }

    pub async fn start_thread_with_tools(
        &self,
        config: Config,
        dynamic_tools: Vec<vac_protocol::dynamic_tools::DynamicToolSpec>,
        persist_extended_history: bool,
    ) -> VACResult<NewThread> {
        let environments = default_thread_environment_selections(
            self.state.environment_manager.as_ref(),
            &config.cwd,
        );
        Box::pin(self.start_thread_with_options(StartThreadOptions {
            config,
            initial_history: InitialHistory::New,
            session_source: None,
            dynamic_tools,
            persist_extended_history,
            metrics_service_name: None,
            parent_trace: None,
            environments,
        }))
        .await
    }

    pub async fn start_thread_with_options(
        &self,
        options: StartThreadOptions,
    ) -> VACResult<NewThread> {
        let session_source = options
            .session_source
            .unwrap_or_else(|| self.state.session_source.clone());
        Box::pin(self.state.spawn_thread_with_source(
            options.config,
            options.initial_history,
            Arc::clone(&self.state.auth_manager),
            self.agent_control(),
            session_source,
            options.dynamic_tools,
            options.persist_extended_history,
            options.metrics_service_name,
            /*inherited_shell_snapshot*/ None,
            /*inherited_exec_policy*/ None,
            options.parent_trace,
            options.environments,
            /*user_shell_override*/ None,
        ))
        .await
    }

    pub async fn resume_thread_from_rollout(
        &self,
        config: Config,
        rollout_path: PathBuf,
        auth_manager: Arc<AuthManager>,
        parent_trace: Option<W3cTraceContext>,
    ) -> VACResult<NewThread> {
        let initial_history = RolloutRecorder::get_rollout_history(&rollout_path).await?;
        Box::pin(self.resume_thread_with_history(
            config,
            initial_history,
            auth_manager,
            /*persist_extended_history*/ false,
            parent_trace,
        ))
        .await
    }

    pub async fn resume_thread_with_history(
        &self,
        config: Config,
        initial_history: InitialHistory,
        auth_manager: Arc<AuthManager>,
        persist_extended_history: bool,
        parent_trace: Option<W3cTraceContext>,
    ) -> VACResult<NewThread> {
        let environments = default_thread_environment_selections(
            self.state.environment_manager.as_ref(),
            &config.cwd,
        );
        Box::pin(self.state.spawn_thread(
            config,
            initial_history,
            auth_manager,
            self.agent_control(),
            Vec::new(),
            persist_extended_history,
            /*metrics_service_name*/ None,
            parent_trace,
            environments,
            /*user_shell_override*/ None,
        ))
        .await
    }

    pub(crate) async fn start_thread_with_user_shell_override_for_tests(
        &self,
        config: Config,
        user_shell_override: crate::shell::Shell,
    ) -> VACResult<NewThread> {
        let environments = default_thread_environment_selections(
            self.state.environment_manager.as_ref(),
            &config.cwd,
        );
        Box::pin(self.state.spawn_thread(
            config,
            InitialHistory::New,
            Arc::clone(&self.state.auth_manager),
            self.agent_control(),
            Vec::new(),
            /*persist_extended_history*/ false,
            /*metrics_service_name*/ None,
            /*parent_trace*/ None,
            environments,
            /*user_shell_override*/ Some(user_shell_override),
        ))
        .await
    }

    pub(crate) async fn resume_thread_from_rollout_with_user_shell_override_for_tests(
        &self,
        config: Config,
        rollout_path: PathBuf,
        auth_manager: Arc<AuthManager>,
        user_shell_override: crate::shell::Shell,
    ) -> VACResult<NewThread> {
        let initial_history = RolloutRecorder::get_rollout_history(&rollout_path).await?;
        let environments = default_thread_environment_selections(
            self.state.environment_manager.as_ref(),
            &config.cwd,
        );
        Box::pin(self.state.spawn_thread(
            config,
            initial_history,
            auth_manager,
            self.agent_control(),
            Vec::new(),
            /*persist_extended_history*/ false,
            /*metrics_service_name*/ None,
            /*parent_trace*/ None,
            environments,
            /*user_shell_override*/ Some(user_shell_override),
        ))
        .await
    }

    /// Removes the thread from the manager's internal map, though the thread is stored
    /// as `Arc<VACThread>`, it is possible that other references to it exist elsewhere.
    /// Returns the thread if the thread was found and removed.
    pub async fn remove_thread(&self, thread_id: &ThreadId) -> Option<Arc<VACThread>> {
        self.state.threads.write().await.remove(thread_id)
    }

    /// Tries to shut down all tracked threads concurrently within the provided timeout.
    /// Threads that complete shutdown are removed from the manager; incomplete shutdowns
    /// remain tracked so callers can retry or inspect them later.
    pub async fn shutdown_all_threads_bounded(&self, timeout: Duration) -> ThreadShutdownReport {
        let threads = {
            let threads = self.state.threads.read().await;
            threads
                .iter()
                .map(|(thread_id, thread)| (*thread_id, Arc::clone(thread)))
                .collect::<Vec<_>>()
        };

        let mut shutdowns = threads
            .into_iter()
            .map(|(thread_id, thread)| async move {
                let outcome = match tokio::time::timeout(timeout, thread.shutdown_and_wait()).await
                {
                    Ok(Ok(())) => ShutdownOutcome::Complete,
                    Ok(Err(_)) => ShutdownOutcome::SubmitFailed,
                    Err(_) => ShutdownOutcome::TimedOut,
                };
                (thread_id, outcome)
            })
            .collect::<FuturesUnordered<_>>();
        let mut report = ThreadShutdownReport::default();

        while let Some((thread_id, outcome)) = shutdowns.next().await {
            match outcome {
                ShutdownOutcome::Complete => report.completed.push(thread_id),
                ShutdownOutcome::SubmitFailed => report.submit_failed.push(thread_id),
                ShutdownOutcome::TimedOut => report.timed_out.push(thread_id),
            }
        }

        let mut tracked_threads = self.state.threads.write().await;
        for thread_id in &report.completed {
            tracked_threads.remove(thread_id);
        }

        report
            .completed
            .sort_by_key(std::string::ToString::to_string);
        report
            .submit_failed
            .sort_by_key(std::string::ToString::to_string);
        report
            .timed_out
            .sort_by_key(std::string::ToString::to_string);
        report
    }

    /// Branch an existing thread by snapshotting rollout history according to
    /// `snapshot` and starting a new thread with identical configuration
    /// (unless overridden by the caller's `config`). The new thread will have
    /// a fresh id.
    pub async fn branch_thread<S>(
        &self,
        snapshot: S,
        config: Config,
        path: PathBuf,
        persist_extended_history: bool,
        parent_trace: Option<W3cTraceContext>,
    ) -> VACResult<NewThread>
    where
        S: Into<BranchSnapshot>,
    {
        let snapshot = snapshot.into();
        let history = RolloutRecorder::get_rollout_history(&path).await?;
        self.branch_thread_from_history(
            snapshot,
            config,
            history,
            persist_extended_history,
            parent_trace,
        )
        .await
    }

    /// Branch an existing thread from already-loaded store history.
    pub async fn branch_thread_from_history<S>(
        &self,
        snapshot: S,
        config: Config,
        history: InitialHistory,
        persist_extended_history: bool,
        parent_trace: Option<W3cTraceContext>,
    ) -> VACResult<NewThread>
    where
        S: Into<BranchSnapshot>,
    {
        self.branch_thread_with_initial_history(
            snapshot.into(),
            config,
            history,
            persist_extended_history,
            parent_trace,
        )
        .await
    }

    async fn branch_thread_with_initial_history(
        &self,
        snapshot: BranchSnapshot,
        config: Config,
        history: InitialHistory,
        persist_extended_history: bool,
        parent_trace: Option<W3cTraceContext>,
    ) -> VACResult<NewThread> {
        let interrupted_marker = InterruptedTurnHistoryMarker::from_config(&config);
        let history = branch_history_from_snapshot(snapshot, history, interrupted_marker);
        let environments = default_thread_environment_selections(
            self.state.environment_manager.as_ref(),
            &config.cwd,
        );
        Box::pin(self.state.spawn_thread(
            config,
            history,
            Arc::clone(&self.state.auth_manager),
            self.agent_control(),
            Vec::new(),
            persist_extended_history,
            /*metrics_service_name*/ None,
            parent_trace,
            environments,
            /*user_shell_override*/ None,
        ))
        .await
    }

    pub(crate) fn agent_control(&self) -> AgentControl {
        AgentControl::new(Arc::downgrade(&self.state))
    }

    #[cfg(test)]
    pub(crate) fn captured_ops(&self) -> Vec<(ThreadId, Op)> {
        self.state
            .ops_log
            .as_ref()
            .and_then(|ops_log| ops_log.lock().ok().map(|log| log.clone()))
            .unwrap_or_default()
    }
}

impl ThreadManagerState {
    pub(crate) async fn list_thread_ids(&self) -> Vec<ThreadId> {
        self.threads
            .read()
            .await
            .iter()
            .filter_map(|(thread_id, thread)| {
                (!thread.session_source.is_internal()).then_some(*thread_id)
            })
            .collect()
    }

    /// Fetch a thread by ID or return ThreadNotFound.
    pub(crate) async fn get_thread(&self, thread_id: ThreadId) -> VACResult<Arc<VACThread>> {
        let threads = self.threads.read().await;
        match threads.get(&thread_id) {
            Some(thread) if !thread.session_source.is_internal() => Ok(thread.clone()),
            Some(_) | None => Err(VACErr::ThreadNotFound(thread_id)),
        }
    }

    pub(crate) async fn read_stored_thread(
        &self,
        params: ReadThreadParams,
    ) -> VACResult<StoredThread> {
        let thread_id = params.thread_id;
        self.thread_store
            .read_thread(params)
            .await
            .map_err(|err| match err {
                ThreadStoreError::ThreadNotFound { thread_id } => {
                    VACErr::ThreadNotFound(thread_id)
                }
                ThreadStoreError::InvalidRequest { message } => {
                    if message.starts_with("no rollout found for thread id ") {
                        VACErr::ThreadNotFound(thread_id)
                    } else {
                        VACErr::Fatal(format!(
                            "failed to read stored thread {thread_id}: invalid thread-store request: {message}"
                        ))
                    }
                }
                err => VACErr::Fatal(format!("failed to read stored thread {thread_id}: {err}")),
            })
    }

    /// Send an operation to a thread by ID.
    pub(crate) async fn send_op(&self, thread_id: ThreadId, op: Op) -> VACResult<String> {
        let thread = self.get_thread(thread_id).await?;
        if let Some(ops_log) = &self.ops_log
            && let Ok(mut log) = ops_log.lock()
        {
            log.push((thread_id, op.clone()));
        }
        thread.submit(op).await
    }

    #[cfg(test)]
    /// Append a prebuilt message to a thread by ID outside the normal user-input path.
    pub(crate) async fn append_message(
        &self,
        thread_id: ThreadId,
        message: ResponseItem,
    ) -> VACResult<String> {
        let thread = self.get_thread(thread_id).await?;
        thread.append_message(message).await
    }

    /// Remove a thread from the manager by ID, returning it when present.
    pub(crate) async fn remove_thread(&self, thread_id: &ThreadId) -> Option<Arc<VACThread>> {
        self.threads.write().await.remove(thread_id)
    }

    /// Spawn a new thread with no history using a provided config.
    pub(crate) async fn spawn_new_thread(
        &self,
        config: Config,
        agent_control: AgentControl,
    ) -> VACResult<NewThread> {
        Box::pin(self.spawn_new_thread_with_source(
            config,
            agent_control,
            self.session_source.clone(),
            /*persist_extended_history*/ false,
            /*metrics_service_name*/ None,
            /*inherited_shell_snapshot*/ None,
            /*inherited_exec_policy*/ None,
            /*environments*/ None,
        ))
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn spawn_new_thread_with_source(
        &self,
        config: Config,
        agent_control: AgentControl,
        session_source: SessionSource,
        persist_extended_history: bool,
        metrics_service_name: Option<String>,
        inherited_shell_snapshot: Option<Arc<ShellSnapshot>>,
        inherited_exec_policy: Option<Arc<crate::exec_policy::ExecPolicyManager>>,
        environments: Option<Vec<TurnEnvironmentSelection>>,
    ) -> VACResult<NewThread> {
        let environments = environments.unwrap_or_else(|| {
            default_thread_environment_selections(self.environment_manager.as_ref(), &config.cwd)
        });
        Box::pin(self.spawn_thread_with_source(
            config,
            InitialHistory::New,
            Arc::clone(&self.auth_manager),
            agent_control,
            session_source,
            Vec::new(),
            persist_extended_history,
            metrics_service_name,
            inherited_shell_snapshot,
            inherited_exec_policy,
            /*parent_trace*/ None,
            environments,
            /*user_shell_override*/ None,
        ))
        .await
    }

    pub(crate) async fn resume_thread_with_history_with_source(
        &self,
        options: ResumeThreadWithHistoryOptions,
    ) -> VACResult<NewThread> {
        let ResumeThreadWithHistoryOptions {
            config,
            initial_history,
            agent_control,
            session_source,
            inherited_shell_snapshot,
            inherited_exec_policy,
        } = options;
        let environments =
            default_thread_environment_selections(self.environment_manager.as_ref(), &config.cwd);
        Box::pin(self.spawn_thread_with_source(
            config,
            initial_history,
            Arc::clone(&self.auth_manager),
            agent_control,
            session_source,
            Vec::new(),
            /*persist_extended_history*/ false,
            /*metrics_service_name*/ None,
            inherited_shell_snapshot,
            inherited_exec_policy,
            /*parent_trace*/ None,
            environments,
            /*user_shell_override*/ None,
        ))
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn branch_thread_with_source(
        &self,
        config: Config,
        initial_history: InitialHistory,
        agent_control: AgentControl,
        session_source: SessionSource,
        persist_extended_history: bool,
        inherited_shell_snapshot: Option<Arc<ShellSnapshot>>,
        inherited_exec_policy: Option<Arc<crate::exec_policy::ExecPolicyManager>>,
        environments: Option<Vec<TurnEnvironmentSelection>>,
    ) -> VACResult<NewThread> {
        let environments = environments.unwrap_or_else(|| {
            default_thread_environment_selections(self.environment_manager.as_ref(), &config.cwd)
        });
        Box::pin(self.spawn_thread_with_source(
            config,
            initial_history,
            Arc::clone(&self.auth_manager),
            agent_control,
            session_source,
            Vec::new(),
            persist_extended_history,
            /*metrics_service_name*/ None,
            inherited_shell_snapshot,
            inherited_exec_policy,
            /*parent_trace*/ None,
            environments,
            /*user_shell_override*/ None,
        ))
        .await
    }

    /// Spawn a new thread with optional history and register it with the manager.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn spawn_thread(
        &self,
        config: Config,
        initial_history: InitialHistory,
        auth_manager: Arc<AuthManager>,
        agent_control: AgentControl,
        dynamic_tools: Vec<vac_protocol::dynamic_tools::DynamicToolSpec>,
        persist_extended_history: bool,
        metrics_service_name: Option<String>,
        parent_trace: Option<W3cTraceContext>,
        environments: Vec<TurnEnvironmentSelection>,
        user_shell_override: Option<crate::shell::Shell>,
    ) -> VACResult<NewThread> {
        Box::pin(self.spawn_thread_with_source(
            config,
            initial_history,
            auth_manager,
            agent_control,
            self.session_source.clone(),
            dynamic_tools,
            persist_extended_history,
            metrics_service_name,
            /*inherited_shell_snapshot*/ None,
            /*inherited_exec_policy*/ None,
            parent_trace,
            environments,
            user_shell_override,
        ))
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn spawn_thread_with_source(
        &self,
        config: Config,
        initial_history: InitialHistory,
        auth_manager: Arc<AuthManager>,
        agent_control: AgentControl,
        session_source: SessionSource,
        dynamic_tools: Vec<vac_protocol::dynamic_tools::DynamicToolSpec>,
        persist_extended_history: bool,
        metrics_service_name: Option<String>,
        inherited_shell_snapshot: Option<Arc<ShellSnapshot>>,
        inherited_exec_policy: Option<Arc<crate::exec_policy::ExecPolicyManager>>,
        parent_trace: Option<W3cTraceContext>,
        environments: Vec<TurnEnvironmentSelection>,
        user_shell_override: Option<crate::shell::Shell>,
    ) -> VACResult<NewThread> {
        let is_resumed_thread = matches!(&initial_history, InitialHistory::Resumed(_));
        if let InitialHistory::Resumed(resumed) = &initial_history {
            let mut threads = self.threads.write().await;
            if let Some(thread) = threads.get(&resumed.conversation_id).cloned() {
                if thread.is_running() {
                    if let Some(requested_rollout_path) = resumed.rollout_path.as_deref()
                        && thread.rollout_path().as_deref() != Some(requested_rollout_path)
                    {
                        return Err(VACErr::InvalidRequest(format!(
                            "thread {} is already running with a different rollout path",
                            resumed.conversation_id
                        )));
                    }
                    return Ok(NewThread {
                        thread_id: resumed.conversation_id,
                        session_configured: thread.session_configured(),
                        thread,
                    });
                }
                threads.remove(&resumed.conversation_id);
            }
        }
        let environment_selections =
            resolve_environment_selections(self.environment_manager.as_ref(), &environments)?;
        let watch_registration = match environment_selections.primary_turn_environment() {
            Some(turn_environment) if !turn_environment.environment.is_remote() => {
                self.skills_watcher
                    .register_config(
                        &config,
                        self.skills_manager.as_ref(),
                        self.plugins_manager.as_ref(),
                        Some(turn_environment.environment.get_filesystem()),
                    )
                    .await
            }
            Some(_) | None => crate::file_watcher::WatchRegistration::default(),
        };
        let parent_rollout_thread_trace = self
            .parent_rollout_thread_trace_for_source(&session_source, &initial_history)
            .await;
        let tracked_session_source = session_source.clone();
        let VACSpawnOk { vac, thread_id, .. } = VAC::spawn(VACSpawnArgs {
            config,
            auth_manager,
            models_manager: Arc::clone(&self.models_manager),
            environment_manager: Arc::clone(&self.environment_manager),
            skills_manager: Arc::clone(&self.skills_manager),
            plugins_manager: Arc::clone(&self.plugins_manager),
            mcp_manager: Arc::clone(&self.mcp_manager),
            skills_watcher: Arc::clone(&self.skills_watcher),
            conversation_history: initial_history,
            session_source,
            agent_control,
            dynamic_tools,
            persist_extended_history,
            metrics_service_name,
            inherited_shell_snapshot,
            inherited_exec_policy,
            parent_rollout_thread_trace,
            user_shell_override,
            parent_trace,
            environment_selections,
            analytics_events_client: self.analytics_events_client.clone(),
            thread_store: Arc::clone(&self.thread_store),
        })
        .await?;
        let new_thread = self
            .finalize_thread_spawn(vac, thread_id, tracked_session_source, watch_registration)
            .await?;
        if is_resumed_thread
            && let Err(err) = new_thread.thread.apply_goal_resume_runtime_effects().await
        {
            warn!("failed to apply goal resume runtime effects: {err}");
        }
        Ok(new_thread)
    }

    async fn finalize_thread_spawn(
        &self,
        vac: VAC,
        thread_id: ThreadId,
        session_source: SessionSource,
        watch_registration: crate::file_watcher::WatchRegistration,
    ) -> VACResult<NewThread> {
        let event = vac.next_event().await?;
        let session_configured = match event {
            Event {
                id,
                msg: EventMsg::SessionConfigured(session_configured),
            } if id == INITIAL_SUBMIT_ID => session_configured,
            _ => {
                return Err(VACErr::SessionConfiguredNotFirstEvent);
            }
        };

        {
            let mut threads = self.threads.write().await;
            if let std::collections::hash_map::Entry::Vacant(e) = threads.entry(thread_id) {
                let thread = Arc::new(VACThread::new(
                    vac,
                    session_configured.clone(),
                    session_configured.rollout_path.clone(),
                    session_source,
                    watch_registration,
                ));
                e.insert(thread.clone());
                return Ok(NewThread {
                    thread_id,
                    thread,
                    session_configured,
                });
            }
        }

        if let Err(err) = vac.shutdown_and_wait().await {
            warn!("failed to shut down duplicate thread {thread_id}: {err}");
        }
        Err(VACErr::InvalidRequest(format!(
            "thread {thread_id} is already running"
        )))
    }

    pub(crate) fn notify_thread_created(&self, thread_id: ThreadId) {
        let _ = self.thread_created_tx.send(thread_id);
    }

    async fn parent_rollout_thread_trace_for_source(
        &self,
        session_source: &SessionSource,
        initial_history: &InitialHistory,
    ) -> vac_rollout_trace::ThreadTraceContext {
        // A fresh v2 child belongs to the same rollout tree as its parent, so
        // session startup derives its child trace from the parent's thread
        // context. Resumed children already have a prior `ThreadStarted` event
        // for this thread id; deriving a child trace during resume would write
        // that start event again and make the bundle unreplayable.
        let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) = session_source
        else {
            return vac_rollout_trace::ThreadTraceContext::disabled();
        };
        if matches!(initial_history, InitialHistory::Resumed(_)) {
            return vac_rollout_trace::ThreadTraceContext::disabled();
        }
        // Parent lookup can fail if the parent was closed or released between
        // spawn preparation and session construction. Tracing is diagnostic, so
        // that race should not block child creation; the child simply starts
        // without a parent rollout trace.
        self.get_thread(*parent_thread_id)
            .await
            .ok()
            .map(|thread| thread.vac.session.services.rollout_thread_trace.clone())
            .unwrap_or_else(vac_rollout_trace::ThreadTraceContext::disabled)
    }
}

/// Return a branch snapshot cut strictly before the nth user message (0-based).
///
/// Out-of-range values keep the full committed history at a turn boundary, but
/// when the source thread is currently mid-turn they fall back to cutting
/// before the active turn's opening boundary so the branch omits the unfinished
/// suffix entirely.
fn truncate_before_nth_user_message(
    history: InitialHistory,
    n: usize,
    snapshot_state: &SnapshotTurnState,
) -> InitialHistory {
    let items: Vec<RolloutItem> = history.get_rollout_items();
    let user_positions = truncation::user_message_positions_in_rollout(&items);
    let rolled = if snapshot_state.ends_mid_turn && n >= user_positions.len() {
        if let Some(cut_idx) = snapshot_state
            .active_turn_start_index
            .or_else(|| user_positions.last().copied())
        {
            items[..cut_idx].to_vec()
        } else {
            items
        }
    } else {
        truncation::truncate_rollout_before_nth_user_message_from_start(&items, n)
    };

    if rolled.is_empty() {
        InitialHistory::New
    } else {
        InitialHistory::Branched(rolled)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotTurnState {
    ends_mid_turn: bool,
    active_turn_id: Option<String>,
    active_turn_start_index: Option<usize>,
}

fn snapshot_turn_state(history: &InitialHistory) -> SnapshotTurnState {
    let rollout_items = history.get_rollout_items();
    let mut builder = ThreadHistoryBuilder::new();
    for item in &rollout_items {
        builder.handle_rollout_item(item);
    }
    let active_turn_id = builder.active_turn_id_if_explicit();
    if builder.has_active_turn() && active_turn_id.is_some() {
        let active_turn_snapshot = builder.active_turn_snapshot();
        if active_turn_snapshot
            .as_ref()
            .is_some_and(|turn| turn.status != TurnStatus::InProgress)
        {
            return SnapshotTurnState {
                ends_mid_turn: false,
                active_turn_id: None,
                active_turn_start_index: None,
            };
        }

        return SnapshotTurnState {
            ends_mid_turn: true,
            active_turn_id,
            active_turn_start_index: builder.active_turn_start_index(),
        };
    }

    let Some(last_user_position) = truncation::user_message_positions_in_rollout(&rollout_items)
        .last()
        .copied()
    else {
        return SnapshotTurnState {
            ends_mid_turn: false,
            active_turn_id: None,
            active_turn_start_index: None,
        };
    };

    // Synthetic branch/resume histories can contain user/assistant response items
    // without explicit turn lifecycle events. If the persisted snapshot has no
    // terminating boundary after its last user message, treat it as mid-turn.
    SnapshotTurnState {
        ends_mid_turn: !rollout_items[last_user_position + 1..].iter().any(|item| {
            matches!(
                item,
                RolloutItem::EventMsg(EventMsg::TurnComplete(_) | EventMsg::TurnAborted(_))
            )
        }),
        active_turn_id: None,
        active_turn_start_index: None,
    }
}

fn branch_history_from_snapshot(
    snapshot: BranchSnapshot,
    history: InitialHistory,
    interrupted_marker: InterruptedTurnHistoryMarker,
) -> InitialHistory {
    let snapshot_state = snapshot_turn_state(&history);
    match snapshot {
        BranchSnapshot::TruncateBeforeNthUserMessage(nth_user_message) => {
            truncate_before_nth_user_message(history, nth_user_message, &snapshot_state)
        }
        BranchSnapshot::Interrupted => {
            let history = match history {
                InitialHistory::New => InitialHistory::New,
                InitialHistory::Cleared => InitialHistory::Cleared,
                InitialHistory::Branched(history) => InitialHistory::Branched(history),
                InitialHistory::Resumed(resumed) => InitialHistory::Branched(resumed.history),
            };
            if snapshot_state.ends_mid_turn {
                append_interrupted_boundary(
                    history,
                    snapshot_state.active_turn_id,
                    interrupted_marker,
                )
            } else {
                history
            }
        }
    }
}

/// Append the same persisted interrupt boundary used by the live interrupt path
/// to an existing branch snapshot after the source thread has been confirmed to
/// be mid-turn.
fn append_interrupted_boundary(
    history: InitialHistory,
    turn_id: Option<String>,
    interrupted_marker: InterruptedTurnHistoryMarker,
) -> InitialHistory {
    let aborted_event = RolloutItem::EventMsg(EventMsg::TurnAborted(TurnAbortedEvent {
        turn_id,
        reason: TurnAbortReason::Interrupted,
        completed_at: None,
        duration_ms: None,
    }));

    match history {
        InitialHistory::New | InitialHistory::Cleared => {
            let mut history = Vec::new();
            if let Some(marker) = interrupted_turn_history_marker(interrupted_marker) {
                history.push(RolloutItem::ResponseItem(marker));
            }
            history.push(aborted_event);
            InitialHistory::Branched(history)
        }
        InitialHistory::Branched(mut history) => {
            if let Some(marker) = interrupted_turn_history_marker(interrupted_marker) {
                history.push(RolloutItem::ResponseItem(marker));
            }
            history.push(aborted_event);
            InitialHistory::Branched(history)
        }
        InitialHistory::Resumed(mut resumed) => {
            if let Some(marker) = interrupted_turn_history_marker(interrupted_marker) {
                resumed.history.push(RolloutItem::ResponseItem(marker));
            }
            resumed.history.push(aborted_event);
            InitialHistory::Branched(resumed.history)
        }
    }
}

#[cfg(test)]
#[path = "thread_manager_tests.rs"]
mod tests;
