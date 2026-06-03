# O5.4 Workspace Consolidation Report

Status: Planned / NotEvaluated

Workspace member scan detected `72` direct crate directories with `Cargo.toml` under `vac-rs/*`.

No workspace merge is executed in this slice because crate consolidation without a build gate is unsafe.

## Member inventory sample

```json
[
  {
    "crate_dir": "agent-identity",
    "rs_bytes": 27832,
    "cargo_toml": "vac-rs/agent-identity/Cargo.toml"
  },
  {
    "crate_dir": "analytics",
    "rs_bytes": 191805,
    "cargo_toml": "vac-rs/analytics/Cargo.toml"
  },
  {
    "crate_dir": "ansi-escape",
    "rs_bytes": 2164,
    "cargo_toml": "vac-rs/ansi-escape/Cargo.toml"
  },
  {
    "crate_dir": "app-server",
    "rs_bytes": 2830391,
    "cargo_toml": "vac-rs/app-server/Cargo.toml"
  },
  {
    "crate_dir": "app-server-client",
    "rs_bytes": 54644,
    "cargo_toml": "vac-rs/app-server-client/Cargo.toml"
  },
  {
    "crate_dir": "app-server-protocol",
    "rs_bytes": 7669,
    "cargo_toml": "vac-rs/app-server-protocol/Cargo.toml"
  },
  {
    "crate_dir": "app-server-transport",
    "rs_bytes": 309248,
    "cargo_toml": "vac-rs/app-server-transport/Cargo.toml"
  },
  {
    "crate_dir": "apply-patch",
    "rs_bytes": 156543,
    "cargo_toml": "vac-rs/apply-patch/Cargo.toml"
  },
  {
    "crate_dir": "arg0",
    "rs_bytes": 19500,
    "cargo_toml": "vac-rs/arg0/Cargo.toml"
  },
  {
    "crate_dir": "async-utils",
    "rs_bytes": 2044,
    "cargo_toml": "vac-rs/async-utils/Cargo.toml"
  },
  {
    "crate_dir": "aws-auth",
    "rs_bytes": 12257,
    "cargo_toml": "vac-rs/aws-auth/Cargo.toml"
  },
  {
    "crate_dir": "backend-client",
    "rs_bytes": 42963,
    "cargo_toml": "vac-rs/backend-client/Cargo.toml"
  },
  {
    "crate_dir": "chatgpt",
    "rs_bytes": 25131,
    "cargo_toml": "vac-rs/chatgpt/Cargo.toml"
  },
  {
    "crate_dir": "cli",
    "rs_bytes": 189497,
    "cargo_toml": "vac-rs/cli/Cargo.toml"
  },
  {
    "crate_dir": "code-mode",
    "rs_bytes": 121082,
    "cargo_toml": "vac-rs/code-mode/Cargo.toml"
  },
  {
    "crate_dir": "collaboration-mode-templates",
    "rs_bytes": 280,
    "cargo_toml": "vac-rs/collaboration-mode-templates/Cargo.toml"
  },
  {
    "crate_dir": "config",
    "rs_bytes": 444683,
    "cargo_toml": "vac-rs/config/Cargo.toml"
  },
  {
    "crate_dir": "connectors",
    "rs_bytes": 31776,
    "cargo_toml": "vac-rs/connectors/Cargo.toml"
  },
  {
    "crate_dir": "core",
    "rs_bytes": 8657371,
    "cargo_toml": "vac-rs/core/Cargo.toml"
  },
  {
    "crate_dir": "core-plugins",
    "rs_bytes": 619338,
    "cargo_toml": "vac-rs/core-plugins/Cargo.toml"
  },
  {
    "crate_dir": "core-skills",
    "rs_bytes": 235755,
    "cargo_toml": "vac-rs/core-skills/Cargo.toml"
  },
  {
    "crate_dir": "device-key",
    "rs_bytes": 57129,
    "cargo_toml": "vac-rs/device-key/Cargo.toml"
  },
  {
    "crate_dir": "exec",
    "rs_bytes": 140731,
    "cargo_toml": "vac-rs/exec/Cargo.toml"
  },
  {
    "crate_dir": "exec-server",
    "rs_bytes": 477901,
    "cargo_toml": "vac-rs/exec-server/Cargo.toml"
  },
  {
    "crate_dir": "execpolicy",
    "rs_bytes": 84676,
    "cargo_toml": "vac-rs/execpolicy/Cargo.toml"
  },
  {
    "crate_dir": "external-agent-migration",
    "rs_bytes": 70587,
    "cargo_toml": "vac-rs/external-agent-migration/Cargo.toml"
  },
  {
    "crate_dir": "external-agent-sessions",
    "rs_bytes": 46573,
    "cargo_toml": "vac-rs/external-agent-sessions/Cargo.toml"
  },
  {
    "crate_dir": "features",
    "rs_bytes": 61099,
    "cargo_toml": "vac-rs/features/Cargo.toml"
  },
  {
    "crate_dir": "feedback",
    "rs_bytes": 34456,
    "cargo_toml": "vac-rs/feedback/Cargo.toml"
  },
  {
    "crate_dir": "file-search",
    "rs_bytes": 43034,
    "cargo_toml": "vac-rs/file-search/Cargo.toml"
  },
  {
    "crate_dir": "file-system",
    "rs_bytes": 6151,
    "cargo_toml": "vac-rs/file-system/Cargo.toml"
  },
  {
    "crate_dir": "git-utils",
    "rs_bytes": 95411,
    "cargo_toml": "vac-rs/git-utils/Cargo.toml"
  },
  {
    "crate_dir": "hooks",
    "rs_bytes": 258289,
    "cargo_toml": "vac-rs/hooks/Cargo.toml"
  },
  {
    "crate_dir": "install-context",
    "rs_bytes": 8160,
    "cargo_toml": "vac-rs/install-context/Cargo.toml"
  },
  {
    "crate_dir": "keyring-store",
    "rs_bytes": 7474,
    "cargo_toml": "vac-rs/keyring-store/Cargo.toml"
  },
  {
    "crate_dir": "linux-sandbox",
    "rs_bytes": 269671,
    "cargo_toml": "vac-rs/linux-sandbox/Cargo.toml"
  },
  {
    "crate_dir": "lmstudio",
    "rs_bytes": 14741,
    "cargo_toml": "vac-rs/lmstudio/Cargo.toml"
  },
  {
    "crate_dir": "local-runtime-owner",
    "rs_bytes": 252932,
    "cargo_toml": "vac-rs/local-runtime-owner/Cargo.toml"
  },
  {
    "crate_dir": "login",
    "rs_bytes": 306416,
    "cargo_toml": "vac-rs/login/Cargo.toml"
  },
  {
    "crate_dir": "model-provider",
    "rs_bytes": 57130,
    "cargo_toml": "vac-rs/model-provider/Cargo.toml"
  },
  {
    "crate_dir": "model-provider-info",
    "rs_bytes": 33328,
    "cargo_toml": "vac-rs/model-provider-info/Cargo.toml"
  },
  {
    "crate_dir": "models-manager",
    "rs_bytes": 63326,
    "cargo_toml": "vac-rs/models-manager/Cargo.toml"
  },
  {
    "crate_dir": "network-proxy",
    "rs_bytes": 313446,
    "cargo_toml": "vac-rs/network-proxy/Cargo.toml"
  },
  {
    "crate_dir": "ollama",
    "rs_bytes": 28202,
    "cargo_toml": "vac-rs/ollama/Cargo.toml"
  },
  {
    "crate_dir": "otel",
    "rs_bytes": 197846,
    "cargo_toml": "vac-rs/otel/Cargo.toml"
  },
  {
    "crate_dir": "plugin",
    "rs_bytes": 12168,
    "cargo_toml": "vac-rs/plugin/Cargo.toml"
  },
  {
    "crate_dir": "process-hardening",
    "rs_bytes": 6271,
    "cargo_toml": "vac-rs/process-hardening/Cargo.toml"
  },
  {
    "crate_dir": "protocol",
    "rs_bytes": 597407,
    "cargo_toml": "vac-rs/protocol/Cargo.toml"
  },
  {
    "crate_dir": "realtime-webrtc",
    "rs_bytes": 10132,
    "cargo_toml": "vac-rs/realtime-webrtc/Cargo.toml"
  },
  {
    "crate_dir": "response-debug-context",
    "rs_bytes": 6199,
    "cargo_toml": "vac-rs/response-debug-context/Cargo.toml"
  },
  {
    "crate_dir": "rmcp-client",
    "rs_bytes": 253902,
    "cargo_toml": "vac-rs/rmcp-client/Cargo.toml"
  },
  {
    "crate_dir": "rollout",
    "rs_bytes": 267970,
    "cargo_toml": "vac-rs/rollout/Cargo.toml"
  },
  {
    "crate_dir": "rollout-trace",
    "rs_bytes": 427293,
    "cargo_toml": "vac-rs/rollout-trace/Cargo.toml"
  },
  {
    "crate_dir": "runtime-protocol",
    "rs_bytes": 807828,
    "cargo_toml": "vac-rs/runtime-protocol/Cargo.toml"
  },
  {
    "crate_dir": "sandboxing",
    "rs_bytes": 170947,
    "cargo_toml": "vac-rs/sandboxing/Cargo.toml"
  },
  {
    "crate_dir": "secrets",
    "rs_bytes": 22674,
    "cargo_toml": "vac-rs/secrets/Cargo.toml"
  },
  {
    "crate_dir": "shell-command",
    "rs_bytes": 192423,
    "cargo_toml": "vac-rs/shell-command/Cargo.toml"
  },
  {
    "crate_dir": "shell-escalation",
    "rs_bytes": 76599,
    "cargo_toml": "vac-rs/shell-escalation/Cargo.toml"
  },
  {
    "crate_dir": "skills",
    "rs_bytes": 6581,
    "cargo_toml": "vac-rs/skills/Cargo.toml"
  },
  {
    "crate_dir": "state",
    "rs_bytes": 505994,
    "cargo_toml": "vac-rs/state/Cargo.toml"
  },
  {
    "crate_dir": "terminal-detection",
    "rs_bytes": 40999,
    "cargo_toml": "vac-rs/terminal-detection/Cargo.toml"
  },
  {
    "crate_dir": "test-binary-support",
    "rs_bytes": 2358,
    "cargo_toml": "vac-rs/test-binary-support/Cargo.toml"
  },
  {
    "crate_dir": "thread-store",
    "rs_bytes": 259793,
    "cargo_toml": "vac-rs/thread-store/Cargo.toml"
  },
  {
    "crate_dir": "tools",
    "rs_bytes": 402925,
    "cargo_toml": "vac-rs/tools/Cargo.toml"
  },
  {
    "crate_dir": "tui",
    "rs_bytes": 6526559,
    "cargo_toml": "vac-rs/tui/Cargo.toml"
  },
  {
    "crate_dir": "uds",
    "rs_bytes": 14573,
    "cargo_toml": "vac-rs/uds/Cargo.toml"
  },
  {
    "crate_dir": "vac-api",
    "rs_bytes": 352783,
    "cargo_toml": "vac-rs/vac-api/Cargo.toml"
  },
  {
    "crate_dir": "vac-backend-openapi-models",
    "rs_bytes": 20021,
    "cargo_toml": "vac-rs/vac-backend-openapi-models/Cargo.toml"
  },
  {
    "crate_dir": "vac-client",
    "rs_bytes": 91703,
    "cargo_toml": "vac-rs/vac-client/Cargo.toml"
  },
  {
    "crate_dir": "vac-experimental-api-macros",
    "rs_bytes": 10063,
    "cargo_toml": "vac-rs/vac-experimental-api-macros/Cargo.toml"
  },
  {
    "crate_dir": "vac-mcp",
    "rs_bytes": 160574,
    "cargo_toml": "vac-rs/vac-mcp/Cargo.toml"
  },
  {
    "crate_dir": "windows-sandbox-rs",
    "rs_bytes": 462813,
    "cargo_toml": "vac-rs/windows-sandbox-rs/Cargo.toml"
  }
]
```

## Required next gate

Use cargo dependency graph to identify one-dependent micro-crates and avoid merging sandbox/FFI crates.
