# Plan 32 Evidence — Threshold documentation refresh

Date: 2026-05-28
Environment: ChatGPT sandbox source checkpoint

## Decision

Plan 32 threshold text is refreshed so hard gates are no longer described as staged warnings after Plan 30/31/33 default-path closeout.

Current default hard-error posture:

- `missing_field`
- `missing_source_domain`
- `app_server_dependency_present`
- `app_server_import_present`
- `pty_false_green`
- `message_processor_copy`
- `unsupported_control_default_defer`

## Interpretation

A real TTY pass remains operator evidence, but synthetic/no-operator PTY proof must not be reported green. `BLOCKED-OPERATOR` remains non-pass release evidence.

## Validation

- Static sweep for stale warning-level Plan 32 wording.
- YAML parse for runtime-owner capability/workflow manifests.
