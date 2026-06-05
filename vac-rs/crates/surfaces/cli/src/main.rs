use crate::apply_cli::ApplyCommand;
use crate::apply_cli::run_apply_command;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::Shell;
use clap_complete::generate;
use owo_colors::OwoColorize;
use std::io::IsTerminal;
use supports_color::Stream;
use vac_arg0::Arg0DispatchPaths;
use vac_arg0::arg0_dispatch_or_else;
use vac_core::project_workspace::build_project_workspace_confirmation_dialog;
use vac_core::project_workspace::project_workspace_startup_notice;
#[cfg(feature = "exec-runtime")]
use vac_exec::Cli as ExecCli;
#[cfg(feature = "exec-runtime")]
use vac_exec::Command as ExecCommand;
#[cfg(feature = "exec-runtime")]
use vac_exec::ReviewArgs;
use vac_tui::AppExitInfo;
use vac_tui::Cli as TuiCli;
use vac_tui::ExitReason;
use vac_tui::UpdateAction;
use vac_utils_cli::CliConfigOverrides;

mod apply_cli;
mod approve_cli;
mod auth_cli;
mod doctor_cli;
mod enforcement_cli;
mod init_cli;
mod plan_cli;
mod registry_cli;
mod session_cli;
mod why_cli;
mod workflow_cli;

#[cfg(not(windows))]
mod wsl_paths;

/// VAC CLI
///
/// If no subcommand is specified, VAC opens the root interactive TUI.
#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    subcommand_negates_reqs = true,
    bin_name = "vac",
    override_usage = "vac [OPTIONS] [PROMPT]\n       vac [OPTIONS] <COMMAND> [ARGS]"
)]
struct VacCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    interactive: TuiCli,

    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Run VAC non-interactively.
    #[cfg(feature = "exec-runtime")]
    #[clap(visible_alias = "e")]
    Exec(ExecCli),

    /// Run a code review non-interactively.
    #[cfg(feature = "exec-runtime")]
    Review(ReviewArgs),

    /// Apply the latest diff produced by VAC agent as a `git apply` to your local working tree.
    #[clap(visible_alias = "a")]
    Apply(ApplyCommand),

    /// Configure authentication and model providers.
    Auth(auth_cli::AuthCommand),

    /// Respond to local VAC approval requests.
    Approve(approve_cli::ApproveCommand),

    /// Inspect VAC control-plane manifests and diagnostics.
    Doctor(doctor_cli::DoctorCommand),

    /// Inspect VAC enforcement claims and substrate observations.
    Enforcement(enforcement_cli::EnforcementCommand),

    /// Bootstrap or inspect the VAC-Init workflow control plane.
    Init(init_cli::InitCommand),

    /// Validate and inspect bounded Semantic Plans.
    Plan(plan_cli::PlanCommand),

    /// Manage VAC registry migrations.
    Registry(registry_cli::RegistryCommand),

    /// Explain safe rationale for a file, line, range, or symbol.
    Why(why_cli::WhyCommand),

    /// Inspect or run VAC control-plane workflows.
    Workflow(workflow_cli::WorkflowCommand),

    /// Resume a previous interactive session (picker by default; use --last to continue the most recent).
    Resume(ResumeCommand),

    /// Create, inspect, or close root VAC session artifacts.
    Session(session_cli::SessionCommand),

    /// Generate shell completion scripts.
    Completion(CompletionCommand),
}

#[derive(Debug, Parser)]
struct CompletionCommand {
    /// Shell to generate completions for.
    #[clap(value_enum, default_value_t = Shell::Bash)]
    shell: Shell,
}

#[derive(Debug, Parser)]
struct ResumeCommand {
    /// Conversation/session id (UUID) or thread name. UUIDs take precedence if it parses.
    /// If omitted, use --last to pick the most recent recorded session.
    #[arg(value_name = "SESSION_ID")]
    session_id: Option<String>,

    /// Continue the most recent session without showing the picker.
    #[arg(long = "last", default_value_t = false)]
    last: bool,

    /// Show all sessions (disables cwd filtering and shows CWD column).
    #[arg(long = "all", default_value_t = false)]
    all: bool,

    /// Include non-interactive sessions in the resume picker and --last selection.
    #[arg(long = "include-non-interactive", default_value_t = false)]
    include_non_interactive: bool,

    #[clap(flatten)]
    config_overrides: TuiCli,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        cli_main(arg0_paths).await?;
        Ok(())
    })
}

async fn cli_main(arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
    let VacCli {
        config_overrides: root_config_overrides,
        mut interactive,
        subcommand,
    } = VacCli::parse();

    match subcommand {
        None => {
            prepend_config_flags(
                &mut interactive.config_overrides,
                root_config_overrides.clone(),
            );
            let exit_info = run_interactive_tui(interactive, arg0_paths.clone()).await?;
            handle_app_exit(exit_info)?;
        }
        #[cfg(feature = "exec-runtime")]
        Some(Subcommand::Exec(mut exec_cli)) => {
            exec_cli
                .shared
                .inherit_exec_root_options(&interactive.shared);
            prepend_config_flags(
                &mut exec_cli.config_overrides,
                root_config_overrides.clone(),
            );
            vac_exec::run_main(exec_cli, arg0_paths.clone()).await?;
        }
        #[cfg(feature = "exec-runtime")]
        Some(Subcommand::Review(review_args)) => {
            let mut exec_cli = ExecCli::try_parse_from(["vac", "exec"])?;
            exec_cli.command = Some(ExecCommand::Review(review_args));
            prepend_config_flags(
                &mut exec_cli.config_overrides,
                root_config_overrides.clone(),
            );
            vac_exec::run_main(exec_cli, arg0_paths.clone()).await?;
        }
        Some(Subcommand::Apply(mut apply_cli)) => {
            prepend_config_flags(
                &mut apply_cli.config_overrides,
                root_config_overrides.clone(),
            );
            run_apply_command(apply_cli, /*cwd*/ None).await?;
        }
        Some(Subcommand::Approve(approve_cli)) => {
            approve_cli.run()?;
        }
        Some(Subcommand::Auth(auth_cli)) => match auth_cli.run().await? {
            auth_cli::AuthAction::LaunchExistingLogin {
                forced_login_method,
            } => {
                interactive
                    .config_overrides
                    .raw_overrides
                    .push(format!("forced_login_method={}", forced_login_method));
                interactive
                    .config_overrides
                    .raw_overrides
                    .push("model_provider=\"vastar\"".to_string());
                let exit_info = run_interactive_tui(interactive, arg0_paths.clone()).await?;
                handle_app_exit(exit_info)?;
            }
            auth_cli::AuthAction::ConfiguredProvider => {}
        },
        Some(Subcommand::Doctor(doctor_cli)) => {
            let code = doctor_cli.run()?;
            if code != 0 {
                std::process::exit(code);
            }
        }
        Some(Subcommand::Enforcement(enforcement_cli)) => {
            enforcement_cli.run()?;
        }
        Some(Subcommand::Init(init_cli)) => {
            init_cli.run()?;
        }
        Some(Subcommand::Plan(plan_cli)) => {
            plan_cli.run()?;
        }
        Some(Subcommand::Registry(registry_cli)) => {
            registry_cli.run()?;
        }
        Some(Subcommand::Why(why_cli)) => {
            why_cli.run()?;
        }
        Some(Subcommand::Workflow(workflow_cli)) => {
            workflow_cli.run()?;
        }
        Some(Subcommand::Resume(ResumeCommand {
            session_id,
            last,
            all,
            include_non_interactive,
            config_overrides,
        })) => {
            interactive = finalize_resume_interactive(
                interactive,
                root_config_overrides.clone(),
                session_id,
                last,
                all,
                include_non_interactive,
                config_overrides,
            );
            let exit_info = run_interactive_tui(interactive, arg0_paths.clone()).await?;
            handle_app_exit(exit_info)?;
        }
        Some(Subcommand::Session(session_cli)) => {
            session_cli.run()?;
        }
        Some(Subcommand::Completion(completion_cli)) => {
            print_completion(completion_cli);
        }
    }

    Ok(())
}

fn format_exit_messages(exit_info: AppExitInfo, color_enabled: bool) -> Vec<String> {
    let AppExitInfo {
        token_usage,
        thread_id: conversation_id,
        ..
    } = exit_info;

    let mut lines = Vec::new();
    if !token_usage.is_zero() {
        lines.push(token_usage.to_string());
    }

    if let Some(resume_cmd) =
        vac_core::util::resume_command(/*thread_name*/ None, conversation_id)
    {
        let command = if color_enabled {
            resume_cmd.cyan().to_string()
        } else {
            resume_cmd
        };
        lines.push(format!("To continue this session, run {command}"));
    }

    lines
}

/// Handle the app exit and print the results. Optionally run the update action.
fn handle_app_exit(exit_info: AppExitInfo) -> anyhow::Result<()> {
    match exit_info.exit_reason {
        ExitReason::Fatal(message) => {
            eprintln!("ERROR: {message}");
            std::process::exit(1);
        }
        ExitReason::UserRequested => { /* normal exit */ }
    }

    let update_action = exit_info.update_action;
    let color_enabled = supports_color::on(Stream::Stdout).is_some();
    for line in format_exit_messages(exit_info, color_enabled) {
        println!("{line}");
    }
    if let Some(action) = update_action {
        run_update_action(action)?;
    }
    Ok(())
}

/// Run the update action returned by the TUI and print the result.
fn run_update_action(action: UpdateAction) -> anyhow::Result<()> {
    println!();
    let cmd_str = action.command_str();
    println!("Updating VAC via `{cmd_str}`...");

    let status = {
        #[cfg(windows)]
        {
            if action == UpdateAction::StandaloneWindows {
                let (cmd, args) = action.command_args();
                std::process::Command::new(cmd).args(args).status()?
            } else {
                std::process::Command::new("cmd")
                    .args(["/C", &cmd_str])
                    .status()?
            }
        }
        #[cfg(not(windows))]
        {
            let (cmd, args) = action.command_args();
            let command_path = crate::wsl_paths::normalize_for_wsl(cmd);
            let normalized_args: Vec<String> = args
                .iter()
                .map(crate::wsl_paths::normalize_for_wsl)
                .collect();
            std::process::Command::new(&command_path)
                .args(&normalized_args)
                .status()?
        }
    };
    if !status.success() {
        anyhow::bail!("`{cmd_str}` failed with status {status}");
    }
    println!("\nUpdate ran successfully. Please restart VAC.");
    Ok(())
}

async fn run_interactive_tui(
    mut interactive: TuiCli,
    arg0_paths: Arg0DispatchPaths,
) -> std::io::Result<AppExitInfo> {
    if let Some(prompt) = interactive.prompt.take() {
        // Normalize CRLF/CR to LF so CLI-provided text can't leak `\r` into TUI state.
        interactive.prompt = Some(prompt.replace("\r\n", "\n").replace('\r', "\n"));
    }

    if let Some(preflight) = project_workspace_cli_preflight(&interactive) {
        eprintln!("{preflight}");
    }

    let terminal_info = vac_terminal_detection::terminal_info();
    if terminal_info.name == vac_terminal_detection::TerminalName::Dumb {
        if !(std::io::stdin().is_terminal() && std::io::stderr().is_terminal()) {
            return Ok(AppExitInfo::fatal(
                "TERM is set to \"dumb\". Refusing to start the interactive TUI because no terminal is available for a confirmation prompt (stdin/stderr is not a TTY). Run in a supported terminal or unset TERM.",
            ));
        }

        eprintln!(
            "WARNING: TERM is set to \"dumb\". VAC's interactive TUI may not work in this terminal."
        );
        if !confirm("Continue anyway? [y/N]: ")? {
            return Ok(AppExitInfo::fatal(
                "Refusing to start the interactive TUI because TERM is set to \"dumb\". Run in a supported terminal or unset TERM.",
            ));
        }
    }

    vac_tui::run_main(
        interactive,
        arg0_paths,
        vac_config::LoaderOverrides::default(),
    )
    .await
}

fn project_workspace_cli_preflight(interactive: &TuiCli) -> Option<String> {
    let root = interactive
        .cwd
        .clone()
        .or_else(|| std::env::current_dir().ok())?;
    project_workspace_startup_notice(&root).map(|notice| {
        let mut preflight = notice.render_cli_preflight();
        if let Some(dialog) = build_project_workspace_confirmation_dialog(&root) {
            preflight.push_str(
                "
",
            );
            preflight.push_str(&dialog.render_text());
        }
        preflight
    })
}

fn confirm(prompt: &str) -> std::io::Result<bool> {
    eprintln!("{prompt}");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let answer = input.trim();
    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}

/// Build the final `TuiCli` for a `vac resume` invocation.
fn finalize_resume_interactive(
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    session_id: Option<String>,
    last: bool,
    show_all: bool,
    include_non_interactive: bool,
    mut resume_cli: TuiCli,
) -> TuiCli {
    // Start with the parsed interactive CLI so resume shares the same
    // configuration surface area as `vac` without additional flags.
    // Clap cannot express the conditional positional meaning cleanly: with
    // `--last`, the first positional is a follow-up prompt, not a session id.
    let (resume_session_id, resume_prompt) = if last && resume_cli.prompt.is_none() {
        (None, session_id)
    } else {
        (session_id, resume_cli.prompt)
    };
    resume_cli.prompt = resume_prompt;
    interactive.resume_picker = resume_session_id.is_none() && !last;
    interactive.resume_last = last;
    interactive.resume_session_id = resume_session_id;
    interactive.resume_show_all = show_all;
    interactive.resume_include_non_interactive = include_non_interactive;

    // Merge resume-scoped flags and overrides with highest precedence.
    merge_interactive_cli_flags(&mut interactive, resume_cli);

    // Propagate any root-level config overrides (e.g. `-c key=value`).
    prepend_config_flags(&mut interactive.config_overrides, root_config_overrides);

    interactive
}

/// Merge flags provided to `vac resume` so they take precedence over any
/// root-level flags. Only overrides fields explicitly set on the subcommand-scoped
/// CLI. Also appends `-c key=value` overrides with highest precedence.
fn merge_interactive_cli_flags(interactive: &mut TuiCli, subcommand_cli: TuiCli) {
    let TuiCli {
        shared,
        approval_policy,
        web_search,
        prompt,
        config_overrides,
        ..
    } = subcommand_cli;
    interactive
        .shared
        .apply_subcommand_overrides(shared.into_inner());
    if let Some(approval) = approval_policy {
        interactive.approval_policy = Some(approval);
    }
    if web_search {
        interactive.web_search = true;
    }
    if let Some(prompt) = prompt {
        // Normalize CRLF/CR to LF so CLI-provided text can't leak `\r` into TUI state.
        interactive.prompt = Some(prompt.replace("\r\n", "\n").replace('\r', "\n"));
    }

    interactive
        .config_overrides
        .raw_overrides
        .extend(config_overrides.raw_overrides);
}

/// Prepend root-level overrides so they have lower precedence than
/// CLI-specific ones specified after the subcommand (if any).
fn prepend_config_flags(
    subcommand_config_overrides: &mut CliConfigOverrides,
    cli_config_overrides: CliConfigOverrides,
) {
    subcommand_config_overrides
        .raw_overrides
        .splice(0..0, cli_config_overrides.raw_overrides);
}

fn print_completion(cmd: CompletionCommand) {
    let mut app = VacCli::command();
    let name = "vac";
    generate(cmd.shell, &mut app, name, &mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};
    use vac_protocol::ThreadId;
    use vac_tui::TokenUsage;

    fn temp_project_root(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vac-cli-project-workspace-{name}-{unique}"));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn load_cli_surface_manifest() -> String {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../..")
            .join(".vac/surfaces/cli.yaml");
        fs::read_to_string(&manifest_path)
            .unwrap_or_else(|err| panic!("read {}: {err}", manifest_path.display()))
    }

    fn assert_cli_surface_declares(manifest: &str, command: &str) {
        let expected = format!("command: {command}");
        assert!(
            manifest.lines().any(|line| line.trim() == expected),
            "missing CLI surface route `{command}` in .vac/surfaces/cli.yaml"
        );
    }

    fn finalize_resume_from_args(args: &[&str]) -> TuiCli {
        let cli = VacCli::try_parse_from(args).expect("parse");
        let VacCli {
            interactive,
            config_overrides: root_overrides,
            subcommand,
        } = cli;

        let Subcommand::Resume(ResumeCommand {
            session_id,
            last,
            all,
            include_non_interactive,
            config_overrides: resume_cli,
        }) = subcommand.expect("resume present")
        else {
            unreachable!()
        };

        finalize_resume_interactive(
            interactive,
            root_overrides,
            session_id,
            last,
            all,
            include_non_interactive,
            resume_cli,
        )
    }

    #[cfg(feature = "exec-runtime")]
    #[test]
    fn exec_resume_last_accepts_prompt_positional() {
        let cli = ExecCli::try_parse_from(["vac", "exec", "resume", "--last", "hello"])
            .expect("parse exec resume");
        assert_matches!(cli.command, Some(ExecCommand::Resume(_)));
    }

    #[test]
    fn resume_root_flags_are_preserved() {
        // `resume` flattens the current TUI flag surface. `--approval-policy`
        // was a stale test-only spelling present in the VAC fork baseline
        // (4ae59e3/ab33524); the public flag is `--ask-for-approval`.
        let interactive = finalize_resume_from_args(&[
            "vac",
            "--model",
            "gpt-5",
            "resume",
            "--last",
            "--ask-for-approval",
            "on-request",
            "continue work",
        ]);

        assert!(interactive.resume_last);
        assert_eq!(interactive.resume_session_id, None);
        assert_eq!(interactive.prompt.as_deref(), Some("continue work"));
        assert_matches!(
            interactive.approval_policy,
            Some(vac_utils_cli::ApprovalModeCliArg::OnRequest)
        );
        assert_eq!(
            interactive.shared.into_inner().model.as_deref(),
            Some("gpt-5")
        );
    }

    #[test]
    fn resume_with_session_id_disables_picker() {
        let interactive = finalize_resume_from_args(&["vac", "resume", "session-123"]);

        assert!(!interactive.resume_picker);
        assert_eq!(
            interactive.resume_session_id.as_deref(),
            Some("session-123")
        );
    }

    #[test]
    fn resume_without_target_enables_picker() {
        let interactive = finalize_resume_from_args(&["vac", "resume"]);

        assert!(interactive.resume_picker);
        assert!(!interactive.resume_last);
        assert_eq!(interactive.resume_session_id, None);
    }

    #[test]
    fn top_level_cli_commands_are_declared_in_cli_surface_manifest() {
        let manifest = load_cli_surface_manifest();
        assert_cli_surface_declares(&manifest, "vac");

        for command in VacCli::command().get_subcommands() {
            assert_cli_surface_declares(&manifest, &format!("vac {}", command.get_name()));
        }

        for alias in ["vac e", "vac a"] {
            assert_cli_surface_declares(&manifest, alias);
        }
    }

    #[test]
    fn completion_surface_is_small() {
        let commands = VacCli::command()
            .get_subcommands()
            .map(|command| command.get_name().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            commands,
            vec![
                "apply",
                "auth",
                "approve",
                "doctor",
                "init",
                "plan",
                "registry",
                "why",
                "workflow",
                "resume",
                "session",
                "completion",
            ]
        );
    }

    #[test]
    fn interactive_preflight_warns_for_missing_vac_without_blocking_prompt() {
        let root = temp_project_root("missing-vac-preflight");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("cargo");
        let mut cli = TuiCli::try_parse_from(["vac", "hello"]).expect("parse tui");
        cli.cwd = Some(root.clone());

        let preflight = project_workspace_cli_preflight(&cli).expect("preflight");

        assert!(preflight.contains("ordinary_prompt_allowed: true"));
        assert!(preflight.contains("--bootstrap-soft --yes"));
        assert_eq!(cli.prompt.as_deref(), Some("hello"));

        fs::create_dir(root.join(".vac")).expect("create vac");
        assert_eq!(project_workspace_cli_preflight(&cli), None);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn exit_messages_include_resume_hint() {
        let exit_info = AppExitInfo {
            token_usage: TokenUsage::default(),
            thread_id: Some(ThreadId::new()),
            thread_name: None,
            update_action: None,
            exit_reason: ExitReason::UserRequested,
        };

        let lines = format_exit_messages(exit_info, false);

        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("To continue this session, run vac resume "));
    }
}
