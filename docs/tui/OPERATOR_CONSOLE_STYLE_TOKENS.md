# Operator Console Style Tokens

This document defines the semantic style layer for the VAC terminal operator console.
The renderer remains terminal-first and snapshot-safe: the base renderer emits deterministic plain text, while `operator_style` and `operator_ui_styles` apply semantic roles.

## Roles

- `chrome`: frame, titlebar, tabbar, bottom statusline, composer chrome.
- `muted`: omitted tool lines, queued/background context, blank-space-safe helper rows.
- `accent`: active surfaces such as `runtime jobs`, `capability dashboard`, diagnostics, context usage, and tool timeline headings.
- `success`: ready/valid/passed/VIL-native state.
- `warning`: approval required, risk, policy, and guarded operations.
- `danger`: destructive bash, failed state, hard errors, and unsafe classifications.
- `user`: user prompt rows.
- `agent`: assistant/thinking/status rows.
- `status`: neutral operational text.

The ANSI snapshot gate validates that every role exists and that `strip_ansi(style_operator_text(snapshot))` is identical to the plain snapshot. This prevents styling from changing layout or terminal viewport density.
