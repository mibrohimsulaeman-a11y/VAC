use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use super::registry::find_vac_root;

/// Severity tier reported by a docs-maintenance scan finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocsMaintenanceSeverity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for DocsMaintenanceSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// A single docs-maintenance finding: severity, offending path, message, and an optional fix hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsMaintenanceIssue {
    severity: DocsMaintenanceSeverity,
    path: PathBuf,
    message: String,
    hint: Option<String>,
}

impl DocsMaintenanceIssue {
    pub fn new(
        severity: DocsMaintenanceSeverity,
        path: impl Into<PathBuf>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            path: path.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "{}: {}: {}",
            self.severity,
            self.path.display(),
            self.message
        )];
        if let Some(hint) = self.hint.as_deref() {
            lines.push(format!("  hint: {hint}"));
        }
        lines
    }
}

/// Aggregate docs-maintenance scan result: counters, resolved repo root, and the list of findings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsMaintenanceReport {
    repo_root: Option<PathBuf>,
    docs_checked: usize,
    links_checked: usize,
    manifest_dirs_checked: usize,
    manifest_dirs_present: usize,
    issues: Vec<DocsMaintenanceIssue>,
}

impl DocsMaintenanceReport {
    pub fn repo_root(&self) -> Option<&Path> {
        self.repo_root.as_deref()
    }

    pub fn docs_checked(&self) -> usize {
        self.docs_checked
    }

    pub fn links_checked(&self) -> usize {
        self.links_checked
    }

    pub fn manifest_dirs_checked(&self) -> usize {
        self.manifest_dirs_checked
    }

    pub fn manifest_dirs_present(&self) -> usize {
        self.manifest_dirs_present
    }

    pub fn issues(&self) -> &[DocsMaintenanceIssue] {
        &self.issues
    }

    pub fn is_failure(&self) -> bool {
        !self.issues.is_empty()
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(if self.is_failure() {
            "docs maintenance: failure".to_string()
        } else {
            "docs maintenance: ok".to_string()
        });
        lines.push(format!(
            "docs maintenance summary: docs_checked={} links_checked={} manifest_dirs_present={}/{} issues={}",
            self.docs_checked,
            self.links_checked,
            self.manifest_dirs_present,
            self.manifest_dirs_checked,
            self.issues.len()
        ));
        match self.repo_root() {
            Some(repo_root) => lines.push(format!("repo root: {}", repo_root.display())),
            None => lines.push("repo root: <unknown>".to_string()),
        }
        if self.issues.is_empty() {
            lines.push(
                "docs maintenance: docs index links and manifest directories are aligned"
                    .to_string(),
            );
            return lines;
        }

        lines.push("docs maintenance issues:".to_string());
        for issue in &self.issues {
            lines.extend(issue.render_lines());
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

/// Scan the VAC repository rooted at `start` and return a [`DocsMaintenanceReport`]
/// covering required docs files, control-plane index links, and `.vac/` manifest directories.
///
/// The scan walks up from `start` to locate the `.vac/` root, verifies the seven required
/// docs files (root `README.md` and `AGENTS.md`, the control-plane `INDEX.md` family, and
/// `docs/legal/NOTICES.md`), validates README/AGENTS pointers, expands the local link graph
/// for each indexed docs file, and checks every `.vac/` manifest directory has at least one
/// YAML manifest. Findings are accumulated rather than short-circuited so an operator sees
/// every regression in one pass.
pub fn load_docs_maintenance_report(start: impl AsRef<Path>) -> DocsMaintenanceReport {
    let mut issues = Vec::new();
    let mut docs_checked = 0usize;
    let mut links_checked = 0usize;
    let mut manifest_dirs_checked = 0usize;
    let mut manifest_dirs_present = 0usize;

    let Ok(vac_root) = find_vac_root(start) else {
        issues.push(
            DocsMaintenanceIssue::new(
                DocsMaintenanceSeverity::Error,
                PathBuf::from(".vac"),
                "unable to locate `.vac` root",
            )
            .with_hint("run the command from the repository root or a subdirectory inside it"),
        );
        return DocsMaintenanceReport {
            repo_root: None,
            docs_checked,
            links_checked,
            manifest_dirs_checked,
            manifest_dirs_present,
            issues,
        };
    };

    let repo_root = vac_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| vac_root.clone());

    for rel in [
        "README.md",
        "AGENTS.md",
        "docs/workflow-control-plane/INDEX.md",
        "docs/workflow-control-plane/IMPLEMENTATION_PLAN.md",
        "docs/workflow-control-plane/plans/INDEX.md",
        "docs/workflow-control-plane/schema/INDEX.md",
        "docs/legal/NOTICES.md",
    ] {
        let path = repo_root.join(rel);
        docs_checked += 1;
        if !path.is_file() {
            issues.push(
                DocsMaintenanceIssue::new(
                    DocsMaintenanceSeverity::Error,
                    path,
                    "required docs file missing",
                )
                .with_hint("restore the file or update the control-plane docs index"),
            );
        }
    }

    for (file, required_link) in [
        ("README.md", "docs/workflow-control-plane/INDEX.md"),
        ("AGENTS.md", "docs/workflow-control-plane/INDEX.md"),
        ("README.md", "docs/legal/NOTICES.md"),
    ] {
        let path = repo_root.join(file);
        if let Ok(text) = fs::read_to_string(&path) {
            links_checked += 1;
            if !text.contains(required_link) {
                issues.push(
                    DocsMaintenanceIssue::new(
                        DocsMaintenanceSeverity::Error,
                        path,
                        format!("missing control-plane docs link `{required_link}`"),
                    )
                    .with_hint("point the file at docs/workflow-control-plane/INDEX.md"),
                );
            }
        }
    }

    for rel in [
        "docs/workflow-control-plane/INDEX.md",
        "docs/workflow-control-plane/plans/INDEX.md",
        "docs/workflow-control-plane/schema/INDEX.md",
    ] {
        let path = repo_root.join(rel);
        if let Ok(text) = fs::read_to_string(&path) {
            for link in extract_local_links(&text) {
                links_checked += 1;
                let target = resolve_link_target(&repo_root, &path, &link);
                if !target.exists() {
                    issues.push(
                        DocsMaintenanceIssue::new(
                            DocsMaintenanceSeverity::Error,
                            path.clone(),
                            format!("stale docs link `{link}` -> `{}`", target.display()),
                        )
                        .with_hint("update the link target or restore the referenced file"),
                    );
                }
            }
        }
    }

    for rel in [
        ".vac/capabilities",
        ".vac/workflows",
        ".vac/policies",
        ".vac/surfaces",
        ".vac/registry",
    ] {
        let path = repo_root.join(rel);
        manifest_dirs_checked += 1;
        match fs::read_dir(&path) {
            Ok(entries) => {
                let has_manifest = entries
                    .filter_map(Result::ok)
                    .any(|entry| entry.path().is_file() && is_yaml(&entry.path()));
                if has_manifest {
                    manifest_dirs_present += 1;
                } else {
                    issues.push(
                        DocsMaintenanceIssue::new(
                            DocsMaintenanceSeverity::Error,
                            path,
                            "manifest directory exists but contains no YAML manifests",
                        )
                        .with_hint("seed at least one manifest or update the docs plan if the directory is intentionally empty"),
                    );
                }
            }
            Err(_) => {
                issues.push(
                    DocsMaintenanceIssue::new(
                        DocsMaintenanceSeverity::Error,
                        path,
                        "manifest directory missing",
                    )
                    .with_hint("restore the directory or update the control-plane docs index"),
                );
            }
        }
    }

    DocsMaintenanceReport {
        repo_root: Some(repo_root),
        docs_checked,
        links_checked,
        manifest_dirs_checked,
        manifest_dirs_present,
        issues,
    }
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("yaml" | "yml")
    )
}

fn resolve_link_target(repo_root: &Path, source: &Path, link: &str) -> PathBuf {
    let link = link.split('#').next().unwrap_or(link);
    let link = link.split('?').next().unwrap_or(link);
    if link.starts_with("http://") || link.starts_with("https://") || link.starts_with("mailto:") {
        return PathBuf::from(link);
    }
    if let Some(stripped) = link.strip_prefix('/') {
        return repo_root.join(stripped);
    }
    let base_dir = source.parent().unwrap_or(repo_root);
    base_dir.join(link)
}

fn extract_local_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut offset = 0usize;

    while let Some(open_bracket) = text[offset..].find("](") {
        let after_open = offset + open_bracket + 2;
        let Some(close_paren) = text[after_open..].find(')') else {
            break;
        };
        let candidate = text[after_open..after_open + close_paren].trim();
        if !candidate.is_empty()
            && !candidate.starts_with("http://")
            && !candidate.starts_with("https://")
            && !candidate.starts_with("mailto:")
            && !candidate.starts_with('#')
        {
            links.push(candidate.to_string());
        }
        offset = after_open + close_paren + 1;
    }

    links
}

impl fmt::Display for DocsMaintenanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render_text())
    }
}

#[cfg(test)]
mod tests {
    use super::load_docs_maintenance_report;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    fn make_temp_repo() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vac-docs-maintenance-{unique}"));
        fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(root.join(".vac/workflows")).expect("workflows dir");
        fs::create_dir_all(root.join(".vac/policies")).expect("policies dir");
        fs::create_dir_all(root.join(".vac/surfaces")).expect("surfaces dir");
        fs::create_dir_all(root.join(".vac/registry")).expect("registry dir");
        root
    }

    fn write_manifest(path: impl AsRef<Path>) {
        fs::write(path, "schema_version: 1\nkind: capability\nid: vac.test\n").expect("manifest");
    }

    fn seed_docs_index(root: &Path, broken_link: bool) {
        fs::write(
            root.join("README.md"),
            "[Workflow control plane](docs/workflow-control-plane/INDEX.md)\n[Legal notices](docs/legal/NOTICES.md)\n",
        )
        .expect("readme");
        fs::write(
            root.join("AGENTS.md"),
            "[Workflow control plane](docs/workflow-control-plane/INDEX.md)\n[Legal notices](docs/legal/NOTICES.md)\n",
        )
        .expect("agents");
        fs::create_dir_all(root.join("docs/workflow-control-plane/plans")).expect("plans dir");
        fs::create_dir_all(root.join("docs/workflow-control-plane/schema")).expect("schema dir");
        fs::create_dir_all(root.join("docs/legal")).expect("legal docs dir");
        fs::write(
            root.join("docs/workflow-control-plane/INDEX.md"),
            if broken_link {
                "[Broken](plans/MISSING.md)\n"
            } else {
                "[Plans](plans/INDEX.md)\n[Schema](schema/INDEX.md)\n[Implementation](IMPLEMENTATION_PLAN.md)\n[Legal notices](../legal/NOTICES.md)\n"
            },
        )
        .expect("control plane index");
        fs::write(
            root.join("docs/workflow-control-plane/IMPLEMENTATION_PLAN.md"),
            "# Plan\n",
        )
        .expect("implementation plan");
        fs::write(
            root.join("docs/workflow-control-plane/plans/INDEX.md"),
            "[Plan 22](22-enterprise-hygiene.md)\n",
        )
        .expect("plan index");
        fs::write(
            root.join("docs/workflow-control-plane/plans/22-enterprise-hygiene.md"),
            "# Plan 22\n",
        )
        .expect("plan 22");
        fs::write(
            root.join("docs/workflow-control-plane/schema/INDEX.md"),
            "[Capability](capability-manifest.schema.md)\n",
        )
        .expect("schema index");
        fs::write(
            root.join("docs/workflow-control-plane/schema/capability-manifest.schema.md"),
            "# schema\n",
        )
        .expect("schema doc");
        fs::write(root.join("docs/legal/NOTICES.md"), "# Legal notices\n")
            .expect("legal notices doc");
    }

    #[test]
    fn docs_maintenance_report_passes_when_docs_and_manifests_are_present() {
        let root = make_temp_repo();
        seed_docs_index(&root, false);
        write_manifest(root.join(".vac/capabilities/chat.yaml"));
        write_manifest(root.join(".vac/workflows/maintenance.build-check.yaml"));
        write_manifest(root.join(".vac/policies/default-local.yaml"));
        write_manifest(root.join(".vac/surfaces/capabilities.yaml"));
        write_manifest(root.join(".vac/registry/product.yaml"));

        let report = load_docs_maintenance_report(&root);
        let rendered = report.render_text();
        assert!(
            rendered.contains("docs maintenance: ok"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("docs maintenance summary: docs_checked=7"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("manifest_dirs_present=5/5"),
            "rendered:\n{rendered}"
        );
        assert_eq!(report.issues().len(), 0, "rendered:\n{rendered}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn docs_maintenance_report_flags_stale_link_and_missing_manifest_dir() {
        let root = make_temp_repo();
        seed_docs_index(&root, true);
        write_manifest(root.join(".vac/capabilities/chat.yaml"));
        // Leave the workflow dir empty on purpose to trigger the manifest-dir check.
        write_manifest(root.join(".vac/policies/default-local.yaml"));
        write_manifest(root.join(".vac/surfaces/capabilities.yaml"));
        write_manifest(root.join(".vac/registry/product.yaml"));

        let report = load_docs_maintenance_report(&root);
        let rendered = report.render_text();
        assert!(report.is_failure(), "rendered:\n{rendered}");
        assert!(
            rendered.contains("stale docs link `plans/MISSING.md`"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("manifest directory exists but contains no YAML manifests"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
