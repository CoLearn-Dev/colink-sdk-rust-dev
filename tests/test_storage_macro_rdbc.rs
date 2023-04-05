mod common;

use common::*;

#[tokio::test]
async fn test_storage_macro_rdbc_mysql(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    let key_name = "storage_macro_test:db:$rdbc:q0:Alice:alice<at>berkeley.edu";
    cl.create_entry("storage_macro_test:db:url", b"mysql://" as &[u8])
        .await?;
    cl.create_entry("storage_macro_test:db:username", b"root" as &[u8])
        .await?;
    cl.create_entry(
        "storage_macro_test:db:q0",
        b"SELECT * FROM users WHERE name = ? AND email = ?" as &[u8],
    )
    .await?;
    let query_result = cl.read_entry(key_name).await?;
    assert_eq!(query_result, b"storage_macro" as &[u8]);

    Ok(())
}
