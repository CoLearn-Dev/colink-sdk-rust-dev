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
        let response: i32 = con.set_nx(key_name, payload)?;
        if response == 0 {
            Err("key already exists.")?
        }
        Ok(response.to_string())
    }

    #[async_recursion]
    pub(crate) async fn _read_entry_redis(
        &self,
        address: &str,
        key_name: &str,
    ) -> Result<Vec<u8>, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response: Option<Vec<u8>> = con.get(key_name)?;
        match response {
            Some(response) => Ok(response),
            None => Err("key does not exist.")?,
        }
    }

    #[async_recursion]
    pub(crate) async fn _update_entry_redis(
        &self,
        address: &str,
        key_name: &str,
        payload: &[u8],
        is_append: bool,
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response = if is_append {
            con.append::<&str, &[u8], i32>(key_name, payload)?
                .to_string()
        } else {
            con.set(key_name, payload)?
        };
        Ok(response)
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_redis(
        &self,
        address: &str,
        key_name: &str,
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response: i32 = con.del(key_name)?;
        if response == 0 {
            Err("key does not exist.")?
        }
        Ok(response.to_string())
    }
}
