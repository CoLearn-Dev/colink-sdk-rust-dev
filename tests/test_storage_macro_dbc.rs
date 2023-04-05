use common::*;

mod common;

#[tokio::test]
async fn test_storage_macro_dbc_mysql(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry("storage_macro_test:db:url", b"mysql://" as &[u8])
        .await?;
    cl.create_entry("storage_macro_test:db:username", b"root" as &[u8])
        .await?;
    cl.create_entry(
        "storage_macro_test:db:q0",
        b"SELECT * FROM users WHERE name = ? AND email = ?" as &[u8],
    )
    .await?;

    let key_name = "storage_macro_test:db:$rdbc:q0:Alice:alice<at>berkeley.edu";
    let query_result = cl.read_entry(key_name).await?;
    assert_eq!(query_result, b"storage_macro" as &[u8]);

    Ok(())
}
