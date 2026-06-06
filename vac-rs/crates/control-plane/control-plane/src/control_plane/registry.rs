use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

use super::capability_registry::CapabilityRegistry;
use super::capability_registry::load_capability_registry;
use super::policy_registry::PolicyRegistry;
use super::policy_registry::load_policy_registry;
use super::registry_diagnostics::RegistryLoadReport;
use super::surface_registry::SurfaceRegistry;
use super::surface_registry::load_surface_registry;
use super::surface_registry::load_surface_registry_with_known_capabilities;
use super::workflow_manifest::workflow_step_use_resolves;
use super::workflow_registry::WorkflowRegistry;
use super::workflow_registry::load_workflow_registry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedManifest<T> {
    pub path: PathBuf,
    pub manifest: T,
}

impl<T> LocatedManifest<T> {
    pub fn new(path: impl Into<PathBuf>, manifest: T) -> Self {
        Self {
            path: path.into(),
            manifest,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestRegistry<T> {
    pub vac_root: PathBuf,
    pub manifest_dir: PathBuf,
    pub manifests: Vec<LocatedManifest<T>>,
}

impl<T> ManifestRegistry<T> {
    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }

    pub fn len(&self) -> usize {
        self.manifests.len()
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RegistryLoadError {
    #[error("{path}: unable to locate `.vac` root")]
    VacRootNotFound { path: PathBuf },
    #[error("{path}: {message}")]
    Directory { path: PathBuf, message: String },
    #[error("{path}: duplicate manifest id `{id}` (previously loaded from `{previous_path}`)")]
    DuplicateManifestId {
        path: PathBuf,
        id: String,
        previous_path: PathBuf,
    },
    #[error("{path}: multiple registry load errors")]
    Aggregate {
        path: PathBuf,
        errors: Vec<RegistryLoadError>,
    },
    #[error("{path}:{field_path}: {message}")]
    Manifest {
        path: PathBuf,
        field_path: String,
        message: String,
    },
}

impl RegistryLoadError {
    pub fn path(&self) -> &Path {
        match self {
            Self::VacRootNotFound { path }
            | Self::Directory { path, .. }
            | Self::DuplicateManifestId { path, .. }
            | Self::Aggregate { path, .. }
            | Self::Manifest { path, .. } => path,
        }
    }

    pub fn field_path(&self) -> Option<&str> {
        match self {
            Self::Manifest { field_path, .. } => Some(field_path.as_str()),
            _ => None,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::VacRootNotFound { .. } => "unable to locate `.vac` root",
            Self::Directory { message, .. } => message.as_str(),
            Self::DuplicateManifestId { .. } => "duplicate manifest id",
            Self::Aggregate { .. } => "multiple registry load errors",
            Self::Manifest { message, .. } => message.as_str(),
        }
    }

    pub fn aggregate_errors(&self) -> Option<&[RegistryLoadError]> {
        match self {
            Self::Aggregate { errors, .. } => Some(errors.as_slice()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlPlaneRegistry {
    pub vac_root: PathBuf,
    pub capabilities: CapabilityRegistry,
    pub workflows: WorkflowRegistry,
    pub policies: PolicyRegistry,
    pub surfaces: SurfaceRegistry,
}

impl ControlPlaneRegistry {
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
            && self.workflows.is_empty()
            && self.policies.is_empty()
            && self.surfaces.is_empty()
    }

    pub fn manifest_count(&self) -> usize {
        self.capabilities.len() + self.workflows.len() + self.policies.len() + self.surfaces.len()
    }
}

pub(crate) fn load_manifest_registry<T, F, G>(
    start: impl AsRef<Path>,
    subdir: &str,
    load_one: F,
    id_of: G,
) -> Result<ManifestRegistry<T>, RegistryLoadError>
where
    F: FnMut(&Path) -> Result<T, RegistryLoadError>,
    G: Fn(&T) -> &str,
{
    let vac_root = find_vac_root(start)?;
    load_manifest_registry_at_root(vac_root, subdir, load_one, id_of)
}

pub(crate) fn load_manifest_registry_at_root<T, F, G>(
    vac_root: impl AsRef<Path>,
    subdir: &str,
    mut load_one: F,
    id_of: G,
) -> Result<ManifestRegistry<T>, RegistryLoadError>
where
    F: FnMut(&Path) -> Result<T, RegistryLoadError>,
    G: Fn(&T) -> &str,
{
    let vac_root = vac_root.as_ref().to_path_buf();
    let manifest_dir = vac_root.join(subdir);
    let manifest_paths = collect_manifest_paths(&manifest_dir)?;
    let mut manifests = Vec::with_capacity(manifest_paths.len());
    let mut errors = Vec::new();
    for path in manifest_paths {
        match load_one(&path) {
            Ok(manifest) => manifests.push(LocatedManifest::new(path, manifest)),
            Err(error) => push_flattened_registry_error(&mut errors, error),
        }
    }
    manifests.sort_by(|left, right| left.path.cmp(&right.path));
    errors.extend(validate_unique_ids(&manifests, id_of));
    if !errors.is_empty() {
        return Err(aggregate_registry_errors(manifest_dir, errors));
    }
    Ok(ManifestRegistry {
        vac_root,
        manifest_dir,
        manifests,
    })
}

pub(crate) fn find_vac_root(start: impl AsRef<Path>) -> Result<PathBuf, RegistryLoadError> {
    let start = start.as_ref();
    for candidate in start.ancestors() {
        if candidate.file_name() == Some(OsStr::new(".vac")) && candidate.is_dir() {
            return Ok(candidate.to_path_buf());
        }
        let vac_root = candidate.join(".vac");
        if vac_root.is_dir() {
            return Ok(vac_root);
        }
    }

    Err(RegistryLoadError::VacRootNotFound {
        path: start.to_path_buf(),
    })
}

pub fn load_control_plane_registry(
    start: impl AsRef<Path>,
) -> Result<ControlPlaneRegistry, RegistryLoadError> {
    let vac_root = find_vac_root(start)?;
    load_control_plane_registry_at_root(vac_root)
}

pub(crate) fn load_control_plane_registry_at_root(
    vac_root: impl AsRef<Path>,
) -> Result<ControlPlaneRegistry, RegistryLoadError> {
    let vac_root = vac_root.as_ref().to_path_buf();
    let mut errors = Vec::new();

    let capabilities = match load_capability_registry(&vac_root) {
        Ok(registry) => Some(registry),
        Err(error) => {
            push_flattened_registry_error(&mut errors, error);
            None
        }
    };

    let known_capabilities = capabilities
        .as_ref()
        .map(|capabilities| {
            capabilities
                .manifests
                .iter()
                .map(|entry| entry.manifest.id.clone())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let workflows = match load_workflow_registry(&vac_root) {
        Ok(registry) => Some(registry),
        Err(error) => {
            push_flattened_registry_error(&mut errors, error);
            None
        }
    };

    let policies = match load_policy_registry(&vac_root) {
        Ok(registry) => Some(registry),
        Err(error) => {
            push_flattened_registry_error(&mut errors, error);
            None
        }
    };

    let surfaces = match if capabilities.is_some() {
        load_surface_registry_with_known_capabilities(&vac_root, &known_capabilities)
    } else {
        load_surface_registry(&vac_root)
    } {
        Ok(registry) => Some(registry),
        Err(error) => {
            push_flattened_registry_error(&mut errors, error);
            None
        }
    };

    let (capabilities, workflows, policies, surfaces) =
        match (capabilities, workflows, policies, surfaces) {
            (Some(capabilities), Some(workflows), Some(policies), Some(surfaces)) => {
                (capabilities, workflows, policies, surfaces)
            }
            _ => return Err(aggregate_registry_errors(vac_root, errors)),
        };

    errors.extend(validate_cross_family_unique_ids(
        &capabilities,
        &workflows,
        &policies,
        &surfaces,
    ));
    errors.extend(validate_workflow_step_uses(&workflows, &known_capabilities));
    if !errors.is_empty() {
        return Err(aggregate_registry_errors(vac_root, errors));
    }

    Ok(ControlPlaneRegistry {
        vac_root,
        capabilities,
        workflows,
        policies,
        surfaces,
    })
}

pub fn load_control_plane_registry_report(start: impl AsRef<Path>) -> RegistryLoadReport {
    match find_vac_root(start) {
        Ok(vac_root) => match load_control_plane_registry_at_root(&vac_root) {
            Ok(registry) => RegistryLoadReport::success(registry),
            Err(error) => RegistryLoadReport::from_error_at_root(Some(vac_root), error),
        },
        Err(error) => RegistryLoadReport::from_error(error),
    }
}

fn collect_manifest_paths(dir: &Path) -> Result<Vec<PathBuf>, RegistryLoadError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    if !dir.is_dir() {
        return Err(RegistryLoadError::Directory {
            path: dir.to_path_buf(),
            message: "expected a directory".to_string(),
        });
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(|error| RegistryLoadError::Directory {
        path: dir.to_path_buf(),
        message: error.to_string(),
    })? {
        let entry = entry.map_err(|error| RegistryLoadError::Directory {
            path: dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let extension = path.extension().and_then(OsStr::to_str);
        if matches!(extension, Some("yaml" | "yml")) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn validate_unique_ids<T, G>(manifests: &[LocatedManifest<T>], id_of: G) -> Vec<RegistryLoadError>
where
    G: Fn(&T) -> &str,
{
    let mut seen: HashMap<String, PathBuf> = HashMap::new();
    let mut errors = Vec::new();
    for entry in manifests {
        let id = id_of(&entry.manifest).to_string();
        if let Some(previous_path) = seen.get(&id) {
            errors.push(RegistryLoadError::DuplicateManifestId {
                path: entry.path.clone(),
                id,
                previous_path: previous_path.clone(),
            });
        } else {
            seen.insert(id, entry.path.clone());
        }
    }
    errors
}

fn validate_cross_family_unique_ids(
    capabilities: &CapabilityRegistry,
    workflows: &WorkflowRegistry,
    policies: &PolicyRegistry,
    surfaces: &SurfaceRegistry,
) -> Vec<RegistryLoadError> {
    let mut seen: HashMap<String, PathBuf> = HashMap::new();
    let mut errors = Vec::new();
    for entry in &capabilities.manifests {
        insert_cross_family_id(&mut seen, &mut errors, &entry.manifest.id, &entry.path);
    }
    for entry in &workflows.manifests {
        insert_cross_family_id(&mut seen, &mut errors, &entry.manifest.id, &entry.path);
    }
    for entry in &policies.manifests {
        insert_cross_family_id(&mut seen, &mut errors, &entry.manifest.id, &entry.path);
    }
    for entry in &surfaces.manifests {
        insert_cross_family_id(&mut seen, &mut errors, &entry.manifest.id, &entry.path);
    }
    errors
}

fn insert_cross_family_id(
    seen: &mut HashMap<String, PathBuf>,
    errors: &mut Vec<RegistryLoadError>,
    id: &str,
    path: &Path,
) {
    if let Some(previous_path) = seen.get(id) {
        errors.push(RegistryLoadError::DuplicateManifestId {
            path: path.to_path_buf(),
            id: id.to_string(),
            previous_path: previous_path.clone(),
        });
    } else {
        seen.insert(id.to_string(), path.to_path_buf());
    }
}

fn validate_workflow_step_uses(
    workflows: &WorkflowRegistry,
    known_capabilities: &HashSet<String>,
) -> Vec<RegistryLoadError> {
    let mut errors = Vec::new();
    for workflow in &workflows.manifests {
        for (index, step) in workflow.manifest.steps.iter().enumerate() {
            if workflow_step_use_resolves(&step.uses, known_capabilities) {
                continue;
            }
            errors.push(RegistryLoadError::Manifest {
                path: workflow.path.clone(),
                field_path: format!("steps[{index}].uses"),
                message:
                    "step uses must resolve to workflow vocabulary or a declared capability id"
                        .to_string(),
            });
        }
    }
    errors
}

fn push_flattened_registry_error(errors: &mut Vec<RegistryLoadError>, error: RegistryLoadError) {
    match error {
        RegistryLoadError::Aggregate { errors: nested, .. } => {
            for error in nested {
                push_flattened_registry_error(errors, error);
            }
        }
        error => errors.push(error),
    }
}

fn aggregate_registry_errors(
    path: impl Into<PathBuf>,
    mut errors: Vec<RegistryLoadError>,
) -> RegistryLoadError {
    if errors.len() == 1 {
        errors.remove(0)
    } else {
        RegistryLoadError::Aggregate {
            path: path.into(),
            errors,
        }
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
