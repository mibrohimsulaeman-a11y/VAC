use crate::enforcement_banner::enforcement_banner_lines;
use crate::enforcement_banner::enforcement_status_label;
use crate::history_cell::PlainHistoryCell;
use crate::legacy_core::config::Config;
use crate::surface_route_catalog::SurfaceRouteCatalog;
use ratatui::style::Stylize;
use ratatui::text::Line;
use std::path::Path;
use vac_core::control_plane::RegistryLoadReport;
use vac_core::control_plane::RegistryLoadState;
use vac_core::control_plane::build_root_feature_conversion_report;
use vac_core::control_plane::capability_manifest::CapabilityCliSurface;
use vac_core::control_plane::capability_manifest::CapabilityCompatibilityTransport;
use vac_core::control_plane::capability_manifest::CapabilityManifest;
use vac_core::control_plane::capability_manifest::CapabilityOwner;
use vac_core::control_plane::capability_manifest::CapabilityPolicy;
use vac_core::control_plane::capability_manifest::CapabilityPolicyValue;
use vac_core::control_plane::capability_manifest::CapabilityStatus;
use vac_core::control_plane::capability_manifest::CapabilitySurfaces;
use vac_core::control_plane::capability_manifest::CapabilityValidation;
use vac_core::control_plane::load_architecture_invariant_report;
use vac_core::control_plane::load_control_plane_registry_report;
use vac_core::control_plane::load_ownership_scan_report;
use vac_core::control_plane::load_policy_doctor_report_for_path;

pub(crate) fn new_capability_dashboard_output(config: &Config) -> PlainHistoryCell {
    PlainHistoryCell::new(render_capability_dashboard_lines_for_config(config))
}

pub(crate) fn new_capability_dashboard_view(
    config: &Config,
) -> crate::bottom_pane::SelectionViewParams {
    let cwd = config.cwd.as_path();
    let report = load_control_plane_registry_report(cwd);
    let mut items = Vec::new();
    if let Some(registry) = report.registry() {
        for located in &registry.capabilities.manifests {
            let manifest = &located.manifest;
            let owner = format_capability_owner(&manifest.owner);
            let readiness = format_capability_status(manifest.status);
            let gates = format_capability_validation(&manifest.validation);
            items.push(crate::bottom_pane::SelectionItem {
                name: manifest.id.clone(),
                description: Some(format!(
                    "{readiness} · owner={owner} · gates={gates} · evidence=/capabilities drill-down"
                )),
                selected_description: Some(format!(
                    "title: {}\nowner: {}\nstatus: {}\npolicy: {}\nsurfaces: {}\nsource: {}",
                    manifest.title,
                    owner,
                    readiness,
                    format_capability_policy(&manifest.policy),
                    format_capability_surfaces(&manifest.surfaces),
                    located.path.display(),
                )),
                dismiss_on_select: false,
                search_value: Some(format!("{} {} {}", manifest.id, manifest.title, owner)),
                ..Default::default()
            });
        }
    }
    if items.is_empty() {
        items.push(crate::bottom_pane::SelectionItem {
            name: "no capabilities loaded".to_string(),
            description: Some(
                "registry empty or still loading; run vac doctor registry".to_string(),
            ),
            is_disabled: true,
            ..Default::default()
        });
    }
    crate::bottom_pane::SelectionViewParams {
        view_id: Some("capability-dashboard"),
        title: Some("VAC Capability Dashboard".to_string()),
        subtitle: Some(
            "↑/↓ navigate · Enter keeps drill-down open · Esc closes · type to filter".to_string(),
        ),
        items,
        is_searchable: true,
        search_placeholder: Some("filter capability, owner, or readiness".to_string()),
        row_display: crate::bottom_pane::SelectionRowDisplay::Wrapped,
        ..Default::default()
    }
}

fn render_capability_dashboard_lines(cwd: &Path) -> Vec<Line<'static>> {
    let report = load_control_plane_registry_report(cwd);
    render_capability_dashboard_lines_with_report(cwd, &report)
}

fn render_capability_dashboard_lines_for_config(config: &Config) -> Vec<Line<'static>> {
    let cwd = config.cwd.as_path();
    let report = load_control_plane_registry_report(cwd);
    render_capability_dashboard_lines_with_report_and_config(cwd, &report, Some(config))
}

fn render_capability_dashboard_lines_with_report(
    cwd: &Path,
    report: &RegistryLoadReport,
) -> Vec<Line<'static>> {
    render_capability_dashboard_lines_with_report_and_config(cwd, report, None)
}

fn render_capability_dashboard_lines_with_report_and_config(
    cwd: &Path,
    report: &RegistryLoadReport,
    config: Option<&Config>,
) -> Vec<Line<'static>> {
    let ownership_report = load_ownership_scan_report(cwd);
    let shell_state = build_operator_dashboard_state(cwd, report, &ownership_report, config);
    let mut lines = crate::operator_ui::render_capability_dashboard_visual_lines(&shell_state, 156);
    lines.push("".into());
    lines.push("/capabilities".magenta().into());
    lines.push("".into());
    lines.push("Control plane registry:".bold().into());
    lines.extend(
        report
            .render_tui_lines()
            .into_iter()
            .map(|line| format!("  {line}").into()),
    );

    if report.state() == RegistryLoadState::Loading {
        lines.push("".into());
        lines.push("Loading capability registry...".dim().into());
        return lines;
    }

    lines.push("".into());
    lines.push("Enforcement:".bold().into());
    lines.extend(enforcement_banner_lines(cwd));

    if let Some(registry) = report.registry() {
        lines.push("".into());
        lines.push("Capabilities:".bold().into());
        if registry.capabilities.manifests.is_empty() {
            lines.push("  <none>".dim().into());
        } else {
            for (index, located_manifest) in registry.capabilities.manifests.iter().enumerate() {
                lines.extend(render_capability_manifest_lines(
                    index + 1,
                    &located_manifest.manifest,
                ));
            }
        }
    }

    lines.push("".into());
    lines.push("Ownership scan:".bold().into());
    lines.extend(
        ownership_report
            .render_scan_lines()
            .into_iter()
            .map(|line| format!("  {line}").into()),
    );

    if let Some(registry) = report.registry() {
        let conversion_report = build_root_feature_conversion_report(registry, &ownership_report);
        lines.push("".into());
        lines.push("Root feature conversion:".bold().into());
        lines.extend(render_root_feature_conversion_lines(&conversion_report));
    }

    let architecture_report = load_architecture_invariant_report(cwd);
    lines.push("".into());
    lines.push("Architecture invariants:".bold().into());
    lines.extend(
        architecture_report
            .render_lines()
            .into_iter()
            .map(|line| format!("  {line}").into()),
    );

    let route_catalog = SurfaceRouteCatalog::load(cwd);
    lines.push("".into());
    lines.push("Surface routes:".bold().into());
    lines.extend(render_surface_route_catalog_lines(&route_catalog));

    let policy_report = load_policy_doctor_report_for_path(cwd);
    lines.push("".into());
    lines.push("Policy manifests:".bold().into());
    lines.extend(
        policy_report
            .render_tui_lines()
            .into_iter()
            .map(|line| format!("  {line}").into()),
    );

    lines
}

fn strings_to_lines(lines: Vec<String>) -> Vec<Line<'static>> {
    lines.into_iter().map(Line::from).collect()
}

fn build_operator_dashboard_state(
    cwd: &Path,
    report: &RegistryLoadReport,
    ownership_report: &vac_core::control_plane::OwnershipScanReport,
    config: Option<&Config>,
) -> crate::operator_ui::CapabilityDashboardState {
    let capability_count = report
        .registry()
        .map(|registry| registry.capabilities.manifests.len())
        .unwrap_or(0);
    let ready_count = report
        .registry()
        .map(|registry| {
            registry
                .capabilities
                .manifests
                .iter()
                .filter(|located| matches!(located.manifest.status, CapabilityStatus::Ready))
                .count()
        })
        .unwrap_or(0);
    let valid_percent = if capability_count == 0 {
        0
    } else {
        ((ready_count * 100) / capability_count).min(100) as u8
    };
    let registry_summary = report.render_tui_lines();
    let capability_rows = report
        .registry()
        .map(|registry| {
            registry
                .capabilities
                .manifests
                .iter()
                .map(|located| {
                    let manifest = &located.manifest;
                    format!(
                        "{} | {} | {} | {} | {} | {}",
                        manifest.id,
                        format_capability_status(manifest.status),
                        format_capability_owner(&manifest.owner),
                        format_capability_surfaces(&manifest.surfaces),
                        format_capability_policy(&manifest.policy),
                        format_capability_validation(&manifest.validation),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let diagnostics: Vec<String> = report
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.render_line())
        .collect();
    let capability_records = report
        .registry()
        .map(|registry| {
            registry
                .capabilities
                .manifests
                .iter()
                .map(|located| {
                    let manifest = &located.manifest;
                    crate::operator_ui::CapabilityDashboardRecord::new(
                        manifest.id.clone(),
                        crate::operator_ui::CapabilityManifestStatus::parse(
                            format_capability_status(manifest.status),
                        )
                        .unwrap_or(crate::operator_ui::CapabilityManifestStatus::Broken),
                        format_capability_owner(&manifest.owner),
                        format_capability_surfaces(&manifest.surfaces),
                        format_capability_policy(&manifest.policy),
                        format_capability_validation(&manifest.validation),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let diagnostic_records = diagnostics
        .iter()
        .cloned()
        .map(crate::operator_ui::ControlPlaneDiagnostic::from_text)
        .collect::<Vec<_>>();

    crate::operator_ui::CapabilityDashboardState {
        chrome_title: "vac · interactive".to_string(),
        chrome_mode: "registry /capabilities".to_string(),
        capability_count,
        owned_domains: ownership_report.owned_count(),
        unowned_domains: ownership_report
            .unowned_source_domain_count()
            .max(ownership_report.unowned_count()),
        valid_percent,
        registry_summary,
        capability_rows,
        diagnostics,
        capability_records,
        diagnostic_records,
        status_bar: capability_dashboard_status_bar(cwd, config, valid_percent),
    }
}

fn capability_dashboard_status_bar(
    cwd: &Path,
    config: Option<&Config>,
    valid_percent: u8,
) -> crate::operator_ui::OperatorStatusBarState {
    let model = config
        .and_then(|config| config.model.clone())
        .unwrap_or_else(|| "runtime".to_string());
    let validation = format!("valid {valid_percent}% · {}", enforcement_status_label(cwd));
    let status_bar =
        crate::operator_ui::OperatorStatusBarState::conversation(model, "0 tok", validation);
    if let Some(config) = config {
        status_bar.with_profile_rulebook(
            crate::history_cell::operator_profile_label(config),
            crate::history_cell::operator_rulebook_label(config),
        )
    } else {
        status_bar
    }
}

fn render_surface_route_catalog_lines(catalog: &SurfaceRouteCatalog) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut rendered_any_kind = false;
    rendered_any_kind |= render_route_kind_lines(&mut lines, "tui", catalog.tui_routes());
    rendered_any_kind |= render_route_kind_lines(&mut lines, "slash", catalog.slash_routes());
    rendered_any_kind |= render_route_kind_lines(&mut lines, "palette", catalog.palette_routes());
    rendered_any_kind |= render_route_kind_lines(&mut lines, "cli", catalog.cli_routes());
    rendered_any_kind |=
        render_route_kind_lines(&mut lines, "statusline", catalog.statusline_routes());

    if !rendered_any_kind {
        lines.push("  <none>".dim().into());
    }

    lines
}

fn render_route_kind_lines(
    lines: &mut Vec<Line<'static>>,
    kind: &str,
    routes: Vec<(String, crate::surface_route_catalog::SurfaceRouteMetadata)>,
) -> bool {
    if routes.is_empty() {
        return false;
    }

    lines.push(format!("  {kind}:").bold().into());
    for (name, route) in routes {
        let label = if kind == "slash" {
            format!("/{}", name)
        } else {
            name
        };
        let mut text = format!("    {label} — {} / {}", route.status, route.capability);
        if let Some(owner) = route.owner.as_deref() {
            text.push_str(&format!(" owner={}", format_surface_route_owner(owner)));
        }
        if !route.visible {
            let reason = route
                .reason
                .as_deref()
                .unwrap_or("route marked unavailable");
            text.push_str(&format!(" (disabled: {reason})"));
        }
        lines.push(text.into());
    }

    true
}

fn render_root_feature_conversion_lines(
    report: &vac_core::control_plane::RootFeatureConversionReport,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (index, line) in report.render_lines().into_iter().enumerate() {
        lines.push(format!("  {line}").into());
        if index == 0 {
            lines.push(
                "  legend: complete=ready owned, partial=planned/partial, hidden=missing capability or ownership metadata gap, source:* hidden=unowned source domain, overclaimed=claims missing source"
                    .dim()
                    .into(),
            );
        }
    }
    lines
}

fn render_capability_manifest_lines(
    index: usize,
    manifest: &CapabilityManifest,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(format!("  {index}. {} — {}", manifest.id, manifest.title).into());
    lines.push(format!("     status: {}", format_capability_status(manifest.status)).into());
    lines.push(format!("     owner: {}", format_capability_owner(&manifest.owner)).into());
    lines.push(
        format!(
            "     surfaces: {}",
            format_capability_surfaces(&manifest.surfaces)
        )
        .into(),
    );
    lines.push(
        format!(
            "     policy: {}",
            format_capability_policy(&manifest.policy)
        )
        .into(),
    );
    lines.push(
        format!(
            "     validation: {}",
            format_capability_validation(&manifest.validation)
        )
        .into(),
    );
    if let Some(description) = manifest.description.as_deref() {
        lines.push(format!("     description: {description}").into());
    }
    if !manifest.depends_on.is_empty() {
        lines.push(format!("     depends_on: {}", manifest.depends_on.join(", ")).into());
    }
    if !manifest.optional_depends_on.is_empty() {
        lines.push(
            format!(
                "     optional_depends_on: {}",
                manifest.optional_depends_on.join(", ")
            )
            .into(),
        );
    }
    if !manifest.docs.is_empty() {
        lines.push(format!("     docs: {}", manifest.docs.join(", ")).into());
    }
    if !manifest.compatibility_transport.is_empty() {
        lines.push(
            format!(
                "     compatibility_transport: {}",
                format_capability_compatibility_transport(&manifest.compatibility_transport)
            )
            .into(),
        );
    }
    lines
}

fn format_capability_owner(owner: &CapabilityOwner) -> String {
    match owner {
        CapabilityOwner::Path(path) => path.clone(),
        CapabilityOwner::Structured(object) => {
            if let Some(team) = &object.team {
                format!("{}/{} (team: {team})", object.crate_name, object.module)
            } else {
                format!("{}/{}", object.crate_name, object.module)
            }
        }
    }
}

fn format_surface_route_owner(owner: &str) -> String {
    owner.replace("::", "/")
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

fn format_capability_surfaces(surfaces: &CapabilitySurfaces) -> String {
    let mut parts = Vec::new();
    parts.push(format!("palette={}", surfaces.palette));
    if let Some(tui) = &surfaces.tui {
        parts.push(format!("tui=[{}]", tui.routes.join(", ")));
    }
    if let Some(slash) = &surfaces.slash {
        parts.push(format!("slash=[{}]", slash.commands.join(", ")));
    }
    if let Some(cli) = &surfaces.cli {
        parts.push(format!("cli={}", format_capability_cli_surface(cli)));
    }
    parts.join(", ")
}

fn format_capability_cli_surface(cli: &CapabilityCliSurface) -> String {
    if cli.commands.is_empty() {
        format!("enabled={}", cli.enabled)
    } else {
        format!(
            "enabled={}, commands=[{}]",
            cli.enabled,
            cli.commands.join(", ")
        )
    }
}

fn format_capability_policy(policy: &CapabilityPolicy) -> String {
    let mut parts = Vec::new();
    if let Some(risk) = policy.risk.as_ref() {
        parts.push(format!("risk={risk}"));
    }
    if let Some(mutates_files) = policy.mutates_files.as_ref() {
        parts.push(format!(
            "mutates_files={}",
            format_capability_policy_value(mutates_files)
        ));
    }
    if let Some(network) = policy.network.as_ref() {
        parts.push(format!(
            "network={}",
            format_capability_policy_value(network)
        ));
    }
    if let Some(redaction) = policy.redaction.as_ref() {
        parts.push(format!(
            "redaction={}",
            format_capability_policy_value(redaction)
        ));
    }
    if !policy.approval_required_for.is_empty() {
        parts.push(format!(
            "approval_required_for=[{}]",
            policy.approval_required_for.join(", ")
        ));
    }
    if parts.is_empty() {
        "<none>".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_capability_policy_value(value: &CapabilityPolicyValue) -> String {
    match value {
        CapabilityPolicyValue::Bool(value) => value.to_string(),
        CapabilityPolicyValue::Text(value) => value.clone(),
    }
}

fn format_capability_validation(validation: &CapabilityValidation) -> String {
    let mut parts = Vec::new();
    if !validation.commands.is_empty() {
        parts.push(format!("commands=[{}]", validation.commands.join(", ")));
    }
    if !validation.gates.is_empty() {
        parts.push(format!("gates=[{}]", validation.gates.join(", ")));
    }
    if parts.is_empty() {
        "<none>".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_capability_compatibility_transport(
    transports: &[CapabilityCompatibilityTransport],
) -> String {
    transports
        .iter()
        .map(|transport| {
            format!(
                "{} [{}] owner={} replacement={} target_removal_plan={}",
                transport.dependency,
                transport.status,
                transport.owner,
                transport.replacement,
                transport.target_removal_plan
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::render_capability_dashboard_lines;
    use super::render_capability_dashboard_lines_with_report;
    use ratatui::text::Line;
    use std::fs;
    use tempfile::tempdir;
    use vac_core::control_plane::RegistryLoadReport;
    use vac_core::control_plane::load_control_plane_registry_report;

    fn render_to_text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn capability_dashboard_reports_missing_root() {
        let tempdir = tempdir().expect("tempdir");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));
        assert!(rendered.contains("registry: empty"));
        assert!(rendered.contains("unable to locate `.vac` root"));
    }

    #[test]
    fn capability_dashboard_renders_loading_state() {
        let tempdir = tempdir().expect("tempdir");

        let rendered = render_to_text(&render_capability_dashboard_lines_with_report(
            tempdir.path(),
            &RegistryLoadReport::loading(),
        ));

        assert!(rendered.contains("registry: loading"));
        assert!(rendered.contains("Loading capability registry..."));
        assert!(!rendered.contains("Ownership scan:"));
    }

    #[test]
    fn capability_dashboard_lists_loaded_capabilities() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/vac.chat.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: partial
reason: capability coverage is still being expanded for the root dashboard
owner:
  crate: vac-surface-tui
  module: chatwidget
states:
- empty
- loading
- success
- failure
surfaces:
  tui:
    routes: [/capabilities]
  slash:
    commands: [/capabilities]
  palette: true
  cli:
    enabled: true
    commands: [vac, "vac exec"]
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for: [write_files, execute_process]
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");

        let report = load_control_plane_registry_report(tempdir.path());
        assert!(report.registry().is_some(), "{}", report.render_tui_text());

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));
        assert!(rendered.contains("registry: loaded 1 manifests"));
        assert!(rendered.contains("root seed coverage: blocked"));
        assert!(rendered.contains("capabilities=1/13"));
        assert!(rendered.contains("policies=0/6"));
        assert!(rendered.contains("Capabilities:"));
        assert!(rendered.contains("vac.chat"));
        assert!(rendered.contains("Chat"));
        assert!(rendered.contains("status: partial"));
        assert!(rendered.contains("owner: vac-surface-tui/chatwidget"));
        assert!(rendered.contains("surfaces:"));
        assert!(rendered.contains("policy:"));
        assert!(rendered.contains("validation:"));
        assert!(rendered.contains("Ownership scan:"));
        assert!(rendered.contains("ownership scan: total=1 owned=0 unowned=1 ready_unowned=0"));
        assert!(rendered.contains("ownership targets: total=0 test_only=0 retired=0"));
        assert!(rendered.contains("ownership targets matrix: <none>"));
        assert!(rendered.contains("warning: capability missing ownership metadata"));
    }

    #[test]
    fn capability_dashboard_lists_surface_routes() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/vac.workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: ready
owner:
  crate: vac-surface-tui
  module: workflow_browser
states:
- empty
- loading
- success
- failure
surfaces:
  tui:
    routes: [/workflow]
  slash:
    commands: [/workflow]
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/slash.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.slash
title: Slash surface
routes:
  - kind: slash
    command: /workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: ready
capabilities: [vac.workflow]
"#,
        )
        .expect("slash manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/palette.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.palette
title: Palette surface
routes:
  - kind: palette
    action: open_workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: ready
capabilities: [vac.workflow]
"#,
        )
        .expect("palette manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/cli.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.cli
title: CLI surface
routes:
  - kind: cli
    command: vac doctor registry
    capability: vac.workflow
    owner: "vac-rs/cli"
    visible: true
    status: ready
capabilities: [vac.workflow]
"#,
        )
        .expect("cli manifest");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));
        assert!(rendered.contains("Surface routes:"));
        assert!(rendered.contains("slash:"), "rendered:\n{rendered}");
        assert!(rendered.contains("/workflow — ready / vac.workflow"));
        assert!(rendered.contains("palette:"));
        assert!(rendered.contains("open_workflow — ready / vac.workflow"));
        assert!(rendered.contains("cli:"));
        assert!(rendered.contains("vac doctor registry — ready / vac.workflow"));
    }

    #[test]
    fn capability_dashboard_lists_policy_manifests() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/policies")).expect("policies dir");
        fs::write(
            tempdir.path().join(".vac/policies/sandbox.yaml"),
            r#"
schema_version: 1
kind: policy
id: vac.sandbox
title: Sandbox policy
default_decision: approval_required
rules:
  - id: deny-credential-read
    match:
      action: credential_read
      data_class: credential
    decision: deny
    reason: credentials must never be exposed to workflows
"#,
        )
        .expect("policy manifest");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));
        assert!(
            rendered.contains("Policy manifests:"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("policy: manifests=1 rules=1 allow=0 approval_required=0 deny=1"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("vac.sandbox"));
        assert!(rendered.contains("Sandbox policy"));
    }

    #[test]
    fn capability_dashboard_root_features_renders_hidden_summary() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac")).expect("vac root");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));

        assert!(
            rendered.contains("Root feature conversion:"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("root feature conversion:"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("hidden=13"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("legend: complete=ready owned"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn capability_dashboard_root_features_renders_source_hidden_domains() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/ownership.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: ownership metadata drill-down is still being completed
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
states:
- empty
- loading
- success
- failure
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join("vac-rs/core/Cargo.toml"),
            r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
        )
        .expect("cargo toml");
        fs::write(
            tempdir.path().join("vac-rs/core/src/lib.rs"),
            "pub mod control_plane;\npub mod unowned_domain;\n",
        )
        .expect("lib rs");
        fs::write(tempdir.path().join("vac-rs/core/src/control_plane.rs"), "")
            .expect("owned domain");
        fs::write(tempdir.path().join("vac-rs/core/src/unowned_domain.rs"), "")
            .expect("unowned domain");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));

        assert!(
            rendered.contains("source domains:"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("vac-core/unowned_domain hidden"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("source:* hidden=unowned source domain"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn capability_dashboard_root_features_marks_missing_ownership_hidden() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/chat.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
owner:
  crate: vac-core
  module: chatwidget
states:
- empty
- loading
- success
- failure
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join("vac-rs/core/Cargo.toml"),
            r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
        )
        .expect("cargo toml");
        fs::write(
            tempdir.path().join("vac-rs/core/src/lib.rs"),
            "pub mod chatwidget;\n",
        )
        .expect("lib rs");
        fs::write(tempdir.path().join("vac-rs/core/src/chatwidget.rs"), "").expect("chatwidget rs");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));

        assert!(
            rendered.contains("[hidden] status=ready"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("warning: ready capability missing ownership metadata"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn capability_dashboard_root_features_renders_overclaimed_modules() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/chat.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
owner:
  crate: vac-core
  module: missing_chat
ownership:
  crates:
    - vac-core
  modules:
    - missing_chat
states:
- empty
- loading
- success
- failure
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join("vac-rs/core/Cargo.toml"),
            r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
        )
        .expect("cargo toml");
        fs::write(
            tempdir.path().join("vac-rs/core/src/lib.rs"),
            "pub mod chatwidget;\n",
        )
        .expect("lib rs");
        fs::write(tempdir.path().join("vac-rs/core/src/chatwidget.rs"), "").expect("chatwidget rs");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));

        assert!(rendered.contains("overclaimed=1"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("overclaimed_modules=[missing_chat]"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn capability_dashboard_root_features_renders_partial_reason() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join("vac-rs/cli/src")).expect("source dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/build.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.build
title: Build
status: planned
reason: build hardening pending phase 4
owner:
  crate: vac-surface-cli
  module: main
ownership:
  crates:
    - vac-cli
  modules:
    - main
states:
- empty
- loading
- success
- failure
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join("vac-rs/cli/Cargo.toml"),
            r#"
[package]
name = "vac-cli"
version = "0.0.0"
edition = "2024"
"#,
        )
        .expect("cargo toml");
        fs::write(
            tempdir.path().join("vac-rs/cli/src/main.rs"),
            "fn main() {}\n",
        )
        .expect("main rs");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));

        assert!(rendered.contains("partial=1"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("[partial] status=planned"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("reason=\"build hardening pending phase 4\""),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn capability_dashboard_lists_tui_and_statusline_routes() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/vac.workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: ready
owner:
  crate: vac-surface-tui
  module: workflow_browser
states:
- empty
- loading
- success
- failure
surfaces:
  tui:
    routes: [/capabilities]
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/tui.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.tui
title: TUI surface
routes:
  - kind: tui
    path: /capabilities
    capability: vac.workflow
    owner: "vac-tui::capability_dashboard"
    visible: true
    status: ready
capabilities: [vac.workflow]
"#,
        )
        .expect("tui manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/statusline.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.statusline
title: Statusline surface
routes:
  - kind: statusline
    label: session
    capability: vac.workflow
    owner: "vac-tui::statusline"
    visible: true
    status: partial
    reason: "statusline guidance still under development"
capabilities: [vac.workflow]
"#,
        )
        .expect("statusline manifest");

        let rendered = render_to_text(&render_capability_dashboard_lines(tempdir.path()));
        assert!(
            rendered.contains("Surface routes:"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("  tui:"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("/capabilities \u{2014} ready / vac.workflow"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("  statusline:"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("session \u{2014} partial / vac.workflow owner=vac-tui/statusline"),
            "rendered:\n{rendered}"
        );
    }
}
