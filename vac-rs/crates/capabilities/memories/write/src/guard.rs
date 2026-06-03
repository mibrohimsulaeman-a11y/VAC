use tracing::info;
use vac_core::config::Config;
use vac_login::AuthManager;
use vac_protocol::protocol::RateLimitSnapshot;
use vac_protocol::protocol::RateLimitWindow;

pub(crate) async fn rate_limits_ok(auth_manager: &AuthManager, config: &Config) -> bool {
    rate_limits_check(auth_manager, config)
        .await
        .unwrap_or(true)
}

async fn rate_limits_check(auth_manager: &AuthManager, config: &Config) -> Option<bool> {
    let auth = auth_manager.auth().await?;
    if !auth.uses_vac_backend() {
        return None;
    }

    // The old backend-client/OpenAPI rate-limit fetch path was cloud-coupled.
    // Local coding-agent memory startup must not call that backend just to decide
    // whether local memory writes may begin. Until a provider-neutral local quota
    // snapshot exists, fail open and keep the threshold observable in logs.
    let min_remaining_percent = config.memories.min_rate_limit_remaining_percent;
    info!(
        min_remaining_percent,
        "allowing memories startup without backend-client rate-limit fetch in local coding-agent build"
    );
    Some(true)
}

fn snapshot_allows_startup(snapshot: &RateLimitSnapshot, min_remaining_percent: i64) -> bool {
    if snapshot.rate_limit_reached_type.is_some() {
        return false;
    }

    let max_used_percent = 100.0 - min_remaining_percent.clamp(0, 100) as f64;
    window_allows_startup(snapshot.primary.as_ref(), max_used_percent)
        && window_allows_startup(snapshot.secondary.as_ref(), max_used_percent)
}

fn window_allows_startup(window: Option<&RateLimitWindow>, max_used_percent: f64) -> bool {
    match window {
        Some(window) => window.used_percent <= max_used_percent,
        None => true,
    }
}

#[cfg(test)]
#[path = "guard_tests.rs"]
mod tests;
