use anyhow::Result;

#[derive(Clone)]
pub struct MitmState;

pub(crate) struct MitmUpstreamConfig {
    pub(crate) allow_upstream_proxy: bool,
    pub(crate) allow_local_binding: bool,
}

impl std::fmt::Debug for MitmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MitmState")
            .field("runtime_proxy_feature", &false)
            .finish()
    }
}

impl MitmState {
    pub(crate) fn new(config: MitmUpstreamConfig) -> Result<Self> {
        let _ = (config.allow_upstream_proxy, config.allow_local_binding);
        anyhow::bail!(
            "network MITM runtime is disabled; build vac-network-proxy with feature `runtime-proxy` to enable TLS interception"
        )
    }
}
