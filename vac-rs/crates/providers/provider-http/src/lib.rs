//! Provider HTTP transport crate.
//!
//! This crate is the physical source-of-record for provider-generic HTTP, SSE,
//! websocket, responses, model, memory, and local file upload
//! transport. The legacy `vac-api` crate is now a compatibility re-export layer
//! so existing call sites can migrate incrementally without keeping transport
//! implementation in a cloud-account-named crate.

pub(crate) mod api_bridge;
pub(crate) mod auth;
mod client_sse;
pub mod client_transport;
pub(crate) mod common;
mod custom_ca;
mod default_client;
pub(crate) mod endpoint;
pub(crate) mod error;
pub(crate) mod files;
pub(crate) mod provider;
pub mod provider_features;
pub(crate) mod rate_limits;
mod request;
mod request_telemetry;
pub(crate) mod requests;
mod retry;
pub(crate) mod sse;
pub(crate) mod telemetry;
mod transport;
mod transport_error;

pub use crate::client_sse::sse_stream;
pub use crate::client_transport::with_chatgpt_cloudflare_cookie_store;
pub use crate::custom_ca::BuildCustomCaTransportError;
#[doc(hidden)]
pub use crate::custom_ca::build_reqwest_client_for_subprocess_tests;
pub use crate::custom_ca::build_reqwest_client_with_custom_ca;
pub use crate::custom_ca::maybe_build_rustls_client_config_with_custom_ca;
pub use crate::default_client::VACHttpClient;
pub use crate::default_client::VACRequestBuilder;
pub use crate::request::PreparedRequestBody;
pub use crate::request::Request;
pub use crate::request::RequestBody;
pub use crate::request::RequestCompression;
pub use crate::request::Response;
pub use crate::request_telemetry::RequestTelemetry;
pub use crate::requests::headers::build_conversation_headers;
pub use crate::retry::RetryOn;
pub use crate::retry::RetryPolicy;
pub use crate::retry::backoff;
pub use crate::retry::run_with_retry;
pub use crate::transport::ByteStream;
pub use crate::transport::HttpTransport;
pub use crate::transport::ReqwestTransport;
pub use crate::transport::StreamResponse;
pub use crate::transport_error::StreamError;
pub use crate::transport_error::TransportError;

pub use crate::api_bridge::map_api_error;
pub use crate::auth::AuthError;
pub use crate::auth::AuthHeaderTelemetry;
pub use crate::auth::AuthProvider;
pub use crate::auth::SharedAuthProvider;
pub use crate::auth::auth_header_telemetry;
pub use crate::common::CompactionInput;
pub use crate::common::MemorySummarizeInput;
pub use crate::common::MemorySummarizeOutput;
pub use crate::common::OpenAiVerbosity;
pub use crate::common::RawMemory;
pub use crate::common::RawMemoryMetadata;
pub use crate::common::Reasoning;
pub use crate::common::ResponseCreateWsRequest;
pub use crate::common::ResponseEvent;
pub use crate::common::ResponseStream;
pub use crate::common::ResponsesApiRequest;
pub use crate::common::ResponsesWsRequest;
pub use crate::common::TextControls;
pub use crate::common::WS_REQUEST_HEADER_TRACEPARENT_CLIENT_METADATA_KEY;
pub use crate::common::WS_REQUEST_HEADER_TRACESTATE_CLIENT_METADATA_KEY;
pub use crate::common::create_text_param_for_request;
pub use crate::common::response_create_client_metadata;
pub use crate::endpoint::CompactClient;
pub use crate::endpoint::MemoriesClient;
pub use crate::endpoint::ModelsClient;
pub use crate::endpoint::ResponsesClient;
pub use crate::endpoint::ResponsesOptions;
pub use crate::endpoint::ResponsesWebsocketClient;
pub use crate::endpoint::ResponsesWebsocketConnection;
pub use crate::error::ApiError;
pub use crate::files::upload_local_file;
pub use crate::provider::Provider;
pub use crate::provider::RetryConfig;
pub use crate::provider::is_azure_responses_provider;
pub use crate::provider_features::DEFAULT_PROVIDER_FEATURE_STATUS;
pub use crate::provider_features::ProviderFeatureStatus;
pub use crate::requests::Compression;
pub use crate::sse::stream_from_fixture;
pub use crate::telemetry::SseTelemetry;
pub use crate::telemetry::WebsocketTelemetry;
pub use vac_protocol::protocol::RealtimeAudioFrame;
pub use vac_protocol::protocol::RealtimeEvent;
