use async_recursion::async_recursion;
use redis::Commands;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    fn _get_con_from_address(&self, redis_address: &str) -> Result<redis::Connection, Error> {
        let client = redis::Client::open(redis_address)?;
        let con = client.get_connection()?;
        Ok(con)
    }

    async fn _get_con_from_stored_credentials(
        &self,
        key_path: &str,
    ) -> Result<redis::Connection, Error> {
        let redis_url_key = format!("{}:redis_url", key_path);
        let redis_url = self.read_entry(redis_url_key.as_str()).await?;
        let redis_url_string = String::from_utf8(redis_url)?;
        self._get_con_from_address(redis_url_string.as_str())
    }

    #[async_recursion]
    pub(crate) async fn _create_entry_redis(
        &self,
        address: &str,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response = con.set(key_name, payload)?;
        Ok(response)
    }

    #[async_recursion]
    pub(crate) async fn _read_entry_redis(
        &self,
        address: &str,
        key_name: &str,
    ) -> Result<Vec<u8>, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response: Vec<u8> = con.get(key_name)?;
        Ok(response)
    }

    #[async_recursion]
    pub(crate) async fn _update_entry_redis(
        &self,
        address: &str,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response = con.set(key_name, payload)?;
        Ok(response)
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_redis(
        &self,
        address: &str,
        key_name: &str,
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response = con.del(key_name)?;
        Ok(response)
    }
}
