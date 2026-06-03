# O5/O6 Three Audit Findings Closeout

Status: `SV-Done` source/static, `TV-Pending` cargo/toolchain validation.

This closeout implements the three latest audit packs as one bounded source slice:

1. **Server-bound prune / rename map**: legacy ChatGPT transport and account paths are default-off, explicit opt-in gates were added, ChatGPT product URLs were removed from local-agent user-facing errors, and provider realtime is fail-closed unless opted in.
2. **TUI performance / UX**: the benchmark harness now has explicit ignored timing tests, desired-height caching is wired into `ChatWidget`, height-only terminal resize no longer schedules full transcript reflow, `/workflow run` can execute directly from manifests when the registry run report is missing, and `/debug-config` reports rule details instead of a TODO.
3. **Control-plane spec gap**: CLI surfaces now include `vac approve`, `vac workflow list|inspect|run`, and `vac plan create|approve|execute|abandon`; CLI `vac init --interactive` records operator choices; TUI `/init` records the same operator choices and then uses plain `vac init` (no `--interactive` flag) because the TUI is already the operator-guided surface; evidence hashing now uses full canonical evidence YAML; live evidence writer supports optional Ed25519 signing via `VAC_EVIDENCE_ED25519_SIGNING_KEY_BASE64`; semantic anchor resolution skips attributes and handles multiline Rust item candidates.

## Static gate

`bash scripts/check-vac-o5o6-three-audit-findings-static.sh`

## Cargo/toolchain caveat

No `cargo`/`rustc` was available in the sandbox used for this slice. The source gates validate contracts and callsite shape, but compile correctness remains `TV-Pending`.
