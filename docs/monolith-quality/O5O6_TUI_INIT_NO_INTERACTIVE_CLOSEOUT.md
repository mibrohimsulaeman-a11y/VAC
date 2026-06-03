# O5/O6 TUI VAC-Init No-Interactive Closeout

## Root cause

The previous three-audit closeout introduced `vac init --interactive` as the CLI spelling for operator-guided VAC-Init choices. That is correct for a manual CLI invocation, but it is the wrong UX contract for the TUI: the TUI is already the interactive/operator surface, so a TUI-triggered init must not require users to type or route through `--interactive`.

## Change

- `/init` in the TUI now prepares `.vac/.init/operator_choices.yaml` directly with `surface: tui`, `source: tui_slash_init`, and `requires_cli_interactive_flag: false`.
- The user-facing hint now tells operators to run plain `vac init` from the TUI/local command surface, without `--interactive`.
- The old `/init` AGENTS.md overwrite guard was removed from this path; existing `AGENTS.md` remains untouched while VAC-Init operator choices are prepared.
- A source/static gate (`scripts/check-vac-tui-init-no-interactive-static.sh`) enforces the no-`vac init --interactive` TUI contract.

## Validation status

SV-Done: source/static gate coverage and artifact packaging.

TV-Pending: cargo check/test/clippy and live TUI smoke; the sandbox does not provide `cargo`/`rustc`.
