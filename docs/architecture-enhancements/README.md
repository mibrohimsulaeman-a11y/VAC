> VAC v1.5 status note: this is historical/reference material. Current runtime authority is `.vac/registry/compiled/`; default execution is local control-plane. Server/gateway paths are optional `vac-broker`, `vac-remote-service`, and `vac-messaging-gateway` boundaries, not the default setup.

# Readme

Status: superseded pre-v1.5 architecture note.

This file previously described donor/upstream-style architecture experiments. The current VAC development state is the v1.5 control-plane architecture:

- `.vac/` authoring manifests compile into `.vac/registry/compiled/*.json` runtime truth.
- `vac-rs/crates/runtime/vac-agent-loop` owns the bounded agent runtime.
- `vac-rs/crates/runtime/vac-broker` is optional and not the default product runtime.
- `vac-rs/crates/integrations/vac-messaging-gateway` and `vac-rs/crates/integrations/vac-remote-service` are optional integration boundaries.
- Setup docs must use local source/checkpoint instructions, not stale external source or registry links.

Current references:

```text
README.md
GETTING-STARTED.md
docs/workflow-control-plane/VAC_CURRENT_DEVELOPMENT_STATE.md
docs/workflow-control-plane/VAC_RUNTIME_V1_5_BOUND_AGENT.md
```
