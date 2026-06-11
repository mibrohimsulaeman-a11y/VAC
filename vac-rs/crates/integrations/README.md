# VAC integrations

Integration crates are optional adapters around the local VAC control-plane runtime. They must not become the default product runtime.

| Crate | Role |
|---|---|
| `vac-mcp-client` | MCP client adapter |
| `vac-mcp-server` | local MCP tool surface adapter |
| `vac-mcp-proxy` | optional MCP proxy adapter |
| `vac-messaging-gateway` | optional Slack/Telegram/Discord style channel gateway |
| `vac-remote-service` | optional remote-service adapter |
| `vac-telemetry` | optional telemetry adapter |

All integrations remain governed by compiled `.vac/registry/compiled/*.json`, policies, and approval gates.
