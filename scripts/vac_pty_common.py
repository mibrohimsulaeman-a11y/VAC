#!/usr/bin/env python3
"""Shared PTY/terminal helpers for VAC smoke scripts."""
from __future__ import annotations

import fcntl
import os
import re
import selectors
import struct
import termios
import time


ANSI_CSI_RE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")


def strip_ansi(text: str) -> str:
    return ANSI_CSI_RE.sub("", text)


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


def set_pty_size(fd: int, rows: int, cols: int) -> None:
    winsize = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, winsize)


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
