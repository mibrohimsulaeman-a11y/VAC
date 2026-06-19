use crate::utils::plugins::{PluginConfig, execute_plugin_command, get_plugin_path};
use std::process::Command;

fn get_board_plugin_config() -> PluginConfig {
    PluginConfig {
        name: "agent-board".to_string(),
        base_url: "https://github.com/mibrohimsulaeman-a11y/VAC".to_string(),
        targets: vec![
            "linux-x86_64".to_string(),
            "windows-x86_64".to_string(),
            "darwin-x86_64".to_string(),
            "darwin-aarch64".to_string(),
        ],
        version: None,
        repo: Some("VAC".to_string()),
        owner: Some("mibrohimsulaeman-a11y".to_string()),
        version_arg: None,
        prefer_server_version: false,
    }
}

/// Pass-through to agent-board plugin. All args after 'board' are forwarded directly.
/// Run `vac board --help` for available commands.
pub async fn run_board(args: Vec<String>) -> Result<(), String> {
    let config = get_board_plugin_config();
    let board_path = get_plugin_path(config).await;

    let mut cmd = Command::new(board_path);
    cmd.args(&args);
    execute_plugin_command(cmd, "agent-board".to_string())
}
