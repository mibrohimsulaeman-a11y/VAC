// Runtime-effective feature-off experimental unavailable badge support for slash palette.
use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::IntoStaticStr;

/// Commands that can be invoked by starting a message with a leading slash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandAvailability {
    Ready,
    Experimental,
    FeatureOff,
    Unavailable,
}

impl CommandAvailability {
    pub fn badge(self) -> Option<&'static str> {
        match self {
            Self::Ready => None,
            Self::Experimental => Some("[experimental]"),
            Self::FeatureOff => Some("[feature-off]"),
            Self::Unavailable => Some("[unavailable]"),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr, IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum SlashCommand {
    // DO NOT ALPHA-SORT! Enum order is presentation order in the popup, so
    // more frequently used commands should be listed first.
    Model,
    Fast,
    Ide,
    Approvals,
    Permissions,
    Keymap,
    Vim,
    #[strum(serialize = "setup-default-sandbox")]
    ElevateSandbox,
    #[strum(serialize = "sandbox-add-read-dir")]
    SandboxReadRoot,
    Experimental,
    #[strum(to_string = "autoreview")]
    AutoReview,
    Memories,
    Skills,
    Hooks,
    Review,
    Rename,
    New,
    Resume,
    Branch,
    Init,
    Compact,
    Plan,
    Goal,
    Collab,
    Agent,
    Side,
    Copy,
    Diff,
    Mention,
    Status,
    Activity,
    Runtime,
    Capabilities,
    Workflow,
    DebugConfig,
    Title,
    Statusline,
    Theme,
    Mcp,
    Apps,
    Plugins,
    Logout,
    Quit,
    Exit,
    Feedback,
    Rollout,
    Ps,
    #[strum(to_string = "stop", serialize = "clean")]
    Stop,
    Clear,
    Personality,
    Realtime,
    Settings,
    TestApproval,
    #[strum(serialize = "subagents")]
    MultiAgents,
    // Debugging commands.
    #[strum(serialize = "debug-m-drop")]
    MemoryDrop,
    #[strum(serialize = "debug-m-update")]
    MemoryUpdate,
}

impl SlashCommand {
    pub fn availability(self) -> CommandAvailability {
        match self {
            SlashCommand::Realtime | SlashCommand::Settings => CommandAvailability::FeatureOff,
            SlashCommand::Collab | SlashCommand::Experimental => CommandAvailability::Experimental,
            SlashCommand::MemoryDrop | SlashCommand::MemoryUpdate => CommandAvailability::Unavailable,
            _ => CommandAvailability::Ready,
        }
    }

    pub fn palette_badge(self) -> Option<&'static str> {
        self.availability().badge()
    }

    pub fn palette_description(self) -> String {
        match self.palette_badge() {
            Some(badge) => format!("{} {}", badge, self.description()),
            None => self.description().to_string(),
        }
    }

    /// User-visible description shown in the popup.
    pub fn description(self) -> &'static str {
        match self {
            SlashCommand::Feedback => "send logs to maintainers",
            SlashCommand::New => "start a new chat during a conversation",
            SlashCommand::Init => "start VAC-Init guided setup in the TUI (no --interactive flag)",
            SlashCommand::Compact => "summarize conversation to prevent hitting the context limit",
            SlashCommand::Review => "review my current changes and find issues",
            SlashCommand::Rename => "rename the current thread",
            SlashCommand::Resume => "resume a saved chat",
            SlashCommand::Clear => "clear the terminal and start a new chat",
            SlashCommand::Branch => "branch the current chat",
            SlashCommand::Quit | SlashCommand::Exit => "exit VAC",
            SlashCommand::Copy => "copy last response as markdown",
            SlashCommand::Diff => "show git diff (including untracked files)",
            SlashCommand::Mention => "mention a file",
            SlashCommand::Skills => "use skills to improve how VAC performs specific tasks",
            SlashCommand::Hooks => "view and manage lifecycle hooks",
            SlashCommand::Status => "show current session configuration and token usage",
            SlashCommand::Activity => "toggle the Activity sidebar open and closed",
            SlashCommand::Runtime => "show autopilot scheduler and runtime jobs",
            SlashCommand::Capabilities => "show declared capabilities and registry state",
            SlashCommand::Workflow => "show declared workflows and registry state",
            SlashCommand::DebugConfig => "show config layers and requirement sources for debugging",
            SlashCommand::Title => "configure which items appear in the terminal title",
            SlashCommand::Statusline => "configure which items appear in the status line",
            SlashCommand::Theme => "choose a syntax highlighting theme",
            SlashCommand::Ps => "list background terminals",
            SlashCommand::Stop => "stop all background terminals",
            SlashCommand::MemoryDrop => "hidden debug memory drop command",
            SlashCommand::MemoryUpdate => "hidden debug memory update command",
            SlashCommand::Model => "choose what model and reasoning effort to use",
            SlashCommand::Fast => {
                "toggle Fast mode to enable fastest inference with increased plan usage"
            }
            SlashCommand::Ide => {
                "include current selection, open files, and other context from your IDE"
            }
            SlashCommand::Personality => "choose a communication style for VAC",
            SlashCommand::Realtime => "toggle realtime voice mode (experimental)",
            SlashCommand::Settings => "configure realtime microphone/speaker",
            SlashCommand::Plan => "switch to Plan mode",
            SlashCommand::Goal => "set or view the goal for a long-running task",
            SlashCommand::Collab => "change collaboration mode (experimental)",
            SlashCommand::Agent | SlashCommand::MultiAgents => "switch the active agent thread",
            SlashCommand::Side => "start a side conversation in an ephemeral branch",
            SlashCommand::Approvals => "choose what VAC is allowed to do",
            SlashCommand::Permissions => "choose what VAC is allowed to do",
            SlashCommand::Keymap => "remap TUI shortcuts",
            SlashCommand::Vim => "toggle Vim mode for the composer",
            SlashCommand::ElevateSandbox => "set up elevated agent sandbox",
            SlashCommand::SandboxReadRoot => {
                "let sandbox read a directory: /sandbox-add-read-dir <absolute_path>"
            }
            SlashCommand::Experimental => "toggle experimental features",
            SlashCommand::AutoReview => "approve one retry of a recent auto-review denial",
            SlashCommand::Memories => "configure memory use and generation",
            SlashCommand::Mcp => "list configured MCP tools; use /mcp verbose for details",
            SlashCommand::Apps => "manage apps",
            SlashCommand::Plugins => "browse plugins",
            SlashCommand::Logout => "log out of VAC",
            SlashCommand::Rollout => "print the rollout file path",
            SlashCommand::TestApproval => "test approval request",
        }
    }

    /// Command string without the leading '/'. Provided for compatibility with
    /// existing code that expects a method named `command()`.
    pub fn command(self) -> &'static str {
        self.into()
    }

    /// Whether this command supports inline args (for example `/review ...`).
    pub fn supports_inline_args(self) -> bool {
        matches!(
            self,
            SlashCommand::Review
                | SlashCommand::Rename
                | SlashCommand::Plan
                | SlashCommand::Goal
                | SlashCommand::Fast
                | SlashCommand::Ide
                | SlashCommand::Mcp
                | SlashCommand::Workflow
                | SlashCommand::Side
                | SlashCommand::Resume
                | SlashCommand::SandboxReadRoot
        )
    }

    /// Whether this command remains available inside an active side conversation.
    pub fn available_in_side_conversation(self) -> bool {
        matches!(
            self,
            SlashCommand::Copy
                | SlashCommand::Diff
                | SlashCommand::Mention
                | SlashCommand::Status
                | SlashCommand::Ide
        )
    }

    /// Whether this command can be run while a task is in progress.
    pub fn available_during_task(self) -> bool {
        match self {
            SlashCommand::New
            | SlashCommand::Resume
            | SlashCommand::Branch
            | SlashCommand::Init
            | SlashCommand::Compact
            | SlashCommand::Model
            | SlashCommand::Fast
            | SlashCommand::Personality
            | SlashCommand::Approvals
            | SlashCommand::Permissions
            | SlashCommand::Keymap
            | SlashCommand::Vim
            | SlashCommand::ElevateSandbox
            | SlashCommand::SandboxReadRoot
            | SlashCommand::Experimental
            | SlashCommand::Memories
            | SlashCommand::Review
            | SlashCommand::Plan
            | SlashCommand::Clear
            | SlashCommand::Logout
            | SlashCommand::MemoryDrop
            | SlashCommand::MemoryUpdate => false,
            SlashCommand::Diff
            | SlashCommand::Copy
            | SlashCommand::Rename
            | SlashCommand::Mention
            | SlashCommand::Skills
            | SlashCommand::Hooks
            | SlashCommand::Status
            | SlashCommand::Runtime
            | SlashCommand::Capabilities
            | SlashCommand::Workflow
            | SlashCommand::DebugConfig
            | SlashCommand::Ps
            | SlashCommand::Stop
            | SlashCommand::Goal
            | SlashCommand::Mcp
            | SlashCommand::Apps
            | SlashCommand::Plugins
            | SlashCommand::Title
            | SlashCommand::Statusline
            | SlashCommand::AutoReview
            | SlashCommand::Feedback
            | SlashCommand::Ide
            | SlashCommand::Quit
            | SlashCommand::Exit
            | SlashCommand::Side
            | SlashCommand::Activity => true,
            SlashCommand::Rollout => true,
            SlashCommand::TestApproval => true,
            SlashCommand::Realtime => true,
            SlashCommand::Settings => true,
            SlashCommand::Collab => true,
            SlashCommand::Agent | SlashCommand::MultiAgents => true,
            SlashCommand::Theme => false,
        }
    }

    fn is_visible(self) -> bool {
        match self {
            SlashCommand::MemoryDrop | SlashCommand::MemoryUpdate => false,
            SlashCommand::SandboxReadRoot => cfg!(target_os = "windows"),
            SlashCommand::Copy => !cfg!(target_os = "android"),
            SlashCommand::Rollout | SlashCommand::TestApproval => cfg!(debug_assertions),
            _ => true,
        }
    }
}

/// Return all built-in commands in a Vec paired with their command string.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::iter()
        .filter(|command| command.is_visible())
        .map(|c| (c.command(), c))
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    use super::SlashCommand;
    use super::built_in_slash_commands;
    use crate::surface_route_catalog::SurfaceRouteCatalog;

    #[test]
    fn all_visible_builtin_commands_are_declared_in_slash_surface_manifest() {
        let root = std::env::current_dir().expect("current dir");
        let catalog = SurfaceRouteCatalog::load(root);
        let missing = built_in_slash_commands()
            .into_iter()
            .filter_map(|(name, _)| {
                catalog
                    .slash_route(name)
                    .is_none()
                    .then(|| format!("/{name}"))
            })
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "missing slash routes in .vac/surfaces/slash.yaml: {}",
            missing.join(", ")
        );
    }

    #[test]
    fn stop_command_is_canonical_name() {
        assert_eq!(SlashCommand::Stop.command(), "stop");
    }

    #[test]
    fn clean_alias_parses_to_stop_command() {
        assert_eq!(SlashCommand::from_str("clean"), Ok(SlashCommand::Stop));
    }

    #[test]
    fn clean_alias_is_declared_as_first_class_slash_route() {
        let root = std::env::current_dir().expect("current dir");
        let catalog = SurfaceRouteCatalog::load(root);
        assert!(
            catalog.slash_route("clean").is_some(),
            "/clean alias must remain a first-class slash surface route"
        );
    }

    #[test]
    fn certain_commands_are_available_during_task() {
        assert!(SlashCommand::Goal.available_during_task());
        assert!(SlashCommand::Ide.available_during_task());
        assert!(SlashCommand::Title.available_during_task());
        assert!(SlashCommand::Statusline.available_during_task());
    }

    #[test]
    fn auto_review_command_is_autoreview() {
        assert_eq!(SlashCommand::AutoReview.command(), "autoreview");
        assert_eq!(
            SlashCommand::from_str("autoreview"),
            Ok(SlashCommand::AutoReview)
        );
    }
}


#[cfg(test)]
pub(crate) fn palette_badge_for_benchmark(command: &str, available: bool) -> String {
    if available {
        format!("{command} [ready]")
    } else {
        format!("{command} [feature-off]")
    }
}
