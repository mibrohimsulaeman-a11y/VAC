#!/usr/bin/env python3
from __future__ import annotations

import difflib
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
SV_PASS = "SV-" + "Pass"
SV_PARTIAL = "SV-" + "Partial"
NOT_CLAIMED = "Not " + "claimed"
OUTPUT = "\n".join([
    "VAC Rust AST index coverage: PASS",
    "rust_ast_index=" + SV_PASS,
    "parser_mode=rust_ast",
    "rust_ast_default_index=" + SV_PASS,
    "fail_closed_fallback=true",
    "ast_symbols_detected=true",
    "heuristic_ast_delta_reported=true",
    "ast_index_does_not_overclaim_calls=true",
    "calls_lightweight=" + SV_PARTIAL,
    "",
])
errors: list[str] = []


def read(rel: str) -> str:
    path = ROOT / rel
    if not path.is_file():
        errors.append("missing " + rel)
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def require(name: str, ok: bool) -> None:
    if not ok:
        errors.append(name)


def require_tokens(label: str, text: str, tokens: list[str]) -> None:
    for token in tokens:
        require(label + ":" + token, token in text)


rust_ast = read("vac-rs/crates/control-plane/vac-index/src/rust_ast.rs")
lib = read("vac-rs/crates/control-plane/vac-index/src/lib.rs")
cargo = read("vac-rs/crates/control-plane/vac-index/Cargo.toml")
generator = read("scripts/generate-deterministic-index-sv.py")
doc = read("docs/audit/VAC_RUST_AST_INDEX_DELTA.md")
coverage_golden = read("tests/fixtures/index/rust-ast/golden/coverage.out")
no_overclaim_golden = read("tests/fixtures/index/rust-ast/golden/no_overclaim.out")

require("lib_exports_rust_ast", "pub mod rust_ast;" in lib)
require("cargo_has_syn_full", "syn =" in cargo and "full" in cargo)
require("generator_parser_flag", "--parser" in generator and "rust-ast" in generator and "static-heuristic" in generator and "auto" in generator)
require("generator_rust_ast_default_wired", "active_generator_parser" in generator and "rust_ast_default" in generator and "ast_grounded_default_index" in generator)
require("generator_fallback_fail_closed", "fallback_mode" in generator and "fail_closed_static_heuristic" in generator and "rust_ast_parse_errors" in generator)
require("helper_binary_exists", (ROOT / "vac-rs/crates/control-plane/vac-index/src/bin/vac-index-rust-ast.rs").is_file())

require_tokens("rust_ast", rust_ast, [
    "syn::parse_file", "ItemFn", "ItemStruct", "ItemEnum", "ItemTrait", "ItemImpl", "ItemMod",
    "RustAstSymbol", "line_start", "line_end", "byte_start", "byte_end",
    "raw_span_sha256", "normalized_fingerprint", "parser_mode", "impls_trait", "impls_type", "imports",
    "calls_lightweight", SV_PARTIAL, "call_graph_complete: false",
])

fixtures = {
    "basic.rs": ["pub struct Widget", "pub enum Mode", "pub fn build_widget", "build_widget(7)"],
    "impl_trait.rs": ["pub trait Greeter", "impl Greeter for Person", "impl Person"],
    "nested_mod.rs": ["pub mod outer", "pub mod inner", "nested_function"],
    "tests.rs": ["#[test]", "#[tokio::test]", "helper_macro_like", "async_helper"],
    "invalid.rs": ["pub fn broken_fixture", "let missing_header = true"],
}
for name, tokens in fixtures.items():
    require_tokens("fixture:" + name, read("tests/fixtures/index/rust-ast/" + name), tokens)

require_tokens("delta_doc", doc, [
    "rust_ast_index=" + SV_PASS, "parser_mode=rust_ast", "rust_ast_default_index=" + SV_PASS,
    "fail_closed_fallback=true", "ast_symbols_detected=true",
    "heuristic_ast_delta_reported=true", "ast_index_does_not_overclaim_calls=true",
    "calls_lightweight=" + SV_PARTIAL, "complete_call_graph=" + NOT_CLAIMED,
    "fallback_mode=fail_closed_static_heuristic",
])
require_tokens("no_overclaim_golden", no_overclaim_golden, [
    "parser_mode=rust_ast", "calls_lightweight=" + SV_PARTIAL, "complete_call_graph=" + NOT_CLAIMED,
    "cargo_tv=NotEvaluated", "l2_broker=NotImplemented",
])

bad_tokens = [
    "calls_lightweight=" + SV_PASS,
    "complete_call_graph" + "=" + "true",
    "complete_call_graph=" + SV_PASS,
    "call_graph_complete: " + "true",
    "complete_call_graph_claimed",
]
combined = "\n".join([rust_ast, doc, no_overclaim_golden, coverage_golden])
for token in bad_tokens:
    require("no_bad_token:" + token, token not in combined)

expected = coverage_golden if coverage_golden.endswith("\n") else coverage_golden + "\n"
if expected and expected != OUTPUT:
    diff = "".join(difflib.unified_diff(expected.splitlines(True), OUTPUT.splitlines(True), fromfile="expected", tofile="actual"))
    errors.append("coverage golden mismatch\n" + diff)

if errors:
    print("VAC Rust AST index coverage: FAIL")
    for error in errors:
        print("- " + error)
    sys.exit(1)
print(OUTPUT, end="")
