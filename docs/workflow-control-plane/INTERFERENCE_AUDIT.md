# VAC workflow-control-plane interference audit

This audit lists remaining source areas that interfere with the migration to a VAC-native workflow-control-plane repository pattern.

The criterion here is not only whether code is reachable. Some code is reachable only because the current product CLI still exposes old direct subcommands and direct dependencies. Those areas should be decoupled from `vac-cli` and either moved behind `.vac` capability/workflow manifests, quarantined as advanced/internal, or deleted.

## Current root state

The root has already been reduced to a VAC-native shape:

```text
.vac/
vac-cli/
vac-rs/
donor/vac/
docs/workflow-control-plane/
docs/donor-migration/
```

Cargo metadata is valid after the previous cleanup:

```text
workspace members: 105
```

Previously removed as proven orphan/non-product:

```text
vac-rs/agent-graph-store
vac-rs/core-api
vac-rs/debug-client
vac-rs/execpolicy-legacy
```

## Main finding

The biggest remaining interference is not orphan files. It is `vac-rs/cli/src/main.rs`, which still acts as a monolithic legacy command multiplexer.

It directly exposes or depends on many subsystems that should become workflow/capability-managed or be deleted:

```text
app-server
cloud tasks
responses API proxy
stdio-to-uds
exec-server
MCP server
plugin marketplace
login/chat auth stack
debug tools
rollout trace
remote app server mode
```

This keeps old product assumptions alive and prevents `.vac/` from becoming the primary control plane.

## Delete now candidates

These are high-confidence candidates because they are not required for root workflow-control-plane migration and do not need to remain in product source.

| Area | Action | Reason |
|---|---|---|
| `vac-rs/default.nix` | delete | Nix path was already removed from root; product path is Cargo + `.vac`. |
| `vac-rs/config.md` | delete or replace with `.vac` config plan | Old standalone config docs conflict with workflow-control-plane docs. |
| `vac-rs/scripts/` | delete unless a script is referenced by Cargo/package build | Old ad-hoc scripts should become maintenance workflows. |
| `vac-rs/deny.toml` | hold or delete after deciding whether cargo-deny is a release workflow | If kept, it should be invoked by `.vac/workflows/maintenance.release-gate.yaml`. |
| `vac-rs/clippy.toml` | keep for Rust quality | Not a blocker. |
| `vac-rs/rustfmt.toml` | keep for Rust quality | Not a blocker. |
| `vac-rs/.config/` | inspect/delete if only nextest/dev tooling | Dev config should become maintenance workflow only if used. |

## Decouple from `vac-cli`, then delete/quarantine

These are currently direct or transitive product dependencies. Do not delete by `rm -rf` until `vac-rs/cli/Cargo.toml` and `vac-rs/cli/src/main.rs` are simplified.

### 1. App server family

```text
vac-rs/app-server
vac-rs/app-server-client
vac-rs/app-server-protocol
vac-rs/app-server-test-client
vac-rs/app-server-transport
vac-rs/stdio-to-uds
```

Current direct CLI usage:

```text
Subcommand::AppServer
DebugAppServerCommand
remote app server flags
protocol/schema generation
stdio-to-uds proxy
```

Why it interferes:

- Preserves app-server-first architecture.
- Pulls a large protocol/schema tree into the product CLI.
- Keeps remote-control assumptions outside the `.vac` workflow model.
- Contributes to the current websocket compile blocker via related API stack.

Target action:

1. Remove `AppServer`, `DebugAppServer`, app-server schema generation, remote app-server flags, and `stdio-to-uds` from the default `vac-cli` command surface.
2. If any remote/server capability is needed later, reintroduce it as a `.vac/capabilities/remote.yaml` and `.vac/workflows/product.remote.*.yaml` flow.
3. Then delete or quarantine the app-server family.

### 2. Cloud tasks family

```text
vac-rs/cloud-tasks
vac-rs/cloud-tasks-client
vac-rs/cloud-tasks-mock-client
vac-rs/cloud-requirements
```

Current direct CLI usage:

```text
Subcommand::Cloud
vac_cloud_tasks::run_main(...)
```

Why it interferes:

- Cloud task browsing is not part of the initial local workflow-control-plane.
- It introduces product behavior outside `.vac/workflows`.

Target action:

1. Remove `Subcommand::Cloud` from root CLI.
2. Delete direct dependency from `vac-rs/cli/Cargo.toml`.
3. Keep only if converted later into `vac.cloud` capability with TUI surface and policy.

### 3. Responses/realtime API family

```text
vac-rs/responses-api-proxy
vac-rs/realtime-webrtc
vac-rs/vac-api
vac-rs/vac-client
vac-rs/vac-backend-openapi-models
vac-rs/backend-client
```

Current direct CLI usage:

```text
Subcommand::ResponsesApiProxy
vac_responses_api_proxy::run_main(...)
```

Why it interferes:

- Maintains API/proxy-centric product shape.
- Pulls WebSocket/WebRTC surface into the root CLI.
- Current build blocker is in `vac-rs/vac-api` websocket extension/config code.

Target action:

1. Decide if VAC initial product needs this API/proxy stack.
2. If not needed for local TUI/workflow-control-plane, remove `ResponsesApiProxy` and direct dependencies.
3. Then delete/quarantine proxy/realtime/API crates.
4. If needed, model as `vac.api` capability and keep only after build is fixed.

### 4. Exec server / daemon-like services

```text
vac-rs/exec-server
vac-rs/stdio-to-uds
vac-rs/uds
```

Current direct CLI usage:

```text
Subcommand::ExecServer
Subcommand::StdioToUds
```

Why it interferes:

- Creates hidden service/runtime paths outside workflow control plane.
- Makes `vac-cli` a multi-service binary instead of one workflow-native product CLI.

Target action:

1. Remove direct service subcommands from product CLI.
2. Keep `vac-rs/exec` if it powers local `vac exec` or TUI agent execution.
3. Delete server/UDS bridge crates after dependency removal.

### 5. Plugin marketplace stack

```text
vac-rs/core-plugins
vac-rs/plugin
vac-rs/cli/src/marketplace_cmd.rs
```

Current direct CLI usage:

```text
Subcommand::Plugin
PluginSubcommand::Marketplace
```

Why it interferes:

- Adds plugin marketplace as a separate control plane.
- Competes with `.vac/capabilities` and `.vac/workflows`.

Target action:

1. Remove plugin marketplace command from initial product CLI.
2. Later reintroduce plugin-like behavior only as `.vac/capabilities/plugins.yaml` if needed.
3. Delete/quarantine marketplace code after decoupling.

### 6. Login/chat identity stack

```text
vac-rs/login
vac-rs/chatgpt
vac-rs/device-key
vac-rs/aws-auth
vac-rs/connectors
```

Current direct CLI usage:

```text
Subcommand::Login
Subcommand::Logout
run_login_with_chatgpt
run_login_with_device_code
agent identity auth
```

Why it interferes:

- Product identity is VAC/Vastar; auth provider naming and UX should be redesigned under `.vac/capabilities/auth.yaml`.
- Current auth flow is not workflow-control-plane governed.

Target action:

1. Keep only minimal API-key/env auth needed for root `vac` to run.
2. Remove or hide chat/device/browser login flows from initial product CLI.
3. Reintroduce auth readiness as `/capabilities` and `.vac/capabilities/auth.yaml`.

### 7. Analytics, feedback, rollout, telemetry

```text
vac-rs/analytics
vac-rs/feedback
vac-rs/rollout
vac-rs/rollout-trace
vac-rs/otel
vac-rs/response-debug-context
```

Current direct CLI usage:

```text
Debug trace reduce
rollout trace bundle replay
feedback/analytics transitive runtime hooks
```

Why it interferes:

- Adds telemetry/rollout mechanics before core product control plane is stable.
- Creates product behavior that is not declared by `.vac/policies`.

Target action:

1. Remove debug trace reduce and rollout replay from initial CLI.
2. Keep only what is strictly required by `vac-core` until dependency decoupling.
3. Later add observability as `vac.observability` capability, not rollout-specific code.

### 8. Provider integrations not needed for initial product

```text
vac-rs/lmstudio
vac-rs/ollama
vac-rs/model-provider-info
vac-rs/models-manager
```

Current usage:

```text
Debug models
model catalog
provider management
```

Why it interferes:

- Provider catalog can stay, but it should be represented in `.vac/capabilities/model.yaml`.
- Debug/provider tooling should not dominate product CLI before workflow-control-plane exists.

Target action:

1. Keep minimal model provider path needed by chat/TUI.
2. Move provider readiness into capability dashboard.
3. Delete provider integrations only after confirming not needed by chat/TUI.

## Keep/adapt

These are relevant to VAC product and should not be deleted now.

| Area | Reason |
|---|---|
| `vac-rs/tui` | Product TUI. |
| `vac-rs/core` | Product runtime core. |
| `vac-rs/cli` | Product CLI entrypoint, but must be simplified. |
| `vac-rs/apply-patch` | Needed for approval/review/apply UX. |
| `vac-rs/exec` | Likely needed for local command/tool execution. |
| `vac-rs/tools` | Needed for agent tool execution. |
| `vac-rs/sandboxing`, `linux-sandbox`, `windows-sandbox-rs` | Needed for policy/safety. |
| `vac-rs/config`, `state`, `thread-store` | Likely needed for sessions/config/state. |
| `vac-rs/protocol` | Likely needed by TUI/core event protocol. |
| `vac-rs/mcp`, `vac-rs/rmcp-client` | Keep if MCP remains product capability. |
| `vac-rs/memories/*` | Candidate for workflow-control-plane capability. |
| `vac-rs/skills`, `core-skills` | Candidate capability, but should become manifest-driven. |
| `vac-rs/file-search`, `file-system`, `git-utils`, `hooks` | Likely product tool/context functionality. |

## Recommended execution order

### Step A — simplify CLI command surface

Edit `vac-rs/cli/src/main.rs` so initial product CLI exposes only:

```text
vac                launch TUI
vac exec           non-interactive prompt
vac review         review flow if currently functional
vac apply          apply patch if currently functional
vac resume         session resume if currently functional
vac completion     optional
vac workflow       planned after control plane
```

Everything else should be removed, hidden behind feature flags, or moved to `.vac/workflows` after the control plane exists.

### Step B — remove direct dependencies from `vac-rs/cli/Cargo.toml`

First decouple these direct deps:

```text
vac-app-server
vac-app-server-protocol
vac-app-server-test-client
vac-cloud-tasks
vac-responses-api-proxy
vac-mcp-server
vac-stdio-to-uds
vac-rollout-trace
vac-core-plugins
```

Then rerun:

```bash
cd vac-rs
cargo metadata --no-deps --format-version 1
cargo check -p vac-surface-cli
```

### Step C — delete newly unreachable crates

After Step B, rerun reachability audit from `vac-cli`. Delete crates that become unreachable.

### Step D — model remaining product capabilities in `.vac`

Before keeping any old subsystem, add capability manifest:

```text
.vac/capabilities/<domain>.yaml
```

If no manifest and no TUI/CLI surface exists, the subsystem is not product code.

## Current conclusion

Do not delete more crates blindly yet. The next high-impact cleanup is to simplify `vac-rs/cli/src/main.rs` and `vac-rs/cli/Cargo.toml`. That will make many old subsystems unreachable, then they can be deleted safely.
