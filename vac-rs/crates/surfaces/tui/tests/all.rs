// Single integration test binary that aggregates all test modules.
// The submodules live in `tests/suite/`.
#![cfg(feature = "full-tui")]
mod test_backend;

#[allow(unused_imports)]
use vac_cli as _; // Keep dev-dep for cargo-shear; tests spawn the vac binary.

mod suite;
