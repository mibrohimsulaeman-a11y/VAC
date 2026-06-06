use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceManifest {
    pub schema_version: u32,
    pub kind: SurfaceManifestKind,
    pub id: String,
    pub title: String,
    pub routes: Vec<SurfaceRoute>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SurfaceManifestKind {
    #[serde(rename = "surface")]
    Surface,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceRouteKind {
    Tui,
    Slash,
    Palette,
    Cli,
    Statusline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceRouteStatus {
    Ready,
    Partial,
    Planned,
    Unavailable,
    CliOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceRoute {
    pub kind: SurfaceRouteKind,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    pub capability: String,
    #[serde(default)]
    pub owner: Option<String>,
    pub visible: bool,
    pub status: SurfaceRouteStatus,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{path}:{field_path}: {message}")]
pub struct SurfaceManifestError {
    path: PathBuf,
    field_path: String,
    message: String,
}

impl SurfaceManifestError {
    pub fn new(
        path: impl Into<PathBuf>,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            field_path: field_path.into(),
            message: message.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn field_path(&self) -> &str {
        &self.field_path
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSurfaceManifest {
    schema_version: Option<u32>,
    kind: Option<String>,
    id: Option<String>,
    title: Option<String>,
    routes: Option<Vec<RawSurfaceRoute>>,
    capabilities: Option<Vec<String>>,
    #[serde(default)]
    vac_init_batch13_15: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSurfaceRoute {
    kind: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    label: Option<String>,
    capability: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    visible: Option<bool>,
    status: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

pub fn load_surface_manifest(
    path: impl AsRef<Path>,
) -> Result<SurfaceManifest, SurfaceManifestError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)
        .map_err(|err| SurfaceManifestError::new(path, "root", err.to_string()))?;
    parse_surface_manifest(path, &contents)
}

pub fn parse_surface_manifest(
    path: impl AsRef<Path>,
    contents: &str,
) -> Result<SurfaceManifest, SurfaceManifestError> {
    let path = path.as_ref();
    let deserializer = serde_yaml::Deserializer::from_str(contents);
    let raw: RawSurfaceManifest =
        serde_path_to_error::deserialize(deserializer).map_err(|error| {
            SurfaceManifestError::new(
                path,
                serde_path_to_string(error.path()),
                error.into_inner().to_string(),
            )
        })?;
    SurfaceManifest::from_raw(path, raw)
}

pub fn validate_surface_manifest(
    path: impl AsRef<Path>,
    manifest: &SurfaceManifest,
) -> Result<(), SurfaceManifestError> {
    validate_surface_manifest_internal(path.as_ref(), manifest, None)
}

pub fn validate_surface_manifest_against_known_capabilities(
    path: impl AsRef<Path>,
    manifest: &SurfaceManifest,
    known_capabilities: &HashSet<String>,
) -> Result<(), SurfaceManifestError> {
    validate_surface_manifest_internal(path.as_ref(), manifest, Some(known_capabilities))
}

impl SurfaceManifest {
    fn from_raw(path: &Path, raw: RawSurfaceManifest) -> Result<Self, SurfaceManifestError> {
        let schema_version = raw.schema_version.ok_or_else(|| {
            SurfaceManifestError::new(path, "schema_version", "missing required field")
        })?;
        if schema_version != 1 {
            return Err(SurfaceManifestError::new(
                path,
                "schema_version",
                format!("unsupported schema version {schema_version}; expected 1"),
            ));
        }

        let kind = match raw.kind.as_deref() {
            Some("surface") => SurfaceManifestKind::Surface,
            Some(other) => {
                return Err(SurfaceManifestError::new(
                    path,
                    "kind",
                    format!("unknown manifest kind `{other}`; expected `surface`"),
                ));
            }
            None => {
                return Err(SurfaceManifestError::new(
                    path,
                    "kind",
                    "missing required field",
                ));
            }
        };

        let id = normalize_id(raw.id, path)?;
        let title = normalize_non_empty_string(raw.title, path, "title")?;
        let routes = parse_routes(raw.routes, path)?;
        let capabilities = normalize_capabilities(raw.capabilities, path)?;
        let _readiness_metadata = raw.vac_init_batch13_15;

        let manifest = Self {
            schema_version,
            kind,
            id,
            title,
            routes,
            capabilities,
        };
        validate_surface_manifest_internal(path, &manifest, None)?;
        Ok(manifest)
    }
}

fn validate_surface_manifest_internal(
    path: &Path,
    manifest: &SurfaceManifest,
    known_capabilities: Option<&HashSet<String>>,
) -> Result<(), SurfaceManifestError> {
    validate_surface_schema_version(path, manifest.schema_version)?;
    validate_surface_id(path, &manifest.id)?;
    validate_surface_title(path, &manifest.title)?;
    validate_surface_capabilities(path, &manifest.capabilities)?;
    validate_surface_routes(
        path,
        &manifest.routes,
        &manifest.capabilities,
        known_capabilities,
    )?;
    for (idx, route) in manifest.routes.iter().enumerate() {
        let field = format!("routes[{idx}]");
        if matches!(route.status, SurfaceRouteStatus::Planned) && route.visible {
            return Err(SurfaceManifestError::new(
                path,
                format!("{field}.visible"),
                "surface.planned_visible: planned routes must not be visible",
            ));
        }
        if matches!(route.status, SurfaceRouteStatus::Partial)
            && route.reason.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(SurfaceManifestError::new(
                path,
                format!("{field}.reason"),
                "surface.partial_missing_guidance: partial routes require degraded guidance",
            ));
        }
    }

    Ok(())
}

fn validate_surface_schema_version(
    path: &Path,
    schema_version: u32,
) -> Result<(), SurfaceManifestError> {
    if schema_version != 1 {
        return Err(SurfaceManifestError::new(
            path,
            "schema_version",
            format!("unsupported schema version {schema_version}; expected 1"),
        ));
    }
    Ok(())
}

fn validate_surface_id(path: &Path, id: &str) -> Result<(), SurfaceManifestError> {
    validate_non_empty(path, "id", id)?;
    ensure_no_whitespace(path, "id", id)?;
    if !id.starts_with("surface.") {
        return Err(SurfaceManifestError::new(
            path,
            "id",
            "surface ids must start with `surface.`",
        ));
    }
    Ok(())
}

fn validate_surface_title(path: &Path, title: &str) -> Result<(), SurfaceManifestError> {
    validate_non_empty(path, "title", title)
}

fn validate_surface_capabilities(
    path: &Path,
    capabilities: &[String],
) -> Result<(), SurfaceManifestError> {
    if capabilities.is_empty() {
        return Err(SurfaceManifestError::new(
            path,
            "capabilities",
            "surface must declare at least one capability",
        ));
    }

    let mut seen = HashSet::new();
    for (index, capability) in capabilities.iter().enumerate() {
        validate_capability_id(path, &format!("capabilities[{index}]"), capability)?;
        if !seen.insert(capability.clone()) {
            return Err(SurfaceManifestError::new(
                path,
                format!("capabilities[{index}]"),
                "capability ids must be unique",
            ));
        }
    }

    Ok(())
}

fn validate_surface_routes(
    path: &Path,
    routes: &[SurfaceRoute],
    manifest_capabilities: &[String],
    known_capabilities: Option<&HashSet<String>>,
) -> Result<(), SurfaceManifestError> {
    if routes.is_empty() {
        return Err(SurfaceManifestError::new(
            path,
            "routes",
            "surface has no routes",
        ));
    }

    let manifest_capabilities: HashSet<_> = manifest_capabilities.iter().cloned().collect();
    let mut seen_routes = HashSet::new();
    let mut routed_capabilities = HashSet::new();

    for (index, route) in routes.iter().enumerate() {
        let field_path = format!("routes[{index}]");
        validate_route_kind(path, &field_path, route)?;
        validate_capability_id(path, &format!("{field_path}.capability"), &route.capability)?;
        routed_capabilities.insert(route.capability.clone());
        if !seen_routes.insert(route_key(route)) {
            return Err(SurfaceManifestError::new(
                path,
                field_path,
                "route ids must be unique",
            ));
        }

        if route.visible
            && route
                .owner
                .as_ref()
                .is_none_or(|owner| owner.trim().is_empty())
        {
            return Err(SurfaceManifestError::new(
                path,
                format!("{field_path}.owner"),
                "visible routes must declare a capability owner",
            ));
        }

        if matches!(
            route.status,
            SurfaceRouteStatus::Unavailable | SurfaceRouteStatus::CliOnly
        ) && route
            .reason
            .as_ref()
            .is_none_or(|reason| reason.trim().is_empty())
        {
            return Err(SurfaceManifestError::new(
                path,
                format!("{field_path}.reason"),
                "cli-only or unavailable routes must say why",
            ));
        }

        if route.visible
            && !matches!(
                route.status,
                SurfaceRouteStatus::Ready
                    | SurfaceRouteStatus::Partial
                    | SurfaceRouteStatus::Planned
            )
        {
            return Err(SurfaceManifestError::new(
                path,
                format!("{field_path}.status"),
                "visible routes must be ready, partial, or planned",
            ));
        }

        if route.visible && !manifest_capabilities.contains(&route.capability) {
            return Err(SurfaceManifestError::new(
                path,
                format!("{field_path}.capability"),
                "visible route capability must be listed in surface capabilities",
            ));
        }

        if let Some(known_capabilities) = known_capabilities
            && !known_capabilities.contains(&route.capability)
        {
            return Err(SurfaceManifestError::new(
                path,
                format!("{field_path}.capability"),
                "surface references an unknown capability",
            ));
        }
    }

    for capability in &manifest_capabilities {
        if !routed_capabilities.contains(capability) {
            return Err(SurfaceManifestError::new(
                path,
                "capabilities",
                "surface has a listed capability that is not declared in any route",
            ));
        }
    }

    Ok(())
}

#[allow(clippy::expect_used)] // option fields are validated present before unwrap per route kind
fn validate_route_kind(
    path: &Path,
    field_path: &str,
    route: &SurfaceRoute,
) -> Result<(), SurfaceManifestError> {
    let has_path = route.path.is_some();
    let has_command = route.command.is_some();
    let has_action = route.action.is_some();
    let has_label = route.label.is_some();

    match route.kind {
        SurfaceRouteKind::Tui => {
            if !has_path || has_command || has_action || has_label {
                return Err(SurfaceManifestError::new(
                    path,
                    field_path,
                    "tui routes require only `path`",
                ));
            }
            let path_value = route.path.as_ref().expect("validated path");
            validate_non_empty(path, &format!("{field_path}.path"), path_value)?;
            ensure_no_whitespace(path, &format!("{field_path}.path"), path_value)?;
            if !path_value.starts_with('/') {
                return Err(SurfaceManifestError::new(
                    path,
                    format!("{field_path}.path"),
                    "tui routes must start with `/`",
                ));
            }
        }
        SurfaceRouteKind::Slash => {
            if !has_command || has_path || has_action || has_label {
                return Err(SurfaceManifestError::new(
                    path,
                    field_path,
                    "slash routes require only `command`",
                ));
            }
            let command = route.command.as_ref().expect("validated command");
            validate_non_empty(path, &format!("{field_path}.command"), command)?;
            if !command.starts_with('/') {
                return Err(SurfaceManifestError::new(
                    path,
                    format!("{field_path}.command"),
                    "slash routes must start with `/`",
                ));
            }
        }
        SurfaceRouteKind::Palette => {
            if !has_action || has_path || has_command || has_label {
                return Err(SurfaceManifestError::new(
                    path,
                    field_path,
                    "palette routes require only `action`",
                ));
            }
            let action = route.action.as_ref().expect("validated action");
            validate_non_empty(path, &format!("{field_path}.action"), action)?;
            ensure_no_whitespace(path, &format!("{field_path}.action"), action)?;
        }
        SurfaceRouteKind::Cli => {
            if !has_command || has_path || has_action || has_label {
                return Err(SurfaceManifestError::new(
                    path,
                    field_path,
                    "cli routes require only `command`",
                ));
            }
            let command = route.command.as_ref().expect("validated command");
            validate_non_empty(path, &format!("{field_path}.command"), command)?;
        }
        SurfaceRouteKind::Statusline => {
            if !has_label || has_path || has_command || has_action {
                return Err(SurfaceManifestError::new(
                    path,
                    field_path,
                    "statusline routes require only `label`",
                ));
            }
            let label = route.label.as_ref().expect("validated label");
            validate_non_empty(path, &format!("{field_path}.label"), label)?;
        }
    }

    Ok(())
}

fn route_key(route: &SurfaceRoute) -> String {
    let target = route
        .path
        .as_deref()
        .or(route.command.as_deref())
        .or(route.action.as_deref())
        .or(route.label.as_deref())
        .unwrap_or_default();
    format!("{:?}:{target}", route.kind)
}

fn normalize_id(raw: Option<String>, path: &Path) -> Result<String, SurfaceManifestError> {
    let value = normalize_non_empty_string(raw, path, "id")?;
    if !value.starts_with("surface.") {
        return Err(SurfaceManifestError::new(
            path,
            "id",
            "surface ids must start with `surface.`",
        ));
    }
    Ok(value)
}

fn normalize_capabilities(
    raw: Option<Vec<String>>,
    path: &Path,
) -> Result<Vec<String>, SurfaceManifestError> {
    let values = raw
        .ok_or_else(|| SurfaceManifestError::new(path, "capabilities", "missing required field"))?;
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        normalized.push(normalize_non_empty_string(
            Some(value),
            path,
            &format!("capabilities[{index}]"),
        )?);
    }
    Ok(normalized)
}

fn validate_capability_id(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), SurfaceManifestError> {
    validate_non_empty(path, field_path, value)?;
    ensure_no_whitespace(path, field_path, value)?;
    if !value.starts_with("vac.") {
        return Err(SurfaceManifestError::new(
            path,
            field_path,
            "capability ids must start with `vac.`",
        ));
    }
    Ok(())
}

fn normalize_non_empty_string(
    raw: Option<String>,
    path: &Path,
    field_path: &str,
) -> Result<String, SurfaceManifestError> {
    let value =
        raw.ok_or_else(|| SurfaceManifestError::new(path, field_path, "missing required field"))?;
    if value.trim().is_empty() {
        return Err(SurfaceManifestError::new(
            path,
            field_path,
            "value must not be empty",
        ));
    }
    Ok(value)
}

fn validate_non_empty(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), SurfaceManifestError> {
    if value.trim().is_empty() {
        return Err(SurfaceManifestError::new(
            path,
            field_path,
            "value must not be empty",
        ));
    }
    Ok(())
}

fn ensure_no_whitespace(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), SurfaceManifestError> {
    if value.chars().any(char::is_whitespace) {
        return Err(SurfaceManifestError::new(
            path,
            field_path,
            "value must not contain whitespace",
        ));
    }
    Ok(())
}

fn parse_routes(
    raw: Option<Vec<RawSurfaceRoute>>,
    path: &Path,
) -> Result<Vec<SurfaceRoute>, SurfaceManifestError> {
    let raw =
        raw.ok_or_else(|| SurfaceManifestError::new(path, "routes", "missing required field"))?;
    let mut routes = Vec::with_capacity(raw.len());
    for (index, raw_route) in raw.into_iter().enumerate() {
        let field_path = format!("routes[{index}]");
        let kind = parse_route_kind(raw_route.kind, path, &format!("{field_path}.kind"))?;
        let visible = raw_route.visible.ok_or_else(|| {
            SurfaceManifestError::new(
                path,
                format!("{field_path}.visible"),
                "missing required field",
            )
        })?;
        let status = parse_route_status(raw_route.status, path, &format!("{field_path}.status"))?;
        routes.push(SurfaceRoute {
            kind,
            path: raw_route.path,
            command: raw_route.command,
            action: raw_route.action,
            label: raw_route.label,
            capability: normalize_non_empty_string(
                raw_route.capability,
                path,
                &format!("{field_path}.capability"),
            )?,
            owner: raw_route.owner.filter(|value| !value.trim().is_empty()),
            visible,
            status,
            reason: raw_route.reason.filter(|value| !value.trim().is_empty()),
        });
    }
    Ok(routes)
}

fn parse_route_kind(
    raw: Option<String>,
    path: &Path,
    field_path: &str,
) -> Result<SurfaceRouteKind, SurfaceManifestError> {
    let raw =
        raw.ok_or_else(|| SurfaceManifestError::new(path, field_path, "missing required field"))?;
    match raw.as_str() {
        "tui" => Ok(SurfaceRouteKind::Tui),
        "slash" => Ok(SurfaceRouteKind::Slash),
        "palette" => Ok(SurfaceRouteKind::Palette),
        "cli" => Ok(SurfaceRouteKind::Cli),
        "statusline" => Ok(SurfaceRouteKind::Statusline),
        other => Err(SurfaceManifestError::new(
            path,
            field_path,
            format!("unknown route kind `{other}`"),
        )),
    }
}

fn parse_route_status(
    raw: Option<String>,
    path: &Path,
    field_path: &str,
) -> Result<SurfaceRouteStatus, SurfaceManifestError> {
    let raw =
        raw.ok_or_else(|| SurfaceManifestError::new(path, field_path, "missing required field"))?;
    match raw.as_str() {
        "ready" => Ok(SurfaceRouteStatus::Ready),
        "partial" => Ok(SurfaceRouteStatus::Partial),
        "planned" => Ok(SurfaceRouteStatus::Planned),
        "unavailable" => Ok(SurfaceRouteStatus::Unavailable),
        "cli_only" | "cli-only" => Ok(SurfaceRouteStatus::CliOnly),
        other => Err(SurfaceManifestError::new(
            path,
            field_path,
            format!(
                "unknown route status `{other}`; expected ready, partial, planned, unavailable, cli_only, or cli-only"
            ),
        )),
    }
}

fn serde_path_to_string(path: &serde_path_to_error::Path) -> String {
    let mut segments = Vec::new();
    for segment in path.iter() {
        match segment {
            serde_path_to_error::Segment::Map { key } => segments.push(key.to_string()),
            serde_path_to_error::Segment::Seq { index } => segments.push(format!("[{index}]")),
            serde_path_to_error::Segment::Enum { variant } => segments.push(variant.to_string()),
            &serde_path_to_error::Segment::Unknown => segments.push("?".to_string()),
        }
    }
    if segments.is_empty() {
        "root".to_string()
    } else {
        segments.join(".")
    }
}

/// Returns the canonical target string of a surface route (path/command/action/label).
/// Used for cross-surface route deduplication and reverse-drift checks.
pub fn surface_route_target(route: &SurfaceRoute) -> Option<&str> {
    route
        .path
        .as_deref()
        .or(route.command.as_deref())
        .or(route.action.as_deref())
        .or(route.label.as_deref())
}

/// Stable dedup key for cross-surface duplicate detection (kind + canonical target).
pub fn surface_route_dedup_key(route: &SurfaceRoute) -> Option<(SurfaceRouteKind, String)> {
    surface_route_target(route).map(|target| (route.kind, target.to_string()))
}

/// Human-readable label for a route kind. Used in cross-surface diagnostics.
pub fn surface_route_kind_label(kind: SurfaceRouteKind) -> &'static str {
    match kind {
        SurfaceRouteKind::Tui => "tui",
        SurfaceRouteKind::Slash => "slash",
        SurfaceRouteKind::Palette => "palette",
        SurfaceRouteKind::Cli => "cli",
        SurfaceRouteKind::Statusline => "statusline",
    }
}

#[cfg(test)]
#[path = "surface_manifest_tests.rs"]
mod tests;
