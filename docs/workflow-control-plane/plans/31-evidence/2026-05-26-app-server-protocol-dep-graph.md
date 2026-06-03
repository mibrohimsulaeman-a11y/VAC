# L20 app-server-protocol dependency graph audit

Date: 2026-05-26
Plan: 31E prep evidence
Mode: READ-ONLY source audit; this evidence file is the only intended repository change.

## Scope and method

- Discovery source: every `vac-rs/**/Cargo.toml` manifest declaring `vac-app-server-protocol = { workspace = true }`.
- Use-site source: `grep -rn "vac_app_server_protocol" <crate>/src/`.
- Classification: `SHALLOW` = 1-3 use sites, mechanical retire; `MEDIUM` = 4-20; `DEEP` = 20+ real fan-out.
- Note: current checkout reports **19** workspace-dependency manifests, not 22. The root workspace dependency entry and the `app-server-protocol` crate manifest are not counted as dependents because they do not declare `{ workspace = true }`.

## Summary table

| Retirement rank | Crate | Package | Use sites | Files | Class | Retirement focus |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 1 | `app-server/tests/common` | `app_test_support` | 0 | 0 | **SHALLOW** | Manifest-only/test-support dependency; remove or replace once callers stop needing protocol fixtures. |
| 2 | `chatgpt` | `vac-chatgpt` | 1 | 1 | **DONE** | L36 retired via `vac_core::connectors::AppInfo`; direct `vac-app-server-protocol` dep removed; SHA `7605f53`. |
| 3 | `model-provider-info` | `vac-model-provider-info` | 1 | 1 | **MEDIUM** | L23 reclassified: no owner-native `vac_protocol::AuthMode`; retiring requires canonical auth enum move/duplication or conversion/local type boundary. |
| 4 | `external-agent-sessions` | `vac-external-agent-sessions` | 2 | 1 | **MEDIUM** | L-THREADITEM confirmed `ThreadItem`/`build_turns_from_rollout_items` are deeply app-server v2 projection-coupled; use a slim `vac_protocol::items::ThreadHistoryItem` trait/adapter instead of relocating raw DTOs. |
| 5 | `models-manager` | `vac-models-manager` | 2 | 2 | **DONE** | L-31E-MM retired via `vac_protocol::auth::AuthMode`; direct `vac-app-server-protocol` dep removed. |
| 6 | `core-skills` | `vac-core-skills` | 3 | 3 | **DONE** | L23 retired via `vac_config::ConfigLayerSource`; direct `vac-app-server-protocol` dep removed; SHA `5f9b59f`. |
| 7 | `otel` | `vac-otel` | 6 | 1 | **MEDIUM** | AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency. |
| 8 | `connectors` | `vac-connectors` | 7 | 5 | **MEDIUM** | Connector/app metadata coupling; move AppInfo/AppBranding/AppMetadata to connector/core API owner. |
| 9 | `login` | `vac-login` | 7 | 6 | **MEDIUM** | AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency. |
| 10 | `tools` | `vac-tools` | 8 | 4 | **MEDIUM** | Connector/app metadata coupling; move AppInfo/AppBranding/AppMetadata to connector/core API owner. |
| 11 | `config` | `vac-config` | 12 | 9 | **MEDIUM** | Config provenance/schema coupling; retire after config-owned replacement types are available. |
| 12 | `core-plugins` | `vac-core-plugins` | 12 | 4 | **MEDIUM** | Config provenance/schema coupling; retire after config-owned replacement types are available. |
| 13 | `core` | `vac-core` | 29 | 20 | **DEEP** | AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency. |
| 14 | `exec-server` | `vac-exec-server` | 39 | 14 | **DEEP** | JSON-RPC envelope/error coupling; needs shared JSON-RPC crate or exec-owned protocol facades. |
| 15 | `tui` | `vac-tui` | 39 | 2 | **DEEP** | TUI app-server session adapter aliases protocol types; retire after UI session DTO boundary is stable. |
| 16 | `app-server-client` | `vac-app-server-client` | 41 | 1 | **DEEP** | Transport/client protocol boundary; retire late after replacement protocol crate/package exists. |
| 17 | `app-server-transport` | `vac-app-server-transport` | 57 | 11 | **DEEP** | AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency. |
| 18 | `analytics` | `vac-analytics` | 84 | 6 | **DEEP** | Telemetry/reducer/test fixtures consume request/response/thread protocol; retire after event DTO decoupling. |
| 19 | `app-server` | `vac-app-server` | 630 | 32 | **DEEP** | AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency. |

## L-AUTH structural move note

- 2026-05-27 / L-AUTH: `AuthMode` canonical ownership moved from `vac_app_server_protocol` into `vac_protocol::auth::AuthMode`; `vac_app_server_protocol::AuthMode` remains available via re-export. This unblocks follow-up mechanical consumer migrations without changing downstream call sites in this lane.
- 2026-05-27 / L-31E-MM: `vac-models-manager` now re-exports and tests against `vac_protocol::auth::AuthMode`; `vac-app-server-protocol` was removed from `vac-rs/models-manager/Cargo.toml`.
- 2026-05-27 / L-THREADITEM: relocation stopped after dependency audit. `ThreadItem` and `build_turns_from_rollout_items` depend on app-server v2 DTOs (`Turn`, `UserInput`, command/file/MCP/dynamic/collab projection types, and item builders). Do not mark `external-agent-sessions` unblocked; use the documented trait/adapter boundary in `2026-05-27-threaditem-relocation.md`.

## Recommended Plan 31E micro-slicing

1. **Manifest-only/test-support cleanup**: `app-server/tests/common` has 0 source references. Confirm no test-only fixture path still needs the dependency, then remove first.
2. **AuthMode shallow slice**: retire `model-provider-info`, `models-manager`, then `chatgpt`/`external-agent-sessions` as small mechanical moves once replacement owner types exist.
3. **Config provenance slice**: retire `core-skills`, then `otel`, `login`, `connectors`, `tools`, `core-plugins`, and `config` by moving config/app/plugin schema types to their owning crates.
4. **Exec JSON-RPC slice**: retire `exec-server` behind a shared JSON-RPC envelope/error crate or exec-server-local protocol facade.
5. **Core fan-out slice**: retire `core` after config, connector metadata, MCP elicitation, and thread-history replacement types are available.
6. **Client/transport boundary slice**: retire `app-server-client` and `app-server-transport` after the successor app protocol package is introduced.
7. **UI and analytics adapter slice**: retire `tui` and `analytics` after their adapter DTOs stop importing raw app-server protocol types.
8. **Final server slice**: retire `app-server` last; it is the primary producer/consumer of the current protocol and has the largest fan-out.

## Per-crate grep evidence

### `analytics`

- Package: `vac-analytics`
- Classification: **DEEP**
- `grep -rn` use-site count: **84**
- Source files touched: **6**
- Retirement focus: Telemetry/reducer/test fixtures consume request/response/thread protocol; retire after event DTO decoupling.
- File distribution:
  - `vac-rs/analytics/src/analytics_client_tests.rs`: 32
  - `vac-rs/analytics/src/client.rs`: 8
  - `vac-rs/analytics/src/client_tests.rs`: 21
  - `vac-rs/analytics/src/events.rs`: 1
  - `vac-rs/analytics/src/facts.rs`: 8
  - `vac-rs/analytics/src/reducer.rs`: 14

```text
vac-rs/analytics/src/events.rs:23:use vac_app_server_protocol::VACErrorInfo;
vac-rs/analytics/src/facts.rs:6:use vac_app_server_protocol::ClientRequest;
vac-rs/analytics/src/facts.rs:7:use vac_app_server_protocol::ClientResponsePayload;
vac-rs/analytics/src/facts.rs:8:use vac_app_server_protocol::InitializeParams;
vac-rs/analytics/src/facts.rs:9:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/analytics/src/facts.rs:10:use vac_app_server_protocol::RequestId;
vac-rs/analytics/src/facts.rs:11:use vac_app_server_protocol::ServerNotification;
vac-rs/analytics/src/facts.rs:12:use vac_app_server_protocol::ServerRequest;
vac-rs/analytics/src/facts.rs:13:use vac_app_server_protocol::ServerResponse;
vac-rs/analytics/src/client_tests.rs:9:use vac_app_server_protocol::ApprovalsReviewer as AppServerApprovalsReviewer;
vac-rs/analytics/src/client_tests.rs:10:use vac_app_server_protocol::AskForApproval as AppServerAskForApproval;
vac-rs/analytics/src/client_tests.rs:11:use vac_app_server_protocol::ClientRequest;
vac-rs/analytics/src/client_tests.rs:12:use vac_app_server_protocol::ClientResponsePayload;
vac-rs/analytics/src/client_tests.rs:13:use vac_app_server_protocol::PermissionProfile as AppServerPermissionProfile;
vac-rs/analytics/src/client_tests.rs:14:use vac_app_server_protocol::RequestId;
vac-rs/analytics/src/client_tests.rs:15:use vac_app_server_protocol::SandboxPolicy as AppServerSandboxPolicy;
vac-rs/analytics/src/client_tests.rs:16:use vac_app_server_protocol::SessionSource as AppServerSessionSource;
vac-rs/analytics/src/client_tests.rs:17:use vac_app_server_protocol::Thread;
vac-rs/analytics/src/client_tests.rs:18:use vac_app_server_protocol::ThreadArchiveParams;
vac-rs/analytics/src/client_tests.rs:19:use vac_app_server_protocol::ThreadArchiveResponse;
vac-rs/analytics/src/client_tests.rs:20:use vac_app_server_protocol::ThreadBranchResponse;
vac-rs/analytics/src/client_tests.rs:21:use vac_app_server_protocol::ThreadResumeResponse;
vac-rs/analytics/src/client_tests.rs:22:use vac_app_server_protocol::ThreadStartResponse;
vac-rs/analytics/src/client_tests.rs:23:use vac_app_server_protocol::ThreadStatus as AppServerThreadStatus;
vac-rs/analytics/src/client_tests.rs:24:use vac_app_server_protocol::Turn;
vac-rs/analytics/src/client_tests.rs:25:use vac_app_server_protocol::TurnStartParams;
vac-rs/analytics/src/client_tests.rs:26:use vac_app_server_protocol::TurnStartResponse;
vac-rs/analytics/src/client_tests.rs:27:use vac_app_server_protocol::TurnStatus as AppServerTurnStatus;
vac-rs/analytics/src/client_tests.rs:28:use vac_app_server_protocol::TurnSteerParams;
vac-rs/analytics/src/client_tests.rs:29:use vac_app_server_protocol::TurnSteerResponse;
vac-rs/analytics/src/client.rs:29:use vac_app_server_protocol::ClientRequest;
vac-rs/analytics/src/client.rs:30:use vac_app_server_protocol::ClientResponsePayload;
vac-rs/analytics/src/client.rs:31:use vac_app_server_protocol::InitializeParams;
vac-rs/analytics/src/client.rs:32:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/analytics/src/client.rs:33:use vac_app_server_protocol::RequestId;
vac-rs/analytics/src/client.rs:34:use vac_app_server_protocol::ServerNotification;
vac-rs/analytics/src/client.rs:35:use vac_app_server_protocol::ServerRequest;
vac-rs/analytics/src/client.rs:36:use vac_app_server_protocol::ServerResponse;
vac-rs/analytics/src/reducer.rs:53:use vac_app_server_protocol::ClientRequest;
vac-rs/analytics/src/reducer.rs:54:use vac_app_server_protocol::ClientResponse;
vac-rs/analytics/src/reducer.rs:55:use vac_app_server_protocol::InitializeParams;
vac-rs/analytics/src/reducer.rs:56:use vac_app_server_protocol::RequestId;
vac-rs/analytics/src/reducer.rs:57:use vac_app_server_protocol::ServerNotification;
vac-rs/analytics/src/reducer.rs:58:use vac_app_server_protocol::TurnSteerResponse;
vac-rs/analytics/src/reducer.rs:59:use vac_app_server_protocol::UserInput;
vac-rs/analytics/src/reducer.rs:60:use vac_app_server_protocol::VACErrorInfo;
vac-rs/analytics/src/reducer.rs:745:        thread: vac_app_server_protocol::Thread,
vac-rs/analytics/src/reducer.rs:1123:fn analytics_turn_status(status: vac_app_server_protocol::TurnStatus) -> Option<TurnStatus> {
vac-rs/analytics/src/reducer.rs:1125:        vac_app_server_protocol::TurnStatus::Completed => Some(TurnStatus::Completed),
vac-rs/analytics/src/reducer.rs:1126:        vac_app_server_protocol::TurnStatus::Failed => Some(TurnStatus::Failed),
vac-rs/analytics/src/reducer.rs:1127:        vac_app_server_protocol::TurnStatus::Interrupted => Some(TurnStatus::Interrupted),
vac-rs/analytics/src/reducer.rs:1128:        vac_app_server_protocol::TurnStatus::InProgress => None,
vac-rs/analytics/src/analytics_client_tests.rs:65:use vac_app_server_protocol::ApprovalsReviewer as AppServerApprovalsReviewer;
vac-rs/analytics/src/analytics_client_tests.rs:66:use vac_app_server_protocol::AskForApproval as AppServerAskForApproval;
vac-rs/analytics/src/analytics_client_tests.rs:67:use vac_app_server_protocol::ClientInfo;
vac-rs/analytics/src/analytics_client_tests.rs:68:use vac_app_server_protocol::ClientRequest;
vac-rs/analytics/src/analytics_client_tests.rs:69:use vac_app_server_protocol::ClientResponsePayload;
vac-rs/analytics/src/analytics_client_tests.rs:70:use vac_app_server_protocol::InitializeCapabilities;
vac-rs/analytics/src/analytics_client_tests.rs:71:use vac_app_server_protocol::InitializeParams;
vac-rs/analytics/src/analytics_client_tests.rs:72:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/analytics/src/analytics_client_tests.rs:73:use vac_app_server_protocol::NonSteerableTurnKind;
vac-rs/analytics/src/analytics_client_tests.rs:74:use vac_app_server_protocol::RequestId;
vac-rs/analytics/src/analytics_client_tests.rs:75:use vac_app_server_protocol::SandboxPolicy as AppServerSandboxPolicy;
vac-rs/analytics/src/analytics_client_tests.rs:76:use vac_app_server_protocol::ServerNotification;
vac-rs/analytics/src/analytics_client_tests.rs:77:use vac_app_server_protocol::SessionSource as AppServerSessionSource;
vac-rs/analytics/src/analytics_client_tests.rs:78:use vac_app_server_protocol::Thread;
vac-rs/analytics/src/analytics_client_tests.rs:79:use vac_app_server_protocol::ThreadArchiveParams;
vac-rs/analytics/src/analytics_client_tests.rs:80:use vac_app_server_protocol::ThreadArchiveResponse;
vac-rs/analytics/src/analytics_client_tests.rs:81:use vac_app_server_protocol::ThreadResumeResponse;
vac-rs/analytics/src/analytics_client_tests.rs:82:use vac_app_server_protocol::ThreadStartResponse;
vac-rs/analytics/src/analytics_client_tests.rs:83:use vac_app_server_protocol::ThreadStatus as AppServerThreadStatus;
vac-rs/analytics/src/analytics_client_tests.rs:84:use vac_app_server_protocol::Turn;
vac-rs/analytics/src/analytics_client_tests.rs:85:use vac_app_server_protocol::TurnCompletedNotification;
vac-rs/analytics/src/analytics_client_tests.rs:86:use vac_app_server_protocol::TurnError as AppServerTurnError;
vac-rs/analytics/src/analytics_client_tests.rs:87:use vac_app_server_protocol::TurnStartParams;
vac-rs/analytics/src/analytics_client_tests.rs:88:use vac_app_server_protocol::TurnStartedNotification;
vac-rs/analytics/src/analytics_client_tests.rs:89:use vac_app_server_protocol::TurnStatus as AppServerTurnStatus;
vac-rs/analytics/src/analytics_client_tests.rs:90:use vac_app_server_protocol::TurnSteerParams;
vac-rs/analytics/src/analytics_client_tests.rs:91:use vac_app_server_protocol::TurnSteerResponse;
vac-rs/analytics/src/analytics_client_tests.rs:92:use vac_app_server_protocol::UserInput;
vac-rs/analytics/src/analytics_client_tests.rs:93:use vac_app_server_protocol::VACErrorInfo;
vac-rs/analytics/src/analytics_client_tests.rs:240:    ClientResponsePayload::TurnStart(vac_app_server_protocol::TurnStartResponse {
vac-rs/analytics/src/analytics_client_tests.rs:286:    vac_error_info: Option<vac_app_server_protocol::VACErrorInfo>,
vac-rs/analytics/src/analytics_client_tests.rs:2543:                Some(vac_app_server_protocol::VACErrorInfo::BadRequest),
```

### `app-server`

- Package: `vac-app-server`
- Classification: **DEEP**
- `grep -rn` use-site count: **630**
- Source files touched: **32**
- Retirement focus: AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency.
- File distribution:
  - `vac-rs/app-server/src/app_server_tracing.rs`: 3
  - `vac-rs/app-server/src/bespoke_event_handling.rs`: 83
  - `vac-rs/app-server/src/command_exec.rs`: 19
  - `vac-rs/app-server/src/config_api.rs`: 30
  - `vac-rs/app-server/src/config_manager_service.rs`: 12
  - `vac-rs/app-server/src/config_manager_service_tests.rs`: 4
  - `vac-rs/app-server/src/device_key_api.rs`: 18
  - `vac-rs/app-server/src/dynamic_tools.rs`: 2
  - `vac-rs/app-server/src/error_code.rs`: 1
  - `vac-rs/app-server/src/external_agent_config_api.rs`: 13
  - `vac-rs/app-server/src/filters.rs`: 1
  - `vac-rs/app-server/src/fs_api.rs`: 16
  - `vac-rs/app-server/src/fs_watch.rs`: 7
  - `vac-rs/app-server/src/fuzzy_file_search.rs`: 5
  - `vac-rs/app-server/src/in_process.rs`: 24
  - `vac-rs/app-server/src/lib.rs`: 8
  - `vac-rs/app-server/src/message_processor.rs`: 32
  - `vac-rs/app-server/src/message_processor/tracing_tests.rs`: 18
  - `vac-rs/app-server/src/models.rs`: 3
  - `vac-rs/app-server/src/outgoing_message.rs`: 28
  - `vac-rs/app-server/src/request_serialization.rs`: 1
  - `vac-rs/app-server/src/server_request_error.rs`: 2
  - `vac-rs/app-server/src/thread_state.rs`: 6
  - `vac-rs/app-server/src/thread_status.rs`: 16
  - `vac-rs/app-server/src/transport.rs`: 2
  - `vac-rs/app-server/src/transport_tests.rs`: 12
  - `vac-rs/app-server/src/vac_message_processor.rs`: 243
  - `vac-rs/app-server/src/vac_message_processor/apps_list_helpers.rs`: 5
  - `vac-rs/app-server/src/vac_message_processor/plugin_app_helpers.rs`: 3
  - `vac-rs/app-server/src/vac_message_processor/plugin_mcp_oauth.rs`: 2
  - `vac-rs/app-server/src/vac_message_processor/plugins.rs`: 4
  - `vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs`: 7

```text
vac-rs/app-server/src/fuzzy_file_search.rs:9:use vac_app_server_protocol::FuzzyFileSearchMatchType;
vac-rs/app-server/src/fuzzy_file_search.rs:10:use vac_app_server_protocol::FuzzyFileSearchResult;
vac-rs/app-server/src/fuzzy_file_search.rs:11:use vac_app_server_protocol::FuzzyFileSearchSessionCompletedNotification;
vac-rs/app-server/src/fuzzy_file_search.rs:12:use vac_app_server_protocol::FuzzyFileSearchSessionUpdatedNotification;
vac-rs/app-server/src/fuzzy_file_search.rs:13:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/lib.rs:57:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/app-server/src/lib.rs:58:use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server/src/lib.rs:59:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server/src/lib.rs:60:use vac_app_server_protocol::RemoteControlStatusChangedNotification;
vac-rs/app-server/src/lib.rs:61:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/lib.rs:62:use vac_app_server_protocol::TextPosition as AppTextPosition;
vac-rs/app-server/src/lib.rs:63:use vac_app_server_protocol::TextRange as AppTextRange;
vac-rs/app-server/src/lib.rs:110:pub use vac_app_server_protocol as protocol;
vac-rs/app-server/src/outgoing_message.rs:13:use vac_app_server_protocol::ClientResponsePayload;
vac-rs/app-server/src/outgoing_message.rs:14:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/outgoing_message.rs:15:use vac_app_server_protocol::RequestId;
vac-rs/app-server/src/outgoing_message.rs:16:use vac_app_server_protocol::Result;
vac-rs/app-server/src/outgoing_message.rs:17:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/outgoing_message.rs:18:use vac_app_server_protocol::ServerRequest;
vac-rs/app-server/src/outgoing_message.rs:19:use vac_app_server_protocol::ServerRequestPayload;
vac-rs/app-server/src/outgoing_message.rs:655:    use vac_app_server_protocol::AccountLoginCompletedNotification;
vac-rs/app-server/src/outgoing_message.rs:656:    use vac_app_server_protocol::AccountRateLimitsUpdatedNotification;
vac-rs/app-server/src/outgoing_message.rs:657:    use vac_app_server_protocol::AccountUpdatedNotification;
vac-rs/app-server/src/outgoing_message.rs:658:    use vac_app_server_protocol::ApplyPatchApprovalParams;
vac-rs/app-server/src/outgoing_message.rs:659:    use vac_app_server_protocol::AuthMode;
vac-rs/app-server/src/outgoing_message.rs:660:    use vac_app_server_protocol::CommandExecutionApprovalDecision;
vac-rs/app-server/src/outgoing_message.rs:661:    use vac_app_server_protocol::CommandExecutionRequestApprovalParams;
vac-rs/app-server/src/outgoing_message.rs:662:    use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server/src/outgoing_message.rs:663:    use vac_app_server_protocol::DynamicToolCallParams;
vac-rs/app-server/src/outgoing_message.rs:664:    use vac_app_server_protocol::FileChangeRequestApprovalParams;
vac-rs/app-server/src/outgoing_message.rs:665:    use vac_app_server_protocol::GuardianWarningNotification;
vac-rs/app-server/src/outgoing_message.rs:666:    use vac_app_server_protocol::ModelRerouteReason;
vac-rs/app-server/src/outgoing_message.rs:667:    use vac_app_server_protocol::ModelReroutedNotification;
vac-rs/app-server/src/outgoing_message.rs:668:    use vac_app_server_protocol::ModelVerification;
vac-rs/app-server/src/outgoing_message.rs:669:    use vac_app_server_protocol::ModelVerificationNotification;
vac-rs/app-server/src/outgoing_message.rs:670:    use vac_app_server_protocol::RateLimitSnapshot;
vac-rs/app-server/src/outgoing_message.rs:671:    use vac_app_server_protocol::RateLimitWindow;
vac-rs/app-server/src/outgoing_message.rs:672:    use vac_app_server_protocol::ServerResponse;
vac-rs/app-server/src/outgoing_message.rs:673:    use vac_app_server_protocol::ToolRequestUserInputParams;
vac-rs/app-server/src/outgoing_message.rs:947:                    vac_app_server_protocol::ThreadArchiveResponse {},
vac-rs/app-server/src/outgoing_message.rs:997:                    vac_app_server_protocol::ThreadArchiveResponse {},
vac-rs/app-server/src/fs_watch.rs:17:use vac_app_server_protocol::FsChangedNotification;
vac-rs/app-server/src/fs_watch.rs:18:use vac_app_server_protocol::FsUnwatchParams;
vac-rs/app-server/src/fs_watch.rs:19:use vac_app_server_protocol::FsUnwatchResponse;
vac-rs/app-server/src/fs_watch.rs:20:use vac_app_server_protocol::FsWatchParams;
vac-rs/app-server/src/fs_watch.rs:21:use vac_app_server_protocol::FsWatchResponse;
vac-rs/app-server/src/fs_watch.rs:22:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/fs_watch.rs:23:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/vac_message_processor.rs:54:use vac_app_server_protocol::Account;
vac-rs/app-server/src/vac_message_processor.rs:55:use vac_app_server_protocol::AccountLoginCompletedNotification;
vac-rs/app-server/src/vac_message_processor.rs:56:use vac_app_server_protocol::AccountUpdatedNotification;
vac-rs/app-server/src/vac_message_processor.rs:57:use vac_app_server_protocol::AddCreditsNudgeCreditType;
vac-rs/app-server/src/vac_message_processor.rs:58:use vac_app_server_protocol::AddCreditsNudgeEmailStatus;
vac-rs/app-server/src/vac_message_processor.rs:59:use vac_app_server_protocol::AppInfo;
vac-rs/app-server/src/vac_message_processor.rs:60:use vac_app_server_protocol::AppSummary;
vac-rs/app-server/src/vac_message_processor.rs:61:use vac_app_server_protocol::AppsListParams;
vac-rs/app-server/src/vac_message_processor.rs:62:use vac_app_server_protocol::AppsListResponse;
vac-rs/app-server/src/vac_message_processor.rs:63:use vac_app_server_protocol::AskForApproval;
vac-rs/app-server/src/vac_message_processor.rs:64:use vac_app_server_protocol::AuthMode;
vac-rs/app-server/src/vac_message_processor.rs:65:use vac_app_server_protocol::CancelLoginAccountParams;
vac-rs/app-server/src/vac_message_processor.rs:66:use vac_app_server_protocol::CancelLoginAccountResponse;
vac-rs/app-server/src/vac_message_processor.rs:67:use vac_app_server_protocol::CancelLoginAccountStatus;
vac-rs/app-server/src/vac_message_processor.rs:68:use vac_app_server_protocol::ClientRequest;
vac-rs/app-server/src/vac_message_processor.rs:69:use vac_app_server_protocol::ClientResponsePayload;
vac-rs/app-server/src/vac_message_processor.rs:70:use vac_app_server_protocol::CollaborationModeListParams;
vac-rs/app-server/src/vac_message_processor.rs:71:use vac_app_server_protocol::CollaborationModeListResponse;
vac-rs/app-server/src/vac_message_processor.rs:72:use vac_app_server_protocol::CommandExecParams;
vac-rs/app-server/src/vac_message_processor.rs:73:use vac_app_server_protocol::CommandExecResizeParams;
vac-rs/app-server/src/vac_message_processor.rs:74:use vac_app_server_protocol::CommandExecTerminateParams;
vac-rs/app-server/src/vac_message_processor.rs:75:use vac_app_server_protocol::CommandExecWriteParams;
vac-rs/app-server/src/vac_message_processor.rs:76:use vac_app_server_protocol::ConversationGitInfo;
vac-rs/app-server/src/vac_message_processor.rs:77:use vac_app_server_protocol::ConversationSummary;
vac-rs/app-server/src/vac_message_processor.rs:78:use vac_app_server_protocol::DynamicToolSpec as ApiDynamicToolSpec;
vac-rs/app-server/src/vac_message_processor.rs:79:use vac_app_server_protocol::ExperimentalFeature as ApiExperimentalFeature;
vac-rs/app-server/src/vac_message_processor.rs:80:use vac_app_server_protocol::ExperimentalFeatureListParams;
vac-rs/app-server/src/vac_message_processor.rs:81:use vac_app_server_protocol::ExperimentalFeatureListResponse;
vac-rs/app-server/src/vac_message_processor.rs:82:use vac_app_server_protocol::ExperimentalFeatureStage as ApiExperimentalFeatureStage;
vac-rs/app-server/src/vac_message_processor.rs:83:use vac_app_server_protocol::FeedbackUploadParams;
vac-rs/app-server/src/vac_message_processor.rs:84:use vac_app_server_protocol::FeedbackUploadResponse;
vac-rs/app-server/src/vac_message_processor.rs:85:use vac_app_server_protocol::FuzzyFileSearchParams;
vac-rs/app-server/src/vac_message_processor.rs:86:use vac_app_server_protocol::FuzzyFileSearchResponse;
vac-rs/app-server/src/vac_message_processor.rs:87:use vac_app_server_protocol::FuzzyFileSearchSessionStartParams;
vac-rs/app-server/src/vac_message_processor.rs:88:use vac_app_server_protocol::FuzzyFileSearchSessionStartResponse;
vac-rs/app-server/src/vac_message_processor.rs:89:use vac_app_server_protocol::FuzzyFileSearchSessionStopParams;
vac-rs/app-server/src/vac_message_processor.rs:90:use vac_app_server_protocol::FuzzyFileSearchSessionStopResponse;
vac-rs/app-server/src/vac_message_processor.rs:91:use vac_app_server_protocol::FuzzyFileSearchSessionUpdateParams;
vac-rs/app-server/src/vac_message_processor.rs:92:use vac_app_server_protocol::FuzzyFileSearchSessionUpdateResponse;
vac-rs/app-server/src/vac_message_processor.rs:93:use vac_app_server_protocol::GetAccountParams;
vac-rs/app-server/src/vac_message_processor.rs:94:use vac_app_server_protocol::GetAccountRateLimitsResponse;
vac-rs/app-server/src/vac_message_processor.rs:95:use vac_app_server_protocol::GetAccountResponse;
vac-rs/app-server/src/vac_message_processor.rs:96:use vac_app_server_protocol::GetAuthStatusParams;
vac-rs/app-server/src/vac_message_processor.rs:97:use vac_app_server_protocol::GetAuthStatusResponse;
vac-rs/app-server/src/vac_message_processor.rs:98:use vac_app_server_protocol::GetConversationSummaryParams;
vac-rs/app-server/src/vac_message_processor.rs:99:use vac_app_server_protocol::GetConversationSummaryResponse;
vac-rs/app-server/src/vac_message_processor.rs:100:use vac_app_server_protocol::GitDiffToRemoteResponse;
vac-rs/app-server/src/vac_message_processor.rs:101:use vac_app_server_protocol::GitInfo as ApiGitInfo;
vac-rs/app-server/src/vac_message_processor.rs:102:use vac_app_server_protocol::HookMetadata;
vac-rs/app-server/src/vac_message_processor.rs:103:use vac_app_server_protocol::HooksListParams;
vac-rs/app-server/src/vac_message_processor.rs:104:use vac_app_server_protocol::HooksListResponse;
vac-rs/app-server/src/vac_message_processor.rs:105:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/vac_message_processor.rs:106:use vac_app_server_protocol::ListMcpServerStatusParams;
vac-rs/app-server/src/vac_message_processor.rs:107:use vac_app_server_protocol::ListMcpServerStatusResponse;
vac-rs/app-server/src/vac_message_processor.rs:108:use vac_app_server_protocol::LoginAccountParams;
vac-rs/app-server/src/vac_message_processor.rs:109:use vac_app_server_protocol::LoginAccountResponse;
vac-rs/app-server/src/vac_message_processor.rs:110:use vac_app_server_protocol::LoginApiKeyParams;
vac-rs/app-server/src/vac_message_processor.rs:111:use vac_app_server_protocol::LogoutAccountResponse;
vac-rs/app-server/src/vac_message_processor.rs:112:use vac_app_server_protocol::MarketplaceAddParams;
vac-rs/app-server/src/vac_message_processor.rs:113:use vac_app_server_protocol::MarketplaceAddResponse;
vac-rs/app-server/src/vac_message_processor.rs:114:use vac_app_server_protocol::MarketplaceInterface;
vac-rs/app-server/src/vac_message_processor.rs:115:use vac_app_server_protocol::MarketplaceRemoveParams;
vac-rs/app-server/src/vac_message_processor.rs:116:use vac_app_server_protocol::MarketplaceRemoveResponse;
vac-rs/app-server/src/vac_message_processor.rs:117:use vac_app_server_protocol::MarketplaceUpgradeErrorInfo;
vac-rs/app-server/src/vac_message_processor.rs:118:use vac_app_server_protocol::MarketplaceUpgradeParams;
vac-rs/app-server/src/vac_message_processor.rs:119:use vac_app_server_protocol::MarketplaceUpgradeResponse;
vac-rs/app-server/src/vac_message_processor.rs:120:use vac_app_server_protocol::McpResourceReadParams;
vac-rs/app-server/src/vac_message_processor.rs:121:use vac_app_server_protocol::McpResourceReadResponse;
vac-rs/app-server/src/vac_message_processor.rs:122:use vac_app_server_protocol::McpServerOauthLoginCompletedNotification;
vac-rs/app-server/src/vac_message_processor.rs:123:use vac_app_server_protocol::McpServerOauthLoginParams;
vac-rs/app-server/src/vac_message_processor.rs:124:use vac_app_server_protocol::McpServerOauthLoginResponse;
vac-rs/app-server/src/vac_message_processor.rs:125:use vac_app_server_protocol::McpServerRefreshResponse;
vac-rs/app-server/src/vac_message_processor.rs:126:use vac_app_server_protocol::McpServerStatus;
vac-rs/app-server/src/vac_message_processor.rs:127:use vac_app_server_protocol::McpServerStatusDetail;
vac-rs/app-server/src/vac_message_processor.rs:128:use vac_app_server_protocol::McpServerToolCallParams;
vac-rs/app-server/src/vac_message_processor.rs:129:use vac_app_server_protocol::McpServerToolCallResponse;
vac-rs/app-server/src/vac_message_processor.rs:130:use vac_app_server_protocol::MemoryResetResponse;
vac-rs/app-server/src/vac_message_processor.rs:131:use vac_app_server_protocol::MockExperimentalMethodParams;
vac-rs/app-server/src/vac_message_processor.rs:132:use vac_app_server_protocol::MockExperimentalMethodResponse;
vac-rs/app-server/src/vac_message_processor.rs:133:use vac_app_server_protocol::ModelListParams;
vac-rs/app-server/src/vac_message_processor.rs:134:use vac_app_server_protocol::ModelListResponse;
vac-rs/app-server/src/vac_message_processor.rs:135:use vac_app_server_protocol::PermissionProfileModificationParams;
vac-rs/app-server/src/vac_message_processor.rs:136:use vac_app_server_protocol::PermissionProfileSelectionParams;
vac-rs/app-server/src/vac_message_processor.rs:137:use vac_app_server_protocol::PluginDetail;
vac-rs/app-server/src/vac_message_processor.rs:138:use vac_app_server_protocol::PluginInstallParams;
vac-rs/app-server/src/vac_message_processor.rs:139:use vac_app_server_protocol::PluginInstallResponse;
vac-rs/app-server/src/vac_message_processor.rs:140:use vac_app_server_protocol::PluginInterface;
vac-rs/app-server/src/vac_message_processor.rs:141:use vac_app_server_protocol::PluginListParams;
vac-rs/app-server/src/vac_message_processor.rs:142:use vac_app_server_protocol::PluginListResponse;
vac-rs/app-server/src/vac_message_processor.rs:143:use vac_app_server_protocol::PluginMarketplaceEntry;
vac-rs/app-server/src/vac_message_processor.rs:144:use vac_app_server_protocol::PluginReadParams;
vac-rs/app-server/src/vac_message_processor.rs:145:use vac_app_server_protocol::PluginReadResponse;
vac-rs/app-server/src/vac_message_processor.rs:146:use vac_app_server_protocol::PluginShareDeleteParams;
vac-rs/app-server/src/vac_message_processor.rs:147:use vac_app_server_protocol::PluginShareDeleteResponse;
vac-rs/app-server/src/vac_message_processor.rs:148:use vac_app_server_protocol::PluginShareListItem;
vac-rs/app-server/src/vac_message_processor.rs:149:use vac_app_server_protocol::PluginShareListParams;
vac-rs/app-server/src/vac_message_processor.rs:150:use vac_app_server_protocol::PluginShareListResponse;
vac-rs/app-server/src/vac_message_processor.rs:151:use vac_app_server_protocol::PluginShareSaveParams;
vac-rs/app-server/src/vac_message_processor.rs:152:use vac_app_server_protocol::PluginShareSaveResponse;
vac-rs/app-server/src/vac_message_processor.rs:153:use vac_app_server_protocol::PluginSkillReadParams;
vac-rs/app-server/src/vac_message_processor.rs:154:use vac_app_server_protocol::PluginSkillReadResponse;
vac-rs/app-server/src/vac_message_processor.rs:155:use vac_app_server_protocol::PluginSource;
vac-rs/app-server/src/vac_message_processor.rs:156:use vac_app_server_protocol::PluginSummary;
vac-rs/app-server/src/vac_message_processor.rs:157:use vac_app_server_protocol::PluginUninstallParams;
vac-rs/app-server/src/vac_message_processor.rs:158:use vac_app_server_protocol::PluginUninstallResponse;
vac-rs/app-server/src/vac_message_processor.rs:159:use vac_app_server_protocol::RequestId;
vac-rs/app-server/src/vac_message_processor.rs:160:use vac_app_server_protocol::ReviewDelivery as ApiReviewDelivery;
vac-rs/app-server/src/vac_message_processor.rs:161:use vac_app_server_protocol::ReviewStartParams;
vac-rs/app-server/src/vac_message_processor.rs:162:use vac_app_server_protocol::ReviewStartResponse;
vac-rs/app-server/src/vac_message_processor.rs:163:use vac_app_server_protocol::ReviewTarget as ApiReviewTarget;
vac-rs/app-server/src/vac_message_processor.rs:164:use vac_app_server_protocol::SandboxMode;
vac-rs/app-server/src/vac_message_processor.rs:165:use vac_app_server_protocol::SendAddCreditsNudgeEmailParams;
vac-rs/app-server/src/vac_message_processor.rs:166:use vac_app_server_protocol::SendAddCreditsNudgeEmailResponse;
vac-rs/app-server/src/vac_message_processor.rs:167:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/vac_message_processor.rs:168:use vac_app_server_protocol::ServerRequestResolvedNotification;
vac-rs/app-server/src/vac_message_processor.rs:169:use vac_app_server_protocol::SkillSummary;
vac-rs/app-server/src/vac_message_processor.rs:170:use vac_app_server_protocol::SkillsConfigWriteParams;
vac-rs/app-server/src/vac_message_processor.rs:171:use vac_app_server_protocol::SkillsConfigWriteResponse;
vac-rs/app-server/src/vac_message_processor.rs:172:use vac_app_server_protocol::SkillsListParams;
vac-rs/app-server/src/vac_message_processor.rs:173:use vac_app_server_protocol::SkillsListResponse;
vac-rs/app-server/src/vac_message_processor.rs:174:use vac_app_server_protocol::SortDirection;
vac-rs/app-server/src/vac_message_processor.rs:175:use vac_app_server_protocol::Thread;
vac-rs/app-server/src/vac_message_processor.rs:176:use vac_app_server_protocol::ThreadApproveGuardianDeniedActionParams;
vac-rs/app-server/src/vac_message_processor.rs:177:use vac_app_server_protocol::ThreadApproveGuardianDeniedActionResponse;
vac-rs/app-server/src/vac_message_processor.rs:178:use vac_app_server_protocol::ThreadArchiveParams;
vac-rs/app-server/src/vac_message_processor.rs:179:use vac_app_server_protocol::ThreadArchiveResponse;
vac-rs/app-server/src/vac_message_processor.rs:180:use vac_app_server_protocol::ThreadArchivedNotification;
vac-rs/app-server/src/vac_message_processor.rs:181:use vac_app_server_protocol::ThreadBackgroundTerminalsCleanParams;
vac-rs/app-server/src/vac_message_processor.rs:182:use vac_app_server_protocol::ThreadBackgroundTerminalsCleanResponse;
vac-rs/app-server/src/vac_message_processor.rs:183:use vac_app_server_protocol::ThreadBranchParams;
vac-rs/app-server/src/vac_message_processor.rs:184:use vac_app_server_protocol::ThreadBranchResponse;
vac-rs/app-server/src/vac_message_processor.rs:185:use vac_app_server_protocol::ThreadClosedNotification;
vac-rs/app-server/src/vac_message_processor.rs:186:use vac_app_server_protocol::ThreadCompactStartParams;
vac-rs/app-server/src/vac_message_processor.rs:187:use vac_app_server_protocol::ThreadCompactStartResponse;
vac-rs/app-server/src/vac_message_processor.rs:188:use vac_app_server_protocol::ThreadDecrementElicitationParams;
vac-rs/app-server/src/vac_message_processor.rs:189:use vac_app_server_protocol::ThreadDecrementElicitationResponse;
vac-rs/app-server/src/vac_message_processor.rs:190:use vac_app_server_protocol::ThreadGoal;
vac-rs/app-server/src/vac_message_processor.rs:191:use vac_app_server_protocol::ThreadGoalClearParams;
vac-rs/app-server/src/vac_message_processor.rs:192:use vac_app_server_protocol::ThreadGoalClearResponse;
vac-rs/app-server/src/vac_message_processor.rs:193:use vac_app_server_protocol::ThreadGoalClearedNotification;
vac-rs/app-server/src/vac_message_processor.rs:194:use vac_app_server_protocol::ThreadGoalGetParams;
vac-rs/app-server/src/vac_message_processor.rs:195:use vac_app_server_protocol::ThreadGoalGetResponse;
vac-rs/app-server/src/vac_message_processor.rs:196:use vac_app_server_protocol::ThreadGoalSetParams;
vac-rs/app-server/src/vac_message_processor.rs:197:use vac_app_server_protocol::ThreadGoalSetResponse;
vac-rs/app-server/src/vac_message_processor.rs:198:use vac_app_server_protocol::ThreadGoalStatus;
vac-rs/app-server/src/vac_message_processor.rs:199:use vac_app_server_protocol::ThreadGoalUpdatedNotification;
vac-rs/app-server/src/vac_message_processor.rs:200:use vac_app_server_protocol::ThreadIncrementElicitationParams;
vac-rs/app-server/src/vac_message_processor.rs:201:use vac_app_server_protocol::ThreadIncrementElicitationResponse;
vac-rs/app-server/src/vac_message_processor.rs:202:use vac_app_server_protocol::ThreadInjectItemsParams;
vac-rs/app-server/src/vac_message_processor.rs:203:use vac_app_server_protocol::ThreadInjectItemsResponse;
vac-rs/app-server/src/vac_message_processor.rs:204:use vac_app_server_protocol::ThreadItem;
vac-rs/app-server/src/vac_message_processor.rs:205:use vac_app_server_protocol::ThreadListCwdFilter;
vac-rs/app-server/src/vac_message_processor.rs:206:use vac_app_server_protocol::ThreadListParams;
vac-rs/app-server/src/vac_message_processor.rs:207:use vac_app_server_protocol::ThreadListResponse;
vac-rs/app-server/src/vac_message_processor.rs:208:use vac_app_server_protocol::ThreadLoadedListParams;
vac-rs/app-server/src/vac_message_processor.rs:209:use vac_app_server_protocol::ThreadLoadedListResponse;
vac-rs/app-server/src/vac_message_processor.rs:210:use vac_app_server_protocol::ThreadMemoryModeSetParams;
vac-rs/app-server/src/vac_message_processor.rs:211:use vac_app_server_protocol::ThreadMemoryModeSetResponse;
vac-rs/app-server/src/vac_message_processor.rs:212:use vac_app_server_protocol::ThreadMetadataGitInfoUpdateParams;
vac-rs/app-server/src/vac_message_processor.rs:213:use vac_app_server_protocol::ThreadMetadataUpdateParams;
vac-rs/app-server/src/vac_message_processor.rs:214:use vac_app_server_protocol::ThreadMetadataUpdateResponse;
vac-rs/app-server/src/vac_message_processor.rs:215:use vac_app_server_protocol::ThreadNameUpdatedNotification;
vac-rs/app-server/src/vac_message_processor.rs:216:use vac_app_server_protocol::ThreadReadParams;
vac-rs/app-server/src/vac_message_processor.rs:217:use vac_app_server_protocol::ThreadReadResponse;
vac-rs/app-server/src/vac_message_processor.rs:218:use vac_app_server_protocol::ThreadRealtimeAppendAudioParams;
vac-rs/app-server/src/vac_message_processor.rs:219:use vac_app_server_protocol::ThreadRealtimeAppendAudioResponse;
vac-rs/app-server/src/vac_message_processor.rs:220:use vac_app_server_protocol::ThreadRealtimeAppendTextParams;
vac-rs/app-server/src/vac_message_processor.rs:221:use vac_app_server_protocol::ThreadRealtimeAppendTextResponse;
vac-rs/app-server/src/vac_message_processor.rs:222:use vac_app_server_protocol::ThreadRealtimeListVoicesParams;
vac-rs/app-server/src/vac_message_processor.rs:223:use vac_app_server_protocol::ThreadRealtimeListVoicesResponse;
vac-rs/app-server/src/vac_message_processor.rs:224:use vac_app_server_protocol::ThreadRealtimeStartParams;
vac-rs/app-server/src/vac_message_processor.rs:225:use vac_app_server_protocol::ThreadRealtimeStartResponse;
vac-rs/app-server/src/vac_message_processor.rs:226:use vac_app_server_protocol::ThreadRealtimeStartTransport;
vac-rs/app-server/src/vac_message_processor.rs:227:use vac_app_server_protocol::ThreadRealtimeStopParams;
vac-rs/app-server/src/vac_message_processor.rs:228:use vac_app_server_protocol::ThreadRealtimeStopResponse;
vac-rs/app-server/src/vac_message_processor.rs:229:use vac_app_server_protocol::ThreadResumeParams;
vac-rs/app-server/src/vac_message_processor.rs:230:use vac_app_server_protocol::ThreadResumeResponse;
vac-rs/app-server/src/vac_message_processor.rs:231:use vac_app_server_protocol::ThreadRollbackParams;
vac-rs/app-server/src/vac_message_processor.rs:232:use vac_app_server_protocol::ThreadSetNameParams;
vac-rs/app-server/src/vac_message_processor.rs:233:use vac_app_server_protocol::ThreadSetNameResponse;
vac-rs/app-server/src/vac_message_processor.rs:234:use vac_app_server_protocol::ThreadShellCommandParams;
vac-rs/app-server/src/vac_message_processor.rs:235:use vac_app_server_protocol::ThreadShellCommandResponse;
vac-rs/app-server/src/vac_message_processor.rs:236:use vac_app_server_protocol::ThreadSortKey;
vac-rs/app-server/src/vac_message_processor.rs:237:use vac_app_server_protocol::ThreadSourceKind;
vac-rs/app-server/src/vac_message_processor.rs:238:use vac_app_server_protocol::ThreadStartParams;
vac-rs/app-server/src/vac_message_processor.rs:239:use vac_app_server_protocol::ThreadStartResponse;
vac-rs/app-server/src/vac_message_processor.rs:240:use vac_app_server_protocol::ThreadStartedNotification;
vac-rs/app-server/src/vac_message_processor.rs:241:use vac_app_server_protocol::ThreadStatus;
vac-rs/app-server/src/vac_message_processor.rs:242:use vac_app_server_protocol::ThreadTurnsListParams;
vac-rs/app-server/src/vac_message_processor.rs:243:use vac_app_server_protocol::ThreadTurnsListResponse;
vac-rs/app-server/src/vac_message_processor.rs:244:use vac_app_server_protocol::ThreadUnarchiveParams;
vac-rs/app-server/src/vac_message_processor.rs:245:use vac_app_server_protocol::ThreadUnarchiveResponse;
vac-rs/app-server/src/vac_message_processor.rs:246:use vac_app_server_protocol::ThreadUnarchivedNotification;
vac-rs/app-server/src/vac_message_processor.rs:247:use vac_app_server_protocol::ThreadUnsubscribeParams;
vac-rs/app-server/src/vac_message_processor.rs:248:use vac_app_server_protocol::ThreadUnsubscribeResponse;
vac-rs/app-server/src/vac_message_processor.rs:249:use vac_app_server_protocol::ThreadUnsubscribeStatus;
vac-rs/app-server/src/vac_message_processor.rs:250:use vac_app_server_protocol::Turn;
vac-rs/app-server/src/vac_message_processor.rs:251:use vac_app_server_protocol::TurnEnvironmentParams;
vac-rs/app-server/src/vac_message_processor.rs:252:use vac_app_server_protocol::TurnError;
vac-rs/app-server/src/vac_message_processor.rs:253:use vac_app_server_protocol::TurnInterruptParams;
vac-rs/app-server/src/vac_message_processor.rs:254:use vac_app_server_protocol::TurnInterruptResponse;
vac-rs/app-server/src/vac_message_processor.rs:255:use vac_app_server_protocol::TurnStartParams;
vac-rs/app-server/src/vac_message_processor.rs:256:use vac_app_server_protocol::TurnStartResponse;
vac-rs/app-server/src/vac_message_processor.rs:257:use vac_app_server_protocol::TurnStatus;
vac-rs/app-server/src/vac_message_processor.rs:258:use vac_app_server_protocol::TurnSteerParams;
vac-rs/app-server/src/vac_message_processor.rs:259:use vac_app_server_protocol::TurnSteerResponse;
vac-rs/app-server/src/vac_message_processor.rs:260:use vac_app_server_protocol::UserInput as V2UserInput;
vac-rs/app-server/src/vac_message_processor.rs:261:use vac_app_server_protocol::VACErrorInfo;
vac-rs/app-server/src/vac_message_processor.rs:262:use vac_app_server_protocol::WindowsSandboxSetupCompletedNotification;
vac-rs/app-server/src/vac_message_processor.rs:263:use vac_app_server_protocol::WindowsSandboxSetupMode;
vac-rs/app-server/src/vac_message_processor.rs:264:use vac_app_server_protocol::WindowsSandboxSetupStartParams;
vac-rs/app-server/src/vac_message_processor.rs:265:use vac_app_server_protocol::WindowsSandboxSetupStartResponse;
vac-rs/app-server/src/vac_message_processor.rs:266:use vac_app_server_protocol::build_turns_from_rollout_items;
vac-rs/app-server/src/vac_message_processor.rs:421:use vac_app_server_protocol::ServerRequest;
vac-rs/app-server/src/vac_message_processor.rs:2692:        session_start_source: Option<vac_app_server_protocol::ThreadStartSource>,
vac-rs/app-server/src/vac_message_processor.rs:2806:                        .unwrap_or(vac_app_server_protocol::ThreadStartSource::Startup)
vac-rs/app-server/src/vac_message_processor.rs:2808:                        vac_app_server_protocol::ThreadStartSource::Startup => {
vac-rs/app-server/src/vac_message_processor.rs:2811:                        vac_app_server_protocol::ThreadStartSource::Clear => {
vac-rs/app-server/src/vac_message_processor.rs:2956:        approval_policy: Option<vac_app_server_protocol::AskForApproval>,
vac-rs/app-server/src/vac_message_processor.rs:2957:        approvals_reviewer: Option<vac_app_server_protocol::ApprovalsReviewer>,
vac-rs/app-server/src/vac_message_processor.rs:2969:            approval_policy: approval_policy.map(vac_app_server_protocol::AskForApproval::to_core),
vac-rs/app-server/src/vac_message_processor.rs:2971:                .map(vac_app_server_protocol::ApprovalsReviewer::to_core),
vac-rs/app-server/src/vac_message_processor.rs:6382:                    data.push(vac_app_server_protocol::SkillsListEntry {
vac-rs/app-server/src/vac_message_processor.rs:6385:                        errors: vec![vac_app_server_protocol::SkillErrorInfo {
vac-rs/app-server/src/vac_message_processor.rs:6420:            data.push(vac_app_server_protocol::SkillsListEntry {
vac-rs/app-server/src/vac_message_processor.rs:6462:                    data.push(vac_app_server_protocol::HooksListEntry {
vac-rs/app-server/src/vac_message_processor.rs:6466:                        errors: vec![vac_app_server_protocol::HookErrorInfo {
vac-rs/app-server/src/vac_message_processor.rs:6499:            data.push(vac_app_server_protocol::HooksListEntry {
vac-rs/app-server/src/vac_message_processor.rs:6710:                .map(vac_app_server_protocol::ApprovalsReviewer::to_core);
vac-rs/app-server/src/vac_message_processor.rs:8263:    use vac_app_server_protocol::ThreadListCwdFilter;
vac-rs/app-server/src/vac_message_processor.rs:8673:        let active_review_policy: vac_app_server_protocol::ApprovalsReviewer =
vac-rs/app-server/src/vac_message_processor.rs:8776:) -> Vec<vac_app_server_protocol::SkillMetadata> {
vac-rs/app-server/src/vac_message_processor.rs:8781:            vac_app_server_protocol::SkillMetadata {
vac-rs/app-server/src/vac_message_processor.rs:8786:                    vac_app_server_protocol::SkillInterface {
vac-rs/app-server/src/vac_message_processor.rs:8796:                    vac_app_server_protocol::SkillDependencies {
vac-rs/app-server/src/vac_message_processor.rs:8800:                            .map(|tool| vac_app_server_protocol::SkillToolDependency {
vac-rs/app-server/src/vac_message_processor.rs:8851:                vac_app_server_protocol::SkillInterface {
vac-rs/app-server/src/vac_message_processor.rs:8907:) -> Vec<vac_app_server_protocol::SkillErrorInfo> {
vac-rs/app-server/src/vac_message_processor.rs:8910:        .map(|err| vac_app_server_protocol::SkillErrorInfo {
vac-rs/app-server/src/vac_message_processor.rs:9616:) -> Option<vac_app_server_protocol::ActivePermissionProfile> {
vac-rs/app-server/src/vac_message_processor.rs:9642:) -> vac_app_server_protocol::SandboxPolicy {
vac-rs/app-server/src/vac_message_processor.rs:10001:    use vac_app_server_protocol::ServerRequestPayload;
vac-rs/app-server/src/vac_message_processor.rs:10002:    use vac_app_server_protocol::ToolRequestUserInputParams;
vac-rs/app-server/src/transport.rs:12:use vac_app_server_protocol::ExperimentalApi;
vac-rs/app-server/src/transport.rs:13:use vac_app_server_protocol::ServerRequest;
vac-rs/app-server/src/thread_status.rs:12:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/thread_status.rs:13:use vac_app_server_protocol::Thread;
vac-rs/app-server/src/thread_status.rs:14:use vac_app_server_protocol::ThreadActiveFlag;
vac-rs/app-server/src/thread_status.rs:15:use vac_app_server_protocol::ThreadStatus;
vac-rs/app-server/src/thread_status.rs:16:use vac_app_server_protocol::ThreadStatusChangedNotification;
vac-rs/app-server/src/thread_status.rs:483:                vac_app_server_protocol::SessionSource::AppServer,
vac-rs/app-server/src/thread_status.rs:505:                vac_app_server_protocol::SessionSource::Cli,
vac-rs/app-server/src/thread_status.rs:618:                vac_app_server_protocol::SessionSource::Cli,
vac-rs/app-server/src/thread_status.rs:649:                vac_app_server_protocol::SessionSource::Cli,
vac-rs/app-server/src/thread_status.rs:670:                vac_app_server_protocol::SessionSource::Cli,
vac-rs/app-server/src/thread_status.rs:700:                vac_app_server_protocol::SessionSource::Cli,
vac-rs/app-server/src/thread_status.rs:731:                vac_app_server_protocol::SessionSource::Cli,
vac-rs/app-server/src/thread_status.rs:774:                vac_app_server_protocol::SessionSource::Cli,
vac-rs/app-server/src/thread_status.rs:809:                vac_app_server_protocol::SessionSource::Cli,
vac-rs/app-server/src/thread_status.rs:815:                vac_app_server_protocol::SessionSource::AppServer,
vac-rs/app-server/src/thread_status.rs:889:    fn test_thread(thread_id: &str, source: vac_app_server_protocol::SessionSource) -> Thread {
vac-rs/app-server/src/request_serialization.rs:10:use vac_app_server_protocol::ClientRequestSerializationScope;
vac-rs/app-server/src/config_manager_service_tests.rs:6:use vac_app_server_protocol::AppConfig;
vac-rs/app-server/src/config_manager_service_tests.rs:7:use vac_app_server_protocol::AppToolApproval;
vac-rs/app-server/src/config_manager_service_tests.rs:8:use vac_app_server_protocol::AppsConfig;
vac-rs/app-server/src/config_manager_service_tests.rs:9:use vac_app_server_protocol::AskForApproval;
vac-rs/app-server/src/in_process.rs:78:use vac_app_server_protocol::ClientNotification;
vac-rs/app-server/src/in_process.rs:79:use vac_app_server_protocol::ClientRequest;
vac-rs/app-server/src/in_process.rs:80:use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server/src/in_process.rs:81:use vac_app_server_protocol::ExternalAgentConfigDetectParams;
vac-rs/app-server/src/in_process.rs:82:use vac_app_server_protocol::ExternalAgentConfigDetectResponse;
vac-rs/app-server/src/in_process.rs:83:use vac_app_server_protocol::InitializeParams;
vac-rs/app-server/src/in_process.rs:84:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/in_process.rs:85:use vac_app_server_protocol::RequestId;
vac-rs/app-server/src/in_process.rs:86:use vac_app_server_protocol::Result;
vac-rs/app-server/src/in_process.rs:87:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/in_process.rs:88:use vac_app_server_protocol::ServerRequest;
vac-rs/app-server/src/in_process.rs:800:    use vac_app_server_protocol::ClientInfo;
vac-rs/app-server/src/in_process.rs:801:    use vac_app_server_protocol::ConfigRequirementsReadResponse;
vac-rs/app-server/src/in_process.rs:802:    use vac_app_server_protocol::DeviceKeyPublicParams;
vac-rs/app-server/src/in_process.rs:803:    use vac_app_server_protocol::DeviceKeySignParams;
vac-rs/app-server/src/in_process.rs:804:    use vac_app_server_protocol::DeviceKeySignPayload;
vac-rs/app-server/src/in_process.rs:805:    use vac_app_server_protocol::RemoteControlClientConnectionAudience;
vac-rs/app-server/src/in_process.rs:806:    use vac_app_server_protocol::RemoteControlClientEnrollmentAudience;
vac-rs/app-server/src/in_process.rs:807:    use vac_app_server_protocol::SessionSource as ApiSessionSource;
vac-rs/app-server/src/in_process.rs:808:    use vac_app_server_protocol::ThreadStartParams;
vac-rs/app-server/src/in_process.rs:809:    use vac_app_server_protocol::ThreadStartResponse;
vac-rs/app-server/src/in_process.rs:810:    use vac_app_server_protocol::Turn;
vac-rs/app-server/src/in_process.rs:811:    use vac_app_server_protocol::TurnCompletedNotification;
vac-rs/app-server/src/in_process.rs:812:    use vac_app_server_protocol::TurnStatus;
vac-rs/app-server/src/thread_state.rs:12:use vac_app_server_protocol::RequestId;
vac-rs/app-server/src/thread_state.rs:13:use vac_app_server_protocol::ThreadGoal;
vac-rs/app-server/src/thread_state.rs:14:use vac_app_server_protocol::ThreadHistoryBuilder;
vac-rs/app-server/src/thread_state.rs:15:use vac_app_server_protocol::Turn;
vac-rs/app-server/src/thread_state.rs:16:use vac_app_server_protocol::TurnError;
vac-rs/app-server/src/thread_state.rs:32:    pub(crate) thread_summary: vac_app_server_protocol::Thread,
vac-rs/app-server/src/config_manager_service.rs:10:use vac_app_server_protocol::Config as ApiConfig;
vac-rs/app-server/src/config_manager_service.rs:11:use vac_app_server_protocol::ConfigBatchWriteParams;
vac-rs/app-server/src/config_manager_service.rs:12:use vac_app_server_protocol::ConfigLayerMetadata;
vac-rs/app-server/src/config_manager_service.rs:13:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/app-server/src/config_manager_service.rs:14:use vac_app_server_protocol::ConfigReadParams;
vac-rs/app-server/src/config_manager_service.rs:15:use vac_app_server_protocol::ConfigReadResponse;
vac-rs/app-server/src/config_manager_service.rs:16:use vac_app_server_protocol::ConfigValueWriteParams;
vac-rs/app-server/src/config_manager_service.rs:17:use vac_app_server_protocol::ConfigWriteErrorCode;
vac-rs/app-server/src/config_manager_service.rs:18:use vac_app_server_protocol::ConfigWriteResponse;
vac-rs/app-server/src/config_manager_service.rs:19:use vac_app_server_protocol::MergeStrategy;
vac-rs/app-server/src/config_manager_service.rs:20:use vac_app_server_protocol::OverriddenMetadata;
vac-rs/app-server/src/config_manager_service.rs:21:use vac_app_server_protocol::WriteStatus;
vac-rs/app-server/src/fs_api.rs:7:use vac_app_server_protocol::FsCopyParams;
vac-rs/app-server/src/fs_api.rs:8:use vac_app_server_protocol::FsCopyResponse;
vac-rs/app-server/src/fs_api.rs:9:use vac_app_server_protocol::FsCreateDirectoryParams;
vac-rs/app-server/src/fs_api.rs:10:use vac_app_server_protocol::FsCreateDirectoryResponse;
vac-rs/app-server/src/fs_api.rs:11:use vac_app_server_protocol::FsGetMetadataParams;
vac-rs/app-server/src/fs_api.rs:12:use vac_app_server_protocol::FsGetMetadataResponse;
vac-rs/app-server/src/fs_api.rs:13:use vac_app_server_protocol::FsReadDirectoryEntry;
vac-rs/app-server/src/fs_api.rs:14:use vac_app_server_protocol::FsReadDirectoryParams;
vac-rs/app-server/src/fs_api.rs:15:use vac_app_server_protocol::FsReadDirectoryResponse;
vac-rs/app-server/src/fs_api.rs:16:use vac_app_server_protocol::FsReadFileParams;
vac-rs/app-server/src/fs_api.rs:17:use vac_app_server_protocol::FsReadFileResponse;
vac-rs/app-server/src/fs_api.rs:18:use vac_app_server_protocol::FsRemoveParams;
vac-rs/app-server/src/fs_api.rs:19:use vac_app_server_protocol::FsRemoveResponse;
vac-rs/app-server/src/fs_api.rs:20:use vac_app_server_protocol::FsWriteFileParams;
vac-rs/app-server/src/fs_api.rs:21:use vac_app_server_protocol::FsWriteFileResponse;
vac-rs/app-server/src/fs_api.rs:22:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/message_processor.rs:38:use vac_app_server_protocol::AppListUpdatedNotification;
vac-rs/app-server/src/message_processor.rs:39:use vac_app_server_protocol::AuthMode as LoginAuthMode;
vac-rs/app-server/src/message_processor.rs:40:use vac_app_server_protocol::ChatgptAuthTokensRefreshParams;
vac-rs/app-server/src/message_processor.rs:41:use vac_app_server_protocol::ChatgptAuthTokensRefreshReason;
vac-rs/app-server/src/message_processor.rs:42:use vac_app_server_protocol::ChatgptAuthTokensRefreshResponse;
vac-rs/app-server/src/message_processor.rs:43:use vac_app_server_protocol::ClientInfo;
vac-rs/app-server/src/message_processor.rs:44:use vac_app_server_protocol::ClientNotification;
vac-rs/app-server/src/message_processor.rs:45:use vac_app_server_protocol::ClientRequest;
vac-rs/app-server/src/message_processor.rs:46:use vac_app_server_protocol::ClientResponsePayload;
vac-rs/app-server/src/message_processor.rs:47:use vac_app_server_protocol::ConfigBatchWriteParams;
vac-rs/app-server/src/message_processor.rs:48:use vac_app_server_protocol::ConfigValueWriteParams;
vac-rs/app-server/src/message_processor.rs:49:use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server/src/message_processor.rs:50:use vac_app_server_protocol::DeviceKeyCreateParams;
vac-rs/app-server/src/message_processor.rs:51:use vac_app_server_protocol::DeviceKeyPublicParams;
vac-rs/app-server/src/message_processor.rs:52:use vac_app_server_protocol::DeviceKeySignParams;
vac-rs/app-server/src/message_processor.rs:53:use vac_app_server_protocol::ExperimentalApi;
vac-rs/app-server/src/message_processor.rs:54:use vac_app_server_protocol::ExperimentalFeatureEnablementSetParams;
vac-rs/app-server/src/message_processor.rs:55:use vac_app_server_protocol::ExternalAgentConfigImportCompletedNotification;
vac-rs/app-server/src/message_processor.rs:56:use vac_app_server_protocol::ExternalAgentConfigImportParams;
vac-rs/app-server/src/message_processor.rs:57:use vac_app_server_protocol::ExternalAgentConfigImportResponse;
vac-rs/app-server/src/message_processor.rs:58:use vac_app_server_protocol::ExternalAgentConfigMigrationItem;
vac-rs/app-server/src/message_processor.rs:59:use vac_app_server_protocol::ExternalAgentConfigMigrationItemType;
vac-rs/app-server/src/message_processor.rs:60:use vac_app_server_protocol::InitializeResponse;
vac-rs/app-server/src/message_processor.rs:61:use vac_app_server_protocol::JSONRPCError;
vac-rs/app-server/src/message_processor.rs:62:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/message_processor.rs:63:use vac_app_server_protocol::JSONRPCNotification;
vac-rs/app-server/src/message_processor.rs:64:use vac_app_server_protocol::JSONRPCRequest;
vac-rs/app-server/src/message_processor.rs:65:use vac_app_server_protocol::JSONRPCResponse;
vac-rs/app-server/src/message_processor.rs:66:use vac_app_server_protocol::ModelProviderCapabilitiesReadResponse;
vac-rs/app-server/src/message_processor.rs:67:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/message_processor.rs:68:use vac_app_server_protocol::ServerRequestPayload;
vac-rs/app-server/src/message_processor.rs:69:use vac_app_server_protocol::experimental_required_message;
vac-rs/app-server/src/error_code.rs:1:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/server_request_error.rs:1:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/server_request_error.rs:19:    use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/message_processor/tracing_tests.rs:33:use vac_app_server_protocol::ClientInfo;
vac-rs/app-server/src/message_processor/tracing_tests.rs:34:use vac_app_server_protocol::ClientRequest;
vac-rs/app-server/src/message_processor/tracing_tests.rs:35:use vac_app_server_protocol::DeviceKeySignParams;
vac-rs/app-server/src/message_processor/tracing_tests.rs:36:use vac_app_server_protocol::DeviceKeySignPayload;
vac-rs/app-server/src/message_processor/tracing_tests.rs:37:use vac_app_server_protocol::InitializeCapabilities;
vac-rs/app-server/src/message_processor/tracing_tests.rs:38:use vac_app_server_protocol::InitializeParams;
vac-rs/app-server/src/message_processor/tracing_tests.rs:39:use vac_app_server_protocol::InitializeResponse;
vac-rs/app-server/src/message_processor/tracing_tests.rs:40:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/message_processor/tracing_tests.rs:41:use vac_app_server_protocol::JSONRPCRequest;
vac-rs/app-server/src/message_processor/tracing_tests.rs:42:use vac_app_server_protocol::RemoteControlClientConnectionAudience;
vac-rs/app-server/src/message_processor/tracing_tests.rs:43:use vac_app_server_protocol::RequestId;
vac-rs/app-server/src/message_processor/tracing_tests.rs:44:use vac_app_server_protocol::ThreadStartParams;
vac-rs/app-server/src/message_processor/tracing_tests.rs:45:use vac_app_server_protocol::ThreadStartResponse;
vac-rs/app-server/src/message_processor/tracing_tests.rs:46:use vac_app_server_protocol::TurnStartParams;
vac-rs/app-server/src/message_processor/tracing_tests.rs:47:use vac_app_server_protocol::TurnStartResponse;
vac-rs/app-server/src/message_processor/tracing_tests.rs:48:use vac_app_server_protocol::UserInput;
vac-rs/app-server/src/message_processor/tracing_tests.rs:550:                    vac_app_server_protocol::ServerNotification::ThreadStarted(_)
vac-rs/app-server/src/message_processor/tracing_tests.rs:563:                    vac_app_server_protocol::ServerNotification::ThreadStarted(_)
vac-rs/app-server/src/transport_tests.rs:6:use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server/src/transport_tests.rs:7:use vac_app_server_protocol::RequestId;
vac-rs/app-server/src/transport_tests.rs:8:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/transport_tests.rs:9:use vac_app_server_protocol::ThreadGoal;
vac-rs/app-server/src/transport_tests.rs:10:use vac_app_server_protocol::ThreadGoalStatus;
vac-rs/app-server/src/transport_tests.rs:11:use vac_app_server_protocol::ThreadGoalUpdatedNotification;
vac-rs/app-server/src/transport_tests.rs:257:                params: vac_app_server_protocol::CommandExecutionRequestApprovalParams {
vac-rs/app-server/src/transport_tests.rs:268:                        vac_app_server_protocol::AdditionalPermissionProfile {
vac-rs/app-server/src/transport_tests.rs:271:                                vac_app_server_protocol::AdditionalFileSystemPermissions {
vac-rs/app-server/src/transport_tests.rs:321:                params: vac_app_server_protocol::CommandExecutionRequestApprovalParams {
vac-rs/app-server/src/transport_tests.rs:332:                        vac_app_server_protocol::AdditionalPermissionProfile {
vac-rs/app-server/src/transport_tests.rs:335:                                vac_app_server_protocol::AdditionalFileSystemPermissions {
vac-rs/app-server/src/filters.rs:1:use vac_app_server_protocol::ThreadSourceKind;
vac-rs/app-server/src/external_agent_config_api.rs:13:use vac_app_server_protocol::CommandMigration;
vac-rs/app-server/src/external_agent_config_api.rs:14:use vac_app_server_protocol::ExternalAgentConfigDetectParams;
vac-rs/app-server/src/external_agent_config_api.rs:15:use vac_app_server_protocol::ExternalAgentConfigDetectResponse;
vac-rs/app-server/src/external_agent_config_api.rs:16:use vac_app_server_protocol::ExternalAgentConfigImportParams;
vac-rs/app-server/src/external_agent_config_api.rs:17:use vac_app_server_protocol::ExternalAgentConfigMigrationItem;
vac-rs/app-server/src/external_agent_config_api.rs:18:use vac_app_server_protocol::ExternalAgentConfigMigrationItemType;
vac-rs/app-server/src/external_agent_config_api.rs:19:use vac_app_server_protocol::HookMigration;
vac-rs/app-server/src/external_agent_config_api.rs:20:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/external_agent_config_api.rs:21:use vac_app_server_protocol::McpServerMigration;
vac-rs/app-server/src/external_agent_config_api.rs:22:use vac_app_server_protocol::MigrationDetails;
vac-rs/app-server/src/external_agent_config_api.rs:23:use vac_app_server_protocol::PluginsMigration;
vac-rs/app-server/src/external_agent_config_api.rs:24:use vac_app_server_protocol::SubagentMigration;
vac-rs/app-server/src/external_agent_config_api.rs:105:                            .map(|session| vac_app_server_protocol::SessionMigration {
vac-rs/app-server/src/config_api.rs:12:use vac_app_server_protocol::ConfigBatchWriteParams;
vac-rs/app-server/src/config_api.rs:13:use vac_app_server_protocol::ConfigReadParams;
vac-rs/app-server/src/config_api.rs:14:use vac_app_server_protocol::ConfigReadResponse;
vac-rs/app-server/src/config_api.rs:15:use vac_app_server_protocol::ConfigRequirements;
vac-rs/app-server/src/config_api.rs:16:use vac_app_server_protocol::ConfigRequirementsReadResponse;
vac-rs/app-server/src/config_api.rs:17:use vac_app_server_protocol::ConfigValueWriteParams;
vac-rs/app-server/src/config_api.rs:18:use vac_app_server_protocol::ConfigWriteErrorCode;
vac-rs/app-server/src/config_api.rs:19:use vac_app_server_protocol::ConfigWriteResponse;
vac-rs/app-server/src/config_api.rs:20:use vac_app_server_protocol::ConfiguredHookHandler;
vac-rs/app-server/src/config_api.rs:21:use vac_app_server_protocol::ConfiguredHookMatcherGroup;
vac-rs/app-server/src/config_api.rs:22:use vac_app_server_protocol::ExperimentalFeatureEnablementSetParams;
vac-rs/app-server/src/config_api.rs:23:use vac_app_server_protocol::ExperimentalFeatureEnablementSetResponse;
vac-rs/app-server/src/config_api.rs:24:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/config_api.rs:25:use vac_app_server_protocol::ManagedHooksRequirements;
vac-rs/app-server/src/config_api.rs:26:use vac_app_server_protocol::NetworkDomainPermission;
vac-rs/app-server/src/config_api.rs:27:use vac_app_server_protocol::NetworkRequirements;
vac-rs/app-server/src/config_api.rs:28:use vac_app_server_protocol::NetworkUnixSocketPermission;
vac-rs/app-server/src/config_api.rs:29:use vac_app_server_protocol::SandboxMode;
vac-rs/app-server/src/config_api.rs:262:                .map(vac_app_server_protocol::AskForApproval::from)
vac-rs/app-server/src/config_api.rs:268:                .map(vac_app_server_protocol::ApprovalsReviewer::from)
vac-rs/app-server/src/config_api.rs:374:) -> vac_app_server_protocol::ResidencyRequirement {
vac-rs/app-server/src/config_api.rs:376:        CoreResidencyRequirement::Us => vac_app_server_protocol::ResidencyRequirement::Us,
vac-rs/app-server/src/config_api.rs:584:                vac_app_server_protocol::AskForApproval::Never,
vac-rs/app-server/src/config_api.rs:585:                vac_app_server_protocol::AskForApproval::OnRequest,
vac-rs/app-server/src/config_api.rs:591:                vac_app_server_protocol::ApprovalsReviewer::User,
vac-rs/app-server/src/config_api.rs:592:                vac_app_server_protocol::ApprovalsReviewer::AutoReview,
vac-rs/app-server/src/config_api.rs:633:            Some(vac_app_server_protocol::ResidencyRequirement::Us),
vac-rs/app-server/src/config_api.rs:834:                edits: vec![vac_app_server_protocol::ConfigEdit {
vac-rs/app-server/src/config_api.rs:837:                    merge_strategy: vac_app_server_protocol::MergeStrategy::Replace,
vac-rs/app-server/src/config_api.rs:849:                status: vac_app_server_protocol::WriteStatus::Ok,
vac-rs/app-server/src/vac_message_processor/plugin_mcp_oauth.rs:5:use vac_app_server_protocol::McpServerOauthLoginCompletedNotification;
vac-rs/app-server/src/vac_message_processor/plugin_mcp_oauth.rs:6:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/vac_message_processor/apps_list_helpers.rs:3:use vac_app_server_protocol::AppInfo;
vac-rs/app-server/src/vac_message_processor/apps_list_helpers.rs:4:use vac_app_server_protocol::AppListUpdatedNotification;
vac-rs/app-server/src/vac_message_processor/apps_list_helpers.rs:5:use vac_app_server_protocol::AppsListResponse;
vac-rs/app-server/src/vac_message_processor/apps_list_helpers.rs:6:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/vac_message_processor/apps_list_helpers.rs:7:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/vac_message_processor/plugins.rs:4:use vac_app_server_protocol::PluginAvailability;
vac-rs/app-server/src/vac_message_processor/plugins.rs:5:use vac_app_server_protocol::PluginInstallPolicy;
vac-rs/app-server/src/vac_message_processor/plugins.rs:59:                    Vec<vac_app_server_protocol::MarketplaceLoadErrorInfo>,
vac-rs/app-server/src/vac_message_processor/plugins.rs:92:                    .map(|err| vac_app_server_protocol::MarketplaceLoadErrorInfo {
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs:15:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs:16:use vac_app_server_protocol::Thread;
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs:17:use vac_app_server_protocol::ThreadHistoryBuilder;
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs:18:use vac_app_server_protocol::ThreadTokenUsage;
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs:19:use vac_app_server_protocol::ThreadTokenUsageUpdatedNotification;
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs:20:use vac_app_server_protocol::Turn;
vac-rs/app-server/src/vac_message_processor/token_usage_replay.rs:21:use vac_app_server_protocol::TurnStatus;
vac-rs/app-server/src/vac_message_processor/plugin_app_helpers.rs:4:use vac_app_server_protocol::AppInfo;
vac-rs/app-server/src/vac_message_processor/plugin_app_helpers.rs:5:use vac_app_server_protocol::AppSummary;
vac-rs/app-server/src/vac_message_processor/plugin_app_helpers.rs:116:    use vac_app_server_protocol::AppInfo;
vac-rs/app-server/src/dynamic_tools.rs:4:use vac_app_server_protocol::DynamicToolCallOutputContentItem;
vac-rs/app-server/src/dynamic_tools.rs:5:use vac_app_server_protocol::DynamicToolCallResponse;
vac-rs/app-server/src/bespoke_event_handling.rs:22:use vac_app_server_protocol::AccountRateLimitsUpdatedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:23:use vac_app_server_protocol::AdditionalPermissionProfile as V2AdditionalPermissionProfile;
vac-rs/app-server/src/bespoke_event_handling.rs:24:use vac_app_server_protocol::CommandAction as V2ParsedCommand;
vac-rs/app-server/src/bespoke_event_handling.rs:25:use vac_app_server_protocol::CommandExecutionApprovalDecision;
vac-rs/app-server/src/bespoke_event_handling.rs:26:use vac_app_server_protocol::CommandExecutionRequestApprovalParams;
vac-rs/app-server/src/bespoke_event_handling.rs:27:use vac_app_server_protocol::CommandExecutionRequestApprovalResponse;
vac-rs/app-server/src/bespoke_event_handling.rs:28:use vac_app_server_protocol::CommandExecutionSource;
vac-rs/app-server/src/bespoke_event_handling.rs:29:use vac_app_server_protocol::CommandExecutionStatus;
vac-rs/app-server/src/bespoke_event_handling.rs:30:use vac_app_server_protocol::DeprecationNoticeNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:31:use vac_app_server_protocol::DynamicToolCallParams;
vac-rs/app-server/src/bespoke_event_handling.rs:32:use vac_app_server_protocol::DynamicToolCallStatus;
vac-rs/app-server/src/bespoke_event_handling.rs:33:use vac_app_server_protocol::ErrorNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:34:use vac_app_server_protocol::ExecPolicyAmendment as V2ExecPolicyAmendment;
vac-rs/app-server/src/bespoke_event_handling.rs:35:use vac_app_server_protocol::FileChangeApprovalDecision;
vac-rs/app-server/src/bespoke_event_handling.rs:36:use vac_app_server_protocol::FileChangeRequestApprovalParams;
vac-rs/app-server/src/bespoke_event_handling.rs:37:use vac_app_server_protocol::FileChangeRequestApprovalResponse;
vac-rs/app-server/src/bespoke_event_handling.rs:38:use vac_app_server_protocol::GrantedPermissionProfile as V2GrantedPermissionProfile;
vac-rs/app-server/src/bespoke_event_handling.rs:39:use vac_app_server_protocol::GuardianWarningNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:40:use vac_app_server_protocol::HookCompletedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:41:use vac_app_server_protocol::HookStartedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:42:use vac_app_server_protocol::ItemCompletedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:43:use vac_app_server_protocol::ItemStartedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:44:use vac_app_server_protocol::McpServerElicitationAction;
vac-rs/app-server/src/bespoke_event_handling.rs:45:use vac_app_server_protocol::McpServerElicitationRequestParams;
vac-rs/app-server/src/bespoke_event_handling.rs:46:use vac_app_server_protocol::McpServerElicitationRequestResponse;
vac-rs/app-server/src/bespoke_event_handling.rs:47:use vac_app_server_protocol::McpServerStartupState;
vac-rs/app-server/src/bespoke_event_handling.rs:48:use vac_app_server_protocol::McpServerStatusUpdatedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:49:use vac_app_server_protocol::ModelReroutedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:50:use vac_app_server_protocol::ModelVerificationNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:51:use vac_app_server_protocol::NetworkApprovalContext as V2NetworkApprovalContext;
vac-rs/app-server/src/bespoke_event_handling.rs:52:use vac_app_server_protocol::NetworkPolicyAmendment as V2NetworkPolicyAmendment;
vac-rs/app-server/src/bespoke_event_handling.rs:53:use vac_app_server_protocol::NetworkPolicyRuleAction as V2NetworkPolicyRuleAction;
vac-rs/app-server/src/bespoke_event_handling.rs:54:use vac_app_server_protocol::PermissionsRequestApprovalParams;
vac-rs/app-server/src/bespoke_event_handling.rs:55:use vac_app_server_protocol::PermissionsRequestApprovalResponse;
vac-rs/app-server/src/bespoke_event_handling.rs:56:use vac_app_server_protocol::RawResponseItemCompletedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:57:use vac_app_server_protocol::RequestId;
vac-rs/app-server/src/bespoke_event_handling.rs:58:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:59:use vac_app_server_protocol::ServerRequestPayload;
vac-rs/app-server/src/bespoke_event_handling.rs:60:use vac_app_server_protocol::SkillsChangedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:61:use vac_app_server_protocol::ThreadGoalUpdatedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:62:use vac_app_server_protocol::ThreadItem;
vac-rs/app-server/src/bespoke_event_handling.rs:63:use vac_app_server_protocol::ThreadNameUpdatedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:64:use vac_app_server_protocol::ThreadRealtimeClosedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:65:use vac_app_server_protocol::ThreadRealtimeErrorNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:66:use vac_app_server_protocol::ThreadRealtimeItemAddedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:67:use vac_app_server_protocol::ThreadRealtimeOutputAudioDeltaNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:68:use vac_app_server_protocol::ThreadRealtimeSdpNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:69:use vac_app_server_protocol::ThreadRealtimeStartedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:70:use vac_app_server_protocol::ThreadRealtimeTranscriptDeltaNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:71:use vac_app_server_protocol::ThreadRealtimeTranscriptDoneNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:72:use vac_app_server_protocol::ThreadRollbackResponse;
vac-rs/app-server/src/bespoke_event_handling.rs:73:use vac_app_server_protocol::ThreadTokenUsage;
vac-rs/app-server/src/bespoke_event_handling.rs:74:use vac_app_server_protocol::ThreadTokenUsageUpdatedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:75:use vac_app_server_protocol::ToolRequestUserInputOption;
vac-rs/app-server/src/bespoke_event_handling.rs:76:use vac_app_server_protocol::ToolRequestUserInputParams;
vac-rs/app-server/src/bespoke_event_handling.rs:77:use vac_app_server_protocol::ToolRequestUserInputQuestion;
vac-rs/app-server/src/bespoke_event_handling.rs:78:use vac_app_server_protocol::ToolRequestUserInputResponse;
vac-rs/app-server/src/bespoke_event_handling.rs:79:use vac_app_server_protocol::Turn;
vac-rs/app-server/src/bespoke_event_handling.rs:80:use vac_app_server_protocol::TurnCompletedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:81:use vac_app_server_protocol::TurnDiffUpdatedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:82:use vac_app_server_protocol::TurnError;
vac-rs/app-server/src/bespoke_event_handling.rs:83:use vac_app_server_protocol::TurnInterruptResponse;
vac-rs/app-server/src/bespoke_event_handling.rs:84:use vac_app_server_protocol::TurnPlanStep;
vac-rs/app-server/src/bespoke_event_handling.rs:85:use vac_app_server_protocol::TurnPlanUpdatedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:86:use vac_app_server_protocol::TurnStartedNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:87:use vac_app_server_protocol::TurnStatus;
vac-rs/app-server/src/bespoke_event_handling.rs:88:use vac_app_server_protocol::VACErrorInfo as V2VACErrorInfo;
vac-rs/app-server/src/bespoke_event_handling.rs:89:use vac_app_server_protocol::WarningNotification;
vac-rs/app-server/src/bespoke_event_handling.rs:90:use vac_app_server_protocol::build_item_from_guardian_event;
vac-rs/app-server/src/bespoke_event_handling.rs:91:use vac_app_server_protocol::build_turns_from_rollout_items;
vac-rs/app-server/src/bespoke_event_handling.rs:92:use vac_app_server_protocol::guardian_auto_approval_review_notification;
vac-rs/app-server/src/bespoke_event_handling.rs:93:use vac_app_server_protocol::item_event_to_server_notification;
vac-rs/app-server/src/bespoke_event_handling.rs:1477:                .map(vac_app_server_protocol::HookPromptFragment::from)
vac-rs/app-server/src/bespoke_event_handling.rs:1842:                scope: vac_app_server_protocol::PermissionGrantScope::Turn,
vac-rs/app-server/src/bespoke_event_handling.rs:1850:            vac_app_server_protocol::PermissionGrantScope::Session
vac-rs/app-server/src/bespoke_event_handling.rs:2093:    use vac_app_server_protocol::AutoReviewDecisionSource;
vac-rs/app-server/src/bespoke_event_handling.rs:2094:    use vac_app_server_protocol::GuardianApprovalReviewStatus;
vac-rs/app-server/src/bespoke_event_handling.rs:2095:    use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/bespoke_event_handling.rs:2096:    use vac_app_server_protocol::TurnPlanStepStatus;
vac-rs/app-server/src/bespoke_event_handling.rs:2327:                    Some(vac_app_server_protocol::GuardianRiskLevel::High)
vac-rs/app-server/src/bespoke_event_handling.rs:2331:                    Some(vac_app_server_protocol::GuardianUserAuthorization::Low)
vac-rs/app-server/src/bespoke_event_handling.rs:3656:                            vac_app_server_protocol::HookPromptFragment {
vac-rs/app-server/src/bespoke_event_handling.rs:3660:                            vac_app_server_protocol::HookPromptFragment {
vac-rs/app-server/src/device_key_api.rs:10:use vac_app_server_protocol::DeviceKeyAlgorithm;
vac-rs/app-server/src/device_key_api.rs:11:use vac_app_server_protocol::DeviceKeyCreateParams;
vac-rs/app-server/src/device_key_api.rs:12:use vac_app_server_protocol::DeviceKeyCreateResponse;
vac-rs/app-server/src/device_key_api.rs:13:use vac_app_server_protocol::DeviceKeyProtectionClass;
vac-rs/app-server/src/device_key_api.rs:14:use vac_app_server_protocol::DeviceKeyPublicParams;
vac-rs/app-server/src/device_key_api.rs:15:use vac_app_server_protocol::DeviceKeyPublicResponse;
vac-rs/app-server/src/device_key_api.rs:16:use vac_app_server_protocol::DeviceKeySignParams;
vac-rs/app-server/src/device_key_api.rs:17:use vac_app_server_protocol::DeviceKeySignPayload;
vac-rs/app-server/src/device_key_api.rs:18:use vac_app_server_protocol::DeviceKeySignResponse;
vac-rs/app-server/src/device_key_api.rs:19:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/device_key_api.rs:193:    protection_policy: Option<vac_app_server_protocol::DeviceKeyProtectionPolicy>,
vac-rs/app-server/src/device_key_api.rs:196:        .unwrap_or(vac_app_server_protocol::DeviceKeyProtectionPolicy::HardwareOnly)
vac-rs/app-server/src/device_key_api.rs:198:        vac_app_server_protocol::DeviceKeyProtectionPolicy::HardwareOnly => {
vac-rs/app-server/src/device_key_api.rs:201:        vac_app_server_protocol::DeviceKeyProtectionPolicy::AllowOsProtectedNonextractable => {
vac-rs/app-server/src/device_key_api.rs:261:    audience: vac_app_server_protocol::RemoteControlClientConnectionAudience,
vac-rs/app-server/src/device_key_api.rs:264:        vac_app_server_protocol::RemoteControlClientConnectionAudience::RemoteControlClientWebsocket => {
vac-rs/app-server/src/device_key_api.rs:271:    audience: vac_app_server_protocol::RemoteControlClientEnrollmentAudience,
vac-rs/app-server/src/device_key_api.rs:274:        vac_app_server_protocol::RemoteControlClientEnrollmentAudience::RemoteControlClientEnrollment => {
vac-rs/app-server/src/app_server_tracing.rs:16:use vac_app_server_protocol::ClientRequest;
vac-rs/app-server/src/app_server_tracing.rs:17:use vac_app_server_protocol::InitializeParams;
vac-rs/app-server/src/app_server_tracing.rs:18:use vac_app_server_protocol::JSONRPCRequest;
vac-rs/app-server/src/models.rs:3:use vac_app_server_protocol::Model;
vac-rs/app-server/src/models.rs:4:use vac_app_server_protocol::ModelUpgradeInfo;
vac-rs/app-server/src/models.rs:5:use vac_app_server_protocol::ReasoningEffortOption;
vac-rs/app-server/src/command_exec.rs:13:use vac_app_server_protocol::CommandExecOutputDeltaNotification;
vac-rs/app-server/src/command_exec.rs:14:use vac_app_server_protocol::CommandExecOutputStream;
vac-rs/app-server/src/command_exec.rs:15:use vac_app_server_protocol::CommandExecResizeParams;
vac-rs/app-server/src/command_exec.rs:16:use vac_app_server_protocol::CommandExecResizeResponse;
vac-rs/app-server/src/command_exec.rs:17:use vac_app_server_protocol::CommandExecResponse;
vac-rs/app-server/src/command_exec.rs:18:use vac_app_server_protocol::CommandExecTerminalSize;
vac-rs/app-server/src/command_exec.rs:19:use vac_app_server_protocol::CommandExecTerminateParams;
vac-rs/app-server/src/command_exec.rs:20:use vac_app_server_protocol::CommandExecTerminateResponse;
vac-rs/app-server/src/command_exec.rs:21:use vac_app_server_protocol::CommandExecWriteParams;
vac-rs/app-server/src/command_exec.rs:22:use vac_app_server_protocol::CommandExecWriteResponse;
vac-rs/app-server/src/command_exec.rs:23:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server/src/command_exec.rs:24:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server/src/command_exec.rs:724:                    request_id: vac_app_server_protocol::RequestId::Integer(42),
vac-rs/app-server/src/command_exec.rs:752:            request_id: vac_app_server_protocol::RequestId::Integer(99),
vac-rs/app-server/src/command_exec.rs:801:            request_id: vac_app_server_protocol::RequestId::Integer(100),
vac-rs/app-server/src/command_exec.rs:885:            request_id: vac_app_server_protocol::RequestId::Integer(101),
vac-rs/app-server/src/command_exec.rs:953:            request_id: vac_app_server_protocol::RequestId::Integer(1),
vac-rs/app-server/src/command_exec.rs:989:            request_id: vac_app_server_protocol::RequestId::Integer(2),
vac-rs/app-server/src/command_exec.rs:1023:            request_id: vac_app_server_protocol::RequestId::Integer(3),
```

### `app-server-client`

- Package: `vac-app-server-client`
- Classification: **DEEP**
- `grep -rn` use-site count: **41**
- Source files touched: **1**
- Retirement focus: Transport/client protocol boundary; retire late after replacement protocol crate/package exists.
- File distribution:
  - `vac-rs/app-server-client/src/lib.rs`: 41

```text
vac-rs/app-server-client/src/lib.rs:36:use vac_app_server_protocol::ClientInfo;
vac-rs/app-server-client/src/lib.rs:37:use vac_app_server_protocol::ClientNotification;
vac-rs/app-server-client/src/lib.rs:38:use vac_app_server_protocol::ClientRequest;
vac-rs/app-server-client/src/lib.rs:39:use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server-client/src/lib.rs:40:use vac_app_server_protocol::ExternalAgentConfigDetectParams;
vac-rs/app-server-client/src/lib.rs:41:use vac_app_server_protocol::ExternalAgentConfigDetectResponse;
vac-rs/app-server-client/src/lib.rs:42:use vac_app_server_protocol::InitializeCapabilities;
vac-rs/app-server-client/src/lib.rs:43:use vac_app_server_protocol::InitializeParams;
vac-rs/app-server-client/src/lib.rs:44:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server-client/src/lib.rs:45:use vac_app_server_protocol::RequestId;
vac-rs/app-server-client/src/lib.rs:46:use vac_app_server_protocol::Result as JsonRpcResult;
vac-rs/app-server-client/src/lib.rs:47:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server-client/src/lib.rs:48:use vac_app_server_protocol::ServerRequest;
vac-rs/app-server-client/src/lib.rs:962:    use vac_app_server_protocol::ConfigRequirementsReadResponse;
vac-rs/app-server-client/src/lib.rs:963:    use vac_app_server_protocol::ServerNotification;
vac-rs/app-server-client/src/lib.rs:964:    use vac_app_server_protocol::SessionSource as ApiSessionSource;
vac-rs/app-server-client/src/lib.rs:965:    use vac_app_server_protocol::ThreadStartParams;
vac-rs/app-server-client/src/lib.rs:966:    use vac_app_server_protocol::ThreadStartResponse;
vac-rs/app-server-client/src/lib.rs:1009:            vac_app_server_protocol::CommandExecutionOutputDeltaNotification {
vac-rs/app-server-client/src/lib.rs:1020:            vac_app_server_protocol::AgentMessageDeltaNotification {
vac-rs/app-server-client/src/lib.rs:1030:        ServerNotification::ItemCompleted(vac_app_server_protocol::ItemCompletedNotification {
vac-rs/app-server-client/src/lib.rs:1033:            item: vac_app_server_protocol::ThreadItem::AgentMessage {
vac-rs/app-server-client/src/lib.rs:1043:        ServerNotification::TurnCompleted(vac_app_server_protocol::TurnCompletedNotification {
vac-rs/app-server-client/src/lib.rs:1045:            turn: vac_app_server_protocol::Turn {
vac-rs/app-server-client/src/lib.rs:1048:                status: vac_app_server_protocol::TurnStatus::Completed,
vac-rs/app-server-client/src/lib.rs:1076:                params: vac_app_server_protocol::ThreadReadParams {
vac-rs/app-server-client/src/lib.rs:1127:            .request_typed::<vac_app_server_protocol::ThreadReadResponse>(
vac-rs/app-server-client/src/lib.rs:1130:                    params: vac_app_server_protocol::ThreadReadParams {
vac-rs/app-server-client/src/lib.rs:1234:                vac_app_server_protocol::ThreadItem::AgentMessage { text, .. } if text == "hello"
vac-rs/app-server-client/src/lib.rs:1241:            )) if notification.turn.status == vac_app_server_protocol::TurnStatus::Completed
vac-rs/app-server-client/src/lib.rs:1311:                vac_app_server_protocol::ServerNotification::TurnCompleted(
vac-rs/app-server-client/src/lib.rs:1312:                    vac_app_server_protocol::TurnCompletedNotification {
vac-rs/app-server-client/src/lib.rs:1314:                        turn: vac_app_server_protocol::Turn {
vac-rs/app-server-client/src/lib.rs:1317:                            status: vac_app_server_protocol::TurnStatus::Completed,
vac-rs/app-server-client/src/lib.rs:1329:                vac_app_server_protocol::ServerNotification::AgentMessageDelta(
vac-rs/app-server-client/src/lib.rs:1330:                    vac_app_server_protocol::AgentMessageDeltaNotification {
vac-rs/app-server-client/src/lib.rs:1341:                vac_app_server_protocol::ServerNotification::ItemCompleted(
vac-rs/app-server-client/src/lib.rs:1342:                    vac_app_server_protocol::ItemCompletedNotification {
vac-rs/app-server-client/src/lib.rs:1345:                        item: vac_app_server_protocol::ThreadItem::AgentMessage {
vac-rs/app-server-client/src/lib.rs:1360:                vac_app_server_protocol::ServerNotification::CommandExecutionOutputDelta(
vac-rs/app-server-client/src/lib.rs:1361:                    vac_app_server_protocol::CommandExecutionOutputDeltaNotification {
```

### `app-server-transport`

- Package: `vac-app-server-transport`
- Classification: **DEEP**
- `grep -rn` use-site count: **57**
- Source files touched: **11**
- Retirement focus: AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency.
- File distribution:
  - `vac-rs/app-server-transport/src/outgoing_message.rs`: 5
  - `vac-rs/app-server-transport/src/transport/mod.rs`: 8
  - `vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs`: 7
  - `vac-rs/app-server-transport/src/transport/remote_control/mod.rs`: 2
  - `vac-rs/app-server-transport/src/transport/remote_control/protocol.rs`: 1
  - `vac-rs/app-server-transport/src/transport/remote_control/segment.rs`: 1
  - `vac-rs/app-server-transport/src/transport/remote_control/segment_tests.rs`: 4
  - `vac-rs/app-server-transport/src/transport/remote_control/tests.rs`: 17
  - `vac-rs/app-server-transport/src/transport/remote_control/websocket.rs`: 7
  - `vac-rs/app-server-transport/src/transport/stdio.rs`: 3
  - `vac-rs/app-server-transport/src/transport/unix_socket_tests.rs`: 2

```text
vac-rs/app-server-transport/src/transport/unix_socket_tests.rs:18:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/unix_socket_tests.rs:19:use vac_app_server_protocol::JSONRPCNotification;
vac-rs/app-server-transport/src/transport/remote_control/protocol.rs:8:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/remote_control/segment.rs:15:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs:21:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs:321:        JSONRPCMessage::Request(vac_app_server_protocol::JSONRPCRequest { method, .. })
vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs:339:    use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs:340:    use vac_app_server_protocol::JSONRPCRequest;
vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs:341:    use vac_app_server_protocol::RequestId;
vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs:342:    use vac_app_server_protocol::ServerNotification;
vac-rs/app-server-transport/src/transport/remote_control/client_tracker.rs:548:                        vac_app_server_protocol::JSONRPCNotification {
vac-rs/app-server-transport/src/transport/remote_control/segment_tests.rs:14:use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server-transport/src/transport/remote_control/segment_tests.rs:15:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/remote_control/segment_tests.rs:16:use vac_app_server_protocol::JSONRPCNotification;
vac-rs/app-server-transport/src/transport/remote_control/segment_tests.rs:17:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server-transport/src/transport/remote_control/mod.rs:26:use vac_app_server_protocol::RemoteControlConnectionStatus;
vac-rs/app-server-transport/src/transport/remote_control/mod.rs:27:use vac_app_server_protocol::RemoteControlStatusChangedNotification;
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:42:use vac_app_server_protocol::AuthMode;
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:43:use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:44:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:45:use vac_app_server_protocol::RemoteControlConnectionStatus;
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:46:use vac_app_server_protocol::RemoteControlStatusChangedNotification;
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:47:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:232:                    vac_app_server_protocol::JSONRPCNotification {
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:252:    let initialize_message = JSONRPCMessage::Request(vac_app_server_protocol::JSONRPCRequest {
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:253:        id: vac_app_server_protocol::RequestId::Integer(1),
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:310:        JSONRPCMessage::Notification(vac_app_server_protocol::JSONRPCNotification {
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:492:                message: JSONRPCMessage::Request(vac_app_server_protocol::JSONRPCRequest {
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:493:                    id: vac_app_server_protocol::RequestId::Integer(2),
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:759:    let initialize_message = JSONRPCMessage::Request(vac_app_server_protocol::JSONRPCRequest {
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:760:        id: vac_app_server_protocol::RequestId::Integer(1),
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:985:        let initialize_message = JSONRPCMessage::Request(vac_app_server_protocol::JSONRPCRequest {
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:986:            id: vac_app_server_protocol::RequestId::Integer(11),
vac-rs/app-server-transport/src/transport/remote_control/tests.rs:1044:                id: vac_app_server_protocol::RequestId::Integer(11),
vac-rs/app-server-transport/src/transport/remote_control/websocket.rs:48:use vac_app_server_protocol::RemoteControlConnectionStatus;
vac-rs/app-server-transport/src/transport/remote_control/websocket.rs:49:use vac_app_server_protocol::RemoteControlStatusChangedNotification;
vac-rs/app-server-transport/src/transport/remote_control/websocket.rs:1230:    use vac_app_server_protocol::AuthMode;
vac-rs/app-server-transport/src/transport/remote_control/websocket.rs:1231:    use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server-transport/src/transport/remote_control/websocket.rs:1232:    use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/remote_control/websocket.rs:1233:    use vac_app_server_protocol::JSONRPCNotification;
vac-rs/app-server-transport/src/transport/remote_control/websocket.rs:1234:    use vac_app_server_protocol::ServerNotification;
vac-rs/app-server-transport/src/transport/mod.rs:16:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server-transport/src/transport/mod.rs:17:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/mod.rs:270:    use vac_app_server_protocol::ConfigWarningNotification;
vac-rs/app-server-transport/src/transport/mod.rs:271:    use vac_app_server_protocol::JSONRPCNotification;
vac-rs/app-server-transport/src/transport/mod.rs:272:    use vac_app_server_protocol::JSONRPCRequest;
vac-rs/app-server-transport/src/transport/mod.rs:273:    use vac_app_server_protocol::JSONRPCResponse;
vac-rs/app-server-transport/src/transport/mod.rs:274:    use vac_app_server_protocol::RequestId;
vac-rs/app-server-transport/src/transport/mod.rs:275:    use vac_app_server_protocol::ServerNotification;
vac-rs/app-server-transport/src/transport/stdio.rs:20:use vac_app_server_protocol::InitializeParams;
vac-rs/app-server-transport/src/transport/stdio.rs:21:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/app-server-transport/src/transport/stdio.rs:22:use vac_app_server_protocol::JSONRPCRequest;
vac-rs/app-server-transport/src/outgoing_message.rs:5:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/app-server-transport/src/outgoing_message.rs:6:use vac_app_server_protocol::RequestId;
vac-rs/app-server-transport/src/outgoing_message.rs:7:use vac_app_server_protocol::Result;
vac-rs/app-server-transport/src/outgoing_message.rs:8:use vac_app_server_protocol::ServerNotification;
vac-rs/app-server-transport/src/outgoing_message.rs:9:use vac_app_server_protocol::ServerRequest;
```

### `app-server/tests/common`

- Package: `app_test_support`
- Classification: **SHALLOW**
- `grep -rn` use-site count: **0**
- Source files touched: **0**
- Retirement focus: Manifest-only/test-support dependency; remove or replace once callers stop needing protocol fixtures.
- File distribution: none under `src/`.

```text
# no matches from: grep -rn "vac_app_server_protocol" vac-rs/app-server/tests/common/src/
```

### `chatgpt`

- Package: `vac-chatgpt`
- Classification: **SHALLOW**
- `grep -rn` use-site count: **1**
- Source files touched: **1**
- Retirement focus: Connector/app metadata coupling; move AppInfo/AppBranding/AppMetadata to connector/core API owner.
- File distribution:
  - `vac-rs/chatgpt/src/connectors.rs`: 1

```text
vac-rs/chatgpt/src/connectors.rs:6:use vac_app_server_protocol::AppInfo;
```

### `config`

- Package: `vac-config`
- Classification: **MEDIUM**
- `grep -rn` use-site count: **12**
- Source files touched: **9**
- Retirement focus: Config provenance/schema coupling; retire after config-owned replacement types are available.
- File distribution:
  - `vac-rs/config/src/config_toml.rs`: 2
  - `vac-rs/config/src/diagnostics.rs`: 1
  - `vac-rs/config/src/fingerprint.rs`: 1
  - `vac-rs/config/src/lib.rs`: 1
  - `vac-rs/config/src/loader/mod.rs`: 1
  - `vac-rs/config/src/profile_toml.rs`: 1
  - `vac-rs/config/src/state.rs`: 3
  - `vac-rs/config/src/thread_config.rs`: 1
  - `vac-rs/config/src/types.rs`: 1

```text
vac-rs/config/src/lib.rs:123:pub use vac_app_server_protocol::ConfigLayerSource;
vac-rs/config/src/types.rs:822:impl From<SandboxWorkspaceWrite> for vac_app_server_protocol::SandboxSettings {
vac-rs/config/src/profile_toml.rs:76:impl From<ConfigProfile> for vac_app_server_protocol::Profile {
vac-rs/config/src/thread_config.rs:7:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/config/src/fingerprint.rs:6:use vac_app_server_protocol::ConfigLayerMetadata;
vac-rs/config/src/config_toml.rs:34:use vac_app_server_protocol::Tools;
vac-rs/config/src/config_toml.rs:35:use vac_app_server_protocol::UserSavedConfig;
vac-rs/config/src/state.rs:12:use vac_app_server_protocol::ConfigLayer;
vac-rs/config/src/state.rs:13:use vac_app_server_protocol::ConfigLayerMetadata;
vac-rs/config/src/state.rs:14:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/config/src/diagnostics.rs:19:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/config/src/loader/mod.rs:33:use vac_app_server_protocol::ConfigLayerSource;
```

### `connectors`

- Package: `vac-connectors`
- Classification: **MEDIUM**
- `grep -rn` use-site count: **7**
- Source files touched: **5**
- Retirement focus: Connector/app metadata coupling; move AppInfo/AppBranding/AppMetadata to connector/core API owner.
- File distribution:
  - `vac-rs/connectors/src/accessible.rs`: 1
  - `vac-rs/connectors/src/filter.rs`: 1
  - `vac-rs/connectors/src/lib.rs`: 3
  - `vac-rs/connectors/src/merge.rs`: 1
  - `vac-rs/connectors/src/metadata.rs`: 1

```text
vac-rs/connectors/src/metadata.rs:1:use vac_app_server_protocol::AppInfo;
vac-rs/connectors/src/lib.rs:9:use vac_app_server_protocol::AppBranding;
vac-rs/connectors/src/lib.rs:10:use vac_app_server_protocol::AppInfo;
vac-rs/connectors/src/lib.rs:11:use vac_app_server_protocol::AppMetadata;
vac-rs/connectors/src/filter.rs:3:use vac_app_server_protocol::AppInfo;
vac-rs/connectors/src/merge.rs:6:use vac_app_server_protocol::AppInfo;
vac-rs/connectors/src/accessible.rs:6:use vac_app_server_protocol::AppInfo;
```

### `core`

- Package: `vac-core`
- Classification: **DEEP**
- `grep -rn` use-site count: **29**
- Source files touched: **20**
- Retirement focus: AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency.
- File distribution:
  - `vac-rs/core/src/agent/role.rs`: 1
  - `vac-rs/core/src/agents_md.rs`: 1
  - `vac-rs/core/src/apps/render.rs`: 1
  - `vac-rs/core/src/client.rs`: 1
  - `vac-rs/core/src/client_tests.rs`: 1
  - `vac-rs/core/src/config/config_loader_tests.rs`: 2
  - `vac-rs/core/src/config/config_tests.rs`: 1
  - `vac-rs/core/src/connectors.rs`: 3
  - `vac-rs/core/src/context/apps_instructions.rs`: 1
  - `vac-rs/core/src/exec_policy.rs`: 1
  - `vac-rs/core/src/exec_policy_tests.rs`: 1
  - `vac-rs/core/src/mcp_tool_call.rs`: 5
  - `vac-rs/core/src/network_proxy_loader.rs`: 1
  - `vac-rs/core/src/realtime_conversation.rs`: 1
  - `vac-rs/core/src/session/mod.rs`: 2
  - `vac-rs/core/src/session/tests.rs`: 1
  - `vac-rs/core/src/session/tests/guardian_tests.rs`: 1
  - `vac-rs/core/src/thread_manager.rs`: 2
  - `vac-rs/core/src/tools/handlers/request_plugin_install.rs`: 1
  - `vac-rs/core/src/tools/spec_tests.rs`: 1

```text
vac-rs/core/src/context/apps_instructions.rs:1:use vac_app_server_protocol::AppInfo;
vac-rs/core/src/mcp_tool_call.rs:7:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/mcp_tool_call.rs:8:use vac_app_server_protocol::McpElicitationObjectType;
vac-rs/core/src/mcp_tool_call.rs:9:use vac_app_server_protocol::McpElicitationSchema;
vac-rs/core/src/mcp_tool_call.rs:10:use vac_app_server_protocol::McpServerElicitationRequest;
vac-rs/core/src/mcp_tool_call.rs:11:use vac_app_server_protocol::McpServerElicitationRequestParams;
vac-rs/core/src/exec_policy.rs:13:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/agents_md.rs:23:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/agent/role.rs:20:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/thread_manager.rs:36:use vac_app_server_protocol::ThreadHistoryBuilder;
vac-rs/core/src/thread_manager.rs:37:use vac_app_server_protocol::TurnStatus;
vac-rs/core/src/client_tests.rs:23:use vac_app_server_protocol::AuthMode;
vac-rs/core/src/session/tests/guardian_tests.rs:33:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/session/mod.rs:75:use vac_app_server_protocol::McpServerElicitationRequest;
vac-rs/core/src/session/mod.rs:76:use vac_app_server_protocol::McpServerElicitationRequestParams;
vac-rs/core/src/session/tests.rs:105:use vac_app_server_protocol::AppInfo;
vac-rs/core/src/config/config_tests.rs:5095:            vac_app_server_protocol::ConfigLayerSource::User {
vac-rs/core/src/config/config_loader_tests.rs:10:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/config/config_loader_tests.rs:1943:    use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/tools/spec_tests.rs:11:use vac_app_server_protocol::AppInfo;
vac-rs/core/src/tools/handlers/request_plugin_install.rs:6:use vac_app_server_protocol::AppInfo;
vac-rs/core/src/network_proxy_loader.rs:12:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/client.rs:64:use vac_app_server_protocol::AuthMode;
vac-rs/core/src/realtime_conversation.rs:36:use vac_app_server_protocol::AuthMode;
vac-rs/core/src/exec_policy_tests.rs:12:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core/src/connectors.rs:16:pub use vac_app_server_protocol::AppBranding;
vac-rs/core/src/connectors.rs:17:pub use vac_app_server_protocol::AppInfo;
vac-rs/core/src/connectors.rs:18:pub use vac_app_server_protocol::AppMetadata;
vac-rs/core/src/apps/render.rs:3:use vac_app_server_protocol::AppInfo;
```

### `core-plugins`

- Package: `vac-core-plugins`
- Classification: **MEDIUM**
- `grep -rn` use-site count: **12**
- Source files touched: **4**
- Retirement focus: Config provenance/schema coupling; retire after config-owned replacement types are available.
- File distribution:
  - `vac-rs/core-plugins/src/manager_tests.rs`: 1
  - `vac-rs/core-plugins/src/marketplace.rs`: 2
  - `vac-rs/core-plugins/src/remote.rs`: 6
  - `vac-rs/core-plugins/src/remote/share/tests.rs`: 3

```text
vac-rs/core-plugins/src/marketplace.rs:12:use vac_app_server_protocol::PluginAuthPolicy;
vac-rs/core-plugins/src/marketplace.rs:13:use vac_app_server_protocol::PluginInstallPolicy;
vac-rs/core-plugins/src/manager_tests.rs:22:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core-plugins/src/remote/share/tests.rs:10:use vac_app_server_protocol::PluginAuthPolicy;
vac-rs/core-plugins/src/remote/share/tests.rs:11:use vac_app_server_protocol::PluginInstallPolicy;
vac-rs/core-plugins/src/remote/share/tests.rs:12:use vac_app_server_protocol::PluginInterface;
vac-rs/core-plugins/src/remote.rs:12:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/core-plugins/src/remote.rs:13:use vac_app_server_protocol::PluginAuthPolicy;
vac-rs/core-plugins/src/remote.rs:14:use vac_app_server_protocol::PluginAvailability;
vac-rs/core-plugins/src/remote.rs:15:use vac_app_server_protocol::PluginInstallPolicy;
vac-rs/core-plugins/src/remote.rs:16:use vac_app_server_protocol::PluginInterface;
vac-rs/core-plugins/src/remote.rs:17:use vac_app_server_protocol::SkillInterface;
```

### `core-skills`

- Package: `vac-core-skills`
- Classification: **DONE**
- L23 status: **DONE / SHA `5f9b59f`** — replaced direct `vac_app_server_protocol::ConfigLayerSource` imports with config-owned facade `vac_config::ConfigLayerSource`, removed `vac-app-server-protocol` from `vac-rs/core-skills/Cargo.toml`, and validated build/nextest/clippy/workspace-check.
- `grep -rn` use-site count before retirement: **3**
- Source files touched: **3**
- Retirement focus: Config provenance/schema coupling; retire after config-owned replacement types are available.
- File distribution:
  - `vac-rs/core-skills/src/config_rules.rs`: 1
  - `vac-rs/core-skills/src/loader.rs`: 1
  - `vac-rs/core-skills/src/manager_tests.rs`: 1

```text
vac-rs/core-skills/src/manager_tests.rs:11:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core-skills/src/config_rules.rs:4:use vac_app_server_protocol::ConfigLayerSource;
vac-rs/core-skills/src/loader.rs:23:use vac_app_server_protocol::ConfigLayerSource;
```

### `exec-server`

- Package: `vac-exec-server`
- Classification: **DEEP**
- `grep -rn` use-site count: **39**
- Source files touched: **14**
- Retirement focus: JSON-RPC envelope/error coupling; needs shared JSON-RPC crate or exec-owned protocol facades.
- File distribution:
  - `vac-rs/exec-server/src/client.rs`: 4
  - `vac-rs/exec-server/src/client/reqwest_http_client.rs`: 1
  - `vac-rs/exec-server/src/connection.rs`: 1
  - `vac-rs/exec-server/src/fs_helper.rs`: 1
  - `vac-rs/exec-server/src/fs_sandbox.rs`: 1
  - `vac-rs/exec-server/src/local_process.rs`: 1
  - `vac-rs/exec-server/src/rpc.rs`: 9
  - `vac-rs/exec-server/src/sandboxed_file_system.rs`: 1
  - `vac-rs/exec-server/src/server/file_system_handler.rs`: 1
  - `vac-rs/exec-server/src/server/handler.rs`: 2
  - `vac-rs/exec-server/src/server/jsonrpc.rs`: 5
  - `vac-rs/exec-server/src/server/process_handler.rs`: 1
  - `vac-rs/exec-server/src/server/processor.rs`: 10
  - `vac-rs/exec-server/src/server/session_registry.rs`: 1

```text
vac-rs/exec-server/src/sandboxed_file_system.rs:5:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/fs_sandbox.rs:5:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/server/handler.rs:11:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/server/handler.rs:12:use vac_app_server_protocol::RequestId;
vac-rs/exec-server/src/server/process_handler.rs:1:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/server/jsonrpc.rs:1:use vac_app_server_protocol::JSONRPCError;
vac-rs/exec-server/src/server/jsonrpc.rs:2:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/server/jsonrpc.rs:3:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/exec-server/src/server/jsonrpc.rs:4:use vac_app_server_protocol::JSONRPCResponse;
vac-rs/exec-server/src/server/jsonrpc.rs:5:use vac_app_server_protocol::RequestId;
vac-rs/exec-server/src/server/file_system_handler.rs:5:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/server/processor.rs:87:                        request_id: vac_app_server_protocol::RequestId::Integer(-1),
vac-rs/exec-server/src/server/processor.rs:97:                vac_app_server_protocol::JSONRPCMessage::Request(request) => {
vac-rs/exec-server/src/server/processor.rs:125:                vac_app_server_protocol::JSONRPCMessage::Notification(notification) => {
vac-rs/exec-server/src/server/processor.rs:148:                vac_app_server_protocol::JSONRPCMessage::Response(response) => {
vac-rs/exec-server/src/server/processor.rs:155:                vac_app_server_protocol::JSONRPCMessage::Error(error) => {
vac-rs/exec-server/src/server/processor.rs:198:    use vac_app_server_protocol::JSONRPCMessage;
vac-rs/exec-server/src/server/processor.rs:199:    use vac_app_server_protocol::JSONRPCNotification;
vac-rs/exec-server/src/server/processor.rs:200:    use vac_app_server_protocol::JSONRPCRequest;
vac-rs/exec-server/src/server/processor.rs:201:    use vac_app_server_protocol::JSONRPCResponse;
vac-rs/exec-server/src/server/processor.rs:202:    use vac_app_server_protocol::RequestId;
vac-rs/exec-server/src/server/session_registry.rs:8:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/connection.rs:9:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/exec-server/src/fs_helper.rs:6:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/local_process.rs:12:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/client/reqwest_http_client.rs:18:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/rpc.rs:16:use vac_app_server_protocol::JSONRPCError;
vac-rs/exec-server/src/rpc.rs:17:use vac_app_server_protocol::JSONRPCErrorError;
vac-rs/exec-server/src/rpc.rs:18:use vac_app_server_protocol::JSONRPCMessage;
vac-rs/exec-server/src/rpc.rs:19:use vac_app_server_protocol::JSONRPCNotification;
vac-rs/exec-server/src/rpc.rs:20:use vac_app_server_protocol::JSONRPCRequest;
vac-rs/exec-server/src/rpc.rs:21:use vac_app_server_protocol::JSONRPCResponse;
vac-rs/exec-server/src/rpc.rs:22:use vac_app_server_protocol::RequestId;
vac-rs/exec-server/src/rpc.rs:522:    use vac_app_server_protocol::JSONRPCMessage;
vac-rs/exec-server/src/rpc.rs:523:    use vac_app_server_protocol::JSONRPCResponse;
vac-rs/exec-server/src/client.rs:17:use vac_app_server_protocol::JSONRPCNotification;
vac-rs/exec-server/src/client.rs:889:    use vac_app_server_protocol::JSONRPCMessage;
vac-rs/exec-server/src/client.rs:890:    use vac_app_server_protocol::JSONRPCNotification;
vac-rs/exec-server/src/client.rs:891:    use vac_app_server_protocol::JSONRPCResponse;
```

### `external-agent-sessions`

- Package: `vac-external-agent-sessions`
- Classification: **MEDIUM**
- L23 status: **SKIPPED / RECLASSIFIED MEDIUM** — use sites are test-only, but they assert `ThreadItem` values produced by `build_turns_from_rollout_items`; retiring requires a local/core-owned conversion helper rather than a mechanical import swap.
- `grep -rn` use-site count: **2**
- Source files touched: **1**
- Retirement focus: Thread/export conversion coupling; replace rollout-to-thread helpers with local/core-owned conversion.
- File distribution:
  - `vac-rs/external-agent-sessions/src/export.rs`: 2

```text
vac-rs/external-agent-sessions/src/export.rs:202:    use vac_app_server_protocol::ThreadItem;
vac-rs/external-agent-sessions/src/export.rs:203:    use vac_app_server_protocol::build_turns_from_rollout_items;
```

### `login`

- Package: `vac-login`
- Classification: **MEDIUM**
- `grep -rn` use-site count: **7**
- Source files touched: **6**
- Retirement focus: AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency.
- File distribution:
  - `vac-rs/login/src/auth/auth_tests.rs`: 1
  - `vac-rs/login/src/auth/external_bearer.rs`: 1
  - `vac-rs/login/src/auth/manager.rs`: 2
  - `vac-rs/login/src/auth/revoke.rs`: 1
  - `vac-rs/login/src/auth/storage.rs`: 1
  - `vac-rs/login/src/server.rs`: 1

```text
vac-rs/login/src/auth/manager.rs:21:use vac_app_server_protocol::AuthMode;
vac-rs/login/src/auth/manager.rs:22:use vac_app_server_protocol::AuthMode as ApiAuthMode;
vac-rs/login/src/auth/revoke.rs:10:use vac_app_server_protocol::AuthMode as ApiAuthMode;
vac-rs/login/src/auth/storage.rs:25:use vac_app_server_protocol::AuthMode;
vac-rs/login/src/auth/external_bearer.rs:14:use vac_app_server_protocol::AuthMode;
vac-rs/login/src/auth/auth_tests.rs:5:use vac_app_server_protocol::AuthMode;
vac-rs/login/src/server.rs:46:use vac_app_server_protocol::AuthMode;
```

### `model-provider-info`

- Package: `vac-model-provider-info`
- Classification: **MEDIUM**
- L23 status: **SKIPPED / RECLASSIFIED MEDIUM** — no owner-native `vac_protocol::AuthMode` exists; the use site drives provider base URL selection, so retirement needs canonical auth enum move/duplication or a local conversion boundary.
- `grep -rn` use-site count: **1**
- Source files touched: **1**
- Retirement focus: AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency.
- File distribution:
  - `vac-rs/model-provider-info/src/lib.rs`: 1

```text
vac-rs/model-provider-info/src/lib.rs:20:use vac_app_server_protocol::AuthMode;
```

### `models-manager`

- Package: `vac-models-manager`
- Classification: **DONE**
- L-31E-MM status: **DONE** — public `AuthMode` re-export and tests now consume `vac_protocol::auth::AuthMode`; direct `vac-app-server-protocol` dependency removed.
- `grep -rn` use-site count: **2**
- Source files touched: **2**
- Retirement focus: complete; canonical `AuthMode` now comes from `vac_protocol::auth`.
- File distribution:
  - `vac-rs/models-manager/src/lib.rs`: 1
  - `vac-rs/models-manager/src/manager_tests.rs`: 1

```text
vac-rs/models-manager/src/lib.rs:10:pub use vac_protocol::auth::AuthMode;
vac-rs/models-manager/src/manager_tests.rs:21:use vac_protocol::auth::AuthMode;
```

### `otel`

- Package: `vac-otel`
- Classification: **MEDIUM**
- `grep -rn` use-site count: **6**
- Source files touched: **1**
- Retirement focus: AuthMode coupling; move/duplicate canonical auth enum into a non-app-server protocol owner before deleting dependency.
- File distribution:
  - `vac-rs/otel/src/lib.rs`: 6

```text
vac-rs/otel/src/lib.rs:51:impl From<vac_app_server_protocol::AuthMode> for TelemetryAuthMode {
vac-rs/otel/src/lib.rs:52:    fn from(mode: vac_app_server_protocol::AuthMode) -> Self {
vac-rs/otel/src/lib.rs:54:            vac_app_server_protocol::AuthMode::ApiKey => Self::ApiKey,
vac-rs/otel/src/lib.rs:55:            vac_app_server_protocol::AuthMode::Chatgpt
vac-rs/otel/src/lib.rs:56:            | vac_app_server_protocol::AuthMode::ChatgptAuthTokens
vac-rs/otel/src/lib.rs:57:            | vac_app_server_protocol::AuthMode::AgentIdentity => Self::Chatgpt,
```

### `tools`

- Package: `vac-tools`
- Classification: **MEDIUM**
- `grep -rn` use-site count: **8**
- Source files touched: **4**
- Retirement focus: Connector/app metadata coupling; move AppInfo/AppBranding/AppMetadata to connector/core API owner.
- File distribution:
  - `vac-rs/tools/src/request_plugin_install.rs`: 5
  - `vac-rs/tools/src/tool_discovery.rs`: 1
  - `vac-rs/tools/src/tool_discovery_tests.rs`: 1
  - `vac-rs/tools/src/tool_registry_plan_tests.rs`: 1

```text
vac-rs/tools/src/tool_registry_plan_tests.rs:27:use vac_app_server_protocol::AppInfo;
vac-rs/tools/src/tool_discovery_tests.rs:6:use vac_app_server_protocol::AppInfo;
vac-rs/tools/src/request_plugin_install.rs:6:use vac_app_server_protocol::AppInfo;
vac-rs/tools/src/request_plugin_install.rs:7:use vac_app_server_protocol::McpElicitationObjectType;
vac-rs/tools/src/request_plugin_install.rs:8:use vac_app_server_protocol::McpElicitationSchema;
vac-rs/tools/src/request_plugin_install.rs:9:use vac_app_server_protocol::McpServerElicitationRequest;
vac-rs/tools/src/request_plugin_install.rs:10:use vac_app_server_protocol::McpServerElicitationRequestParams;
vac-rs/tools/src/tool_discovery.rs:13:use vac_app_server_protocol::AppInfo;
```

### `tui`

- Package: `vac-tui`
- Classification: **DEEP**
- `grep -rn` use-site count: **39**
- Source files touched: **2**
- Retirement focus: TUI app-server session adapter aliases protocol types; retire after UI session DTO boundary is stable.
- File distribution:
  - `vac-rs/tui/src/app_server_session.rs`: 14
  - `vac-rs/tui/src/session_protocol.rs`: 25

```text
vac-rs/tui/src/session_protocol.rs:51:pub(crate) use vac_app_server_protocol::AdditionalFileSystemPermissions as AppServerAdditionalFileSystemPermissions;
vac-rs/tui/src/session_protocol.rs:52:pub(crate) use vac_app_server_protocol::AdditionalNetworkPermissions as AppServerAdditionalNetworkPermissions;
vac-rs/tui/src/session_protocol.rs:53:pub(crate) use vac_app_server_protocol::AdditionalPermissionProfile as AppServerAdditionalPermissionProfile;
vac-rs/tui/src/session_protocol.rs:54:pub(crate) use vac_app_server_protocol::GuardianCommandSource as AppServerGuardianCommandSource;
vac-rs/tui/src/session_protocol.rs:55:pub(crate) use vac_app_server_protocol::GuardianRiskLevel as AppServerGuardianRiskLevel;
vac-rs/tui/src/session_protocol.rs:56:pub(crate) use vac_app_server_protocol::GuardianUserAuthorization as AppServerGuardianUserAuthorization;
vac-rs/tui/src/session_protocol.rs:57:pub(crate) use vac_app_server_protocol::HookCompletedNotification as AppServerHookCompletedNotification;
vac-rs/tui/src/session_protocol.rs:58:pub(crate) use vac_app_server_protocol::HookEventName as AppServerHookEventName;
vac-rs/tui/src/session_protocol.rs:59:pub(crate) use vac_app_server_protocol::HookExecutionMode as AppServerHookExecutionMode;
vac-rs/tui/src/session_protocol.rs:60:pub(crate) use vac_app_server_protocol::HookHandlerType as AppServerHookHandlerType;
vac-rs/tui/src/session_protocol.rs:61:pub(crate) use vac_app_server_protocol::HookScope as AppServerHookScope;
vac-rs/tui/src/session_protocol.rs:62:pub(crate) use vac_app_server_protocol::HookStartedNotification as AppServerHookStartedNotification;
vac-rs/tui/src/session_protocol.rs:63:pub(crate) use vac_app_server_protocol::ModelVerification as AppServerModelVerification;
vac-rs/tui/src/session_protocol.rs:64:pub(crate) use vac_app_server_protocol::NetworkAccess as AppServerNetworkAccess;
vac-rs/tui/src/session_protocol.rs:65:pub(crate) use vac_app_server_protocol::NetworkApprovalContext as AppServerNetworkApprovalContext;
vac-rs/tui/src/session_protocol.rs:66:pub(crate) use vac_app_server_protocol::NetworkApprovalProtocol as AppServerNetworkApprovalProtocol;
vac-rs/tui/src/session_protocol.rs:67:pub(crate) use vac_app_server_protocol::ThreadSortKey as AppServerThreadSortKey;
vac-rs/tui/src/session_protocol.rs:97:pub(crate) fn thread_goal_from_app_server(goal: vac_app_server_protocol::ThreadGoal) -> ThreadGoal {
vac-rs/tui/src/session_protocol.rs:112:    status: vac_app_server_protocol::TurnPlanStepStatus,
vac-rs/tui/src/session_protocol.rs:115:        vac_app_server_protocol::TurnPlanStepStatus::Pending => {
vac-rs/tui/src/session_protocol.rs:118:        vac_app_server_protocol::TurnPlanStepStatus::InProgress => {
vac-rs/tui/src/session_protocol.rs:121:        vac_app_server_protocol::TurnPlanStepStatus::Completed => {
vac-rs/tui/src/session_protocol.rs:128:    source: vac_app_server_protocol::AutoReviewDecisionSource,
vac-rs/tui/src/session_protocol.rs:131:        vac_app_server_protocol::AutoReviewDecisionSource::Agent => {
vac-rs/tui/src/session_protocol.rs:258:    pub(crate) use vac_app_server_protocol::{
vac-rs/tui/src/app_server_session.rs:167:use vac_app_server_protocol::CommandMigration;
vac-rs/tui/src/app_server_session.rs:168:use vac_app_server_protocol::HookMigration;
vac-rs/tui/src/app_server_session.rs:169:use vac_app_server_protocol::McpServerMigration;
vac-rs/tui/src/app_server_session.rs:170:use vac_app_server_protocol::SessionMigration;
vac-rs/tui/src/app_server_session.rs:171:use vac_app_server_protocol::SubagentMigration;
vac-rs/tui/src/app_server_session.rs:180:use vac_app_server_protocol::ReviewTarget as AppServerReviewTarget;
vac-rs/tui/src/app_server_session.rs:181:use vac_app_server_protocol::SortDirection as AppServerSortDirection;
vac-rs/tui/src/app_server_session.rs:182:use vac_app_server_protocol::ThreadSortKey as AppServerThreadSortKey;
vac-rs/tui/src/app_server_session.rs:3415:) -> vac_app_server_protocol::ReviewDelivery {
vac-rs/tui/src/app_server_session.rs:3417:        ReviewDelivery::Inline => vac_app_server_protocol::ReviewDelivery::Inline,
vac-rs/tui/src/app_server_session.rs:3418:        ReviewDelivery::Detached => vac_app_server_protocol::ReviewDelivery::Detached,
vac-rs/tui/src/app_server_session.rs:3424:) -> vac_app_server_protocol::ThreadMemoryMode {
vac-rs/tui/src/app_server_session.rs:3426:        ThreadMemoryMode::Enabled => vac_app_server_protocol::ThreadMemoryMode::Enabled,
vac-rs/tui/src/app_server_session.rs:3427:        ThreadMemoryMode::Disabled => vac_app_server_protocol::ThreadMemoryMode::Disabled,
```

## Raw dependent manifest list

```text
vac-rs/analytics/Cargo.toml (vac-analytics)
vac-rs/app-server/Cargo.toml (vac-app-server)
vac-rs/app-server/tests/common/Cargo.toml (app_test_support)
vac-rs/app-server-client/Cargo.toml (vac-app-server-client)
vac-rs/app-server-transport/Cargo.toml (vac-app-server-transport)
vac-rs/chatgpt/Cargo.toml (vac-chatgpt)
vac-rs/config/Cargo.toml (vac-config)
vac-rs/connectors/Cargo.toml (vac-connectors)
vac-rs/core/Cargo.toml (vac-core)
vac-rs/core-plugins/Cargo.toml (vac-core-plugins)
vac-rs/core-skills/Cargo.toml (vac-core-skills)
vac-rs/exec-server/Cargo.toml (vac-exec-server)
vac-rs/external-agent-sessions/Cargo.toml (vac-external-agent-sessions)
vac-rs/login/Cargo.toml (vac-login)
vac-rs/model-provider-info/Cargo.toml (vac-model-provider-info)
vac-rs/models-manager/Cargo.toml (vac-models-manager)
vac-rs/otel/Cargo.toml (vac-otel)
vac-rs/tools/Cargo.toml (vac-tools)
vac-rs/tui/Cargo.toml (vac-tui)
TOTAL=19
```

