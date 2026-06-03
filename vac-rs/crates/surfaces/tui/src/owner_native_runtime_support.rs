// Structural owner-native runtime support contract used by the TUI runtime guard.
//
// This module is intentionally code-owned rather than inferred from source text
// scans.  The `.vac/registry/runtime/owner-native-support.yaml` manifest mirrors
// this table for CLI/doctor consumers that must not depend on the TUI crate.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerRuntimeMethodStatus {
    /// The method is implemented on the default owner-native runtime path.
    Implemented,
    /// The method intentionally fails closed outside the default startup/turn path.
    NonDefaultFailClosed,
}

impl OwnerRuntimeMethodStatus {
    pub(crate) const fn as_manifest_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::NonDefaultFailClosed => "non_default_fail_closed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnerRuntimeMethodSupport {
    pub(crate) name: &'static str,
    pub(crate) status: OwnerRuntimeMethodStatus,
    pub(crate) owner: &'static str,
    pub(crate) release_blocking: bool,
}

impl OwnerRuntimeMethodSupport {
    pub(crate) const fn implemented(
        name: &'static str,
        owner: &'static str,
        release_blocking: bool,
    ) -> Self {
        Self {
            name,
            status: OwnerRuntimeMethodStatus::Implemented,
            owner,
            release_blocking,
        }
    }

    pub(crate) const fn fail_closed(name: &'static str, owner: &'static str) -> Self {
        Self {
            name,
            status: OwnerRuntimeMethodStatus::NonDefaultFailClosed,
            owner,
            release_blocking: false,
        }
    }

    pub(crate) const fn is_release_blocking_ready(self) -> bool {
        matches!(self.status, OwnerRuntimeMethodStatus::Implemented) && self.release_blocking
    }
}

pub(crate) const OWNER_RUNTIME_METHOD_SUPPORT: &[OwnerRuntimeMethodSupport] = &[
    OwnerRuntimeMethodSupport::implemented(
        "start_thread_with_session_start_source",
        "ThreadManager::start_thread_with_options",
        true,
    ),
    OwnerRuntimeMethodSupport::implemented(
        "turn_start",
        "RuntimeWriteCommand::StartTurn / Op::UserTurn",
        true,
    ),
    OwnerRuntimeMethodSupport::implemented(
        "turn_steer",
        "RuntimeWriteCommand::SteerTurn / VACThread::steer_input",
        true,
    ),
    OwnerRuntimeMethodSupport::implemented(
        "turn_interrupt",
        "RuntimeWriteCommand::InterruptTurn / Op::Interrupt",
        true,
    ),
    OwnerRuntimeMethodSupport::implemented(
        "startup_interrupt",
        "RuntimeWriteCommand::InterruptTurn / Op::Interrupt",
        true,
    ),
    OwnerRuntimeMethodSupport::implemented(
        "thread_shell_command",
        "RuntimeWriteCommand::RunShellCommand / Op::RunUserShellCommand",
        true,
    ),
    OwnerRuntimeMethodSupport::implemented("thread_list", "ThreadStore::list_threads", true),
    OwnerRuntimeMethodSupport::implemented("thread_read", "ThreadStore::read_thread", true),
    OwnerRuntimeMethodSupport::implemented(
        "resume_thread",
        "ThreadStore::read_thread + ThreadManager::resume_thread_with_history",
        true,
    ),
    OwnerRuntimeMethodSupport::implemented(
        "branch_thread",
        "ThreadStore::read_thread + InitialHistory::Branched",
        true,
    ),
    OwnerRuntimeMethodSupport::implemented("read_account", "local bootstrap/account facade", true),
    OwnerRuntimeMethodSupport::implemented("thread_goal_get", "local runtime goal facade", false),
    OwnerRuntimeMethodSupport::implemented("thread_goal_set", "local runtime goal facade", false),
    OwnerRuntimeMethodSupport::implemented("thread_goal_clear", "local runtime goal facade", false),
    OwnerRuntimeMethodSupport::implemented("thread_loaded_list", "ThreadManager::list_thread_ids", false),
    OwnerRuntimeMethodSupport::implemented("thread_compact_start", "RuntimeWriteCommand::StartCompact / Op::Compact", false),
    OwnerRuntimeMethodSupport::implemented("thread_set_name", "metadata facade", false),
    OwnerRuntimeMethodSupport::implemented("thread_unsubscribe", "TUI local listener lifecycle", false),
    OwnerRuntimeMethodSupport::implemented("thread_inject_items", "local runtime item injection facade", false),
    OwnerRuntimeMethodSupport::implemented("review_start", "safe deferred review facade", false),
    OwnerRuntimeMethodSupport::implemented("skills_list", "local empty skills facade", false),
    OwnerRuntimeMethodSupport::implemented("thread_realtime_audio", "realtime no-op until transport promotion", false),
    OwnerRuntimeMethodSupport::implemented("thread_realtime_stop", "realtime no-op until transport promotion", false),
    OwnerRuntimeMethodSupport::implemented("memory_reset", "safe no-op until memory runtime promotion", false),
    OwnerRuntimeMethodSupport::implemented("logout_account", "safe no-op until account runtime promotion", false),
    OwnerRuntimeMethodSupport::implemented("reload_user_config", "local config reload facade", false),
    OwnerRuntimeMethodSupport::implemented("thread_memory_mode_set", "local thread memory-mode facade", false),
    OwnerRuntimeMethodSupport::implemented("thread_approve_guardian_denied_action", "guardian approval facade", false),
    OwnerRuntimeMethodSupport::implemented("thread_background_terminals_clean", "RuntimeWriteCommand::CleanBackgroundTerminals", false),
    OwnerRuntimeMethodSupport::fail_closed("thread_rollback", "ThreadStore rollback snapshot"),
    OwnerRuntimeMethodSupport::fail_closed("thread_realtime_start", "realtime transport promotion"),
    OwnerRuntimeMethodSupport::fail_closed("resolve_server_request", "legacy app-server server-request registry fallback"),
    OwnerRuntimeMethodSupport::fail_closed("reject_server_request", "legacy app-server server-request registry fallback"),
];

pub(crate) fn release_blocking_owner_runtime_methods() -> impl Iterator<Item = OwnerRuntimeMethodSupport> {
    OWNER_RUNTIME_METHOD_SUPPORT
        .iter()
        .copied()
        .filter(|method| method.release_blocking)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_blocking_methods_are_all_implemented() {
        for method in release_blocking_owner_runtime_methods() {
            assert_eq!(
                method.status,
                OwnerRuntimeMethodStatus::Implemented,
                "{} must be implemented because it is release-blocking",
                method.name
            );
        }
    }

    #[test]
    fn non_default_fail_closed_methods_are_not_release_blocking() {
        let methods: Vec<_> = OWNER_RUNTIME_METHOD_SUPPORT
            .iter()
            .copied()
            .filter(|method| method.status == OwnerRuntimeMethodStatus::NonDefaultFailClosed)
            .map(|method| method.name)
            .collect();
        assert_eq!(
            methods,
            vec![
                "thread_rollback",
                "thread_realtime_start",
                "resolve_server_request",
                "reject_server_request",
            ]
        );
    }
}
