use super::capability_registry::CapabilityRegistry;
use super::registry::LocatedManifest;
use super::registry::ManifestRegistry;
use super::registry::RegistryLoadError;
use super::registry::load_manifest_registry;
use super::surface_manifest::SurfaceManifest;
use super::surface_manifest::SurfaceManifestError;
use super::surface_manifest::SurfaceRouteKind;
use super::surface_manifest::load_surface_manifest;
use super::surface_manifest::surface_route_kind_label;
use super::surface_manifest::surface_route_target;
use super::surface_manifest::validate_surface_manifest_against_known_capabilities;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

pub type SurfaceRegistry = ManifestRegistry<SurfaceManifest>;
pub type SurfaceEntry = LocatedManifest<SurfaceManifest>;

pub fn load_surface_registry(
    start: impl AsRef<Path>,
) -> Result<SurfaceRegistry, RegistryLoadError> {
    load_manifest_registry(
        start,
        "surfaces",
        |path| load_surface_manifest(path).map_err(surface_error_to_registry_error),
        |manifest| manifest.id.as_str(),
    )
}

pub fn load_surface_registry_with_known_capabilities(
    start: impl AsRef<Path>,
    known_capabilities: &HashSet<String>,
) -> Result<SurfaceRegistry, RegistryLoadError> {
    load_manifest_registry(
        start,
        "surfaces",
        |path| {
            let manifest = load_surface_manifest(path).map_err(surface_error_to_registry_error)?;
            validate_surface_manifest_against_known_capabilities(
                path,
                &manifest,
                known_capabilities,
            )
            .map_err(surface_error_to_registry_error)?;
            Ok(manifest)
        },
        |manifest| manifest.id.as_str(),
    )
}

fn surface_error_to_registry_error(error: SurfaceManifestError) -> RegistryLoadError {
    RegistryLoadError::Manifest {
        path: error.path().to_path_buf(),
        field_path: error.field_path().to_string(),
        message: error.message().to_string(),
    }
}

// =====================================================================
// Plan 05 — Cross-surface schema hardening.
//
// `SurfaceRouteCatalog` aggregates every loaded surface route by
// (kind, canonical-target). We intentionally collect *all* entries per
// key (no `Entry::or_insert_with` masking) so cross-surface duplicates
// are surfaced explicitly via `validate_surface_route_catalog_duplicates`.
//
// `SurfaceCrossDiagnostic` reports four classes of drift:
//   * DuplicateRoute     — same (kind,target) declared by >1 surface manifest
//   * OwnerConflict      — same capability has different owners across surfaces
//   * PaletteDrift       — capability `surfaces.palette` flag does not match
//                          presence of palette routes in any surface manifest
//   * SurfaceRouteDrift  — capability declares tui/slash/cli routes that do
//                          not exactly match what surfaces expose for it
//
// Duplicate routes remain blocking because they make dispatch ambiguous.
// Owner and route drift are operator-facing audit signals; shared aggregate
// routes such as `/capabilities` and CLI doctor aliases intentionally create
// non-exact manifest projections.
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRouteCatalogEntry {
    pub surface_id: String,
    pub surface_path: PathBuf,
    pub kind: SurfaceRouteKind,
    pub target: String,
    pub capability: String,
    pub owner: Option<String>,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceRouteCatalog {
    entries: BTreeMap<(SurfaceRouteKind, String), Vec<SurfaceRouteCatalogEntry>>,
}

impl SurfaceRouteCatalog {
    pub fn entries(&self) -> &BTreeMap<(SurfaceRouteKind, String), Vec<SurfaceRouteCatalogEntry>> {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.values().map(|values| values.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn unique_keys(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceCrossSeverity {
    Warning,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceCrossDiagnostic {
    DuplicateRoute {
        kind: SurfaceRouteKind,
        target: String,
        surfaces: Vec<String>,
    },
    OwnerConflict {
        capability: String,
        owners: Vec<(String, String)>,
    },
    PaletteDrift {
        capability: String,
        declared_palette: bool,
        palette_route_surfaces: Vec<String>,
    },
    SurfaceRouteDrift {
        capability: String,
        kind: SurfaceRouteKind,
        declared: Vec<String>,
        present: Vec<String>,
    },
}

impl SurfaceCrossDiagnostic {
    pub fn severity(&self) -> SurfaceCrossSeverity {
        match self {
            Self::DuplicateRoute { .. } => SurfaceCrossSeverity::Failure,
            Self::OwnerConflict { .. }
            | Self::PaletteDrift { .. }
            | Self::SurfaceRouteDrift { .. } => SurfaceCrossSeverity::Warning,
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::DuplicateRoute {
                kind,
                target,
                surfaces,
            } => format!(
                "duplicate {} route `{}` declared by surfaces: {}",
                surface_route_kind_label(*kind),
                target,
                surfaces.join(", ")
            ),
            Self::OwnerConflict { capability, owners } => {
                let pairs: Vec<String> = owners.iter().map(|(s, o)| format!("{s}={o}")).collect();
                format!(
                    "capability `{capability}` has inconsistent owners across surfaces: {}",
                    pairs.join(", ")
                )
            }
            Self::PaletteDrift {
                capability,
                declared_palette,
                palette_route_surfaces,
            } => {
                if *declared_palette {
                    format!(
                        "capability `{capability}` declares palette=true but no surface exposes a palette route"
                    )
                } else {
                    format!(
                        "capability `{capability}` declares palette=false but palette route present in: {}",
                        palette_route_surfaces.join(", ")
                    )
                }
            }
            Self::SurfaceRouteDrift {
                capability,
                kind,
                declared,
                present,
            } => format!(
                "capability `{capability}` {} routes drift: declared=[{}] present=[{}]",
                surface_route_kind_label(*kind),
                declared.join(", "),
                present.join(", ")
            ),
        }
    }
}

pub fn build_surface_route_catalog(registry: &SurfaceRegistry) -> SurfaceRouteCatalog {
    let mut catalog = SurfaceRouteCatalog::default();
    for entry in &registry.manifests {
        for route in &entry.manifest.routes {
            let Some(target) = surface_route_target(route) else {
                continue;
            };
            let key = (route.kind, target.to_string());
            // Do not collapse duplicates via `Entry::or_insert_with` — we
            // accumulate every entry so duplicates can be reported.
            catalog
                .entries
                .entry(key)
                .or_default()
                .push(SurfaceRouteCatalogEntry {
                    surface_id: entry.manifest.id.clone(),
                    surface_path: entry.path.clone(),
                    kind: route.kind,
                    target: target.to_string(),
                    capability: route.capability.clone(),
                    owner: route.owner.clone(),
                    visible: route.visible,
                });
        }
    }
    catalog
}

pub fn validate_surface_route_catalog_duplicates(
    catalog: &SurfaceRouteCatalog,
) -> Vec<SurfaceCrossDiagnostic> {
    let mut out = Vec::new();
    for ((kind, target), entries) in catalog.entries() {
        if entries.len() <= 1 {
            continue;
        }
        let mut surfaces: Vec<String> = entries.iter().map(|e| e.surface_id.clone()).collect();
        surfaces.sort();
        surfaces.dedup();
        out.push(SurfaceCrossDiagnostic::DuplicateRoute {
            kind: *kind,
            target: target.clone(),
            surfaces,
        });
    }
    out
}

pub fn validate_surface_owner_consistency(
    registry: &SurfaceRegistry,
) -> Vec<SurfaceCrossDiagnostic> {
    let mut per_cap: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for entry in &registry.manifests {
        for route in &entry.manifest.routes {
            let Some(owner) = route.owner.as_deref() else {
                continue;
            };
            per_cap
                .entry(route.capability.clone())
                .or_default()
                .entry(owner.to_string())
                .or_default()
                .insert(entry.manifest.id.clone());
        }
    }
    let mut out = Vec::new();
    for (capability, owners_map) in per_cap {
        if owners_map.len() <= 1 {
            continue;
        }
        let mut owners: Vec<(String, String)> = owners_map
            .into_iter()
            .flat_map(|(owner, surfaces)| {
                surfaces
                    .into_iter()
                    .map(move |surface| (surface, owner.clone()))
            })
            .collect();
        owners.sort();
        out.push(SurfaceCrossDiagnostic::OwnerConflict { capability, owners });
    }
    out
}

pub fn validate_surface_reverse_drift(
    surfaces: &SurfaceRegistry,
    capabilities: &CapabilityRegistry,
) -> Vec<SurfaceCrossDiagnostic> {
    let mut palette_targets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut palette_surfaces: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut tui_routes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut slash_routes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut cli_routes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for entry in &surfaces.manifests {
        for route in &entry.manifest.routes {
            let target = surface_route_target(route).unwrap_or("").to_string();
            match route.kind {
                SurfaceRouteKind::Palette => {
                    palette_targets
                        .entry(route.capability.clone())
                        .or_default()
                        .insert(target);
                    palette_surfaces
                        .entry(route.capability.clone())
                        .or_default()
                        .insert(entry.manifest.id.clone());
                }
                SurfaceRouteKind::Tui => {
                    tui_routes
                        .entry(route.capability.clone())
                        .or_default()
                        .insert(target);
                }
                SurfaceRouteKind::Slash => {
                    slash_routes
                        .entry(route.capability.clone())
                        .or_default()
                        .insert(target);
                }
                SurfaceRouteKind::Cli => {
                    cli_routes
                        .entry(route.capability.clone())
                        .or_default()
                        .insert(target);
                }
                SurfaceRouteKind::Statusline => {}
            }
        }
    }

    let mut out = Vec::new();
    for cap_entry in &capabilities.manifests {
        let cap = &cap_entry.manifest;
        let id = &cap.id;

        let declared_palette = cap.surfaces.palette;
        let surfaces_with_palette: Vec<String> = palette_surfaces
            .get(id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default();
        if declared_palette && surfaces_with_palette.is_empty() {
            out.push(SurfaceCrossDiagnostic::PaletteDrift {
                capability: id.clone(),
                declared_palette: true,
                palette_route_surfaces: Vec::new(),
            });
        } else if !declared_palette && !surfaces_with_palette.is_empty() {
            out.push(SurfaceCrossDiagnostic::PaletteDrift {
                capability: id.clone(),
                declared_palette: false,
                palette_route_surfaces: surfaces_with_palette,
            });
        }
        // Silence unused targets lint for palette_targets — used only for
        // future hardening; tracked via `palette_surfaces` here.
        let _ = &palette_targets;

        if let Some(tui_decl) = &cap.surfaces.tui {
            let declared: BTreeSet<String> = tui_decl.routes.iter().cloned().collect();
            let present = tui_routes.get(id).cloned().unwrap_or_default();
            if declared != present {
                out.push(SurfaceCrossDiagnostic::SurfaceRouteDrift {
                    capability: id.clone(),
                    kind: SurfaceRouteKind::Tui,
                    declared: declared.into_iter().collect(),
                    present: present.into_iter().collect(),
                });
            }
        }

        if let Some(slash_decl) = &cap.surfaces.slash {
            let declared: BTreeSet<String> = slash_decl.commands.iter().cloned().collect();
            let present = slash_routes.get(id).cloned().unwrap_or_default();
            if declared != present {
                out.push(SurfaceCrossDiagnostic::SurfaceRouteDrift {
                    capability: id.clone(),
                    kind: SurfaceRouteKind::Slash,
                    declared: declared.into_iter().collect(),
                    present: present.into_iter().collect(),
                });
            }
        }

        if let Some(cli_decl) = &cap.surfaces.cli {
            if cli_decl.enabled && !cli_decl.commands.is_empty() {
                let declared: BTreeSet<String> = cli_decl.commands.iter().cloned().collect();
                let present = cli_routes.get(id).cloned().unwrap_or_default();
                if declared != present {
                    out.push(SurfaceCrossDiagnostic::SurfaceRouteDrift {
                        capability: id.clone(),
                        kind: SurfaceRouteKind::Cli,
                        declared: declared.into_iter().collect(),
                        present: present.into_iter().collect(),
                    });
                }
            }
        }
    }
    out
}

pub fn validate_surface_cross(
    surfaces: &SurfaceRegistry,
    capabilities: Option<&CapabilityRegistry>,
) -> Vec<SurfaceCrossDiagnostic> {
    let catalog = build_surface_route_catalog(surfaces);
    let mut out = validate_surface_route_catalog_duplicates(&catalog);
    out.extend(validate_surface_owner_consistency(surfaces));
    if let Some(capabilities) = capabilities {
        out.extend(validate_surface_reverse_drift(surfaces, capabilities));
    }
    out
}

#[cfg(test)]
#[path = "surface_registry_tests.rs"]
mod tests;
