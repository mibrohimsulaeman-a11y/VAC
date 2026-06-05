//! Local MCP connector helpers.
//!
//! The ChatGPT Apps cloud directory surface was removed for the local TUI+CLI coding build.
//! This crate now keeps only helper logic used to derive `@mention` connector labels from MCP
//! tool metadata.

use std::time::Duration;

pub mod accessible;
pub mod filter;
pub mod merge;
pub mod metadata;

pub const CONNECTORS_CACHE_TTL: Duration = Duration::from_secs(3600);

pub fn normalize_connector_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn connector_name_slug(name: &str) -> String {
    let mut slug = String::new();
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

pub fn connector_install_url(name: &str, connector_id: &str) -> String {
    let slug = connector_name_slug(name);
    format!("vac://mcp-connectors/{slug}?id={connector_id}")
}
