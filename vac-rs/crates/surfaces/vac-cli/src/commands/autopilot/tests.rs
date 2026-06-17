use super::service::{build_systemd_exec_start, shell_join};
use super::*;
fn temp_file_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "vac-{}-{}-{}.toml",
        name,
        std::process::id(),
        nanos
    ))
}

fn write_profile_config(path: &Path, content: &str) {
    let write_result = std::fs::write(path, content);
    assert!(write_result.is_ok());
}

fn test_app_config(config_path: &Path) -> AppConfig {
    AppConfig {
        api_endpoint: "https://test".to_string(),
        api_key: None,
        provider: crate::config::ProviderType::Remote,
        mcp_server_host: None,
        machine_name: None,
        auto_append_gitignore: None,
        profile_name: String::new(),
        config_path: config_path.to_string_lossy().to_string(),
        allowed_tools: None,
        auto_approve: None,
        subagent: None,
        rulebooks: None,
        warden: None,
        providers: std::collections::HashMap::new(),
        model: None,
        system_prompt: None,
        max_turns: None,
        anonymous_id: None,
        collect_telemetry: None,
        editor: None,
        recent_models: Vec::new(),
    }
}

#[test]
fn config_roundtrip_save_load() {
    let path = temp_file_path("autopilot-config");

    let mut config = AutopilotConfigFile::default();
    config.server.listen = "0.0.0.0:4111".to_string();
    config.server.show_token = true;
    config.server.no_auth = true;
    config.server.model = Some("anthropic/claude-sonnet-4-5".to_string());
    config.server.auto_approve_all = true;

    let save_result = config.save_to_path(&path);
    assert!(save_result.is_ok());

    let loaded = AutopilotConfigFile::load_from_path(&path);
    assert!(loaded.is_ok());

    if let Ok(loaded) = loaded {
        assert_eq!(loaded.server.listen, "0.0.0.0:4111");
        assert!(loaded.server.show_token);
        assert!(loaded.server.no_auth);
        assert_eq!(
            loaded.server.model.as_deref(),
            Some("anthropic/claude-sonnet-4-5")
        );
        assert!(loaded.server.auto_approve_all);
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn loopback_base_url_resolves_unspecified_bind() {
    let v4 = loopback_base_url_from_bind("0.0.0.0:4096");
    let v6 = loopback_base_url_from_bind("[::]:4096");

    assert_eq!(v4, "http://127.0.0.1:4096");
    assert_eq!(v6, "http://[::1]:4096");
}

fn sample_schedule(name: &str) -> AutopilotScheduleConfig {
    AutopilotScheduleConfig {
        name: name.to_string(),
        cron: "*/5 * * * *".to_string(),
        prompt: "Check infra".to_string(),
        check: None,
        trigger_on: ScheduleTriggerOn::Failure,
        // workdir: None,
        max_steps: 50,
        channel: None,
        profile: None,
        pause_on_approval: false,
        sandbox: false,
        enabled: true,
    }
}

#[test]
fn schedule_add_remove_enable_disable_happy_path() {
    let mut config = AutopilotConfigFile::default();

    let add_result = add_schedule_in_config(&mut config, sample_schedule("health-check"));
    assert!(add_result.is_ok());
    assert_eq!(config.schedules.len(), 1);

    let disable_result = set_schedule_enabled_in_config(&mut config, "health-check", false);
    assert!(disable_result.is_ok());
    assert!(!config.schedules[0].enabled);

    let enable_result = set_schedule_enabled_in_config(&mut config, "health-check", true);
    assert!(enable_result.is_ok());
    assert!(config.schedules[0].enabled);

    let remove_result = remove_schedule_in_config(&mut config, "health-check");
    assert!(remove_result.is_ok());
    assert!(config.schedules.is_empty());
}

#[test]
fn schedule_duplicate_name_rejected() {
    let mut config = AutopilotConfigFile::default();

    let first = add_schedule_in_config(&mut config, sample_schedule("drift-detect"));
    assert!(first.is_ok());

    let duplicate = add_schedule_in_config(&mut config, sample_schedule("drift-detect"));
    assert!(duplicate.is_err());
}

#[test]
fn schedule_invalid_cron_rejected() {
    let mut config = AutopilotConfigFile::default();
    let mut schedule = sample_schedule("broken");
    schedule.cron = "invalid cron".to_string();

    let result = add_schedule_in_config(&mut config, schedule);
    assert!(result.is_err());
}

#[test]
fn schedule_reserved_name_rejected() {
    let mut config = AutopilotConfigFile::default();
    let schedule = sample_schedule(crate::commands::watch::RELOAD_SENTINEL);

    let result = add_schedule_in_config(&mut config, schedule);
    assert!(result.is_err());
    let message = result.expect_err("reserved schedule name should be rejected");
    assert!(message.contains("reserved"));
}

#[test]
fn schedule_missing_check_script_rejected() {
    let mut config = AutopilotConfigFile::default();
    let mut schedule = sample_schedule("missing-check");
    let missing = temp_file_path("autopilot-missing-check-script");
    let _ = std::fs::remove_file(&missing);
    schedule.check = Some(missing.to_string_lossy().to_string());

    let result = add_schedule_in_config(&mut config, schedule);
    assert!(result.is_err());
    let message = result.expect_err("missing check script should be rejected");
    assert!(message.contains("Check script not found"));
}

#[test]
fn schedule_existing_check_script_is_accepted() {
    let mut config = AutopilotConfigFile::default();
    let mut schedule = sample_schedule("existing-check");
    let script_path = temp_file_path("autopilot-existing-check-script");
    std::fs::write(&script_path, "#!/bin/sh\necho ok\n").expect("write check script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
            .expect("set executable permission");
    }
    schedule.check = Some(script_path.to_string_lossy().to_string());

    let result = add_schedule_in_config(&mut config, schedule);
    assert!(result.is_ok());
    assert_eq!(config.schedules.len(), 1);

    let _ = std::fs::remove_file(script_path);
}

#[test]
fn history_limit_is_bounded() {
    assert_eq!(bounded_history_limit(0), 1);
    assert_eq!(bounded_history_limit(20), 20);
    assert_eq!(bounded_history_limit(10_000), 1000);
}

#[test]
fn load_ignores_gateway_channel_schema() {
    let path = temp_file_path("autopilot-gateway-channels");
    let write_result = std::fs::write(
        &path,
        r##"
[server]
listen = "127.0.0.1:4096"

[channels.slack]
bot_token = "xoxb-test"
app_token = "xapp-test"
"##,
    );
    assert!(write_result.is_ok());

    let loaded = AutopilotConfigFile::load_from_path(&path);
    assert!(loaded.is_ok());

    let _ = std::fs::remove_file(path);
}

#[test]
fn server_config_save_preserves_gateway_and_notifications_sections() {
    let path = temp_file_path("autopilot-preserve");
    let write_result = std::fs::write(
        &path,
        r##"
[server]
listen = "127.0.0.1:4096"
url = "http://127.0.0.1:4096"
token = "gateway-token"

[notifications]
gateway_url = "http://127.0.0.1:4096"
channel = "slack"
chat_id = "#infra"

[channels.slack]
bot_token = "xoxb-old"
app_token = "xapp-old"
"##,
    );
    assert!(write_result.is_ok());

    let load_result = AutopilotConfigFile::load_from_path(&path);
    assert!(load_result.is_ok());
    let mut loaded = match load_result {
        Ok(value) => value,
        Err(error) => panic!("failed to load config: {error}"),
    };

    loaded.server.auto_approve_all = true;
    let save_updated = loaded.save_to_path(&path);
    assert!(save_updated.is_ok());

    let reloaded = std::fs::read_to_string(&path);
    assert!(reloaded.is_ok());
    let reloaded = match reloaded {
        Ok(value) => value,
        Err(error) => panic!("failed to read config: {error}"),
    };

    assert!(reloaded.contains("[channels.slack]"));
    assert!(reloaded.contains("bot_token = \"xoxb-old\""));
    assert!(reloaded.contains("[notifications]"));
    assert!(reloaded.contains("channel = \"slack\""));
    assert!(reloaded.contains("chat_id = \"#infra\""));
    assert!(reloaded.contains("auto_approve_all = true"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn set_default_notification_target_merges_without_overwrite() {
    let path = temp_file_path("autopilot-notification-target");
    let write_result = std::fs::write(
        &path,
        r##"
[server]
listen = "127.0.0.1:4096"

[[schedules]]
name = "health-check"
cron = "*/5 * * * *"
prompt = "Check system health"

[channels.slack]
bot_token = "xoxb-test"
app_token = "xapp-test"
"##,
    );
    assert!(write_result.is_ok());

    let set_result = set_default_notification_target(path.as_path(), "slack", "#ops");
    assert!(set_result.is_ok());

    let reloaded = std::fs::read_to_string(&path);
    assert!(reloaded.is_ok());
    let reloaded = match reloaded {
        Ok(value) => value,
        Err(error) => panic!("failed to read config: {error}"),
    };

    assert!(reloaded.contains("[[schedules]]"));
    assert!(reloaded.contains("[channels.slack]"));
    assert!(reloaded.contains("[notifications]"));
    assert!(reloaded.contains("channel = \"slack\""));
    assert!(reloaded.contains("chat_id = \"#ops\""));

    let _ = std::fs::remove_file(path);
}

#[test]
fn channel_add_with_target_updates_notifications() {
    let path = temp_file_path("autopilot-channel-add-target");
    let write_result = std::fs::write(
        &path,
        r##"
[server]
listen = "127.0.0.1:4096"

[[schedules]]
name = "health-check"
cron = "*/5 * * * *"
prompt = "Check system health"
"##,
    );
    assert!(write_result.is_ok());

    let add_result = add_channel_with_optional_target(
        path.as_path(),
        ChannelType::Slack,
        None,
        Some("xoxb-test".to_string()),
        Some("xapp-test".to_string()),
        Some("#eng".to_string()),
        None,
    );
    assert!(add_result.is_ok());
    assert_eq!(add_result.ok(), Some(Some("#eng".to_string())));

    let reloaded = std::fs::read_to_string(&path);
    assert!(reloaded.is_ok());
    let reloaded = match reloaded {
        Ok(value) => value,
        Err(error) => panic!("failed to read config: {error}"),
    };

    assert!(reloaded.contains("[channels.slack]"));
    assert!(reloaded.contains("bot_token = \"xoxb-test\""));
    assert!(reloaded.contains("app_token = \"xapp-test\""));
    assert!(reloaded.contains("[notifications]"));
    assert!(reloaded.contains("channel = \"slack\""));
    assert!(reloaded.contains("chat_id = \"#eng\""));
    assert!(reloaded.contains("[[schedules]]"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn channel_add_with_invalid_target_is_atomic() {
    let path = temp_file_path("autopilot-channel-add-invalid-target");
    let write_result = std::fs::write(
        &path,
        r##"
[server]
listen = "127.0.0.1:4096"

[[schedules]]
name = "health-check"
cron = "*/5 * * * *"
prompt = "Check system health"
"##,
    );
    assert!(write_result.is_ok());

    let add_result = add_channel_with_optional_target(
        path.as_path(),
        ChannelType::Slack,
        None,
        Some("xoxb-test".to_string()),
        Some("xapp-test".to_string()),
        Some("   ".to_string()),
        None,
    );
    assert!(add_result.is_err());

    let reloaded = std::fs::read_to_string(&path);
    assert!(reloaded.is_ok());
    let reloaded = match reloaded {
        Ok(value) => value,
        Err(error) => panic!("failed to read config: {error}"),
    };

    assert!(!reloaded.contains("[channels.slack]"));
    assert!(!reloaded.contains("[notifications]"));
    assert!(reloaded.contains("[[schedules]]"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn channel_add_rejects_empty_tokens() {
    let path = temp_file_path("autopilot-channel-add-empty-token");

    let empty_telegram_result = add_channel_with_optional_target(
        path.as_path(),
        ChannelType::Telegram,
        Some("   ".to_string()),
        None,
        None,
        None,
        None,
    );
    assert!(empty_telegram_result.is_err());

    let empty_discord_result = add_channel_with_optional_target(
        path.as_path(),
        ChannelType::Discord,
        Some("   ".to_string()),
        None,
        None,
        None,
        None,
    );
    assert!(empty_discord_result.is_err());

    let empty_bot_result = add_channel_with_optional_target(
        path.as_path(),
        ChannelType::Slack,
        None,
        Some("   ".to_string()),
        Some("xapp-test".to_string()),
        None,
        None,
    );
    assert!(empty_bot_result.is_err());

    let empty_app_result = add_channel_with_optional_target(
        path.as_path(),
        ChannelType::Slack,
        None,
        Some("xoxb-test".to_string()),
        Some("   ".to_string()),
        None,
        None,
    );
    assert!(empty_app_result.is_err());

    let _ = std::fs::remove_file(path);
}

#[test]
fn channel_remove_recovers_from_invalid_channel_config() {
    let path = temp_file_path("autopilot-channel-remove-invalid");
    let write_result = std::fs::write(
        &path,
        r##"
[channels.slack]
bot_token = ""
app_token = "xapp-test"
"##,
    );
    assert!(write_result.is_ok());

    let remove_result = remove_channel(path.as_path(), ChannelType::Slack);
    assert!(remove_result.is_ok());

    let reloaded = std::fs::read_to_string(&path);
    assert!(reloaded.is_ok());
    let reloaded = match reloaded {
        Ok(value) => value,
        Err(error) => panic!("failed to read config: {error}"),
    };
    assert!(!reloaded.contains("[channels.slack]"));

    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn schedule_add_writes_to_config() {
    // Schedule add now works — it writes to the config file.
    // We can't easily test it here without a temp config path,
    // so just verify the helper functions work correctly.
    let mut config = AutopilotConfigFile::default();
    let schedule = AutopilotScheduleConfig {
        name: "demo".to_string(),
        cron: "*/5 * * * *".to_string(),
        prompt: "hello".to_string(),
        check: None,
        trigger_on: ScheduleTriggerOn::Failure,
        // workdir: None,
        max_steps: 50,
        channel: None,
        profile: None,
        pause_on_approval: false,
        sandbox: false,
        enabled: true,
    };
    let result = add_schedule_in_config(&mut config, schedule);
    assert!(result.is_ok());
    assert!(config.find_schedule("demo").is_some());
}

#[test]
fn gateway_channel_count_surfaces_invalid_channel_config() {
    let path = temp_file_path("autopilot-invalid-gateway-channel");
    let write_result = std::fs::write(
        &path,
        r##"
[channels.slack]
bot_token = ""
app_token = "xapp-test"
"##,
    );
    assert!(write_result.is_ok());

    let count_result = gateway_channel_count(path.as_path());
    assert!(count_result.is_err());

    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn channel_add_requires_token() {
    // Channel add without token should fail with a helpful message
    let profile_path = temp_file_path("autopilot-channel-token-required-profile");
    write_profile_config(
        &profile_path,
        r#"
[settings]
editor = "nano"

[profiles.default]
api_key = "default-key"
"#,
    );

    let app_config = test_app_config(&profile_path);
    let result = run_channel_command(
        AutopilotChannelCommands::Add {
            channel_type: ChannelType::Telegram,
            token: None,
            bot_token: None,
            app_token: None,
            target: None,
            profile: None,
        },
        &app_config,
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Telegram token required"));

    let _ = std::fs::remove_file(profile_path);
}

#[test]
fn validate_profile_reference_accepts_existing_profile() {
    let profile_path = temp_file_path("autopilot-validate-profile-existing");
    write_profile_config(
        &profile_path,
        r#"
[settings]
editor = "nano"

[profiles.default]
api_key = "default-key"

[profiles.ops]
api_key = "ops-key"
"#,
    );

    let app_config = test_app_config(&profile_path);
    let result = validate_profile_reference("ops", &app_config);
    assert!(result.is_ok());

    let _ = std::fs::remove_file(profile_path);
}

#[test]
fn validate_profile_reference_rejects_reserved_all() {
    let app_config = test_app_config(Path::new("/tmp/non-existent-config.toml"));
    let result = validate_profile_reference("all", &app_config);
    assert!(result.is_err());
    assert!(
        result
            .expect_err("expected reserved profile error")
            .contains("reserved")
    );
}

#[test]
fn validate_profile_reference_rejects_empty_name() {
    let app_config = test_app_config(Path::new("/tmp/non-existent-config.toml"));
    let result = validate_profile_reference("   ", &app_config);
    assert!(result.is_err());
    assert!(
        result
            .expect_err("expected empty profile name error")
            .contains("cannot be empty")
    );
}

#[test]
fn validate_profile_reference_lists_available_profiles_on_missing() {
    let profile_path = temp_file_path("autopilot-validate-profile-missing");
    write_profile_config(
        &profile_path,
        r#"
[settings]
editor = "nano"

[profiles.default]
api_key = "default-key"

[profiles.ops]
api_key = "ops-key"

[profiles.monitoring]
api_key = "monitoring-key"
"#,
    );

    let app_config = test_app_config(&profile_path);
    let result = validate_profile_reference("missing", &app_config);
    assert!(result.is_err());

    let message = result.expect_err("expected missing profile error");
    assert!(message.contains("missing"));
    assert!(message.contains("default"));
    assert!(message.contains("monitoring"));
    assert!(message.contains("ops"));

    let _ = std::fs::remove_file(profile_path);
}

#[test]
fn validate_profile_reference_surfaces_config_load_errors() {
    // Write invalid TOML so load_config_file returns a parse error
    // (a missing file returns Ok(default_config) with a "default" profile).
    let bad_path = temp_file_path("autopilot-validate-profile-bad-config");
    std::fs::write(&bad_path, "{{{{invalid toml!!!!").expect("write bad config");

    let app_config = test_app_config(&bad_path);
    let result = validate_profile_reference("default", &app_config);

    assert!(result.is_err());
    assert!(
        result
            .expect_err("expected config load error")
            .contains("Failed to load config.toml")
    );

    let _ = std::fs::remove_file(bad_path);
}

#[test]
fn resolve_policy_none_allowed_tools_falls_back_to_safe_list() {
    let policy = resolve_server_tool_policy(None, None, false);

    assert_eq!(
        policy.action_for("view", None),
        vac_broker::ToolApprovalAction::Approve
    );
    assert_eq!(
        policy.action_for("run_command", None),
        vac_broker::ToolApprovalAction::Ask
    );
}

#[test]
fn resolve_policy_explicit_allowed_tools() {
    let allowed_tools = vec!["view".to_string()];
    let policy = resolve_server_tool_policy(Some(&allowed_tools), None, false);

    assert_eq!(
        policy.action_for("view", None),
        vac_broker::ToolApprovalAction::Approve
    );
    assert_eq!(
        policy.action_for("run_command", None),
        vac_broker::ToolApprovalAction::Ask
    );
}

#[test]
fn resolve_policy_auto_approve_all_overrides() {
    let policy = resolve_server_tool_policy(None, None, true);

    assert_eq!(
        policy.action_for("run_command", None),
        vac_broker::ToolApprovalAction::Approve
    );
    assert_eq!(
        policy.action_for("some_future_tool", None),
        vac_broker::ToolApprovalAction::Approve
    );
}

#[test]
fn resolve_policy_auto_approve_extras_promoted() {
    let allowed_tools = vec!["view".to_string()];
    let auto_approve = vec!["run_command".to_string()];
    let policy = resolve_server_tool_policy(Some(&allowed_tools), Some(&auto_approve), false);

    assert_eq!(
        policy.action_for("run_command", None),
        vac_broker::ToolApprovalAction::Approve
    );
}

#[test]
fn resolve_profile_run_overrides_loads_profile_values() {
    let path = temp_file_path("profile-overrides");
    write_profile_config(
        &path,
        r#"
[settings]
editor = "nano"

[profiles.default]
api_key = "default-key"
model = "openai/gpt-4o-mini"

[profiles.production]
api_key = "prod-key"
model = "anthropic/claude-sonnet-4-5"
allowed_tools = ["vac__view", ""]
auto_approve = ["vac__run_command", "  "]
system_prompt = "production prompt"
max_turns = 32
"#,
    );

    let resolved =
        resolve_profile_run_overrides("production", Some(path.to_string_lossy().as_ref()));
    assert!(resolved.is_some());

    if let Some(resolved) = resolved {
        assert_eq!(
            resolved.model.as_deref(),
            Some("anthropic/claude-sonnet-4-5")
        );
        assert_eq!(resolved.allowed_tools, Some(vec!["view".to_string()]));
        assert_eq!(resolved.auto_approve, Some(vec!["run_command".to_string()]));
        assert_eq!(resolved.system_prompt.as_deref(), Some("production prompt"));
        assert_eq!(resolved.max_turns, Some(32));
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn resolve_profile_run_overrides_preserves_explicit_empty_tool_lists() {
    let path = temp_file_path("profile-overrides-empty-tools");
    write_profile_config(
        &path,
        r#"
[settings]
editor = "nano"

[profiles.default]
api_key = "default-key"

[profiles.ops]
api_key = "ops-key"
allowed_tools = []
auto_approve = []
"#,
    );

    let resolved = resolve_profile_run_overrides("ops", Some(path.to_string_lossy().as_ref()));

    assert!(resolved.is_some());
    if let Some(resolved) = resolved {
        assert_eq!(resolved.allowed_tools, Some(Vec::new()));
        assert_eq!(resolved.auto_approve, Some(Vec::new()));
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn profile_run_override_resolver_maps_runtime_fields() {
    let path = temp_file_path("gateway-channel-profile");
    write_profile_config(
        &path,
        r#"
[settings]
editor = "nano"

[profiles.default]
api_key = "default-key"

[profiles.ops]
api_key = "ops-key"
model = "anthropic/claude-opus-4-5"
auto_approve = ["view"]
system_prompt = "ops prompt"
max_turns = 12
"#,
    );

    let resolver = ProfileRunOverrideResolver::new(path.to_string_lossy().to_string());
    let resolved = vac_messaging_gateway::dispatcher::RunOverrideResolver::resolve_run_overrides(
        &resolver, "ops",
    );
    assert!(resolved.is_some());

    if let Some(resolved) = resolved {
        assert_eq!(resolved.model.as_deref(), Some("anthropic/claude-opus-4-5"));
        assert!(matches!(
            resolved.auto_approve,
            Some(vac_messaging_gateway::client::AutoApproveOverride::AllowList(ref tools)) if tools == &vec!["view".to_string()]
        ));
        assert_eq!(resolved.system_prompt.as_deref(), Some("ops prompt"));
        assert_eq!(resolved.max_turns, Some(12));
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn channel_profiles_map_reads_explicit_profiles() {
    let mut gateway_cfg = vac_messaging_gateway::GatewayConfig::default();
    gateway_cfg.channels.slack = Some(vac_messaging_gateway::config::SlackConfig {
        bot_token: "xoxb-token".to_string(),
        app_token: "xapp-token".to_string(),
        model: None,
        auto_approve: None,
        profile: Some("ops".to_string()),
    });

    let profiles = gateway_cfg.channels.profiles_map();
    assert_eq!(profiles.get("slack").map(String::as_str), Some("ops"));
    assert!(!profiles.contains_key("telegram"));
}

#[test]
fn gateway_channel_profiles_default_to_boot_profile_when_omitted() {
    let mut gateway_cfg = vac_messaging_gateway::GatewayConfig::default();
    gateway_cfg.channels.slack = Some(vac_messaging_gateway::config::SlackConfig {
        bot_token: "xoxb-token".to_string(),
        app_token: "xapp-token".to_string(),
        model: None,
        auto_approve: None,
        profile: None,
    });

    let profiles = gateway_channel_profiles_with_default(&gateway_cfg.channels, "default");
    assert_eq!(profiles.get("slack").map(String::as_str), Some("default"));
}

#[test]
fn gateway_channel_profiles_preserve_explicit_profile_over_default() {
    let mut gateway_cfg = vac_messaging_gateway::GatewayConfig::default();
    gateway_cfg.channels.slack = Some(vac_messaging_gateway::config::SlackConfig {
        bot_token: "xoxb-token".to_string(),
        app_token: "xapp-token".to_string(),
        model: None,
        auto_approve: None,
        profile: Some("ops".to_string()),
    });

    let profiles = gateway_channel_profiles_with_default(&gateway_cfg.channels, "default");
    assert_eq!(profiles.get("slack").map(String::as_str), Some("ops"));
}

#[test]
fn mcp_allowed_tools_unrestricted_when_policy_is_ask_default() {
    let policy = resolve_server_tool_policy(None, None, false);
    let allowed = mcp_allowed_tools_from_policy(&policy, None);

    assert!(allowed.is_none());
}

#[test]
fn test_gateway_gets_allowlist_from_resolved_policy() {
    let policy = resolve_server_tool_policy(None, None, false);
    let mut gateway_cfg = vac_messaging_gateway::GatewayConfig::default();

    apply_gateway_policy_from_resolved_tools(&mut gateway_cfg, &policy);

    assert!(matches!(
        gateway_cfg.gateway.approval_mode,
        vac_messaging_gateway::ApprovalMode::Allowlist
    ));
    assert!(!gateway_cfg.gateway.approval_allowlist.is_empty());
    assert!(
        gateway_cfg
            .gateway
            .approval_allowlist
            .contains(&"view".to_string())
    );
    assert!(
        gateway_cfg
            .gateway
            .approval_allowlist
            .contains(&"vac__view".to_string())
    );
}

#[test]
fn test_gateway_gets_allow_all_when_auto_approve_all() {
    let policy = resolve_server_tool_policy(None, None, true);
    let mut gateway_cfg = vac_messaging_gateway::GatewayConfig::default();
    gateway_cfg.gateway.approval_mode = vac_messaging_gateway::ApprovalMode::Allowlist;
    gateway_cfg.gateway.approval_allowlist = vec!["view".to_string()];

    apply_gateway_policy_from_resolved_tools(&mut gateway_cfg, &policy);

    assert!(matches!(
        gateway_cfg.gateway.approval_mode,
        vac_messaging_gateway::ApprovalMode::AllowAll
    ));
    assert!(gateway_cfg.gateway.approval_allowlist.is_empty());
}

#[test]
fn status_json_schema_contains_core_fields() {
    let payload = AutopilotStatusJson {
        command: "autopilot.status",
        ok: true,
        profile: "default".to_string(),
        config_path: "/tmp/autopilot.toml".to_string(),
        server_config: AutopilotServerConfig::default(),
        server_allowed_tool_count: 9,
        service: ServiceStatusJson {
            installed: true,
            active: true,
            path: "/tmp/service".to_string(),
        },
        server: EndpointStatusJson {
            expected_enabled: true,
            reachable: true,
            url: "http://127.0.0.1:4096/v1/health".to_string(),
        },
        gateway: EndpointStatusJson {
            expected_enabled: true,
            reachable: false,
            url: "http://127.0.0.1:4096/v1/gateway/status".to_string(),
        },
        sandbox: SandboxStatusJson {
            mode: "persistent".to_string(),
            healthy: Some(true),
            consecutive_ok: Some(42),
            consecutive_failures: Some(0),
            last_ok: Some("2026-01-01T00:00:00Z".to_string()),
            last_error: None,
        },
        scheduler: SchedulerStatusJson {
            expected_enabled: true,
            config_path: "/tmp/autopilot.toml".to_string(),
            config_valid: true,
            trigger_count: 2,
            running: true,
            pid: Some(123),
            stale_pid: false,
            db_path: Some("/tmp/autopilot.db".to_string()),
            error: None,
            recent_runs: vec![ScheduleRunSummaryJson {
                id: 1,
                schedule_name: "example".to_string(),
                status: "completed".to_string(),
                started_at: "2026-01-01T00:00:00Z".to_string(),
                finished_at: Some("2026-01-01T00:00:10Z".to_string()),
                error_message: None,
            }],
        },
        schedules: vec![AutopilotScheduleStatusJson {
            name: "health-check".to_string(),
            cron: "*/5 * * * *".to_string(),
            enabled: true,
            sandbox: false,
            next_run: Some("2026-01-01 00:05".to_string()),
        }],
        channels: vec![AutopilotChannelStatusJson {
            name: "slack".to_string(),
            channel_type: "slack".to_string(),
            target: "#infra".to_string(),
            enabled: true,
            alerts_only: false,
        }],
    };

    let json = serde_json::to_value(payload);
    assert!(json.is_ok());

    if let Ok(value) = json {
        assert_eq!(
            value.get("command").and_then(|v| v.as_str()),
            Some("autopilot.status")
        );
        assert!(value.get("server_config").is_some());
        assert!(value.get("server_allowed_tool_count").is_some());
        assert!(value.get("service").is_some());
        assert!(value.get("server").is_some());
        assert!(value.get("gateway").is_some());
        assert!(value.get("sandbox").is_some());
        assert!(value.get("scheduler").is_some());
        assert!(value.get("schedules").is_some());
        assert!(value.get("channels").is_some());

        // Verify sandbox fields
        let sandbox = value.get("sandbox").expect("sandbox field");
        assert_eq!(
            sandbox.get("mode").and_then(|v| v.as_str()),
            Some("persistent")
        );
        assert_eq!(sandbox.get("healthy").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            sandbox.get("consecutive_ok").and_then(|v| v.as_u64()),
            Some(42)
        );

        let scheduler_runs = value
            .get("scheduler")
            .and_then(|s| s.get("recent_runs"))
            .and_then(|runs| runs.as_array())
            .map(|runs| runs.len())
            .unwrap_or_default();
        assert_eq!(scheduler_runs, 1);
    }
}
#[test]
fn sandbox_user_mapping_always_delegates_to_detect_host_user_mapping() {
    // Both persistent and ephemeral modes must use detect_host_user_mapping()
    // so that bind-mounted host files are writable.  The container entrypoint
    // script handles /etc/passwd fixup when the runtime UID differs.
    //
    // On macOS (CI) this returns ImageDefault; on Linux it returns HostUser.
    let expected = detect_host_user_mapping();
    assert_eq!(
        sandbox_user_mapping_for_mode(&vac_broker::SandboxMode::Persistent),
        expected,
    );
    assert_eq!(
        sandbox_user_mapping_for_mode(&vac_broker::SandboxMode::Ephemeral),
        expected,
    );
}

// The root UID guard in detect_host_user_mapping() cannot be unit-tested
// directly without running as root.  The logic is: uid=0 or gid=0 → fallback
// to ImageDefault.  This is covered by code review; an integration test would
// need a privileged container.

// ── shell_join tests ────────────────────────────────────────────────────

#[test]
fn shell_join_simple_args() {
    let parts = vec![
        "/usr/local/bin/vac".to_string(),
        "autopilot".to_string(),
        "up".to_string(),
        "--foreground".to_string(),
    ];
    assert_eq!(
        shell_join(&parts),
        "/usr/local/bin/vac autopilot up --foreground"
    );
}

#[test]
fn shell_join_preserves_colons_and_dots() {
    let parts = vec![
        "/usr/bin/app".to_string(),
        "--bind".to_string(),
        "127.0.0.1:8080".to_string(),
    ];
    assert_eq!(shell_join(&parts), "/usr/bin/app --bind 127.0.0.1:8080");
}

#[test]
fn shell_join_quotes_spaces() {
    let parts = vec![
        "/usr/bin/app".to_string(),
        "--profile".to_string(),
        "my profile".to_string(),
    ];
    assert_eq!(shell_join(&parts), "/usr/bin/app --profile 'my profile'");
}

#[test]
fn shell_join_escapes_single_quotes() {
    let parts = vec![
        "/usr/bin/app".to_string(),
        "--name".to_string(),
        "it's-a-test".to_string(),
    ];
    // The POSIX '\'' idiom: close quote, escaped literal quote, reopen quote
    assert_eq!(shell_join(&parts), "/usr/bin/app --name 'it'\\''s-a-test'");
}

#[test]
fn shell_join_handles_multiple_single_quotes() {
    let parts = vec!["a'b'c".to_string()];
    assert_eq!(shell_join(&parts), "'a'\\''b'\\''c'");
}

#[test]
fn shell_join_empty_string_arg() {
    // Note: shell_join does NOT quote empty strings because the `all()`
    // check is vacuously true. This is fine in practice because
    // install_systemd_service guards against empty profile_name/config_path
    // before building exec_parts.
    let parts = vec!["/usr/bin/app".to_string(), "".to_string()];
    assert_eq!(shell_join(&parts), "/usr/bin/app ");
}

#[test]
fn shell_join_arg_with_equals_and_spaces() {
    let parts = vec!["/usr/bin/app".to_string(), "--env=FOO BAR".to_string()];
    assert_eq!(shell_join(&parts), "/usr/bin/app '--env=FOO BAR'");
}

// ── systemd sg wrapper quoting tests ────────────────────────────────────
//
// These tests verify the full quoting pipeline:
//   shell_join() → backslash doubling → sg -c "..." wrapper
//
// The resulting string is what goes into ExecStart=. Systemd processes
// C-style escapes (\\ → \, \' → ') before passing argv to sg.
// sg then invokes /bin/sh -c <argv[3]>, so the shell must receive
// valid POSIX-quoted arguments.

/// Simulate what systemd does to the ExecStart value: process C-style
/// escape sequences inside double-quoted regions.
fn simulate_systemd_unescape(exec_start: &str) -> Vec<String> {
    // Simplified systemd ExecStart parser:
    // - Split on whitespace
    // - "..." groups tokens (strip outer quotes, process C-escapes inside)
    // - '...' groups tokens (strip outer quotes, literal content)
    // - Unquoted tokens are literal
    let mut args = Vec::new();
    let mut chars = exec_start.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace between tokens
        while chars.peek() == Some(&' ') {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        let mut token = String::new();
        match chars.peek() {
            Some('"') => {
                chars.next(); // consume opening "
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        chars.next(); // consume closing "
                        break;
                    } else if c == '\\' {
                        chars.next(); // consume backslash
                        if let Some(&escaped) = chars.peek() {
                            // C-style escapes
                            match escaped {
                                '\\' => {
                                    token.push('\\');
                                    chars.next();
                                }
                                '\'' => {
                                    token.push('\'');
                                    chars.next();
                                }
                                '"' => {
                                    token.push('"');
                                    chars.next();
                                }
                                'n' => {
                                    token.push('\n');
                                    chars.next();
                                }
                                't' => {
                                    token.push('\t');
                                    chars.next();
                                }
                                _ => {
                                    token.push('\\');
                                    token.push(escaped);
                                    chars.next();
                                }
                            }
                        } else {
                            token.push('\\');
                        }
                    } else {
                        token.push(c);
                        chars.next();
                    }
                }
            }
            Some('\'') => {
                chars.next(); // consume opening '
                while let Some(&c) = chars.peek() {
                    if c == '\'' {
                        chars.next(); // consume closing '
                        break;
                    }
                    token.push(c);
                    chars.next();
                }
            }
            _ => {
                while let Some(&c) = chars.peek() {
                    if c == ' ' {
                        break;
                    }
                    token.push(c);
                    chars.next();
                }
            }
        }
        args.push(token);
    }
    args
}

#[test]
fn build_systemd_exec_start_ignores_docker_group_wrapper_and_keeps_hardening() {
    let exec_cmd = "/usr/local/bin/vac autopilot up --foreground";
    let (exec_start, no_new_privileges) = build_systemd_exec_start(exec_cmd);
    assert_eq!(exec_start, exec_cmd);
    assert_eq!(no_new_privileges, "true");
}

#[test]
fn build_systemd_exec_start_preserves_quoted_arguments_without_shell_wrapper() {
    let exec_cmd = "/usr/local/bin/vac --profile 'my profile' autopilot up";
    let (exec_start, no_new_privileges) = build_systemd_exec_start(exec_cmd);
    assert_eq!(exec_start, exec_cmd);
    assert_eq!(no_new_privileges, "true");
}

// ── direct ExecStart quoting tests ──────────────────────────────────────
//
// Autopilot now always uses the direct ExecStart path. Systemd execs the
// binary directly (no shell), so systemd's own parser handles quoting.

/// For the non-sg path, verify systemd correctly parses shell_join output.
fn assert_direct_exec_roundtrip(exec_parts: &[String], expected_argv: &[&str]) {
    let exec_start = shell_join(exec_parts);
    let systemd_argv = simulate_systemd_unescape(&exec_start);
    let expected: Vec<String> = expected_argv.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        systemd_argv, expected,
        "systemd-parsed argv for direct exec"
    );
}

#[test]
fn direct_exec_simple_args() {
    let parts = vec![
        "/usr/local/bin/vac".to_string(),
        "autopilot".to_string(),
        "up".to_string(),
        "--foreground".to_string(),
    ];
    assert_direct_exec_roundtrip(
        &parts,
        &["/usr/local/bin/vac", "autopilot", "up", "--foreground"],
    );
}

#[test]
fn direct_exec_spaces_in_profile() {
    let parts = vec![
        "/usr/local/bin/vac".to_string(),
        "--profile".to_string(),
        "my profile".to_string(),
        "autopilot".to_string(),
        "up".to_string(),
    ];
    assert_direct_exec_roundtrip(
        &parts,
        &[
            "/usr/local/bin/vac",
            "--profile",
            "my profile",
            "autopilot",
            "up",
        ],
    );
}
