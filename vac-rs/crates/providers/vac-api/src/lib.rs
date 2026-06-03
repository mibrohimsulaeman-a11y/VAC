//! Legacy provider HTTP compatibility crate.
//!
//! The transport implementation has moved to `vac-provider-http`. This crate is
//! intentionally kept as a thin compatibility surface while downstream call
//! sites migrate from `vac_api::*` to `vac_provider_http::*`.

pub use vac_provider_http::*;
