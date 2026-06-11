# VAC Runtime v1.9 TV-Clean Handoff

## Scope

Workspace: `/home/emp/Documents/VAC/vastar-agentic-cli`
Branch: `fase-a/runtime-patch-gate`
Remote: `https://github.com/mibrohimsulaeman-a11y/VAC.git`

This handoff records the post-State-9 TV closure pass, including runtime P0 compliance gates, cargo gates, TUI lifecycle repair, and source-vs-generated-state hygiene.

## Root-cause summary

### Runtime/control-plane debt closed

The State-9 source tree had static-compliant runtime scaffolding but still carried TV debt in Rust compile/test gates, gate drift markers, legacy config handling, TUI lifecycle integration, and documentation/package hygiene. The closure work fixed these without claiming L2 enforcement.

### TUI lifecycle debt closed

The canonical VAC operator renderer was previously rendering a mockup-like shell while bypassing several existing interactive lifecycle affordances. The visible failures were:

- slash command dropdown and execution were not visible/usable from the canonical renderer,
- Shift+Tab was not mapped to the plan/review lifecycle,
- input submission could appear inert because the renderer behaved like static state screens,
- top mockup tabs were copied into the real product surface,
- context and tool timeline polluted the main feed instead of living in a toggled side panel,
- first-launch and idle looked like separate screens instead of one continuous feed.

The TUI patch keeps runtime behavior wired through existing `InputEvent`/`OutputEvent` paths while changing only the canonical render shell and key mapping:

- `KeyCode::BackTab` maps to `InputEvent::TogglePlanReview`.
- Slash/file helper dropdown is rendered by the canonical operator renderer.
- `/context`, `/timeline`, and `/tools` open the right context/tool panel.
- The main feed stays clean: conversation, thinking, streaming, approvals, input, and output.
- Tool timeline and context gauge moved to the right panel toggled by `Ctrl+Y` or slash commands.
- Top mock tabs `chat/runtime/review/workbench/mcp` were removed from the canonical renderer.
- Route changes now occur through command execution state, not merely by typing a slash-prefixed string.

## Gates

| Gate | Status | Evidence |
|---|---:|---|
| Binary rebuild | PASS | `/home/emp/Documents/VAC-checkpoints/vac-tui-rebuild-20260611T114739Z/cargo_build_vac.log` |
| Binary smoke `vac --version` | PASS | `/home/emp/Documents/VAC-checkpoints/vac-tui-rebuild-version.txt` |
| Targeted TUI/CLI gates | PASS | `/home/emp/Documents/VAC-checkpoints/vac-tui-final-20260611T114809Z/summary.tsv` |
| Static/runtime hygiene replay | PASS | `/home/emp/Documents/VAC-checkpoints/vac-final-after-tui-20260611T114836Z/static-runtime-summary.tsv` |
| Full cargo metadata/fmt/check/clippy/test | PASS | `/home/emp/Documents/VAC-checkpoints/vac-final-cargo-after-tui-20260611T115041Z/cargo-summary.tsv` |
| Automated TUI PTY smoke | PASS | `/home/emp/Documents/VAC-checkpoints/vac-tui-pty-smoke-20260611T115958Z/pty.boolean.txt` |
| Git diff whitespace | PASS | verified by `git diff --check` |
| Root metadata files | PASS | `SANDBOX_HANDOFF.md`, `CHECKPOINT_MANIFEST.json`, and `LOCAL_WORKSPACE_README.md` absent |
| Generated-state tracked check | PASS | `.vac/cache`, `.vac/index`, `.vac/assessment`, `.vac/exports`, `.vac/db`, `.vac/registry/evidence`, `.vac/registry/runtime` tracked count remains zero except source allowlisted files |

## Latest full cargo summary

```text
metadata 0 /home/emp/Documents/VAC-checkpoints/vac-final-cargo-after-tui-20260611T115041Z/metadata.log
fmt      0 /home/emp/Documents/VAC-checkpoints/vac-final-cargo-after-tui-20260611T115041Z/fmt.log
check    0 /home/emp/Documents/VAC-checkpoints/vac-final-cargo-after-tui-20260611T115041Z/check.log
clippy   0 /home/emp/Documents/VAC-checkpoints/vac-final-cargo-after-tui-20260611T115041Z/clippy.log
test     0 /home/emp/Documents/VAC-checkpoints/vac-final-cargo-after-tui-20260611T115041Z/test.log
```

## Latest TUI PTY smoke summary

```text
entered_alt_screen=True
exited_alt_screen=True
entered_tui=True
slash_echo_or_help=True
context_or_timeline=True
text_echo=True
mock_tabs_absent=True
bytes_positive=True
```

## Runtime/spec honesty

- P0 remains L1 cooperative runtime enforcement.
- No L2 claim is made.
- Generated state is not source authority.
- Provider-live end-to-end model response quality remains environment-dependent and is not claimed beyond the PTY/input lifecycle smoke above.

## Operator retest script

```bash
cd /home/emp/Documents/VAC/vastar-agentic-cli
VAC_SKIP_WARDEN=1 ./vac-rs/target/debug/vac --theme dark
```

Retest checklist:

```text
1. Top mock tabs chat/runtime/review/workbench/mcp are absent.
2. Typing / opens slash dropdown.
3. /help executes as a command.
4. /context, /timeline, or /tools opens the right panel.
5. Ctrl+Y toggles the right context/tool panel.
6. Shift+Tab enters plan/review lifecycle.
7. Plain text + Enter submits into the feed.
8. Tool timeline/context are not embedded in the main feed.
```

## Addendum — real TUI lifecycle E2E repair

A follow-up manual test exposed remaining lifecycle defects that the first PTY smoke did not catch:

- first-launch stayed visible indefinitely instead of transitioning to idle,
- submitting text could return to first-launch/empty feed instead of keeping the message in the feed,
- `/model` could open/intercept input without a visible popup in the canonical renderer path,
- the right context/tool area had no collapsed tab/rail, so it was not discoverable or visibly toggleable.

The second repair patched the actual lifecycle path:

- first-launch is now only a startup hydration state for the first ~3 seconds of an empty session,
- submitted user messages force the canonical route back to the chat/feed route and close blocking popups/dropdowns,
- canonical renderer now renders model/profile/rulebook/shortcut/file-change/auto-approve overlays instead of swallowing them,
- invalid or empty model-switcher selections now close the popup and return control to the composer,
- a collapsed right-side `ctx/tls` rail is always visible on wide terminals; `Ctrl+Y`, `/context`, `/timeline`, and `/tools` open the full right panel.

Second-pass evidence:

```text
checkpoint:
  /home/emp/Documents/VAC-checkpoints/vac-before-tui-e2e-lifecycle-fix-20260611T123129Z.tar.gz
  sha256 88f3dae96f1a2b168e13012c66d9a14294978a4d046744f2b10ff22527e7c742

targeted TUI gates:
  /home/emp/Documents/VAC-checkpoints/vac-tui-e2e-lifecycle-20260611T125003Z/summary.tsv

real lifecycle PTY smoke:
  /home/emp/Documents/VAC-checkpoints/vac-tui-real-life-20260611T125213Z/pty.boolean.txt

full cargo compact logs:
  /tmp/vac_meta_real_tui.log
  /tmp/vac_fmt_real_tui.log
  /tmp/vac_check_real_tui.log
  /tmp/vac_clippy_real_tui.log
  /tmp/vac_test_real_tui.log

static/runtime logs:
  /tmp/sv_rtui.log
  /tmp/tui_canon_rtui.log
  /tmp/tui_hard_rtui.log
  /tmp/runtime_realpath_rtui.log
  /tmp/checkpoint_rtui.log
```

Second-pass PTY summary:

```text
entered_alt_screen=True
exited_alt_screen=True
entered_tui=True
idle_after_timer=True
rail_or_panel=True
model_did_not_lock_composer=True
context_panel=True
mock_tabs_absent=True
bytes_positive=True
```
