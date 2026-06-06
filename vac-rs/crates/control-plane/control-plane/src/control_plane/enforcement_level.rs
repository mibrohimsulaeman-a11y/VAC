use serde::Deserialize;
use serde_yaml::Value;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EnforcementLevel {
    L1,
    L2,
}

impl EnforcementLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::L1 => "L1",
            Self::L2 => "L2",
        }
    }
}

impl fmt::Display for EnforcementLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnforcementSandboxObserved {
    pub seccomp: bool,
    pub landlock: bool,
    pub namespace: bool,
    pub broker_mediated: bool,
}

impl EnforcementSandboxObserved {
    pub const fn l1() -> Self {
        Self {
            seccomp: false,
            landlock: false,
            namespace: false,
            broker_mediated: false,
        }
    }

    pub const fn is_l2(self) -> bool {
        self.seccomp && self.landlock && self.namespace && self.broker_mediated
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnforcementObserved {
    pub direct_fs_blocked: bool,
    pub direct_proc_blocked: bool,
    pub direct_network_blocked: bool,
    pub sandbox: EnforcementSandboxObserved,
}

impl EnforcementObserved {
    pub const fn l1() -> Self {
        Self {
            direct_fs_blocked: false,
            direct_proc_blocked: false,
            direct_network_blocked: false,
            sandbox: EnforcementSandboxObserved::l1(),
        }
    }

    pub const fn is_l2(self) -> bool {
        self.direct_fs_blocked
            && self.direct_proc_blocked
            && self.direct_network_blocked
            && self.sandbox.is_l2()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnforcementClaimScope {
    pub fail_closed_enforcement: bool,
    pub out_of_band_blocked: bool,
    pub advisory_only: bool,
}

impl EnforcementClaimScope {
    pub const fn l1() -> Self {
        Self {
            fail_closed_enforcement: false,
            out_of_band_blocked: false,
            advisory_only: true,
        }
    }

    pub const fn l2() -> Self {
        Self {
            fail_closed_enforcement: true,
            out_of_band_blocked: true,
            advisory_only: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnforcementStatusReport {
    pub workspace_root: PathBuf,
    pub claimed_level: EnforcementLevel,
    pub explicit_claim: bool,
    pub observed: EnforcementObserved,
    pub claim_scope: EnforcementClaimScope,
    pub infos: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl EnforcementStatusReport {
    pub fn cli_exit_code(&self) -> i32 {
        if self.errors.is_empty() { 0 } else { 1 }
    }

    pub fn is_failure(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![
            "VAC Doctor Enforcement".to_string(),
            "======================".to_string(),
            format!("workspace_root: {}", self.workspace_root.display()),
            format!("enforcement_level: {}", self.claimed_level),
            format!(
                "claim_source: {}",
                if self.explicit_claim {
                    "explicit"
                } else {
                    "defaulted"
                }
            ),
            format!(
                "observed_level: {}",
                if self.observed.is_l2() { "l2" } else { "l1" }
            ),
            format!("direct_fs_blocked: {}", self.observed.direct_fs_blocked),
            format!("direct_proc_blocked: {}", self.observed.direct_proc_blocked),
            format!(
                "direct_network_blocked: {}",
                self.observed.direct_network_blocked
            ),
            "sandbox:".to_string(),
            format!("  seccomp: {}", self.observed.sandbox.seccomp),
            format!("  landlock: {}", self.observed.sandbox.landlock),
            format!("  namespace: {}", self.observed.sandbox.namespace),
            format!(
                "  broker_mediated: {}",
                self.observed.sandbox.broker_mediated
            ),
            "claim_scope:".to_string(),
            format!(
                "  fail_closed_enforcement: {}",
                self.claim_scope.fail_closed_enforcement
            ),
            format!(
                "  out_of_band_blocked: {}",
                self.claim_scope.out_of_band_blocked
            ),
            format!("  advisory_only: {}", self.claim_scope.advisory_only),
        ];
        if self.claimed_level == EnforcementLevel::L1 {
            lines.push(
                "L1 — advisory/cooperative mode; guarantees reduced to discipline + audit"
                    .to_string(),
            );
        }
        for info in &self.infos {
            lines.push(format!("INFO: {info}"));
        }
        for warning in &self.warnings {
            lines.push(format!("WARN: {warning}"));
        }
        for error in &self.errors {
            lines.push(format!("ERROR: {error}"));
        }
        lines.join("\n")
    }
}

pub fn load_enforcement_status_report(workspace_root: impl AsRef<Path>) -> EnforcementStatusReport {
    let workspace_root = workspace_root.as_ref().to_path_buf();
    let explicit_claim = discover_explicit_claim(&workspace_root);
    let claimed_level = explicit_claim.unwrap_or(EnforcementLevel::L1);
    let observed = detect_observed_enforcement();
    let claim_scope = discover_claim_scope(&workspace_root, claimed_level).unwrap_or_else(|| {
        if claimed_level == EnforcementLevel::L2 {
            EnforcementClaimScope::l2()
        } else {
            EnforcementClaimScope::l1()
        }
    });
    let mut infos = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if explicit_claim.is_none() {
        warnings.push(
            "no explicit enforcement claim found; defaulting to L1 advisory/cooperative mode"
                .to_string(),
        );
    }

    if claimed_level == EnforcementLevel::L2 && !observed.is_l2() {
        errors.push(
            "registry claims L2, but current substrate only demonstrates L1 advisory behavior"
                .to_string(),
        );
        errors.push(
            "claim L2 only when fail-closed FS/PROC/NET blocking and sandbox mediation are observed"
                .to_string(),
        );
    } else if claimed_level == EnforcementLevel::L2 {
        infos.push("L2 claim is consistent with observed fail-closed enforcement".to_string());
    } else {
        infos.push(
            "L1 — advisory/cooperative mode; guarantees reduced to discipline + audit".to_string(),
        );
    }

    EnforcementStatusReport {
        workspace_root,
        claimed_level,
        explicit_claim: explicit_claim.is_some(),
        observed,
        claim_scope,
        infos,
        warnings,
        errors,
    }
}

fn discover_explicit_claim(workspace_root: &Path) -> Option<EnforcementLevel> {
    if let Some(level) = env_claim_level() {
        return Some(level);
    }
    for path in [
        workspace_root.join(".vac/registry/init_state.yaml"),
        workspace_root.join(".vac/registry/registry_status.yaml"),
    ] {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_yaml::from_str::<Value>(&source) else {
            continue;
        };
        if let Some(level) = nested_enforcement_level(&value, &["enforcement_level"]) {
            return Some(level);
        }
        if let Some(level) = nested_enforcement_level(&value, &["enforcement", "level"]) {
            return Some(level);
        }
    }
    None
}

fn discover_claim_scope(
    workspace_root: &Path,
    claimed_level: EnforcementLevel,
) -> Option<EnforcementClaimScope> {
    for path in [
        workspace_root.join(".vac/registry/init_state.yaml"),
        workspace_root.join(".vac/registry/registry_status.yaml"),
    ] {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_yaml::from_str::<Value>(&source) else {
            continue;
        };
        let fail_closed_enforcement =
            nested_bool(&value, &["claim_scope", "fail_closed_enforcement"]);
        let out_of_band_blocked = nested_bool(&value, &["claim_scope", "out_of_band_blocked"]);
        let advisory_only = nested_bool(&value, &["claim_scope", "advisory_only"]);
        if fail_closed_enforcement.is_some()
            || out_of_band_blocked.is_some()
            || advisory_only.is_some()
        {
            return Some(EnforcementClaimScope {
                fail_closed_enforcement: fail_closed_enforcement
                    .unwrap_or(claimed_level == EnforcementLevel::L2),
                out_of_band_blocked: out_of_band_blocked
                    .unwrap_or(claimed_level == EnforcementLevel::L2),
                advisory_only: advisory_only.unwrap_or(claimed_level == EnforcementLevel::L1),
            });
        }
    }
    None
}

fn env_claim_level() -> Option<EnforcementLevel> {
    match std::env::var("VAC_ENFORCEMENT_LEVEL")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("l2" | "2" | "fail_closed" | "fail-closed") => Some(EnforcementLevel::L2),
        Some("l1" | "1" | "advisory" | "advisory_only") => Some(EnforcementLevel::L1),
        _ => None,
    }
}

fn nested_enforcement_level(value: &Value, keys: &[&str]) -> Option<EnforcementLevel> {
    nested_scalar(value, keys).and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
        "l1" | "1" | "advisory" | "advisory_only" => Some(EnforcementLevel::L1),
        "l2" | "2" | "fail_closed" | "fail-closed" => Some(EnforcementLevel::L2),
        _ => None,
    })
}

fn nested_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    nested_scalar(value, keys).and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    })
}

fn nested_scalar(value: &Value, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = current
            .as_mapping()
            .and_then(|mapping| mapping.get(Value::String((*key).to_string())))?;
    }
    match current {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn detect_observed_enforcement() -> EnforcementObserved {
    let direct_fs_blocked = env_bool("VAC_ENFORCEMENT_OBSERVED_DIRECT_FS_BLOCKED").unwrap_or(false);
    let direct_proc_blocked =
        env_bool("VAC_ENFORCEMENT_OBSERVED_DIRECT_PROC_BLOCKED").unwrap_or(false);
    let direct_network_blocked =
        env_bool("VAC_ENFORCEMENT_OBSERVED_DIRECT_NETWORK_BLOCKED").unwrap_or(false);
    let sandbox = EnforcementSandboxObserved {
        seccomp: env_bool("VAC_ENFORCEMENT_OBSERVED_SECCOMP").unwrap_or_else(detect_seccomp),
        landlock: env_bool("VAC_ENFORCEMENT_OBSERVED_LANDLOCK").unwrap_or_else(detect_landlock),
        namespace: env_bool("VAC_ENFORCEMENT_OBSERVED_NAMESPACE")
            .unwrap_or_else(detect_user_namespace),
        broker_mediated: env_bool("VAC_ENFORCEMENT_OBSERVED_BROKER_MEDIATED").unwrap_or(false),
    };

    EnforcementObserved {
        direct_fs_blocked,
        direct_proc_blocked,
        direct_network_blocked,
        sandbox,
    }
}

fn env_bool(name: &str) -> Option<bool> {
    std::env::var(name)
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
}

fn detect_seccomp() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if let Some(value) = line.strip_prefix("Seccomp:") {
                    return value
                        .trim()
                        .parse::<u8>()
                        .map(|value| value > 0)
                        .unwrap_or(false);
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn detect_landlock() -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new("/sys/kernel/security/landlock").exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn detect_user_namespace() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(uid_map) = fs::read_to_string("/proc/self/uid_map") {
            let compact = uid_map.lines().map(str::trim).collect::<Vec<_>>().join(" ");
            return !compact.contains("0 0 4294967295");
        }
        false
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_l1_advisory_without_explicit_claim() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let report = load_enforcement_status_report(tempdir.path());
        assert_eq!(report.claimed_level, EnforcementLevel::L1);
        assert!(!report.explicit_claim);
        assert!(report.errors.is_empty(), "{}", report.render_text());
        assert!(
            report
                .render_text()
                .contains("L1 — advisory/cooperative mode")
        );
    }

    #[test]
    fn explicit_l1_claim_is_allowed() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let registry = tempdir.path().join(".vac/registry");
        fs::create_dir_all(&registry).expect("create registry");
        fs::write(
            registry.join("init_state.yaml"),
            "schema_version: 1\nkind: init_state\nid: init.state\nenforcement_level: L1\n",
        )
        .expect("write init state");
        let report = load_enforcement_status_report(tempdir.path());
        assert_eq!(report.claimed_level, EnforcementLevel::L1);
        assert!(report.explicit_claim);
        assert!(report.errors.is_empty(), "{}", report.render_text());
    }

    #[test]
    fn l2_overclaim_is_blocked_without_l2_substrate() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let registry = tempdir.path().join(".vac/registry");
        fs::create_dir_all(&registry).expect("create registry");
        fs::write(
            registry.join("registry_status.yaml"),
            "schema_version: 1\nkind: registry_status\nid: registry.status\nenforcement_level: L2\n",
        )
        .expect("write registry status");
        let report = load_enforcement_status_report(tempdir.path());
        assert_eq!(report.claimed_level, EnforcementLevel::L2);
        assert!(report.is_failure(), "{}", report.render_text());
        assert!(
            report
                .errors
                .iter()
                .any(|message| message.contains("registry claims L2"))
        );
    }
}
