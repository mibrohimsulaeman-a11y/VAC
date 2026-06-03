#![allow(dead_code)]
//! Semantic anchor resolver facade for bounded patch execution.
//!
//! The production patch guard wires Rust anchors through
//! `vac_init_patch_guard::RustAstAnchorResolver`. This module is the control-plane
//! seam for future language-specific resolvers while keeping strict/degraded mode
//! explicit and machine-checkable.

pub use super::vac_init_patch_guard::LineHeuristicAnchorResolver;
pub use super::vac_init_patch_guard::ResolvedSemanticAnchor;
pub use super::vac_init_patch_guard::RustAstAnchorResolver;
pub use super::vac_init_patch_guard::SemanticAnchorMode;
pub use super::vac_init_patch_guard::SemanticAnchorResolutionError;
pub use super::vac_init_patch_guard::SemanticAnchorResolver;
pub use super::vac_init_patch_guard::resolve_semantic_anchor_in_source_strict;
pub use super::vac_init_patch_guard::resolve_semantic_anchor_with_mode;
