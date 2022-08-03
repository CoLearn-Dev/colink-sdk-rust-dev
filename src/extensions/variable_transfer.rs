use crate::colink_proto::*;
use colink_remote_storage::*;
use prost::Message;
mod colink_remote_storage {
    include!(concat!(env!("OUT_DIR"), "/colink_remote_storage.rs"));
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn set_variable(
        &self,
        key: &str,
        payload: &[u8],
        receivers: &[Participant],
    ) -> Result<(), Error> {
        if self.task_id.is_empty() {
            Err("task_id not found".to_string())?;
        }
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

    pub async fn get_variable(&self, key: &str, sender: &Participant) -> Result<Vec<u8>, Error> {
        if self.task_id.is_empty() {
            Err("task_id not found".to_string())?;
        }
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
