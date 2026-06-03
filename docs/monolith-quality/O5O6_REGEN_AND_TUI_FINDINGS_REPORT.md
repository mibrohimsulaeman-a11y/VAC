# O5/O6 Regen + TUI Findings Execution Report

Status: SV-Done / TV-Pending cargo.

Implemented from latest findings:

- Regenerated derivable `.vac/.init` source projection.
- Pruned `docs/donor-migration` from living docs while preserving registry/code-backed donor provenance.
- Added `maintenance.regen-control-plane` workflow and gate.
- Added dependency-free TUI performance harness entrypoint.
- Improved markdown render cache capacity/data structure.
- Reduced operator console idle polling.
- Removed dead-end memory debug commands from visible slash palette.
- Added workflow action hints and realtime/settings unavailable guidance.
- Added startup profiling spans behind `VAC_TUI_PROFILE_STARTUP`.

Cargo/build/test/clippy remain `TV-Pending` until a Rust toolchain is available.
