# Plan 34 — Rich confirmation and strict promotion evidence

Status: complete for the core Plan 34 confirmation/promotion contract.

## What changed

- `vac_core::project_workspace::ProjectWorkspaceConfirmationDialog` models the first-run rich setup dialog without side effects.
- Dialog choices include `continue_in_memory`, `approve_soft_bootstrap`, `review_strict_promotion`, and `cancel`.
- `ProjectWorkspacePromptAction` maps those choices to runtime-safe actions.
- `ProjectWorkspaceStrictPromotionPreview` renders a review-only strict promotion plan.
- `vac doctor project-workspace <path> --setup-dialog-preview` renders the rich dialog model.
- `vac doctor project-workspace <path> --promote-strict-preview` renders strict promotion without writes.
- `vac doctor project-workspace <path> --promote-strict --yes` materializes strict manifest directories only after explicit approval.
- CLI/TUI startup warnings now include the rich dialog render text when `.vac` is missing in an arbitrary user project.

## Validation

`rustc --edition=2024 --test vac-rs/core/src/project_workspace.rs` passes 17 focused tests covering missing `.vac`, denied/approved soft bootstrap, rich dialog choices, strict promotion preview, denied strict promotion, and approved strict promotion boundaries.

## Completion note

This evidence closes the core Plan 34 contract: ordinary missing-`.vac` projects remain usable, durable `.vac` writes require approval, rich setup choices are modelled in Rust, strict promotion has review-only preview, and approved promotion materializes manifest boundaries explicitly.
