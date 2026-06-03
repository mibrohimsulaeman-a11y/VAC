// Status output formatting and display adapters for the TUI.
//
// This module turns protocol-level snapshots into stable display structures used by `/status`
// output and footer/status-line helpers, while keeping rendering concerns out of transport-facing
// code.
//
// `rate_limits` is the main integration point for ambient status-line/nudge usage-limit items.
// The hardened `/status` card itself is local-only: it does not render limit/credit rows and does
// not initiate account-limit refreshes.
mod account;
mod card;
mod format;
mod helpers;
mod output_contract;
mod rate_limits;

pub(crate) use account::StatusAccountDisplay;
pub(crate) use card::StatusHistoryHandle;
#[cfg(test)]
pub(crate) use card::new_status_output;
#[cfg(test)]
pub(crate) use card::new_status_output_with_rate_limits;
pub(crate) use card::new_status_output_with_rate_limits_handle;
pub(crate) use helpers::compose_agents_summary;
pub(crate) use helpers::format_directory_display;
pub(crate) use helpers::format_tokens_compact;
pub(crate) use helpers::plan_type_display_name;
pub(crate) use output_contract::status_command_requests_rate_limit_refresh;
pub(crate) use rate_limits::RateLimitSnapshotDisplay;
pub(crate) use rate_limits::RateLimitWindowDisplay;
#[cfg(test)]
pub(crate) use rate_limits::rate_limit_snapshot_display;
pub(crate) use rate_limits::rate_limit_snapshot_display_for_limit;

#[cfg(test)]
mod tests;
