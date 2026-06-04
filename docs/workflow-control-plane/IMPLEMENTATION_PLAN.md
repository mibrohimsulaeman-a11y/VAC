# Workflow Control Plane Implementation Plan

The active implementation path is the root VAC product:

- CLI: `vac-rs/crates/surfaces/cli`
- TUI: `vac-rs/crates/surfaces/tui`
- Control plane: `vac-rs/crates/control-plane/control-plane`
- Manifests: `.vac/capabilities`, `.vac/policies`, `.vac/surfaces`, `.vac/workflows`

Production gates should stay reachable from `vac doctor ...`, `vac workflow ...`, or the root TUI.

