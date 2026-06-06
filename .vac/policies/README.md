# Policies (authored)

Fail-closed by default: when a policy, schema, or ownership decision is unclear, the action is BLOCKED.

- `default-local.yaml` — baseline local decision set.
- `filesystem.yaml` — read/write/delete scoped to project vs workspace.
- `network.yaml` — default deny; HTTPS approval-gated; plain HTTP & tunnels denied.
- `tools.yaml` — tool invocation; credential/secret tools denied.
- `approval.yaml` — session/checkpoint writes & credential reads.
- `evidence-signing.yaml` — evidence signing requirement (runner-emitted records).
