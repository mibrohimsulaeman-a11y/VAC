use super::policy_manifest::PolicyManifest;
use super::policy_manifest::PolicyManifestError;
use super::policy_manifest::load_policy_manifest;
use super::registry::LocatedManifest;
use super::registry::ManifestRegistry;
use super::registry::RegistryLoadError;
use super::registry::load_manifest_registry;
use std::path::Path;

pub type PolicyRegistry = ManifestRegistry<PolicyManifest>;
pub type PolicyEntry = LocatedManifest<PolicyManifest>;

pub fn load_policy_registry(start: impl AsRef<Path>) -> Result<PolicyRegistry, RegistryLoadError> {
    load_manifest_registry(
        start,
        "policies",
        |path| load_policy_manifest(path).map_err(policy_error_to_registry_error),
        |manifest| manifest.id.as_str(),
    )
}

fn policy_error_to_registry_error(error: PolicyManifestError) -> RegistryLoadError {
    RegistryLoadError::Manifest {
        path: error.path().to_path_buf(),
        field_path: error.field_path().to_string(),
        message: error.message().to_string(),
    }
}

// ---- Phase 4: path-based policy doctor report ----

use super::policy_manifest::PolicyDoctorReport;
use super::policy_manifest::load_policy_doctor_report;

pub fn load_policy_doctor_report_for_path(start: impl AsRef<Path>) -> PolicyDoctorReport {
    match load_policy_registry(start) {
        Ok(registry) => {
            let manifests: Vec<_> = registry
                .manifests
                .iter()
                .map(|e| e.manifest.clone())
                .collect();
            load_policy_doctor_report(&manifests)
        }
        Err(error) => PolicyDoctorReport {
            manifest_count: 0,
            rule_count: 0,
            allow_rule_count: 0,
            deny_rule_count: 0,
            approval_rule_count: 0,
            manifests: Vec::new(),
            load_errors: vec![error.to_string()],
        },
    }
}
#[cfg(test)]
#[path = "policy_registry_tests.rs"]
mod tests;
