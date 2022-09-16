use colink::{CoLink, SubscriptionMessage, Task};
use std::env;
use tracing::debug;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let now = if args.len() > 2 {
        Some(args[2].parse()?)
    } else {
        None
    };

    let cl = CoLink::new(addr, jwt);

    let latest_key = format!("_internal:protocols:greetings:finished:latest");
    let queue_name = cl.subscribe(&latest_key, now).await?;
    let mut subscriber = cl.new_subscriber(&queue_name).await?;
    let data = subscriber.get_next().await?;
    debug!("Received [{}]", String::from_utf8_lossy(&data));
    let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
    if message.change_type != "delete" {
        let task_id: Task = prost::Message::decode(&*message.payload).unwrap();
        let res = cl
            .read_entry(&format!("tasks:{}:output", task_id.task_id))
            .await?;
        println!("{}", String::from_utf8_lossy(&res));
    }
    Ok(())
}
