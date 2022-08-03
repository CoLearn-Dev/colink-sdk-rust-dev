use crate::colink_proto::*;
use tracing::debug;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::basic_a::CoLink {
    #[cfg(feature = "extension")]
    pub async fn read_or_wait(&self, key: &str) -> Result<Vec<u8>, Error> {
        match self.read_entry(key).await {
            Ok(res) => Ok(res),
            Err(e) => {
                let queue_name = self.subscribe(key, None).await?;
                let mut subscriber = self.new_subscriber(&queue_name).await?;
                let data = subscriber.get_next().await?;
                debug!("Received [{}]", String::from_utf8_lossy(&data));
                self.unsubscribe(&queue_name).await?;
                let message: SubscriptionMessage = prost::Message::decode(&*data).unwrap();
                if message.change_type != "delete" {
                    Ok((*message.payload).to_vec())
                } else {
                    Err(e)
                }
            }
        }
    }
}
