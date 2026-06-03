#!/usr/bin/env python3
"""O6.1 runtime panic surface scanner.

Rustc-free, source/static scanner that excludes test paths, inline #[cfg(test)]
modules/items, comments, and string/char/raw-string literals. It reports the
runtime source surface that still needs cargo-backed de-panic refactors.
"""
from pathlib import Path
import json
import re
import sys

LB = chr(123)
RB = chr(125)
NL = chr(10)
PATS = [".unwrap()", ".expect(", "panic!", "todo!", "unimplemented!", "unreachable!"]


def runtime_rs(p: Path) -> bool:
    rel = p.as_posix()
    if not rel.startswith("vac-rs/") or not rel.endswith(".rs"):
        return False
    parts = rel.split("/")
    if any(x in parts for x in ["tests", "benches", "fixtures", "examples"]):
        return False
    nm = p.name
    if nm.endswith("_test.rs") or nm.endswith("_tests.rs") or nm.startswith("test_") or nm.startswith("tests_") or nm == "tests.rs":
        return False
    return True


def strip_comments_and_literals(text: str) -> str:
    out = []
    state = "normal"
    block_depth = 0
    raw_hashes = 0
    esc = False
    i = 0
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if state == "normal":
            if ch == "/" and nxt == "/":
                out.append(" "); out.append(" ")
                state = "line_comment"; i += 2; continue
            if ch == "/" and nxt == "*":
                out.append(" "); out.append(" ")
                state = "block_comment"; block_depth = 1; i += 2; continue
            if ch == "r":
                m = re.match(r'r(#+)?"', text[i:])
                if m:
                    raw_hashes = 0 if m.group(1) is None else len(m.group(1))
                    out.extend(" " * len(m.group(0)))
                    state = "raw_string"; i += len(m.group(0)); continue
            if ch == '"':
                out.append(" "); state = "string"; esc = False; i += 1; continue
            if ch == "'":
                out.append(" "); state = "char"; esc = False; i += 1; continue
            out.append(ch); i += 1; continue
        if state == "line_comment":
            if ch == "\n":
                out.append("\n"); state = "normal"
            else:
                out.append(" ")
            i += 1; continue
        if state == "block_comment":
            if ch == "/" and nxt == "*":
                block_depth += 1; out.append(" "); out.append(" "); i += 2; continue
            if ch == "*" and nxt == "/":
                block_depth -= 1; out.append(" "); out.append(" "); i += 2
                if block_depth <= 0: state = "normal"
                continue
            out.append("\n" if ch == "\n" else " "); i += 1; continue
        if state == "string":
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                state = "normal"
            out.append("\n" if ch == "\n" else " "); i += 1; continue
        if state == "char":
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == "'":
                state = "normal"
            out.append("\n" if ch == "\n" else " "); i += 1; continue
        if state == "raw_string":
            term = '"' + ("#" * raw_hashes)
            if text.startswith(term, i):
                out.extend(" " * len(term)); i += len(term); state = "normal"
            else:
                out.append("\n" if ch == "\n" else " "); i += 1
            continue
    return "".join(out)


def strip_cfg_test(text: str) -> tuple[str, int]:
    lines = text.split(NL)
    out = []
    removed = 0
    i = 0
    n = len(lines)
    while i < n:
        compact = lines[i].replace(" ", "")
        if "cfg(test)" in compact:
            start = i
            j = i
            while j < n and LB not in lines[j] and not re.match(r"\s*(fn|mod)\b", lines[j]):
                j += 1
            if j < n:
                depth = 0
                k = j
                saw_brace = False
                while k < n:
                    depth += lines[k].count(LB) - lines[k].count(RB)
                    if LB in lines[k]:
                        saw_brace = True
                    k += 1
                    if saw_brace and depth <= 0:
                        break
                    if not saw_brace and k > j + 1:
                        break
                removed += max(1, k - start)
                i = k
                continue
        out.append(lines[i])
        i += 1
    return NL.join(out), removed


def count_patterns(text: str) -> dict[str, int]:
    stripped = strip_comments_and_literals(text)
    return {pat: stripped.count(pat) for pat in PATS}


def main() -> int:
    as_json = "--json" in sys.argv
    files = []
    totals = {pat: 0 for pat in PATS}
    raw_totals = {pat: 0 for pat in PATS}
    cfg_removed_lines = 0
    runtime_files = 0
    for p in sorted(Path("vac-rs").rglob("*.rs")):
        if not runtime_rs(p):
            continue
        runtime_files += 1
        text = p.read_text(errors="ignore")
        raw_counts = count_patterns(text)
        stripped_cfg, removed = strip_cfg_test(text)
        cfg_removed_lines += removed
        counts = count_patterns(stripped_cfg)
        for pat in PATS:
            totals[pat] += counts[pat]
            raw_totals[pat] += raw_counts[pat]
        total = sum(counts.values())
        if total:
            files.append({"path": p.as_posix(), "total": total, "counts": counts})
    files.sort(key=lambda x: (-x["total"], x["path"]))
    result = {
        "runtime_files_scanned": runtime_files,
        "cfg_test_lines_removed": cfg_removed_lines,
        "pattern_counts": totals,
        "raw_path_filtered_counts": raw_totals,
        "total_runtime_panic_surface": sum(totals.values()),
        "top_files": files[:25],
        "method": "exclude test paths + inline cfg(test), strip comments and literals, count panic-capable Rust tokens",
    }
    if as_json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print("runtime_files_scanned", runtime_files)
        print("cfg_test_lines_removed", cfg_removed_lines)
        print("pattern path_filtered_runtime source_runtime_minus_cfg_test_no_comments_literals")
        for pat in PATS:
            print(pat, raw_totals[pat], totals[pat])
        print("total_runtime_panic_surface", result["total_runtime_panic_surface"])
        print("top_files")
        for f in files[:10]:
            print(f["total"], f["path"])
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
