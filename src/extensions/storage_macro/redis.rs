use async_recursion::async_recursion;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[async_recursion]
    pub(crate) async fn _create_entry_redis(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        Ok("".to_string())
    }

    #[async_recursion]
    pub(crate) async fn _read_entry_redis(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        Ok(vec![])
    }

    #[async_recursion]
    pub(crate) async fn _update_entry_redis(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        Ok("".to_string())
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_redis(&self, key_name: &str) -> Result<String, Error> {
        Ok("".to_string())
    }
}
