#![forbid(unsafe_code)]

pub const PRODUCT_NAME: &str = "VAC";
pub const BINARY_NAME: &str = "vac";
pub const CONFIG_DIR: &str = ".vac";
pub const LEGACY_CONFIG_DIR: &str = ".vac";
pub const ENV_PREFIX: &str = "VAC";
pub const LEGACY_ENV_PREFIX: &str = "VAC";
pub const CONTROL_PLANE_VERSION: &str = "1.5";
pub const RULEBOOK_DEFAULT: &str = "vac.core";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrandSource {
    Primary,
    Legacy,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEnv {
    pub key: String,
    pub value: Option<String>,
    pub source: BrandSource,
}

pub fn preferred_env_key(name: &str) -> String {
    format!("{ENV_PREFIX}_{name}")
}

pub fn legacy_env_key(name: &str) -> String {
    format!("{LEGACY_ENV_PREFIX}_{name}")
}

pub fn resolve_env(name: &str) -> ResolvedEnv {
    let key = preferred_env_key(name);
    if let Ok(value) = std::env::var(&key) {
        return ResolvedEnv {
            key,
            value: Some(value),
            source: BrandSource::Primary,
        };
    }
    let legacy_key = legacy_env_key(name);
    if let Ok(value) = std::env::var(&legacy_key) {
        return ResolvedEnv {
            key: legacy_key,
            value: Some(value),
            source: BrandSource::Legacy,
        };
    }
    ResolvedEnv {
        key,
        value: None,
        source: BrandSource::Missing,
    }
}

pub fn product_header(version: &str) -> String {
    format!("{PRODUCT_NAME} {version}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_constants_are_vac_first() {
        assert_eq!(PRODUCT_NAME, "VAC");
        assert_eq!(BINARY_NAME, "vac");
        assert_eq!(CONFIG_DIR, ".vac");
        assert_eq!(ENV_PREFIX, "VAC");
    }

    #[test]
    fn header_is_vac_native() {
        let header = product_header("0.0.0");
        assert_eq!(header, "VAC 0.0.0");
    }
}
