use crate::network_policy::NetworkPolicyDecider;
use crate::runtime::NetworkProxyState;
use anyhow::Result;
use std::net::SocketAddr;
use std::net::TcpListener as StdTcpListener;
use std::sync::Arc;

pub async fn run_socks5(
    _state: Arc<NetworkProxyState>,
    _addr: SocketAddr,
    _policy_decider: Option<Arc<dyn NetworkPolicyDecider>>,
    _enable_udp: bool,
) -> Result<()> {
    anyhow::bail!(
        "network proxy listener runtime is disabled; build vac-network-proxy with feature `runtime-proxy` to run SOCKS5 proxy listeners"
    )
}

pub async fn run_socks5_with_std_listener(
    _state: Arc<NetworkProxyState>,
    _listener: StdTcpListener,
    _policy_decider: Option<Arc<dyn NetworkPolicyDecider>>,
    _enable_udp: bool,
) -> Result<()> {
    anyhow::bail!(
        "network proxy listener runtime is disabled; build vac-network-proxy with feature `runtime-proxy` to run SOCKS5 proxy listeners"
    )
}
