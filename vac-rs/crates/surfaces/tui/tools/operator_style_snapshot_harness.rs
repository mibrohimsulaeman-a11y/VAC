//! Standalone ANSI snapshot writer for the VAC operator-console style contract.
//!
//! Build with direct rustc when Cargo is too heavy in sandbox:
//! rustc --edition 2024 vac-rs/tui/tools/operator_style_snapshot_harness.rs -o /tmp/vac-operator-style-snapshot-harness

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "../src/operator_style.rs"]
mod operator_style;
#[allow(dead_code)]
#[path = "../src/operator_ui.rs"]
mod operator_ui;
#[allow(dead_code)]
#[path = "../src/operator_widget_render.rs"]
mod operator_widget_render;
#[allow(dead_code)]
#[path = "../src/ui_consts.rs"]
mod ui_consts;

fn main() -> io::Result<()> {
    let out_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs/validation/tui-operator-ansi-snapshots"));
    fs::create_dir_all(&out_dir)?;

    for scenario in operator_ui::SnapshotScenario::ALL {
        for viewport in operator_ui::OperatorViewport::VISUAL_FIDELITY_MATRIX {
            let filename =
                operator_ui::render_operator_snapshot_filename_for_viewport(scenario, viewport)
                    .replace(".txt", ".ansi.txt");
            let styled = operator_ui::render_operator_snapshot_ansi_text(scenario, viewport);
            let mut content = String::new();
            content.push_str("# VAC operator TUI ANSI snapshot\n");
            content.push_str(&format!("scenario: {}\n", scenario.title()));
            content.push_str(&format!(
                "viewport: {}x{}\n",
                viewport.width, viewport.height
            ));
            content.push_str(
                "style_roles: plain, chrome, muted, accent, success, warning, danger, user, agent, status\n",
            );
            content.push_str("---\n");
            content.push_str(&styled);
            content.push('\n');
            fs::write(out_dir.join(filename), content)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_scenarios_can_be_styled_and_stripped_without_layout_drift() {
        for scenario in operator_ui::SnapshotScenario::ALL {
            for viewport in operator_ui::OperatorViewport::VISUAL_FIDELITY_MATRIX {
                let plain = operator_ui::render_operator_snapshot_text(scenario, viewport);
                let styled = operator_ui::render_operator_snapshot_ansi_text(scenario, viewport);
                assert_ne!(plain, styled);
                assert_eq!(operator_style::strip_ansi(&styled), plain);
            }
        }
    }

    #[test]
    fn role_order_contains_required_tokens() {
        let tokens = operator_style::STYLE_ROLE_ORDER
            .iter()
            .map(|role| role.token())
            .collect::<Vec<_>>()
            .join(",");
        for required in [
            "plain", "chrome", "muted", "accent", "success", "warning", "danger", "user", "agent",
            "status",
        ] {
            assert!(tokens.contains(required), "missing {required}");
        }
    }
}
