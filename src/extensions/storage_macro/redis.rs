use async_recursion::async_recursion;
use redis::{aio::Connection, AsyncCommands};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    async fn _get_con_from_stored_credentials(&self, key_path: &str) -> Result<Connection, Error> {
        let redis_url_key = format!("{}:redis_url", key_path);
        let redis_url = self.read_entry(redis_url_key.as_str()).await?;
        let redis_url_string = String::from_utf8(redis_url)?;
        let client = redis::Client::open(redis_url_string)?;
        let con = client.get_async_connection().await?;
        Ok(con)
    }

    #[async_recursion]
    pub(crate) async fn _create_entry_redis(
        &self,
        address: &str,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response: i32 = con.set_nx(key_name, payload).await?;
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
        let response: Option<Vec<u8>> = con.get(key_name).await?;
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
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response = con.set(key_name, payload).await?;
        Ok(response)
    }

    #[async_recursion]
    pub(crate) async fn _append_entry_redis(
        &self,
        address: &str,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response = con
            .append::<&str, &[u8], i32>(key_name, payload)
            .await?
            .to_string();
        Ok(response)
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_redis(
        &self,
        address: &str,
        key_name: &str,
    ) -> Result<String, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let response: i32 = con.del(key_name).await?;
        if response == 0 {
            Err("key does not exist.")?
        }
        Ok(response.to_string())
    }

    #[async_recursion]
    pub(crate) async fn _read_keys_redis(
        &self,
        address: &str,
        prefix: &str,
    ) -> Result<Vec<String>, Error> {
        let mut con = self._get_con_from_stored_credentials(address).await?;
        let res: Vec<String> = con.keys(format!("{}:*", prefix)).await?;
        let mut key_list: Vec<String> = Vec::new();
        for key in res {
            key_list.push(key[prefix.len() + 1..].to_string());
        }
        Ok(key_list)
    }
}
