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
