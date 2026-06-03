# Plan 34H Evidence — TUI Confirmation and Strict Promotion Closeout

Date: 2026-05-27
Environment: ChatGPT sandbox

## Result

Plan 34 rich setup UX is implemented at the Rust/CLI/TUI contract layer.

## Evidence

- `ProjectWorkspaceConfirmationDialog` renders the in-memory, soft bootstrap, and strict promotion choices.
- `ProjectWorkspaceStrictPromotionPreview` exposes review-only strict manifest directories before writing.
- `materialize_strict_workspace_promotion(root, approved)` is approval-gated and writes only after explicit approval.
- `vac doctor project-workspace` exposes setup dialog, soft bootstrap, strict preview, and strict promotion paths.
- TUI startup warnings surface the rich zero-config setup dialog when arbitrary user projects are missing `.vac`.

## Safety

- Missing `.vac` is non-fatal for arbitrary user projects.
- Strict product-repo gates remain fail-closed.
- Denied setup writes nothing.
