use crate::config::Config;
pub use vac_rollout::ARCHIVED_SESSIONS_SUBDIR;
pub use vac_rollout::Cursor;
pub use vac_rollout::EventPersistenceMode;
pub use vac_rollout::INTERACTIVE_SESSION_SOURCES;
pub use vac_rollout::RolloutRecorder;
pub use vac_rollout::RolloutRecorderParams;
pub use vac_rollout::SESSIONS_SUBDIR;
pub use vac_rollout::SessionMeta;
pub use vac_rollout::SortDirection;
pub use vac_rollout::ThreadItem;
pub use vac_rollout::ThreadListConfig;
pub use vac_rollout::ThreadListLayout;
pub use vac_rollout::ThreadSortKey;
pub use vac_rollout::ThreadsPage;
pub use vac_rollout::append_thread_name;
pub use vac_rollout::find_archived_thread_path_by_id_str;
#[deprecated(note = "use find_thread_path_by_id_str")]
pub use vac_rollout::find_conversation_path_by_id_str;
pub use vac_rollout::find_thread_meta_by_name_str;
pub use vac_rollout::find_thread_name_by_id;
pub use vac_rollout::find_thread_names_by_ids;
pub use vac_rollout::find_thread_path_by_id_str;
pub use vac_rollout::get_threads_in_root;
pub use vac_rollout::parse_cursor;
pub use vac_rollout::read_head_for_summary;
pub use vac_rollout::read_session_meta_line;
pub use vac_rollout::rollout_date_parts;
pub use vac_utils_path::paths_match_after_normalization;

impl vac_rollout::RolloutConfigView for Config {
    fn vac_home(&self) -> &std::path::Path {
        self.vac_home.as_path()
    }

    fn sqlite_home(&self) -> &std::path::Path {
        self.sqlite_home.as_path()
    }

    fn cwd(&self) -> &std::path::Path {
        self.cwd.as_path()
    }

    fn model_provider_id(&self) -> &str {
        self.model_provider_id.as_str()
    }

    fn generate_memories(&self) -> bool {
        self.memories.generate_memories
    }
}

pub(crate) mod list {
    pub use vac_rollout::find_thread_path_by_id_str;
}

pub(crate) mod recorder {
    pub use vac_rollout::RolloutRecorder;
}

pub(crate) use crate::session_rollout_init_error::map_session_init_error;

pub(crate) mod truncation {
    pub(crate) use crate::thread_rollout_truncation::*;
}
