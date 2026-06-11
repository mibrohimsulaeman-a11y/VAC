//! VAC provider implementation
//!
//! VAC provides an OpenAI-compatible API at `/v1/chat/completions`.
//! This provider routes inference requests through VAC's infrastructure.

mod convert;
mod provider;
mod stream;
mod types;

pub use provider::VacProvider;
pub use types::VacProviderConfig;
