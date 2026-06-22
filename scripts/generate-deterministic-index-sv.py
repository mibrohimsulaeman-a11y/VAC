#!/usr/bin/env python3
"""Generate a deterministic VAC codebase index without requiring Cargo.

P1 default-index hardening: Rust files are parsed through the real vac-index syn-based rust_ast extractor when available. Non-Rust files and Rust parse/helper failures remain fail-closed static fallback records, with fallback counts surfaced in the manifest. It still does not claim a complete call graph.
"""
from __future__ import annotations

import argparse
import hashlib
import os
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from vac_script_common import canonical_hash, now
from vac_script_common import sha256_bytes as sha_bytes

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / ".vac/index"
EXCLUDE_PARTS = {".git", "target", "node_modules", ".venv", "__pycache__"}
EXCLUDE_PREFIXES = (
    ".vac/index/",
    ".vac/cache/",
    ".vac/registry/compiled/",
    ".vac/registry/runtime/",
    ".vac/registry/status.json",
    ".vac/registry/ledger/",
    ".vac/registry/vector-cache/",
    ".vac/memories/",
    ".vac/exports/",
    ".vac/assessment/",
    ".vac/registry/evidence/",
    ".vac/evidence/",
    ".vac/plans/",
    ".vac/ledger/",
    ".vac/registry/spec-sync/",
    ".vac/registry/sessions/",
    ".vac/registry/approvals/",
    ".vac/db/",
)
EXCLUDE_NAMES = {"SV_VALIDATION.log", "SV_POST_EVIDENCE_VALIDATION.log", "CHECKPOINT_MANIFEST.json", "SANDBOX_HANDOFF.md"}
TEXT_SUFFIXES = {".rs", ".toml", ".yaml", ".yml", ".json", ".jsonl", ".md", ".txt", ".js", ".ts", ".sh", ".lock", ".schema", ".sql"}
LANG = {
    ".rs": "rust",
    ".toml": "toml",
    ".yaml": "yaml",
    ".yml": "yaml",
    ".json": "json",
    ".jsonl": "jsonl",
    ".md": "markdown",
    ".js": "javascript",
    ".ts": "typescript",
    ".sh": "shell",
    ".txt": "text",
    ".lock": "lock",
    ".sql": "sql",
}
RUST_SYMBOL_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(async)\s+)?(?:(fn|struct|enum|trait|mod)\s+([A-Za-z_][A-Za-z0-9_]*)|(impl)(?:\s*<[^>]+>)?\s+([^\{]+))"
)
CALL_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")



def role(path: str) -> str:
    if path.startswith(".vac/"):
        return "control_plane"
    if path.startswith("vac-rs/crates/surfaces/"):
        return "surface"
    if path.startswith("vac-rs/crates/runtime/"):
        return "runtime"
    if path.startswith("vac-rs/crates/control-plane/"):
        return "control_plane"
    if path.startswith("vac-rs/crates/foundation/"):
        return "foundation"
    if path.startswith("vac-rs/crates/providers/"):
        return "provider"
    if path.startswith("vac-rs/crates/integrations/"):
        return "integration"
    if path.startswith("vac-cli/"):
        return "js_wrapper"
    if path.startswith("docs/"):
        return "docs"
    if path.startswith("scripts/"):
        return "scripts"
    return "repo"


def is_generated(path: str) -> bool:
    return path.startswith((".vac/registry/", ".vac/assessment/", ".vac/evidence/", ".vac/index/", ".vac/cache/", ".vac/plans/", ".vac/ledger/", ".vac/memories/", ".vac/exports/"))


def include(p: Path) -> bool:
    rel = p.relative_to(ROOT).as_posix()
    if p.is_dir():
        return False
    if any(part in EXCLUDE_PARTS for part in p.parts):
        return False
    if any(rel.startswith(pref) for pref in EXCLUDE_PREFIXES):
        return False
    if p.name in EXCLUDE_NAMES:
        return False
    if p.suffix.lower() in {".png", ".jpg", ".jpeg", ".gif", ".zip", ".db", ".msgpack", ".ico"}:
        return False
    return True


def read_text(p: Path) -> str:
    try:
        return p.read_text(errors="ignore")
    except Exception:
        return ""


def jsonl_write(name: str, rows: list[dict[str, Any]]) -> None:
    (OUT / name).write_text("".join(json.dumps(r, sort_keys=True) + "\n" for r in rows))


def count_lines(text: str) -> int:
    if not text:
        return 1
    return max(1, text.count("\n") + (0 if text.endswith("\n") else 1))


def line_window(text_lines: list[str], start: int, end: int) -> str:
    start_idx = max(0, start - 1)
    end_idx = min(len(text_lines), max(start_idx + 1, end))
    return "\n".join(line.strip() for line in text_lines[start_idx:end_idx])


def ast_symbol_name(kind: str, raw: str) -> str:
    cleaned = raw.strip().replace("where", " ").split("{")[0].strip()
    if kind == "impl":
        cleaned = cleaned.replace("for", "_for_")
        cleaned = re.sub(r"[^A-Za-z0-9_]+", "_", cleaned).strip("_") or "anonymous_impl"
    return cleaned


def rust_symbol_candidates(text: str) -> list[dict[str, Any]]:
    candidates = []
    for lineno, line in enumerate(text.splitlines(), 1):
        match = RUST_SYMBOL_RE.search(line)
        if not match:
            continue
        if match.group(4):
            kind = "impl"
            symbol = ast_symbol_name(kind, match.group(5) or "impl")
        else:
            raw_kind = match.group(2) or "fn"
            kind = "function" if raw_kind == "fn" else {"mod": "module"}.get(raw_kind, raw_kind)
            symbol = match.group(3) or "anonymous"
        candidates.append({"line": lineno, "kind": kind, "symbol": symbol})
    return candidates


def rust_spans(rel: str, text: str, digest: str, total_lines: int) -> list[dict[str, Any]]:
    lines = text.splitlines()
    candidates = rust_symbol_candidates(text)
    spans: list[dict[str, Any]] = []
    for idx, candidate in enumerate(candidates):
        start = int(candidate["line"])
        end = (int(candidates[idx + 1]["line"]) - 1) if idx + 1 < len(candidates) else total_lines
        symbol = str(candidate["symbol"])
        kind = str(candidate["kind"])
        ast_path = f"rust::{rel.replace('/', '::')}::{kind}::{symbol}"
        fingerprint = canonical_hash({
            "path": rel,
            "kind": kind,
            "symbol": symbol,
            "window": line_window(lines, start, end),
        })
        spans.append({
            "span_id": f"span:{rel}:{start}-{end}:{kind}:{symbol}",
            "path": rel,
            "start_line": start,
            "end_line": end,
            "ast_path": ast_path,
            "symbol": symbol,
            "kind": kind,
            "normalized_fingerprint": fingerprint,
            "span_sha256": fingerprint,
            "confidence": "high",
            "parser_mode": "rust_static_heuristic_fail_closed",
        })
    spans.insert(0, {
        "span_id": f"span:{rel}:1-{total_lines}",
        "path": rel,
        "start_line": 1,
        "end_line": total_lines,
        "ast_path": "file",
        "symbol": rel,
        "kind": "file",
        "normalized_fingerprint": digest,
        "span_sha256": digest,
        "confidence": "high" if candidates else "moderate",
        "parser_mode": "rust_static_heuristic_fail_closed" if candidates else "static_heuristic_fail_closed",
    })
    return spans


def fallback_spans(rel: str, digest: str, total_lines: int, lang: str) -> list[dict[str, Any]]:
    return [{
        "span_id": f"span:{rel}:1-{total_lines}",
        "path": rel,
        "start_line": 1,
        "end_line": total_lines,
        "ast_path": "file",
        "symbol": rel,
        "kind": "file",
        "normalized_fingerprint": digest,
        "span_sha256": digest,
        "confidence": "moderate" if lang in {"toml", "yaml", "json", "markdown", "shell", "python"} else "low",
        "parser_mode": "static_heuristic_fail_closed",
    }]


def run_rust_ast_helper(root: Path, rust_files: list[str], required: bool) -> tuple[dict[str, dict[str, Any]], str | None]:
    if not rust_files:
        return {}, None

    command = os.environ.get("VAC_INDEX_RUST_AST_HELPER")
    if command:
        cmd = command.split()
    else:
        cmd = [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            str(root / "vac-rs" / "Cargo.toml"),
            "-p",
            "vac-index",
            "--bin",
            "vac-index-rust-ast",
            "--",
            "--root",
            str(root),
        ]

    try:
        result = subprocess.run(
            cmd,
            input="\n".join(rust_files) + "\n",
            cwd=root,
            text=True,
            capture_output=True,
            check=False,
            timeout=int(os.environ.get("VAC_INDEX_RUST_AST_TIMEOUT", "240")),
        )
    except Exception as exc:
        if required:
            raise RuntimeError(f"rust_ast helper unavailable: {exc}") from exc
        return {}, f"rust_ast helper unavailable: {exc}"

    if result.returncode != 0:
        reason = f"rust_ast helper failed rc={result.returncode}: {result.stderr.strip() or result.stdout.strip()}"
        if required:
            raise RuntimeError(reason)
        return {}, reason

    by_path: dict[str, dict[str, Any]] = {}
    for line in result.stdout.splitlines():
        if not line.strip():
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError as exc:
            reason = f"rust_ast helper emitted invalid JSON: {exc}"
            if required:
                raise RuntimeError(reason) from exc
            return {}, reason
        rel = str(payload.get("path", ""))
        if rel:
            by_path[rel] = payload
    missing = sorted(set(rust_files) - set(by_path))
    if missing:
        reason = f"rust_ast helper omitted {len(missing)} Rust files; first={missing[0]}"
        if required:
            raise RuntimeError(reason)
        return {}, reason
    return by_path, None


def ast_spans(rel: str, digest: str, total_lines: int, ast_index: dict[str, Any]) -> list[dict[str, Any]]:
    spans: list[dict[str, Any]] = [{
        "span_id": f"span:{rel}:1-{total_lines}",
        "path": rel,
        "start_line": 1,
        "end_line": total_lines,
        "ast_path": "file",
        "symbol": rel,
        "kind": "file",
        "normalized_fingerprint": digest,
        "span_sha256": digest,
        "confidence": "high",
        "parser_mode": "rust_ast",
    }]
    for symbol in ast_index.get("symbols", []):
        ast_path = str(symbol.get("ast_path", ""))
        name = str(symbol.get("name", ""))
        kind = str(symbol.get("kind", ""))
        start = int(symbol.get("line_start", 1))
        end = int(symbol.get("line_end", start))
        spans.append({
            "span_id": f"span:{rel}:{start}-{end}:{kind}:{name}",
            "path": rel,
            "start_line": start,
            "end_line": end,
            "ast_path": ast_path,
            "symbol": name,
            "kind": kind,
            "normalized_fingerprint": str(symbol.get("normalized_fingerprint", digest)),
            "span_sha256": str(symbol.get("raw_span_sha256", digest)),
            "confidence": "high",
            "parser_mode": "rust_ast",
        })
    return spans


def add_ast_symbols_relations(
    rel: str,
    ast_index: dict[str, Any],
    spans_for_file: list[dict[str, Any]],
    symbols: list[dict[str, Any]],
    relations: list[dict[str, Any]],
) -> None:
    by_ast_path = {span.get("ast_path"): span for span in spans_for_file if span.get("kind") != "file"}
    for span in spans_for_file:
        if span.get("kind") == "file":
            continue
        symbols.append({
            "symbol_id": f"{rel}:{span['start_line']}:{span['symbol']}",
            "path": rel,
            "line": span["start_line"],
            "kind": f"rust_{span['kind']}",
            "name": span["symbol"],
            "span_id": span["span_id"],
            "ast_path": span["ast_path"],
            "confidence": span["confidence"],
            "parser_mode": "rust_ast",
        })
    for relation in ast_index.get("relations", []):
        source = str(relation.get("source", ""))
        target = str(relation.get("target", ""))
        kind = str(relation.get("relation_kind", ""))
        source_span = by_ast_path.get(source, {}).get("span_id", f"span:{rel}:1-1")
        relations.append({
            "relation_id": f"rel:{rel}:ast:{hashlib.sha256((source + kind + target).encode()).hexdigest()[:16]}",
            "path": rel,
            "line": int(by_ast_path.get(source, {}).get("start_line", 1)),
            "source_span": source_span,
            "relation": kind,
            "relation_kind": kind,
            "target": target,
            "confidence": str(relation.get("confidence", "high")),
            "parser_mode": "rust_ast",
            "status": str(relation.get("status", "SV-Pass")),
        })


def add_rust_risks(rel: str, text: str, spans_for_file: list[dict[str, Any]], risks: list[dict[str, Any]]) -> None:
    for lineno, line in enumerate(text.splitlines(), 1):
        source_span = current_span_id_for_line(spans_for_file, rel, lineno)
        low = line.lower()
        if any(tok in low for tok in ["command::new", "std::process", "tokio::process"]):
            risks.append({"finding_id": f"risk:{rel}:{lineno}:process", "path": rel, "line": lineno, "span_id": source_span, "pattern": "process_execution", "inferred_risk": "execute_process", "confidence": 0.9, "method": "static_heuristic_lightweight"})
        if any(tok in low for tok in ["tokio::net", "reqwest::", "hyper::", "socket", ".get(&url)", "networkaccess"]):
            risks.append({"finding_id": f"risk:{rel}:{lineno}:network", "path": rel, "line": lineno, "span_id": source_span, "pattern": "network_access", "inferred_risk": "network_access", "confidence": 0.82, "method": "static_heuristic_lightweight"})
        if any(tok in low for tok in ["remove_file", "remove_dir", "write(", "create(", "truncate", "rename("]):
            risks.append({"finding_id": f"risk:{rel}:{lineno}:filesystem", "path": rel, "line": lineno, "span_id": source_span, "pattern": "filesystem_mutation", "inferred_risk": "filesystem_write", "confidence": 0.78, "method": "static_heuristic_lightweight"})
        if "unsafe" in low:
            risks.append({"finding_id": f"risk:{rel}:{lineno}:unsafe", "path": rel, "line": lineno, "span_id": source_span, "pattern": "unsafe_rust", "inferred_risk": "unsafe_code", "confidence": 0.7, "method": "static_heuristic_lightweight"})


def current_span_id_for_line(spans: list[dict[str, Any]], rel: str, line: int) -> str:
    candidates = [s for s in spans if s["path"] == rel and int(s.get("start_line", 1)) <= line <= int(s.get("end_line", 1)) and s.get("kind") != "file"]
    if candidates:
        return candidates[0]["span_id"]
    for s in spans:
        if s["path"] == rel:
            return s["span_id"]
    return f"span:{rel}:1-1"


def add_rust_symbols_relations_risks(rel: str, text: str, spans_for_file: list[dict[str, Any]], symbols: list[dict[str, Any]], relations: list[dict[str, Any]], risks: list[dict[str, Any]]) -> None:
    symbol_spans = [s for s in spans_for_file if s.get("kind") != "file"]
    for span in symbol_spans:
        symbols.append({
            "symbol_id": f"{rel}:{span['start_line']}:{span['symbol']}",
            "path": rel,
            "line": span["start_line"],
            "kind": f"rust_{span['kind']}",
            "name": span["symbol"],
            "span_id": span["span_id"],
            "ast_path": span["ast_path"],
            "confidence": span["confidence"],
        })
    for lineno, line in enumerate(text.splitlines(), 1):
        source_span = current_span_id_for_line(spans_for_file, rel, lineno)
        m = re.search(r"^\s*use\s+(.+);", line)
        if m:
            target = m.group(1).strip()
            relations.append({
                "relation_id": f"rel:{rel}:{lineno}:import:{hashlib.sha256(target.encode()).hexdigest()[:12]}",
                "path": rel,
                "line": lineno,
                "source_span": source_span,
                "relation": "import",
                "relation_kind": "imports",
                "target": target,
                "confidence": "high",
            })
        if re.search(r"^\s*impl\b", line):
            relations.append({
                "relation_id": f"rel:{rel}:{lineno}:implements",
                "path": rel,
                "line": lineno,
                "source_span": source_span,
                "relation": "implements",
                "relation_kind": "implements",
                "target": line.strip()[:160],
                "confidence": "moderate",
            })
        for call in CALL_RE.findall(line):
            if call in {"if", "for", "while", "match", "return", "Some", "Ok", "Err", "format", "vec"}:
                continue
            relations.append({
                "relation_id": f"rel:{rel}:{lineno}:call:{call}:{hashlib.sha256(line.encode()).hexdigest()[:8]}",
                "path": rel,
                "line": lineno,
                "source_span": source_span,
                "relation": "call",
                "relation_kind": "calls",
                "target": call,
                "confidence": "moderate",
            })
        low = line.lower()
        if any(tok in low for tok in ["command::new", "std::process", "tokio::process"]):
            risks.append({"finding_id": f"risk:{rel}:{lineno}:process", "path": rel, "line": lineno, "span_id": source_span, "pattern": "process_execution", "inferred_risk": "execute_process", "confidence": 0.9, "method": "static_heuristic_lightweight"})
        if any(tok in low for tok in ["tokio::net", "reqwest::", "hyper::", "socket", ".get(&url)", "networkaccess"]):
            risks.append({"finding_id": f"risk:{rel}:{lineno}:network", "path": rel, "line": lineno, "span_id": source_span, "pattern": "network_access", "inferred_risk": "network_access", "confidence": 0.82, "method": "static_heuristic_lightweight"})
        if any(tok in low for tok in ["remove_file", "remove_dir", "write(", "create(", "truncate", "rename("]):
            risks.append({"finding_id": f"risk:{rel}:{lineno}:filesystem", "path": rel, "line": lineno, "span_id": source_span, "pattern": "filesystem_mutation", "inferred_risk": "filesystem_write", "confidence": 0.78, "method": "static_heuristic_lightweight"})
        if "unsafe" in low:
            risks.append({"finding_id": f"risk:{rel}:{lineno}:unsafe", "path": rel, "line": lineno, "span_id": source_span, "pattern": "unsafe_rust", "inferred_risk": "unsafe_code", "confidence": 0.7, "method": "static_heuristic_lightweight"})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", nargs="?", default=str(ROOT))
    parser.add_argument("--parser", choices=["auto", "static-heuristic", "rust-ast"], default=os.environ.get("VAC_INDEX_PARSER", "auto"))
    args = parser.parse_args()
    selected_parser = args.parser
    OUT.mkdir(parents=True, exist_ok=True)
    files: list[dict[str, Any]] = []
    spans: list[dict[str, Any]] = []
    symbols: list[dict[str, Any]] = []
    relations: list[dict[str, Any]] = []
    risks: list[dict[str, Any]] = []
    read_plans: list[dict[str, Any]] = []

    paths = sorted(x for x in ROOT.rglob("*") if x.is_file() and include(x))
    rust_file_rels = [p.relative_to(ROOT).as_posix() for p in paths if LANG.get(p.suffix.lower(), "unknown") == "rust"]
    rust_ast_results: dict[str, dict[str, Any]] = {}
    rust_ast_unavailable_reason: str | None = None
    if selected_parser in {"auto", "rust-ast"}:
        rust_ast_results, rust_ast_unavailable_reason = run_rust_ast_helper(
            ROOT, rust_file_rels, required=selected_parser == "rust-ast"
        )
    rust_ast_active = bool(rust_ast_results) and selected_parser in {"auto", "rust-ast"}
    active_generator_parser = "rust_ast_default" if rust_ast_active else "static_heuristic_fail_closed"
    rust_ast_lane = "default_active" if rust_ast_active else "fallback_unavailable_fail_closed"
    rust_ast_files = 0
    rust_ast_parse_errors = 0
    rust_fallback_files = 0

    for p in paths:
        rel = p.relative_to(ROOT).as_posix()
        data = p.read_bytes()
        digest = sha_bytes(data)
        lang = LANG.get(p.suffix.lower(), "unknown")
        text = read_text(p)
        line_count = count_lines(text)
        parser_mode = "static_heuristic_fail_closed"
        ast_payload = rust_ast_results.get(rel) if lang == "rust" and rust_ast_active else None
        if lang == "rust" and ast_payload and ast_payload.get("ok") and isinstance(ast_payload.get("index"), dict):
            parser_mode = "rust_ast"
            file_spans = ast_spans(rel, digest, line_count, ast_payload["index"])
            rust_ast_files += 1
        elif lang == "rust":
            parser_mode = "not_parsed_fail_closed" if rust_ast_active else "rust_static_heuristic_fail_closed"
            file_spans = rust_spans(rel, text, digest, line_count)
            rust_fallback_files += 1
            if ast_payload and not ast_payload.get("ok"):
                rust_ast_parse_errors += 1
        else:
            file_spans = fallback_spans(rel, digest, line_count, lang)

        file_record = {
            "path": rel,
            "sha256": digest,
            "language": lang,
            "role": role(rel),
            "generated": is_generated(rel),
            "vendor": False,
            "test": ("/tests/" in rel or rel.startswith("tests/")),
            "bytes": len(data),
            "parser_mode": parser_mode,
        }
        files.append(file_record)
        spans.extend(file_spans)
        for span in file_spans:
            read_plans.append({
                "ticket_id": f"read:{hashlib.sha256(span['span_id'].encode()).hexdigest()[:16]}",
                "path": rel,
                "span_id": span["span_id"],
                "span_sha256": span["span_sha256"],
                "allowed_purpose": "semantic_assessment",
                "line_range": {"start": span["start_line"], "end": min(int(span["end_line"]), int(span["start_line"]) + 240)},
                "confidence": span["confidence"],
            })
        if lang == "rust" and ast_payload and ast_payload.get("ok") and isinstance(ast_payload.get("index"), dict):
            add_ast_symbols_relations(rel, ast_payload["index"], file_spans, symbols, relations)
            add_rust_risks(rel, text, file_spans, risks)
        elif lang == "rust":
            add_rust_symbols_relations_risks(rel, text, file_spans, symbols, relations, risks)

    jsonl_write("files.jsonl", files)
    jsonl_write("spans.jsonl", spans)
    jsonl_write("symbols.jsonl", symbols)
    jsonl_write("relations.jsonl", relations)
    jsonl_write("risks.jsonl", risks)
    jsonl_write("read_plans.jsonl", read_plans)
    repo = {
        "schema_version": 1,
        "kind": "repo_manifest",
        "id": "repo.current",
        "generated_at": now(),
        "deterministic_generated_at": now(),
        "evidence_recorded_at": os.environ.get("VAC_EVIDENCE_RECORDED_AT", now()),
        "workspace_root": "/vac",
        "file_count": len(files),
        "rust_files": sum(1 for f in files if f["language"] == "rust"),
        "parser_contract": "rust_ast_default_with_fail_closed_fallback" if rust_ast_active else "rust_static_heuristic_fail_closed",
        "selected_parser": selected_parser,
        "active_generator_parser": active_generator_parser,
        "rust_ast_lane": rust_ast_lane,
        "rust_ast_files": rust_ast_files,
        "rust_fallback_files": rust_fallback_files,
        "rust_ast_parse_errors": rust_ast_parse_errors,
        "rust_ast_unavailable_reason": rust_ast_unavailable_reason,
    }
    jsonl_write("repo_manifest.jsonl", [repo])
    counts = {
        "files": len(files),
        "symbols": len(symbols),
        "relations": len(relations),
        "risks": len(risks),
        "spans": len(spans),
        "read_plans": len(read_plans),
    }
    manifest = {
        "schema_version": 1,
        "kind": "index_manifest",
        "id": "index.workspace.current",
        "generated_at": now(),
        "deterministic_generated_at": now(),
        "evidence_recorded_at": os.environ.get("VAC_EVIDENCE_RECORDED_AT", now()),
        "outputs": {
            "repo_manifest": ".vac/index/repo_manifest.jsonl",
            "files": ".vac/index/files.jsonl",
            "symbols": ".vac/index/symbols.jsonl",
            "relations": ".vac/index/relations.jsonl",
            "spans": ".vac/index/spans.jsonl",
            "risks": ".vac/index/risks.jsonl",
            "read_plans": ".vac/index/read_plans.jsonl",
        },
        "coverage": {
            "languages": sorted(set(f["language"] for f in files)),
            "low_confidence_files": [f["path"] for f in files if f["parser_mode"] == "static_heuristic_fail_closed" and f["language"] == "unknown"][:200],
            "selected_parser": selected_parser,
            "active_generator_parser": active_generator_parser,
            "rust_ast_lane": rust_ast_lane,
            "rust_ast_mode": "rust_ast_default_with_fail_closed_fallback" if rust_ast_active else "static_heuristic_fail_closed",
            "rust_ast_files": rust_ast_files,
            "rust_ast_parse_errors": rust_ast_parse_errors,
            "rust_fallback_files": rust_fallback_files,
            "rust_ast_unavailable_reason": rust_ast_unavailable_reason,
            "polyglot_mode": "static_heuristic_fail_closed",
            "span_granularity": "file,function,impl,module,type",
            "ast_grounded": rust_ast_active,
            "ast_grounded_default_index": rust_ast_active,
            "fallback_mode": "fail_closed_static_heuristic",
            "relation_granularity": "imports,impls_trait,impls_type,calls_lightweight,read_write_risk",
            "calls_lightweight": "SV-Partial",
            "complete_call_graph": "Not claimed",
        },
        "counts": counts,
    }
    manifest["snapshot_hash"] = canonical_hash({k: v for k, v in manifest.items() if k != "snapshot_hash"})
    (OUT / "index_manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps(counts, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
