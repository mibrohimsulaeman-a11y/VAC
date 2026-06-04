use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Architectural Boundary (Plan 00F Alignment):
/// Per Plan 00F (Runtime-retirement), these directory components are explicitly
/// quarantined from the TUI uniqueness scanner. The `donor/` tree preserves
/// retired upstream frontends for backend porting only — they must NOT trigger
/// duplicate-TUI findings even though they contain legacy TUI vocabulary.
/// `.git/` and `target/` are skipped for performance/correctness.
const PLAN_00F_QUARANTINED_DIRS: &[&str] = &[".git", "donor", "target"];

/// Explicit scan roots. `docs/` is deliberately excluded so this plan's own
/// documentation (which legitimately quotes forbidden vocabulary as part of
/// the invariant description) does not self-trigger the scanner. Allow-listed
/// docs that DO need scanning can opt in via `NO_DUPLICATE_TUI_EXEMPT_FILES`,
/// though that allowlist is currently inactive because no doc root is scanned.
const NO_DUPLICATE_TUI_SCAN_ROOTS: &[&str] = &["vac-rs", ".vac"];

const NO_DUPLICATE_TUI_EXEMPT_FILES: &[&str] = &[
    ".vac/registry/status.yaml",
    ".vac/registry/donor-inventory.yaml",
    ".vac/workflows/README.md",
    ".vac/workflows/maintenance.no-duplicate-tui.yaml",
    ".vac/workflows/maintenance.release-gate.yaml",
    "docs/product/CAPABILITY_MAP.md",
    "docs/product/domain-prds/tui-action-recorder-replay.md",
    "vac-rs/crates/control-plane/control-plane/src/control_plane/donor_domain_contract.rs",
    "vac-rs/crates/control-plane/control-plane/src/control_plane/identity_check.rs",
    "vac-rs/crates/control-plane/control-plane/src/control_plane/no_duplicate_tui.rs",
    "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs",
    "vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_runner/**",
];

/// TUI dependency crates that, combined with a binary target or a TUI-themed
/// crate name, indicate a structural duplicate TUI outside the root TUI crate.
const TUI_DEPENDENCY_CRATES: &[&str] = &["ratatui", "crossterm"];

const TERM_VAC_TUI_RUNTIME: &str = concat!("vac", "_tui", "_runtime");
const TERM_VAC_SHELL_RUNTIME_LOOP: &str = concat!("vac", "_shell", "_runtime_loop");
const TERM_SECOND_TUI_RUNTIME: &str = concat!("second ", "TUI runtime");
const TERM_RENAMED_TERMINAL_APP: &str = concat!("renamed terminal ", "app");
const TERM_OLD_PRODUCT_ASSUMPTION: &str = concat!("old ", "product assumption");

const FORBIDDEN_TERMS: &[&str] = &[
    TERM_VAC_TUI_RUNTIME,
    TERM_VAC_SHELL_RUNTIME_LOOP,
    TERM_SECOND_TUI_RUNTIME,
    TERM_RENAMED_TERMINAL_APP,
    TERM_OLD_PRODUCT_ASSUMPTION,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoDuplicateTuiFinding {
    pub path: String,
    pub line: usize,
    pub term: String,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoDuplicateTuiReport {
    scanned_file_count: usize,
    findings: Vec<NoDuplicateTuiFinding>,
}

impl NoDuplicateTuiReport {
    pub fn scanned_file_count(&self) -> usize {
        self.scanned_file_count
    }

    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn findings(&self) -> &[NoDuplicateTuiFinding] {
        &self.findings
    }

    pub fn summary_line(&self) -> String {
        format!(
            "scanned={} findings={}",
            self.scanned_file_count,
            self.finding_count()
        )
    }

    pub fn failure_reason(&self) -> Option<String> {
        let finding_count = self.finding_count();
        if finding_count == 0 {
            return None;
        }
        let first = &self.findings[0];
        let mut reason = format!(
            "TUI Uniqueness Invariant Violation: A duplicate TUI crate, binary, or forbidden indicator was detected at {}:{} ({}). \
            To prevent fragmentation, UX drift, and maintainability issues, all active operator frontend interfaces must be unified strictly within 'vac-rs/tui/'. \
            For architectural compliance guidelines and the official frontend boundaries, refer to Plan 17: docs/workflow-control-plane/plans/17-maintenance-no-duplicate-tui.md",
            first.path, first.line, first.term
        );
        if finding_count > 1 {
            reason.push_str(&format!(
                " (and {} other duplicate indicators)",
                finding_count - 1
            ));
        }
        Some(reason)
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![format!("tui uniqueness: {}", self.summary_line())];
        if self.findings.is_empty() {
            lines.push("tui uniqueness findings: <none>".to_string());
            return lines;
        }

        lines.push("tui uniqueness findings:".to_string());
        for finding in &self.findings {
            lines.push(format!(
                "  {}:{} {} -> {}",
                finding.path, finding.line, finding.term, finding.snippet
            ));
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

pub fn load_no_duplicate_tui_report(root: impl AsRef<Path>) -> NoDuplicateTuiReport {
    let root = root.as_ref();
    let mut files = Vec::new();
    for relative_root in NO_DUPLICATE_TUI_SCAN_ROOTS {
        collect_no_duplicate_tui_files(root, &root.join(relative_root), &mut files);
    }
    files.sort();
    files.dedup();

    let mut findings = Vec::new();
    for path in &files {
        let Ok(contents) = fs::read_to_string(path) else {
            continue;
        };
        let relative_path = path.strip_prefix(root).unwrap_or(path.as_path());
        let relative = relative_path.display().to_string();
        if let Some(finding) = structural_tui_cargo_finding(relative_path, &contents) {
            findings.push(finding);
        }
        for (line_number, line) in contents.lines().enumerate() {
            let normalized_line = line.to_ascii_lowercase();
            for term in FORBIDDEN_TERMS {
                if normalized_line.contains(&term.to_ascii_lowercase()) {
                    findings.push(NoDuplicateTuiFinding {
                        path: relative.clone(),
                        line: line_number + 1,
                        term: (*term).to_string(),
                        snippet: line.trim().chars().take(160).collect(),
                    });
                }
            }
        }
    }
    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.term.cmp(&right.term))
    });

    NoDuplicateTuiReport {
        scanned_file_count: files.len(),
        findings,
    }
}

/// Minimal subset of a Cargo manifest needed to detect a structural duplicate
/// TUI crate. Parses `[package]`, `[[bin]]`, and `[dependencies]` only.
/// Unknown fields are tolerated — this is a focused TUI-uniqueness probe,
/// not a full Cargo manifest validator (the build gate handles that).
#[derive(Deserialize, Default)]
struct CargoManifest {
    #[serde(default)]
    package: Option<CargoPackage>,
    #[serde(default, rename = "bin")]
    bins: Vec<CargoBin>,
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize)]
struct CargoPackage {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct CargoBin {
    #[serde(default)]
    #[allow(dead_code)]
    name: Option<String>,
}

fn structural_tui_cargo_finding(relative: &Path, contents: &str) -> Option<NoDuplicateTuiFinding> {
    if relative.file_name() != Some(OsStr::new("Cargo.toml")) {
        return None;
    }
    let rel = relative.display().to_string();

    // Exempted: workspace root + active product TUI crate live inside this exempt path.
    if rel == "vac-rs/Cargo.toml" || rel.starts_with("vac-rs/crates/surfaces/tui/") {
        return None;
    }

    // Parse the manifest structurally. Malformed TOML is not a TUI finding —
    // the build gate will catch it. Treat parse errors as "no finding".
    let manifest: CargoManifest = match toml::from_str(contents) {
        Ok(value) => value,
        Err(_) => return None,
    };

    // A pure workspace manifest (no [package]) cannot itself be a duplicate TUI crate.
    let package = manifest.package.as_ref()?;

    let matched_tui_deps: Vec<&str> = TUI_DEPENDENCY_CRATES
        .iter()
        .copied()
        .filter(|name| manifest.dependencies.contains_key(*name))
        .collect();

    if matched_tui_deps.is_empty() {
        return None;
    }

    let has_binary = !manifest.bins.is_empty();
    let package_name = package.name.as_deref().unwrap_or("");
    let suspicious_name = package_name.to_ascii_lowercase().contains("tui");

    if has_binary || suspicious_name {
        return Some(NoDuplicateTuiFinding {
            path: rel,
            line: 1,
            term: "structural duplicate TUI crate".to_string(),
            snippet: format!(
                "Cargo manifest declares a TUI-like crate or binary (deps: {}) outside vac-rs/crates/surfaces/tui",
                matched_tui_deps.join(", ")
            ),
        });
    }

    None
}

fn collect_no_duplicate_tui_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    if !dir.exists() {
        return;
    }
    if should_skip_path(root, dir) {
        return;
    }
    if dir.is_file() {
        if should_scan_file(dir) {
            files.push(dir.to_path_buf());
        }
        return;
    }
    if !dir.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if should_skip_path(root, &path) {
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_no_duplicate_tui_files(root, &path, files);
        } else if file_type.is_file() && should_scan_file(&path) {
            files.push(path);
        }
    }
}

fn should_skip_path(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };

    if NO_DUPLICATE_TUI_EXEMPT_FILES.iter().any(|exempt| {
        if let Some(prefix) = exempt.strip_suffix("/**") {
            relative.starts_with(prefix)
        } else {
            relative == Path::new(exempt)
        }
    }) {
        return true;
    }

    // Skip Plan 00F quarantined dirs — see PLAN_00F_QUARANTINED_DIRS docs.
    relative.components().any(|component| {
        PLAN_00F_QUARANTINED_DIRS
            .iter()
            .any(|dir| component.as_os_str() == OsStr::new(*dir))
    })
}

fn should_scan_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };
    if matches!(file_name, name if name == OsStr::new("Cargo.toml") || name == OsStr::new("Cargo.lock") || name == OsStr::new("package.json") || name == OsStr::new("README"))
    {
        return true;
    }

    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("rs" | "md" | "toml" | "yaml" | "yml" | "json" | "txt" | "lock")
    )
}

#[cfg(test)]
mod tests {
    use super::load_no_duplicate_tui_report;
    use std::fs;

    #[test]
    fn no_duplicate_tui_reports_product_terms_and_ignores_allowlisted_legacy_docs() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let term = concat!("vac", "_tui", "_runtime");

        fs::create_dir_all(tempdir.path().join("vac-rs/src")).expect("product source dir");
        fs::create_dir_all(tempdir.path().join("donor/vac/src")).expect("donor source dir");
        fs::create_dir_all(tempdir.path().join(".git")).expect("git dir");
        fs::create_dir_all(tempdir.path().join("docs/product")).expect("product docs dir");

        fs::write(
            tempdir.path().join("vac-rs/src/lib.rs"),
            format!("pub const LEGACY: &str = \"{term}\";"),
        )
        .expect("product source fixture");
        fs::write(
            tempdir.path().join("donor/vac/src/lib.rs"),
            format!("pub const LEGACY: &str = \"{term}\";"),
        )
        .expect("donor source fixture");
        fs::write(
            tempdir.path().join(".git/HEAD"),
            format!("ref: refs/heads/{term}"),
        )
        .expect("git fixture");
        fs::write(
            tempdir.path().join("docs/product/CAPABILITY_MAP.md"),
            format!("quarantine {term}"),
        )
        .expect("allowlisted legacy docs fixture");

        let report = load_no_duplicate_tui_report(tempdir.path());
        assert_eq!(report.scanned_file_count(), 1);
        assert_eq!(report.finding_count(), 1);
        assert_eq!(report.findings()[0].path, "vac-rs/src/lib.rs");
        assert_eq!(report.findings()[0].line, 1);
        assert_eq!(report.findings()[0].term, term);
    }

    #[test]
    fn no_duplicate_tui_detects_structural_alt_tui_crate() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("vac-rs/alt-tui")).expect("alt tui dir");
        fs::write(
            tempdir.path().join("vac-rs/alt-tui/Cargo.toml"),
            r#"[package]
name = "alt-tui"
version = "0.0.0"

[[bin]]
name = "alt-tui"
path = "src/main.rs"

[dependencies]
ratatui = { workspace = true }
"#,
        )
        .expect("alt tui cargo fixture");

        let report = load_no_duplicate_tui_report(tempdir.path());
        assert_eq!(report.finding_count(), 1);
        assert_eq!(report.findings()[0].term, "structural duplicate TUI crate");

        // Assert the failure reason is highly educational and references Plan 17
        let reason = report.failure_reason().expect("has failure reason");
        assert!(reason.contains("TUI Uniqueness Invariant Violation"));
        assert!(reason.contains("Plan 17"));
        assert!(reason.contains("17-maintenance-no-duplicate-tui.md"));
    }

    #[test]
    fn no_duplicate_tui_ignores_approved_roots_and_ignores_library_usages() {
        let tempdir = tempfile::tempdir().expect("tempdir");

        // Approved workspace manifest passes
        fs::create_dir_all(tempdir.path().join("vac-rs")).expect("vac-rs dir");
        fs::write(
            tempdir.path().join("vac-rs/Cargo.toml"),
            r#"[workspace]
members = ["crates/surfaces/tui", "ansi-escape"]
[workspace.dependencies]
ratatui = "0.29.0"
"#,
        )
        .expect("workspace cargo fixture");

        // Approved active product TUI passes
        fs::create_dir_all(tempdir.path().join("vac-rs/crates/surfaces/tui")).expect("tui dir");
        fs::write(
            tempdir.path().join("vac-rs/crates/surfaces/tui/Cargo.toml"),
            r#"[package]
name = "vac-tui"
version = "0.0.0"
[dependencies]
ratatui = { workspace = true }
"#,
        )
        .expect("tui cargo fixture");

        // General non-TUI utility library (like ansi-escape) that does not define [[bin]] passes
        fs::create_dir_all(tempdir.path().join("vac-rs/ansi-escape")).expect("ansi-escape dir");
        fs::write(
            tempdir.path().join("vac-rs/ansi-escape/Cargo.toml"),
            r#"[package]
name = "ansi-escape"
version = "0.0.0"
[dependencies]
ratatui = { workspace = true }
"#,
        )
        .expect("ansi-escape cargo fixture");

        let report = load_no_duplicate_tui_report(tempdir.path());
        assert_eq!(
            report.finding_count(),
            0,
            "No duplicate findings should be reported, got: {:?}",
            report.findings()
        );
        assert!(report.passed());
    }

    #[test]
    fn no_duplicate_tui_detects_crossterm_based_duplicate_tui_binary() {
        let tempdir = tempfile::tempdir().expect("tempdir");

        fs::create_dir_all(tempdir.path().join("vac-rs/custom-terminal-app"))
            .expect("custom app dir");
        fs::write(
            tempdir.path().join("vac-rs/custom-terminal-app/Cargo.toml"),
            r#"[package]
name = "custom-terminal-app"
version = "0.0.0"

[[bin]]
name = "custom-app"
path = "src/main.rs"

[dependencies]
crossterm = "0.28.1"
"#,
        )
        .expect("cargo manifest fixture");

        let report = load_no_duplicate_tui_report(tempdir.path());
        assert_eq!(report.finding_count(), 1);
        assert_eq!(report.findings()[0].term, "structural duplicate TUI crate");
    }

    #[test]
    fn no_duplicate_tui_does_not_self_trigger_on_plan_doc() {
        // Plan 17's own doc legitimately quotes forbidden vocabulary while
        // describing the invariant. Because `docs/` is not in the scan roots,
        // a plan doc that names a forbidden term must NOT produce a finding.
        let tempdir = tempfile::tempdir().expect("tempdir");
        let term = concat!("vac", "_tui", "_runtime");

        fs::create_dir_all(tempdir.path().join("docs/workflow-control-plane/plans"))
            .expect("plan dir");
        fs::write(
            tempdir
                .path()
                .join("docs/workflow-control-plane/plans/17-maintenance-no-duplicate-tui.md"),
            format!("Forbidden vocabulary referenced for context: {term}"),
        )
        .expect("plan doc fixture");

        let report = load_no_duplicate_tui_report(tempdir.path());
        assert_eq!(report.scanned_file_count(), 0, "docs/ must not be scanned");
        assert_eq!(report.finding_count(), 0);
        assert!(report.passed());
    }

    #[test]
    fn no_duplicate_tui_ignores_malformed_cargo_manifest() {
        // Malformed TOML must not crash or produce a false-positive structural
        // finding. The build gate is responsible for catching syntactically
        // bad manifests; this scanner only flags duplicate TUI shape.
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("vac-rs/broken-crate")).expect("broken crate dir");
        fs::write(
            tempdir.path().join("vac-rs/broken-crate/Cargo.toml"),
            "this is = not valid [[[ toml ratatui crossterm",
        )
        .expect("broken cargo fixture");

        let report = load_no_duplicate_tui_report(tempdir.path());
        assert_eq!(report.finding_count(), 0);
        assert!(report.passed());
    }
}
