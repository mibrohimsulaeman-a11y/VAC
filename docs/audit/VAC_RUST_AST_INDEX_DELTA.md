# VAC Rust AST Index Delta Audit

Status: **rust_ast_index=SV-Pass**, **parser_mode=rust_ast**, **rust_ast_default_index=SV-Pass**, **calls_lightweight=SV-Partial**.

This audit records the P1 Rust AST index closure where the default deterministic index generator now uses the `vac-index` `syn`-based `rust_ast` extractor for Rust files when the helper is available. Non-Rust files remain static fail-closed records, and Rust parse/helper failures downgrade to `not_parsed_fail_closed` / static fallback records instead of being reported as clean absence.

## Lane comparison

| Axis | previous static_heuristic lane | rust_ast default lane | Status |
| --- | --- | --- | --- |
| Source scope | repo text files, with Rust regex/static extraction | Rust files through `vac-index::rust_ast`; non-Rust through fail-closed fallback | SV-Pass |
| Entity grounding | line-pattern heuristic | `syn::parse_file` item traversal | SV-Pass |
| Symbols | function/impl/module/type heuristic spans | functions, structs, enums, traits, impl blocks, test functions | SV-Pass |
| Relations | imports/calls/implements/read-write risk heuristic | imports, impls_trait, impls_type, calls_lightweight | SV-Pass for imports/impls; SV-Partial for calls |
| Fallback | implicit heuristic default | explicit `fallback_mode=fail_closed_static_heuristic`; invalid Rust fixture downgrades to `not_parsed_fail_closed` | SV-Pass |
| Evidence | line span + hash/fingerprint | path, lines, bytes, raw span hash, normalized fingerprint, parser_mode=rust_ast | SV-Pass |
| Call graph claim | Not complete | Not complete | Not claimed |

## Status tokens

```text
rust_ast_index=SV-Pass
parser_mode=rust_ast
rust_ast_default_index=SV-Pass
fail_closed_fallback=true
ast_symbols_detected=true
heuristic_ast_delta_reported=true
ast_index_does_not_overclaim_calls=true
calls_lightweight=SV-Partial
complete_call_graph=Not claimed
fallback_mode=fail_closed_static_heuristic
```

## Non-claims

- `calls_lightweight` is partial evidence only; it must not be promoted to a complete call graph.
- The default index is AST-grounded for Rust symbols/spans when the helper is available; it is not an all-language AST index.
- Rust parse/helper failures are explicit fallback evidence. They must lower confidence for affected files and must not support an `absent` missing-code proof.
- Product assessment must not claim complete repo-wide call-graph coverage until call extraction is upgraded beyond `calls_lightweight`.
