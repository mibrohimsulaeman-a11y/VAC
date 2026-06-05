use http::HeaderMap;
use vac_protocol::protocol::RealtimeOutputModality;
use vac_protocol::protocol::RealtimeVoice;
use vac_provider_http::ApiError;
use vac_provider_http::HttpTransport;
use vac_provider_http::Provider;
use vac_provider_http::RealtimeAudioFrame;
use vac_provider_http::RealtimeEvent;
use vac_provider_http::SharedAuthProvider;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RealtimeEventParser {
    V1,
    RealtimeV2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RealtimeSessionMode {
    Conversational,
    Transcription,
}

#[derive(Clone, Debug)]
pub struct RealtimeSessionConfig {
    pub instructions: String,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub event_parser: RealtimeEventParser,
    pub session_mode: RealtimeSessionMode,
    pub output_modality: RealtimeOutputModality,
    pub voice: RealtimeVoice,
}

#[derive(Clone, Debug)]
pub struct RealtimeWebsocketClient {
    _provider: Provider,
}

impl RealtimeWebsocketClient {
    pub fn new(provider: Provider) -> Self {
        Self {
            _provider: provider,
        }
    }

    pub async fn connect(
        &self,
        _session_config: RealtimeSessionConfig,
        _extra_headers: HeaderMap,
        _default_headers: HeaderMap,
    ) -> Result<RealtimeWebsocketConnection, ApiError> {
        Err(unsupported_realtime_transport())
    }

    pub async fn connect_webrtc_sideband(
        &self,
        _session_config: RealtimeSessionConfig,
        _call_id: &str,
        _sideband_headers: HeaderMap,
        _default_headers: HeaderMap,
    ) -> Result<RealtimeWebsocketConnection, ApiError> {
        Err(unsupported_realtime_transport())
    }
}

#[derive(Clone, Debug)]
pub struct RealtimeWebsocketConnection;

impl RealtimeWebsocketConnection {
    pub fn writer(&self) -> RealtimeWebsocketWriter {
        RealtimeWebsocketWriter
    }

    pub fn events(&self) -> RealtimeWebsocketEvents {
        RealtimeWebsocketEvents
    }
}

#[derive(Clone, Debug)]
pub struct RealtimeWebsocketWriter;

impl RealtimeWebsocketWriter {
    pub async fn send_response_create(&self) -> Result<(), ApiError> {
        Err(unsupported_realtime_transport())
    }

    pub async fn send_conversation_item_create(&self, _text: String) -> Result<(), ApiError> {
        Err(unsupported_realtime_transport())
    }

    pub async fn send_conversation_function_call_output(
        &self,
        _call_id: String,
        _output: String,
    ) -> Result<(), ApiError> {
        Err(unsupported_realtime_transport())
    }

    pub async fn send_audio_frame(&self, _frame: RealtimeAudioFrame) -> Result<(), ApiError> {
        Err(unsupported_realtime_transport())
    }

    pub async fn send_payload(&self, _payload: String) -> Result<(), ApiError> {
        Err(unsupported_realtime_transport())
    }
}

#[derive(Clone, Debug)]
pub struct RealtimeWebsocketEvents;

impl RealtimeWebsocketEvents {
    pub async fn next_event(&self) -> Result<Option<RealtimeEvent>, ApiError> {
        Err(unsupported_realtime_transport())
    }
}

#[derive(Clone)]
pub struct RealtimeCallClient<T> {
    _transport: T,
    _provider: Provider,
    _auth: SharedAuthProvider,
}

impl<T: Clone> std::fmt::Debug for RealtimeCallClient<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealtimeCallClient").finish_non_exhaustive()
    }
}

impl<T> RealtimeCallClient<T>
where
    T: HttpTransport,
{
    pub fn new(transport: T, provider: Provider, auth: SharedAuthProvider) -> Self {
        Self {
            _transport: transport,
            _provider: provider,
            _auth: auth,
        }
    }

    pub async fn create_with_session_and_headers(
        &self,
        _sdp: String,
        _session_config: RealtimeSessionConfig,
        _extra_headers: HeaderMap,
    ) -> Result<RealtimeCallCreateResponse, ApiError> {
        Err(unsupported_realtime_transport())
    }
}

#[derive(Clone, Debug)]
pub struct RealtimeCallCreateResponse {
    pub sdp: String,
    pub call_id: String,
}

fn unsupported_realtime_transport() -> ApiError {
    ApiError::InvalidRequest {
        message: "legacy realtime transport is not available in the vac-api compatibility surface"
            .to_string(),
    }
}
