use crate::colink_proto::*;
use colink_remote_storage_proto::*;
use prost::Message;

mod colink_remote_storage_proto {
    #![allow(clippy::derive_partial_eq_without_eq)]
    include!(concat!(env!("OUT_DIR"), "/colink_remote_storage.rs"));
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn remote_storage_create(
        &self,
        providers: &[String],
        key: &str,
        payload: &[u8],
        is_public: bool,
    ) -> Result<(), Error> {
        let mut participants = vec![Participant {
            user_id: self.get_user_id()?,
            role: "requester".to_string(),
        }];
        for provider in providers {
            participants.push(Participant {
                user_id: provider.to_string(),
                role: "provider".to_string(),
            });
        }
        let params = CreateParams {
            remote_key_name: key.to_string(),
            payload: payload.to_vec(),
            is_public,
        };
        let mut payload = vec![];
        params.encode(&mut payload).unwrap();
        self.run_task("remote_storage.create", &payload, &participants, false)
            .await?;
        Ok(())
    }

    pub async fn remote_storage_read(
        &self,
        provider: &str,
        key: &str,
        is_public: bool,
        holder_id: &str,
    ) -> Result<Vec<u8>, Error> {
        let participants = vec![
            Participant {
                user_id: self.get_user_id()?,
                role: "requester".to_string(),
            },
            Participant {
                user_id: provider.to_string(),
                role: "provider".to_string(),
            },
        ];
        let params = ReadParams {
            remote_key_name: key.to_string(),
            is_public,
            holder_id: holder_id.to_string(),
        };
        let mut payload = vec![];
        params.encode(&mut payload).unwrap();
        let task_id = self
            .run_task("remote_storage.read", &payload, &participants, false)
            .await?;
        let status = self
            .read_or_wait(&format!("tasks:{}:status", task_id))
            .await?;
        if status[0] == 0 {
            let data = self
                .read_or_wait(&format!("tasks:{}:output", task_id))
                .await?;
            Ok(data)
        } else {
            Err(format!("remote_storage.read: status_code: {}", status[0]))?
        }
    }

    pub async fn remote_storage_update(
        &self,
        providers: &[String],
        key: &str,
        payload: &[u8],
        is_public: bool,
    ) -> Result<(), Error> {
        let mut participants = vec![Participant {
            user_id: self.get_user_id()?,
            role: "requester".to_string(),
        }];
        for provider in providers {
            participants.push(Participant {
                user_id: provider.to_string(),
                role: "provider".to_string(),
            });
        }
        let params = UpdateParams {
            remote_key_name: key.to_string(),
            payload: payload.to_vec(),
            is_public,
        };
        let mut payload = vec![];
        params.encode(&mut payload).unwrap();
        self.run_task("remote_storage.update", &payload, &participants, false)
            .await?;
        Ok(())
    }

    pub async fn remote_storage_delete(
        &self,
        providers: &[String],
        key: &str,
        is_public: bool,
    ) -> Result<(), Error> {
        let mut participants = vec![Participant {
            user_id: self.get_user_id()?,
            role: "requester".to_string(),
        }];
        for provider in providers {
            participants.push(Participant {
                user_id: provider.to_string(),
                role: "provider".to_string(),
            });
        }
        let params = DeleteParams {
            remote_key_name: key.to_string(),
            is_public,
        };
        let mut payload = vec![];
        params.encode(&mut payload).unwrap();
        self.run_task("remote_storage.delete", &payload, &participants, false)
            .await?;
        Ok(())
    }
}
