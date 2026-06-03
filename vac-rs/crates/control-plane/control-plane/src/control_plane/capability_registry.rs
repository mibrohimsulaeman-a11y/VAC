use super::capability_manifest::CapabilityManifest;
use super::capability_manifest::CapabilityManifestError;
use super::capability_manifest::load_capability_manifest;
use super::registry::LocatedManifest;
use super::registry::ManifestRegistry;
use super::registry::RegistryLoadError;
use super::registry::load_manifest_registry;
use std::path::Path;

pub type CapabilityRegistry = ManifestRegistry<CapabilityManifest>;
pub type CapabilityEntry = LocatedManifest<CapabilityManifest>;

pub fn load_capability_registry(
    start: impl AsRef<Path>,
) -> Result<CapabilityRegistry, RegistryLoadError> {
    load_manifest_registry(
        start,
        "capabilities",
        |path| load_capability_manifest(path).map_err(capability_error_to_registry_error),
        |manifest| manifest.id.as_str(),
    )
}

fn capability_error_to_registry_error(error: CapabilityManifestError) -> RegistryLoadError {
    RegistryLoadError::Manifest {
        path: error.path().to_path_buf(),
        field_path: error.field_path().to_string(),
        message: error.message().to_string(),
    }
}

#[cfg(test)]
#[path = "capability_registry_tests.rs"]
mod tests;
