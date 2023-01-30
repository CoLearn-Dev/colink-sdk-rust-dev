use colink::{
    extensions::instant_server::{InstantRegistry, InstantServer},
    CoLink, SubscriptionMessage,
};

async fn user_remote_storage_fn(
    cla: &CoLink,
    clb: &CoLink,
    msg: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let user_id_a = cla.get_user_id().unwrap();
    let user_id_b = clb.get_user_id().unwrap();

    // create
    cla.remote_storage_create(
        &[user_id_b.clone()],
        "remote_storage_demo",
        msg.as_bytes(),
        false,
    )
    .await?;

    let data = clb
        .read_or_wait(&format!(
            "_remote_storage:private:{}:remote_storage_demo",
            user_id_a
        ))
        .await?;
    // println!("{}", String::from_utf8_lossy(&data));
    assert!(String::from_utf8_lossy(&data).eq(msg));

    // read
    let data = cla
        .remote_storage_read(&user_id_b, "remote_storage_demo", false, "")
        .await?;
    // println!("{}", String::from_utf8_lossy(&data));
    assert!(String::from_utf8_lossy(&data).eq(msg));

    // update
    let queue_name = clb
        .subscribe(
            &format!("_remote_storage:private:{}:remote_storage_demo", user_id_a),
            None,
        )
        .await?;

    cla.remote_storage_update(
        &[user_id_b.clone()],
        "remote_storage_demo",
        format!("update {}", msg).as_bytes(),
        false,
    )
    .await?;

    let mut subscriber = clb.new_subscriber(&queue_name).await?;
    let data = subscriber.get_next().await?;
    let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
    if message.change_type != "delete" {
        // println!("{}", String::from_utf8_lossy(&*message.payload));
        assert!(String::from_utf8_lossy(&*message.payload).eq(&format!("update {}", msg)));
    } else {
        Err("Receive delete change_type.")?
    }

    // delete
    cla.remote_storage_delete(&[user_id_b.clone()], "remote_storage_demo", false)
        .await?;

    let data = subscriber.get_next().await?;
    clb.unsubscribe(&queue_name).await?;
    let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
    if message.change_type != "delete" {
        Err("Receive non-delete change_type.")?
    }

    Ok(())
}

#[tokio::test]
async fn test_user_remote_storage() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let _ir = InstantRegistry::new();
    let isa = InstantServer::new();
    let isb = InstantServer::new();
    let cla = isa.get_colink().switch_to_generated_user().await?;
    let clb = isb.get_colink().switch_to_generated_user().await?;

    let test_msg = "hello";

    user_remote_storage_fn(&cla, &clb, test_msg).await?;

    Ok(())
}
