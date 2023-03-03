use std::cell::{RefCell, RefMut};
use std::sync::Arc;
use async_recursion::async_recursion;
use rdbc;
use rdbc::Connection;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    async fn _get_conn_from_stored_credentials(&self, key_path: &str) -> Result<RefMut<dyn Connection>, Error> {
        let db_url_key = format!("{}:db_url", key_path);
        let db_url = self.read_entry(db_url_key.as_str()).await?;
        let db_url_string = String::from_utf8(db_url)?;
        let driver: Arc<dyn rdbc::Driver> = Arc::new(rdbc::Driver);
        let conn = driver.connect(db_url_string.as_str())?;
        Ok(conn.borrow_mut())
    }

    async fn _search_and_generate_query_string(&self, key_path: &str) -> Result<String, Error> {
        let split_key_path: Vec<&str> = key_path.split(":").collect();
        for i in 0..split_key_path.len() {
            let current_key_path = split_key_path[0..i].join(":");
            let payload = self.read_entry(current_key_path.as_str()).await;
            if payload.is_ok() {
                let query_string_raw = String::from_utf8(payload.unwrap())?;
                let count = query_string_raw.matches("?").count();
                if count != split_key_path.len() - i {
                    return Err("Number of parameters does not match specified query string")?
                }
                let mut query_string = String::from_utf8(query)?;
                for j in 0..count {
                    query_string = query_string.replacen("?", split_key_path[i + j], 1);
                }
                Ok(query_string)
            }
        }
        Err("no query string found.")?
    }

    pub(crate) async fn _read_entry_rdbc(
        &self,
        key_name: &str,
    ) -> Result<Vec<u8>, Error> {
        let mut conn = self._get_conn_from_stored_credentials(key_name).await?;
        let query_string = self._search_and_generate_query_string(key_name, 0).await?;
        let mut stmt = conn.prepare(query_string.as_str())?;
        let mut rs = stmt.execute_query()?;
        let mut result: Vec<u8> = Vec::new();
        while rs.next()? {
            let mut value = rs.get_string(1)?;
            result.push(value);
        }
        Ok(result)
    }
}