# PRD — TUI Action Registry, Recorder, and Replay

## Overview

VAC needs a consistent action/keybinding model and a way to reproduce TUI flows.

This PRD covers:

```text
vac.action_registry
vac.keybindings
vac.overlays
vac.file_picker
vac.recorder
vac.replay
vac.recent_commands
```

## Product goal

VAC TUI should be discoverable, keyboard-consistent, and testable through real operator flows.

## User problems

- TUI keyboard behavior can become inconsistent.
- Overlay focus bugs are hard to reproduce.
- PTY dogfood needs repeatable input traces.
- Users need fast access to files and recent actions.

## Capability model

| Capability | Purpose | Priority |
|---|---|---:|
| `vac.action_registry` | Canonical action ids and metadata. | P1/P2 |
| `vac.keybindings` | Keybinding map and help surface. | P1/P2 |
| `vac.overlays` | Focus, escape, render-order contract. | P1/P2 |
| `vac.file_picker` | Project file selection surface. | P2 |
| `vac.recorder` | Record TUI input sessions. | P2 |
| `vac.replay` | Replay TUI input traces for regression tests. | P2 |
| `vac.recent_commands` | Reuse recent actions/prompts. | P2 |

## Action registry requirements

Every TUI action should have:

```yaml
action:
  id: action.submit_prompt
  title: Submit prompt
  default_key: Enter
  surface: input
  enabled_when: input_has_text
```

Actions should power slash help, palette, keybinding help, and replay metadata.

## Keybinding requirements

Keybindings should be:

- discoverable,
- conflict-checked,
- surface-aware,
- testable,
- documented in help.

Plain text input must not be stolen by global shortcuts when input owns keyboard.

## Overlay requirements

Overlay stack must define:

- focus owner,
- escape behavior,
- render order,
- input capture rules,
- restore focus behavior.

## File picker requirements

File picker should support:

- fuzzy path search,
- recent files,
- changed files,
- task-relevant files,
- keyboard navigation,
- cancel/confirm behavior.

## Recorder/replay requirements

Recorder should capture:

- key events,
- paste events,
- timing bucket or deterministic ordering,
- surface focus state,
- terminal size if needed,
- expected visible assertions.

Replay should support PTY-level regression gates where feasible.

## TUI surfaces

```text
/
/palette
/files
/recent
/status
```

## Acceptance criteria

### MVP

- Keybindings are discoverable.
- Input focus prevents accidental global shortcut capture.
- Action registry backs help/palette entries.

### Quality gate

- TUI prompt submit, slash command list, approval, and clean exit can be replayed or dogfooded.
- Recorder/replay does not become a second TUI runtime.

### UX

- User can find commands quickly.
- User can pick files without typing full paths.
- TUI bugs can be reproduced with traces.
