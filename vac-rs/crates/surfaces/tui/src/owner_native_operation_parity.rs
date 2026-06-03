// Plan 30 owner-native TUI session operation parity registry.
//
// This is the default-path contract between the TUI neutral
// `LocalRuntimeSession` trait and `vac-local-runtime-owner`.  The legacy
// app-server transport can remain behind a non-default feature, but the
// default product path must have every TUI session operation classified here
// as either implemented through the owner bus/resource set or explicitly
// non-default/deferred.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerNativeOperationStatus {
    /// The operation is served by `vac-local-runtime-owner` command bus or
    /// retained resources on the default product path.
    OwnerNative,
    /// The operation is intentionally failed closed on the default path because
    /// it requires a non-default legacy/remote compatibility surface.
    NonDefaultFailClosed,
}

impl OwnerNativeOperationStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::OwnerNative => "owner_native",
            Self::NonDefaultFailClosed => "non_default_fail_closed",
        }
    }

    pub(crate) const fn release_blocking(self) -> bool {
        // Non-default controls fail closed by design and are not release blockers
        // unless the runtime support contract marks them as critical.  The
        // critical startup/turn surface is owned by owner_native_runtime_support.
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnerNativeOperationParity {
    pub(crate) operation: &'static str,
    pub(crate) status: OwnerNativeOperationStatus,
    pub(crate) owner: &'static str,
}

impl OwnerNativeOperationParity {
    pub(crate) const fn release_blocking(self) -> bool {
        self.status.release_blocking()
    }
}

pub(crate) const OWNER_NATIVE_OPERATION_PARITY: &[OwnerNativeOperationParity] = &[
    OwnerNativeOperationParity {
        operation: "bootstrap",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "vac-local-runtime-owner::startup",
    },
    OwnerNativeOperationParity {
        operation: "read_account",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeReadCommand::ReadAccount",
    },
    OwnerNativeOperationParity {
        operation: "external_agent_config_detect",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeReadCommand::ExternalAgentConfigDetect",
    },
    OwnerNativeOperationParity {
        operation: "external_agent_config_import",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::ExternalAgentConfigImport",
    },
    OwnerNativeOperationParity {
        operation: "thread_list",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "ThreadStore::list_threads",
    },
    OwnerNativeOperationParity {
        operation: "thread_loaded_list",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "ThreadManager::list_thread_ids",
    },
    OwnerNativeOperationParity {
        operation: "thread_read",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "ThreadStore::read_thread / ThreadManager snapshot",
    },
    OwnerNativeOperationParity {
        operation: "start_thread_with_session_start_source",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "ThreadManager::start_thread_with_options",
    },
    OwnerNativeOperationParity {
        operation: "resume_thread",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "ThreadStore + ThreadManager resume",
    },
    OwnerNativeOperationParity {
        operation: "branch_thread",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "ThreadStore + ThreadManager branch",
    },
    OwnerNativeOperationParity {
        operation: "turn_start",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::StartTurn",
    },
    OwnerNativeOperationParity {
        operation: "turn_interrupt",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::InterruptTurn",
    },
    OwnerNativeOperationParity {
        operation: "startup_interrupt",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::InterruptTurn",
    },
    OwnerNativeOperationParity {
        operation: "turn_steer",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::SteerTurn",
    },
    OwnerNativeOperationParity {
        operation: "thread_goal_get",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeReadCommand::GetThreadGoal",
    },
    OwnerNativeOperationParity {
        operation: "thread_goal_set",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::SetThreadGoal",
    },
    OwnerNativeOperationParity {
        operation: "thread_goal_clear",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::ClearThreadGoal",
    },
    OwnerNativeOperationParity {
        operation: "thread_memory_mode_set",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::SetThreadMemoryMode",
    },
    OwnerNativeOperationParity {
        operation: "memory_reset",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::ResetMemory",
    },
    OwnerNativeOperationParity {
        operation: "logout_account",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::LogoutAccount",
    },
    OwnerNativeOperationParity {
        operation: "reload_user_config",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::ReloadConfig",
    },
    OwnerNativeOperationParity {
        operation: "thread_compact_start",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::StartCompact",
    },
    OwnerNativeOperationParity {
        operation: "thread_shell_command",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::RunShellCommand",
    },
    OwnerNativeOperationParity {
        operation: "thread_realtime_start",
        status: OwnerNativeOperationStatus::NonDefaultFailClosed,
        owner: "RuntimeWriteCommand::RealtimeStart",
    },
    OwnerNativeOperationParity {
        operation: "thread_realtime_audio",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::RealtimeAppendAudio",
    },
    OwnerNativeOperationParity {
        operation: "thread_realtime_stop",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::RealtimeStop",
    },
    OwnerNativeOperationParity {
        operation: "review_start",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeWriteCommand::StartReview",
    },
    OwnerNativeOperationParity {
        operation: "skills_list",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "RuntimeReadCommand::ListSkills",
    },
    OwnerNativeOperationParity {
        operation: "thread_set_name",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "ThreadManager::submit(Op::SetThreadName) / ThreadStore metadata",
    },
    OwnerNativeOperationParity {
        operation: "thread_unsubscribe",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "TUI local listener lifecycle",
    },
    OwnerNativeOperationParity {
        operation: "thread_inject_items",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "ThreadManager::inject_response_items",
    },
    OwnerNativeOperationParity {
        operation: "thread_rollback",
        status: OwnerNativeOperationStatus::NonDefaultFailClosed,
        owner: "ThreadStore rollback snapshot",
    },
    OwnerNativeOperationParity {
        operation: "thread_approve_guardian_denied_action",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "Runtime server-request registry",
    },
    OwnerNativeOperationParity {
        operation: "thread_background_terminals_clean",
        status: OwnerNativeOperationStatus::OwnerNative,
        owner: "Runtime retained terminal registry",
    },
    OwnerNativeOperationParity {
        operation: "resolve_server_request",
        status: OwnerNativeOperationStatus::NonDefaultFailClosed,
        owner: "ServerRequestRegistry::resolve",
    },
    OwnerNativeOperationParity {
        operation: "reject_server_request",
        status: OwnerNativeOperationStatus::NonDefaultFailClosed,
        owner: "ServerRequestRegistry::reject",
    },
];

pub(crate) fn operation_parity(operation: &str) -> Option<OwnerNativeOperationParity> {
    OWNER_NATIVE_OPERATION_PARITY
        .iter()
        .copied()
        .find(|entry| entry.operation == operation)
}

pub(crate) fn release_blocking_operations() -> Vec<&'static str> {
    OWNER_NATIVE_OPERATION_PARITY
        .iter()
        .copied()
        .filter(|entry| entry.release_blocking())
        .map(|entry| entry.operation)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan30_default_path_has_no_release_blocking_operations() {
        assert!(release_blocking_operations().is_empty());
    }

    #[test]
    fn plan30_non_default_fail_closed_is_limited_to_noncritical_controls() {
        let non_default: Vec<_> = OWNER_NATIVE_OPERATION_PARITY
            .iter()
            .copied()
            .filter(|entry| entry.status == OwnerNativeOperationStatus::NonDefaultFailClosed)
            .map(|entry| entry.operation)
            .collect();
        assert_eq!(
            non_default,
            vec![
                "thread_realtime_start",
                "thread_rollback",
                "resolve_server_request",
                "reject_server_request",
            ]
        );
    }

    #[test]
    fn plan30_covers_prompt_and_active_controls() {
        for operation in [
            "turn_start",
            "turn_interrupt",
            "turn_steer",
            "thread_compact_start",
            "thread_shell_command",
            "thread_realtime_audio",
            "thread_realtime_stop",
            "review_start",
            "skills_list",
        ] {
            assert_eq!(
                operation_parity(operation).map(|entry| entry.status),
                Some(OwnerNativeOperationStatus::OwnerNative),
                "{operation} must be owner-native on the default path"
            );
        }
    }
}
