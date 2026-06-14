#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fcntl
import hashlib
import http.server
import json
import os
import pty
import re
import selectors
import shlex
import signal
import socket
import socketserver
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
from pathlib import Path
from typing import Any

ENTER_ALT = b"\x1b[?1049h"
EXIT_ALT = b"\x1b[?1049l"
CTRL_C = b"\x03"
ENTER_KEY = b"\r\n"
SESSION_ID = "real-io-session"
CAPABILITY = "vac.real_io_e2e"
PLAN_HASH = "sha256:real-io-plan"
POLICY_HASH = "sha256:real-io-policy"
GATE = "real_io_e2e_gate"


def strip_ansi(text: str) -> str:
    return re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text)


def decode(buf: bytes) -> str:
    return buf.decode("utf-8", errors="replace")


def jcs_sha(value: Any) -> str:
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return "sha256:" + hashlib.sha256(raw.encode("utf-8")).hexdigest()


def strip_nulls(value: Any) -> Any:
    if isinstance(value, dict):
        return {k: strip_nulls(v) for k, v in value.items() if v is not None}
    if isinstance(value, list):
        return [strip_nulls(v) for v in value]
    return value


def action_target(tool_name: str, args: dict[str, Any]) -> tuple[str, str]:
    bare = tool_name.split("__", 1)[-1]
    if bare == "run_command":
        return "execute_process", args["command"]
    if bare == "view":
        return "filesystem_read", args["path"]
    if bare in {"create", "str_replace"}:
        return "filesystem_write", args["path"]
    if bare == "remove":
        return "filesystem_delete", args["path"]
    if bare == "view_web_page":
        return "network_access", args["url"]
    return "tool_execute", bare


def approval_for(tool_call_id: str, tool_name: str, args_without_approval: dict[str, Any]) -> dict[str, Any]:
    action, target = action_target(tool_name, args_without_approval)
    compact_args = strip_nulls(args_without_approval)
    diff_hash = jcs_sha(
        {
            "tool_call_id": tool_call_id,
            "tool_name": tool_name,
            "action": action,
            "target": target,
            "arguments": compact_args,
            "gate": GATE,
        }
    )
    nonce = jcs_sha(
        {
            "session_id": SESSION_ID,
            "tool_call_id": tool_call_id,
            "diff_hash": diff_hash,
            "policy_snapshot_hash": POLICY_HASH,
            "mode": "l1_operator_mediated",
        }
    )
    binding_hash = jcs_sha(
        {
            "plan_hash": PLAN_HASH,
            "diff_hash": diff_hash,
            "policy_snapshot_hash": POLICY_HASH,
            "nonce": nonce,
        }
    )
    return {
        "schema_version": 2,
        "kind": "vac_bound_tool_approval",
        "approval_request_id": f"approval.{tool_call_id}.{binding_hash.removeprefix('sha256:')[:12]}",
        "tool_call_id": tool_call_id,
        "tool_name": tool_name,
        "gate": GATE,
        "decision": "pass",
        "mode": "l1_runtime_mediated",
        "action": action,
        "target": target,
        "session_id": SESSION_ID,
        "capability": CAPABILITY,
        "read_plan_ticket": None,
        "plan_hash": PLAN_HASH,
        "diff_hash": diff_hash,
        "policy_snapshot_hash": POLICY_HASH,
        "nonce": nonce,
        "expires_at": "l1-session-scoped",
        "binding_hash": binding_hash,
        "operator_sig": {"algorithm": "none", "mode": "l1_operator_mediated_integrity_hint"},
        "broker_sig": {"algorithm": "none", "mode": "l1_runtime_mediated_integrity_hint"},
    }


def with_approval(tool_call_id: str, tool_name: str, args: dict[str, Any]) -> dict[str, Any]:
    out = dict(args)
    out["vac_bound_approval"] = approval_for(tool_call_id, tool_name, args)
    return out


def build_tool_plan() -> list[dict[str, Any]]:
    return [
        {
            "id": "call_real_create",
            "name": "vac__create",
            "args": with_approval(
                "call_real_create",
                "vac__create",
                {"path": "work.txt", "file_text": "alpha\n"},
            ),
        },
        {
            "id": "call_real_replace",
            "name": "vac__str_replace",
            "args": with_approval(
                "call_real_replace",
                "vac__str_replace",
                {"path": "work.txt", "old_str": "alpha", "new_str": "beta"},
            ),
        },
        {
            "id": "call_real_view",
            "name": "vac__view",
            "args": with_approval(
                "call_real_view",
                "vac__view",
                {"path": "work.txt", "view_range": [1, -1]},
            ),
        },
    ]


def full_tool_plan() -> list[dict[str, Any]]:
    plan = build_tool_plan()
    plan.append({"id": "call_real_password", "name": "vac__generate_password", "args": {"length": 12}})
    return plan


class ProviderState:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.step = 0
        self.requests: list[dict[str, Any]] = []
        self.paths: list[str] = []
        self.tool_plan = full_tool_plan()
        self.seen_tool_results: list[str] = []

    def next_response(self, request: dict[str, Any]) -> dict[str, Any]:
        with self.lock:
            self.requests.append(request)
            for message in request.get("messages", []):
                if message.get("role") == "tool":
                    content = str(message.get("content", ""))
                    if content not in self.seen_tool_results:
                        self.seen_tool_results.append(content)
            if self.step < len(self.tool_plan):
                spec = self.tool_plan[self.step]
                self.step += 1
                return {"kind": "tool", "spec": spec}
            return {"kind": "final"}


class ProviderHandler(http.server.BaseHTTPRequestHandler):
    state: ProviderState

    def log_message(self, _fmt: str, *_args: Any) -> None:
        return

    def _json(self, payload: Any, status: int = 200) -> None:
        raw = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def do_GET(self) -> None:  # noqa: N802
        if self.path.endswith("/models"):
            self._json({"object": "list", "data": [{"id": "vac-real-io-e2e", "object": "model"}]})
            return
        if self.path.startswith("/v1/rules"):
            self._json({"results": []})
            return
        self._json({"ok": True})

    def do_POST(self) -> None:  # noqa: N802
        length = int(self.headers.get("content-length", "0"))
        request = json.loads(self.rfile.read(length) or b"{}")
        self.state.paths.append(self.path)
        if "chat" not in self.path and "responses" not in self.path:
            self._json({"results": [], "ok": True})
            return
        outcome = self.state.next_response(request)
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("cache-control", "no-cache")
        self.end_headers()
        if outcome["kind"] == "tool":
            self._send_tool_stream(outcome["spec"], request.get("model", "vac-real-io-e2e"))
        else:
            self._send_final_stream(request.get("model", "vac-real-io-e2e"))

    def _event(self, payload: dict[str, Any]) -> None:
        self.wfile.write(f"data: {json.dumps(payload)}\n\n".encode("utf-8"))
        self.wfile.flush()

    def _send_tool_stream(self, spec: dict[str, Any], model: str) -> None:
        chunk_id = f"chatcmpl-{spec['id']}"
        self._event(
            {
                "id": chunk_id,
                "object": "chat.completion.chunk",
                "created": 0,
                "model": model,
                "choices": [{"index": 0, "delta": {"role": "assistant"}, "finish_reason": None}],
            }
        )
        self._event(
            {
                "id": chunk_id,
                "object": "chat.completion.chunk",
                "created": 0,
                "model": model,
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "tool_calls": [
                                {
                                    "index": 0,
                                    "id": spec["id"],
                                    "type": "function",
                                    "function": {
                                        "name": spec["name"],
                                        "arguments": json.dumps(spec["args"], separators=(",", ":")),
                                    },
                                }
                            ]
                        },
                        "finish_reason": None,
                    }
                ],
            }
        )
        self._event(
            {
                "id": chunk_id,
                "object": "chat.completion.chunk",
                "created": 0,
                "model": model,
                "choices": [{"index": 0, "delta": {}, "finish_reason": "tool_calls"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            }
        )
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()

    def _send_final_stream(self, model: str) -> None:
        chunk_id = "chatcmpl-final"
        self._event(
            {
                "id": chunk_id,
                "object": "chat.completion.chunk",
                "created": 0,
                "model": model,
                "choices": [{"index": 0, "delta": {"role": "assistant"}, "finish_reason": None}],
            }
        )
        self._event(
            {
                "id": chunk_id,
                "object": "chat.completion.chunk",
                "created": 0,
                "model": model,
                "choices": [
                    {"index": 0, "delta": {"content": "VAC_REAL_IO_E2E_DONE"}, "finish_reason": None}
                ],
            }
        )
        self._event(
            {
                "id": chunk_id,
                "object": "chat.completion.chunk",
                "created": 0,
                "model": model,
                "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            }
        )
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def start_provider(port: int) -> tuple[socketserver.ThreadingTCPServer, ProviderState]:
    state = ProviderState()

    class Handler(ProviderHandler):
        pass

    Handler.state = state
    server = socketserver.ThreadingTCPServer(("127.0.0.1", port), Handler)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, state


def set_pty_size(fd: int, rows: int = 48, cols: int = 140) -> None:
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


def find_vac_binary(root: Path) -> list[str]:
    env_cmd = os.environ.get("VAC_REAL_IO_E2E_CMD")
    if env_cmd:
        return shlex.split(env_cmd)
    binary = root / "vac-rs" / "target" / "debug" / "vac"
    if binary.exists():
        return [str(binary)]
    return [
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        str(root / "vac-rs" / "Cargo.toml"),
        "-p",
        "vac-cli",
        "--bin",
        "vac",
        "--",
    ]


def write_sandbox(sandbox: Path, provider_port: int) -> Path:
    (sandbox / ".vac").mkdir(parents=True, exist_ok=True)
    config_path = sandbox / ".vac" / "config.toml"
    config_path.write_text(
        f'''
[settings]
machine_name = "real-io-e2e"
auto_append_gitignore = false
anonymous_id = "00000000-0000-0000-0000-000000000000"
collect_telemetry = false
editor = "true"

[profiles.default]
provider = "local"
model = "offline/vac-real-io-e2e"
api_endpoint = "http://127.0.0.1:{provider_port}"
allowed_tools = ["vac__create", "vac__str_replace", "vac__view", "vac__generate_password"]
auto_approve = ["vac__create", "vac__str_replace", "vac__view", "vac__generate_password"]

[profiles.default.providers.offline]
type = "custom"
api_endpoint = "http://127.0.0.1:{provider_port}/v1"
api_key = "deterministic-local-key"
'''.strip()
        + "\n"
    )
    return config_path

def run_vac(root: Path, sandbox: Path, config_path: Path, timeout: float) -> tuple[int, bytes]:
    master_fd, slave_fd = pty.openpty()
    set_pty_size(slave_fd)
    env = os.environ.copy()
    env.update(
        {
            "TERM": "xterm-256color",
            "NO_COLOR": "1",
            "VAC_SKIP_AUTO_UPDATE": "1",
            "VAC_SKIP_DISCOVERY": "1",
            "VAC_SKIP_WARDEN": "1",
            "RUST_BACKTRACE": "0",
            "HOME": str(sandbox / "home"),
        }
    )
    (sandbox / "home").mkdir(exist_ok=True)
    env.pop("RUSTUP_HOME", None)
    cmd = find_vac_binary(root) + [
        "--config",
        str(config_path),
        "--profile",
        "default",
        "--model",
        "offline/vac-real-io-e2e",
        "--theme",
        "dark",
        "--disable-mcp-mtls",
        "--disable-subagents",
        "--ignore-agents-md",
        "--ignore-apps-md",
        'init',
    ]
    proc = subprocess.Popen(
        cmd,
        cwd=sandbox,
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
    sent_prompt = True
    last_enter = 0.0
    prompt = b"real provider mcp io e2e"

    def pump(seconds: float) -> None:
        captured.extend(read_available(master_fd, selector, min(deadline, time.monotonic() + seconds)))

    try:
        while ENTER_ALT not in captured and proc.poll() is None and time.monotonic() < deadline:
            pump(0.25)
        while proc.poll() is None and time.monotonic() < deadline:
            visible = strip_ansi(decode(captured)).lower()
            now = time.monotonic()
            if not sent_prompt:
                if "first launch" in visible or "ready" not in visible:
                    pump(0.25)
                    continue
                for byte in prompt:
                    os.write(master_fd, bytes([byte]))
                    pump(0.01)
                os.write(master_fd, ENTER_KEY)
                sent_prompt = True
                pump(0.5)
                continue
            if "vac_real_io_e2e_done" in visible:
                os.write(master_fd, CTRL_C)
                pump(0.2)
                os.write(master_fd, CTRL_C)
                pump(1.0)
                break
            if now - last_enter > 0.75:
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


def assert_sandbox_effects(sandbox: Path) -> list[str]:
    errors: list[str] = []
    work = sandbox / "work.txt"
    if not work.exists() or work.read_text() != "beta\n":
        errors.append("work.txt was not created and rewritten to beta by actual MCP file tools")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Real vac-cli interactive provider/MCP IO E2E")
    parser.add_argument("root", nargs="?", default=".")
    parser.add_argument("--timeout", type=float, default=160.0)
    parser.add_argument("--keep-sandbox", action="store_true")
    parser.add_argument("--dump", action="store_true")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    provider_port = free_port()
    server, state = start_provider(provider_port)
    tmp = tempfile.TemporaryDirectory(prefix="vac-real-io-e2e-")
    sandbox = Path(tmp.name)
    keep = args.keep_sandbox
    try:
        config_path = write_sandbox(sandbox, provider_port)
        code, output = run_vac(root, sandbox, config_path, args.timeout)
        text = decode(output)
        visible = strip_ansi(text).lower()
        if args.dump:
            sys.stdout.write(text)
        checks = {
            "entered_alt_screen": ENTER_ALT in output,
            "exited_alt_screen": EXIT_ALT in output,
            "provider_received_chat_requests": len(state.requests) >= len(state.tool_plan) + 1,
            "provider_saw_tool_results": len(state.seen_tool_results) >= len(state.tool_plan),
            "visible_done_marker": "vac_real_io_e2e_done" in visible,
            "visible_create": "create" in visible and "work.txt" in visible,
            "visible_replace": "str_replace" in visible or "str replace" in visible,
            "visible_view": "view" in visible,
            "visible_generate_password": "generate_password" in visible or "generate password" in visible,
        }
        failed = [name for name, ok in checks.items() if not ok]
        failed.extend(assert_sandbox_effects(sandbox))
        if code != 0:
            failed.append(f"process_exit_code={code}")
        if failed:
            print("VAC real provider/MCP IO E2E: FAIL")
            print(f"sandbox={sandbox}")
            print(f"chat_requests={len(state.requests)} paths={state.paths}")
            print(f"tool_results_seen={len(state.seen_tool_results)}")
            for item in failed:
                print(f"- {item}")
            if not args.dump:
                print("--- captured tail ---")
                print(text[-8000:])
            return 1
        print("VAC real provider/MCP IO E2E: PASS")
        print(f"sandbox={sandbox}")
        print(f"chat_requests={len(state.requests)}")
        print(f"tool_results_seen={len(state.seen_tool_results)}")
        print("actual_io=create,str_replace,view,generate_password")
        for name in checks:
            print(f"- {name}")
        return 0
    finally:
        server.shutdown()
        server.server_close()
        if keep:
            print(f"kept_sandbox={sandbox}")
        else:
            tmp.cleanup()


if __name__ == "__main__":
    raise SystemExit(main())
