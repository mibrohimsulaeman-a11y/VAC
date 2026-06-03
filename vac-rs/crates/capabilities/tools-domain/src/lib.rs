//! Static capability-domain contract for the VAC O5/O6 core decomposition.
//!
//! This crate is intentionally dependency-light: it gives the control plane a
//! compile-addressable ownership boundary for the `tools` domain before the
//! legacy `vac-core` modules are deleted in a later mechanical move. Runtime
//! callers should depend on this crate for domain metadata and on the existing
//! implementation modules only through the compatibility exports recorded in
//! `.vac/registry/core-decomposition-ledger.yaml`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityDomainContract {
    pub id: &'static str,
    pub domain: &'static str,
    pub package: &'static str,
    pub manifest: &'static str,
    pub status: &'static str,
}

pub const CAPABILITY_CONTRACT: CapabilityDomainContract = CapabilityDomainContract {
    id: "vac.tools",
    domain: "tools",
    package: "vac-capability-tools",
    manifest: ".vac/capabilities/tools.yaml",
    status: "static_boundary_ready",
};

pub const fn capability_contract() -> &'static CapabilityDomainContract {
    &CAPABILITY_CONTRACT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_points_to_domain_manifest() {
        let contract = capability_contract();
        assert_eq!(contract.domain, "tools");
        assert_eq!(contract.package, "vac-capability-tools");
        assert!(contract.manifest.ends_with(".yaml"));
    }
}
