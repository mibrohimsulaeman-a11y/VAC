# VAC runtime crates

Runtime crates implement the local-control-plane-first execution layer.

| Crate | Role | Default runtime? |
|---|---|---|
| `vac-agent-loop` | Bounded agent loop contract and E2E harness | yes |
| `vac-runtime-jobs` | one-shot/cron/filewatch registry records | yes |
| `vac-shell-approval` | approval state and destructive command classification | yes |
| `vac-exec` | structured execution adapter | yes, behind gates |
| `vac-watch` | filewatch support | yes, when configured |
| `vac-autopilot` | scheduled local runtime monitor | optional |
| `vac-broker` | mediated service/broker boundary for L2 work | optional |

Server-style functionality is not the default product runtime. It is bounded behind `vac-broker` when enabled by policy and compiled registry state.
