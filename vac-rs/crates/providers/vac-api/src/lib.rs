//! Legacy provider HTTP compatibility crate.
//!
//! The transport implementation has moved to `vac-provider-http`. This crate is
//! intentionally kept as a thin compatibility surface while downstream call
//! sites migrate from `vac_api::*` to `vac_provider_http::*`.

pub mod realtime_compat;
pub use realtime_compat::RealtimeCallClient;
pub use realtime_compat::RealtimeCallCreateResponse;
pub use realtime_compat::RealtimeEventParser;
pub use realtime_compat::RealtimeSessionConfig;
pub use realtime_compat::RealtimeSessionMode;
pub use realtime_compat::RealtimeWebsocketClient;
pub use realtime_compat::RealtimeWebsocketConnection;
pub use realtime_compat::RealtimeWebsocketEvents;
pub use realtime_compat::RealtimeWebsocketWriter;

pub use vac_provider_http::*;
