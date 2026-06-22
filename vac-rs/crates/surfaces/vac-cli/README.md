# vac-cli surface crate

`vac-cli` is the native CLI surface for VAC. It should route operator actions into the VAC v1.9 control plane and runtime crates; it should not be treated as the owner of runtime authority.

## Current authority model

- Authoring: `.vac/{capabilities,policies,workflows,surfaces,specs}/*.yaml`
- Execution: `.vac/registry/compiled/**/*.json`
- Audit: `.vac/registry/{evidence,approvals,spec-sync}/**/*.json`
- Runtime jobs: `.vac/registry/runtime/jobs.json`

CLI commands may edit authoring manifests through typed flows, but agent execution and policy decisions must read compiled JSON snapshots.

## Default runtime

Local control-plane execution is the default. Optional service boundaries are explicit:

- `vac-rs/crates/runtime/vac-broker` for mediated broker/service execution.
- `vac-rs/crates/integrations/vac-remote-service` for remote adapter flows.
- `vac-rs/crates/integrations/vac-messaging-gateway` for channel integration.

Do not describe these optional crates as the default runtime.

## Config sample

```toml
[profiles.default]
api_endpoint = "https://api.vastar.ai/vac" # optional remote endpoint
model = "anthropic/claude-sonnet-4-5"
allowed_tools = ["view", "search_docs", "load_skill", "local_code_search"]
auto_approve = ["view", "search_docs", "load_skill"]

[profiles.default.warden]
enabled = false
```

## Validation

Source-level checks:

```bash
bash scripts/vac-static-gate.sh
python3 scripts/check-docs-current-state.py
python3 scripts/vac-runtime-agent-e2e-sv.py
```

Cargo gates remain TV-Pending unless explicitly run in the local environment.

## Autopilot retrospect schedule

Use the bundled retrospect skill as the canonical prompt for a nightly local retrospective:

```bash
vac autopilot schedule add --name retrospect --cron "0 3 * * *" --prompt "$(vac ak skill retrospect)"
```

This is a local cooperative L1 workflow helper. It does not claim L2 broker/OS enforcement.

