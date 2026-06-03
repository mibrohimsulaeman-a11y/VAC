#![allow(dead_code)]
//! Safe rationale and `vac why` trajectory contracts for VAC-Init.
//!
//! This module intentionally stores auditable decision summaries, evidence
//! references, policy references, and memory references. It never stores or
//! exposes raw/private chain-of-thought.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WhyTargetKind {
    File,
    Line,
    Range,
    Symbol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhyQuery {
    pub file: String,
    pub line: Option<usize>,
    pub range: Option<(usize, usize)>,
    pub symbol: Option<String>,
    pub depth: usize,
}

impl WhyQuery {
    pub fn target_kind(&self) -> WhyTargetKind {
        if self.symbol.is_some() {
            WhyTargetKind::Symbol
        } else if self.range.is_some() {
            WhyTargetKind::Range
        } else if self.line.is_some() {
            WhyTargetKind::Line
        } else {
            WhyTargetKind::File
        }
    }

    pub fn validate(&self) -> Result<(), SafeRationaleError> {
        validate_relative_file_path(&self.file)?;
        if let Some((start, end)) = self.range {
            if start == 0 || end < start {
                return Err(SafeRationaleError::InvalidQuery(
                    "range must be one-based and end >= start".to_string(),
                ));
            }
        }
        if let Some(line) = self.line {
            if line == 0 {
                return Err(SafeRationaleError::InvalidQuery(
                    "line must be one-based".to_string(),
                ));
            }
        }
        if self.depth > MAX_WHY_DEPTH {
            return Err(SafeRationaleError::InvalidQuery(format!(
                "depth exceeds maximum {}",
                MAX_WHY_DEPTH
            )));
        }
        Ok(())
    }
}

pub const MAX_WHY_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeMemoryRef {
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeRationaleSummary {
    pub summary: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub team_memory_refs: Vec<SafeMemoryRef>,
    pub semantic_refs: Vec<SafeMemoryRef>,
    pub raw_chain_of_thought_excluded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeChangesetExcerpt {
    pub operation: String,
    pub diff_excerpt: String,
    pub lines_added: usize,
    pub lines_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeApprovalRef {
    pub approved_by: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhyResult {
    pub evidence_id: String,
    pub timestamp: String,
    pub task: String,
    pub plan_id: String,
    pub capability: String,
    pub rationale: SafeRationaleSummary,
    pub changeset: SafeChangesetExcerpt,
    pub approval: Option<SafeApprovalRef>,
    pub chain_depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectorySpan {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub symbol: Option<String>,
    pub evidence_id: String,
    pub plan_id: String,
    pub capability: String,
}

impl TrajectorySpan {
    pub fn matches_query(&self, query: &WhyQuery) -> bool {
        if self.file != query.file {
            return false;
        }
        if let Some(symbol) = &query.symbol {
            return self.symbol.as_ref() == Some(symbol);
        }
        if let Some((start, end)) = query.range {
            return self.start_line <= end && self.end_line >= start;
        }
        if let Some(line) = query.line {
            return self.start_line <= line && self.end_line >= line;
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectoryIndex {
    pub spans: Vec<TrajectorySpan>,
    pub results_by_evidence: BTreeMap<String, WhyResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhyLookupReport {
    pub query: WhyQuery,
    pub results: Vec<WhyResult>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafeRationaleError {
    InvalidQuery(String),
    UnsafeRationale(String),
    InvalidReference(String),
}

pub fn validate_relative_file_path(path: &str) -> Result<(), SafeRationaleError> {
    if path.is_empty() {
        return Err(SafeRationaleError::InvalidQuery(
            "file path is empty".to_string(),
        ));
    }
    if path.starts_with('/') || path.contains("..") || path.contains('\\') {
        return Err(SafeRationaleError::InvalidQuery(format!(
            "file path must be workspace-relative and normalized: {}",
            path
        )));
    }
    Ok(())
}

pub fn validate_safe_id(prefix: &str, id: &str) -> Result<(), SafeRationaleError> {
    if !id.starts_with(prefix) {
        return Err(SafeRationaleError::InvalidReference(format!(
            "{} must start with {}",
            id, prefix
        )));
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err(SafeRationaleError::InvalidReference(format!(
            "id contains invalid characters: {}",
            id
        )));
    }
    Ok(())
}

pub fn contains_raw_chain_of_thought(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "chain of thought",
        "private reasoning",
        "hidden reasoning",
        "scratchpad",
        "internal monologue",
        "raw thoughts",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub fn validate_safe_rationale(result: &WhyResult) -> Result<(), SafeRationaleError> {
    validate_safe_id("evidence.", &result.evidence_id)?;
    validate_safe_id("plan.", &result.plan_id)?;
    validate_safe_id("vac.", &result.capability)?;
    if result.rationale.summary.trim().is_empty() {
        return Err(SafeRationaleError::UnsafeRationale(
            "safe rationale summary is empty".to_string(),
        ));
    }
    if !result.rationale.raw_chain_of_thought_excluded {
        return Err(SafeRationaleError::UnsafeRationale(
            "raw_chain_of_thought_excluded must be true".to_string(),
        ));
    }
    if contains_raw_chain_of_thought(&result.rationale.summary)
        || contains_raw_chain_of_thought(&result.changeset.diff_excerpt)
    {
        return Err(SafeRationaleError::UnsafeRationale(
            "raw/private chain-of-thought text is not allowed in safe rationale output".to_string(),
        ));
    }
    if result.chain_depth > MAX_WHY_DEPTH {
        return Err(SafeRationaleError::UnsafeRationale(format!(
            "chain depth exceeds maximum {}",
            MAX_WHY_DEPTH
        )));
    }
    for policy_ref in &result.rationale.policy_refs {
        validate_safe_id("policy.", policy_ref)?;
    }
    for evidence_ref in &result.rationale.evidence_refs {
        validate_safe_id("evidence.", evidence_ref)?;
    }
    Ok(())
}

pub fn lookup_safe_rationale(index: &TrajectoryIndex, query: WhyQuery) -> WhyLookupReport {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut results = Vec::new();

    if let Err(err) = query.validate() {
        diagnostics.push(format!("invalid query: {:?}", err));
        return WhyLookupReport {
            query,
            results,
            diagnostics,
        };
    }

    for span in &index.spans {
        if !span.matches_query(&query) {
            continue;
        }
        if !seen.insert(span.evidence_id.clone()) {
            continue;
        }
        match index.results_by_evidence.get(&span.evidence_id) {
            Some(result) => match validate_safe_rationale(result) {
                Ok(()) => results.push(result.clone()),
                Err(err) => diagnostics.push(format!(
                    "unsafe rationale skipped for {}: {:?}",
                    span.evidence_id, err
                )),
            },
            None => diagnostics.push(format!(
                "trajectory span references missing evidence: {}",
                span.evidence_id
            )),
        }
    }

    if results.is_empty() && diagnostics.is_empty() {
        diagnostics.push("no safe rationale result found for query".to_string());
    }

    WhyLookupReport {
        query,
        results,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result() -> WhyResult {
        WhyResult {
            evidence_id: "evidence.2026-05-29-ledger".to_string(),
            timestamp: "2026-05-29T00:00:00Z".to_string(),
            task: "Add ledger guard".to_string(),
            plan_id: "plan.add-ledger-guard".to_string(),
            capability: "vac.financial.ledger".to_string(),
            rationale: SafeRationaleSummary {
                summary: "Added bounded ledger guard according to approved plan and policy."
                    .to_string(),
                policy_refs: vec!["policy.ledger.append-only".to_string()],
                evidence_refs: vec!["evidence.2026-05-29-ledger".to_string()],
                team_memory_refs: vec![SafeMemoryRef {
                    id: "mem.team.ledger-rule".to_string(),
                    summary: "Ledger mutation must be append-only.".to_string(),
                }],
                semantic_refs: Vec::new(),
                raw_chain_of_thought_excluded: true,
            },
            changeset: SafeChangesetExcerpt {
                operation: "modify".to_string(),
                diff_excerpt: "Added append-only guard check.".to_string(),
                lines_added: 5,
                lines_removed: 0,
            },
            approval: Some(SafeApprovalRef {
                approved_by: "operator".to_string(),
                content_hash: "a".repeat(64),
            }),
            chain_depth: 1,
        }
    }

    #[test]
    fn validates_safe_result() {
        assert!(validate_safe_rationale(&sample_result()).is_ok());
    }

    #[test]
    fn rejects_raw_chain_of_thought_text() {
        let mut result = sample_result();
        result.rationale.summary = "raw chain of thought: ...".to_string();
        assert!(validate_safe_rationale(&result).is_err());
    }

    #[test]
    fn rejects_invalid_query_paths() {
        let query = WhyQuery {
            file: "../secret.rs".to_string(),
            line: Some(1),
            range: None,
            symbol: None,
            depth: 1,
        };
        assert!(query.validate().is_err());
    }

    #[test]
    fn lookup_matches_line_and_deduplicates_evidence() {
        let result = sample_result();
        let mut by_evidence = BTreeMap::new();
        by_evidence.insert(result.evidence_id.clone(), result);
        let index = TrajectoryIndex {
            spans: vec![
                TrajectorySpan {
                    file: "vac-rs/core/src/ledger.rs".to_string(),
                    start_line: 10,
                    end_line: 20,
                    symbol: Some("append".to_string()),
                    evidence_id: "evidence.2026-05-29-ledger".to_string(),
                    plan_id: "plan.add-ledger-guard".to_string(),
                    capability: "vac.financial.ledger".to_string(),
                },
                TrajectorySpan {
                    file: "vac-rs/core/src/ledger.rs".to_string(),
                    start_line: 15,
                    end_line: 30,
                    symbol: None,
                    evidence_id: "evidence.2026-05-29-ledger".to_string(),
                    plan_id: "plan.add-ledger-guard".to_string(),
                    capability: "vac.financial.ledger".to_string(),
                },
            ],
            results_by_evidence: by_evidence,
        };
        let report = lookup_safe_rationale(
            &index,
            WhyQuery {
                file: "vac-rs/core/src/ledger.rs".to_string(),
                line: Some(16),
                range: None,
                symbol: None,
                depth: 1,
            },
        );
        assert_eq!(report.results.len(), 1);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn empty_lookup_returns_diagnostic_not_panic() {
        let index = TrajectoryIndex {
            spans: Vec::new(),
            results_by_evidence: BTreeMap::new(),
        };
        let report = lookup_safe_rationale(
            &index,
            WhyQuery {
                file: "src/lib.rs".to_string(),
                line: Some(1),
                range: None,
                symbol: None,
                depth: 1,
            },
        );
        assert!(report.results.is_empty());
        assert!(!report.diagnostics.is_empty());
    }
}
