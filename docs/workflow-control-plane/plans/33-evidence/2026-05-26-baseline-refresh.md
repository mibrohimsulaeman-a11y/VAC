# Plan 33 baseline refresh — 2026-05-26

- Lane: L6 — Plan 33 Evidence Freshening
- Snapshot local time: `2026-05-26T10:37:09+07:00`
- Snapshot UTC: `2026-05-26T03:37:09Z`
- Branch: `main`
- HEAD at capture: `e0aa2153b614d1cebc72423b2281636607855bb8`
- Prior baseline reference: `3e85ccf`
- Scope: docs-only evidence refresh. No Rust, `.vac`, scheduled-audit, or unrelated plan files were edited.

## Status conclusion

Plan 33 remains **BLOCKED**.

- Plan 31 implementation is not complete for this gate: app-server references remain nonzero in the source/manifests snapshot.
- Plan 32 gates are still required before Plan 33 can become final delete/defer proof.
- No app-server delete/defer closure is claimed by this refresh.

## 1. Current `grep -rln vac_app_server vac-rs/` snapshot

Command attempted from repo root:

```sh
timeout 180s grep -rln 'vac_app_server' vac-rs/
```

Result:

- Exit status: `124`
- Captured file-list line count before exit/timeout: `2447`

Note: the literal recursive grep includes `vac-rs/target` build artifacts and timed out. The captured count/list below is therefore a bounded snapshot, not a proof that the full recursive command completed. For human delta classification, this file also records a source/manifests-only fallback that excludes `vac-rs/target`.

Captured literal command file list:

```text
vac-rs/models-manager/src/lib.rs
vac-rs/models-manager/src/manager_tests.rs
vac-rs/core-plugins/src/marketplace.rs
vac-rs/core-plugins/src/manager_tests.rs
vac-rs/core-plugins/src/remote/share/tests.rs
vac-rs/core-plugins/src/remote.rs
vac-rs/app-server/tests/common/lib.rs
vac-rs/app-server/tests/common/mcp_process.rs
vac-rs/app-server/tests/common/auth_fixtures.rs
vac-rs/app-server/tests/suite/fuzzy_file_search.rs
vac-rs/app-server/tests/suite/v2/thread_branch.rs
vac-rs/app-server/tests/suite/v2/model_list.rs
vac-rs/app-server/tests/suite/v2/plan_item.rs
vac-rs/app-server/tests/suite/v2/connection_handling_websocket_unix.rs
vac-rs/app-server/tests/suite/v2/turn_start.rs
vac-rs/app-server/tests/suite/v2/device_key.rs
vac-rs/app-server/tests/suite/v2/app_list.rs
vac-rs/app-server/tests/suite/v2/mcp_tool.rs
vac-rs/app-server/tests/suite/v2/thread_name_websocket.rs
vac-rs/app-server/tests/suite/v2/marketplace_upgrade.rs
vac-rs/app-server/tests/suite/v2/thread_archive.rs
vac-rs/app-server/tests/suite/v2/config_rpc.rs
vac-rs/app-server/tests/suite/v2/plugin_read.rs
vac-rs/app-server/tests/suite/v2/plugin_uninstall.rs
vac-rs/app-server/tests/suite/v2/turn_start_zsh_branch.rs
vac-rs/app-server/tests/suite/v2/review.rs
vac-rs/app-server/tests/suite/v2/thread_status.rs
vac-rs/app-server/tests/suite/v2/thread_loaded_list.rs
vac-rs/app-server/tests/suite/v2/memory_reset.rs
vac-rs/app-server/tests/suite/v2/experimental_feature_list.rs
vac-rs/app-server/tests/suite/v2/thread_rollback.rs
vac-rs/app-server/tests/suite/v2/skills_list.rs
vac-rs/app-server/tests/suite/v2/plugin_list.rs
vac-rs/app-server/tests/suite/v2/thread_start.rs
vac-rs/app-server/tests/suite/v2/turn_interrupt.rs
vac-rs/app-server/tests/suite/v2/thread_unarchive.rs
vac-rs/app-server/tests/suite/v2/thread_unsubscribe.rs
vac-rs/app-server/tests/suite/v2/account.rs
vac-rs/app-server/tests/suite/v2/thread_metadata_update.rs
vac-rs/app-server/tests/suite/v2/thread_list.rs
vac-rs/app-server/tests/suite/v2/mcp_server_elicitation.rs
vac-rs/app-server/tests/suite/v2/request_permissions.rs
vac-rs/app-server/tests/suite/v2/plugin_share.rs
vac-rs/app-server/tests/suite/v2/mcp_server_status.rs
vac-rs/app-server/tests/suite/v2/mcp_resource.rs
vac-rs/app-server/tests/suite/v2/plugin_install.rs
vac-rs/app-server/tests/suite/v2/thread_inject_items.rs
vac-rs/app-server/tests/suite/v2/marketplace_add.rs
vac-rs/app-server/tests/suite/v2/compaction.rs
vac-rs/app-server/tests/suite/v2/safety_check_downgrade.rs
vac-rs/app-server/tests/suite/v2/experimental_api.rs
vac-rs/app-server/tests/suite/v2/turn_steer.rs
vac-rs/app-server/tests/suite/v2/thread_read.rs
vac-rs/app-server/tests/suite/v2/initialize.rs
vac-rs/app-server/tests/suite/v2/fs.rs
vac-rs/app-server/tests/suite/v2/client_metadata.rs
vac-rs/app-server/tests/suite/v2/collaboration_mode_list.rs
vac-rs/app-server/tests/suite/v2/remote_thread_store.rs
vac-rs/app-server/tests/suite/v2/dynamic_tools.rs
vac-rs/app-server/tests/suite/v2/connection_handling_websocket.rs
vac-rs/app-server/tests/suite/v2/thread_shell_command.rs
vac-rs/app-server/tests/suite/v2/realtime_conversation.rs
vac-rs/app-server/tests/suite/v2/rate_limits.rs
vac-rs/app-server/tests/suite/v2/windows_sandbox_setup.rs
vac-rs/app-server/tests/suite/v2/thread_resume.rs
vac-rs/app-server/tests/suite/v2/command_exec.rs
vac-rs/app-server/tests/suite/v2/thread_memory_mode_set.rs
vac-rs/app-server/tests/suite/v2/model_provider_capabilities_read.rs
vac-rs/app-server/tests/suite/v2/hooks_list.rs
vac-rs/app-server/tests/suite/v2/request_user_input.rs
vac-rs/app-server/tests/suite/v2/external_agent_config.rs
vac-rs/app-server/tests/suite/v2/output_schema.rs
vac-rs/app-server/tests/suite/v2/marketplace_remove.rs
vac-rs/app-server/tests/suite/conversation_summary.rs
vac-rs/app-server/tests/suite/auth.rs
vac-rs/app-server/src/fuzzy_file_search.rs
vac-rs/app-server/src/lib.rs
vac-rs/app-server/src/outgoing_message.rs
vac-rs/app-server/src/fs_watch.rs
vac-rs/app-server/src/vac_message_processor.rs
vac-rs/app-server/src/transport.rs
vac-rs/app-server/src/thread_status.rs
vac-rs/app-server/src/main.rs
vac-rs/app-server/src/request_serialization.rs
vac-rs/app-server/src/config_manager_service_tests.rs
vac-rs/app-server/src/in_process.rs
vac-rs/app-server/src/thread_state.rs
vac-rs/app-server/src/config_manager_service.rs
vac-rs/app-server/src/fs_api.rs
vac-rs/app-server/src/message_processor.rs
vac-rs/app-server/src/error_code.rs
vac-rs/app-server/src/server_request_error.rs
vac-rs/app-server/src/message_processor/tracing_tests.rs
vac-rs/app-server/src/transport_tests.rs
vac-rs/app-server/src/filters.rs
vac-rs/app-server/src/external_agent_config_api.rs
vac-rs/app-server/src/config_api.rs
vac-rs/app-server/src/vac_message_processor/plugin_mcp_oauth.rs
vac-rs/app-server/src/vac_message_processor/apps_list_helpers.rs
vac-rs/app-server/src/vac_message_processor/plugins.rs
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs
vac-rs/app-server/src/vac_message_processor/plugin_app_helpers.rs
vac-rs/app-server/src/dynamic_tools.rs
vac-rs/app-server/src/bespoke_event_handling.rs
vac-rs/app-server/src/device_key_api.rs
vac-rs/app-server/src/app_server_tracing.rs
vac-rs/app-server/src/models.rs
vac-rs/app-server/src/command_exec.rs
vac-rs/app-server/Cargo.toml
vac-rs/app-server-client/src/lib.rs
vac-rs/app-server-client/Cargo.toml
vac-rs/otel/src/lib.rs
vac-rs/login/tests/suite/auth_refresh.rs
vac-rs/login/tests/suite/logout.rs
vac-rs/login/src/auth/manager.rs
vac-rs/login/src/auth/revoke.rs
vac-rs/login/src/auth/storage.rs
vac-rs/login/src/auth/external_bearer.rs
vac-rs/login/src/auth/auth_tests.rs
vac-rs/login/src/server.rs
vac-rs/app-server-protocol/tests/schema_fixtures.rs
vac-rs/app-server-protocol/src/bin/export.rs
vac-rs/app-server-protocol/src/bin/write_schema_fixtures.rs
vac-rs/app-server-protocol/src/export.rs
vac-rs/app-server-protocol/Cargo.toml
vac-rs/external-agent-sessions/src/export.rs
vac-rs/config/src/lib.rs
vac-rs/config/src/types.rs
vac-rs/config/src/profile_toml.rs
vac-rs/config/src/thread_config.rs
vac-rs/config/src/fingerprint.rs
vac-rs/config/src/config_toml.rs
vac-rs/config/src/state.rs
vac-rs/config/src/diagnostics.rs
vac-rs/config/src/loader/mod.rs
vac-rs/core/tests/suite/deprecation_notice.rs
vac-rs/core/src/context/apps_instructions.rs
vac-rs/core/src/mcp_tool_call.rs
vac-rs/core/src/exec_policy.rs
vac-rs/core/src/agents_md.rs
vac-rs/core/src/agent/role.rs
vac-rs/core/src/thread_manager.rs
vac-rs/core/src/client_tests.rs
vac-rs/core/src/session/tests/guardian_tests.rs
vac-rs/core/src/session/mod.rs
vac-rs/core/src/session/tests.rs
vac-rs/core/src/config/config_tests.rs
vac-rs/core/src/config/config_loader_tests.rs
vac-rs/core/src/tools/spec_tests.rs
vac-rs/core/src/tools/handlers/request_plugin_install.rs
vac-rs/core/src/network_proxy_loader.rs
vac-rs/core/src/client.rs
vac-rs/core/src/realtime_conversation.rs
vac-rs/core/src/exec_policy_tests.rs
vac-rs/core/src/connectors.rs
vac-rs/core/src/apps/render.rs
vac-rs/tools/src/tool_registry_plan_tests.rs
vac-rs/tools/src/tool_discovery_tests.rs
vac-rs/tools/src/request_plugin_install.rs
vac-rs/tools/src/tool_discovery.rs
vac-rs/target/dev-small/.fingerprint/vac-login-28fa2c5b0b8f25eb/lib-vac_login.json
vac-rs/target/dev-small/.fingerprint/vac-core-plugins-e11382c8eeb34d99/lib-vac_core_plugins.json
vac-rs/target/dev-small/.fingerprint/vac-core-skills-b5f2f34004e76746/lib-vac_core_skills.json
vac-rs/target/dev-small/.fingerprint/vac-otel-5f5fcf8d1de0c154/lib-vac_otel.json
vac-rs/target/dev-small/.fingerprint/vac-tui-a81af93b703fbb0c/test-bin-md-events.json
vac-rs/target/dev-small/.fingerprint/vac-connectors-4ff96751003017a9/lib-vac_connectors.json
vac-rs/target/dev-small/.fingerprint/vac-core-c6bfcac37144c9e0/test-lib-vac_core.json
vac-rs/target/dev-small/.fingerprint/vac-tui-c9fa9a8e8d4eb7b7/test-integration-test-all.json
vac-rs/target/dev-small/.fingerprint/vac-models-manager-9704e271951699c2/lib-vac_models_manager.json
vac-rs/target/dev-small/.fingerprint/vac-core-plugins-1d9a7198d718f861/lib-vac_core_plugins.json
vac-rs/target/dev-small/.fingerprint/vac-tui-d0144349b7af6054/lib-vac_tui.json
vac-rs/target/dev-small/.fingerprint/vac-core-plugins-37ae9a7ade030fb1/lib-vac_core_plugins.json
vac-rs/target/dev-small/.fingerprint/vac-app-server-47830cee9fd4208e/lib-vac_app_server.json
vac-rs/target/dev-small/.fingerprint/vac-connectors-0fde945c147d298e/lib-vac_connectors.json
vac-rs/target/dev-small/.fingerprint/vac-login-b183b8700301ea3b/lib-vac_login.json
vac-rs/target/dev-small/.fingerprint/vac-tui-15cbf1b99e5671c2/test-integration-test-test_backend.json
vac-rs/target/dev-small/.fingerprint/vac-models-manager-e3068b390e3d1733/lib-vac_models_manager.json
vac-rs/target/dev-small/.fingerprint/vac-core-cb8fa20b03e9e4a1/lib-vac_core.json
vac-rs/target/dev-small/.fingerprint/vac-app-server-bf4d0434884f3fdc/lib-vac_app_server.json
vac-rs/target/dev-small/.fingerprint/vac-app-server-client-c6f919d205ffef74/lib-vac_app_server_client.json
vac-rs/target/dev-small/.fingerprint/vac-config-d76e2124c6128e3d/lib-vac_config.json
vac-rs/target/dev-small/.fingerprint/vac-tools-6b7e4208191644af/lib-vac_tools.json
vac-rs/target/dev-small/.fingerprint/vac-core-skills-6916ec49b73eacc2/lib-vac_core_skills.json
vac-rs/target/dev-small/.fingerprint/vac-models-manager-b9acd6181fd4ec48/lib-vac_models_manager.json
vac-rs/target/dev-small/.fingerprint/vac-core-408f5d045f2e8209/lib-vac_core.json
vac-rs/target/dev-small/.fingerprint/vac-core-ba43d18ef63d8fc6/test-lib-vac_core.json
vac-rs/target/dev-small/.fingerprint/vac-app-server-transport-54739220a44ba852/lib-vac_app_server_transport.json
vac-rs/target/dev-small/.fingerprint/vac-tools-f7ff5d502c644817/lib-vac_tools.json
vac-rs/target/dev-small/.fingerprint/vac-tui-981f80e2a2ab6fc8/test-bin-vac-tui.json
vac-rs/target/dev-small/.fingerprint/vac-analytics-5d906ebc729402e7/lib-vac_analytics.json
vac-rs/target/dev-small/.fingerprint/vac-model-provider-info-61266160f46093a9/lib-vac_model_provider_info.json
vac-rs/target/dev-small/.fingerprint/vac-core-1a6bcc372f9ce031/lib-vac_core.json
vac-rs/target/dev-small/.fingerprint/vac-model-provider-info-ecad28245da299f8/lib-vac_model_provider_info.json
vac-rs/target/dev-small/.fingerprint/vac-exec-server-57bb872661550296/lib-vac_exec_server.json
vac-rs/target/dev-small/.fingerprint/vac-app-server-protocol-26769ceb8ad5252b/lib-vac_app_server_protocol.json
vac-rs/target/dev-small/.fingerprint/vac-core-plugins-76b71d107b739562/lib-vac_core_plugins.json
vac-rs/target/dev-small/.fingerprint/vac-config-9649634e71e8c759/lib-vac_config.json
vac-rs/target/dev-small/.fingerprint/vac-login-03b35045cfa846f7/lib-vac_login.json
vac-rs/target/dev-small/.fingerprint/vac-tui-722201cdd25c320c/test-integration-test-manager_dependency_regression.json
vac-rs/target/dev-small/.fingerprint/vac-app-server-transport-6ded6e92a9093c0b/lib-vac_app_server_transport.json
vac-rs/target/dev-small/.fingerprint/vac-core-e6c51e133ebb01e4/lib-vac_core.json
vac-rs/target/dev-small/.fingerprint/vac-core-skills-c303ae155938751c/lib-vac_core_skills.json
vac-rs/target/dev-small/.fingerprint/vac-models-manager-1ebe4bf24ee1fa90/lib-vac_models_manager.json
vac-rs/target/dev-small/.fingerprint/vac-core-skills-9ba4d9f97ba9d64c/lib-vac_core_skills.json
vac-rs/target/dev-small/.fingerprint/vac-core-2d354e4d51289b52/lib-vac_core.json
vac-rs/target/dev-small/.fingerprint/vac-exec-server-298e6d7941c8d5a0/lib-vac_exec_server.json
vac-rs/target/dev-small/.fingerprint/vac-app-server-client-0937e4637bbcf3bd/lib-vac_app_server_client.json
vac-rs/target/dev-small/.fingerprint/vac-tui-421a4bbbe6ae2256/lib-vac_tui.json
vac-rs/target/dev-small/.fingerprint/vac-analytics-f2fa8650acb33e40/lib-vac_analytics.json
vac-rs/target/dev-small/.fingerprint/vac-model-provider-info-47c83d56ac6cc922/lib-vac_model_provider_info.json
vac-rs/target/dev-small/.fingerprint/vac-tui-134b82f8a40f678b/test-lib-vac_tui.json
vac-rs/target/dev-small/.fingerprint/vac-analytics-8f854f11b0d5700c/lib-vac_analytics.json
vac-rs/target/dev-small/.fingerprint/vac-exec-server-ae3882299f8334f5/lib-vac_exec_server.json
vac-rs/target/dev-small/.fingerprint/vac-chatgpt-512a0daa6656418e/lib-vac_chatgpt.json
vac-rs/target/dev-small/.fingerprint/vac-otel-f45c15c814d1642e/lib-vac_otel.json
vac-rs/target/dev-small/.fingerprint/vac-config-f32f7f980d50b156/lib-vac_config.json
vac-rs/target/dev-small/.fingerprint/vac-login-a9bc28a96e92fab6/lib-vac_login.json
vac-rs/target/dev-small/.fingerprint/vac-app-server-protocol-2fdc7899c3047532/lib-vac_app_server_protocol.json
vac-rs/target/dev-small/.fingerprint/vac-chatgpt-dcdc28486a6373a3/lib-vac_chatgpt.json
vac-rs/target/dev-small/.fingerprint/vac-otel-cf9157ef79edac50/lib-vac_otel.json
... [2447 total lines captured; truncated in this evidence file for readability]
```

### Source/manifests fallback excluding `vac-rs/target`

Command:

```sh
find vac-rs -path 'vac-rs/target' -prune -o \
  \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' \) -type f -print0 \
  | xargs -0 grep -l 'vac_app_server' | sort
```

- Source/manifests count: `211`

```text
vac-rs/analytics/src/analytics_client_tests.rs
vac-rs/analytics/src/client.rs
vac-rs/analytics/src/client_tests.rs
vac-rs/analytics/src/events.rs
vac-rs/analytics/src/facts.rs
vac-rs/analytics/src/reducer.rs
vac-rs/app-server/Cargo.toml
vac-rs/app-server-client/Cargo.toml
vac-rs/app-server-client/src/lib.rs
vac-rs/app-server-protocol/Cargo.toml
vac-rs/app-server-protocol/src/bin/export.rs
vac-rs/app-server-protocol/src/bin/write_schema_fixtures.rs
vac-rs/app-server-protocol/src/export.rs
vac-rs/app-server-protocol/tests/schema_fixtures.rs
vac-rs/app-server/src/app_server_tracing.rs
vac-rs/app-server/src/bespoke_event_handling.rs
vac-rs/app-server/src/command_exec.rs
vac-rs/app-server/src/config_api.rs
vac-rs/app-server/src/config_manager_service.rs
vac-rs/app-server/src/config_manager_service_tests.rs
vac-rs/app-server/src/device_key_api.rs
vac-rs/app-server/src/dynamic_tools.rs
vac-rs/app-server/src/error_code.rs
vac-rs/app-server/src/external_agent_config_api.rs
vac-rs/app-server/src/filters.rs
vac-rs/app-server/src/fs_api.rs
vac-rs/app-server/src/fs_watch.rs
vac-rs/app-server/src/fuzzy_file_search.rs
vac-rs/app-server/src/in_process.rs
vac-rs/app-server/src/lib.rs
vac-rs/app-server/src/main.rs
vac-rs/app-server/src/message_processor.rs
vac-rs/app-server/src/message_processor/tracing_tests.rs
vac-rs/app-server/src/models.rs
vac-rs/app-server/src/outgoing_message.rs
vac-rs/app-server/src/request_serialization.rs
vac-rs/app-server/src/server_request_error.rs
vac-rs/app-server/src/thread_state.rs
vac-rs/app-server/src/thread_status.rs
vac-rs/app-server/src/transport.rs
vac-rs/app-server/src/transport_tests.rs
vac-rs/app-server/src/vac_message_processor/apps_list_helpers.rs
vac-rs/app-server/src/vac_message_processor/plugin_app_helpers.rs
vac-rs/app-server/src/vac_message_processor/plugin_mcp_oauth.rs
vac-rs/app-server/src/vac_message_processor/plugins.rs
vac-rs/app-server/src/vac_message_processor.rs
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs
vac-rs/app-server/tests/common/auth_fixtures.rs
vac-rs/app-server/tests/common/lib.rs
vac-rs/app-server/tests/common/mcp_process.rs
vac-rs/app-server/tests/suite/auth.rs
vac-rs/app-server/tests/suite/conversation_summary.rs
vac-rs/app-server/tests/suite/fuzzy_file_search.rs
vac-rs/app-server/tests/suite/v2/account.rs
vac-rs/app-server/tests/suite/v2/app_list.rs
vac-rs/app-server/tests/suite/v2/client_metadata.rs
vac-rs/app-server/tests/suite/v2/collaboration_mode_list.rs
vac-rs/app-server/tests/suite/v2/command_exec.rs
vac-rs/app-server/tests/suite/v2/compaction.rs
vac-rs/app-server/tests/suite/v2/config_rpc.rs
vac-rs/app-server/tests/suite/v2/connection_handling_websocket.rs
vac-rs/app-server/tests/suite/v2/connection_handling_websocket_unix.rs
vac-rs/app-server/tests/suite/v2/device_key.rs
vac-rs/app-server/tests/suite/v2/dynamic_tools.rs
vac-rs/app-server/tests/suite/v2/experimental_api.rs
vac-rs/app-server/tests/suite/v2/experimental_feature_list.rs
vac-rs/app-server/tests/suite/v2/external_agent_config.rs
vac-rs/app-server/tests/suite/v2/fs.rs
vac-rs/app-server/tests/suite/v2/hooks_list.rs
vac-rs/app-server/tests/suite/v2/initialize.rs
vac-rs/app-server/tests/suite/v2/marketplace_add.rs
vac-rs/app-server/tests/suite/v2/marketplace_remove.rs
vac-rs/app-server/tests/suite/v2/marketplace_upgrade.rs
vac-rs/app-server/tests/suite/v2/mcp_resource.rs
vac-rs/app-server/tests/suite/v2/mcp_server_elicitation.rs
vac-rs/app-server/tests/suite/v2/mcp_server_status.rs
vac-rs/app-server/tests/suite/v2/mcp_tool.rs
vac-rs/app-server/tests/suite/v2/memory_reset.rs
vac-rs/app-server/tests/suite/v2/model_list.rs
vac-rs/app-server/tests/suite/v2/model_provider_capabilities_read.rs
vac-rs/app-server/tests/suite/v2/output_schema.rs
vac-rs/app-server/tests/suite/v2/plan_item.rs
vac-rs/app-server/tests/suite/v2/plugin_install.rs
vac-rs/app-server/tests/suite/v2/plugin_list.rs
vac-rs/app-server/tests/suite/v2/plugin_read.rs
vac-rs/app-server/tests/suite/v2/plugin_share.rs
vac-rs/app-server/tests/suite/v2/plugin_uninstall.rs
vac-rs/app-server/tests/suite/v2/rate_limits.rs
vac-rs/app-server/tests/suite/v2/realtime_conversation.rs
vac-rs/app-server/tests/suite/v2/remote_thread_store.rs
vac-rs/app-server/tests/suite/v2/request_permissions.rs
vac-rs/app-server/tests/suite/v2/request_user_input.rs
vac-rs/app-server/tests/suite/v2/review.rs
vac-rs/app-server/tests/suite/v2/safety_check_downgrade.rs
vac-rs/app-server/tests/suite/v2/skills_list.rs
vac-rs/app-server/tests/suite/v2/thread_archive.rs
vac-rs/app-server/tests/suite/v2/thread_branch.rs
vac-rs/app-server/tests/suite/v2/thread_inject_items.rs
vac-rs/app-server/tests/suite/v2/thread_list.rs
vac-rs/app-server/tests/suite/v2/thread_loaded_list.rs
vac-rs/app-server/tests/suite/v2/thread_memory_mode_set.rs
vac-rs/app-server/tests/suite/v2/thread_metadata_update.rs
vac-rs/app-server/tests/suite/v2/thread_name_websocket.rs
vac-rs/app-server/tests/suite/v2/thread_read.rs
vac-rs/app-server/tests/suite/v2/thread_resume.rs
vac-rs/app-server/tests/suite/v2/thread_rollback.rs
vac-rs/app-server/tests/suite/v2/thread_shell_command.rs
vac-rs/app-server/tests/suite/v2/thread_start.rs
vac-rs/app-server/tests/suite/v2/thread_status.rs
vac-rs/app-server/tests/suite/v2/thread_unarchive.rs
vac-rs/app-server/tests/suite/v2/thread_unsubscribe.rs
vac-rs/app-server/tests/suite/v2/turn_interrupt.rs
vac-rs/app-server/tests/suite/v2/turn_start.rs
vac-rs/app-server/tests/suite/v2/turn_start_zsh_branch.rs
vac-rs/app-server/tests/suite/v2/turn_steer.rs
vac-rs/app-server/tests/suite/v2/windows_sandbox_setup.rs
vac-rs/app-server-transport/Cargo.toml
vac-rs/app-server-transport/src/outgoing_message.rs
vac-rs/app-server-transport/src/transport/mod.rs
vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs
vac-rs/app-server-transport/src/transport/remote_control/mod.rs
vac-rs/app-server-transport/src/transport/remote_control/protocol.rs
vac-rs/app-server-transport/src/transport/remote_control/segment.rs
vac-rs/app-server-transport/src/transport/remote_control/segment_tests.rs
vac-rs/app-server-transport/src/transport/remote_control/tests.rs
vac-rs/app-server-transport/src/transport/remote_control/websocket.rs
vac-rs/app-server-transport/src/transport/stdio.rs
vac-rs/app-server-transport/src/transport/unix_socket_tests.rs
vac-rs/chatgpt/src/connectors.rs
vac-rs/config/src/config_toml.rs
vac-rs/config/src/diagnostics.rs
vac-rs/config/src/fingerprint.rs
vac-rs/config/src/lib.rs
vac-rs/config/src/loader/mod.rs
vac-rs/config/src/profile_toml.rs
vac-rs/config/src/state.rs
vac-rs/config/src/thread_config.rs
vac-rs/config/src/types.rs
vac-rs/connectors/src/accessible.rs
vac-rs/connectors/src/filter.rs
vac-rs/connectors/src/lib.rs
vac-rs/connectors/src/merge.rs
vac-rs/connectors/src/metadata.rs
vac-rs/core-plugins/src/manager_tests.rs
vac-rs/core-plugins/src/marketplace.rs
vac-rs/core-plugins/src/remote.rs
vac-rs/core-plugins/src/remote/share/tests.rs
vac-rs/core-skills/src/config_rules.rs
vac-rs/core-skills/src/loader.rs
vac-rs/core-skills/src/manager_tests.rs
vac-rs/core/src/agent/role.rs
vac-rs/core/src/agents_md.rs
vac-rs/core/src/apps/render.rs
vac-rs/core/src/client.rs
vac-rs/core/src/client_tests.rs
vac-rs/core/src/config/config_loader_tests.rs
vac-rs/core/src/config/config_tests.rs
vac-rs/core/src/connectors.rs
vac-rs/core/src/context/apps_instructions.rs
vac-rs/core/src/exec_policy.rs
vac-rs/core/src/exec_policy_tests.rs
vac-rs/core/src/mcp_tool_call.rs
vac-rs/core/src/network_proxy_loader.rs
vac-rs/core/src/realtime_conversation.rs
vac-rs/core/src/session/mod.rs
vac-rs/core/src/session/tests/guardian_tests.rs
vac-rs/core/src/session/tests.rs
vac-rs/core/src/thread_manager.rs
vac-rs/core/src/tools/handlers/request_plugin_install.rs
vac-rs/core/src/tools/spec_tests.rs
vac-rs/core/tests/suite/deprecation_notice.rs
vac-rs/exec-server/src/client/reqwest_http_client.rs
vac-rs/exec-server/src/client.rs
vac-rs/exec-server/src/connection.rs
vac-rs/exec-server/src/fs_helper.rs
vac-rs/exec-server/src/fs_sandbox.rs
vac-rs/exec-server/src/local_process.rs
vac-rs/exec-server/src/rpc.rs
vac-rs/exec-server/src/sandboxed_file_system.rs
vac-rs/exec-server/src/server/file_system_handler.rs
vac-rs/exec-server/src/server/handler.rs
vac-rs/exec-server/src/server/jsonrpc.rs
vac-rs/exec-server/src/server/process_handler.rs
vac-rs/exec-server/src/server/processor.rs
vac-rs/exec-server/src/server/session_registry.rs
vac-rs/exec-server/tests/common/exec_server.rs
vac-rs/exec-server/tests/http_client.rs
vac-rs/exec-server/tests/http_request.rs
vac-rs/exec-server/tests/initialize.rs
vac-rs/exec-server/tests/process.rs
vac-rs/exec-server/tests/websocket.rs
vac-rs/external-agent-sessions/src/export.rs
vac-rs/login/src/auth/auth_tests.rs
vac-rs/login/src/auth/external_bearer.rs
vac-rs/login/src/auth/manager.rs
vac-rs/login/src/auth/revoke.rs
vac-rs/login/src/auth/storage.rs
vac-rs/login/src/server.rs
vac-rs/login/tests/suite/auth_refresh.rs
vac-rs/login/tests/suite/logout.rs
vac-rs/model-provider-info/src/lib.rs
vac-rs/models-manager/src/lib.rs
vac-rs/models-manager/src/manager_tests.rs
vac-rs/otel/src/lib.rs
vac-rs/tools/src/request_plugin_install.rs
vac-rs/tools/src/tool_discovery.rs
vac-rs/tools/src/tool_discovery_tests.rs
vac-rs/tools/src/tool_registry_plan_tests.rs
vac-rs/tui/src/app_server_session.rs
vac-rs/tui/src/legacy_core.rs
vac-rs/tui/src/session_protocol.rs
```

## 2. Current `cargo +1.93.0 tree -p vac-surface-tui` app-server dependency status

Command from `vac-rs/`:

```sh
cargo +1.93.0 tree -p vac-surface-tui
```

- Exit status: `0`
- App-server matching lines in tree output: `21`

App-server matches:

```text
946:├── vac-app-server v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server)
983:│   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol)
2082:│   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
2187:│   │   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
2289:│   │   │   │   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
2301:│   │   │   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
2731:│   │   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
2769:│   │   │       │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
2805:│   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
2806:│   ├── vac-app-server-transport v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-transport)
2828:│   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
3113:│   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
3232:│   │   │   │   └── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
3253:│   │   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
3267:│   │   │   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
3525:│   │   │   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
3642:│   │   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
3911:├── vac-app-server-client v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-client)
3917:│   ├── vac-app-server v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server) (*)
3918:│   ├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
3927:├── vac-app-server-protocol v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-protocol) (*)
```

## 3. Current `vac doctor registry ..` output

Command from `vac-rs/`:

```sh
vac doctor registry ..
```

- Exit status: `2`

Stdout:

```text
```

Stderr:

```text
error: unexpected argument 'registry' found

Usage: vac doctor [OPTIONS]

For more information, try '--help'.
```

## 4. Delta vs prior `33-evidence` baseline

Prior baseline reference used for diff: `3e85ccf`.

### Counts

| Evidence item | Prior baseline | Current snapshot | Delta / note |
|---|---:|---:|---|
| Source/manifests `vac_app_server` file count excluding `target` | `211` | `211` | `0` |
| Literal `grep -rln vac_app_server vac-rs/` captured lines | not comparable from prior committed file | `2447` | literal command timed out if exit `124` |
| `cargo +1.93.0 tree -p vac-surface-tui` app-server matching lines | prior Plan 33 baseline did not capture this exact command | `21` | current dependency status captured above |
| `vac doctor registry ..` exit status | prior Plan 33 baseline did not capture this exact command | `2` | current CLI rejects this subcommand if exit nonzero |

### Source/manifests file-list diff

```diff
```

### What changed

- Runtime-ownership work after the prior baseline has changed the working tree and dependency surface while this snapshot was taken.
- The literal recursive grep now hits enough build artifacts under `vac-rs/target` that the bounded exact command timed out after 180 seconds; its partial captured list is recorded instead of pretending a complete count.
- The source/manifests fallback still shows nonzero `vac_app_server` references.
- `vac doctor registry ..` is not currently accepted by the available `vac doctor` CLI surface; output is captured verbatim above.

### What did not change

- Plan 33 is still not a delete-proof closeout.
- App-server references are nonzero, so Plan 31 implementation remains a blocker for this gate.
- Plan 32 gates remain required before Plan 33 can claim final delete/defer proof.

## Validation / hygiene

- Disk guard before capture: \`df -h . /tmp\` had at least 55G available during the snapshot workflow.
- Docs-only lane: this evidence file and the Plan 33 status/evidence index are the only intended edits.
- Unrelated existing dirty work from other agents was observed and left untouched.

