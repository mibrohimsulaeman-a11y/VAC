//! Source-level production callsite anchors for VAC-Init runtime gates.
//!
//! These helpers keep the four exported runtime-gate entrypoints visible from
//! production source outside the control-plane module itself. The executable
//! paths still use the richer bridge helpers (`evaluate_vac_init_runtime_*`),
//! while doctor/static gates can prove every boundary is intentionally bound.

use crate::control_plane::VacInitEvidenceCompletionGateContext;
use crate::control_plane::VacInitPreCommandGateContext;
use crate::control_plane::VacInitPrePatchGateContext;
use crate::control_plane::VacInitPrePlanGateContext;
use crate::control_plane::VacInitRuntimeGateReport;

pub fn evaluate_bound_pre_plan_runtime_gate(
    ctx: &VacInitPrePlanGateContext,
) -> VacInitRuntimeGateReport {
    crate::control_plane::evaluate_vac_init_pre_plan_gate(ctx)
}

pub fn evaluate_bound_pre_patch_runtime_gate(
    ctx: &VacInitPrePatchGateContext,
) -> VacInitRuntimeGateReport {
    crate::control_plane::evaluate_vac_init_pre_patch_gate(ctx)
}

pub fn evaluate_bound_pre_command_runtime_gate(
    ctx: &VacInitPreCommandGateContext,
) -> VacInitRuntimeGateReport {
    crate::control_plane::evaluate_vac_init_pre_command_gate(ctx)
}

pub fn evaluate_bound_evidence_completion_runtime_gate(
    ctx: &VacInitEvidenceCompletionGateContext,
) -> VacInitRuntimeGateReport {
    crate::control_plane::evaluate_vac_init_evidence_completion_gate(ctx)
}
