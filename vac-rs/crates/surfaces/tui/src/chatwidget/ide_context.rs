// Chat-widget local-tool stub for removed IDE context integration.

use crate::session_protocol::UserInput;
use super::ChatWidget;

#[derive(Default)]
pub(super) struct IdeContextState {
    enabled: bool,
}

impl IdeContextState {
    pub(super) fn is_enabled(&self) -> bool { self.enabled }
    fn disable(&mut self) { self.enabled = false; }
}

impl ChatWidget {
    pub(super) fn handle_ide_command(&mut self) {
        self.ide_context.disable();
        self.sync_ide_context_status_indicator();
        self.add_info_message(
            "IDE context integration was removed from this local coding tool build.".to_string(),
            Some("Use local files, @mentions, or MCP tools instead.".to_string()),
        );
    }

    pub(super) fn handle_ide_command_args(&mut self, _args: &str) {
        self.handle_ide_command();
    }

    pub(super) fn maybe_apply_ide_context(&mut self, _items: &mut Vec<UserInput>) {}

    pub(super) fn sync_ide_context_status_indicator(&mut self) {
        self.bottom_pane.set_ide_context_active(false);
    }
}
