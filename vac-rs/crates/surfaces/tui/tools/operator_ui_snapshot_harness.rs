//! Standalone snapshot writer for the VAC operator-console TUI render model.
//!
//! Build with direct rustc when Cargo is too heavy in sandbox:
//! rustc --edition 2024 vac-rs/tui/tools/operator_ui_snapshot_harness.rs -o /tmp/vac-operator-ui-snapshot-harness

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
        .unwrap_or_else(|| PathBuf::from("docs/validation/tui-operator-ui-snapshots"));
    fs::create_dir_all(&out_dir)?;

    for scenario in operator_ui::SnapshotScenario::ALL {
        for viewport in operator_ui::OperatorViewport::VISUAL_FIDELITY_MATRIX {
            let filename =
                operator_ui::render_operator_snapshot_filename_for_viewport(scenario, viewport);
            let mut content = String::new();
            content.push_str("# VAC operator TUI snapshot\n");
            content.push_str(&format!("scenario: {}\n", scenario.title()));
            content.push_str(&format!(
                "viewport: {}x{}\n",
                viewport.width, viewport.height
            ));
            content.push_str("---\n");
            content.push_str(&operator_ui::render_operator_snapshot_text(
                scenario, viewport,
            ));
            content.push('\n');
            fs::write(out_dir.join(filename), content)?;
        }
    }

    Ok(())
}
