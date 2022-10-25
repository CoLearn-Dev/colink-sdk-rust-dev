use crate::{colink_proto::*, utils::get_path_timestamp};
use prost::Message;
use tracing::debug;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn wait_task(&self, task_id: &str) -> Result<(), Error> {
        let task_key = format!("_internal:tasks:{}", task_id);
        let start_timestamp = match self
            .read_entries(&[StorageEntry {
                key_name: task_key.clone(),
                ..Default::default()
            }])
            .await
        {
            Ok(res) => {
                let task: Task = Message::decode(&*res[0].payload).unwrap();
                if task.status == "finished" {
                    return Ok(());
                }
                get_path_timestamp(&res[0].key_path) + 1
            }
            Err(_) => 0,
        };
        let queue_name = self.subscribe(&task_key, Some(start_timestamp)).await?;
        let mut subscriber = self.new_subscriber(&queue_name).await?;
        loop {
            let data = subscriber.get_next().await?;
            debug!("Received [{}]", String::from_utf8_lossy(&data));
            let message: SubscriptionMessage = Message::decode(&*data).unwrap();
            if message.change_type != "delete" {
                let task: Task = Message::decode(&*message.payload).unwrap();
                if task.status == "finished" {
                    break;
                }
            }
        }
        self.unsubscribe(&queue_name).await?;
        Ok(())
    }
}
