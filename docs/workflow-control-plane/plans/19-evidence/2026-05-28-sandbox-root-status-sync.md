# Plan 19 root capability promotion and status sync — 2026-05-28

Status: complete for root manifest/status synchronization.

Promotions synchronized in this slice:

- `vac.local_runtime_owner` is ready for the default product path.
- `vac.tui_session_runtime` is ready for the default product path.
- `vac.runtime_approval_bridge` is ready for the default product path.
- `vac.release` is ready for release-gate policy semantics, with real operator evidence still represented as a gate input.
- runtime-owner and no-app-server workflows are ready gate manifests.

Deferred explicitly:

- Physical deletion of all `vac-app-server*` workspace crates remains a non-default historical compatibility cleanup task.
- Full heavy cargo workspace build remains outside sandbox reliability; targeted validation is the accepted slice gate here.
