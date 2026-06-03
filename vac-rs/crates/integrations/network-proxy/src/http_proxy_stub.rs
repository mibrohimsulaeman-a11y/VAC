use crate::network_policy::NetworkPolicyDecider;
use crate::runtime::NetworkProxyState;
use anyhow::Result;
use std::net::SocketAddr;
use std::net::TcpListener as StdTcpListener;
use std::sync::Arc;

pub async fn run_http_proxy(
    _state: Arc<NetworkProxyState>,
    _addr: SocketAddr,
    _policy_decider: Option<Arc<dyn NetworkPolicyDecider>>,
) -> Result<()> {
    anyhow::bail!(
        "network proxy listener runtime is disabled; build vac-network-proxy with feature `runtime-proxy` to run HTTP proxy listeners"
    )
}

pub async fn run_http_proxy_with_std_listener(
    _state: Arc<NetworkProxyState>,
    _listener: StdTcpListener,
    _policy_decider: Option<Arc<dyn NetworkPolicyDecider>>,
) -> Result<()> {
    anyhow::bail!(
        "network proxy listener runtime is disabled; build vac-network-proxy with feature `runtime-proxy` to run HTTP proxy listeners"
    )
}
