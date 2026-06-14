//! Deterministic VAC code index contracts.
//!
//! Product indexing must be AST-path/fingerprint grounded. Current SV artifacts
//! may use static heuristic records, but those records are explicitly labelled
//! low-confidence fail-closed and must not be advertised as AST-grounded until
//! tree-sitter-rust or ra_ap_syntax is the authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(71);
    out.push_str("sha256:");
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[must_use]
pub fn raw_span_sha256(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

#[must_use]
pub fn normalize_text_for_fingerprint(text: &str) -> String {
    let normalized_eol = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized_eol
        .split('\n')
        .map(|line| line.trim_end_matches([' ', '\t']))
        .collect::<Vec<_>>();
    if matches!(lines.last(), Some(last) if last.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[must_use]
pub fn normalized_text_fingerprint(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    sha256_hex(normalize_text_for_fingerprint(&text).as_bytes())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileRecord {
    pub path: String,
    pub role: String,
    pub language: String,
    pub sha256: String,
    pub bytes: u64,
    pub parser_mode: ParserMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParserMode {
    RustAst,
    StaticHeuristic,
    NotParsedFailClosed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpanRecord {
    pub span_id: String,
    pub path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub ast_path: String,
    pub symbol: String,
    pub kind: String,
    pub normalized_fingerprint: String,
    pub span_sha256: String,
    pub confidence: ScannerConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationRecord {
    pub relation_id: String,
    pub source_span: String,
    pub target_symbol: String,
    pub relation_kind: RelationKind,
    pub confidence: ScannerConfidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Imports,
    Calls,
    Implements,
    Reads,
    Writes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadPlanTicket {
    pub ticket_id: String,
    pub span_id: String,
    pub reason: String,
    pub max_lines: u32,
    pub required_for_assessment: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScannerConfidence {
    Certain,
    High,
    Moderate,
    Low,
    Uncertain,
}

impl ScannerConfidence {
    #[must_use]
    pub fn fail_closed_decision(self) -> &'static str {
        match self {
            Self::Certain | Self::High => "allow_if_policy_allows",
            Self::Moderate | Self::Low | Self::Uncertain => "approval_required",
        }
    }
}

#[must_use]
pub fn rust_symbol_span(
    path: &str,
    line: u32,
    kind: &str,
    symbol: &str,
    normalized_fingerprint: &str,
) -> SpanRecord {
    SpanRecord {
        span_id: format!("span:{}:{}:{}", path, line, symbol),
        path: path.to_string(),
        line_start: line,
        line_end: line,
        ast_path: format!("crate::{symbol}"),
        symbol: symbol.to_string(),
        kind: kind.to_string(),
        normalized_fingerprint: normalized_fingerprint.to_string(),
        span_sha256: normalized_fingerprint.to_string(),
        confidence: ScannerConfidence::High,
    }
}

pub fn resolve_semantic_anchor<'a>(
    spans: &'a [SpanRecord],
    ast_path: &str,
    fingerprint: &str,
) -> Result<&'a SpanRecord, AnchorResolutionError> {
    let matches: Vec<&SpanRecord> = spans
        .iter()
        .filter(|span| span.ast_path == ast_path && span.normalized_fingerprint == fingerprint)
        .collect();
    match matches.as_slice() {
        [] => Err(AnchorResolutionError::NoMatch),
        [span] => Ok(*span),
        _ => Err(AnchorResolutionError::MultipleMatches),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnchorResolutionError {
    NoMatch,
    MultipleMatches,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexCoverageReport {
    pub rust_ast_mode: String,
    pub rust_files: u64,
    pub rust_symbol_spans: u64,
    pub fallback_files: u64,
    pub fail_closed_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionSpanCandidate {
    pub path: String,
    pub symbol: String,
    pub kind: String,
    pub start_line: u32,
    pub end_line: u32,
    pub ast_path: String,
    pub normalized_fingerprint: String,
}

pub fn coverage_gate(
    report: &IndexCoverageReport,
    min_rust_symbol_spans: u64,
) -> Result<(), String> {
    if report.rust_ast_mode.is_empty() {
        return Err("index coverage report missing rust_ast_mode".to_string());
    }
    if report.rust_symbol_spans < min_rust_symbol_spans {
        return Err(format!(
            "rust symbol span coverage below threshold: {} < {}",
            report.rust_symbol_spans, min_rust_symbol_spans
        ));
    }
    if report.fallback_files > 0 && !report.fail_closed_fallback {
        return Err("polyglot/static fallback must be fail-closed".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParserModeTruth {
    pub manifest_label: String,
    pub product_claim_allowed: bool,
    pub effective_confidence: ScannerConfidence,
    pub warning: Option<String>,
}

/// Normalize index parser claims so product surfaces cannot call a regex/SV
/// artifact AST-grounded.  Full product authority requires a real parser mode;
/// static heuristic mode remains useful but fail-closed.
#[must_use]
pub fn parser_mode_truth(manifest_label: &str) -> ParserModeTruth {
    let lower = manifest_label.to_ascii_lowercase();
    if lower.contains("tree_sitter") || lower.contains("ra_ap_syntax") || lower == "rust_ast" {
        ParserModeTruth {
            manifest_label: manifest_label.to_string(),
            product_claim_allowed: true,
            effective_confidence: ScannerConfidence::High,
            warning: None,
        }
    } else {
        ParserModeTruth {
            manifest_label: manifest_label.to_string(),
            product_claim_allowed: false,
            effective_confidence: ScannerConfidence::Low,
            warning: Some("index is static heuristic / SV scaffold; do not advertise AST-grounded product scoring".to_string()),
        }
    }
}

#[must_use]
pub fn read_plan_ticket_count(value: &serde_json::Value) -> u64 {
    value
        .pointer("/counts/read_plans")
        .or_else(|| value.pointer("/summary/read_plans"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_only_change_refreshes_raw_hash_but_keeps_normalized_fingerprint() {
        let lf = b"fn demo() {\n    println!(\"ok\");\n}\n";
        let crlf = b"fn demo() {\r\n    println!(\"ok\");\r\n}\r\n";

        assert_ne!(raw_span_sha256(lf), raw_span_sha256(crlf));
        assert_eq!(
            normalized_text_fingerprint(lf),
            normalized_text_fingerprint(crlf)
        );
    }

    #[test]
    fn trailing_whitespace_and_final_newline_do_not_change_text_fingerprint() {
        let clean = b"let value = 1;\nlet next = 2";
        let formatted = b"let value = 1;   \r\nlet next = 2\t\r\n";

        assert_ne!(raw_span_sha256(clean), raw_span_sha256(formatted));
        assert_eq!(
            normalized_text_fingerprint(clean),
            normalized_text_fingerprint(formatted)
        );
    }
}
