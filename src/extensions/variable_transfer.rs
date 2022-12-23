use crate::colink_proto::*;
pub(crate) mod p2p_inbox;
mod remote_storage;

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
        if self
            ._set_variable_p2p(key, payload, receivers)
            .await
            .is_ok()
        {
            return Ok(());
        }
        self._set_variable_remote_storage(key, payload, receivers)
            .await?;
        Ok(())
    }

    pub async fn get_variable(&self, key: &str, sender: &Participant) -> Result<Vec<u8>, Error> {
        if self.task_id.is_empty() {
            Err("task_id not found".to_string())?;
        }
        if let Ok(res) = self._get_variable_p2p(key, sender).await {
            return Ok(res);
        }
        let res = self._get_variable_remote_storage(key, sender).await?;
        Ok(res)
    }
}
