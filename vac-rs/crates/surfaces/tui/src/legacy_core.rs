// Transitional direct `vac-core` re-exports used by TUI while 00D removes app-server coupling.
//
// Historically TUI imported these through `legacy app-server-client bridge`, which made
// ordinary config/test/helpers look like app-server-client dependencies. Keep the local module
// shape stable so callsites can move in small slices without changing behavior.

#![allow(unused_imports)]

pub use vac_core::DEFAULT_AGENTS_MD_FILENAME;
pub use vac_core::LOCAL_AGENTS_MD_FILENAME;
pub use vac_core::McpManager;
pub use vac_core::append_message_history_entry;
pub use vac_core::check_execpolicy_for_warnings;
pub use vac_core::format_exec_policy_error_with_source;
pub use vac_core::grant_read_root_non_elevated;
pub use vac_core::lookup_message_history_entry;
pub use vac_core::message_history_metadata;
pub use vac_core::web_search_detail;

pub mod config {
    pub use vac_core::config::*;

    pub mod edit {
        pub use vac_core::config::edit::*;
    }
}

pub mod connectors {
    pub use vac_core::connectors::*;
}

pub mod otel_init {
    pub use vac_core::otel_init::*;
}

pub mod personality_migration {
    pub use vac_core::personality_migration::*;
}

pub mod review_format {
    pub use vac_core::review_format::*;
}

pub mod review_prompts {
    pub use vac_core::review_prompts::*;
}

pub mod test_support {
    pub use vac_core::test_support::*;
}

pub mod util {
    pub use vac_core::util::*;
}

pub mod windows_sandbox {
    pub use vac_core::windows_sandbox::*;
}
