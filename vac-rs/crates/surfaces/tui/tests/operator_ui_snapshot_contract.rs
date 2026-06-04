#[allow(dead_code)]
#[path = "../src/operator_style.rs"]
mod operator_style;
#[allow(dead_code)]
#[path = "../src/operator_ui.rs"]
mod operator_ui;
#[allow(dead_code)]
#[path = "../src/operator_widget_render.rs"]
mod operator_widget_render;

#[test]
fn generated_operator_snapshots_are_bounded_widget_screens() {
    for scenario in operator_ui::SnapshotScenario::ALL {
        let viewport = scenario.default_viewport();
        let rendered = operator_ui::render_operator_snapshot(scenario, viewport);
        assert!(!rendered.is_empty(), "{} snapshot empty", scenario.slug());
        assert!(
            rendered.len() <= viewport.height,
            "{} height exceeds viewport",
            scenario.slug()
        );
        assert!(
            rendered
                .iter()
                .all(|line| line.chars().count() <= viewport.width),
            "{} width exceeds viewport",
            scenario.slug()
        );
    }
}

#[test]
fn generated_operator_panel_snapshots_keep_widget_geometry() {
    for scenario in [
        operator_ui::SnapshotScenario::AgentWorking,
        operator_ui::SnapshotScenario::ApprovalPopup,
        operator_ui::SnapshotScenario::RuntimeJobs,
        operator_ui::SnapshotScenario::CapabilityDashboard,
    ] {
        let rendered =
            operator_ui::render_operator_snapshot(scenario, scenario.default_viewport()).join("\n");
        assert!(
            rendered.contains('╭'),
            "{} should contain ratatui panel geometry",
            scenario.slug()
        );
    }
}

#[test]
fn generated_operator_snapshots_keep_user_reference_surfaces_visible() {
    let first_launch = operator_ui::render_operator_snapshot_text(
        operator_ui::SnapshotScenario::FirstLaunch,
        operator_ui::SnapshotScenario::FirstLaunch.default_viewport(),
    );
    assert!(first_launch.contains("vac · interactive"));
    assert!(first_launch.contains("Vastar Agentic CLI"));
    assert!(first_launch.contains("hydrating startup snapshot"));
    assert!(first_launch.contains("VAC operator console"));
    assert!(!first_launch.contains("VIL-native"));
    assert!(first_launch.contains("type / for commands"));

    let idle = operator_ui::render_operator_snapshot_text(
        operator_ui::SnapshotScenario::Idle,
        operator_ui::SnapshotScenario::Idle.default_viewport(),
    );
    assert!(idle.contains("vac · interactive"));
    assert!(idle.contains("VAC"));
    assert!(!idle.contains("VIL-native"));
    assert!(idle.contains("ready"));
    assert!(idle.contains("recent tasks"));
    assert!(idle.contains("no persisted recent task loaded"));
    assert!(idle.contains("shift+tab"));
    assert!(!idle.contains("VAC 0.4.3 available"));

    let agent = operator_ui::render_operator_snapshot_text(
        operator_ui::SnapshotScenario::AgentWorking,
        operator_ui::SnapshotScenario::AgentWorking.default_viewport(),
    );
    assert!(agent.contains("vac · interactive"));
    assert!(agent.contains("tool timeline"));
    assert!(!agent.contains("glob"));
    assert!(agent.contains("file_read"));
    assert!(agent.contains("file_write"));
    assert!(agent.contains("cargo check"));
    assert!(agent.contains("context"));

    let approval = operator_ui::render_operator_snapshot_text(
        operator_ui::SnapshotScenario::ApprovalPopup,
        operator_ui::SnapshotScenario::ApprovalPopup.default_viewport(),
    );
    assert!(approval.contains("vac · interactive"));
    assert!(approval.contains("approval required"));
    assert!(approval.contains("DESTRUCTIVE"));
    assert!(approval.contains("approve once"));
    assert!(approval.contains("approve+remember"));
    assert!(approval.contains("reject with reason"));
    assert!(approval.contains("batch 1/2"));

    let runtime = operator_ui::render_operator_snapshot_text(
        operator_ui::SnapshotScenario::RuntimeJobs,
        operator_ui::SnapshotScenario::RuntimeJobs.default_viewport(),
    );
    assert!(runtime.contains("vac · interactive"));
    assert!(runtime.contains("runtime"));
    assert!(runtime.contains("autopilot ● running"));
    assert!(runtime.contains("pid 48213"));
    assert!(runtime.contains("inspect 7c0f1a"));
    assert!(runtime.contains("node progress"));

    let dashboard = operator_ui::render_operator_snapshot_text(
        operator_ui::SnapshotScenario::CapabilityDashboard,
        operator_ui::SnapshotScenario::CapabilityDashboard.default_viewport(),
    );
    assert!(dashboard.contains("TUI Capability Dashboard"));
    assert!(dashboard.contains("/capabilities"));
    assert!(dashboard.contains("Diagnostics"));
    assert!(dashboard.contains("no YAML/control-plane errors detected"));
    assert!(dashboard.contains("VALIDATION / DOCS"));
    assert!(!dashboard.contains(".vac/capabilities/tui.yml:42:13"));
    assert!(!dashboard.contains("almost_ready"));
    assert!(!dashboard.contains("metrics  ["));
    assert!(!dashboard.contains("layout  left:"));
    assert!(!dashboard.contains("right / Diagnostics"));
}

#[test]
fn operator_visual_fidelity_matrix_has_all_screenshot_sizes() {
    let matrix = operator_ui::OperatorViewport::VISUAL_FIDELITY_MATRIX;
    assert_eq!(matrix.len(), 3);
    assert!(
        matrix
            .iter()
            .any(|viewport| viewport.width == 120 && viewport.height == 36)
    );
    assert!(
        matrix
            .iter()
            .any(|viewport| viewport.width == 140 && viewport.height == 40)
    );
    assert!(
        matrix
            .iter()
            .any(|viewport| viewport.width == 180 && viewport.height == 48)
    );

    for scenario in operator_ui::SnapshotScenario::ALL {
        for viewport in matrix {
            let rendered = operator_ui::render_operator_snapshot(scenario, viewport);
            assert!(!rendered.is_empty());
            assert!(rendered.len() <= viewport.height);
            assert!(
                rendered
                    .iter()
                    .all(|line| line.chars().count() <= viewport.width),
                "{} matrix width {}x{}",
                scenario.slug(),
                viewport.width,
                viewport.height
            );
            if matches!(
                scenario,
                operator_ui::SnapshotScenario::AgentWorking
                    | operator_ui::SnapshotScenario::ApprovalPopup
                    | operator_ui::SnapshotScenario::RuntimeJobs
                    | operator_ui::SnapshotScenario::CapabilityDashboard
            ) {
                assert!(rendered.join("\n").contains('╭'));
            }
            let filename =
                operator_ui::render_operator_snapshot_filename_for_viewport(scenario, viewport);
            assert!(filename.contains(scenario.slug()));
        }
    }
}
