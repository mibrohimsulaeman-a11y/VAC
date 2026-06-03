// Local Runtime Contract adapter for the root TUI.
//
// Step 00D wiring split into focused submodules (P1.1):
// - `submit`: mint `RuntimeCommand::StartTask`, `RuntimeSubmitPlan`,
//   `default_autonomy_mode`, `LegacyCompatTransport` marker.
// - `approval`: `ApprovalCorrelation`, registry, decision command + 4x
//   `approval_resolved_for_*_decision` helpers.
// - `bridge`: `RuntimeBridge` -- protocol -> runtime event projection.
// - `activity`: typed activity items for the sidebar (00D-8 foundation).
// - `labels`: history label primitives + text/preview/redact helpers.

mod activity;
mod approval;
mod bridge;
mod labels;
mod submit;

#[cfg(test)]
mod tests;

pub(crate) use activity::*;
pub(crate) use approval::*;
pub(crate) use bridge::*;
pub(crate) use labels::*;
pub(crate) use submit::*;

pub(crate) use vac_core::local_runtime::RuntimeEvent;
