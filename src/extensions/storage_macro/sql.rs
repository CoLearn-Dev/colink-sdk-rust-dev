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

    async fn _generate_query_string(&self, key_path: &str, query_number: i32) -> Result<String, Error> {
        let query_key = format!("{}:q{}", key_path, query_number);
        let query = self.read_entry(query_key.as_str()).await?;
        let query_string = String::from_utf8(query)?;
        let count = query_string.matches("?").count();
        let mut query_string = query_string;
        for i in 0..count {
            let param_key = format!("{}:p{}", key_path, i);
            let param = self.read_entry(param_key.as_str()).await?;
            let param_string = String::from_utf8(param)?;
            query_string = query_string.replacen("?", param_string.as_str(), 1);
        }
        Ok(query_string)
    }

    pub(crate) async fn _read_entry_rdbc(
        &self,
        key_name: &str,
    ) -> Result<Vec<u8>, Error> {
        let mut conn = self._get_conn_from_stored_credentials(key_name).await?;
        let query_string = self._generate_query_string(key_name, 0).await?;
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