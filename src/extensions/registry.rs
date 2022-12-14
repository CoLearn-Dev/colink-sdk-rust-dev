use crate::colink_proto::*;
pub use colink_registry_proto::{Registries, Registry, UserRecord};
use prost::Message;
mod colink_registry_proto {
    include!(concat!(env!("OUT_DIR"), "/colink_registry.rs"));
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn update_registries(&self, registries: &Registries) -> Result<(), Error> {
        let participants = vec![Participant {
            user_id: self.get_user_id()?,
            role: "update_registries".to_string(),
        }];
        let mut payload = vec![];
        registries.encode(&mut payload).unwrap();
        let task_id = self
            .run_task("registry", &payload, &participants, false)
            .await?;
        self.wait_task(&task_id).await?;
        Ok(())
    }
}
