use crate::colink_proto::*;
use std::sync::Arc;
pub(crate) mod p2p_inbox;
mod remote_storage;
mod tls_utils;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[deprecated(note = "please use `send_variable` instead")]
    pub async fn set_variable(
        &self,
        key: &str,
        payload: &[u8],
        receivers: &[Participant],
    ) -> Result<(), Error> {
        self.send_variable(key, payload, receivers).await
    }

    pub async fn send_variable(
        &self,
        key: &str,
        payload: &[u8],
        receivers: &[Participant],
    ) -> Result<(), Error> {
        if self.task_id.is_empty() {
            Err("task_id not found".to_string())?;
        }
        let payload = Arc::new(payload.to_vec());
        for receiver in receivers {
            let cl = self.clone();
            let key = key.to_string();
            let payload = payload.clone();
            let receiver = receiver.clone();
            tokio::spawn(async move {
                if cl
                    ._send_variable_p2p(&key, &payload, &receiver)
                    .await
                    .is_err()
                {
                    cl.send_variable_with_remote_storage(&key, &payload, &[receiver.clone()])
                        .await?;
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
            });
        }
        Ok(())
    }

    #[deprecated(note = "please use `receive_variable` instead")]
    pub async fn get_variable(&self, key: &str, sender: &Participant) -> Result<Vec<u8>, Error> {
        self.receive_variable(key, sender).await
    }

    pub async fn receive_variable(
        &self,
        key: &str,
        sender: &Participant,
    ) -> Result<Vec<u8>, Error> {
        if self.task_id.is_empty() {
            Err("task_id not found".to_string())?;
        }
        if let Ok(res) = self._receive_variable_p2p(key, sender).await {
            return Ok(res);
        }
        let res = self
            .receive_variable_with_remote_storage(key, sender)
            .await?;
        Ok(res)
    }
}
