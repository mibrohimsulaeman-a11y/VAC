//! VAC control-plane crate.
//!
//! This crate is the physical source-of-record for the `.vac` control plane:
//! registries, workflows, policy manifests, ownership scans, architecture
//! invariants, approval lifecycle, and local runtime contract types.
//!
//! `vac-core` re-exports these modules for compatibility, but the files live
//! here so ownership, review, and future Cargo layering match the control-plane
//! architecture rather than the historical core god-crate layout.

pub mod control_plane;
pub mod local_runtime;

pub use control_plane::*;
