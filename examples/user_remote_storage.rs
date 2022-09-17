use colink::{decode_jwt_without_validation, CoLink, SubscriptionMessage};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt_a = &args[1];
    let jwt_b = &args[2];
    let msg = if args.len() > 3 { &args[3] } else { "hello" };
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
    println!("{}", String::from_utf8_lossy(&data));

    // read
    let data = cl
        .remote_storage_read(&user_id_b, "remote_storage_demo", false, "")
        .await?;
    println!("{}", String::from_utf8_lossy(&data));

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
        println!("{}", String::from_utf8_lossy(&*message.payload));
    } else {
        Err("Receive delete change_type.")?
    }

    // delete
    cl.remote_storage_delete(&[user_id_b.clone()], "remote_storage_demo", false)
        .await?;

    let data = subscriber.get_next().await?;
    clb.unsubscribe(&queue_name).await?;
    let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
    if message.change_type == "delete" {
        println!("Deleted");
    } else {
        Err("Receive non-delete change_type.")?
    }

    Ok(())
}
