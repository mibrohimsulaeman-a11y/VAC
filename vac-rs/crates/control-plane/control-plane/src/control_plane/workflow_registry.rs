use super::registry::LocatedManifest;
use super::registry::ManifestRegistry;
use super::registry::RegistryLoadError;
use super::registry::load_manifest_registry;
use super::workflow_manifest::WorkflowManifest;
use super::workflow_manifest::WorkflowManifestError;
use super::workflow_manifest::load_workflow_manifest;
use super::workflow_manifest::validate_workflow_manifest_against_known_capabilities;
use std::collections::HashSet;
use std::path::Path;

pub type WorkflowRegistry = ManifestRegistry<WorkflowManifest>;
pub type WorkflowEntry = LocatedManifest<WorkflowManifest>;

pub fn load_workflow_registry(
    start: impl AsRef<Path>,
) -> Result<WorkflowRegistry, RegistryLoadError> {
    load_manifest_registry(
        start,
        "workflows",
        |path| load_workflow_manifest(path).map_err(workflow_error_to_registry_error),
        |manifest| manifest.id.as_str(),
    )
}

fn workflow_error_to_registry_error(error: WorkflowManifestError) -> RegistryLoadError {
    RegistryLoadError::Manifest {
        path: error.path().to_path_buf(),
        field_path: error.field_path().to_string(),
        message: error.message().to_string(),
    }
}

#[cfg(test)]
#[path = "workflow_registry_tests.rs"]
mod tests;

pub fn load_workflow_registry_with_known_capabilities(
    start: impl AsRef<Path>,
    known_capabilities: &HashSet<String>,
) -> Result<WorkflowRegistry, RegistryLoadError> {
    let registry = load_workflow_registry(start)?;
    let mut errors = Vec::new();
    for entry in &registry.manifests {
        if let Err(error) = validate_workflow_manifest_against_known_capabilities(
            &entry.path,
            &entry.manifest,
            known_capabilities,
        ) {
            errors.push(workflow_error_to_registry_error(error));
        }
    }

    if errors.is_empty() {
        Ok(registry)
    } else if errors.len() == 1 {
        Err(errors.remove(0))
    } else {
        Err(RegistryLoadError::Aggregate {
            path: registry.manifest_dir,
            errors,
        })
    }
}
