use crate::{colink_proto::*, utils::get_path_timestamp};
use prost::Message;
use tracing::debug;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn wait_user_init(&self) -> Result<(), Error> {
        let is_initialized_key = "_internal:_is_initialized";
        let start_timestamp = match self
            .read_entries(&[StorageEntry {
                key_name: is_initialized_key.to_string(),
                ..Default::default()
            }])
            .await
        {
            Ok(res) => {
                if res[0].payload[0] == 1 {
                    return Ok(());
                }
                get_path_timestamp(&res[0].key_path) + 1
            }
            Err(_) => 0,
        };
        let queue_name = self
            .subscribe(is_initialized_key, Some(start_timestamp))
            .await?;
        let mut subscriber = self.new_subscriber(&queue_name).await?;
        loop {
            let data = subscriber.get_next().await?;
            debug!("Received [{}]", String::from_utf8_lossy(&data));
            let message: SubscriptionMessage = Message::decode(&*data).unwrap();
            if message.change_type != "delete" && message.payload[0] == 1 {
                break;
            }
        }
        self.unsubscribe(&queue_name).await?;
        Ok(())
    }
}
