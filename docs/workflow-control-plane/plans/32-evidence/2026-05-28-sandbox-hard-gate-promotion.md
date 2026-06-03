# Plan 32 hard-gate promotion — 2026-05-28

Status: complete for the default runtime-owner gate threshold.

Promoted to hard errors:

- `missing_field`
- `missing_source_domain`
- `app_server_dependency_present`
- `app_server_import_present`
- `message_processor_copy`
- `pty_false_green`
- `unsupported_control_default_defer`

Allowed non-green evidence:

- `app_server_compat_defer_present` remains a warning only when optional/non-default compatibility is explicitly marked with metadata and does not affect the default product path.
- `VAC_RUNTIME_OWNER_NONDEFAULT_DEFER_ACCEPTED: plan30-owner-native-default-parity` marks non-default command-bus fail-closed sentinels that are not used by the default TUI path.
