// Stub realtime prompt module.
//
// Realtime prompt preparation is not available in this build.

/// Returns the given prompt or an empty string if realtime backend prompt is disabled.
#[expect(
    dead_code,
    reason = "retained for realtime compatibility while the transport bridge is unavailable"
)]
pub(crate) fn prepare_realtime_backend_prompt(
    prompt: Option<Option<String>>,
    _experimental_backend_prompt: Option<String>,
) -> String {
    match prompt {
        Some(Some(p)) => p,
        Some(None) => String::new(),
        None => String::new(),
    }
}
