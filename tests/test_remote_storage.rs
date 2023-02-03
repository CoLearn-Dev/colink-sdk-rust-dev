mod common;
use colink::SubscriptionMessage;
use common::*;

#[tokio::test]
async fn test_remote_storage() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _iss, mut cls) = set_up_test_env(2).await?;
    let (cl_a, cl_b) = (cls.pop().unwrap(), cls.pop().unwrap());
    let msg = "hello";
    let user_id_a = cl_a.get_user_id().unwrap();
    let user_id_b = cl_b.get_user_id().unwrap();
    // create
    cl_a.remote_storage_create(
        &[user_id_b.clone()],
        "remote_storage_demo",
        msg.as_bytes(),
        false,
    )
    .await?;
    let data = cl_b
        .read_or_wait(&format!(
            "_remote_storage:private:{}:remote_storage_demo",
            user_id_a
        ))
        .await?;
    assert!(String::from_utf8_lossy(&data).eq(msg));
    // read
    let data = cl_a
        .remote_storage_read(&user_id_b, "remote_storage_demo", false, "")
        .await?;
    assert!(String::from_utf8_lossy(&data).eq(msg));
    // update
    let queue_name = cl_b
        .subscribe(
            &format!("_remote_storage:private:{}:remote_storage_demo", user_id_a),
            None,
        )
        .await?;
    cl_a.remote_storage_update(
        &[user_id_b.clone()],
        "remote_storage_demo",
        format!("update {}", msg).as_bytes(),
        false,
    )
    .await?;
    let mut subscriber = cl_b.new_subscriber(&queue_name).await?;
    let data = subscriber.get_next().await?;
    let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
    if message.change_type != "delete" {
        assert!(String::from_utf8_lossy(&*message.payload).eq(&format!("update {}", msg)));
    } else {
        Err("Receive delete change_type.")?
    }
    // delete
    cl_a.remote_storage_delete(&[user_id_b.clone()], "remote_storage_demo", false)
        .await?;

    let data = subscriber.get_next().await?;
    cl_b.unsubscribe(&queue_name).await?;
    let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
    if message.change_type != "delete" {
        Err("Receive non-delete change_type.")?
    }

    Ok(())
}
