#!/usr/bin/env python3
"""Current-state docs/link hygiene gate for VAC v1.9.

This SV gate rejects obsolete upstream package/install links while allowing the
current Vastar-AI repository coordinate and the future Vastar GHCR namespace.
It does not call the network and does not run Cargo.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
ERRORS: list[str] = []
SKIP_PARTS = {".git", "target", "node_modules", "__pycache__"}
SKIP_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".zip", ".db", ".pyc", ".msgpack"}
SKIP_BANNED_REL = {
    "scripts/check-docs-current-state.py",
    "scripts/check-brand-allowlist.py",
    "scripts/vac-sv-deep-validate.py",
    "scripts/sv_static_validate.py",
    "scripts/vac-runtime-agent-e2e-sv.py",
}
BANNED_PATTERNS = [
    (re.compile(r"ghcr\.io/vac/agent", re.I), "obsolete container image namespace"),
    (re.compile(r"docker\s+pull\s+ghcr\.io/vac/agent", re.I), "obsolete container pull instruction"),
    (re.compile(r"github\.com/vac/cli", re.I), "obsolete GitHub org/repo"),
    (re.compile(r"github\.com/vac/vac", re.I), "obsolete GitHub org/repo"),
    (re.compile(r"github\.com/YOUR_USERNAME/agent", re.I), "template clone instruction"),
    (re.compile(r"vac\.gitbook\.io", re.I), "obsolete external docs domain"),
    (re.compile(r"apiv2\.vac\.dev", re.I), "obsolete API endpoint domain"),
    (re.compile(r"api\.vac\.dev", re.I), "obsolete API endpoint domain"),
    (re.compile(r"app\.vac\.dev", re.I), "obsolete app endpoint domain"),
    (re.compile(r"rules\.vac\.dev", re.I), "obsolete rulebook endpoint domain"),
    (re.compile(r"\bcd\s+agent\b", re.I), "legacy checkout directory"),
    (re.compile(r"libs/server", re.I), "legacy server source path"),
    (re.compile(r"libs/gateway", re.I), "legacy gateway source path"),
    (re.compile(r"libs/mcp/server", re.I), "legacy MCP server source path"),
    (re.compile(r"stakpak|stackpak|stakai|stakapk", re.I), "legacy upstream brand literal"),
    (re.compile(r"<VAC_[A-Z0-9_]+>"), "unresolved VAC placeholder link or source coordinate"),
    (re.compile(r"VAC_(SOURCE_REMOTE|RELEASES_TBD|PRODUCT_SITE_TBD|ISSUES_TBD|BOARD_SOURCE_TBD|BROWSER_SOURCE_TBD|WARDEN_SOURCE_TBD)", re.I), "unresolved VAC placeholder token"),
]
REQUIRED_DOC_TOKENS = {
    "README.md": ["VAC v1.9", ".vac/cache/compiled", "TV-Pending / NotEvaluated"],
    "GETTING-STARTED.md": [
        "local-control-plane-first",
        "Runtime components must authorize from compiled snapshot cache/DB state",
        "cargo metadata --manifest-path vac-rs/Cargo.toml",
        ".vac/db/runtime.db",
    ],
    "AGENTS.md": ["Runtime authority is compiled JSON under `.vac/cache/compiled`", "python3 scripts/check-v19-storage-classes.py"],
    "CONTRIBUTING.md": ["https://github.com/Vastar-AI/vac.git", "Runtime tests", "TV-Pending"],
    "docs/workflow-control-plane/VAC_RUNTIME_V19_STORAGE_PACKAGING_CLOSURE_PLAN.md": [
        "tracked authority",
        "runtime.db",
        "source ZIP excludes",
    ],
    "docs/workflow-control-plane/VAC_CURRENT_DEVELOPMENT_STATE.md": [
        "VAC v1.9 current development state",
        "state-export ZIP",
        "TV-Pending / NotEvaluated",
    ],
    "vac-rs/crates/runtime/vac-agent-loop/README.md": [
        "compiled JSON",
        "Pre-command gate",
        "Completion lock",
    ],
}
EXPECTED_SOURCE_TOKENS = {
    "vac-rs/crates/surfaces/vac-cli/src/config/mod.rs": "https://api.vastar.ai/vac",
    "vac-rs/crates/foundation/vac-foundation/src/container.rs": "ghcr.io/vastar-ai/vac",
    "vac-rs/crates/surfaces/vac-tui/src/services/helper_block.rs": "https://github.com/Vastar-AI/vac/issues/new",
    "package.json": "https://github.com/Vastar-AI/vac.git",
    "release.sh": "https://github.com/Vastar-AI/vac/releases",
    "vac-rs/crates/surfaces/vac-cli/src/commands/auto_update.rs": "https://github.com/Vastar-AI/vac",
    "vac-rs/crates/surfaces/vac-cli/src/commands/board.rs": 'owner: Some("Vastar-AI".to_string())',
    "vac-rs/crates/surfaces/vac-cli/src/commands/browser.rs": 'owner: Some("Vastar-AI".to_string())',
    "vac-rs/crates/surfaces/vac-cli/src/commands/mod.rs": "https://github.com/Vastar-AI/vac",
}


def fail(message: str) -> None:
    ERRORS.append(message)


def text_files():
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        if any(part in SKIP_PARTS for part in path.parts):
            continue
        if path.suffix.lower() in SKIP_SUFFIXES:
            continue
        yield path


def check_banned_tokens() -> None:
    for path in text_files():
        rel = path.relative_to(ROOT).as_posix()
        if rel in SKIP_BANNED_REL:
            continue
        text = path.read_text(errors="ignore")
        for lineno, line in enumerate(text.splitlines(), 1):
            for pattern, reason in BANNED_PATTERNS:
                if pattern.search(line):
                    fail(f"{rel}:{lineno}: {reason}: {line.strip()}")


def check_required_docs() -> None:
    for rel, tokens in REQUIRED_DOC_TOKENS.items():
        path = ROOT / rel
        if not path.is_file():
            fail(f"missing current-state doc: {rel}")
            continue
        text = path.read_text(errors="ignore")
        for token in tokens:
            if token not in text:
                fail(f"{rel} missing required current-state token: {token}")


def check_historical_banners() -> None:
    arch = ROOT / "docs/architecture-enhancements"
    if arch.is_dir():
        for path in arch.glob("*.md"):
            if path.name == "README.md":
                continue
            text = path.read_text(errors="ignore")
            if "VAC v1.9 status note" not in text and "VAC v1.5 status note" not in text:
                fail(f"historical architecture doc missing current status note: {path.relative_to(ROOT)}")
    for rel in ["platform-testing/windows-testing-report.md", "platform-testing/autopilot-e2e-tests.md"]:
        path = ROOT / rel
        if path.is_file():
            text = path.read_text(errors="ignore")
            if "VAC v1.9 status note" not in text and "VAC v1.5 status note" not in text:
                fail(f"historical platform doc missing current status note: {rel}")


def check_source_url_constants() -> None:
    for rel, token in EXPECTED_SOURCE_TOKENS.items():
        path = ROOT / rel
        if not path.is_file():
            fail(f"missing source file for URL check: {rel}")
            continue
        if token not in path.read_text(errors="ignore"):
            fail(f"{rel} missing current canonical token: {token}")


def main() -> int:
    check_banned_tokens()
    check_required_docs()
    check_historical_banners()
    check_source_url_constants()
    if ERRORS:
        print("VAC docs current-state gate: FAIL")
        for error in ERRORS[:300]:
            print(" -", error)
        if len(ERRORS) > 300:
            print(f"... {len(ERRORS) - 300} more")
        return 1
    print("VAC docs current-state gate: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
