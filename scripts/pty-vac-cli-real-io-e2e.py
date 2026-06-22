#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import http.server
import json
import os
import pty
import selectors
import shlex
import signal
import socket
import socketserver
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import Any

from vac_pty_common import decode, read_available, set_pty_size, strip_ansi

ENTER_ALT = b"\x1b[?1049h"
EXIT_ALT = b"\x1b[?1049l"
CTRL_C = b"\x03"
ENTER_KEY = b"\r\n"
SESSION_ID = "real-io-session"
CAPABILITY = "vac.real_io_e2e"
PLAN_HASH = "sha256:real-io-plan"
POLICY_HASH = "sha256:real-io-policy"
GATE = "real_io_e2e_gate"



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


def structured_command(
    command_id: str,
    runner: str,
    args: list[str],
    *,
    risk: str = "execute_process",
    approval: str = "policy",
) -> dict[str, Any]:
    return {"id": command_id, "runner": runner, "args": args, "risk": risk, "approval": approval}


def tool_spec(tool_call_id: str, tool_name: str, args: dict[str, Any]) -> dict[str, Any]:
    return {"id": tool_call_id, "name": tool_name, "args": args}


def build_positive_tool_plan(web_url: str) -> list[dict[str, Any]]:
    command_args = {
        "command": "printf VAC_RUN_COMMAND_OK",
        "structured_command": structured_command(
            "real_io.printf", "printf", ["VAC_RUN_COMMAND_OK"]
        ),
        "description": "deterministic sandbox command",
        "timeout": 5,
    }
    return [
        tool_spec(
            "call_real_create",
            "vac__create",
            with_approval(
                "call_real_create",
                "vac__create",
                {"path": "work.txt", "file_text": "alpha\n"},
            ),
        ),
        tool_spec(
            "call_real_replace",
            "vac__str_replace",
            with_approval(
                "call_real_replace",
                "vac__str_replace",
                {"path": "work.txt", "old_str": "alpha", "new_str": "beta"},
            ),
        ),
        tool_spec(
            "call_real_view",
            "vac__view",
            with_approval(
                "call_real_view",
                "vac__view",
                {"path": "work.txt", "view_range": [1, -1]},
            ),
        ),
        tool_spec(
            "call_real_create_delete_target",
            "vac__create",
            with_approval(
                "call_real_create_delete_target",
                "vac__create",
                {"path": "delete-me.txt", "file_text": "delete me\n"},
            ),
        ),
        tool_spec(
            "call_real_run_command",
            "vac__run_command",
            with_approval("call_real_run_command", "vac__run_command", command_args),
        ),
        tool_spec(
            "call_real_remove",
            "vac__remove",
            with_approval(
                "call_real_remove",
                "vac__remove",
                {"path": "delete-me.txt", "recursive": False},
            ),
        ),
        tool_spec(
            "call_real_view_web_page",
            "vac__view_web_page",
            with_approval("call_real_view_web_page", "vac__view_web_page", {"url": web_url}),
        ),
        tool_spec("call_real_password", "vac__generate_password", {"length": 12}),
    ]


def build_negative_tool_plan() -> list[dict[str, Any]]:
    missing_approval_command = {
        "command": "printf SHOULD_NOT_RUN",
        "structured_command": structured_command(
            "negative.missing_approval", "printf", ["SHOULD_NOT_RUN"]
        ),
        "description": "must be blocked before execution",
        "timeout": 5,
    }
    denied_shell_args = {
        "command": "sh -c touch should-not-exist.txt",
        "structured_command": structured_command(
            "negative.shell_runner",
            "sh",
            ["-c", "touch", "should-not-exist.txt"],
        ),
        "description": "must reject shell runner",
        "timeout": 5,
    }
    remove_mismatch_args = {"path": "protected.txt", "recursive": False}
    remove_mismatch_args["vac_bound_approval"] = approval_for(
        "call_negative_remove_binding_mismatch",
        "vac__remove",
        {"path": "delete-me.txt", "recursive": False},
    )
    return [
        tool_spec(
            "call_negative_run_command_missing_approval",
            "vac__run_command",
            missing_approval_command,
        ),
        tool_spec(
            "call_negative_run_command_shell_runner",
            "vac__run_command",
            with_approval(
                "call_negative_run_command_shell_runner",
                "vac__run_command",
                denied_shell_args,
            ),
        ),
        tool_spec(
            "call_negative_remove_binding_mismatch",
            "vac__remove",
            remove_mismatch_args,
        ),
        tool_spec(
            "call_negative_view_web_page_non_loopback_http",
            "vac__view_web_page",
            with_approval(
                "call_negative_view_web_page_non_loopback_http",
                "vac__view_web_page",
                {"url": "http://example.com/"},
            ),
        ),
    ]


def full_tool_plan(web_url: str) -> list[dict[str, Any]]:
    return build_positive_tool_plan(web_url) + build_negative_tool_plan()


class ProviderState:
    def __init__(self, web_url: str) -> None:
        self.lock = threading.Lock()
        self.step = 0
        self.requests: list[dict[str, Any]] = []
        self.paths: list[str] = []
        self.tool_plan = full_tool_plan(web_url)
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


def start_provider(port: int, web_url: str) -> tuple[socketserver.ThreadingTCPServer, ProviderState]:
    state = ProviderState(web_url)

    class Handler(ProviderHandler):
        pass

    Handler.state = state
    server = socketserver.ThreadingTCPServer(("127.0.0.1", port), Handler)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, state


class LoopbackContentHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, _fmt: str, *_args: Any) -> None:
        return

    def do_GET(self) -> None:  # noqa: N802
        body = (
            "<html><head><title>VAC Loopback Fixture</title></head>"
            "<body><h1>VAC_LOOPBACK_WEB_OK</h1>"
            "<p>local governed network fixture</p></body></html>"
        ).encode("utf-8")
        self.send_response(200)
        self.send_header("content-type", "text/html; charset=utf-8")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def start_loopback_content_server(port: int) -> socketserver.ThreadingTCPServer:
    server = socketserver.ThreadingTCPServer(("127.0.0.1", port), LoopbackContentHandler)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server



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
    (sandbox / "protected.txt").write_text("protected\n", encoding="utf-8")
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
allowed_tools = ["vac__create", "vac__str_replace", "vac__view", "vac__run_command", "vac__remove", "vac__view_web_page", "vac__generate_password"]
auto_approve = ["vac__create", "vac__str_replace", "vac__view", "vac__run_command", "vac__remove", "vac__view_web_page", "vac__generate_password"]

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
    set_pty_size(slave_fd, 48, 140)
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
    if not work.exists() or work.read_text(encoding="utf-8") != "beta\n":
        errors.append("work.txt was not created and rewritten to beta by actual MCP file tools")
    if (sandbox / "delete-me.txt").exists():
        errors.append("delete-me.txt still exists after actual MCP remove tool")
    protected = sandbox / "protected.txt"
    if not protected.exists() or protected.read_text(encoding="utf-8") != "protected\n":
        errors.append("protected.txt was changed despite remove binding-mismatch negative path")
    if (sandbox / "should-not-exist.txt").exists():
        errors.append("should-not-exist.txt was created despite denied structured-command negative path")
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
    content_port = free_port()
    web_url = f"http://127.0.0.1:{content_port}/fixture.html"
    server, state = start_provider(provider_port, web_url)
    content_server = start_loopback_content_server(content_port)
    tmp = tempfile.TemporaryDirectory(prefix="vac-real-io-e2e-")
    sandbox = Path(tmp.name)
    keep = args.keep_sandbox
    try:
        config_path = write_sandbox(sandbox, provider_port)
        code, output = run_vac(root, sandbox, config_path, args.timeout)
        text = decode(output)
        clean_text = strip_ansi(text)
        visible = clean_text.lower()
        tool_results_text = "\n".join(state.seen_tool_results)
        evidence_text = (clean_text + "\n" + tool_results_text).replace("\\_", "_")
        evidence_lower = evidence_text.lower()
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
            "visible_run_command": "run_command" in visible or "run command" in visible,
            "visible_remove": "remove" in visible or "delete-me.txt" in visible,
            "visible_view_web_page": "view_web_page" in visible or "view web page" in visible,
            "visible_generate_password": "generate_password" in visible or "generate password" in visible,
            "tool_result_run_command": "VAC_RUN_COMMAND_OK" in evidence_text,
            "tool_result_loopback_web": "VAC_LOOPBACK_WEB_OK" in evidence_text,
            "negative_missing_bound_approval": "vac_bound_approval_required" in evidence_lower,
            "negative_structured_command_rejected": "vac_structured_command_required" in evidence_lower,
            "negative_binding_mismatch_rejected": "vac_bound_approval_binding_mismatch" in evidence_lower,
            "negative_non_loopback_http_rejected": "insecure_url" in evidence_lower,
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
        print("actual_io=create,str_replace,view,generate_password,run_command,remove,view_web_page")
        print("negative_io=missing_bound_approval,structured_command_reject,binding_mismatch,non_loopback_http_reject")
        for name in checks:
            print(f"- {name}")
        return 0
    finally:
        server.shutdown()
        server.server_close()
        content_server.shutdown()
        content_server.server_close()
        if keep:
            print(f"kept_sandbox={sandbox}")
        else:
            tmp.cleanup()


if __name__ == "__main__":
    raise SystemExit(main())
