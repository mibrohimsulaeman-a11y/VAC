// Fixture source for deterministic Rust AST indexing.
// This file intentionally lives outside Cargo test targets; `#[test]` tokens
// below are parser inputs, not executable unit tests.

#[test]
fn unit_works() {
    assert_eq!(2 + 2, 4);
}

#[tokio::test]
async fn async_works() {
    helper_macro_like!("ignored or partial");
    async_helper().await;
}

async fn async_helper() {}
