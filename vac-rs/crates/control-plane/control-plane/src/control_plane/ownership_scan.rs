use super::capability_manifest::CapabilityOwner;
use super::capability_manifest::CapabilityOwnership;
use super::capability_manifest::CapabilityOwnershipTarget;
use super::capability_manifest::CapabilityOwnershipTargetKind;
use super::capability_manifest::CapabilityStatus;
use super::registry::ControlPlaneRegistry;
use super::registry_diagnostics::RegistryLoadReport;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipScanEntry {
    pub path: String,
    pub id: String,
    pub title: String,
    pub status: CapabilityStatus,
    pub owner: String,
    pub ownership: Option<CapabilityOwnership>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipTargetEntry {
    pub crate_name: String,
    pub module: String,
    pub capabilities: Vec<String>,
    pub test_only: bool,
    pub retired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInventoryEntry {
    pub crate_name: String,
    pub module: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceInventoryReport {
    entries: Vec<SourceInventoryEntry>,
}

impl SourceInventoryReport {
    pub fn entries(&self) -> &[SourceInventoryEntry] {
        &self.entries
    }

    pub fn crate_count(&self) -> usize {
        self.entries
            .iter()
            .map(|entry| entry.crate_name.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn module_count(&self) -> usize {
        self.entries.len()
    }

    pub fn contains_module(&self, crate_name: &str, module: &str) -> bool {
        self.entries.iter().any(|entry| {
            entry.crate_name == crate_name && module_domain_matches(&entry.module, module)
        })
    }

    pub fn contains_module_any_crate(&self, module: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| module_domain_matches(&entry.module, module))
    }

    pub fn missing_claimed_modules(&self, ownership: &CapabilityOwnership) -> Vec<String> {
        let mut missing = Vec::new();
        if ownership.targets.is_empty() {
            for module in &ownership.modules {
                let found = if ownership.crates.is_empty() {
                    self.contains_module_any_crate(module)
                } else {
                    ownership
                        .crates
                        .iter()
                        .any(|crate_name| self.contains_module(crate_name, module))
                };
                if !found {
                    missing.push(module.clone());
                }
            }
        }
        for target in &ownership.targets {
            if target.retired {
                continue;
            }
            if let Some((crate_name, module)) = ownership_target_module_domain(target)
                && !self.contains_module(crate_name, module)
            {
                missing.push(format!("{crate_name}::{module}"));
            }
        }
        missing
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnownedSourceDomainEntry {
    pub crate_name: String,
    pub module: String,
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnershipScanReport {
    registry: RegistryLoadReport,
    targets: Vec<OwnershipTargetEntry>,
    entries: Vec<OwnershipScanEntry>,
    source_inventory: SourceInventoryReport,
    unowned_source_domains: Vec<UnownedSourceDomainEntry>,
}

impl OwnershipScanReport {
    pub fn registry(&self) -> &RegistryLoadReport {
        &self.registry
    }

    pub fn entries(&self) -> &[OwnershipScanEntry] {
        &self.entries
    }

    pub fn targets(&self) -> &[OwnershipTargetEntry] {
        &self.targets
    }

    pub fn source_inventory(&self) -> &SourceInventoryReport {
        &self.source_inventory
    }

    pub fn unowned_source_domains(&self) -> &[UnownedSourceDomainEntry] {
        &self.unowned_source_domains
    }

    pub fn owned_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.ownership.is_some())
            .count()
    }

    pub fn unowned_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.ownership.is_none())
            .count()
    }

    pub fn ready_unowned_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| {
                entry.ownership.is_none() && matches!(entry.status, CapabilityStatus::Ready)
            })
            .count()
    }

    pub fn is_failure(&self) -> bool {
        self.registry.is_failure()
            || self.ready_unowned_count() > 0
            || self.unowned_source_domain_count() > 0
    }

    pub fn target_count(&self) -> usize {
        self.targets.len()
    }

    pub fn test_only_target_count(&self) -> usize {
        self.targets.iter().filter(|entry| entry.test_only).count()
    }

    pub fn retired_target_count(&self) -> usize {
        self.targets.iter().filter(|entry| entry.retired).count()
    }

    pub fn source_domain_count(&self) -> usize {
        self.source_inventory.module_count()
    }

    pub fn unowned_source_domain_count(&self) -> usize {
        self.unowned_source_domains.len()
    }

    pub fn missing_claimed_modules(&self, ownership: &CapabilityOwnership) -> Vec<String> {
        self.source_inventory.missing_claimed_modules(ownership)
    }

    pub fn render_scan_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "ownership scan: total={} owned={} unowned={} ready_unowned={}",
            self.entries.len(),
            self.owned_count(),
            self.unowned_count(),
            self.ready_unowned_count()
        )];
        lines.push(format!(
            "ownership targets: total={} test_only={} retired={}",
            self.target_count(),
            self.test_only_target_count(),
            self.retired_target_count()
        ));
        lines.push(format!(
            "source domains: total={} unowned={}",
            self.source_domain_count(),
            self.unowned_source_domain_count()
        ));
        if self.unowned_source_domains.is_empty() {
            lines.push("source domains matrix: <none hidden>".to_string());
        } else {
            lines.push("source domains matrix:".to_string());
            lines.push("  ┌──────────────────────────────────────┬────────────────────────────────────────┐".to_string());
            lines.push("  │ Unowned Domain (Crate/Module)        │ Hidden File Path                       │".to_string());
            lines.push("  ├──────────────────────────────────────┼────────────────────────────────────────┤".to_string());
            for entry in &self.unowned_source_domains {
                let domain_name = format!("{}/{}", entry.crate_name, entry.module);
                lines.push(format!(
                    "  │ {:<36} │ {:<38} │",
                    truncate_or_pad(&domain_name, 36),
                    truncate_or_pad(&entry.source_path, 38)
                ));
            }
            lines.push("  └──────────────────────────────────────┴────────────────────────────────────────┘".to_string());
            for entry in &self.unowned_source_domains {
                lines.push(format!(
                    "  hidden: {}/{} hidden source={}",
                    entry.crate_name, entry.module, entry.source_path
                ));
            }
        }
        if self.targets.is_empty() {
            lines.push("ownership targets matrix: <none>".to_string());
        } else {
            lines.push("ownership targets matrix:".to_string());
            lines.push(
                "  ┌──────────────────────────────┬────────────────────────┬────────────┐"
                    .to_string(),
            );
            lines.push(
                "  │ Target Crate/Module          │ Claimed Capabilities   │ Class      │"
                    .to_string(),
            );
            lines.push(
                "  ├──────────────────────────────┼────────────────────────┼────────────┤"
                    .to_string(),
            );
            for entry in &self.targets {
                let target_name = format!("{}/{}", entry.crate_name, entry.module);
                let capabilities = entry.capabilities.join(", ");
                let classification = if entry.retired {
                    "retired"
                } else if entry.test_only {
                    "test_only"
                } else {
                    "owned"
                };
                lines.push(format!(
                    "  │ {:<28} │ {:<22} │ {:<10} │",
                    truncate_or_pad(&target_name, 28),
                    truncate_or_pad(&capabilities, 22),
                    truncate_or_pad(classification, 10)
                ));
            }
            lines.push(
                "  └──────────────────────────────┴────────────────────────┴────────────┘"
                    .to_string(),
            );
            for entry in &self.targets {
                let classification = if entry.retired {
                    "retired"
                } else if entry.test_only {
                    "test_only"
                } else {
                    "owned"
                };
                let capabilities = entry.capabilities.join(", ");
                lines.push(format!(
                    "  target: {}/{} classification: {} capabilities=[{}]",
                    entry.crate_name, entry.module, classification, capabilities
                ));
            }
        }

        if self.entries.is_empty() {
            lines.push("ownership matrix: <none>".to_string());
            return lines;
        }

        lines.push("ownership matrix:".to_string());
        for (index, entry) in self.entries.iter().enumerate() {
            lines.push(format!("  {}. {} — {}", index + 1, entry.id, entry.title));
            lines.push(format!("     path: {}", entry.path));
            lines.push(format!(
                "     status: {}",
                format_capability_status(entry.status)
            ));
            lines.push(format!("     owner: {}", entry.owner));
            match entry.ownership.as_ref() {
                Some(ownership) => {
                    let targets_desc = if ownership.targets.is_empty() {
                        "".to_string()
                    } else {
                        let target_strings: Vec<String> = ownership
                            .targets
                            .iter()
                            .map(|t| {
                                format!(
                                    "{}{}{}",
                                    ownership_target_display(t),
                                    if t.test_only { " (test_only)" } else { "" },
                                    if t.retired { " (retired)" } else { "" }
                                )
                            })
                            .collect();
                        format!(" targets=[{}]", target_strings.join(", "))
                    };
                    let deletion_desc = if let Some(plan) = &ownership.deletion_plan {
                        format!(" deletion_plan=\"{plan}\"")
                    } else {
                        "".to_string()
                    };
                    lines.push(format!(
                        "     ownership: crates=[{}] modules=[{}]{}{} test_only={} retired={}",
                        ownership.crates.join(", "),
                        ownership.modules.join(", "),
                        targets_desc,
                        deletion_desc,
                        ownership.test_only,
                        ownership.retired
                    ));
                    let missing = self.missing_claimed_modules(ownership);
                    if !missing.is_empty() {
                        lines.push(format!(
                            "     warning: claimed modules missing from source inventory [{}]",
                            missing.join(", ")
                        ));
                    }
                }
                None => {
                    lines.push("     ownership: <missing>".to_string());
                    lines.push("     warning: capability missing ownership metadata".to_string());
                }
            }
            if entry.ownership.is_none() && matches!(entry.status, CapabilityStatus::Ready) {
                lines.push("     warning: ready capability missing ownership metadata".to_string());
            }
        }
        lines
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = self.registry.render_lines();
        lines.extend(self.render_scan_lines());
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

pub fn load_ownership_scan_report(start: impl AsRef<Path>) -> OwnershipScanReport {
    let registry = super::registry::load_control_plane_registry_report(start);
    let entries = registry.registry().map(scan_entries).unwrap_or_default();
    let targets = build_targets(&entries);
    let source_inventory = registry
        .vac_root()
        .map(build_source_inventory_from_vac_root)
        .unwrap_or_default();
    let unowned_source_domains = build_unowned_source_domains(&source_inventory, &targets);
    OwnershipScanReport {
        registry,
        targets,
        entries,
        source_inventory,
        unowned_source_domains,
    }
}

pub fn build_ownership_scan_report_for_registry(
    registry: &ControlPlaneRegistry,
) -> OwnershipScanReport {
    let entries = scan_entries(registry);
    let targets = build_targets(&entries);
    let source_inventory = build_source_inventory_from_vac_root(&registry.vac_root);
    let unowned_source_domains = build_unowned_source_domains(&source_inventory, &targets);
    OwnershipScanReport {
        registry: RegistryLoadReport::empty(),
        targets,
        entries,
        source_inventory,
        unowned_source_domains,
    }
}

pub fn build_source_inventory_from_vac_root(vac_root: impl AsRef<Path>) -> SourceInventoryReport {
    let Some(repo_root) = vac_root.as_ref().parent() else {
        return SourceInventoryReport::default();
    };
    let source_root = repo_root.join("vac-rs");
    if !source_root.is_dir() {
        return SourceInventoryReport::default();
    }

    let mut cargo_tomls = Vec::new();
    collect_cargo_tomls(&source_root, &mut cargo_tomls);
    let mut entries = Vec::new();
    for cargo_toml in cargo_tomls {
        entries.extend(source_inventory_entries_from_cargo_toml(
            &source_root,
            &cargo_toml,
        ));
    }
    entries.sort_by(|left, right| {
        left.crate_name
            .cmp(&right.crate_name)
            .then_with(|| left.module.cmp(&right.module))
            .then_with(|| left.source_path.cmp(&right.source_path))
    });
    entries.dedup_by(|left, right| {
        left.crate_name == right.crate_name
            && left.module == right.module
            && left.source_path == right.source_path
    });
    SourceInventoryReport { entries }
}

fn collect_cargo_tomls(dir: &Path, cargo_tomls: &mut Vec<PathBuf>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("target") {
            continue;
        }
        if path.is_dir() {
            collect_cargo_tomls(&path, cargo_tomls);
        } else if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
            cargo_tomls.push(path);
        }
    }
}

fn source_inventory_entries_from_cargo_toml(
    source_root: &Path,
    cargo_toml: &Path,
) -> Vec<SourceInventoryEntry> {
    let Ok(contents) = fs::read_to_string(cargo_toml) else {
        return Vec::new();
    };
    let Some(crate_name) = parse_package_name(&contents) else {
        return Vec::new();
    };
    let Some(crate_dir) = cargo_toml.parent() else {
        return Vec::new();
    };
    let src_dir = crate_dir.join("src");
    let mut modules = BTreeMap::<String, String>::new();

    for root_file in ["lib.rs", "main.rs"] {
        let path = src_dir.join(root_file);
        if !path.is_file() {
            continue;
        }
        let root_module = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(root_file);
        modules.insert(
            root_module.to_string(),
            display_source_path(source_root, &path),
        );
        collect_declared_source_modules(source_root, &src_dir, &path, None, &mut modules);
    }
    for module in parse_declared_bin_modules(&contents) {
        modules
            .entry(module.clone())
            .or_insert_with(|| module_source_path(source_root, &src_dir, &module));
    }

    modules
        .into_iter()
        .map(|(module, source_path)| SourceInventoryEntry {
            crate_name: crate_name.clone(),
            module,
            source_path,
        })
        .collect()
}

fn parse_package_name(contents: &str) -> Option<String> {
    let mut in_package = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package && line.starts_with("name") {
            let (_, value) = line.split_once('=')?;
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn collect_declared_source_modules(
    source_root: &Path,
    src_dir: &Path,
    source_file: &Path,
    parent_module: Option<&str>,
    modules: &mut BTreeMap<String, String>,
) {
    for include_file in parse_included_source_files(source_file) {
        collect_declared_source_modules(
            source_root,
            src_dir,
            &include_file,
            parent_module,
            modules,
        );
    }
    for module in parse_declared_modules(source_file) {
        let dotted_module = parent_module
            .map(|parent| format!("{parent}.{module}"))
            .unwrap_or_else(|| module.clone());
        let source_path =
            module_source_path_for_parent(source_root, src_dir, parent_module, &module);
        modules.entry(dotted_module.clone()).or_insert(source_path);
        let nested_file = module_file_path_for_parent(src_dir, parent_module, &module);
        if nested_file.is_file() {
            collect_declared_source_modules(
                source_root,
                src_dir,
                &nested_file,
                Some(&dotted_module),
                modules,
            );
        }
    }
}

fn parse_declared_modules(path: &Path) -> Vec<String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut modules = Vec::new();
    for raw_line in contents.lines() {
        let mut line = raw_line.trim();
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if let Some(stripped) = line.strip_prefix("pub(crate) ") {
            line = stripped.trim_start();
        } else if let Some(stripped) = line.strip_prefix("pub ") {
            line = stripped.trim_start();
        }
        let Some(rest) = line.strip_prefix("mod ") else {
            continue;
        };
        let name = rest
            .split(|character: char| {
                character == ';' || character == '{' || character.is_whitespace()
            })
            .next()
            .unwrap_or("")
            .trim();
        if is_module_name(name) {
            modules.push(name.to_string());
        }
    }
    modules
}

fn parse_included_source_files(path: &Path) -> Vec<PathBuf> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Some(parent_dir) = path.parent() else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let Some(rest) = line.strip_prefix("include!(") else {
            continue;
        };
        let Some((literal, _)) = rest.split_once(')') else {
            continue;
        };
        let include_path = literal
            .trim()
            .trim_end_matches(';')
            .trim()
            .trim_matches('"');
        if include_path.is_empty() {
            continue;
        }
        let include_file = parent_dir.join(include_path);
        if include_file.is_file() {
            files.push(include_file);
        }
    }
    files
}

fn parse_declared_bin_modules(contents: &str) -> Vec<String> {
    let mut modules = Vec::new();
    let mut in_bin = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_bin = line == "[[bin]]";
            continue;
        }
        if in_bin
            && line.starts_with("path")
            && let Some((_, value)) = line.split_once('=')
        {
            let path = value.trim().trim_matches('"');
            if let Some(stem) = Path::new(path).file_stem().and_then(|stem| stem.to_str())
                && is_module_name(stem)
            {
                modules.push(stem.to_string());
            }
        }
    }
    modules
}

fn module_source_path(source_root: &Path, src_dir: &Path, module: &str) -> String {
    display_source_path(source_root, &module_file_path(src_dir, module))
}

fn module_source_path_for_parent(
    source_root: &Path,
    src_dir: &Path,
    parent_module: Option<&str>,
    module: &str,
) -> String {
    display_source_path(
        source_root,
        &module_file_path_for_parent(src_dir, parent_module, module),
    )
}

fn module_file_path(src_dir: &Path, module: &str) -> PathBuf {
    module_file_path_for_parent(src_dir, None, module)
}

fn module_file_path_for_parent(
    src_dir: &Path,
    parent_module: Option<&str>,
    module: &str,
) -> PathBuf {
    let mut parts: Vec<&str> = parent_module
        .map(|parent| parent.split('.').collect())
        .unwrap_or_default();
    parts.push(module);

    let mut file = src_dir.to_path_buf();
    for part in &parts {
        file.push(part);
    }
    file.set_extension("rs");
    if file.is_file() {
        return file;
    }

    let mut mod_file = src_dir.to_path_buf();
    for part in &parts {
        mod_file.push(part);
    }
    mod_file.push("mod.rs");
    if mod_file.is_file() {
        return mod_file;
    }

    file
}

fn display_source_path(source_root: &Path, path: &Path) -> String {
    path.strip_prefix(source_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn is_module_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn build_unowned_source_domains(
    source_inventory: &SourceInventoryReport,
    targets: &[OwnershipTargetEntry],
) -> Vec<UnownedSourceDomainEntry> {
    // Plan 21 enforcement is intentionally repo-wide: orphan crates/modules with
    // zero ownership targets are not allowed to stay invisible just because no
    // capability has claimed their crate yet. A source module is considered
    // visible when any capability claims it, including test_only or retired
    // classifications. Parent module claims cover dotted child modules
    // (`control_plane` covers `control_plane.foo`). Classifications stay
    // observable in the ownership target matrix instead of becoming duplicate
    // hidden-domain rows.
    source_inventory
        .entries()
        .iter()
        .filter(|source| {
            !is_ignorable_source_domain(source)
                && !targets
                    .iter()
                    .any(|target| target_claims_source_domain(target, source))
        })
        .map(|source| UnownedSourceDomainEntry {
            crate_name: source.crate_name.clone(),
            module: source.module.clone(),
            source_path: source.source_path.clone(),
        })
        .collect()
}

fn target_claims_source_domain(
    target: &OwnershipTargetEntry,
    source: &SourceInventoryEntry,
) -> bool {
    target.crate_name == source.crate_name && module_domain_matches(&source.module, &target.module)
}

fn module_domain_matches(source_module: &str, claimed_module: &str) -> bool {
    let source_module = normalize_module_domain(source_module);
    let claimed_module = normalize_module_domain(claimed_module);
    if source_module == claimed_module {
        return true;
    }
    if source_module
        .strip_prefix(&claimed_module)
        .is_some_and(|suffix| suffix.starts_with('.'))
    {
        return true;
    }
    if claimed_module
        .strip_prefix(&source_module)
        .is_some_and(|suffix| suffix.starts_with('.'))
    {
        return true;
    }

    let wrapped_claim = format!(".{claimed_module}");
    source_module.ends_with(&wrapped_claim) || source_module.contains(&format!("{wrapped_claim}."))
}

fn normalize_module_domain(module: &str) -> String {
    module.replace("::", ".")
}

fn is_ignorable_source_domain(source: &SourceInventoryEntry) -> bool {
    matches!(source.module.as_str(), "lib" | "main" | "tests")
        || source.module.ends_with(".tests")
        || source.module.ends_with("_tests")
        || source.module.contains(".tests.")
}

fn scan_entries(registry: &ControlPlaneRegistry) -> Vec<OwnershipScanEntry> {
    registry
        .capabilities
        .manifests
        .iter()
        .map(|entry| OwnershipScanEntry {
            path: entry.path.display().to_string(),
            id: entry.manifest.id.clone(),
            title: entry.manifest.title.clone(),
            status: entry.manifest.status,
            owner: format_capability_owner(&entry.manifest.owner),
            ownership: entry.manifest.ownership.clone(),
        })
        .collect()
}

fn build_targets(entries: &[OwnershipScanEntry]) -> Vec<OwnershipTargetEntry> {
    let mut targets = BTreeMap::<(String, String), OwnershipTargetEntry>::new();

    for entry in entries {
        let Some(ownership) = entry.ownership.as_ref() else {
            continue;
        };

        // 1. Classic Cartesian crates/modules. When granular targets are
        // present, crates/modules are summary metadata only; treating both as
        // active claims recreates the historical overclaim problem.
        if ownership.targets.is_empty() {
            for crate_name in &ownership.crates {
                for module in &ownership.modules {
                    let key = (crate_name.clone(), module.clone());
                    let target = targets.entry(key).or_insert_with(|| OwnershipTargetEntry {
                        crate_name: crate_name.clone(),
                        module: module.clone(),
                        capabilities: Vec::new(),
                        test_only: ownership.test_only,
                        retired: ownership.retired,
                    });
                    if !target
                        .capabilities
                        .iter()
                        .any(|capability| capability == &entry.id)
                    {
                        target.capabilities.push(entry.id.clone());
                    }
                    target.test_only &= ownership.test_only;
                    target.retired &= ownership.retired;
                }
            }
        }

        // 2. Granular targets (solves Cartesian overclaims!)
        for raw_target in &ownership.targets {
            let Some((crate_name, module)) = ownership_target_module_domain(raw_target) else {
                continue;
            };
            let key = (crate_name.to_string(), module.to_string());
            let target = targets.entry(key).or_insert_with(|| OwnershipTargetEntry {
                crate_name: crate_name.to_string(),
                module: module.to_string(),
                capabilities: Vec::new(),
                test_only: raw_target.test_only,
                retired: raw_target.retired,
            });
            if !target
                .capabilities
                .iter()
                .any(|capability| capability == &entry.id)
            {
                target.capabilities.push(entry.id.clone());
            }
            target.test_only &= raw_target.test_only;
            target.retired &= raw_target.retired;
        }
    }

    let mut targets: Vec<_> = targets.into_values().collect();
    for target in &mut targets {
        target.capabilities.sort();
    }
    targets.sort_by(|left, right| {
        left.crate_name
            .cmp(&right.crate_name)
            .then_with(|| left.module.cmp(&right.module))
    });
    targets
}

fn ownership_target_module_domain(target: &CapabilityOwnershipTarget) -> Option<(&str, &str)> {
    if !matches!(target.kind, CapabilityOwnershipTargetKind::Module) {
        return None;
    }
    Some((target.crate_name.as_deref()?, target.module.as_deref()?))
}

fn ownership_target_display(target: &CapabilityOwnershipTarget) -> String {
    match target.kind {
        CapabilityOwnershipTargetKind::Module => {
            match (target.crate_name.as_deref(), target.module.as_deref()) {
                (Some(crate_name), Some(module)) => format!("{crate_name}::{module}"),
                (Some(crate_name), None) => format!("{crate_name}::<missing-module>"),
                (None, Some(module)) => format!("<missing-crate>::{module}"),
                (None, None) => "module:<missing>".to_string(),
            }
        }
        CapabilityOwnershipTargetKind::Crate => target
            .crate_name
            .as_deref()
            .map(|crate_name| format!("{crate_name}::*"))
            .unwrap_or_else(|| "crate:<missing>".to_string()),
        CapabilityOwnershipTargetKind::Path => {
            let include = if target.include.is_empty() {
                "<empty>".to_string()
            } else {
                target.include.join("|")
            };
            format!("path:{include}")
        }
    }
}

fn format_capability_owner(owner: &CapabilityOwner) -> String {
    match owner {
        CapabilityOwner::Path(path) => path.clone(),
        CapabilityOwner::Structured(object) => {
            let mut parts = vec![format!("{}/{}", object.crate_name, object.module)];
            if let Some(team) = object.team.as_deref()
                && !team.trim().is_empty()
            {
                parts.push(format!("team={team}"));
            }
            parts.join(" ")
        }
    }
}

fn format_capability_status(status: CapabilityStatus) -> &'static str {
    match status {
        CapabilityStatus::Planned => "planned",
        CapabilityStatus::Partial => "partial",
        CapabilityStatus::Ready => "ready",
        CapabilityStatus::Deprecated => "deprecated",
        CapabilityStatus::Disabled => "disabled",
    }
}

impl fmt::Display for OwnershipScanReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render_text())
    }
}

fn truncate_or_pad(s: &str, width: usize) -> String {
    if s.len() > width {
        let mut truncated = s[..width - 3].to_string();
        truncated.push_str("...");
        truncated
    } else {
        format!("{s:<width$}")
    }
}

#[cfg(test)]
#[path = "ownership_scan_tests.rs"]
mod tests;
