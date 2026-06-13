#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fcntl
import os
import pty
import re
import struct
import termios
import selectors
import shlex
import signal
import subprocess
import sys
import time
from pathlib import Path

ENTER_ALT = b"\x1b[?1049h"
EXIT_ALT = b"\x1b[?1049l"
SHIFT_TAB = b"\x1b[Z"
CTRL_C = b"\x03"


def strip_ansi(text: str) -> str:
    return re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text)


def contains_ordered_text(text: str, needle: str) -> bool:
    pos = 0
    for ch in needle:
        found = text.find(ch, pos)
        if found < 0:
            return False
        pos = found + 1
    return True


def decode(buf: bytes) -> str:
    return buf.decode("utf-8", errors="replace")


def set_pty_size(fd: int, rows: int = 40, cols: int = 120) -> None:
    winsize = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, winsize)


def default_command(root: Path) -> list[str]:
    env_cmd = os.environ.get("VAC_TUI_SMOKE_CMD")
    if env_cmd:
        return shlex.split(env_cmd)

    binary = root / "vac-rs" / "target" / "debug" / "examples" / "tui_smoke"
    if binary.exists():
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
        "tui_smoke",
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


def run_smoke(root: Path, timeout: float) -> tuple[int, bytes]:
    try:
        master_fd, slave_fd = pty.openpty()
        set_pty_size(slave_fd)
    except OSError as error:
        print(f"VAC TUI PTY lifecycle smoke: SKIP pty unavailable: {error}")
        return 0, b""

    env = os.environ.copy()
    env.setdefault("TERM", "xterm-256color")
    env.setdefault("NO_COLOR", "1")
    env.setdefault("VAC_SKIP_AUTO_UPDATE", "1")
    env.setdefault("VAC_TUI_SMOKE", "1")
    env.setdefault("VAC_SKIP_DISCOVERY", "1")
    env.setdefault("RUST_BACKTRACE", "0")

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

    def pump(seconds: float) -> None:
        captured.extend(read_available(master_fd, selector, min(deadline, time.monotonic() + seconds)))

    try:
        pump(4.0)
        # Activate plan mode from the normal composer first. Shift+Tab moves the
        # operator to the review route, so sending /plan after Shift+Tab can be
        # consumed by review-mode key handling instead of the composer.
        for payload, pause in [
            (b"/plan\r\r", 1.2),
            (SHIFT_TAB, 0.8),
            (SHIFT_TAB, 0.5),
            (b"/context\r\r", 0.8),
            (b"hello from pty smoke", 0.5),
            (b"" * len(b"hello from pty smoke"), 0.5),
            (CTRL_C, 0.5),
            (CTRL_C, 1.5),
        ]:
            if proc.poll() is not None:
                break
            os.write(master_fd, payload)
            pump(pause)

        while proc.poll() is None and time.monotonic() < deadline:
            pump(0.2)

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
    parser = argparse.ArgumentParser(description="VAC TUI PTY lifecycle smoke gate")
    parser.add_argument("root", nargs="?", default=".")
    parser.add_argument("--timeout", type=float, default=25.0)
    parser.add_argument("--dump", action="store_true")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    code, output = run_smoke(root, args.timeout)
    text = decode(output).lower()
    visible_text = strip_ansi(text)

    if args.dump:
        sys.stdout.write(decode(output))

    if code == 0 and not output:
        return 0

    checks = {
        "entered_alt_screen": ENTER_ALT in output,
        "exited_alt_screen": EXIT_ALT in output,
        "shift_tab_or_plan_visible": "plan mode" in visible_text or "plan review" in visible_text or "draft/review" in visible_text,
        "context_or_tool_panel_visible": "context window" in visible_text or "tool timeline" in visible_text or "/context" in visible_text,
        "plain_text_echo_visible": contains_ordered_text(visible_text, "hello from pty smoke"),
        "mock_tabs_absent": " workbench " not in visible_text and " mcp " not in visible_text,
    }

    failed = [name for name, ok in checks.items() if not ok]
    if code != 0:
        failed.append(f"process_exit_code={code}")

    if failed:
        print("VAC TUI PTY lifecycle smoke: FAIL")
        for name in failed:
            print(f"- {name}")
        if not args.dump:
            tail = decode(output[-4000:])
            if tail:
                print("--- captured tail ---")
                print(tail)
        return 1

    print("VAC TUI PTY lifecycle smoke: PASS")
    for name in checks:
        print(f"- {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
