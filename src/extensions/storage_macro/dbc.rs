use std::sync::Arc;
use rdbc;
use rdbc_sqlite::SqliteDriver;
use async_recursion::async_recursion;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[async_recursion]
    async fn _search_and_generate_query_string(&self, address: &str, key_name: &str) -> Result<String, Error> {
        let split_key_path: Vec<&str> = key_name.split(":").collect();
        for i in 0..split_key_path.len() {
            let current_key_path = format!("{}:{}", address, split_key_path[0..i].join(":"));
            let payload = self.read_entry(current_key_path.as_str()).await;
            if payload.is_ok() {
                let query_string_raw = String::from_utf8(payload.unwrap())?;
                let count = query_string_raw.matches("?").count();
                if count != split_key_path.len() - i {
                    return Err("Number of parameters does not match specified query string")?
                }
                let mut query_string = query_string_raw;
                for j in 0..count {
                    query_string = query_string.replacen("?", split_key_path[i + j], 1);
                }
                return Ok(query_string)
            }
        }
        Err("no query string found.")?
    }

    #[async_recursion]
    pub(crate) async fn _read_entry_rdbc(
        &self,
        address: &str,
        key_name: &str,
    ) -> Result<Vec<u8>, Error> {
        let url_key = format!("{}:url", address);
        let url = self.read_entry(url_key.as_str()).await?;
        let url_string = String::from_utf8(url)?;
        let query_string = self._search_and_generate_query_string(address, key_name).await?;
        let driver: Arc<dyn rdbc::Driver> = Arc::new( SqliteDriver::new());

        let conn = driver.connect(url_string.as_str()).unwrap();
        let mut conn = (*conn).borrow_mut();
        let stmt = conn.prepare(query_string.as_str()).unwrap();
        let mut stmt = (*stmt).borrow_mut();
        let mut rs = stmt.execute_query(&vec![]).unwrap();
        let mut result: Vec<u8> = Vec::new();
        //let meta = rs.borrow_mut().meta_data().unwrap();
        while {
            let mut rs = (*rs).borrow_mut();
            let value = rs.get_string(1).unwrap().ok_or("Failed to parse query results")?;
            result.extend(value.into_bytes());
            rs.next()
        } {}
        Ok(result)
    }
}