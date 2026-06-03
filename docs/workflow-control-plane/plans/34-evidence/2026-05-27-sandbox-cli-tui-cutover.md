# Plan 34 Sandbox Evidence — CLI/TUI prompt cutover

Date: 2026-05-27

## Slice

This slice wires the zero-config project workspace contract into the user-facing CLI/TUI startup path.

## Code changes

- `vac_core::project_workspace::ProjectWorkspaceStartupNotice` renders a concise first-run warning/preflight notice for missing `.vac` user projects.
- `vac_core::project_workspace::project_workspace_startup_notice(root)` exposes a side-effect-free notice only when bootstrap is available and ordinary prompts may continue.
- `vac-rs/cli/src/main.rs` emits a CLI preflight notice before launching the interactive TUI when the selected `--cd`/current root has no `.vac`.
- `vac-rs/tui/src/lib.rs` appends the same zero-config notice to TUI `startup_warnings`, so the runtime event surface can show it without requiring `.vac`.
- `vac doctor project-workspace <path> --bootstrap-soft --yes` is now the explicit approval-gated materialization path for `.vac/profile.yaml`, `.vac/.gitignore`, and local-only boundaries.
- `vac doctor project-workspace <path> --bootstrap-soft` without `--yes` renders the preview and exits as approval-required without writing files.

## Guarantees

- Missing `.vac` in arbitrary user projects does not block ordinary prompt submission.
- Disk writes require explicit approval via `--bootstrap-soft --yes`.
- Strict product-repo behavior remains controlled by `--strict-product-repo` and is not weakened.
