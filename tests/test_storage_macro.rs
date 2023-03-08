mod common;
use colink::CoLink;
use common::*;
use rand::Rng;

#[tokio::test]
async fn test_storage_macro_chunk() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    let key_name = "storage_macro_test_chunk:$chunk";
    test_crud(&cl, key_name).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_redis() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry("storage_macro_test_redis:redis_url", b"redis://127.0.0.1")
        .await?;
    let key_name = "storage_macro_test_redis:$redis:redis_key";
    test_crud(&cl, key_name).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_fs() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry("test_storage_macro_fs:path", b"/tmp/colink-sm-fs-test/test")
        .await?;
    let key_name = "test_storage_macro_fs:$fs";
    test_crud(&cl, key_name).await?;

    cl.create_entry(
        "test_storage_macro_fs_dir:path",
        b"/tmp/colink-sm-fs-test/test-dir",
    )
    .await?;
    let key_name = "test_storage_macro_fs_dir:$fs:test-file";
    test_crud(&cl, key_name).await?;
    let key_name = "test_storage_macro_fs_dir:$fs:test-dir:test-file";
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
    println!("Create");
    assert!(cl.create_entry(key_name, b"").await.is_err());
    let data = cl.read_entry(key_name).await?;
    println!("Read");
    assert_eq!(data, payload);
    let new_payload = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(3e6 as usize)
        .collect::<Vec<u8>>();
    cl.update_entry(key_name, &new_payload).await?;
    println!("Update");
    let data = cl.read_entry(key_name).await?;
    assert_eq!(data, new_payload);
    cl.delete_entry(key_name).await?;
    println!("Delete");
    assert!(cl.read_entry(key_name).await.is_err());
    assert!(cl.delete_entry(key_name).await.is_err());
    Ok(())
}

#[tokio::test]
async fn test_storage_macro_append(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    let key_name = "storage_macro_test_append";
    test_append(&cl, key_name, 10 as usize).await?;

    Ok(())
}

async fn test_append(
    cl: &CoLink,
    key_name: &str,
    size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let payload0 = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(size)
        .collect::<Vec<u8>>();
    let payload1 = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(size)
        .collect::<Vec<u8>>();
    cl.create_entry(key_name, &payload0).await?;
    cl.update_entry(&format!("{}:$append", key_name), &payload1)
        .await?;
    let data = cl.read_entry(key_name).await?;
    assert_eq!(data, [payload0, payload1].concat());
    cl.delete_entry(key_name).await?;
    Ok(())
}

#[tokio::test]
async fn test_storage_macro_redis_keys(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry("storage_macro_test_redis:redis_url", b"redis://127.0.0.1")
        .await?;
    let key_name = "storage_macro_test_redis:$redis:redis_key";
    test_read_keys(&cl, key_name).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_fs_keys(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry(
        "test_storage_macro_fs_dir:path",
        b"/tmp/colink-sm-fs-test/test-dir",
    )
    .await?;
    let key_name = "test_storage_macro_fs_dir:$fs";
    test_read_keys(&cl, key_name).await?;

    Ok(())
}

async fn test_read_keys(
    cl: &CoLink,
    key_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let payload0 = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(10 as usize)
        .collect::<Vec<u8>>();
    let payload1 = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(5 as usize)
        .collect::<Vec<u8>>();
    cl.create_entry(&format!("{}:0", key_name), &payload0)
        .await?;
    cl.create_entry(&format!("{}:1", key_name), &payload1)
        .await?;
    let keys = cl
        .read_keys(&format!("{}::{}", cl.get_user_id()?, key_name), false)
        .await?;
    assert_eq!(keys.len(), 2);
    let data = cl.read_entry(&keys[0].key_path).await?;
    match data.len() {
        10 => assert_eq!(data, payload0),
        5 => assert_eq!(data, payload1),
        _ => assert!(false),
    }
    let data = cl.read_entry(&keys[1].key_path).await?;
    match data.len() {
        10 => assert_eq!(data, payload0),
        5 => assert_eq!(data, payload1),
        _ => assert!(false),
    }
    cl.delete_entry(&format!("{}:0", key_name)).await?;
    cl.delete_entry(&format!("{}:1", key_name)).await?;
    Ok(())
}

#[tokio::test]
async fn test_storage_macro_redis_append(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry(
        "test_storage_macro_redis_append:redis_url",
        b"redis://127.0.0.1",
    )
    .await?;
    let key_name = "test_storage_macro_redis_append:$redis:redis_key";
    test_append(&cl, key_name, 5e6 as usize).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_chunk_append(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    let key_name = "test_storage_macro_chunk_append:$chunk";
    test_append(&cl, key_name, 5e6 as usize).await?;
    test_append(&cl, key_name, 10 as usize).await?;
    test_append(&cl, key_name, 1024 * 1024 as usize).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_fs_append(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry(
        "test_storage_macro_fs_append:path",
        b"/tmp/colink-sm-fs-test/append-test",
    )
    .await?;
    let key_name = "test_storage_macro_fs_append:$fs";
    test_append(&cl, key_name, 5e6 as usize).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_redis_chunk(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry(
        "test_storage_macro_redis_chunk:redis_url",
        b"redis://127.0.0.1",
    )
    .await?;
    let key_name = "test_storage_macro_redis_chunk:$redis:redis_chunk:$chunk";
    test_crud(&cl, key_name).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_redis_chunk_append(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry(
        "test_storage_macro_redis_chunk_append:redis_url",
        b"redis://127.0.0.1",
    )
    .await?;
    let key_name = "test_storage_macro_redis_chunk_append:$redis:redis_chunk:$chunk";
    test_append(&cl, key_name, 5e6 as usize).await?;
    test_append(&cl, key_name, 10 as usize).await?;
    test_append(&cl, key_name, 1024 * 1024 as usize).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_fs_chunk(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry(
        "test_storage_macro_fs_chunk:path",
        b"/tmp/colink-sm-fs-test/chunk-test",
    )
    .await?;
    let key_name = "test_storage_macro_fs_chunk:$fs:$chunk";
    test_crud(&cl, key_name).await?;

    Ok(())
}

#[tokio::test]
async fn test_storage_macro_fs_chunk_append(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry(
        "test_storage_macro_fs_chunk_append:path",
        b"/tmp/colink-sm-fs-test/chunk-append-test",
    )
    .await?;
    let key_name = "test_storage_macro_fs_chunk_append:$fs:$chunk";
    test_append(&cl, key_name, 5e6 as usize).await?;
    test_append(&cl, key_name, 10 as usize).await?;
    test_append(&cl, key_name, 1024 * 1024 as usize).await?;

    Ok(())
}
