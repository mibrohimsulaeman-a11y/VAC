#![cfg(feature = "full-tui")]
#[path = "../src/test_backend.rs"]
mod inner;

pub use inner::VT100Backend;
