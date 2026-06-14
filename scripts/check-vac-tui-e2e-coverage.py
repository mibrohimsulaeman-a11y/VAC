#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
DOC = ROOT / "docs/audit/VAC_TUI_LIFECYCLE_E2E_COVERAGE_AUDIT.md"
PTY = ROOT / "scripts/pty-tui-lifecycle-smoke.py"
STATIC = ROOT / "scripts/check-tui-lifecycle-e2e-static.py"
SMOKE = ROOT / "vac-rs/crates/surfaces/vac-tui/examples/tui_smoke.rs"
AGENT_SMOKE = ROOT / "vac-rs/crates/surfaces/vac-tui/examples/tui_agent_tool_smoke.rs"
AGENT_PTY = ROOT / "scripts/pty-tui-agent-tool-lifecycle-smoke.py"
AGENT_MATRIX = ROOT / "tests/fixtures/tui-agent-tool-lifecycle/tool-matrix.json"
REAL_IO = ROOT / "scripts/pty-vac-cli-real-io-e2e.py"
RUNTIME_E2E = ROOT / "scripts/vac-runtime-agent-e2e-sv.py"
BOUND_CASES = ROOT / "tests/fixtures/runtime/bound_agent_e2e_cases.json"

KEY_UNCOVERED_BRIDGE_FILES = [
    "vac-rs/crates/surfaces/vac-tui/src/event_loop.rs",
    "vac-rs/crates/surfaces/vac-tui/src/services/handlers/dialog.rs",
    "vac-rs/crates/surfaces/vac-tui/src/services/handlers/tool.rs",
    "vac-rs/crates/surfaces/vac-tui/src/services/handlers/shell.rs",
]

errors: list[str] = []


def read(path: Path) -> str:
    if not path.is_file():
        errors.append(f"missing {path.relative_to(ROOT)}")
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def require(name: str, condition: bool) -> None:
    if not condition:
        errors.append(name)


doc = read(DOC)
pty = read(PTY)
static = read(STATIC)
smoke = read(SMOKE)
agent_smoke = read(AGENT_SMOKE)
agent_pty = read(AGENT_PTY)
agent_matrix = read(AGENT_MATRIX)
real_io = read(REAL_IO)
runtime_e2e = read(RUNTIME_E2E)
bound_cases = read(BOUND_CASES)

require("audit_doc_states_deterministic_harness_pass", "deterministic_user_agent_all_tools_tui_e2e=TV-Pass" in doc)
require("audit_doc_states_local_real_provider_file_io_pass", "local_real_provider_mcp_file_io_e2e=TV-Pass" in doc)
require("audit_doc_states_external_provider_remote_process_pending", "external_provider_remote_process_io_e2e=TV-Pending" in doc)

require(
    "pty_smoke_is_direct_tui_lifecycle_only",
    "VAC TUI PTY lifecycle smoke" in pty
    and "tui_smoke" in pty
    and "entered_alt_screen" in pty
    and "plain_text_echo_visible" in pty,
)
require(
    "tui_smoke_harness_is_not_agent_loop",
    "vac_tui::run_tui" in smoke
    and "OutputEvent::PlanModeActivated" in smoke
    and "OutputEvent::UserMessage" in smoke
    and "_ => {}" in smoke
    and "run_interactive" not in smoke
    and "vac_agent_loop" not in smoke,
)

require(
    "deterministic_agent_tool_harness_exists",
    "tui_agent_tool_smoke" in str(AGENT_SMOKE)
    and "VAC_AGENT_TOOL_SMOKE_STARTED" in agent_smoke
    and "OutputEvent::AcceptTool" in agent_smoke
    and "OutputEvent::AskUserResponse" in agent_smoke
    and "InputEvent::ToolResult" in agent_smoke
    and "ShowAskUserPopup" in agent_smoke,
)
require(
    "agent_tool_pty_smoke_gate_exists",
    "VAC TUI agent tool lifecycle smoke" in agent_pty
    and "tool_count=" in agent_pty
    and "marker_visible" in agent_pty
    and "approval_lifecycle_visible" in agent_pty,
)
require(
    "agent_tool_matrix_fixture_exists",
    "tui.agent_tool_lifecycle.v1" in agent_matrix
    and "required_visible_markers" in agent_matrix
    and "VAC_AGENT_TOOL_SMOKE_DONE" in agent_matrix,
)
require(
    "static_lifecycle_gate_exists",
    "VAC TUI E2E static lifecycle gate" in static
    and "official_pty_smoke_gate_present" in static
    and "shift_tab_toggles_visible_lifecycle" in static,
)

require(
    "local_real_provider_io_harness_exists",
    "VAC real provider/MCP IO E2E" in real_io
    and "vac-real-io-e2e" in real_io
    and "actual_io=create,str_replace,view,generate_password" in real_io
    and "work.txt" in real_io,
)
require(
    "runtime_agent_e2e_exists_but_is_decoupled_from_tui",
    "VAC runtime agent E2E SV" in runtime_e2e
    and "BoundAgentE2EInput" in runtime_e2e
    and "run_tui" not in runtime_e2e,
)
require(
    "runtime_agent_fixture_matrix_exists",
    "runtime_e2e_fixture_matrix" in bound_cases
    and "happy_path_done" in bound_cases
    and "destructive_command_pauses_for_approval" in bound_cases,
)

for rel in KEY_UNCOVERED_BRIDGE_FILES:
    src = read(ROOT / rel)
    has_tests = bool(re.search(r"#\[(tokio::)?test\]|mod tests", src))
    require(f"audit_doc_lists_uncovered_bridge_file:{rel}", rel in doc)
    # This gate intentionally tracks the current gap. If tests are later added,
    # update the audit doc and this script so coverage can move from TV-Pending.
    if has_tests:
        errors.append(
            f"{rel} now has local tests; update TUI E2E coverage audit instead of leaving TV-Pending gap text stale"
        )

required_tool_names = [
    "run_command",
    "run_command_task",
    "run_remote_command",
    "run_remote_command_task",
    "get_all_tasks",
    "wait_for_tasks",
    "get_task_details",
    "cancel_task",
    "view",
    "str_replace",
    "create",
    "remove",
    "view_web_page",
    "generate_password",
    "ask_user",
]
for tool in required_tool_names:
    require(f"audit_doc_tool_matrix_mentions:{tool}", f"`{tool}`" in doc)

if errors:
    print("VAC TUI E2E coverage audit: FAIL")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print("VAC TUI E2E coverage audit: PASS")
print("terminal_lifecycle_smoke=TV-Pass-direct-tui")
print("runtime_agent_e2e=TV-Pass-decoupled-from-tui")
print("deterministic_user_agent_all_tools_tui_e2e=TV-Pass")
print("local_real_provider_mcp_file_io_e2e=TV-Pass")
print("external_provider_remote_process_io_e2e=TV-Pending")
print(f"uncovered_bridge_files={len(KEY_UNCOVERED_BRIDGE_FILES)}")
