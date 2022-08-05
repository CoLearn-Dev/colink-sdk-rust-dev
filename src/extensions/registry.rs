use crate::colink_proto::*;
pub use colink_registry_proto::{Registries, Registry};
use prost::Message;
mod colink_registry_proto {
    include!(concat!(env!("OUT_DIR"), "/colink_registry.rs"));
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn set_registries(&self, registries: &Registries) -> Result<(), Error> {
        let participants = vec![Participant {
            user_id: self.get_user_id()?,
            role: "set_registries".to_string(),
        }];
        let mut payload = vec![];
        registries.encode(&mut payload).unwrap();
        self.run_task("registry", &payload, &participants, false)
            .await?;
        Ok(())
    }
}
