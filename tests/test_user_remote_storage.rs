use colink::{
    decode_jwt_without_validation,
    extensions::instant_server::{InstantRegistry, InstantServer},
    generate_user, prepare_import_user_signature, CoLink,
    SubscriptionMessage
};

async fn user_remote_storage_fn(jwt_a:&str,jwt_b:&str,addr:&str,msg:&str) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let user_id_a = decode_jwt_without_validation(jwt_a).unwrap().user_id;
    let user_id_b = decode_jwt_without_validation(jwt_b).unwrap().user_id;
    
    let cl = CoLink::new(addr, jwt_a);
    // create
    cl.remote_storage_create(
        &[user_id_b.clone()],
        "remote_storage_demo",
        msg.as_bytes(),
        false,
    )
    .await?;

    let clb = CoLink::new(addr, jwt_b);
    let data = clb
        .read_or_wait(&format!(
            "_remote_storage:private:{}:remote_storage_demo",
            user_id_a
        ))
        .await?;
    // println!("{}", String::from_utf8_lossy(&data));
    assert!(String::from_utf8_lossy(&data).eq(msg));

    // read
    let data = cl
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

    cl.remote_storage_update(
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
    cl.remote_storage_delete(&[user_id_b.clone()], "remote_storage_demo", false)
        .await?;

    let data = subscriber.get_next().await?;
    clb.unsubscribe(&queue_name).await?;
    let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
    if message.change_type != "delete" {
        Err("Receive non-delete change_type.")?
    }

    Ok(())
}

async fn gen_new_uesr(cl:&CoLink) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let expiration_timestamp = chrono::Utc::now().timestamp() + 86400 * 31;
    let (pk, sk) = generate_user();
    let core_pub_key = cl.request_info().await?.core_public_key;
    let (signature_timestamp, sig) =
        prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);
    let user_jwt = cl
        .import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
        .await?;
    Ok(user_jwt)
}

#[tokio::test]
async fn test_user_remote_storage() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let _ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink();
    let core_addr = cl.get_core_addr()?;

    let user_a_jwt=gen_new_uesr(&cl).await?;
    let user_b_jwt=gen_new_uesr(&cl).await?;

    let test_msg="hello";

    user_remote_storage_fn(&user_a_jwt,&user_b_jwt,&core_addr,test_msg).await?;

    Ok(())
}
