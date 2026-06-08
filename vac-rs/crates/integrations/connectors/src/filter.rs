//! Local-tool compatibility filters.
//!
//! The cloud VAC connector apps directory and originator-specific blocklists were removed. These helpers
//! now preserve the old call-site shape while only de-duplicating already-accessible local MCP
//! connectors.

use std::collections::HashSet;
use vac_runtime_protocol::AppInfo;

pub fn filter_tool_suggest_discoverable_connectors(
    directory_connectors: Vec<AppInfo>,
    accessible_connectors: &[AppInfo],
    discoverable_connector_ids: &HashSet<String>,
    _originator_value: &str,
) -> Vec<AppInfo> {
    let accessible_connector_ids: HashSet<&str> = accessible_connectors
        .iter()
        .filter(|connector| connector.is_accessible)
        .map(|connector| connector.id.as_str())
        .collect();

    let mut connectors = directory_connectors
        .into_iter()
        .filter(|connector| !accessible_connector_ids.contains(connector.id.as_str()))
        .filter(|connector| discoverable_connector_ids.contains(connector.id.as_str()))
        .collect::<Vec<_>>();
    connectors.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });
    connectors
}

pub fn filter_disallowed_connectors(
    connectors: Vec<AppInfo>,
    _originator_value: &str,
) -> Vec<AppInfo> {
    connectors
}
