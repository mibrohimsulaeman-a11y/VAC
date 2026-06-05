use clap::Parser;
use clap::Subcommand;
use std::fs;
use std::path::{Path, PathBuf};

/// Manage VAC registry migrations.
#[derive(Debug, Parser)]
pub struct RegistryCommand {
    #[command(subcommand)]
    command: RegistrySubcommand,
}

impl RegistryCommand {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            RegistrySubcommand::EvidenceV2(command) => command.run(),
            RegistrySubcommand::Migrate(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum RegistrySubcommand {
    /// Manage signed evidence v2 registry records.
    EvidenceV2(RegistryEvidenceV2Command),

    /// Dry-run or apply checked registry migrations.
    Migrate(RegistryMigrateCommand),
}

#[derive(Debug, Parser)]
struct RegistryEvidenceV2Command {
    #[command(subcommand)]
    command: RegistryEvidenceV2Subcommand,
}

impl RegistryEvidenceV2Command {
    fn run(self) -> anyhow::Result<()> {
        match self.command {
            RegistryEvidenceV2Subcommand::Resign(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum RegistryEvidenceV2Subcommand {
    /// Re-sign existing evidence v2 records using configured Ed25519 identities.
    Resign(RegistryEvidenceV2ResignCommand),
}

#[derive(Debug, Parser)]
struct RegistryEvidenceV2ResignCommand {
    /// Apply signature rewrites. Required to avoid accidental registry mutation.
    #[arg(long, default_value_t = false)]
    apply: bool,

    /// Workspace root.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl RegistryEvidenceV2ResignCommand {
    fn run(self) -> anyhow::Result<()> {
        if !self.apply {
            anyhow::bail!(
                "vac registry evidence-v2 resign requires --apply after setting broker/operator signing keys"
            );
        }

        let root = normalize_root(&self.path)?;
        let signer =
            vac_core::control_plane::EvidenceV2Signer::require_broker_and_operator_from_env()
                .map_err(anyhow::Error::msg)?;
        let store = vac_core::control_plane::EvidenceV2GitRefStore::new_with_signer(&root, signer);
        let report = store
            .resign_existing_records()
            .map_err(|err| anyhow::anyhow!("{err:?}"))?;

        println!("vac registry evidence-v2 resign: PASS");
        println!("workspace: {}", root.display());
        println!("evidence_records: {}", report.evidence_records);
        println!("xref_markers: {}", report.xref_markers);
        println!("anchors: {}", report.anchors);
        println!("rewritten_files: {}", report.rewritten_files.len());
        Ok(())
    }
}

#[derive(Debug, Parser)]
struct RegistryMigrateCommand {
    /// Show migration effects without writing.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Apply migrations through atomic writes.
    #[arg(long, default_value_t = false)]
    apply: bool,

    /// Workspace root.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl RegistryMigrateCommand {
    fn run(self) -> anyhow::Result<()> {
        if self.dry_run == self.apply {
            anyhow::bail!("vac registry migrate requires exactly one of --dry-run or --apply");
        }

        let root = normalize_root(&self.path)?;
        let migration_dir = root.join(".vac/registry/migrations");
        if !migration_dir.is_dir() {
            anyhow::bail!(
                "vac registry migrate requires migration directory {}",
                migration_dir.display()
            );
        }

        let engine_previews =
            vac_core::control_plane::preview_vac_init_registry_migrations_with_engine(&root)
                .map_err(anyhow::Error::msg)?;

        let mut migrations = Vec::new();
        for entry in fs::read_dir(&migration_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
                let source = fs::read_to_string(&path)?;
                migrations.push((path, source));
            }
        }
        migrations.sort_by(|left, right| left.0.cmp(&right.0));

        println!("vac registry migrate");
        println!("mode: {}", if self.dry_run { "dry_run" } else { "apply" });
        println!("workspace: {}", root.display());
        println!("engine: vac_init_migration_runtime");
        println!("migrations: {}", engine_previews.len());
        for preview in &engine_previews {
            println!("  - {}", preview.migration_id);
            println!(
                "      version: {} -> {}",
                preview.from_version, preview.to_version
            );
            println!("      verification: {}", preview.verification_command_id);
            for change in &preview.changes {
                println!(
                    "      {:?}: {}:{} reversible={}",
                    change.action, change.target, change.field, change.reversible
                );
            }
        }

        let mut applied = 0usize;
        for (_path, source) in migrations {
            for target in migration_targets(&source) {
                if self.apply {
                    apply_compatibility_kind_cleanup(&root, &target)?;
                    applied += 1;
                }
            }
        }

        if self.apply {
            println!("applied_targets: {applied}");
        }
        Ok(())
    }
}

fn migration_targets(source: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("target:") {
            targets.push(
                value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            );
        }
    }
    targets.sort();
    targets.dedup();
    targets
}

fn apply_compatibility_kind_cleanup(root: &Path, target: &str) -> anyhow::Result<()> {
    if target.starts_with('/') || target.contains("..") || target.contains('\\') {
        anyhow::bail!("invalid migration target `{target}`");
    }
    let path = root.join(target);
    if !path.exists() {
        return Ok(());
    }
    let source = fs::read_to_string(&path)?;
    let mut migrated = source.replace(
        "kind: product\n",
        "kind: registry_status\nlegacy_kind: product\n",
    );
    migrated = migrated.replace(
        "kind: status\n",
        "kind: registry_status\nlegacy_kind: status\n",
    );
    migrated = migrated.replace(
        "kind: donor_inventory\n",
        "kind: registry_status\nlegacy_kind: donor_inventory\n",
    );
    if migrated != source {
        vac_core::control_plane::write_vac_init_store_record_atomic(root, target, &migrated)
            .map_err(anyhow::Error::msg)?;
    }
    Ok(())
}

fn normalize_root(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path.to_path_buf())
    }
}
