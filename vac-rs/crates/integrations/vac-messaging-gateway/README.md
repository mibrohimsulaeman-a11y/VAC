# vac-messaging-gateway

Optional messaging/channel gateway for VAC. This crate is not the default product runtime.

Runtime jobs and operator TUI state are still sourced from `.vac/registry/runtime/jobs.json` and compiled registry snapshots. Any channel action must pass policy and approval rules before execution.
