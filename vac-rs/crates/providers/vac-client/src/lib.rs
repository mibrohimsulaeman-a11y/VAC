//! Legacy compatibility crate for provider HTTP transport.
//!
//! Generic HTTP/SSE/websocket transport now lives in `vac-provider-http`. This
//! crate intentionally contains no implementation modules and re-exports the
//! provider-neutral source of truth for older downstream imports. Hosted-account
//! compatibility remains default-off through the `provider-chatgpt` feature.

pub use vac_provider_http::*;
