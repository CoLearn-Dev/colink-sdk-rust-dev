use colink::{
    extensions::instant_server::{InstantRegistry, InstantServer},
    CoLink,
};
use rand::Rng;

#[tokio::test]
async fn test_storage_macro_chunk() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let _ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink().switch_to_generated_user().await?;

    let key_name = "storage_macro_test_chunk:$chunk";
    test_crud(&cl, key_name).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_redis() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let _ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink().switch_to_generated_user().await?;

    cl.create_entry("storage_macro_test_redis:redis_url", b"redis://localhost")
        .await?;
    let key_name = "storage_macro_test_redis:$redis:redis_key";
    test_crud(&cl, key_name).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_append(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let _ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink().switch_to_generated_user().await?;

    let key_name = "storage_macro_test_append";
    let payload0 = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(10)
        .collect::<Vec<u8>>();
    let payload1 = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(10)
        .collect::<Vec<u8>>();
    cl.create_entry(key_name, &payload0).await?;
    cl.update_entry(&format!("{}:$append", key_name), &payload1)
        .await?;
    let data = cl.read_entry(key_name).await?;
    assert_eq!(data, [payload0, payload1].concat());

    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_storage_macro_chunk_redis(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let _ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink().switch_to_generated_user().await?;

    cl.create_entry(
        "test_storage_macro_chunk_redis:redis_url",
        b"redis://localhost",
    )
    .await?;
    let key_name = "test_storage_macro_chunk_redis:$redis:redis_chunk:$chunk";
    test_crud(&cl, key_name).await?;

    Ok(())
}

async fn test_crud(
    cl: &CoLink,
    key_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let payload = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(5e6 as usize)
        .collect::<Vec<u8>>();
    cl.create_entry(key_name, &payload).await?;
    assert!(cl.create_entry(key_name, b"").await.is_err());
    let data = cl.read_entry(key_name).await?;
    assert_eq!(data, payload);
    let new_payload = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(3e6 as usize)
        .collect::<Vec<u8>>();
    cl.update_entry(key_name, &new_payload).await?;
    let data = cl.read_entry(key_name).await?;
    assert_eq!(data, new_payload);
    cl.delete_entry(key_name).await?;
    assert!(cl.read_entry(key_name).await.is_err());
    assert!(cl.delete_entry(key_name).await.is_err());
    Ok(())
}
