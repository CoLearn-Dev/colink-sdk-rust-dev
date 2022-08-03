use colink_sdk::{CoLink, CoLinkInternalTaskIdList, StorageEntry, SubscriptionMessage, Task};
use prost::Message;
use std::env;
use tracing::debug;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let protocol_name = &args[2];

    let cl = CoLink::new(addr, jwt);

    /*
       In CoLink storage, we have `_internal:protocols:{}:waiting` which is a list maintaining all the tasks in the waiting status,
           and another entry `_internal:protocols:{}:waiting:latest` which is updated every time a task changes to the waiting status.
       Here the goal is to confirm all current tasks that are in the waiting status.
       To make sure we cover them all, we subscribe to the history of `_internal:protocols:{}:waiting:latest`.
       To avoid going through the whole history, we use `_internal:protocols:{}:waiting` to find the earliest timestamp
           among all the tasks that are currently still in the status of waiting and specify that in our subscription.
       The subscription results in a superset of what we need, but we can filter out those that have been processed locally.
    */

    let list_key = format!("_internal:protocols:{}:waiting", protocol_name);
    let latest_key = format!("_internal:protocols:{}:waiting:latest", protocol_name);
    // Step 1: get the list of key_path which contains the timestamp.
    let res = cl
        .read_entries(&[StorageEntry {
            key_name: list_key,
            ..Default::default()
        }])
        .await;
    // Step 2: find the earliest timestamp in the list.
    let start_timestamp = match res {
        Ok(res) => {
            let list_entry = &res[0];
            let list: CoLinkInternalTaskIdList = Message::decode(&*list_entry.payload).unwrap();
            if list.task_ids_with_key_paths.is_empty() {
                get_timestamp(&list_entry.key_path)
            } else {
                list.task_ids_with_key_paths
                    .iter()
                    .map(|x| get_timestamp(&x.key_path))
                    .min()
                    .unwrap_or(i64::MAX)
            }
        }
        Err(_) => 0i64,
    };
    // Step 3: subscribe and get a queue_name.
    let queue_name = cl.subscribe(&latest_key, Some(start_timestamp)).await?;
    // Step 4: set up a subscriber with the queue_name.
    let mut subscriber = cl.new_subscriber(&queue_name).await?;
    loop {
        // Step 5: process subscription message.
        let data = subscriber.get_next().await?;
        debug!("Received [{}]", String::from_utf8_lossy(&data));
        let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
        // Step 5.1: match the change_type.
        if message.change_type != "delete" {
            let task_id: Task = prost::Message::decode(&*message.payload).unwrap();
            let res = cl
                .read_entries(&[StorageEntry {
                    key_name: format!("_internal:tasks:{}", task_id.task_id),
                    ..Default::default()
                }])
                .await?;
            let task_entry = &res[0];
            let task: Task = prost::Message::decode(&*task_entry.payload).unwrap();
            // IMPORTANT: Step 5.2: you must check the status of the task received from the subscription.
            if task.status == "waiting" {
                cl.confirm_task(&task_id.task_id, true, false, "").await?;
            }
        }
    }
}

fn get_timestamp(key_path: &str) -> i64 {
    let pos = key_path.rfind('@').unwrap();
    key_path[pos + 1..].parse().unwrap()
}
