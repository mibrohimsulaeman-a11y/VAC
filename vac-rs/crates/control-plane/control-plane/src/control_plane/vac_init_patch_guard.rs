#![allow(dead_code)]
//! Bounded patch guard contract for VAC-Init Semantic Plans.
//!
//! The strict semantic-anchor resolver is backed by `syn::parse_file`. The old
//! line-only fallback remains available only when a caller explicitly requests
//! `SemanticAnchorMode::DegradedLineHeuristic`; strict mode fails closed when the
//! Rust parser cannot build an AST or when an anchor is missing/ambiguous.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatchOperation {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchLineRange {
    pub start: usize,
    pub end: usize,
}

impl PatchLineRange {
    pub const fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedPatchScope {
    pub path: String,
    pub operation: PatchOperation,
    pub line_range: Option<PatchLineRange>,
    pub semantic_anchor: Option<String>,
    pub ownership: String,
    pub allow_new_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchBudget {
    pub max_patches: usize,
    pub max_new_files: usize,
    pub max_line_delta: isize,
    pub patches_used: usize,
    pub new_files_used: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchAttempt {
    pub path: String,
    pub operation: PatchOperation,
    pub line_range: Option<PatchLineRange>,
    pub semantic_anchor_resolved: bool,
    pub ownership: String,
    pub creates_new_file: bool,
    pub lines_added: usize,
    pub lines_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchGuardIssue {
    pub code: String,
    pub message: String,
}

impl PatchGuardIssue {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchGuardReport {
    pub allowed: bool,
    pub issues: Vec<PatchGuardIssue>,
}

impl PatchGuardReport {
    pub fn pass() -> Self {
        Self {
            allowed: true,
            issues: Vec::new(),
        }
    }

    pub fn fail(issues: Vec<PatchGuardIssue>) -> Self {
        Self {
            allowed: false,
            issues,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchGuardContext {
    pub scopes: BTreeMap<String, ApprovedPatchScope>,
    pub budget: PatchBudget,
    pub forbidden_globs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSemanticAnchor {
    pub requested: String,
    pub kind: String,
    pub name: String,
    pub line_range: PatchLineRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticAnchorResolutionError {
    EmptyAnchor,
    NotFound(String),
    Ambiguous { anchor: String, matches: usize },
}

pub fn validate_patch_attempt_with_semantic_source(
    ctx: &PatchGuardContext,
    attempt: &PatchAttempt,
    sources: &BTreeMap<String, String>,
) -> PatchGuardReport {
    let mut prepared_ctx = ctx.clone();
    let mut prepared = attempt.clone();
    if let Some(scope) = prepared_ctx.scopes.get_mut(&attempt.path)
        && scope.line_range.is_none()
        && let Some(anchor) = &scope.semantic_anchor
    {
        if let Some(source) = sources.get(&attempt.path) {
            match resolve_semantic_anchor_in_source(anchor, source) {
                Ok(resolved) => {
                    prepared.semantic_anchor_resolved = true;
                    scope.line_range = Some(resolved.line_range.clone());
                    if prepared.line_range.is_none() {
                        prepared.line_range = Some(resolved.line_range);
                    }
                }
                Err(_) => {
                    prepared.semantic_anchor_resolved = false;
                }
            }
        } else {
            prepared.semantic_anchor_resolved = false;
        }
    }
    validate_patch_attempt(&prepared_ctx, &prepared)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticAnchorMode {
    /// Strict policy: only Rust AST-backed item candidates are accepted; fallback line heuristics are rejected.
    StrictAst,
    /// Degraded policy: use the line heuristic only for unsupported migrations and surface the degraded kind.
    DegradedLineHeuristic,
}

pub trait SemanticAnchorResolver {
    fn language(&self) -> &'static str;
    fn resolve(
        &self,
        anchor: &str,
        source: &str,
    ) -> Result<ResolvedSemanticAnchor, SemanticAnchorResolutionError>;
}

#[derive(Debug, Clone, Copy, Default)]
/// Strict syn-backed resolver covers doc comments and stacked attributes before declarations.
pub struct RustSynAnchorResolver;

/// Compatibility alias for older imports. It delegates to the strict `syn` backend.
#[derive(Debug, Clone, Copy, Default)]
pub struct RustAstAnchorResolver;

#[derive(Debug, Clone, Copy, Default)]
pub struct LineHeuristicAnchorResolver;

pub fn resolve_semantic_anchor_in_source(
    anchor: &str,
    source: &str,
) -> Result<ResolvedSemanticAnchor, SemanticAnchorResolutionError> {
    resolve_semantic_anchor_with_mode(anchor, source, SemanticAnchorMode::StrictAst)
}

pub fn resolve_semantic_anchor_in_source_strict(
    anchor: &str,
    source: &str,
) -> Result<ResolvedSemanticAnchor, SemanticAnchorResolutionError> {
    resolve_semantic_anchor_with_mode(anchor, source, SemanticAnchorMode::StrictAst)
}

pub fn resolve_semantic_anchor_with_mode(
    anchor: &str,
    source: &str,
    mode: SemanticAnchorMode,
) -> Result<ResolvedSemanticAnchor, SemanticAnchorResolutionError> {
    match mode {
        SemanticAnchorMode::StrictAst => RustSynAnchorResolver.resolve(anchor, source),
        SemanticAnchorMode::DegradedLineHeuristic => {
            LineHeuristicAnchorResolver.resolve(anchor, source)
        }
    }
}

impl SemanticAnchorResolver for RustSynAnchorResolver {
    fn language(&self) -> &'static str {
        "rust-syn"
    }

    fn resolve(
        &self,
        anchor: &str,
        source: &str,
    ) -> Result<ResolvedSemanticAnchor, SemanticAnchorResolutionError> {
        resolve_rust_semantic_anchor(anchor, source, rust_syn_item_candidates)
    }
}

impl SemanticAnchorResolver for RustAstAnchorResolver {
    fn language(&self) -> &'static str {
        "rust-syn-compat"
    }

    fn resolve(
        &self,
        anchor: &str,
        source: &str,
    ) -> Result<ResolvedSemanticAnchor, SemanticAnchorResolutionError> {
        RustSynAnchorResolver.resolve(anchor, source)
    }
}

impl SemanticAnchorResolver for LineHeuristicAnchorResolver {
    fn language(&self) -> &'static str {
        "line-heuristic-degraded"
    }

    fn resolve(
        &self,
        anchor: &str,
        source: &str,
    ) -> Result<ResolvedSemanticAnchor, SemanticAnchorResolutionError> {
        resolve_rust_semantic_anchor(anchor, source, legacy_line_heuristic_item_candidates)
    }
}

fn parse_anchor(anchor: &str) -> (Option<String>, String) {
    let trimmed = anchor.trim();
    if let Some((kind, name)) = trimmed.split_once(':') {
        let kind = kind.trim();
        let name = name.trim();
        return (
            (!kind.is_empty()).then(|| kind.to_string()),
            name.to_string(),
        );
    }
    let mut parts = trimmed.split_whitespace();
    if let (Some(kind), Some(name), None) = (parts.next(), parts.next(), parts.next())
        && matches!(
            kind,
            "fn" | "method"
                | "struct"
                | "enum"
                | "trait"
                | "impl"
                | "mod"
                | "module"
                | "type"
                | "const"
                | "static"
                | "symbol"
        )
    {
        return (Some(kind.to_string()), name.to_string());
    }
    (None, trimmed.to_string())
}

fn resolve_rust_semantic_anchor(
    anchor: &str,
    source: &str,
    candidate_fn: fn(&str) -> Vec<RustAstItemCandidate>,
) -> Result<ResolvedSemanticAnchor, SemanticAnchorResolutionError> {
    let requested = anchor.trim();
    if requested.is_empty() {
        return Err(SemanticAnchorResolutionError::EmptyAnchor);
    }
    let (kind_filter, name_filter) = parse_anchor(requested);
    let mut matches = Vec::new();
    for candidate in candidate_fn(source) {
        let kind_matches = kind_filter.as_ref().is_none_or(|expected| {
            let expected = expected.as_str();
            expected == candidate.kind
                || expected == "symbol"
                || (expected == "module" && candidate.kind == "mod")
        });
        if kind_matches && candidate.name == name_filter {
            matches.push(ResolvedSemanticAnchor {
                requested: requested.to_string(),
                kind: candidate.kind,
                name: candidate.name,
                line_range: candidate.line_range,
            });
        }
    }
    match matches.len() {
        0 => Err(SemanticAnchorResolutionError::NotFound(
            requested.to_string(),
        )),
        1 => Ok(matches.remove(0)),
        n => Err(SemanticAnchorResolutionError::Ambiguous {
            anchor: requested.to_string(),
            matches: n,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustAstItemCandidate {
    kind: String,
    name: String,
    line_range: PatchLineRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SynItemRecord {
    kind: String,
    name: String,
}

fn rust_syn_item_candidates(source: &str) -> Vec<RustAstItemCandidate> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut records = Vec::new();
    collect_syn_item_records(&file.items, None, &mut records);
    records_to_line_candidates(source, records)
}

fn collect_syn_item_records(
    items: &[syn::Item],
    impl_owner: Option<&str>,
    out: &mut Vec<SynItemRecord>,
) {
    for item in items {
        match item {
            syn::Item::Const(item) => push_syn_record(out, "const", &item.ident),
            syn::Item::Enum(item) => push_syn_record(out, "enum", &item.ident),
            syn::Item::Fn(item) => push_syn_record(out, "fn", &item.sig.ident),
            syn::Item::Impl(item) => {
                if let Some(name) = syn_type_anchor_name(&item.self_ty) {
                    out.push(SynItemRecord {
                        kind: "impl".to_string(),
                        name: name.clone(),
                    });
                    for impl_item in &item.items {
                        if let syn::ImplItem::Fn(method) = impl_item {
                            let method_name = method.sig.ident.to_string();
                            out.push(SynItemRecord {
                                kind: "method".to_string(),
                                name: method_name.clone(),
                            });
                            out.push(SynItemRecord {
                                kind: "impl".to_string(),
                                name: method_name.clone(),
                            });
                            out.push(SynItemRecord {
                                kind: "fn".to_string(),
                                name: format!("{name}::{method_name}"),
                            });
                        }
                    }
                }
            }
            syn::Item::Mod(item) => {
                push_syn_record(out, "mod", &item.ident);
                if let Some((_brace, nested)) = &item.content {
                    let owner = item.ident.to_string();
                    collect_syn_item_records(nested, Some(&owner), out);
                }
            }
            syn::Item::Static(item) => push_syn_record(out, "static", &item.ident),
            syn::Item::Struct(item) => push_syn_record(out, "struct", &item.ident),
            syn::Item::Trait(item) => push_syn_record(out, "trait", &item.ident),
            syn::Item::Type(item) => push_syn_record(out, "type", &item.ident),
            _ => {}
        }
    }
    let _ = impl_owner;
}

fn push_syn_record(out: &mut Vec<SynItemRecord>, kind: &str, ident: &syn::Ident) {
    out.push(SynItemRecord {
        kind: kind.to_string(),
        name: ident.to_string(),
    });
}

fn syn_type_anchor_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Type::Reference(reference) => syn_type_anchor_name(&reference.elem),
        syn::Type::Paren(paren) => syn_type_anchor_name(&paren.elem),
        syn::Type::Group(group) => syn_type_anchor_name(&group.elem),
        _ => None,
    }
}

fn records_to_line_candidates(
    source: &str,
    records: Vec<SynItemRecord>,
) -> Vec<RustAstItemCandidate> {
    let mut next_search_line: BTreeMap<(String, String), usize> = BTreeMap::new();
    records
        .into_iter()
        .filter_map(|record| {
            let lookup_name = record
                .name
                .rsplit_once("::")
                .map_or(record.name.as_str(), |(_owner, method)| method);
            let lookup_kind = match record.kind.as_str() {
                "method" => "fn",
                other => other,
            };
            let key = (record.kind.clone(), record.name.clone());
            let from_line = *next_search_line.get(&key).unwrap_or(&1);
            let range = if record.kind == "impl" {
                find_rust_declaration_line_range(source, "impl", lookup_name, from_line).or_else(
                    || find_rust_declaration_line_range(source, "fn", lookup_name, from_line),
                )?
            } else {
                find_rust_declaration_line_range(source, lookup_kind, lookup_name, from_line)?
            };
            next_search_line.insert(key, range.start.saturating_add(1));
            Some(RustAstItemCandidate {
                kind: record.kind,
                name: record.name,
                line_range: range,
            })
        })
        .collect()
}

fn find_rust_declaration_line_range(
    source: &str,
    kind: &str,
    name: &str,
    from_line: usize,
) -> Option<PatchLineRange> {
    let lines: Vec<&str> = source.lines().collect();
    for (idx, raw) in lines.iter().enumerate().skip(from_line.saturating_sub(1)) {
        let line_no = idx + 1;
        if !declaration_line_matches(raw, kind, name) {
            continue;
        }
        let end = find_decl_end_line_from_lines(&lines, idx).unwrap_or(line_no);
        return Some(PatchLineRange {
            start: line_no,
            end: end.max(line_no),
        });
    }
    None
}

fn declaration_line_matches(raw: &str, kind: &str, name: &str) -> bool {
    let line = raw.split("//").next().unwrap_or(raw).trim_start();
    if line.is_empty()
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with("///")
        || line.starts_with("//!")
        || line.starts_with('#')
    {
        return false;
    }
    if kind == "impl" {
        return contains_ident(line, "impl") && contains_ident(line, name);
    }
    declaration_name_after_keyword(line, kind).as_deref() == Some(name)
}

fn declaration_name_after_keyword(raw: &str, kind: &str) -> Option<String> {
    let mut line = raw.trim_start();
    loop {
        let before = line;
        line = line
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub(super) ")
            .trim_start_matches("pub(in crate) ")
            .trim_start_matches("pub ")
            .trim_start_matches("async ")
            .trim_start_matches("unsafe ")
            .trim_start_matches("const ")
            .trim_start_matches("extern \"C\" ")
            .trim_start_matches("extern ");
        if before == line {
            break;
        }
    }
    let rest = line.strip_prefix(kind)?.trim_start();
    let name: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

fn contains_ident(line: &str, ident: &str) -> bool {
    if ident.is_empty() {
        return false;
    }
    let mut pos = 0usize;
    while let Some(offset) = line[pos..].find(ident) {
        let start = pos + offset;
        let end = start + ident.len();
        let bytes = line.as_bytes();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        pos = end;
        if pos >= bytes.len() {
            break;
        }
    }
    false
}

fn is_ident_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn find_decl_end_line_from_lines(lines: &[&str], start_idx: usize) -> Option<usize> {
    let mut depth = 0isize;
    let mut saw_body = false;
    for (idx, raw) in lines.iter().enumerate().skip(start_idx) {
        let code = raw.split("//").next().unwrap_or(raw);
        for ch in code.chars() {
            match ch {
                '{' => {
                    saw_body = true;
                    depth += 1;
                }
                '}' if saw_body => {
                    depth -= 1;
                    if depth <= 0 {
                        return Some(idx + 1);
                    }
                }
                ';' if !saw_body => return Some(idx + 1),
                _ => {}
            }
        }
        if !saw_body && idx > start_idx + 2 {
            return Some(idx + 1);
        }
    }
    Some(start_idx + 1)
}

fn legacy_line_heuristic_item_candidates(source: &str) -> Vec<RustAstItemCandidate> {
    source
        .lines()
        .enumerate()
        .filter_map(|(idx, raw)| {
            let line = raw.trim_start();
            let stripped = line
                .trim_start_matches("pub(crate) ")
                .trim_start_matches("pub(super) ")
                .trim_start_matches("pub ")
                .trim_start_matches("async ")
                .trim_start_matches("unsafe ")
                .trim_start_matches("const ");
            for keyword in ["fn", "struct", "enum", "trait", "mod"] {
                if let Some(rest) = stripped.strip_prefix(keyword) {
                    let name: String = rest
                        .trim_start()
                        .chars()
                        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                        .collect();
                    if !name.is_empty() {
                        return Some(RustAstItemCandidate {
                            kind: format!("{keyword}:degraded"),
                            name,
                            line_range: PatchLineRange {
                                start: idx + 1,
                                end: idx + 1,
                            },
                        });
                    }
                }
            }
            None
        })
        .collect()
}

pub fn validate_patch_attempt(ctx: &PatchGuardContext, attempt: &PatchAttempt) -> PatchGuardReport {
    let mut issues = Vec::new();

    let Some(scope) = ctx.scopes.get(&attempt.path) else {
        issues.push(PatchGuardIssue::new(
            "patch.file.outside_plan",
            format!("file '{}' is not in plan.allowed_files", attempt.path),
        ));
        return PatchGuardReport::fail(issues);
    };

    if glob_forbidden(&attempt.path, &ctx.forbidden_globs) {
        issues.push(PatchGuardIssue::new(
            "patch.file.forbidden",
            format!("file '{}' matches forbidden globs", attempt.path),
        ));
    }
    if scope.operation != attempt.operation {
        issues.push(PatchGuardIssue::new(
            "patch.operation.mismatch",
            "patch operation does not match approved plan operation",
        ));
    }
    if scope.ownership != attempt.ownership {
        issues.push(PatchGuardIssue::new(
            "patch.ownership.mismatch",
            "patch ownership does not match approved scope",
        ));
    }
    if attempt.creates_new_file && !scope.allow_new_file {
        issues.push(PatchGuardIssue::new(
            "patch.new_file.undeclared",
            "new file creation must be declared in the semantic plan",
        ));
    }
    if ctx.budget.patches_used >= ctx.budget.max_patches {
        issues.push(PatchGuardIssue::new(
            "patch.budget.max_patches",
            "patch budget exhausted",
        ));
    }
    if attempt.creates_new_file && ctx.budget.new_files_used >= ctx.budget.max_new_files {
        issues.push(PatchGuardIssue::new(
            "patch.budget.max_new_files",
            "new file budget exhausted",
        ));
    }
    // Bound the *gross* churn (added + removed lines), not just the net delta.
    // A balanced rewrite (e.g. +1000/-1000) has a net delta of zero yet can
    // rewrite an unbounded region, defeating the bounded-patch contract. Gross
    // churn is always >= |net delta|, so this only ever tightens the guard and
    // never admits a patch the previous net-delta check would have rejected.
    let gross_churn = attempt.lines_added.saturating_add(attempt.lines_removed) as isize;
    if gross_churn > ctx.budget.max_line_delta.abs() {
        issues.push(PatchGuardIssue::new(
            "patch.budget.max_line_delta",
            "patch churn exceeds bounded-patch budget",
        ));
    }
    match (&scope.line_range, &attempt.line_range) {
        (Some(approved), Some(actual)) if !approved.contains(actual) => {
            issues.push(PatchGuardIssue::new(
                "patch.range.out_of_bounds",
                "patch line range is outside approved plan range",
            ));
        }
        (Some(_), None) if attempt.operation != PatchOperation::Create => {
            issues.push(PatchGuardIssue::new(
                "patch.range.missing",
                "patch must provide a line range or resolved anchor",
            ));
        }
        _ => {}
    }
    if scope.line_range.is_none()
        && scope.semantic_anchor.is_some()
        && !attempt.semantic_anchor_resolved
    {
        issues.push(PatchGuardIssue::new(
            "patch.anchor.unresolved",
            "semantic anchor could not be resolved; request plan refresh",
        ));
    }

    if issues.is_empty() {
        PatchGuardReport::pass()
    } else {
        PatchGuardReport::fail(issues)
    }
}

fn glob_forbidden(path: &str, patterns: &[String]) -> bool {
    // Mirror `vac_init_semantic_plan::is_forbidden`: the same `plan.forbidden_files`
    // patterns flow into both matchers, so they must agree. `dir/**` matches the
    // directory itself or paths beneath it on a path-segment boundary, never a
    // sibling that merely shares the textual prefix (e.g. `target/**` must not
    // match `targetfoo`).
    let path = path.trim_start_matches("./");
    patterns.iter().any(|pattern| {
        let pattern = pattern.trim().trim_start_matches("./");
        if pattern.is_empty() {
            return false;
        }
        if path == pattern {
            return true;
        }
        if let Some(directory) = pattern.strip_suffix("/**") {
            return path == directory
                || path
                    .strip_prefix(directory)
                    .is_some_and(|suffix| suffix.starts_with('/'));
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return path.starts_with(prefix);
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> PatchGuardContext {
        PatchGuardContext {
            scopes: BTreeMap::from([(
                "src/lib.rs".to_string(),
                ApprovedPatchScope {
                    path: "src/lib.rs".to_string(),
                    operation: PatchOperation::Modify,
                    line_range: Some(PatchLineRange { start: 10, end: 50 }),
                    semantic_anchor: None,
                    ownership: "vac.test.fixture".to_string(),
                    allow_new_file: false,
                },
            )]),
            budget: PatchBudget {
                max_patches: 2,
                max_new_files: 0,
                max_line_delta: 25,
                patches_used: 0,
                new_files_used: 0,
            },
            forbidden_globs: vec!["target/**".to_string()],
        }
    }

    fn attempt() -> PatchAttempt {
        PatchAttempt {
            path: "src/lib.rs".to_string(),
            operation: PatchOperation::Modify,
            line_range: Some(PatchLineRange { start: 12, end: 20 }),
            semantic_anchor_resolved: true,
            ownership: "vac.test.fixture".to_string(),
            creates_new_file: false,
            lines_added: 5,
            lines_removed: 1,
        }
    }

    #[test]
    fn accepts_bounded_patch() {
        let report = validate_patch_attempt(&context(), &attempt());
        assert!(report.allowed, "{:?}", report.issues);
    }

    #[test]
    fn forbidden_glob_matches_on_segment_boundary() {
        let patterns = vec!["target/**".to_string()];
        // A path beneath the forbidden directory is matched.
        assert!(glob_forbidden("target/debug/app", &patterns));
        // The directory itself is matched.
        assert!(glob_forbidden("target", &patterns));
        // A sibling that merely shares the textual prefix is NOT matched.
        assert!(!glob_forbidden("targetfoo/app", &patterns));
    }

    #[test]
    fn rejects_file_outside_plan() {
        let mut attempt = attempt();
        attempt.path = "src/other.rs".to_string();
        let report = validate_patch_attempt(&context(), &attempt);
        assert!(!report.allowed);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.file.outside_plan")
        );
    }

    #[test]
    fn rejects_operation_mismatch() {
        let mut attempt = attempt();
        attempt.operation = PatchOperation::Delete;
        let report = validate_patch_attempt(&context(), &attempt);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.operation.mismatch")
        );
    }

    #[test]
    fn rejects_range_out_of_bounds() {
        let mut attempt = attempt();
        attempt.line_range = Some(PatchLineRange { start: 1, end: 2 });
        let report = validate_patch_attempt(&context(), &attempt);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.range.out_of_bounds")
        );
    }

    #[test]
    fn rejects_new_file_not_declared() {
        let mut attempt = attempt();
        attempt.creates_new_file = true;
        attempt.operation = PatchOperation::Create;
        let report = validate_patch_attempt(&context(), &attempt);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.new_file.undeclared")
        );
    }

    #[test]
    fn rejects_line_delta_budget_exceeded() {
        let mut attempt = attempt();
        attempt.lines_added = 100;
        let report = validate_patch_attempt(&context(), &attempt);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.budget.max_line_delta")
        );
    }

    #[test]
    fn rejects_high_churn_even_with_zero_net_delta() {
        // A balanced rewrite (+1000/-1000) has a net delta of zero but a gross
        // churn of 2000 lines, which must still exhaust the bounded-patch budget.
        let mut attempt = attempt();
        attempt.lines_added = 1000;
        attempt.lines_removed = 1000;
        let report = validate_patch_attempt(&context(), &attempt);
        assert!(!report.allowed);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.budget.max_line_delta")
        );
    }

    #[test]
    fn unresolved_anchor_requires_plan_refresh() {
        let mut ctx = context();
        ctx.scopes.insert(
            "src/anchor.rs".to_string(),
            ApprovedPatchScope {
                path: "src/anchor.rs".to_string(),
                operation: PatchOperation::Modify,
                line_range: None,
                semantic_anchor: Some("run".to_string()),
                ownership: "vac.test.fixture".to_string(),
                allow_new_file: false,
            },
        );
        let mut attempt = attempt();
        attempt.path = "src/anchor.rs".to_string();
        attempt.line_range = None;
        attempt.semantic_anchor_resolved = false;
        let report = validate_patch_attempt(&ctx, &attempt);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.anchor.unresolved")
        );
    }

    #[test]
    fn resolves_function_anchor_to_line_range() {
        let source =
            "pub fn setup() {\n    helper();\n}\n\npub fn run() {\n    println!(\"go\");\n}\n";
        let resolved = resolve_semantic_anchor_in_source("fn:run", source).unwrap();
        assert_eq!(resolved.kind, "fn");
        assert_eq!(resolved.name, "run");
        assert_eq!(resolved.line_range, PatchLineRange { start: 5, end: 7 });
    }

    #[test]
    fn rejects_comment_false_positive() {
        let source = "// fn ghost() {}\npub fn real() {}\n";
        let err = resolve_semantic_anchor_in_source("fn:ghost", source).unwrap_err();
        assert_eq!(
            err,
            SemanticAnchorResolutionError::NotFound("fn:ghost".to_string())
        );
    }

    #[test]
    fn rejects_ambiguous_untyped_anchor() {
        let source = "pub fn run() {}\npub struct run { value: u8 }\n";
        let err = resolve_semantic_anchor_in_source("run", source).unwrap_err();
        assert_eq!(
            err,
            SemanticAnchorResolutionError::Ambiguous {
                anchor: "run".to_string(),
                matches: 2,
            }
        );
    }

    #[test]
    fn resolves_impl_and_method_anchors() {
        let source = "pub struct Worker;\nimpl Worker {\n    pub fn tick(&self) { }\n}\n";
        let impl_anchor = resolve_semantic_anchor_in_source("impl:Worker", source).unwrap();
        assert_eq!(impl_anchor.line_range, PatchLineRange { start: 2, end: 4 });
        let method_anchor = resolve_semantic_anchor_in_source("method:tick", source).unwrap();
        assert_eq!(
            method_anchor.line_range,
            PatchLineRange { start: 3, end: 3 }
        );
        let qualified = resolve_semantic_anchor_in_source("fn:Worker::tick", source).unwrap();
        assert_eq!(qualified.line_range, PatchLineRange { start: 3, end: 3 });
    }

    #[test]
    fn resolves_module_anchor() {
        let source = "pub mod nested {\n    pub fn run() {}\n}\n";
        let resolved = resolve_semantic_anchor_in_source("module:nested", source).unwrap();
        assert_eq!(resolved.kind, "mod");
        assert_eq!(resolved.line_range, PatchLineRange { start: 1, end: 3 });
    }

    #[test]
    fn degraded_fallback_is_not_used_by_strict_mode() {
        let source = "not valid rust but fn fake() {}";
        let strict = resolve_semantic_anchor_in_source("fn:fake", source).unwrap_err();
        assert_eq!(
            strict,
            SemanticAnchorResolutionError::NotFound("fn:fake".to_string())
        );
        let degraded = resolve_semantic_anchor_with_mode(
            "fn:fake",
            source,
            SemanticAnchorMode::DegradedLineHeuristic,
        );
        assert!(degraded.is_err());
    }

    #[test]
    fn semantic_source_validation_resolves_plan_anchor_fail_closed() {
        let mut ctx = context();
        ctx.scopes.insert(
            "src/anchor.rs".to_string(),
            ApprovedPatchScope {
                path: "src/anchor.rs".to_string(),
                operation: PatchOperation::Modify,
                line_range: None,
                semantic_anchor: Some("fn:run".to_string()),
                ownership: "vac.test.fixture".to_string(),
                allow_new_file: false,
            },
        );
        let mut attempt = attempt();
        attempt.path = "src/anchor.rs".to_string();
        attempt.line_range = None;
        attempt.semantic_anchor_resolved = false;
        let sources = BTreeMap::from([(
            "src/anchor.rs".to_string(),
            "pub fn run() {\n    println!(\"ok\");\n}\n".to_string(),
        )]);
        let report = validate_patch_attempt_with_semantic_source(&ctx, &attempt, &sources);
        assert!(report.allowed, "{:?}", report.issues);
    }

    #[test]
    fn semantic_source_validation_requests_refresh_when_anchor_missing() {
        let mut ctx = context();
        ctx.scopes.insert(
            "src/anchor.rs".to_string(),
            ApprovedPatchScope {
                path: "src/anchor.rs".to_string(),
                operation: PatchOperation::Modify,
                line_range: None,
                semantic_anchor: Some("fn:missing".to_string()),
                ownership: "vac.test.fixture".to_string(),
                allow_new_file: false,
            },
        );
        let mut attempt = attempt();
        attempt.path = "src/anchor.rs".to_string();
        attempt.line_range = None;
        attempt.semantic_anchor_resolved = false;
        let sources = BTreeMap::from([(
            "src/anchor.rs".to_string(),
            "pub fn run() {\n    println!(\"ok\");\n}\n".to_string(),
        )]);
        let report = validate_patch_attempt_with_semantic_source(&ctx, &attempt, &sources);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.anchor.unresolved")
        );
    }

    #[test]
    fn semantic_source_validation_rejects_range_outside_resolved_anchor() {
        let mut ctx = context();
        ctx.scopes.insert(
            "src/anchor.rs".to_string(),
            ApprovedPatchScope {
                path: "src/anchor.rs".to_string(),
                operation: PatchOperation::Modify,
                line_range: None,
                semantic_anchor: Some("fn:run".to_string()),
                ownership: "vac.test.fixture".to_string(),
                allow_new_file: false,
            },
        );
        let mut attempt = attempt();
        attempt.path = "src/anchor.rs".to_string();
        attempt.line_range = Some(PatchLineRange { start: 10, end: 11 });
        attempt.semantic_anchor_resolved = false;
        let sources = BTreeMap::from([(
            "src/anchor.rs".to_string(),
            "pub fn run() {\n    println!(\"ok\");\n}\n".to_string(),
        )]);
        let report = validate_patch_attempt_with_semantic_source(&ctx, &attempt, &sources);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "patch.range.out_of_bounds")
        );
    }

    #[test]
    fn syn_anchor_rejects_comment_false_positive() {
        let source = "// fn run() {}\nfn other() {}\n";
        let err = resolve_semantic_anchor_in_source("fn:run", source).unwrap_err();
        assert_eq!(
            err,
            SemanticAnchorResolutionError::NotFound("fn:run".to_string())
        );
    }

    #[test]
    fn syn_anchor_resolves_impl_method() {
        let source = "struct Runner;\nimpl Runner {\n    fn run(&self) { }\n}\n";
        let resolved = resolve_semantic_anchor_in_source("impl:run", source).unwrap();
        assert_eq!(resolved.kind, "impl");
        assert_eq!(resolved.name, "run");
    }

    #[test]
    fn syn_anchor_resolves_module() {
        let source = "pub mod runtime {\n    pub fn run() {}\n}\n";
        let resolved = resolve_semantic_anchor_in_source("mod:runtime", source).unwrap();
        assert_eq!(resolved.kind, "mod");
        assert_eq!(resolved.name, "runtime");
    }
}
