# Plan 34 evidence — sandbox bootstrap core

Date: 2026-05-27

## Scope

Implemented the Rust-owned core contract for Plan 34C–34E without claiming full CLI/TUI zero-config behavior.

## Code changes

- `vac-rs/core/src/project_workspace.rs`
  - Added `ProjectWorkspaceBootstrapPlan` for side-effect-free first-run preview text.
  - Added `build_soft_workspace_bootstrap_plan(root)` for reviewable bootstrap UX copy and disk-change summary.
  - Added `materialize_soft_workspace_bootstrap(root, approved)` with an explicit approval gate.
  - Denied bootstrap returns `approval_required` and writes nothing.
  - Approved bootstrap creates `.vac/profile.yaml`, `.vac/.gitignore`, and local-only directories only.
  - Existing strict workspaces are refused to avoid silent downgrade/overwrite.
  - `ProjectWorkspaceReport::render_text()` now embeds a bootstrap preview when `.vac` is missing for arbitrary user projects.
  - `load_project_workspace_report_with_options(root, strict_product_repo)` and `vac doctor project-workspace --strict-product-repo` expose a strict product-repo error path with no false-green bootstrap preview.

## Validation

```text
rustfmt --edition 2024 vac-rs/core/src/project_workspace.rs: passed
rustc --edition 2024 --test vac-rs/core/src/project_workspace.rs: passed
project_workspace tests: 12 passed, 0 failed
```

## Non-claims

- No interactive TUI prompt-submission cutover is claimed.
- No automatic `.vac` creation is claimed.
- No strict manifest generation is claimed.
- No Plan 31/33 protocol-compatibility retirement is claimed.
