//! Lifecycle owner for VAC's local runtime path.
//!
//! This crate owns the non-app-server startup seams for retained resources,
//! TUI bootstrap data, prompt/control dispatch, owner event projection,
//! server-request resolution, and local compatibility-safe command surfaces on
//! the default product path. App-server compatibility remains quarantine-only
//! outside the default local runtime owner path.

pub mod command_bus;
pub mod event_stream;
pub mod external_agent_config;
pub mod plugin_surface;
pub mod retained_resources;
pub mod server_requests;
pub mod session;
pub mod shutdown;
pub mod startup;

pub use command_bus::RuntimeAccount;
pub use command_bus::RuntimeAccountRead;
pub use command_bus::RuntimeCommandBus;
pub use command_bus::RuntimeCommandBusError;
pub use command_bus::RuntimeExternalAgentConfigDetectCommand;
pub use command_bus::RuntimeExternalAgentConfigImportCommand;
pub use command_bus::RuntimeExternalAgentConfigMigrationDetails;
pub use command_bus::RuntimeExternalAgentConfigMigrationItem;
pub use command_bus::RuntimeExternalAgentConfigMigrationItemType;
pub use command_bus::RuntimeExternalAgentConfigNamedMigration;
pub use command_bus::RuntimeExternalAgentConfigPluginsMigration;
pub use command_bus::RuntimeExternalAgentConfigSessionMigration;
pub use command_bus::RuntimePluginSurfaceCommand;
pub use command_bus::RuntimePluginSurfaceOperation;
pub use command_bus::RuntimeReadCommand;
pub use command_bus::RuntimeReadResponse;
pub use command_bus::RuntimeRealtimeAppendAudioCommand;
pub use command_bus::RuntimeRealtimeStartCommand;
pub use command_bus::RuntimeRealtimeStopCommand;
pub use command_bus::RuntimeReviewStartCommand;
pub use command_bus::RuntimeShellCommand;
pub use command_bus::RuntimeSkillsListCommand;
pub use command_bus::RuntimeSkillsListEntry;
pub use command_bus::RuntimeSkillsListExtraRootsForCwd;
pub use command_bus::RuntimeSkillsListResponse;
pub use command_bus::RuntimeThreadCommand;
pub use command_bus::RuntimeThreadGoalSetCommand;
pub use command_bus::RuntimeTurnStartCommand;
pub use command_bus::RuntimeTurnSteerCommand;
pub use command_bus::RuntimeWriteCommand;
pub use command_bus::RuntimeWriteResponse;
pub use event_stream::OwnerEventClassification;
pub use event_stream::ProtocolEventMapping;
pub use event_stream::ProtocolProjection;
pub use event_stream::RuntimeEventDelivery;
pub use event_stream::RuntimeEventEnvelope;
pub use event_stream::RuntimeEventKind;
pub use event_stream::RuntimeEventStream;
pub use event_stream::RuntimeEventStreamError;
pub use event_stream::RuntimeEventStreamItem;
pub use event_stream::RuntimeEventSubscriber;
pub use event_stream::RuntimeOwnerEventPayload;
pub use event_stream::RuntimeOwnerEventProjector;
pub use event_stream::RuntimeTuiCompatibilityEvent;
pub use event_stream::classify_protocol_event;
pub use event_stream::classify_runtime_event;
pub use external_agent_config::ExternalAgentConfigDetectOptions;
pub use external_agent_config::ExternalAgentConfigService;
pub use plugin_surface::RuntimeMarketplaceInterface;
pub use plugin_surface::RuntimeMarketplaceLoadErrorInfo;
pub use plugin_surface::RuntimePluginAuthPolicy;
pub use plugin_surface::RuntimePluginAvailability;
pub use plugin_surface::RuntimePluginDetail;
pub use plugin_surface::RuntimePluginInstallPolicy;
pub use plugin_surface::RuntimePluginInstallResponse;
pub use plugin_surface::RuntimePluginInterface;
pub use plugin_surface::RuntimePluginListResponse;
pub use plugin_surface::RuntimePluginMarketplaceEntry;
pub use plugin_surface::RuntimePluginReadResponse;
pub use plugin_surface::RuntimePluginSetEnabledResponse;
pub use plugin_surface::RuntimePluginSource;
pub use plugin_surface::RuntimePluginSummary;
pub use plugin_surface::RuntimePluginUninstallResponse;
pub use plugin_surface::RuntimeSkillInterface;
pub use plugin_surface::RuntimeSkillSummary;
pub use retained_resources::RuntimeRetainedResources;
pub use server_requests::McpElicitationAction;
pub use server_requests::McpElicitationDecision;
pub use server_requests::PendingServerRequest;
pub use server_requests::PendingServerRequestKind;
pub use server_requests::PermissionScope;
pub use server_requests::PermissionsDecision;
pub use server_requests::RuntimeRequestDecision;
pub use server_requests::ServerRequestId;
pub use server_requests::ServerRequestRegistry;
pub use server_requests::ServerRequestRegistryError;
pub use server_requests::ServerRequestResolution;
pub use server_requests::UserInputAnswer;
pub use session::LocalRuntimeOwnerSession;
pub use shutdown::RuntimeShutdownHandle;
pub use startup::DEFAULT_PATH_APP_SERVER_FALLBACKS;
pub use startup::LocalRuntimeAccountDisplay;
pub use startup::LocalRuntimeAuthMode;
pub use startup::LocalRuntimeBootstrap;
pub use startup::LocalRuntimeFeedbackAudience;
pub use startup::LocalRuntimeOwner;
pub use startup::OWNER_NATIVE_DEFAULT_SURFACES;
pub use startup::RuntimeStartupError;
pub use startup::RuntimeStartupInput;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use tempfile::TempDir;
    use vac_core::config::Config;
    use vac_exec_server::EnvironmentManager;
    use vac_features::Feature;
    use vac_protocol::ThreadId;
    use vac_protocol::protocol::ConversationStartParams;
    use vac_protocol::protocol::RealtimeOutputModality;
    use vac_protocol::protocol::SessionSource;

    async fn test_config(vac_home: &TempDir) -> Config {
        Config::load_default_with_cli_overrides_for_vac_home(
            vac_home.path().to_path_buf(),
            Vec::new(),
        )
        .await
        .expect("load default config")
    }

    #[test]
    fn skeleton_seams_construct_without_app_server_runtime() {
        let owner = LocalRuntimeOwner::new();
        let command_bus = RuntimeCommandBus::new();
        let events = RuntimeEventStream::new();
        let requests = ServerRequestRegistry::new();
        let shutdown = RuntimeShutdownHandle::new();

        assert_eq!(owner, LocalRuntimeOwner::default());
        assert_eq!(command_bus, RuntimeCommandBus::default());
        assert_eq!(events, RuntimeEventStream::default());
        assert_eq!(requests, ServerRequestRegistry::default());
        assert_eq!(shutdown, RuntimeShutdownHandle::default());
    }

    #[test]
    fn skeleton_reaches_core_semantic_contract_only() {
        let _command_type = std::any::type_name::<vac_core::local_runtime::RuntimeCommand>();
        let _event_type = std::any::type_name::<vac_core::local_runtime::RuntimeEvent>();
    }

    #[test]
    fn manifest_does_not_name_forbidden_app_server_crates() {
        let manifest =
            std::fs::read_to_string(format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR")))
                .expect("crate manifest is readable");

        for forbidden in [
            "vac-app-server",
            "vac-app-server-client",
            "vac-app-server-protocol",
            "vac-app-server-transport",
        ] {
            assert!(
                !manifest.contains(forbidden),
                "local runtime owner must not depend on {forbidden}"
            );
        }
    }

    #[test]
    fn default_path_has_no_app_server_fallbacks() {
        assert!(DEFAULT_PATH_APP_SERVER_FALLBACKS.is_empty());
        for surface in [
            "prompt submit / turn execution",
            "typed TUI request dispatch",
            "server-request resolve/reject registry",
        ] {
            assert!(
                OWNER_NATIVE_DEFAULT_SURFACES.contains(&surface),
                "{surface} must be recorded as owner-native default-path coverage"
            );
        }
    }

    #[tokio::test]
    async fn retained_resource_builder_preserves_manager_handles() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let input = RuntimeStartupInput::new(
            Arc::clone(&config),
            Arc::clone(&environment_manager),
            SessionSource::Cli,
            false,
        );

        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(input)
            .await;

        assert!(Arc::ptr_eq(
            &retained.environment_manager(),
            &environment_manager
        ));
        assert!(!retained.auth_manager().vac_api_key_env_enabled());
        assert!(Arc::ptr_eq(&retained.config(), &config));
        assert_eq!(
            retained.thread_manager().session_source(),
            SessionSource::Cli
        );
        assert!(retained.thread_manager().list_thread_ids().await.is_empty());
        assert_eq!(
            retained.environment_manager().default_environment_id(),
            Some("local")
        );
        drop(config);
    }

    #[tokio::test]
    async fn command_bus_proves_read_only_account_and_thread_list_shape() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                Arc::clone(&config),
                environment_manager,
                SessionSource::Cli,
                false,
            ))
            .await;
        let command_bus = RuntimeCommandBus::new();

        let account = command_bus
            .execute_read(RuntimeReadCommand::ReadAccount {
                auth_manager: retained.auth_manager(),
                requires_vastar_auth: config.model_provider.requires_vastar_auth,
            })
            .await
            .expect("account read command succeeds");
        assert_eq!(
            account,
            RuntimeReadResponse::Account(RuntimeAccountRead {
                account: None,
                requires_vastar_auth: config.model_provider.requires_vastar_auth,
            })
        );

        let threads = command_bus
            .execute_read(RuntimeReadCommand::ThreadList {
                thread_manager: retained.thread_manager(),
            })
            .await
            .expect("thread list read command succeeds");
        assert_eq!(threads, RuntimeReadResponse::ThreadList(Vec::new()));
    }

    #[tokio::test]
    async fn command_bus_rejects_invalid_review_target_before_thread_lookup() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                config,
                environment_manager,
                SessionSource::Cli,
                false,
            ))
            .await;
        let command_bus = RuntimeCommandBus::new();
        let err = command_bus
            .execute_write(RuntimeWriteCommand::StartReview(Box::new(
                RuntimeReviewStartCommand {
                    thread_manager: retained.thread_manager(),
                    thread_id: ThreadId::new(),
                    target: vac_protocol::protocol::ReviewTarget::Custom {
                        instructions: "   ".to_string(),
                    },
                },
            )))
            .await
            .expect_err("empty custom review instructions should be rejected");

        assert!(matches!(
            err,
            RuntimeCommandBusError::InvalidReviewTarget(_)
        ));
    }

    #[tokio::test]
    async fn command_bus_rejects_invalid_thread_goal_before_thread_lookup() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                config,
                environment_manager,
                SessionSource::Cli,
                false,
            ))
            .await;
        let command_bus = RuntimeCommandBus::new();
        let err = command_bus
            .execute_write(RuntimeWriteCommand::SetThreadGoal(Box::new(
                RuntimeThreadGoalSetCommand {
                    thread_manager: retained.thread_manager(),
                    thread_id: ThreadId::new(),
                    objective: Some("   ".to_string()),
                    status: None,
                    token_budget: None,
                },
            )))
            .await
            .expect_err("empty thread goal objective should be rejected");

        assert!(matches!(
            err,
            RuntimeCommandBusError::InvalidThreadGoalObjective(_)
        ));
    }

    #[tokio::test]
    async fn command_bus_p30b_write_commands_route_through_owner_shape() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                config,
                environment_manager,
                SessionSource::Cli,
                false,
            ))
            .await;
        let thread_manager = retained.thread_manager();
        let thread_id = ThreadId::new();

        let interrupt = RuntimeWriteCommand::InterruptTurn(RuntimeThreadCommand {
            thread_manager: Arc::clone(&thread_manager),
            thread_id,
        });
        let steer = RuntimeWriteCommand::SteerTurn(RuntimeTurnSteerCommand {
            thread_manager: Arc::clone(&thread_manager),
            thread_id,
            turn_id: "turn-p30b".to_string(),
            items: Vec::new(),
        });
        let compact = RuntimeWriteCommand::StartCompact(RuntimeThreadCommand {
            thread_manager: Arc::clone(&thread_manager),
            thread_id,
        });
        let shell = RuntimeWriteCommand::RunShellCommand(RuntimeShellCommand {
            thread_manager,
            thread_id,
            command: "echo p30b".to_string(),
        });

        assert!(matches!(interrupt, RuntimeWriteCommand::InterruptTurn(_)));
        assert!(matches!(steer, RuntimeWriteCommand::SteerTurn(_)));
        assert!(matches!(compact, RuntimeWriteCommand::StartCompact(_)));
        assert!(matches!(shell, RuntimeWriteCommand::RunShellCommand(_)));
    }

    #[tokio::test]
    async fn command_bus_rejects_empty_shell_before_thread_lookup() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                config,
                environment_manager,
                SessionSource::Cli,
                false,
            ))
            .await;

        let err = RuntimeCommandBus::new()
            .execute_write(RuntimeWriteCommand::RunShellCommand(RuntimeShellCommand {
                thread_manager: retained.thread_manager(),
                thread_id: ThreadId::new(),
                command: "   ".to_string(),
            }))
            .await
            .expect_err("blank shell command should fail before lookup");

        assert!(matches!(err, RuntimeCommandBusError::EmptyShellCommand));
    }

    #[tokio::test]
    async fn command_bus_exposes_config_account_memory_write_shapes() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                Arc::clone(&config),
                environment_manager,
                SessionSource::Cli,
                false,
            ))
            .await;
        let command_bus = RuntimeCommandBus::new();

        let reload = command_bus
            .execute_write(RuntimeWriteCommand::ReloadConfig {
                thread_manager: retained.thread_manager(),
            })
            .await
            .expect("reload config command succeeds with no loaded threads");
        assert_eq!(reload, RuntimeWriteResponse::ConfigReloaded);

        let reset = command_bus
            .execute_write(RuntimeWriteCommand::ResetMemory {
                sqlite_home: config.sqlite_home.clone(),
                model_provider_id: config.model_provider_id.clone(),
                vac_home: config.vac_home.clone().to_path_buf(),
            })
            .await
            .expect("memory reset command succeeds against temp state");
        assert_eq!(reset, RuntimeWriteResponse::MemoryReset);

        let logout = command_bus
            .execute_write(RuntimeWriteCommand::LogoutAccount {
                auth_manager: retained.auth_manager(),
            })
            .await
            .expect("logout command succeeds without cached auth");
        assert_eq!(logout, RuntimeWriteResponse::AccountLoggedOut);
    }

    #[tokio::test]
    async fn command_bus_reports_missing_thread_memory_mode_update() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                Arc::clone(&config),
                environment_manager,
                SessionSource::Cli,
                false,
            ))
            .await;
        let command_bus = RuntimeCommandBus::new();
        let missing_thread_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000001").expect("valid thread id");

        let err = command_bus
            .execute_write(RuntimeWriteCommand::SetThreadMemoryMode {
                thread_manager: retained.thread_manager(),
                thread_store: None,
                thread_id: missing_thread_id,
                mode: vac_protocol::protocol::ThreadMemoryMode::Disabled,
            })
            .await
            .expect_err("missing thread without store must not report success");

        assert!(matches!(
            err,
            RuntimeCommandBusError::ThreadMemoryModeSet(_)
        ));
    }

    #[tokio::test]
    async fn command_bus_realtime_start_preserves_feature_gate() {
        let vac_home = TempDir::new().expect("temp vac home");
        let mut config = test_config(&vac_home).await;
        config
            .features
            .disable(Feature::RealtimeConversation)
            .expect("realtime feature should be configurable in tests");
        let config = Arc::new(config);
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                Arc::clone(&config),
                Arc::new(EnvironmentManager::default_for_tests()),
                SessionSource::Cli,
                false,
            ))
            .await;
        let started = retained
            .thread_manager()
            .start_thread_with_options(vac_core::StartThreadOptions {
                config: (*config).clone(),
                initial_history: vac_protocol::protocol::InitialHistory::New,
                session_source: None,
                dynamic_tools: Vec::new(),
                persist_extended_history: false,
                metrics_service_name: None,
                parent_trace: None,
                environments: Vec::new(),
            })
            .await
            .expect("thread should start");
        let command_bus = RuntimeCommandBus::new();

        let err = command_bus
            .execute_write(RuntimeWriteCommand::RealtimeStart(
                RuntimeRealtimeStartCommand {
                    thread_manager: retained.thread_manager(),
                    thread_id: started.thread_id,
                    params: ConversationStartParams {
                        output_modality: RealtimeOutputModality::Audio,
                        prompt: None,
                        realtime_session_id: None,
                        transport: None,
                        voice: None,
                    },
                },
            ))
            .await
            .expect_err("disabled realtime must fail before submitting core op");

        assert_eq!(
            err.to_string(),
            format!(
                "thread {} does not support realtime conversation",
                started.thread_id
            )
        );
    }

    #[tokio::test]
    async fn command_bus_p30g_skills_supports_arbitrary_cwd_without_fallback() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                Arc::clone(&config),
                Arc::new(EnvironmentManager::default_for_tests()),
                SessionSource::Cli,
                false,
            ))
            .await;
        let command_bus = RuntimeCommandBus::new();
        let response = command_bus
            .execute_read(RuntimeReadCommand::ListSkills(Box::new(
                RuntimeSkillsListCommand {
                    thread_manager: retained.thread_manager(),
                    environment_manager: retained.environment_manager(),
                    config: Arc::clone(&config),
                    cwds: vec![config.cwd.join("other").to_path_buf()],
                    force_reload: true,
                    per_cwd_extra_user_roots: None,
                },
            )))
            .await
            .expect("arbitrary cwd should resolve config and list skills through owner bus");

        let RuntimeReadResponse::SkillsList(response) = response else {
            panic!("expected skills/list response");
        };
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].cwd, config.cwd.join("other").to_path_buf());
    }

    #[tokio::test]
    async fn command_bus_p30g_plugin_list_and_external_agent_detect_are_owner_backed() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let retained = LocalRuntimeOwner::new()
            .build_retained_resources(RuntimeStartupInput::new(
                Arc::clone(&config),
                Arc::new(EnvironmentManager::default_for_tests()),
                SessionSource::Cli,
                false,
            ))
            .await;
        let command_bus = RuntimeCommandBus::new();

        let plugin_response = command_bus
            .execute_read(RuntimeReadCommand::PluginSurface(Box::new(
                RuntimePluginSurfaceCommand {
                    operation: RuntimePluginSurfaceOperation::List,
                    thread_manager: Some(retained.thread_manager()),
                    auth_manager: Some(retained.auth_manager()),
                    config: Some(Arc::clone(&config)),
                    cwd: None,
                    plugin_id: None,
                    plugin_name: None,
                    marketplace_path: None,
                    remote_marketplace_name: None,
                    enabled: None,
                },
            )))
            .await
            .expect("plugin list should run through owner provider");
        let RuntimeReadResponse::PluginList(plugin_response) = plugin_response else {
            panic!("expected plugin list response");
        };
        assert!(plugin_response.marketplaces.is_empty());

        let response = command_bus
            .execute_read(RuntimeReadCommand::ExternalAgentConfigDetect(
                RuntimeExternalAgentConfigDetectCommand {
                    include_home: false,
                    vac_home: vac_home.path().to_path_buf(),
                    cwds: None,
                },
            ))
            .await
            .expect("external-agent detect should run through the owner provider");
        let RuntimeReadResponse::ExternalAgentConfigDetect { items } = response else {
            panic!("expected external-agent detect response");
        };
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn command_bus_p30g_external_agent_import_accepts_no_background_work() {
        let vac_home = TempDir::new().expect("temp vac home");
        let response = RuntimeCommandBus::new()
            .execute_write(RuntimeWriteCommand::ExternalAgentConfigImport(
                RuntimeExternalAgentConfigImportCommand {
                    vac_home: vac_home.path().to_path_buf(),
                    migration_items: Vec::new(),
                },
            ))
            .await
            .expect("empty import has no background work");

        assert_eq!(
            response,
            RuntimeWriteResponse::ExternalAgentConfigImportAccepted
        );
    }

    #[tokio::test]
    async fn command_bus_p30g_external_agent_session_import_requires_completion_owner() {
        let vac_home = TempDir::new().expect("temp vac home");
        let err = RuntimeCommandBus::new()
            .execute_write(RuntimeWriteCommand::ExternalAgentConfigImport(
                RuntimeExternalAgentConfigImportCommand {
                    vac_home: vac_home.path().to_path_buf(),
                    migration_items: vec![RuntimeExternalAgentConfigMigrationItem {
                        item_type: RuntimeExternalAgentConfigMigrationItemType::Sessions,
                        description: "session".to_string(),
                        cwd: None,
                        details: Some(RuntimeExternalAgentConfigMigrationDetails {
                            sessions: vec![RuntimeExternalAgentConfigSessionMigration {
                                path: vac_home.path().join("session.jsonl"),
                                cwd: vac_home.path().to_path_buf(),
                                title: None,
                            }],
                            ..Default::default()
                        }),
                    }],
                },
            ))
            .await
            .expect_err("session imports still need a thread-manager-backed owner completion path");

        assert!(matches!(
            err,
            RuntimeCommandBusError::ExternalAgentConfigBackgroundImportRequired
        ));
    }

    #[tokio::test]
    async fn bootstrap_preserves_unauthenticated_account_and_models() {
        let vac_home = TempDir::new().expect("temp vac home");
        let mut config = test_config(&vac_home).await;
        config.model = Some("configured-model-from-plan-27".to_string());
        let config = Arc::new(config);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
        let owner = LocalRuntimeOwner::new();
        let retained = owner
            .build_retained_resources(RuntimeStartupInput::new(
                config,
                environment_manager,
                SessionSource::Cli,
                false,
            ))
            .await;

        let bootstrap = owner
            .build_bootstrap(&retained)
            .await
            .expect("bootstrap should build");

        assert_eq!(bootstrap.account_email, None);
        assert_eq!(bootstrap.auth_mode, None);
        assert_eq!(bootstrap.status_account_display, None);
        assert!(!bootstrap.has_chatgpt_account);
        assert_eq!(
            bootstrap.feedback_audience,
            LocalRuntimeFeedbackAudience::External
        );
        assert!(!bootstrap.available_models.is_empty());
        assert_eq!(bootstrap.default_model, "configured-model-from-plan-27");
    }

    #[tokio::test]
    async fn owner_start_returns_session_with_bootstrap_and_retained_resources() {
        let vac_home = TempDir::new().expect("temp vac home");
        let config = Arc::new(test_config(&vac_home).await);
        let environment_manager = Arc::new(EnvironmentManager::default_for_tests());

        let session = LocalRuntimeOwner::new()
            .start(RuntimeStartupInput::new(
                config,
                Arc::clone(&environment_manager),
                SessionSource::Cli,
                false,
            ))
            .await
            .expect("owner startup should succeed");

        assert!(Arc::ptr_eq(
            &session.retained().environment_manager(),
            &environment_manager
        ));
        assert!(!session.bootstrap().available_models.is_empty());
        assert!(session.events().is_empty());
    }
}
