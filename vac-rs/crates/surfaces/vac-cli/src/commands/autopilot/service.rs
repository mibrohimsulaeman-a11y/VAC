use std::path::PathBuf;

use crate::config::AppConfig;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    MacOS,
    Linux,
    Windows,
    Unknown,
}

pub(crate) fn detect_platform() -> Platform {
    #[cfg(target_os = "macos")]
    {
        return Platform::MacOS;
    }
    #[cfg(target_os = "linux")]
    {
        return Platform::Linux;
    }
    #[cfg(target_os = "windows")]
    {
        return Platform::Windows;
    }
    #[allow(unreachable_code)]
    Platform::Unknown
}

pub(crate) const AUTOPILOT_SYSTEMD_SERVICE: &str = "vac-autopilot";
const AUTOPILOT_LAUNCHD_LABEL: &str = "dev.vac.autopilot";

pub(crate) fn autopilot_log_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vac")
        .join("autopilot")
        .join("logs")
}

pub(crate) fn autopilot_service_path() -> PathBuf {
    match detect_platform() {
        Platform::Linux => dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("systemd")
            .join("user")
            .join(format!("{}.service", AUTOPILOT_SYSTEMD_SERVICE)),
        Platform::MacOS => dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{}.plist", AUTOPILOT_LAUNCHD_LABEL)),
        Platform::Windows | Platform::Unknown => PathBuf::new(),
    }
}

pub(crate) fn autopilot_service_installed() -> bool {
    let path = autopilot_service_path();
    !path.as_os_str().is_empty() && path.exists()
}

/// Check if the autopilot process is currently running via PID file + process check.
pub(crate) fn is_autopilot_running() -> Option<u32> {
    let config = crate::commands::watch::ScheduleConfig::load_default().ok()?;
    let pid_file = config
        .db_path()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("autopilot.pid");
    let pid_str = std::fs::read_to_string(&pid_file).ok()?;
    let pid: u32 = pid_str.trim().parse().ok()?;
    if crate::commands::watch::is_process_running(pid) {
        Some(pid)
    } else {
        // Stale PID file — clean it up
        let _ = std::fs::remove_file(&pid_file);
        None
    }
}

pub(crate) fn install_autopilot_service(config: &AppConfig) -> Result<(), String> {
    match detect_platform() {
        Platform::Linux => install_systemd_service(config),
        Platform::MacOS => install_launchd_service(config),
        Platform::Windows => Err("Windows autopilot service is not yet supported".to_string()),
        Platform::Unknown => Err("Unsupported platform for autopilot service".to_string()),
    }
}

pub(crate) fn uninstall_autopilot_service() -> Result<(), String> {
    match detect_platform() {
        Platform::Linux => uninstall_systemd_service(),
        Platform::MacOS => uninstall_launchd_service(),
        Platform::Windows => Err("Windows autopilot service is not yet supported".to_string()),
        Platform::Unknown => Err("Unsupported platform for autopilot service".to_string()),
    }
}

pub(crate) fn start_autopilot_service() -> Result<(), String> {
    match detect_platform() {
        Platform::Linux => {
            run_command(
                "systemctl",
                &["--user", "daemon-reload"],
                "Failed to reload systemd",
            )?;
            run_command(
                "systemctl",
                &["--user", "start", AUTOPILOT_SYSTEMD_SERVICE],
                "Failed to start autopilot service",
            )
        }
        Platform::MacOS => {
            let plist = autopilot_service_path();
            let load_output = std::process::Command::new("launchctl")
                .args(["load", plist.to_string_lossy().as_ref()])
                .output()
                .map_err(|e| format!("Failed to load launchd service: {}", e))?;

            if !load_output.status.success() {
                let stderr = String::from_utf8_lossy(&load_output.stderr);
                if !stderr.to_ascii_lowercase().contains("already loaded") {
                    return Err(format!("Failed to load launchd service: {}", stderr));
                }
            }

            run_command(
                "launchctl",
                &["start", AUTOPILOT_LAUNCHD_LABEL],
                "Failed to start launchd service",
            )
        }
        Platform::Windows => Err("Windows autopilot service is not yet supported".to_string()),
        Platform::Unknown => Err("Unsupported platform for autopilot service".to_string()),
    }
}

pub(crate) fn stop_autopilot_service() -> Result<(), String> {
    match detect_platform() {
        Platform::Linux => run_command(
            "systemctl",
            &["--user", "stop", AUTOPILOT_SYSTEMD_SERVICE],
            "Failed to stop autopilot service",
        ),
        Platform::MacOS => {
            let output = std::process::Command::new("launchctl")
                .args(["stop", AUTOPILOT_LAUNCHD_LABEL])
                .output()
                .map_err(|e| format!("Failed to stop launchd service: {}", e))?;

            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr
                    .to_ascii_lowercase()
                    .contains("could not find service")
                {
                    Ok(())
                } else {
                    Err(format!("Failed to stop launchd service: {}", stderr))
                }
            }
        }
        Platform::Windows => Err("Windows autopilot service is not yet supported".to_string()),
        Platform::Unknown => Err("Unsupported platform for autopilot service".to_string()),
    }
}

pub(crate) fn autopilot_service_active() -> bool {
    match detect_platform() {
        Platform::Linux => std::process::Command::new("systemctl")
            .args(["--user", "is-active", "--quiet", AUTOPILOT_SYSTEMD_SERVICE])
            .status()
            .map(|status| status.success())
            .unwrap_or(false),
        Platform::MacOS => std::process::Command::new("launchctl")
            .args(["list", AUTOPILOT_LAUNCHD_LABEL])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false),
        Platform::Windows | Platform::Unknown => false,
    }
}

fn install_systemd_service(config: &AppConfig) -> Result<(), String> {
    let binary =
        std::env::current_exe().map_err(|e| format!("Failed to resolve vac binary path: {}", e))?;
    let service_path = autopilot_service_path();

    if let Some(parent) = service_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create systemd directory: {}", e))?;
    }

    let log_dir = autopilot_log_dir();
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| format!("Failed to create autopilot log directory: {}", e))?;

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    let mut exec_parts = vec![binary.display().to_string()];
    if !config.profile_name.is_empty() {
        exec_parts.push("--profile".to_string());
        exec_parts.push(config.profile_name.clone());
    }
    if !config.config_path.is_empty() {
        exec_parts.push("--config".to_string());
        exec_parts.push(config.config_path.clone());
    }
    exec_parts.extend([
        "autopilot".to_string(),
        "up".to_string(),
        "--foreground".to_string(),
        "--from-service".to_string(),
    ]);

    let exec_cmd = shell_join(&exec_parts);
    let (exec_start, no_new_privileges) = build_systemd_exec_start(&exec_cmd);

    let unit = format!(
        "[Unit]\nDescription=VAC Autopilot Runtime\nAfter=network.target\n\n[Service]\nType=simple\nExecStart={}\nRestart=on-failure\nRestartSec=5\nWorkingDirectory={}\nEnvironment=HOME={}\nEnvironment=PATH=/usr/local/bin:/usr/bin:/bin\nStandardOutput=append:{}/stdout.log\nStandardError=append:{}/stderr.log\nNoNewPrivileges={}\n\n[Install]\nWantedBy=default.target\n",
        exec_start,
        home.display(),
        home.display(),
        log_dir.display(),
        log_dir.display(),
        no_new_privileges,
    );

    std::fs::write(&service_path, unit)
        .map_err(|e| format!("Failed to write systemd service file: {}", e))?;

    run_command(
        "systemctl",
        &["--user", "daemon-reload"],
        "Failed to reload systemd",
    )?;
    run_command(
        "systemctl",
        &["--user", "enable", AUTOPILOT_SYSTEMD_SERVICE],
        "Failed to enable autopilot service",
    )?;

    Ok(())
}

/// Build the `ExecStart=` value for the systemd unit file.
///
/// Autopilot now always uses a direct exec path so the service can keep
/// `NoNewPrivileges=true` regardless of docker-group membership.
pub(crate) fn build_systemd_exec_start(exec_cmd: &str) -> (String, &'static str) {
    (exec_cmd.to_string(), "true")
}

fn uninstall_systemd_service() -> Result<(), String> {
    let service_path = autopilot_service_path();

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "stop", AUTOPILOT_SYSTEMD_SERVICE])
        .status();
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "disable", AUTOPILOT_SYSTEMD_SERVICE])
        .status();

    if service_path.exists() {
        std::fs::remove_file(&service_path)
            .map_err(|e| format!("Failed to remove systemd service file: {}", e))?;
    }

    run_command(
        "systemctl",
        &["--user", "daemon-reload"],
        "Failed to reload systemd",
    )?;

    Ok(())
}

fn install_launchd_service(config: &AppConfig) -> Result<(), String> {
    let binary =
        std::env::current_exe().map_err(|e| format!("Failed to resolve vac binary path: {}", e))?;
    let plist_path = autopilot_service_path();

    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create LaunchAgents directory: {}", e))?;
    }

    let log_dir = autopilot_log_dir();
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| format!("Failed to create autopilot log directory: {}", e))?;

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    let mut args = Vec::new();
    if !config.profile_name.is_empty() {
        args.push("<string>--profile</string>".to_string());
        args.push(format!(
            "<string>{}</string>",
            xml_escape(&config.profile_name)
        ));
    }
    if !config.config_path.is_empty() {
        args.push("<string>--config</string>".to_string());
        args.push(format!(
            "<string>{}</string>",
            xml_escape(&config.config_path)
        ));
    }
    args.extend([
        "<string>autopilot</string>".to_string(),
        "<string>up</string>".to_string(),
        "<string>--foreground</string>".to_string(),
        "<string>--from-service</string>".to_string(),
    ]);

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        {}
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>WorkingDirectory</key>
    <string>{}</string>
    <key>StandardOutPath</key>
    <string>{}/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{}/stderr.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>HOME</key>
        <string>{}</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>
</dict>
</plist>
"#,
        AUTOPILOT_LAUNCHD_LABEL,
        xml_escape(&binary.display().to_string()),
        args.join("\n        "),
        xml_escape(&home.display().to_string()),
        xml_escape(&log_dir.display().to_string()),
        xml_escape(&log_dir.display().to_string()),
        xml_escape(&home.display().to_string()),
    );

    std::fs::write(&plist_path, plist)
        .map_err(|e| format!("Failed to write launchd plist: {}", e))?;

    Ok(())
}

fn uninstall_launchd_service() -> Result<(), String> {
    let plist_path = autopilot_service_path();

    let _ = std::process::Command::new("launchctl")
        .args(["stop", AUTOPILOT_LAUNCHD_LABEL])
        .status();
    let _ = std::process::Command::new("launchctl")
        .args(["unload", plist_path.to_string_lossy().as_ref()])
        .status();

    if plist_path.exists() {
        std::fs::remove_file(&plist_path)
            .map_err(|e| format!("Failed to remove launchd plist: {}", e))?;
    }

    Ok(())
}

fn run_command(command: &str, args: &[&str], context: &str) -> Result<(), String> {
    let output = std::process::Command::new(command)
        .args(args)
        .output()
        .map_err(|e| format!("{}: {}", context, e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{}: {}",
            context,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub(crate) fn shell_join(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| {
            if part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.' | ':'))
            {
                part.clone()
            } else {
                format!("'{}'", part.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
