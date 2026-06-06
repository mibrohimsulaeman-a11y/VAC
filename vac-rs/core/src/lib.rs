//! Thin VAC core orchestration/export crate.
//!
//! O5/O6 zero-residual decomposition keeps `vac-core` as a small facade over
//! control-plane, configuration, project-workspace, and CLI compatibility
//! exports. Historical runtime/session compatibility bridges were removed for
//! zero-residual runtime validation; domain implementation now lives behind
//! direct capability crates instead of legacy path-bridge includes.

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub use vac_control_plane::control_plane;
pub use vac_control_plane::local_runtime;

pub mod config {
    use std::path::PathBuf;

    pub fn find_vac_home() -> std::io::Result<PathBuf> {
        if let Some(path) = std::env::var_os("VAC_HOME") {
            return Ok(PathBuf::from(path));
        }
        dirs::home_dir()
            .map(|home| home.join(".vac"))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "home directory is unavailable",
                )
            })
    }

    pub use crate::config_impl::edit;
    pub use crate::config_impl::*;
}

#[path = "../../crates/capabilities/release/src/core_migrated/config/mod.rs"]
pub mod config_impl;

pub use vac_capability_ownership::project_workspace;

#[path = "../../crates/capabilities/release/src/core_migrated/util.rs"]
pub mod util_impl;

pub mod util {
    pub use crate::util_impl::backoff;
    pub(crate) use crate::util_impl::emit_feedback_auth_recovery_tags;
    pub(crate) use crate::util_impl::error_or_panic;
    pub use crate::util_impl::normalize_thread_name;
    pub use crate::util_impl::resolve_path;
    pub use crate::util_impl::resume_command;
}

// Top-level compatibility re-exports
pub use client::ModelClient;
pub use client::ModelClientSession;
pub use client::X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER;
pub use client::X_VAC_TURN_METADATA_HEADER;
pub use client_common::Prompt;
pub use client_common::REVIEW_PROMPT;
pub use client_common::ResponseEvent;
pub use compact::content_items_to_text;
pub use event_mapping::parse_turn_item;
pub use exec_policy::check_execpolicy_for_warnings;
pub use exec_policy::format_exec_policy_error_with_source;
pub use exec_policy::load_exec_policy;
pub use installation_id::resolve_installation_id;
pub use landlock::spawn_command_under_linux_sandbox;
pub use prompt_debug::build_prompt_input;
pub use rollout::ARCHIVED_SESSIONS_SUBDIR;
pub use rollout::EventPersistenceMode;
pub use rollout::RolloutRecorder;
pub use rollout::RolloutRecorderParams;
pub use rollout::SESSIONS_SUBDIR;
pub use rollout::ThreadListConfig;
pub use rollout::ThreadListLayout;
pub use rollout::ThreadSortKey;
pub use rollout::find_archived_thread_path_by_id_str;
pub use rollout::find_thread_meta_by_name_str;
pub use rollout::find_thread_path_by_id_str;
pub use rollout::get_threads_in_root;
pub use rollout::paths_match_after_normalization;
pub use session::SteerInputError;
pub use skills::SkillMetadata;
pub use skills::SkillsManager;
pub use skills::build_skill_name_counts;
pub(crate) use skills::maybe_emit_implicit_skill_invocation;
pub(crate) use skills::skills_load_input_from_config;
pub use thread_manager::BranchSnapshot;
pub use thread_manager::NewThread;
pub use thread_manager::StartThreadOptions;
pub use thread_manager::ThreadManager;
pub use thread_manager::thread_store_from_config;
pub use turn_metadata::build_turn_metadata_header;
pub use vac_rollout::StateDbHandle;
pub use vac_thread::ThreadConfigSnapshot;
pub use vac_thread::VACThread;
pub use vac_thread::VACThreadTurnContextOverrides;

// Re-exports for skills submodules
pub use skills::SkillError;
pub use skills::SkillInjections;
pub use skills::SkillLoadOutcome;
pub use skills::SkillsLoadInput;
pub use skills::build_available_skills;
pub(crate) use skills::build_skill_injections;
pub(crate) use skills::collect_env_var_dependencies;
pub(crate) use skills::collect_explicit_skill_mentions;
pub use skills::default_skill_metadata_budget;
pub use skills::manager;
pub(crate) use skills::resolve_skill_dependencies_for_turn;

// Re-exports for message history
pub use message_history::append_entry as append_message_history_entry;
pub use message_history::history_metadata as message_history_metadata;
pub use message_history::lookup as lookup_message_history_entry;

// Re-exports for plugins/mentions
pub(crate) use plugins::mentions;

#[path = "../../crates/capabilities/release/src/core_migrated/safety.rs"]
pub mod safety;

#[path = "../../crates/capabilities/sessions/src/core_migrated/message_history.rs"]
pub mod message_history;

#[path = "../../crates/capabilities/ownership/src/core_migrated/agents_md.rs"]
pub mod agents_md;

pub mod injection {
    pub use vac_core_skills::injection::*;
}

pub mod apply_patch {
    use crate::function_tool::FunctionCallError;
    use crate::safety::{SafetyCheck, assess_patch_safety};
    use crate::session::turn_context::TurnContext;
    use crate::tools::sandboxing::ExecApprovalRequirement;
    pub use vac_apply_patch::*;
    use vac_protocol::permissions::FileSystemSandboxPolicy;
    use vac_sandboxing::SandboxType;

    #[derive(Debug)]
    pub(crate) struct DelegateToRuntime {
        pub(crate) action: ApplyPatchAction,
        pub(crate) auto_approved: bool,
        pub(crate) exec_approval_requirement: ExecApprovalRequirement,
    }

    #[derive(Debug)]
    pub(crate) enum InternalApplyPatchInvocation {
        Output(Result<String, FunctionCallError>),
        DelegateToRuntime(DelegateToRuntime),
    }

    pub(crate) fn convert_apply_patch_to_protocol(
        action: &ApplyPatchAction,
    ) -> std::collections::HashMap<std::path::PathBuf, vac_protocol::protocol::FileChange> {
        use vac_protocol::protocol::FileChange;
        action
            .changes()
            .iter()
            .map(|(path, change)| {
                let change = match change {
                    ApplyPatchFileChange::Add { content } => FileChange::Add {
                        content: content.clone(),
                    },
                    ApplyPatchFileChange::Delete { content } => FileChange::Delete {
                        content: content.clone(),
                    },
                    ApplyPatchFileChange::Update {
                        unified_diff,
                        move_path,
                        ..
                    } => FileChange::Update {
                        unified_diff: unified_diff.clone(),
                        move_path: move_path.clone(),
                    },
                };
                (path.clone(), change)
            })
            .collect()
    }

    pub(crate) async fn invoke_apply_patch(
        turn: &TurnContext,
        file_system_sandbox_policy: &FileSystemSandboxPolicy,
        action: ApplyPatchAction,
    ) -> InternalApplyPatchInvocation {
        let policy = turn.approval_policy.value();
        let permission_profile = turn.permission_profile();
        let cwd = &turn.cwd;
        let windows_sandbox_level = turn.windows_sandbox_level;

        let safety = assess_patch_safety(
            &action,
            policy,
            &permission_profile,
            file_system_sandbox_policy,
            cwd,
            windows_sandbox_level,
        );

        match safety {
            SafetyCheck::Reject { reason } => InternalApplyPatchInvocation::Output(Err(
                FunctionCallError::RespondToModel(format!("patch rejected: {reason}")),
            )),
            SafetyCheck::AutoApprove { sandbox_type, .. } => {
                InternalApplyPatchInvocation::DelegateToRuntime(DelegateToRuntime {
                    action,
                    auto_approved: true,
                    exec_approval_requirement: ExecApprovalRequirement::Skip {
                        bypass_sandbox: sandbox_type == SandboxType::None,
                        proposed_execpolicy_amendment: None,
                    },
                })
            }
            SafetyCheck::AskUser => {
                InternalApplyPatchInvocation::DelegateToRuntime(DelegateToRuntime {
                    action,
                    auto_approved: false,
                    exec_approval_requirement: ExecApprovalRequirement::NeedsApproval {
                        reason: None,
                        proposed_execpolicy_amendment: None,
                    },
                })
            }
        }
    }
}

// New path bridges for capability core_migrated files
#[path = "../../crates/capabilities/sessions/src/core_migrated/session_prefix.rs"]
pub mod session_prefix;

#[path = "../../crates/capabilities/ownership/src/core_migrated/connectors.rs"]
pub mod connectors;

#[path = "../../crates/capabilities/ownership/src/core_migrated/network_policy_decision.rs"]
pub mod network_policy_decision;

#[path = "../../crates/capabilities/sessions/src/core_migrated/memory_usage.rs"]
pub mod memory_usage;

#[path = "../../crates/capabilities/release/src/core_migrated/utils/path_utils.rs"]
pub mod path_utils;

#[path = "../../crates/capabilities/release/src/core_migrated/command_canonicalization.rs"]
pub mod command_canonicalization;

#[path = "../../crates/capabilities/ownership/src/core_migrated/state/mod.rs"]
pub mod state;

#[path = "../../crates/capabilities/sessions/src/core_migrated/arc_monitor.rs"]
pub mod arc_monitor;

#[path = "../../crates/capabilities/sessions/src/core_migrated/session_startup_prewarm.rs"]
pub mod session_startup_prewarm;

#[path = "../../crates/capabilities/release/src/core_migrated/flags.rs"]
pub mod flags;

#[path = "../../crates/capabilities/release/src/core_migrated/commit_attribution.rs"]
pub mod commit_attribution;

#[path = "../../crates/capabilities/sessions/src/core_migrated/turn_metadata.rs"]
pub mod turn_metadata;

#[path = "../../crates/capabilities/release/src/core_migrated/config_lock.rs"]
pub mod config_lock;

#[path = "../../crates/capabilities/sessions/src/core_migrated/user_shell_command.rs"]
pub mod user_shell_command;

// Identity capability domain
#[path = "../../crates/capabilities/identity/src/core_migrated/agent/mod.rs"]
pub mod agent;

#[path = "../../crates/capabilities/identity/src/core_migrated/cloud_account_disabled.rs"]
pub mod cloud_account_disabled;

#[path = "../../crates/capabilities/identity/src/core_migrated/installation_id.rs"]
pub mod installation_id;

// Build capability domain
#[path = "../../crates/capabilities/build/src/core_migrated/file_watcher.rs"]
pub mod file_watcher;

#[path = "../../crates/capabilities/build/src/core_migrated/spawn.rs"]
pub mod spawn;

#[path = "../../crates/capabilities/build/src/core_migrated/shell.rs"]
pub mod shell;

#[path = "../../crates/capabilities/build/src/core_migrated/shell_detect.rs"]
pub mod shell_detect;

#[path = "../../crates/capabilities/build/src/core_migrated/shell_snapshot.rs"]
pub mod shell_snapshot;

#[path = "../../crates/capabilities/build/src/core_migrated/unified_exec/mod.rs"]
pub mod unified_exec;

#[path = "../../crates/capabilities/build/src/core_migrated/environment_selection.rs"]
pub mod environment_selection;

#[path = "../../crates/capabilities/build/src/core_migrated/exec.rs"]
pub mod exec;

#[path = "../../crates/capabilities/build/src/core_migrated/exec_env.rs"]
pub mod exec_env;

#[path = "../../crates/capabilities/build/src/core_migrated/exec_policy.rs"]
pub mod exec_policy;

#[path = "../../crates/capabilities/build/src/core_migrated/hook_runtime.rs"]
pub mod hook_runtime;

#[path = "../../crates/capabilities/build/src/core_migrated/landlock.rs"]
pub mod landlock;

#[path = "../../crates/capabilities/build/src/core_migrated/windows_sandbox.rs"]
pub mod windows_sandbox;

#[path = "../../crates/capabilities/build/src/core_migrated/windows_sandbox_read_grants.rs"]
pub mod windows_sandbox_read_grants;

#[path = "../../crates/capabilities/build/src/core_migrated/sandboxing/mod.rs"]
pub mod sandboxing;

// Tools capability domain
#[path = "../../crates/capabilities/tools-domain/src/core_migrated/tools/mod.rs"]
pub mod tools;

#[path = "../../crates/capabilities/tools-domain/src/core_migrated/function_tool.rs"]
pub mod function_tool;

#[path = "../../crates/capabilities/tools-domain/src/core_migrated/mcp_tool_approval_templates.rs"]
pub mod mcp_tool_approval_templates;

#[path = "../../crates/capabilities/tools-domain/src/core_migrated/mcp_tool_call.rs"]
pub mod mcp_tool_call;

#[path = "../../crates/capabilities/tools-domain/src/core_migrated/mcp_tool_exposure.rs"]
pub mod mcp_tool_exposure;

#[path = "../../crates/capabilities/tools-domain/src/core_migrated/unavailable_tool.rs"]
pub mod unavailable_tool;

// Ownership capability domain
#[path = "../../crates/capabilities/ownership/src/core_migrated/sandbox_tags.rs"]
pub mod sandbox_tags;

// Chat capability domain
#[path = "../../crates/capabilities/chat/src/core_migrated/guardian/mod.rs"]
pub mod guardian;

#[path = "../../crates/capabilities/chat/src/core_migrated/mcp.rs"]
pub mod mcp;

#[path = "../../crates/capabilities/chat/src/core_migrated/mcp_skill_dependencies.rs"]
pub mod mcp_skill_dependencies;

#[path = "../../crates/capabilities/chat/src/core_migrated/mention_syntax.rs"]
pub mod mention_syntax;

#[path = "../../crates/capabilities/chat/src/core_migrated/plugins/mod.rs"]
pub mod plugins;

#[path = "../../crates/capabilities/chat/src/core_migrated/skills.rs"]
pub mod skills;

#[path = "../../crates/capabilities/chat/src/core_migrated/skills_watcher.rs"]
pub mod skills_watcher;

#[path = "../../crates/capabilities/chat/src/core_migrated/vac_delegate.rs"]
pub mod vac_delegate;

#[path = "../../crates/capabilities/chat/src/core_migrated/web_search.rs"]
pub mod web_search;

#[path = "../../crates/capabilities/chat/src/core_migrated/apps/mod.rs"]
pub mod apps;

// Sessions capability domain
#[path = "../../crates/capabilities/sessions/src/core_migrated/client.rs"]
pub mod client;

#[path = "../../crates/capabilities/sessions/src/core_migrated/client_common.rs"]
pub mod client_common;

#[path = "../../crates/capabilities/sessions/src/core_migrated/compact.rs"]
pub mod compact;

#[path = "../../crates/capabilities/sessions/src/core_migrated/compact_remote.rs"]
pub mod compact_remote;

#[path = "../../crates/capabilities/sessions/src/core_migrated/context/mod.rs"]
pub mod context;

#[path = "../../crates/capabilities/sessions/src/core_migrated/context_manager/mod.rs"]
pub mod context_manager;

#[path = "../../crates/capabilities/sessions/src/core_migrated/event_mapping.rs"]
pub mod event_mapping;

#[path = "../../crates/capabilities/sessions/src/core_migrated/goals.rs"]
pub mod goals;

#[path = "../../crates/capabilities/sessions/src/core_migrated/rollout.rs"]
pub mod rollout;

#[path = "../../crates/capabilities/sessions/src/core_migrated/session/mod.rs"]
pub mod session;

#[path = "../../crates/capabilities/sessions/src/core_migrated/session_rollout_init_error.rs"]
pub mod session_rollout_init_error;

#[path = "../../crates/capabilities/sessions/src/core_migrated/stream_events_utils.rs"]
pub mod stream_events_utils;

#[path = "../../crates/capabilities/sessions/src/core_migrated/test_support.rs"]
pub mod test_support;

#[path = "../../crates/capabilities/sessions/src/core_migrated/thread_manager.rs"]
pub mod thread_manager;

#[path = "../../crates/capabilities/sessions/src/core_migrated/thread_rollout_truncation.rs"]
pub mod thread_rollout_truncation;

#[path = "../../crates/capabilities/sessions/src/core_migrated/turn_timing.rs"]
pub mod turn_timing;

#[path = "../../crates/capabilities/sessions/src/core_migrated/turn_diff_tracker.rs"]
pub mod turn_diff_tracker;

#[path = "../../crates/capabilities/sessions/src/core_migrated/tasks/mod.rs"]
pub mod tasks;

#[path = "../../crates/capabilities/sessions/src/core_migrated/vac_thread.rs"]
pub mod vac_thread;

// Docs capability domain
#[path = "../../crates/capabilities/docs/src/core_migrated/mcp_vastar_file.rs"]
pub mod mcp_vastar_file;

#[path = "../../crates/capabilities/docs/src/core_migrated/original_image_detail.rs"]
pub mod original_image_detail;

#[path = "../../crates/capabilities/docs/src/core_migrated/personality_migration.rs"]
pub mod personality_migration;

#[path = "../../crates/capabilities/docs/src/core_migrated/prompt_debug.rs"]
pub mod prompt_debug;

#[path = "../../crates/capabilities/docs/src/core_migrated/review_format.rs"]
pub mod review_format;

#[path = "../../crates/capabilities/docs/src/core_migrated/review_prompts.rs"]
pub mod review_prompts;

#[path = "../../crates/capabilities/ownership/src/core_migrated/state_db_bridge.rs"]
pub mod state_db_bridge;
pub use state_db_bridge::get_state_db;

#[path = "../../crates/capabilities/release/src/core_migrated/otel_init.rs"]
pub mod otel_init;
