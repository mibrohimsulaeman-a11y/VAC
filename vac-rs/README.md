# VAC CLI (Rust Implementation)

We provide VAC CLI as a standalone executable to ensure a zero-dependency install.

## Installing VAC

Today, the easiest way to install VAC is via `npm`:

```shell
npm i -g @vastar/vac
vac
```

You can also install via Homebrew (`brew install --cask vac`) or download a platform-specific release directly from our [GitHub Releases](https://github.com/vastar/vac/releases).

## Documentation quickstart

This README is the Rust workspace-local entry point. For product and operator documentation, start with:

- [Root repository overview](../README.md)
- [Product docs index](../docs/product/INDEX.md)
- [Architecture docs index](../docs/architecture/INDEX.md)
- [Validation gates index](../docs/validation/INDEX.md)
- [Workflow control-plane index](../docs/workflow-control-plane/INDEX.md)

## What's new in the Rust CLI

The Rust implementation is now the maintained VAC CLI and serves as the default experience. It includes a number of features that the legacy TypeScript CLI never supported.

### Config

VAC supports a rich set of configuration options. The Rust CLI uses `config.toml` instead of legacy JSON config. Keep user-facing configuration docs in the root product/architecture docs and keep this README focused on workspace-local development.

### Model Context Protocol Support

#### MCP client

VAC CLI functions as an MCP client that allows the VAC CLI and IDE extension to connect to MCP servers on startup. Product-facing MCP/config documentation should live under the root docs tree, not as broken upstream-style links from this workspace README.

#### MCP server (experimental)

VAC can be launched as an MCP _server_ by running `vac mcp-server`. This allows _other_ MCP clients to use VAC as a tool for another agent.

Use the [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) to try it out:

```shell
npx @modelcontextprotocol/inspector vac mcp-server
```

Use `vac mcp` to add/list/get/remove MCP server launchers defined in `config.toml`, and `vac mcp-server` to run the MCP server directly.

### Notifications

The legacy `notify` setting is deprecated and will be removed in a future release. Existing configurations still work, but new automation should use lifecycle hooks instead. When VAC detects that it is running under WSL 2 inside Windows Terminal (`WT_SESSION` is set), the TUI automatically falls back to native Windows toast notifications so approval prompts and completed turns surface even though Windows Terminal does not implement OSC 9.

### `vac exec` to run VAC programmatically/non-interactively

To run VAC non-interactively, run `vac exec PROMPT` (you can also pass the prompt via `stdin`) and VAC will work on your task until it decides that it is done and exits. If you provide both a prompt argument and piped stdin, VAC appends stdin as a `<stdin>` block after the prompt so patterns like `echo "my output" | vac exec "Summarize this concisely"` work naturally. Output is printed to the terminal directly. You can set the `RUST_LOG` environment variable to see more about what's going on.
Use `vac exec --ephemeral ...` to run without persisting session rollout files to disk.

### Experimenting with the VAC Sandbox

To test to see what happens when a command is run under the sandbox provided by VAC, we provide the following subcommands in VAC CLI:

```
# macOS
vac sandbox macos [--log-denials] [COMMAND]...

# Linux
vac sandbox linux [COMMAND]...

# Windows
vac sandbox windows [COMMAND]...

# Legacy aliases
vac debug seatbelt [--log-denials] [COMMAND]...
vac debug landlock [COMMAND]...
```

To try a writable legacy sandbox mode with these commands, pass an explicit config override such
as `-c 'sandbox_mode="workspace-write"'`.

### Selecting a sandbox policy via `--sandbox`

The Rust CLI exposes a dedicated `--sandbox` (`-s`) flag that lets you pick the sandbox policy **without** having to reach for the generic `-c/--config` option:

```shell
# Run VAC with the default, read-only sandbox
vac --sandbox read-only

# Allow the agent to write within the current workspace while still blocking network access
vac --sandbox workspace-write

# Danger! Disable sandboxing entirely (only do this if you are already running in a container or other isolated env)
vac --sandbox danger-full-access
```

The same setting can be persisted in `~/.vac/config.toml` via the top-level `sandbox_mode = "MODE"` key, e.g. `sandbox_mode = "workspace-write"`.
In `workspace-write`, VAC also includes `~/.vac/memories` in its writable roots so memory maintenance does not require an extra approval.


## Development validation speed

For Rust development, prefer cache-friendly validation:

- Build CLI binaries once, then run the produced binary directly for repeated commands.
- Avoid repeated `cargo run` calls for the same binary in one validation pass.
- Use narrow `cargo test` filters while iterating, and broaden only when the affected area requires it.
- Keep Cargo artifacts warm; avoid `cargo clean` unless explicitly recovering disk or cache state.

Example for this workspace:

```shell
cargo test --manifest-path Cargo.toml -p vac-core registry_diagnostics -- --nocapture
cargo build --manifest-path Cargo.toml -p vac-surface-cli
./target/debug/vac doctor registry ..
./target/debug/vac doctor policy ..
./target/debug/vac doctor surfaces ..
./target/debug/vac doctor workflow ..
```

From the repository root, use `./vac-rs/target/debug/vac ...` after building with `--manifest-path vac-rs/Cargo.toml`.

## Code Organization

This folder is the root of a Cargo workspace. It contains quite a bit of experimental code, but here are the key crates:

- [`core/`](./core) contains the business logic for VAC. Ultimately, we hope this becomes a library crate that is generally useful for building other Rust/native applications that use VAC.
- [`exec/`](./exec) "headless" CLI for use in automation.
- [`tui/`](./tui) CLI that launches a fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`cli/`](./cli) CLI multitool that provides the aforementioned CLIs via subcommands.

If you want to contribute or inspect behavior in detail, start by reading the module-level `README.md` files under each crate and run the project workspace from the top-level `vac-rs` directory so shared config, features, and build scripts stay aligned.
