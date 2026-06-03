# Legal and Build Release Blockers

Status: tracked, not release-cleared.

## Legal attribution

`THIRD_PARTY_NOTICES.md` is the repository-level attribution file. It records the upstream OpenAI Codex CLI Apache-2.0 attribution and source-specific license-header preservation rules.

## Offline build truth

The sandbox source artifact excludes `vac-rs/vendor/`. Therefore offline Cargo build/test gates are `NotEvaluated` unless a vendor bundle is provided and the command is actually executed.

## Registered but not executed

```text
cargo deny check licenses      -> NotEvaluated
cargo deny check advisories    -> NotEvaluated
cargo deny check bans          -> NotEvaluated
cargo about generate           -> NotEvaluated
cargo metadata --offline       -> NotEvaluated unless vendor is present
```

No release artifact may convert these to `Pass` without attaching the command output.
