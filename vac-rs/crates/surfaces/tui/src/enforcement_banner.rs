use crate::legacy_core::config::Config;
use ratatui::style::Stylize;
use ratatui::text::Line;
use std::path::Path;
use vac_core::control_plane::load_enforcement_status_report;

pub(crate) fn enforcement_status_label(workspace_root: impl AsRef<Path>) -> String {
    let report = load_enforcement_status_report(workspace_root);
    let mut label = format!("enforcement {}", report.claimed_level);
    if report.explicit_claim {
        label.push_str(" · explicit");
    } else {
        label.push_str(" · defaulted");
    }
    if report.is_failure() {
        label.push_str(" · blocked");
    } else if report.observed.is_l2() {
        label.push_str(" · fail-closed");
    } else {
        label.push_str(" · advisory");
    }
    label
}

pub(crate) fn enforcement_banner_lines(workspace_root: impl AsRef<Path>) -> Vec<Line<'static>> {
    let report = load_enforcement_status_report(workspace_root);
    let mut lines = vec![
        format!("Enforcement: {}", report.claimed_level)
            .bold()
            .into(),
        format!(
            "  claim: {} · observed: {}",
            if report.explicit_claim {
                "explicit"
            } else {
                "defaulted"
            },
            if report.observed.is_l2() { "L2" } else { "L1" }
        )
        .dim()
        .into(),
        format!(
            "  scope: fail-closed={} out_of_band_blocked={} advisory_only={}",
            report.claim_scope.fail_closed_enforcement,
            report.claim_scope.out_of_band_blocked,
            report.claim_scope.advisory_only
        )
        .dim()
        .into(),
    ];

    if report.claimed_level == vac_core::control_plane::EnforcementLevel::L1 {
        lines.push(
            "  L1 — advisory/cooperative mode; guarantees reduced to discipline + audit"
                .dim()
                .into(),
        );
    }
    for warning in report.warnings {
        lines.push(format!("  warning: {warning}").into());
    }
    for error in report.errors {
        lines.push(format!("  error: {error}").red().into());
    }
    lines
}

pub(crate) fn operator_status_validation_label(
    config: &Config,
    baseline: impl Into<String>,
) -> String {
    let baseline = baseline.into();
    let enforcement = enforcement_status_label(&config.cwd);
    format!("{baseline} · {enforcement}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn status_label_defaults_to_advisory() {
        let tempdir = tempdir().expect("tempdir");
        let label = enforcement_status_label(tempdir.path());
        assert!(label.contains("enforcement L1"), "{label}");
        assert!(label.contains("advisory"), "{label}");
    }

    #[test]
    fn banner_renders_explicit_claim() {
        let tempdir = tempdir().expect("tempdir");
        std::fs::create_dir_all(tempdir.path().join(".vac/registry")).expect("registry");
        std::fs::write(
            tempdir.path().join(".vac/registry/init_state.yaml"),
            "schema_version: 1\nkind: init_state\nid: init.state\nenforcement_level: L2\n",
        )
        .expect("write");
        let lines = enforcement_banner_lines(tempdir.path());
        let rendered = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("Enforcement: L2"), "{rendered}");
    }
}
