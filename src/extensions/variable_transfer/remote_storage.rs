use crate::colink_proto::*;
use colink_remote_storage::*;
use prost::Message;
mod colink_remote_storage {
    include!(concat!(env!("OUT_DIR"), "/colink_remote_storage.rs"));
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[deprecated(note = "please use `send_variable_with_remote_storage` instead")]
    pub async fn set_variable_with_remote_storage(
        &self,
        key: &str,
        payload: &[u8],
        receivers: &[Participant],
    ) -> Result<(), Error> {
        self.send_variable_with_remote_storage(key, payload, receivers)
            .await
    }

    pub async fn send_variable_with_remote_storage(
        &self,
        key: &str,
        payload: &[u8],
        receivers: &[Participant],
    ) -> Result<(), Error> {
        let mut new_participants = vec![Participant {
            user_id: self.get_user_id()?,
            role: "requester".to_string(),
        }];
        for p in receivers {
            if p.user_id == self.get_user_id()? {
                self.create_entry(
                    &format!(
                        "_remote_storage:private:{}:_variable_transfer:{}:{}",
                        p.user_id,
                        self.get_task_id()?,
                        key
                    ),
                    payload,
                )
                .await?;
            } else {
                new_participants.push(Participant {
                    user_id: p.user_id.clone(),
                    role: "provider".to_string(),
                });
            }
        }
        let params = CreateParams {
            remote_key_name: format!("_variable_transfer:{}:{}", self.get_task_id()?, key),
            payload: payload.to_vec(),
            ..Default::default()
        };
        let mut payload = vec![];
        params.encode(&mut payload).unwrap();
        self.run_task("remote_storage.create", &payload, &new_participants, false)
            .await?;
        Ok(())
    }

    #[deprecated(note = "please use `recv_variable_with_remote_storage` instead")]
    pub async fn get_variable_with_remote_storage(
        &self,
        key: &str,
        sender: &Participant,
    ) -> Result<Vec<u8>, Error> {
        self.recv_variable_with_remote_storage(key, sender).await
    }

    pub async fn recv_variable_with_remote_storage(
        &self,
        key: &str,
        sender: &Participant,
    ) -> Result<Vec<u8>, Error> {
        let key = format!(
            "_remote_storage:private:{}:_variable_transfer:{}:{}",
            sender.user_id,
            self.get_task_id()?,
            key
        );
        let res = self.read_or_wait(&key).await?;
        Ok(res)
    }
}
