use serde::{Deserialize, Serialize};
use syn::{
    Attribute, ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct, ItemTrait, ItemUse,
    Type, UseTree,
};

use crate::{normalize_text_for_fingerprint, raw_span_sha256, sha256_hex};

const PARSER_MODE: &str = "rust_ast";
const CALLS_LIGHTWEIGHT_STATUS: &str = "SV-Partial";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustAstIndex {
    pub path: String,
    pub parser_mode: String,
    pub symbols: Vec<RustAstSymbol>,
    pub relations: Vec<RustAstRelation>,
    pub calls_lightweight: String,
    pub call_graph_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustAstSymbol {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub ast_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub raw_span_sha256: String,
    pub normalized_fingerprint: String,
    pub parser_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustAstRelation {
    pub path: String,
    pub source: String,
    pub target: String,
    pub relation_kind: String,
    pub confidence: String,
    pub parser_mode: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RustAstError {
    pub path: String,
    pub parser_mode: String,
    pub message: String,
}

#[must_use]
pub fn rust_ast_parser_mode() -> &'static str {
    PARSER_MODE
}

#[must_use]
pub fn calls_lightweight_status() -> &'static str {
    CALLS_LIGHTWEIGHT_STATUS
}

pub fn extract_rust_ast_index(path: &str, source: &str) -> Result<RustAstIndex, RustAstError> {
    let syntax = syn::parse_file(source).map_err(|err| RustAstError {
        path: path.to_string(),
        parser_mode: PARSER_MODE.to_string(),
        message: err.to_string(),
    })?;
    let mut collector = Collector {
        path,
        source,
        cursor: 0,
        symbols: Vec::new(),
        relations: Vec::new(),
    };
    collector.collect_items(&syntax.items, "crate");
    Ok(RustAstIndex {
        path: path.to_string(),
        parser_mode: PARSER_MODE.to_string(),
        symbols: collector.symbols,
        relations: collector.relations,
        calls_lightweight: CALLS_LIGHTWEIGHT_STATUS.to_string(),
        call_graph_complete: false,
    })
}

struct Collector<'a> {
    path: &'a str,
    source: &'a str,
    cursor: usize,
    symbols: Vec<RustAstSymbol>,
    relations: Vec<RustAstRelation>,
}

impl Collector<'_> {
    fn collect_items(&mut self, items: &[Item], ast_prefix: &str) {
        for item in items {
            match item {
                Item::Use(item_use) => self.collect_use(item_use, ast_prefix),
                Item::Fn(item_fn) => self.collect_fn(item_fn, ast_prefix),
                Item::Struct(item_struct) => self.collect_struct(item_struct, ast_prefix),
                Item::Enum(item_enum) => self.collect_enum(item_enum, ast_prefix),
                Item::Trait(item_trait) => self.collect_trait(item_trait, ast_prefix),
                Item::Impl(item_impl) => self.collect_impl(item_impl, ast_prefix),
                Item::Mod(item_mod) => self.collect_mod(item_mod, ast_prefix),
                _ => {}
            }
        }
    }

    fn collect_use(&mut self, item: &ItemUse, ast_prefix: &str) {
        let target = use_tree_to_string(&item.tree);
        let source = format!("{ast_prefix}::use::{target}");
        self.relations.push(RustAstRelation {
            path: self.path.to_string(),
            source,
            target,
            relation_kind: "imports".to_string(),
            confidence: "high".to_string(),
            parser_mode: PARSER_MODE.to_string(),
            status: "SV-Pass".to_string(),
        });
    }

    fn collect_fn(&mut self, item: &ItemFn, ast_prefix: &str) {
        let name = item.sig.ident.to_string();
        let kind = function_kind(&item.attrs);
        let ast_path = format!("{ast_prefix}::{kind}::{name}");
        if let Some(symbol) = self.make_symbol("fn", &name, kind, &ast_path) {
            self.collect_lightweight_calls(&symbol, &name);
            self.symbols.push(symbol);
        }
    }

    fn collect_struct(&mut self, item: &ItemStruct, ast_prefix: &str) {
        let name = item.ident.to_string();
        let ast_path = format!("{ast_prefix}::struct::{name}");
        if let Some(symbol) = self.make_symbol("struct", &name, "struct", &ast_path) {
            self.symbols.push(symbol);
        }
    }

    fn collect_enum(&mut self, item: &ItemEnum, ast_prefix: &str) {
        let name = item.ident.to_string();
        let ast_path = format!("{ast_prefix}::enum::{name}");
        if let Some(symbol) = self.make_symbol("enum", &name, "enum", &ast_path) {
            self.symbols.push(symbol);
        }
    }

    fn collect_trait(&mut self, item: &ItemTrait, ast_prefix: &str) {
        let name = item.ident.to_string();
        let ast_path = format!("{ast_prefix}::trait::{name}");
        if let Some(symbol) = self.make_symbol("trait", &name, "trait", &ast_path) {
            self.symbols.push(symbol);
        }
    }

    fn collect_mod(&mut self, item: &ItemMod, ast_prefix: &str) {
        let name = item.ident.to_string();
        let ast_path = format!("{ast_prefix}::mod::{name}");
        if let Some(symbol) = self.make_symbol("mod", &name, "module", &ast_path) {
            self.symbols.push(symbol);
        }
        if let Some((_, items)) = &item.content {
            self.collect_items(items, &format!("{ast_prefix}::{name}"));
        }
    }

    fn collect_impl(&mut self, item: &ItemImpl, ast_prefix: &str) {
        let type_name = type_name(&item.self_ty);
        let trait_name = item
            .trait_
            .as_ref()
            .and_then(|(_, path, _)| path.segments.last().map(|seg| seg.ident.to_string()));
        let impl_name = trait_name
            .as_ref()
            .map(|name| format!("impl_{name}_for_{type_name}"))
            .unwrap_or_else(|| format!("impl_{type_name}"));
        let ast_path = format!("{ast_prefix}::impl::{impl_name}");
        let impl_symbol = self.make_impl_symbol(&impl_name, &ast_path);
        if let Some(symbol) = impl_symbol.as_ref() {
            self.relations.push(RustAstRelation {
                path: self.path.to_string(),
                source: symbol.ast_path.clone(),
                target: type_name.clone(),
                relation_kind: "impls_type".to_string(),
                confidence: "high".to_string(),
                parser_mode: PARSER_MODE.to_string(),
                status: "SV-Pass".to_string(),
            });
            if let Some(name) = trait_name.as_ref() {
                self.relations.push(RustAstRelation {
                    path: self.path.to_string(),
                    source: symbol.ast_path.clone(),
                    target: name.clone(),
                    relation_kind: "impls_trait".to_string(),
                    confidence: "high".to_string(),
                    parser_mode: PARSER_MODE.to_string(),
                    status: "SV-Pass".to_string(),
                });
            }
        }
        if let Some(symbol) = impl_symbol {
            self.symbols.push(symbol);
        }

        for impl_item in &item.items {
            if let ImplItem::Fn(method) = impl_item {
                let name = method.sig.ident.to_string();
                let kind = function_kind(&method.attrs);
                let method_ast_path = format!("{ast_path}::{kind}::{name}");
                if let Some(symbol) = self.make_symbol("fn", &name, kind, &method_ast_path) {
                    self.collect_lightweight_calls(&symbol, &name);
                    self.symbols.push(symbol);
                }
            }
        }
    }

    fn make_impl_symbol(&mut self, name: &str, ast_path: &str) -> Option<RustAstSymbol> {
        self.locate_keyword("impl", None)
            .map(|range| self.symbol_from_range(name, "impl", ast_path, range))
    }

    fn make_symbol(
        &mut self,
        keyword: &str,
        name: &str,
        kind: &str,
        ast_path: &str,
    ) -> Option<RustAstSymbol> {
        self.locate_keyword(keyword, Some(name))
            .map(|range| self.symbol_from_range(name, kind, ast_path, range))
    }

    fn symbol_from_range(
        &self,
        name: &str,
        kind: &str,
        ast_path: &str,
        range: (usize, usize),
    ) -> RustAstSymbol {
        let (byte_start, byte_end) = range;
        let span = self.source.get(byte_start..byte_end).unwrap_or_default();
        let normalized = normalize_text_for_fingerprint(span);
        RustAstSymbol {
            path: self.path.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            ast_path: ast_path.to_string(),
            line_start: line_for_byte(self.source, byte_start),
            line_end: line_for_byte(self.source, byte_end.saturating_sub(1)),
            byte_start,
            byte_end,
            raw_span_sha256: raw_span_sha256(span.as_bytes()),
            normalized_fingerprint: sha256_hex(normalized.as_bytes()),
            parser_mode: PARSER_MODE.to_string(),
        }
    }

    fn locate_keyword(&mut self, keyword: &str, name: Option<&str>) -> Option<(usize, usize)> {
        let needle = name
            .map(|value| format!("{keyword} {value}"))
            .unwrap_or_else(|| keyword.to_string());
        let relative = self.source.get(self.cursor..)?.find(&needle);
        let found = relative
            .map(|offset| self.cursor + offset)
            .or_else(|| self.source.find(&needle))?;
        let start = line_start(self.source, found);
        let end = item_end(self.source, found + needle.len());
        self.cursor = end;
        Some((start, end))
    }

    fn collect_lightweight_calls(&mut self, symbol: &RustAstSymbol, self_name: &str) {
        let Some(span) = self.source.get(symbol.byte_start..symbol.byte_end) else {
            return;
        };
        for target in lightweight_calls(span) {
            if target == self_name {
                continue;
            }
            self.relations.push(RustAstRelation {
                path: self.path.to_string(),
                source: symbol.ast_path.clone(),
                target,
                relation_kind: "calls_lightweight".to_string(),
                confidence: "low".to_string(),
                parser_mode: PARSER_MODE.to_string(),
                status: CALLS_LIGHTWEIGHT_STATUS.to_string(),
            });
        }
    }
}

fn function_kind(attrs: &[Attribute]) -> &'static str {
    if attrs.iter().any(is_test_attr) {
        "test_function"
    } else {
        "function"
    }
}

fn is_test_attr(attr: &Attribute) -> bool {
    let path = attr.path();
    path.is_ident("test")
        || path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
            == "tokio::test"
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_else(|| "unknown_type".to_string()),
        _ => "unknown_type".to_string(),
    }
}

fn use_tree_to_string(tree: &UseTree) -> String {
    match tree {
        UseTree::Path(path) => format!("{}::{}", path.ident, use_tree_to_string(&path.tree)),
        UseTree::Name(name) => name.ident.to_string(),
        UseTree::Rename(rename) => format!("{} as {}", rename.ident, rename.rename),
        UseTree::Glob(_) => "*".to_string(),
        UseTree::Group(group) => group
            .items
            .iter()
            .map(use_tree_to_string)
            .collect::<Vec<_>>()
            .join(","),
    }
}

fn line_start(source: &str, byte: usize) -> usize {
    source[..byte.min(source.len())]
        .rfind('\n')
        .map(|idx| idx + 1)
        .unwrap_or(0)
}

fn line_for_byte(source: &str, byte: usize) -> usize {
    source[..byte.min(source.len())]
        .bytes()
        .filter(|value| *value == b'\n')
        .count()
        + 1
}

fn item_end(source: &str, after_header: usize) -> usize {
    let tail = source.get(after_header..).unwrap_or_default();
    let brace = tail.find('{').map(|idx| after_header + idx);
    let semi = tail.find(';').map(|idx| after_header + idx + 1);
    match (brace, semi) {
        (Some(open), Some(semi_idx)) if semi_idx < open => line_end(source, semi_idx),
        (Some(open), _) => {
            matching_brace(source, open).map_or_else(|| line_end(source, open), |idx| idx + 1)
        }
        (_, Some(semi_idx)) => line_end(source, semi_idx),
        _ => source.len(),
    }
}

fn line_end(source: &str, byte: usize) -> usize {
    source
        .get(byte..)
        .and_then(|tail| tail.find('\n').map(|idx| byte + idx + 1))
        .unwrap_or(source.len())
}

fn matching_brace(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in source.char_indices().skip_while(|(idx, _)| *idx < open) {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn lightweight_calls(span: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let chars: Vec<(usize, char)> = span.char_indices().collect();
    let mut pos = 0usize;
    while pos < chars.len() {
        let (_, ch) = chars[pos];
        if ch == '_' || ch.is_ascii_alphabetic() {
            let start = pos;
            pos += 1;
            while pos < chars.len() && (chars[pos].1 == '_' || chars[pos].1.is_ascii_alphanumeric())
            {
                pos += 1;
            }
            let name_start = chars[start].0;
            let name_end = chars.get(pos).map(|(idx, _)| *idx).unwrap_or(span.len());
            let ident = &span[name_start..name_end];
            let mut lookahead = pos;
            while lookahead < chars.len() && chars[lookahead].1.is_whitespace() {
                lookahead += 1;
            }
            if lookahead < chars.len()
                && chars[lookahead].1 == '('
                && !is_call_keyword(ident)
                && !calls.iter().any(|item| item == ident)
            {
                calls.push(ident.to_string());
            }
        } else {
            pos += 1;
        }
    }
    calls
}

fn is_call_keyword(value: &str) -> bool {
    matches!(
        value,
        "if" | "while" | "for" | "match" | "loop" | "return" | "Self"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(index: &RustAstIndex) -> Vec<String> {
        index
            .symbols
            .iter()
            .map(|symbol| symbol.name.clone())
            .collect()
    }

    #[test]
    fn detects_basic_rust_entities_and_evidence() {
        let source = include_str!("../../../../../tests/fixtures/index/rust-ast/basic.rs");
        let index = extract_rust_ast_index("tests/fixtures/index/rust-ast/basic.rs", source)
            .map_err(|err| err.message)
            .unwrap_or_else(|message| panic!("parse failed: {message}"));
        let names = names(&index);
        assert!(names.contains(&"Widget".to_string()));
        assert!(names.contains(&"Mode".to_string()));
        assert!(names.contains(&"build_widget".to_string()));
        assert!(
            index
                .symbols
                .iter()
                .all(|symbol| symbol.parser_mode == "rust_ast")
        );
        assert!(
            index
                .symbols
                .iter()
                .all(|symbol| symbol.byte_end > symbol.byte_start)
        );
        assert!(
            index
                .symbols
                .iter()
                .all(|symbol| symbol.raw_span_sha256.starts_with("sha256:"))
        );
    }

    #[test]
    fn detects_trait_and_inherent_impl_relations() {
        let source = include_str!("../../../../../tests/fixtures/index/rust-ast/impl_trait.rs");
        let index = extract_rust_ast_index("tests/fixtures/index/rust-ast/impl_trait.rs", source)
            .map_err(|err| err.message)
            .unwrap_or_else(|message| panic!("parse failed: {message}"));
        assert!(
            index
                .relations
                .iter()
                .any(|rel| rel.relation_kind == "impls_trait" && rel.target == "Greeter")
        );
        assert!(
            index
                .relations
                .iter()
                .any(|rel| rel.relation_kind == "impls_type" && rel.target == "Person")
        );
    }

    #[test]
    fn detects_nested_module_functions() {
        let source = include_str!("../../../../../tests/fixtures/index/rust-ast/nested_mod.rs");
        let index = extract_rust_ast_index("tests/fixtures/index/rust-ast/nested_mod.rs", source)
            .map_err(|err| err.message)
            .unwrap_or_else(|message| panic!("parse failed: {message}"));
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| symbol.name == "outer" && symbol.kind == "module")
        );
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| symbol.name == "inner" && symbol.kind == "module")
        );
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| symbol.name == "nested_function" && symbol.kind == "function")
        );
    }

    #[test]
    fn marks_test_functions_and_partial_calls() {
        let source = include_str!("../../../../../tests/fixtures/index/rust-ast/tests.rs");
        let index = extract_rust_ast_index("tests/fixtures/index/rust-ast/tests.rs", source)
            .map_err(|err| err.message)
            .unwrap_or_else(|message| panic!("parse failed: {message}"));
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| symbol.name == "unit_works" && symbol.kind == "test_function")
        );
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| symbol.name == "async_works" && symbol.kind == "test_function")
        );
        assert_eq!(index.calls_lightweight, "SV-Partial");
        assert!(!index.call_graph_complete);
        assert!(
            index
                .relations
                .iter()
                .all(|rel| rel.relation_kind != "complete_call_graph")
        );
    }
}
