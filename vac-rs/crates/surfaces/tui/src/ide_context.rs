// Local-tool stub for removed IDE / VSCode integration.
//
// The local coding build no longer opens IDE IPC pipes or reads VSCode context.
// `/ide` degrades with a user-facing hint while preserving call-site shape.

use crate::session_protocol::UserInput;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdeContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdeContextError;

impl IdeContextError {
    pub(crate) fn user_facing_hint(&self) -> String {
        "IDE / VSCode context integration was removed from this local coding tool build."
            .to_string()
    }
    pub(crate) fn prompt_skip_hint(&self) -> String {
        self.user_facing_hint()
    }
}

impl fmt::Display for IdeContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IDE context integration removed")
    }
}

pub(crate) fn fetch_ide_context(_cwd: &Path) -> Result<IdeContext, IdeContextError> {
    Err(IdeContextError)
}

pub(crate) fn apply_ide_context_to_user_input(_context: &IdeContext, _items: &mut Vec<UserInput>) {}

pub(crate) fn extract_prompt_request_with_offset(message: &str) -> Option<(usize, String)> {
    if message.starts_with("# Context from my IDE setup:") {
        if let Some(pos) = message.find("## My request for VAC:\n") {
            let offset = pos + "## My request for VAC:\n".len();
            return Some((offset, message[offset..].to_string()));
        }
        if let Some(pos) = message.find("## My request for VAC:\r\n") {
            let offset = pos + "## My request for VAC:\r\n".len();
            return Some((offset, message[offset..].to_string()));
        }
    }
    None
}

pub(crate) fn has_prompt_context(_context: &IdeContext) -> bool {
    false
}
