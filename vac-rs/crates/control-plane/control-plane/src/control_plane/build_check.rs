use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

pub const DEFAULT_BUILD_CHECK_PACKAGE: &str = "vac-surface-cli";
pub const DEFAULT_BUILD_CHECK_TOOLCHAIN: &str = "1.95.0";
pub const DEFAULT_BUILD_CHECK_JOBS: u16 = 1;
pub const DEFAULT_BUILD_CHECK_TIMEOUT_SECONDS: u64 = 600;
pub const BUILD_CHECK_TOOLCHAIN_ENV: &str = "VAC_BUILD_TOOLCHAIN";
pub const BUILD_CHECK_TIMEOUT_ENV: &str = "VAC_BUILD_CHECK_TIMEOUT_SECONDS";
const SUMMARY_MAX_LINES: usize = 12;
const SUMMARY_MAX_LINE_CHARS: usize = 240;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCheckRequest {
    pub package: String,
    pub toolchain: String,
    pub jobs: u16,
    pub incremental: bool,
    pub repo_root: PathBuf,
    pub cargo_program: PathBuf,
    pub timeout: Duration,
}

impl BuildCheckRequest {
    pub fn for_repo_root(repo_root: impl AsRef<Path>) -> Self {
        let repo_root = repo_root.as_ref();
        Self {
            package: DEFAULT_BUILD_CHECK_PACKAGE.to_string(),
            toolchain: discover_toolchain_channel(repo_root)
                .unwrap_or_else(|| DEFAULT_BUILD_CHECK_TOOLCHAIN.to_string()),
            jobs: DEFAULT_BUILD_CHECK_JOBS,
            incremental: false,
            repo_root: repo_root.to_path_buf(),
            cargo_program: PathBuf::from("cargo"),
            timeout: Duration::from_secs(DEFAULT_BUILD_CHECK_TIMEOUT_SECONDS),
        }
    }

    pub fn with_cargo_program(mut self, cargo_program: impl Into<PathBuf>) -> Self {
        self.cargo_program = cargo_program.into();
        self
    }

    pub fn with_toolchain(mut self, toolchain: impl Into<String>) -> Self {
        self.toolchain = toolchain.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Apply environment-driven overrides for toolchain and timeout. Reads
    /// `VAC_BUILD_TOOLCHAIN` and `VAC_BUILD_CHECK_TIMEOUT_SECONDS`. Unset or
    /// blank variables leave existing fields untouched. Malformed timeout
    /// values are ignored so callers do not need to special case parse errors.
    pub fn apply_env_overrides(mut self) -> Self {
        if let Ok(toolchain) = std::env::var(BUILD_CHECK_TOOLCHAIN_ENV) {
            let trimmed = toolchain.trim();
            if !trimmed.is_empty() {
                self.toolchain = trimmed.to_string();
            }
        }
        if let Ok(seconds) = std::env::var(BUILD_CHECK_TIMEOUT_ENV)
            && let Ok(parsed) = seconds.trim().parse::<u64>()
        {
            self.timeout = Duration::from_secs(parsed);
        }
        self
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.repo_root.join("vac-rs/Cargo.toml")
    }

    pub fn command_display(&self) -> String {
        format!(
            "CARGO_BUILD_JOBS={} CARGO_INCREMENTAL={} cargo +{} check --manifest-path vac-rs/Cargo.toml -p {}",
            self.jobs,
            if self.incremental { "1" } else { "0" },
            self.toolchain,
            self.package
        )
    }

    fn cargo_incremental_value(&self) -> &'static str {
        if self.incremental { "1" } else { "0" }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCheckReport {
    pub command_display: String,
    pub exit_status: Option<i32>,
    pub duration_ms: u128,
    pub success: bool,
    pub stdout_summary: Vec<String>,
    pub stderr_summary: Vec<String>,
    pub diagnostics: Vec<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub timed_out: bool,
}

impl BuildCheckReport {
    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "build check: success={} exit_status={} duration_ms={} truncated={} timed_out={}",
            self.success,
            self.exit_status
                .map(|status| status.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            self.duration_ms,
            self.stdout_truncated || self.stderr_truncated,
            self.timed_out
        )];
        lines.push(format!("command: {}", self.command_display));
        if !self.diagnostics.is_empty() {
            lines.push("diagnostics:".to_string());
            lines.extend(
                self.diagnostics
                    .iter()
                    .map(|diagnostic| format!("  {diagnostic}")),
            );
        }
        if !self.stdout_summary.is_empty() {
            lines.push("stdout summary:".to_string());
            lines.extend(self.stdout_summary.iter().map(|line| format!("  {line}")));
        }
        if !self.stderr_summary.is_empty() {
            lines.push("stderr summary:".to_string());
            lines.extend(self.stderr_summary.iter().map(|line| format!("  {line}")));
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

#[allow(clippy::expect_used)] // child stdout/stderr are guaranteed `Some` by piped Stdio config
pub fn run_build_check(request: &BuildCheckRequest) -> std::io::Result<BuildCheckReport> {
    let start = Instant::now();
    let mut command = Command::new(&request.cargo_program);
    command
        .arg(format!("+{}", request.toolchain))
        .arg("check")
        .arg("--manifest-path")
        .arg("vac-rs/Cargo.toml")
        .arg("-p")
        .arg(&request.package)
        .current_dir(&request.repo_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .env("CARGO_BUILD_JOBS", request.jobs.to_string())
        .env("CARGO_INCREMENTAL", request.cargo_incremental_value())
        .env("PATH", env_or_empty("PATH"));
    for key in ["HOME", "CARGO_HOME", "RUSTUP_HOME", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }

    let mut child = command.spawn()?;
    let mut stdout_stream = child.stdout.take().expect("stdout pipe configured");
    let mut stderr_stream = child.stderr.take().expect("stderr pipe configured");

    let stdout_thread = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout_stream.read_to_end(&mut buffer);
        buffer
    });
    let stderr_thread = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr_stream.read_to_end(&mut buffer);
        buffer
    });

    let deadline = start + request.timeout;
    let mut timed_out = false;
    let mut exit_status: Option<ExitStatus> = None;
    loop {
        match child.try_wait()? {
            Some(status) => {
                exit_status = Some(status);
                break;
            }
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }

    let stdout_bytes = stdout_thread.join().unwrap_or_default();
    let stderr_bytes = stderr_thread.join().unwrap_or_default();
    let duration = start.elapsed();
    let (exit_code, success) = match exit_status {
        Some(status) => (status.code(), status.success()),
        None => (None, false),
    };
    let mut report = report_from_output(
        request.command_display(),
        duration,
        exit_code,
        success,
        &stdout_bytes,
        &stderr_bytes,
    );
    report.timed_out = timed_out;
    if timed_out {
        let banner = format!("build check timed out after {}s", request.timeout.as_secs());
        report.diagnostics.insert(0, banner);
    }
    Ok(report)
}

pub fn report_from_output(
    command_display: String,
    duration: Duration,
    exit_status: Option<i32>,
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> BuildCheckReport {
    let (stdout_summary, stdout_truncated) = summarize_output(stdout);
    let (stderr_summary, stderr_truncated) = summarize_output(stderr);
    let diagnostics = build_diagnostics(&stdout_summary, &stderr_summary);
    BuildCheckReport {
        command_display,
        exit_status,
        duration_ms: duration.as_millis(),
        success,
        stdout_summary,
        stderr_summary,
        diagnostics,
        stdout_truncated,
        stderr_truncated,
        timed_out: false,
    }
}

fn env_or_empty(key: &str) -> OsString {
    std::env::var_os(key).unwrap_or_default()
}

fn discover_toolchain_channel(repo_root: &Path) -> Option<String> {
    let toolchain_path = repo_root.join("vac-rs/rust-toolchain.toml");
    let text = fs::read_to_string(toolchain_path).ok()?;
    parse_toolchain_channel(&text)
}

fn parse_toolchain_channel(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        let (key, value) = trimmed.split_once('=')?;
        if key.trim() != "channel" {
            return None;
        }
        let channel = value.trim().trim_matches('"').trim_matches('\'');
        if channel.is_empty() {
            None
        } else {
            Some(channel.to_string())
        }
    })
}

fn summarize_output(bytes: &[u8]) -> (Vec<String>, bool) {
    let text = String::from_utf8_lossy(bytes).replace('\r', "");
    let mut lines = text
        .lines()
        .map(redact_sensitive_line)
        .map(truncate_line)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let truncated = lines.len() > SUMMARY_MAX_LINES;
    if truncated {
        lines = lines[lines.len() - SUMMARY_MAX_LINES..].to_vec();
    }
    (lines, truncated)
}

fn build_diagnostics(stdout_summary: &[String], stderr_summary: &[String]) -> Vec<String> {
    let mut diagnostics: Vec<String> = stdout_summary
        .iter()
        .chain(stderr_summary.iter())
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("error:") || lower.contains("warning:")
        })
        .cloned()
        .collect();
    if let Some(missing) = detect_missing_toolchain(stdout_summary, stderr_summary) {
        diagnostics.push(missing);
    }
    diagnostics
}

fn detect_missing_toolchain(
    stdout_summary: &[String],
    stderr_summary: &[String],
) -> Option<String> {
    for line in stdout_summary.iter().chain(stderr_summary.iter()) {
        let lower = line.to_ascii_lowercase();
        if lower.contains("is not installed") && lower.contains("toolchain") {
            return Some(format!(
                "hint: rustup toolchain missing; install with `rustup toolchain install <toolchain>` ({line})"
            ));
        }
    }
    None
}

fn truncate_line(line: String) -> String {
    let mut truncated = String::new();
    for (index, character) in line.chars().enumerate() {
        if index >= SUMMARY_MAX_LINE_CHARS {
            truncated.push('…');
            return truncated;
        }
        truncated.push(character);
    }
    truncated
}

fn redact_sensitive_line(line: &str) -> String {
    line.split_whitespace()
        .map(redact_sensitive_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_sensitive_token(token: &str) -> String {
    let Some((key, _value)) = token.split_once('=') else {
        return token.to_string();
    };
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>()
        .to_ascii_uppercase();
    if normalized.contains("TOKEN")
        || normalized.contains("SECRET")
        || normalized.contains("PASSWORD")
        || normalized.contains("API_KEY")
    {
        format!("{key}=<redacted>")
    } else {
        token.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    fn make_fake_cargo(script: &str) -> (tempfile::TempDir, PathBuf) {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempfile::tempdir().expect("tempdir");
        let cargo = tempdir.path().join("cargo");
        let mut file = fs::File::create(&cargo).expect("fake cargo");
        file.write_all(script.as_bytes())
            .expect("fake cargo script");
        let mut permissions = file.metadata().expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&cargo, permissions).expect("permissions");
        (tempdir, cargo)
    }

    #[test]
    fn request_uses_allowlisted_cargo_check_command() {
        let request = BuildCheckRequest::for_repo_root("/repo");

        assert_eq!(request.package, "vac-surface-cli");
        assert_eq!(request.toolchain, "1.95.0");
        assert_eq!(request.jobs, 1);
        assert!(!request.incremental);
        assert_eq!(
            request.command_display(),
            "CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.95.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli"
        );
        assert_eq!(
            request.manifest_path(),
            PathBuf::from("/repo/vac-rs/Cargo.toml")
        );
    }

    #[test]
    fn request_uses_toolchain_from_repo_root_when_present() {
        let root = std::env::temp_dir().join(format!(
            "vac-build-check-toolchain-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("vac-rs")).expect("repo root");
        fs::write(
            root.join("vac-rs/rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.99.0\"\n",
        )
        .expect("toolchain");

        let request = BuildCheckRequest::for_repo_root(&root);
        assert_eq!(request.toolchain, "1.99.0");
        assert!(
            request.command_display().contains("cargo +1.99.0 check"),
            "{}",
            request.command_display()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn report_summarizes_diagnostics_and_redacts_sensitive_tokens() {
        let stderr =
            b"warning: one\nTOKEN=abc SECRET=value PASSWORD=pw API_KEY=key\nerror: failed\n";
        let report = report_from_output(
            "cargo check".to_string(),
            Duration::from_millis(12),
            Some(1),
            false,
            b"ok\n",
            stderr,
        );

        assert!(!report.success);
        assert_eq!(report.exit_status, Some(1));
        assert_eq!(report.duration_ms, 12);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("warning: one"))
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("error: failed"))
        );
        let rendered = report.render_text();
        assert!(rendered.contains("TOKEN=<redacted>"));
        assert!(rendered.contains("SECRET=<redacted>"));
        assert!(rendered.contains("PASSWORD=<redacted>"));
        assert!(rendered.contains("API_KEY=<redacted>"));
        assert!(!rendered.contains("TOKEN=abc"));
    }

    #[test]
    fn report_truncates_to_last_summary_lines() {
        let output = (0..20)
            .map(|index| format!("line {index}"))
            .collect::<Vec<_>>()
            .join("\n");

        let report = report_from_output(
            "cargo check".to_string(),
            Duration::from_millis(1),
            Some(0),
            true,
            output.as_bytes(),
            b"",
        );

        assert!(report.stdout_truncated);
        assert_eq!(report.stdout_summary.len(), 12);
        assert_eq!(
            report.stdout_summary.first().map(String::as_str),
            Some("line 8")
        );
        assert_eq!(
            report.stdout_summary.last().map(String::as_str),
            Some("line 19")
        );
    }

    #[test]
    #[cfg(unix)]
    fn fake_cargo_success_generates_success_report() {
        let (tempdir, cargo) =
            make_fake_cargo("#!/bin/sh\nprintf 'checked %s\\n' \"$*\"\nexit 0\n");
        let repo = tempdir.path().join("repo");
        fs::create_dir_all(repo.join("vac-rs")).expect("repo");
        fs::write(repo.join("vac-rs/Cargo.toml"), "[workspace]\n").expect("manifest");

        let request = BuildCheckRequest::for_repo_root(&repo).with_cargo_program(cargo);
        let report = run_build_check(&request).expect("fake cargo report");

        assert!(report.success, "{}", report.render_text());
        assert_eq!(report.exit_status, Some(0));
        assert!(report.stdout_summary.iter().any(|line| {
            line.contains("+1.95.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli")
        }));
    }

    #[test]
    #[cfg(unix)]
    fn fake_cargo_failure_generates_failure_report() {
        let (tempdir, cargo) = make_fake_cargo(
            "#!/bin/sh\nprintf 'warning: fake warn\\nerror: fake fail\\nTOKEN=abc\\n' >&2\nexit 42\n",
        );
        let repo = tempdir.path().join("repo");
        fs::create_dir_all(repo.join("vac-rs")).expect("repo");
        fs::write(repo.join("vac-rs/Cargo.toml"), "[workspace]\n").expect("manifest");

        let request = BuildCheckRequest::for_repo_root(&repo).with_cargo_program(cargo);
        let report = run_build_check(&request).expect("fake cargo report");

        assert!(!report.success);
        assert_eq!(report.exit_status, Some(42));
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("warning: fake warn"))
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("error: fake fail"))
        );
        assert!(report.render_text().contains("TOKEN=<redacted>"));
    }

    #[test]
    #[cfg(unix)]
    fn fake_cargo_timeout_kills_long_running_process() {
        let (tempdir, cargo) = make_fake_cargo("#!/bin/sh\nexec sleep 5\n");
        let repo = tempdir.path().join("repo");
        fs::create_dir_all(repo.join("vac-rs")).expect("repo");
        fs::write(repo.join("vac-rs/Cargo.toml"), "[workspace]\n").expect("manifest");

        let request = BuildCheckRequest::for_repo_root(&repo)
            .with_cargo_program(cargo)
            .with_timeout(Duration::from_millis(200));
        let started = Instant::now();
        let report = run_build_check(&request).expect("fake cargo report");
        let elapsed = started.elapsed();

        assert!(report.timed_out, "{}", report.render_text());
        assert!(!report.success);
        assert!(report.exit_status.is_none());
        assert!(
            elapsed < Duration::from_secs(2),
            "timeout enforcement should kill child quickly, elapsed={elapsed:?}"
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("build check timed out")),
            "{}",
            report.render_text()
        );
    }

    #[test]
    fn with_toolchain_builder_overrides_default() {
        let request =
            BuildCheckRequest::for_repo_root("/repo").with_toolchain("nightly-2026-05-01");
        assert_eq!(request.toolchain, "nightly-2026-05-01");
        assert!(
            request
                .command_display()
                .contains("cargo +nightly-2026-05-01 check")
        );
    }

    #[test]
    fn missing_toolchain_diagnostic_surfaces_hint() {
        let stderr = b"error: toolchain '1.95.0' is not installed\n";
        let report = report_from_output(
            "cargo +1.95.0 check".to_string(),
            Duration::from_millis(5),
            Some(1),
            false,
            b"",
            stderr,
        );

        assert!(!report.success);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("rustup toolchain install")),
            "{}",
            report.render_text()
        );
    }
}
