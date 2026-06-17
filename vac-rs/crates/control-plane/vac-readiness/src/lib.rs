//! VAC readiness triplet computation.
//!
//! Runtime must trust only `effective`, and `effective` must fail-closed to the
//! weakest of declared and computed status.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessLevel {
    Deprecated,
    Planned,
    Partial,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadinessTriplet {
    pub declared: ReadinessLevel,
    pub computed: ReadinessLevel,
    pub effective: ReadinessLevel,
    pub reason: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ReadinessSignals {
    pub registry_valid: bool,
    pub policy_valid: bool,
    pub ownership_valid: bool,
    pub index_current: bool,
    pub assessment_span_grounded: bool,
    pub spec_sync_clean: bool,
    pub evidence_valid: bool,
    pub tv_cargo_gates_passed: bool,
}

impl ReadinessTriplet {
    #[must_use]
    pub fn fail_closed(mut self) -> Self {
        let weakest = weakest_readiness(self.declared, self.computed);
        if self.effective > weakest {
            self.effective = weakest;
        }
        self
    }
}

#[must_use]
pub fn compute_readiness(declared: ReadinessLevel, signals: &ReadinessSignals) -> ReadinessTriplet {
    let mut blockers = Vec::new();
    for (ok, label) in [
        (signals.registry_valid, "registry"),
        (signals.policy_valid, "policy"),
        (signals.ownership_valid, "ownership"),
        (signals.index_current, "index"),
        (signals.assessment_span_grounded, "assessment"),
        (signals.spec_sync_clean, "spec_sync"),
        (signals.evidence_valid, "evidence"),
        (signals.tv_cargo_gates_passed, "tv_cargo"),
    ] {
        if !ok {
            blockers.push(label.to_string());
        }
    }
    let computed = if blockers.is_empty() {
        ReadinessLevel::Ready
    } else if blockers
        .iter()
        .any(|item| matches!(item.as_str(), "registry" | "policy" | "ownership"))
    {
        ReadinessLevel::Planned
    } else {
        ReadinessLevel::Partial
    };
    let effective = weakest_readiness(declared, computed);
    ReadinessTriplet {
        declared,
        computed,
        effective,
        reason: if blockers.is_empty() {
            "all computed signals pass".to_string()
        } else {
            "lowered by computed blockers".to_string()
        },
        blockers,
    }
}

#[must_use]
pub fn weakest_readiness(a: ReadinessLevel, b: ReadinessLevel) -> ReadinessLevel {
    if a <= b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_signals_pass() -> ReadinessSignals {
        ReadinessSignals {
            registry_valid: true,
            policy_valid: true,
            ownership_valid: true,
            index_current: true,
            assessment_span_grounded: true,
            spec_sync_clean: true,
            evidence_valid: true,
            tv_cargo_gates_passed: true,
        }
    }

    #[test]
    fn effective_readiness_is_lowered_to_weakest_declared_and_computed() {
        let triplet = ReadinessTriplet {
            declared: ReadinessLevel::Partial,
            computed: ReadinessLevel::Planned,
            effective: ReadinessLevel::Ready,
            reason: "fixture".to_string(),
            blockers: vec![],
        };

        assert_eq!(triplet.fail_closed().effective, ReadinessLevel::Planned);
    }

    #[test]
    fn compute_readiness_reports_blockers_and_clamps_effective() {
        let mut signals = all_signals_pass();
        signals.policy_valid = false;

        let triplet = compute_readiness(ReadinessLevel::Ready, &signals);

        assert_eq!(triplet.computed, ReadinessLevel::Planned);
        assert_eq!(triplet.effective, ReadinessLevel::Planned);
        assert_eq!(triplet.blockers, vec!["policy".to_string()]);
    }
}
