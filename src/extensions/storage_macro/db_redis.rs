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
        address: &str,
    ) -> Result<redis::Connection, Error> {
        // get credentials from storage
        let redis_username = self
            .read_entry(format!("{}:{}", address, "redis_username").as_str())
            .await?;
        let redis_username_string = String::from_utf8(redis_username)?;
        let redis_password = self
            .read_entry(format!("{}:{}", address, "redis_password").as_str())
            .await?;
        let redis_password_string = String::from_utf8(redis_password)?;
        let redis_host_name = self
            .read_entry(format!("{}:{}", address, "redis_host_name").as_str())
            .await?;
        let redis_host_name_string = String::from_utf8(redis_host_name)?;

        // get the uri scheme from storage, if doesn't exist, default to non-ssl
        let uri_scheme = self
            .read_entry(format!("{}:{}", address, "redis_use_secure").as_str())
            .await;
        let mut uri_scheme_string = "redis://".to_string();
        if uri_scheme.is_ok() {
            uri_scheme_string = "rediss://".to_string();
        }

        let redis_address = format!(
            "{}://{}:{}@{}",
            uri_scheme_string, redis_username_string, redis_password_string, redis_host_name_string
        );
        self._get_con_from_address(&redis_address)
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
