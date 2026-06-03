//! Provider-generic HTTP transport primitives.
//!
//! This module is the source-of-truth replacement for the legacy `vac-client`
//! transport crate. `vac-client` remains only as a compatibility re-export so
//! downstream code can migrate without retaining duplicate transport logic.

mod custom_ca;
mod default_client;
mod error;
mod request;
mod retry;
mod sse;
mod telemetry;
mod transport;

pub use self::custom_ca::BuildCustomCaTransportError;
#[doc(hidden)]
pub use self::custom_ca::build_reqwest_client_for_subprocess_tests;
pub use self::custom_ca::build_reqwest_client_with_custom_ca;
pub use self::custom_ca::maybe_build_rustls_client_config_with_custom_ca;
pub use self::default_client::VACHttpClient;
pub use self::default_client::VACRequestBuilder;
pub use self::error::StreamError;
pub use self::error::TransportError;
pub use self::request::PreparedRequestBody;
pub use self::request::Request;
pub use self::request::RequestBody;
pub use self::request::RequestCompression;
pub use self::request::Response;
pub use self::retry::RetryOn;
pub use self::retry::RetryPolicy;
pub use self::retry::backoff;
pub use self::retry::run_with_retry;
pub use self::sse::sse_stream;
pub use self::telemetry::RequestTelemetry;
pub use self::transport::ByteStream;
pub use self::transport::HttpTransport;
pub use self::transport::ReqwestTransport;
pub use self::transport::StreamResponse;

#[cfg(feature = "provider-chatgpt")]
pub fn with_chatgpt_cloudflare_cookie_store(
    builder: reqwest::ClientBuilder,
) -> reqwest::ClientBuilder {
    tracing::debug!(target: "vac_provider_http::provider_features", "legacy hosted-account cookie store requires external provider adapter; returning unmodified HTTP client");
    builder
}

#[cfg(not(feature = "provider-chatgpt"))]
pub fn with_chatgpt_cloudflare_cookie_store(
    builder: reqwest::ClientBuilder,
) -> reqwest::ClientBuilder {
    tracing::debug!(target: "vac_provider_http::provider_features", "legacy hosted-account cookie store disabled; returning unmodified HTTP client");
    builder
}

pub fn is_allowed_chatgpt_host(_host: &str) -> bool {
    false
}
