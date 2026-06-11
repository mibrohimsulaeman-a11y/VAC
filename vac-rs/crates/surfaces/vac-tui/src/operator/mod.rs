//! VAC operator console snapshot and mockup-aligned screen renderers.
//!
//! This module is intentionally separated from the legacy chat view so the TUI has a single
//! anti-hardcode boundary: renderers consume `OperatorSnapshot` only. The snapshot loader reads
//! AppState-adjacent runtime data, environment overrides, and `.vac/registry` JSON outputs.

pub mod chrome;
pub mod components;
pub mod mode;
pub mod screens;
pub mod snapshot;
pub mod theme;

pub use mode::OperatorMode;
pub use snapshot::{OperatorSnapshot, RuntimeJobSnapshot, ToolTimelineItem};
