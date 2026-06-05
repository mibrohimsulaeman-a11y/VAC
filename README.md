# VAC (Vastar Agentic CLI)

VAC is the developer agent control plane and terminal user interface (TUI) for Vastar. It enables end-to-end execution of agentic workflows with granular policy control, validation gating, and approval workflows.

---

## 🚀 Product Structure & Topology

The project is structured as a mono-repository with the following key components:

- **Product Command / Entry Point:** `vac` (compiled from CLI surface)
- **Terminal User Interface (TUI):** [vac-rs/crates/surfaces/tui](file:///home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/crates/surfaces/tui)
- **Command Line Interface (CLI):** [vac-rs/crates/surfaces/cli](file:///home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/crates/surfaces/cli)
- **Control Plane & Core Logic:** [vac-rs/crates/control-plane](file:///home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/crates/control-plane)
- **Package Launcher:** [vac-cli/bin/vac.js](file:///home/emp/Documents/VAC/vastar-agentic-cli/vac-cli/bin/vac.js)

---

## 🛠️ Rust Development & Test Loop

> [!IMPORTANT]
> **MANDATORY RULES FOR ALL DEVELOPERS AND AGENTIC WORKERS:**
> 1. **NEVER use `cargo test`** unless absolutely necessary or explicitly instructed. Always use `cargo nextest run` (which runs tests in parallel and is significantly faster).
> 2. **ALWAYS use sccache**. The project contains a `.cargo/config.toml` that forces `sccache`. Do not disable it or bypass it.
> 3. **DO NOT use separate target directories** (such as `--target-dir /tmp/vac-validate-target` or any custom target folder outside the workspace). All builds and tests MUST run inside the standard workspace `target` directory to ensure full caching and prevent workspace littering.

### 1. Build and Run Directly
Avoid using repeated `cargo run` invocations for CLI checks. Instead, build the binary once and invoke the built executable directly:
```bash
# Build the CLI
cargo build --manifest-path vac-rs/Cargo.toml -p vac-cli

# Run doctor checks directly
./vac-rs/target/debug/vac doctor registry .
./vac-rs/target/debug/vac doctor policy .
./vac-rs/target/debug/vac doctor surfaces .
./vac-rs/target/debug/vac doctor workflow .
```

### 2. Targeted Unit Testing
Run the narrowest relevant test filter to avoid long recompilations:
```bash
cargo nextest run --manifest-path vac-rs/Cargo.toml -p vac-surface-tui surface_route_catalog::tests
```

### 3. Key Environment Variables
- **`VAC_BUILD_CHECK_REPO_ROOT`**: Points the workflow runner to the repository root directory (necessary for cargo build-checks inside tests to locate the main workspace).
  ```bash
  VAC_BUILD_CHECK_REPO_ROOT=$(pwd) cargo nextest run --manifest-path vac-rs/Cargo.toml -p vac-surface-tui
  ```
- **`VAC_SKIP_BUILD_CHECK`**: Set to `true`/`1` to bypass cargo compilation checks in workflow runner tests.

### 4. Interactive Snapshot Testing
VAC uses `insta` for TUI assertion snapshots. If layout changes occur, review and accept snapshots via:
```bash
cd vac-rs
cargo insta accept
```

---

## 🎛️ Control Plane & Declarative Registry

The declarative product control plane lives under [docs/workflow-control-plane/INDEX.md](file:///home/emp/Documents/VAC/vastar-agentic-cli/docs/workflow-control-plane/INDEX.md) and the `.vac/` folder.

All features must follow these core guidelines:
1. **Capability Manifest First:** Add/update capability manifests under `.vac/capabilities/` before introducing backend-only behavior.
2. **TUI/CLI Reachability:** Every capability must be exposed or visible in either the CLI commands or TUI routes.
3. **Reasoning & Status Alignment:** Maintain clear `status` mappings (`ready`, `partial`, `planned`). `partial` statuses must specify a `reason` (e.g., `"Under development"`).

### Self-Check Validation
Before opening a pull request, verify index links and manifest integrity:
```bash
./vac-rs/target/debug/vac doctor docs .
```
