#![allow(dead_code)]
//! Registry migration runtime contracts for VAC-Init.
//!
//! This module is intentionally small-dependency and deterministic: it validates
//! migration plans, derives rollback plans, and applies simple registry text
//! transformations in dry-run/apply flows. The actual file-system executor is
//! expected to call `vac doctor registry` after applying the returned actions.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MigrationActionKind {
    AddField,
    RemoveField,
    RenameField,
    ChangeKind,
    ChangeId,
    ChangeType,
    VersionBump,
}

impl MigrationActionKind {
    pub const fn inverse(self) -> Self {
        match self {
            Self::AddField => Self::RemoveField,
            Self::RemoveField => Self::AddField,
            Self::RenameField => Self::RenameField,
            Self::ChangeKind => Self::ChangeKind,
            Self::ChangeId => Self::ChangeId,
            Self::ChangeType => Self::ChangeType,
            Self::VersionBump => Self::VersionBump,
        }
    }

    pub const fn requires_from_to(self) -> bool {
        matches!(
            self,
            Self::RenameField
                | Self::ChangeKind
                | Self::ChangeId
                | Self::ChangeType
                | Self::VersionBump
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationAction {
    pub action: MigrationActionKind,
    pub target: String,
    pub field: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

impl MigrationAction {
    pub fn inverse(&self) -> Self {
        Self {
            action: self.action.inverse(),
            target: self.target.clone(),
            field: self.field.clone(),
            from: self.to.clone(),
            to: self.from.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), MigrationRuntimeError> {
        if self.target.trim().is_empty()
            || self.target.starts_with('/')
            || self.target.contains("..")
        {
            return Err(MigrationRuntimeError::InvalidTarget(self.target.clone()));
        }
        if self.field.trim().is_empty() {
            return Err(MigrationRuntimeError::InvalidAction(
                "migration field is empty".to_string(),
            ));
        }
        if self.action.requires_from_to() && (self.from.is_none() || self.to.is_none()) {
            return Err(MigrationRuntimeError::InvalidAction(
                "migration action requires both from and to values".to_string(),
            ));
        }
        if matches!(self.action, MigrationActionKind::AddField) && self.to.is_none() {
            return Err(MigrationRuntimeError::InvalidAction(
                "add-field migration requires a to value".to_string(),
            ));
        }
        if matches!(self.action, MigrationActionKind::RemoveField) && self.from.is_none() {
            return Err(MigrationRuntimeError::InvalidAction(
                "remove-field migration requires a from value".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredMigrationCommand {
    pub id: String,
    pub runner: String,
    pub args: Vec<String>,
}

impl StructuredMigrationCommand {
    pub fn is_structured(&self) -> bool {
        is_dotted_id(&self.id)
            && !self.runner.trim().is_empty()
            && !self.runner.contains('/')
            && !self.runner.contains('\\')
            && self
                .args
                .iter()
                .all(|arg| !arg.contains("&&") && !arg.contains('|') && !arg.contains('>'))
    }

    pub fn is_registry_doctor(&self) -> bool {
        self.runner == "vac"
            && self.args.first().map(String::as_str) == Some("doctor")
            && self.args.get(1).map(String::as_str) == Some("registry")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRecord {
    pub id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub changes: Vec<MigrationAction>,
    pub rollback: Vec<MigrationAction>,
    pub verification_command: StructuredMigrationCommand,
}

impl MigrationRecord {
    pub fn validate(&self) -> Result<(), String> {
        validate_migration_record_depth(self).map_err(|err| err.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRuntimeRequest {
    pub dry_run: bool,
    pub apply: bool,
    pub migration_id: String,
}

impl MigrationRuntimeRequest {
    pub fn validate_mode(&self) -> Result<(), String> {
        if self.dry_run == self.apply {
            return Err("choose exactly one of dry-run or apply".to_string());
        }
        if !is_dotted_id(&self.migration_id) {
            return Err("migration id must be dotted".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationRuntimeMode {
    DryRun,
    Apply,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRuntimePlan {
    pub migration_id: String,
    pub mode: MigrationRuntimeMode,
    pub actions: Vec<MigrationAction>,
    pub verification_command: StructuredMigrationCommand,
    pub requires_registry_doctor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPreviewChange {
    pub target: String,
    pub action: MigrationActionKind,
    pub field: String,
    pub reversible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationDryRunReport {
    pub migration_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub changes: Vec<MigrationPreviewChange>,
    pub verification_command_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationRuntimeError {
    InvalidId,
    InvalidVersionOrder,
    MissingChanges,
    MissingRollback,
    InvalidVerificationCommand,
    VerificationMustRunRegistryDoctor,
    InvalidTarget(String),
    InvalidAction(String),
    RollbackMismatch { index: usize },
    UnsupportedTextAction(MigrationActionKind),
    TextPatternMissing { target: String, field: String },
}

impl std::fmt::Display for MigrationRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidId => write!(f, "migration id must be dotted"),
            Self::InvalidVersionOrder => write!(f, "migration to_version must be >= from_version"),
            Self::MissingChanges => write!(f, "migration requires changes"),
            Self::MissingRollback => write!(f, "migration requires rollback actions"),
            Self::InvalidVerificationCommand => {
                write!(f, "migration verification command must be structured")
            }
            Self::VerificationMustRunRegistryDoctor => {
                write!(f, "migration verification must run vac doctor registry")
            }
            Self::InvalidTarget(target) => write!(f, "migration target is invalid: {target}"),
            Self::InvalidAction(message) => write!(f, "migration action is invalid: {message}"),
            Self::RollbackMismatch { index } => write!(
                f,
                "rollback action at index {index} is not the inverse of change"
            ),
            Self::UnsupportedTextAction(action) => {
                write!(f, "text migration action is unsupported: {action:?}")
            }
            Self::TextPatternMissing { target, field } => {
                write!(f, "migration field {field} not found in {target}")
            }
        }
    }
}

pub fn validate_migration_record_depth(
    record: &MigrationRecord,
) -> Result<(), MigrationRuntimeError> {
    if !is_dotted_id(&record.id) {
        return Err(MigrationRuntimeError::InvalidId);
    }
    if record.to_version < record.from_version {
        return Err(MigrationRuntimeError::InvalidVersionOrder);
    }
    if record.changes.is_empty() {
        return Err(MigrationRuntimeError::MissingChanges);
    }
    if record.rollback.is_empty() {
        return Err(MigrationRuntimeError::MissingRollback);
    }
    if !record.verification_command.is_structured() {
        return Err(MigrationRuntimeError::InvalidVerificationCommand);
    }
    if !record.verification_command.is_registry_doctor() {
        return Err(MigrationRuntimeError::VerificationMustRunRegistryDoctor);
    }
    for action in &record.changes {
        action.validate()?;
    }
    for action in &record.rollback {
        action.validate()?;
    }
    if record.rollback.len() != record.changes.len() {
        return Err(MigrationRuntimeError::RollbackMismatch {
            index: record.rollback.len(),
        });
    }
    for (index, (change, rollback)) in record
        .changes
        .iter()
        .zip(record.rollback.iter())
        .enumerate()
    {
        if &change.inverse() != rollback {
            return Err(MigrationRuntimeError::RollbackMismatch { index });
        }
    }
    Ok(())
}

pub fn build_migration_plan(
    record: &MigrationRecord,
    request: &MigrationRuntimeRequest,
) -> Result<MigrationRuntimePlan, MigrationRuntimeError> {
    validate_migration_record_depth(record)?;
    request
        .validate_mode()
        .map_err(MigrationRuntimeError::InvalidAction)?;
    if request.migration_id != record.id {
        return Err(MigrationRuntimeError::InvalidId);
    }
    let mode = if request.dry_run {
        MigrationRuntimeMode::DryRun
    } else {
        MigrationRuntimeMode::Apply
    };
    Ok(MigrationRuntimePlan {
        migration_id: record.id.clone(),
        mode,
        actions: record.changes.clone(),
        verification_command: record.verification_command.clone(),
        requires_registry_doctor: true,
    })
}

pub fn build_rollback_plan(
    record: &MigrationRecord,
) -> Result<MigrationRuntimePlan, MigrationRuntimeError> {
    validate_migration_record_depth(record)?;
    Ok(MigrationRuntimePlan {
        migration_id: format!("{}.rollback", record.id),
        mode: MigrationRuntimeMode::Rollback,
        actions: record.rollback.clone(),
        verification_command: record.verification_command.clone(),
        requires_registry_doctor: true,
    })
}

pub fn preview_migration(
    record: &MigrationRecord,
) -> Result<MigrationDryRunReport, MigrationRuntimeError> {
    validate_migration_record_depth(record)?;
    Ok(MigrationDryRunReport {
        migration_id: record.id.clone(),
        from_version: record.from_version,
        to_version: record.to_version,
        changes: record
            .changes
            .iter()
            .map(|action| MigrationPreviewChange {
                target: action.target.clone(),
                action: action.action,
                field: action.field.clone(),
                reversible: record.rollback.contains(&action.inverse()),
            })
            .collect(),
        verification_command_id: record.verification_command.id.clone(),
    })
}

pub fn apply_migration_action_to_yaml_text(
    source: &str,
    action: &MigrationAction,
) -> Result<String, MigrationRuntimeError> {
    action.validate()?;
    match action.action {
        MigrationActionKind::ChangeKind
        | MigrationActionKind::ChangeId
        | MigrationActionKind::ChangeType
        | MigrationActionKind::VersionBump => replace_yaml_scalar(
            source,
            &action.field,
            action.from.as_deref(),
            action.to.as_deref(),
            action,
        ),
        MigrationActionKind::RenameField => rename_yaml_field(source, action),
        MigrationActionKind::AddField => add_yaml_field(source, action),
        MigrationActionKind::RemoveField => remove_yaml_field(source, action),
    }
}

pub fn compatibility_kind_migration(kind: &str) -> Option<&'static str> {
    match kind {
        "product" | "status" | "donor_inventory" => Some("registry_status"),
        _ => None,
    }
}

fn replace_yaml_scalar(
    source: &str,
    field: &str,
    from: Option<&str>,
    to: Option<&str>,
    action: &MigrationAction,
) -> Result<String, MigrationRuntimeError> {
    let Some(to) = to else {
        return Err(MigrationRuntimeError::InvalidAction(
            "missing replacement value".to_string(),
        ));
    };
    let mut changed = false;
    let out = source
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix(&format!("{field}:")) {
                let current = rest.trim();
                if from.is_none_or(|expected| expected == current) {
                    changed = true;
                    let indent = &line[..line.len() - trimmed.len()];
                    return format!("{indent}{field}: {to}");
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !changed {
        return Err(MigrationRuntimeError::TextPatternMissing {
            target: action.target.clone(),
            field: field.to_string(),
        });
    }
    Ok(format_with_final_newline(&out, source.ends_with('\n')))
}

fn rename_yaml_field(
    source: &str,
    action: &MigrationAction,
) -> Result<String, MigrationRuntimeError> {
    let Some(to) = action.to.as_deref() else {
        return Err(MigrationRuntimeError::InvalidAction(
            "missing destination field".to_string(),
        ));
    };
    let mut changed = false;
    let out = source
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&format!("{}:", action.field)) {
                changed = true;
                let indent = &line[..line.len() - trimmed.len()];
                return format!("{indent}{to}:{}", &trimmed[action.field.len() + 1..]);
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !changed {
        return Err(MigrationRuntimeError::TextPatternMissing {
            target: action.target.clone(),
            field: action.field.clone(),
        });
    }
    Ok(format_with_final_newline(&out, source.ends_with('\n')))
}

fn add_yaml_field(source: &str, action: &MigrationAction) -> Result<String, MigrationRuntimeError> {
    if source
        .lines()
        .any(|line| line.trim_start().starts_with(&format!("{}:", action.field)))
    {
        return Ok(source.to_string());
    }
    let Some(to) = action.to.as_deref() else {
        return Err(MigrationRuntimeError::InvalidAction(
            "missing added value".to_string(),
        ));
    };
    let mut out = source.trim_end_matches('\n').to_string();
    out.push('\n');
    out.push_str(&format!("{}: {}\n", action.field, to));
    Ok(out)
}

fn remove_yaml_field(
    source: &str,
    action: &MigrationAction,
) -> Result<String, MigrationRuntimeError> {
    let mut changed = false;
    let out = source
        .lines()
        .filter(|line| {
            let remove = line.trim_start().starts_with(&format!("{}:", action.field));
            if remove {
                changed = true;
            }
            !remove
        })
        .map(str::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    if !changed {
        return Err(MigrationRuntimeError::TextPatternMissing {
            target: action.target.clone(),
            field: action.field.clone(),
        });
    }
    Ok(format_with_final_newline(&out, source.ends_with('\n')))
}

fn format_with_final_newline(value: &str, final_newline: bool) -> String {
    if final_newline && !value.ends_with('\n') {
        format!("{value}\n")
    } else {
        value.to_string()
    }
}

fn is_dotted_id(value: &str) -> bool {
    value.contains('.')
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_record() -> MigrationRecord {
        MigrationRecord {
            id: "migration.v1-hardening-h".to_string(),
            from_version: 1,
            to_version: 2,
            changes: vec![MigrationAction {
                action: MigrationActionKind::ChangeKind,
                target: ".vac/registry/status.yaml".to_string(),
                field: "kind".to_string(),
                from: Some("status".to_string()),
                to: Some("registry_status".to_string()),
            }],
            rollback: vec![MigrationAction {
                action: MigrationActionKind::ChangeKind,
                target: ".vac/registry/status.yaml".to_string(),
                field: "kind".to_string(),
                from: Some("registry_status".to_string()),
                to: Some("status".to_string()),
            }],
            verification_command: StructuredMigrationCommand {
                id: "vac.doctor.registry".to_string(),
                runner: "vac".to_string(),
                args: vec![
                    "doctor".to_string(),
                    "registry".to_string(),
                    ".".to_string(),
                ],
            },
        }
    }

    #[test]
    fn migration_record_requires_rollback_and_structured_verification() {
        assert!(valid_record().validate().is_ok());

        let mut no_rollback = valid_record();
        no_rollback.rollback.clear();
        assert!(no_rollback.validate().is_err());

        let mut freeform = valid_record();
        freeform.verification_command.runner = "./target/debug/vac".to_string();
        assert!(freeform.validate().is_err());
    }

    #[test]
    fn migration_runtime_mode_is_exactly_one_of_dry_run_or_apply() {
        let ok = MigrationRuntimeRequest {
            dry_run: true,
            apply: false,
            migration_id: "migration.v1-hardening-h".to_string(),
        };
        assert!(ok.validate_mode().is_ok());

        let invalid = MigrationRuntimeRequest {
            dry_run: true,
            apply: true,
            migration_id: "migration.v1-hardening-h".to_string(),
        };
        assert!(invalid.validate_mode().is_err());
    }

    #[test]
    fn compatibility_kind_cleanup_targets_registry_status() {
        assert_eq!(
            compatibility_kind_migration("product"),
            Some("registry_status")
        );
        assert_eq!(
            compatibility_kind_migration("status"),
            Some("registry_status")
        );
        assert_eq!(
            compatibility_kind_migration("donor_inventory"),
            Some("registry_status")
        );
        assert_eq!(compatibility_kind_migration("capability"), None);
    }

    #[test]
    fn builds_dry_run_and_rollback_plans_with_registry_doctor() {
        let record = valid_record();
        let request = MigrationRuntimeRequest {
            dry_run: true,
            apply: false,
            migration_id: record.id.clone(),
        };
        let plan = build_migration_plan(&record, &request).unwrap();
        assert_eq!(plan.mode, MigrationRuntimeMode::DryRun);
        assert!(plan.requires_registry_doctor);
        assert_eq!(plan.verification_command.id, "vac.doctor.registry");

        let rollback = build_rollback_plan(&record).unwrap();
        assert_eq!(rollback.mode, MigrationRuntimeMode::Rollback);
        assert_eq!(rollback.actions, record.rollback);
    }

    #[test]
    fn preview_marks_actions_reversible() {
        let report = preview_migration(&valid_record()).unwrap();
        assert_eq!(report.from_version, 1);
        assert_eq!(report.to_version, 2);
        assert!(report.changes.iter().all(|change| change.reversible));
    }

    #[test]
    fn applies_yaml_kind_change_and_rollback() {
        let record = valid_record();
        let source = "schema_version: 1\nkind: status\nid: vac.status\n";
        let migrated = apply_migration_action_to_yaml_text(source, &record.changes[0]).unwrap();
        assert!(migrated.contains("kind: registry_status"));
        let rolled_back =
            apply_migration_action_to_yaml_text(&migrated, &record.rollback[0]).unwrap();
        assert_eq!(rolled_back, source);
    }

    #[test]
    fn rejects_non_inverse_rollback() {
        let mut record = valid_record();
        record.rollback[0].to = Some("wrong".to_string());
        assert!(matches!(
            validate_migration_record_depth(&record),
            Err(MigrationRuntimeError::RollbackMismatch { index: 0 })
        ));
    }
}
