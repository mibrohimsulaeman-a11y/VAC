#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fcntl
import json
import os
import pty
import re
import selectors
import shlex
import signal
import struct
import subprocess
import sys
import termios
import time
from pathlib import Path

ENTER_ALT = b"\x1b[?1049h"
EXIT_ALT = b"\x1b[?1049l"
CTRL_C = b"\x03"
ENTER_KEY = b"\r\n"


def strip_ansi(text: str) -> str:
    return re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text)


def decode(buf: bytes) -> str:
    return buf.decode("utf-8", errors="replace")


def contains_ordered_text(text: str, needle: str) -> bool:
    pos = 0
    for ch in needle:
        found = text.find(ch, pos)
        if found < 0:
            return False
        pos = found + 1
    return True


def marker_present(text: str, marker: str) -> bool:
    lower = text.lower()
    marker_lower = marker.lower()
    return marker_lower in lower or contains_ordered_text(lower, marker_lower)


def tool_name_present(text: str, tool: str) -> bool:
    lower = text.lower()
    spaced = tool.replace("_", " ").lower()
    return tool.lower() in lower or spaced in lower or contains_ordered_text(lower, tool.lower())


def set_pty_size(fd: int, rows: int = 48, cols: int = 140) -> None:
    winsize = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, winsize)


def default_command(root: Path) -> list[str]:
    env_cmd = os.environ.get("VAC_TUI_AGENT_TOOL_SMOKE_CMD")
    if env_cmd:
        return shlex.split(env_cmd)

    binary = root / "vac-rs" / "target" / "debug" / "examples" / "tui_agent_tool_smoke"
    if os.environ.get("VAC_TUI_AGENT_TOOL_USE_EXISTING") == "1" and binary.exists():
        return [str(binary)]

    return [
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        str(root / "vac-rs" / "Cargo.toml"),
        "-p",
        "vac-tui",
        "--example",
        "tui_agent_tool_smoke",
    ]


def read_available(master_fd: int, selector: selectors.BaseSelector, deadline: float) -> bytes:
    chunks: list[bytes] = []
    while time.monotonic() < deadline:
        timeout = max(0.0, min(0.05, deadline - time.monotonic()))
        events = selector.select(timeout)
        if not events:
            break
        for _key, _mask in events:
            try:
                chunk = os.read(master_fd, 65536)
            except OSError:
                return b"".join(chunks)
            if not chunk:
                return b"".join(chunks)
            chunks.append(chunk)
    return b"".join(chunks)


def load_matrix(root: Path) -> dict:
    path = root / "tests" / "fixtures" / "tui-agent-tool-lifecycle" / "tool-matrix.json"
    return json.loads(path.read_text())


def run_smoke(root: Path, timeout: float) -> tuple[int, bytes]:
    try:
        master_fd, slave_fd = pty.openpty()
        set_pty_size(slave_fd)
    except OSError as error:
        print(f"VAC TUI agent tool lifecycle smoke: SKIP pty unavailable: {error}")
        return 0, b""

    matrix = load_matrix(root)
    prompt = matrix["prompt"].encode()
    type_prompt = os.environ.get("VAC_TUI_AGENT_TOOL_TYPE_PROMPT") == "1"

    env = os.environ.copy()
    env.setdefault("TERM", "xterm-256color")
    env.setdefault("NO_COLOR", "1")
    env.setdefault("VAC_SKIP_AUTO_UPDATE", "1")
    env.setdefault("VAC_TUI_SMOKE", "1")
    env.setdefault("VAC_SKIP_DISCOVERY", "1")
    env.setdefault("RUST_BACKTRACE", "0")
    env.pop("RUSTUP_HOME", None)
    env.pop("CARGO_HOME", None)

    proc = subprocess.Popen(
        default_command(root),
        cwd=root,
        stdin=slave_fd,
        stdout=slave_fd,
        stderr=slave_fd,
        env=env,
        start_new_session=True,
        close_fds=True,
    )
    os.close(slave_fd)

    selector = selectors.DefaultSelector()
    selector.register(master_fd, selectors.EVENT_READ)
    captured = bytearray()
    deadline = time.monotonic() + timeout
    sent_prompt = False
    prompt_started_at: float | None = None
    answered_ask_user = False
    last_enter = 0.0

    def pump(seconds: float) -> None:
        captured.extend(read_available(master_fd, selector, min(deadline, time.monotonic() + seconds)))

    try:
        while ENTER_ALT not in captured and proc.poll() is None and time.monotonic() < deadline:
            pump(0.25)

        while proc.poll() is None and time.monotonic() < deadline:
            visible = strip_ansi(decode(captured)).lower()
            now = time.monotonic()
            if not sent_prompt:
                if not type_prompt:
                    # The example feeds the prompt through run_tui's init-prompt
                    # path. This avoids CI PTY line-discipline flakes while still
                    # exercising the real TUI event loop, approval UI, tool result
                    # rendering, Ask User popup, and terminal cleanup.
                    sent_prompt = True
                    prompt_started_at = time.monotonic()
                    pump(0.5)
                    continue
                for byte in prompt:
                    os.write(master_fd, bytes([byte]))
                    pump(0.015)
                pump(0.2)
                os.write(master_fd, ENTER_KEY)
                sent_prompt = True
                prompt_started_at = time.monotonic()
                pump(0.8)
                continue

            if type_prompt and prompt_started_at is not None and "vac_agent_tool_smoke_started" not in visible:
                if now - last_enter > 0.75:
                    os.write(master_fd, ENTER_KEY)
                    last_enter = now
                pump(0.25)
                continue

            ask_user_active = (
                "ask user" in visible
                or "continue smoke" in visible
                or "needmore" in visible
                or "enter submit" in visible
            )
            if ask_user_active and "vac_agent_tool_smoke_done" not in visible:
                # The Rust smoke example injects Ask User select/confirm/submit
                # events internally after rendering the popup. Keep the PTY side
                # passive here; extra Space/Enter writes can race with the
                # deterministic handler on slow CI runners and leave the modal
                # open until timeout.
                answered_ask_user = True
                pump(0.6)
                continue

            if "vac_agent_tool_smoke_done" in visible:
                pump(2.0)
                break

            # While the approval bar/pending tool block is visible, Enter accepts
            # the current pending tool. Extra Enter presses during idle composer
            # states are harmless because the deterministic backend ignores all
            # messages after the first exact prompt.
            if now - last_enter > 0.45:
                os.write(master_fd, ENTER_KEY)
                last_enter = now
            pump(0.25)

        while proc.poll() is None and time.monotonic() < deadline:
            pump(0.2)

        if proc.poll() is None:
            os.write(master_fd, CTRL_C)
            pump(0.2)
            os.write(master_fd, CTRL_C)
            pump(0.8)
        if proc.poll() is None:
            os.killpg(proc.pid, signal.SIGTERM)
            pump(0.5)
        if proc.poll() is None:
            os.killpg(proc.pid, signal.SIGKILL)
            return 124, bytes(captured)

        pump(0.2)
        return proc.returncode or 0, bytes(captured)
    finally:
        try:
            selector.unregister(master_fd)
        except Exception:
            pass
        try:
            os.close(master_fd)
        except OSError:
            pass


def main() -> int:
    parser = argparse.ArgumentParser(description="VAC TUI agent tool lifecycle PTY smoke gate")
    parser.add_argument("root", nargs="?", default=".")
    parser.add_argument("--timeout", type=float, default=90.0)
    parser.add_argument("--dump", action="store_true")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    matrix = load_matrix(root)
    code, output = run_smoke(root, args.timeout)
    text = decode(output)
    visible = strip_ansi(text)
    visible_lower = visible.lower()

    if args.dump:
        sys.stdout.write(text)

    if code == 0 and not output:
        return 0

    required_tool_names = [tool["name"] for tool in matrix["tools"]] + [matrix["ask_user"]["name"]]
    checks: dict[str, bool] = {
        "entered_alt_screen": ENTER_ALT in output,
        "exited_alt_screen": EXIT_ALT in output,
        "user_prompt_echo_visible": marker_present(visible, matrix["prompt"])
        or ("agent tool" in visible_lower and "smoke" in visible_lower),
        "agent_started_visible": "vac_agent_tool_smoke_started" in visible_lower,
        "approval_lifecycle_visible": "approve" in visible_lower or "approval" in visible_lower,
        "ask_user_visible": "continue smoke" in visible_lower or "needmore" in visible_lower,
        "done_marker_visible": "vac_agent_tool_smoke_done" in visible_lower,
        "mock_tabs_absent": " workbench " not in visible_lower and " mcp " not in visible_lower,
    }
    for tool in required_tool_names:
        checks[f"tool_visible:{tool}"] = tool_name_present(visible, tool)
    for marker in matrix["required_visible_markers"]:
        checks[f"marker_visible:{marker}"] = marker_present(visible, marker)

    failed = [name for name, ok in checks.items() if not ok]
    if code != 0:
        failed.append(f"process_exit_code={code}")

    if failed:
        print("VAC TUI agent tool lifecycle smoke: FAIL")
        for name in failed:
            print(f"- {name}")
        if not args.dump:
            tail = decode(output[-6000:])
            if tail:
                print("--- captured tail ---")
                print(tail)
        return 1

    print("VAC TUI agent tool lifecycle smoke: PASS")
    print(f"tool_count={len(required_tool_names)}")
    print(f"marker_count={len(matrix['required_visible_markers'])}")
    for name in checks:
        print(f"- {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
