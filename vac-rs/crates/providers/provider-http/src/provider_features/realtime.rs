//! Realtime transport feature seam.
//!
//! Realtime websocket helpers are default-off and may only be reached through
//! explicit `provider-realtime` builds.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimeProviderConfig {
    pub websocket_url: String,
}
