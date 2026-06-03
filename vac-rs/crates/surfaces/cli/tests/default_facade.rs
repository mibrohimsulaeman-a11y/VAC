#[test]
fn default_surface_cli_facade_is_linkable() {
    let _ = env!("CARGO_PKG_NAME");
    assert_eq!(env!("CARGO_PKG_NAME"), "vac-surface-cli");
}
