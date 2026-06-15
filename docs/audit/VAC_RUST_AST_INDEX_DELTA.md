# VAC Rust AST Index Delta Audit

Status: **rust_ast_index=SV-Pass**, **parser_mode=rust_ast**, **calls_lightweight=SV-Partial**.

This audit records the first P1 Rust AST index lane. It does not replace the default deterministic index generator yet. The default generator remains `static_heuristic_fail_closed`; the Rust AST lane is a parallel `syn`-based prototype in `vac-index` that is validated by fixtures and compared against heuristic coverage.

## Lane comparison

| Axis | static_heuristic lane | rust_ast lane | Status |
| --- | --- | --- | --- |
| Source scope | repo text files, with Rust regex/static extraction | Rust source fixtures and `vac-index::rust_ast` extractor | SV-Pass |
| Entity grounding | line-pattern heuristic | `syn::parse_file` item traversal | SV-Pass |
| Symbols | function/impl/module/type heuristic spans | functions, structs, enums, traits, impl blocks, test functions | SV-Pass |
| Relations | imports/calls/implements/read-write risk heuristic | imports, impls_trait, impls_type, calls_lightweight | SV-Pass for imports/impls; SV-Partial for calls |
| Evidence | line span + hash/fingerprint | path, lines, bytes, raw span hash, normalized fingerprint, parser_mode=rust_ast | SV-Pass |
| Call graph claim | Not complete | Not complete | Not claimed |

## Status tokens

```text
rust_ast_index=SV-Pass
parser_mode=rust_ast
ast_symbols_detected=true
heuristic_ast_delta_reported=true
ast_index_does_not_overclaim_calls=true
calls_lightweight=SV-Partial
complete_call_graph=Not claimed
static_heuristic_default_index=still_active
```

## Non-claims

- `calls_lightweight` is partial evidence only; it must not be promoted to a complete call graph.
- The default `.vac/index` generator is not yet AST-grounded; it only exposes parser selection metadata and keeps `active_generator_parser=static_heuristic_fail_closed`.
- Product assessment must not use this prototype to claim repo-wide Rust AST scoring until the AST lane is wired into the default generator and fixture coverage expands beyond the initial Rust cases.
