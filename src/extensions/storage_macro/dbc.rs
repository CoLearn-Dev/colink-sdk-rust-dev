use async_recursion::async_recursion;
use rdbc2;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[async_recursion]
    async fn _search_and_generate_query_string(
        &self,
        string_before_dbc: &str,
        string_after_dbc: &str,
    ) -> Result<String, Error> {
        let split_key_path: Vec<&str> = string_after_dbc.split(':').collect();
        println!("split_key_path: {:?}", split_key_path);
        for i in 0..split_key_path.len() {
            let current_key_path = format!(
                "{}:{}",
                string_before_dbc,
                split_key_path[0..i + 1].join(":")
            );
            println!("current_key_path: {}", current_key_path);
            let payload = self.read_entry(current_key_path.as_str()).await;
            if payload.is_ok() {
                let query_string_raw = String::from_utf8(payload.unwrap())?;
                let count = query_string_raw.matches('?').count();
                if count != split_key_path.len() - (i + 1) {
                    return Err("Number of parameters does not match specified query string")?;
                }
                let mut query_string = query_string_raw;
                for j in 0..count {
                    query_string = query_string.replacen('?', split_key_path[i + j + 1], 1);
                }
                return Ok(query_string);
            } else {
                continue;
            }
        }
        Err("no query string found.")?
    }

    #[async_recursion]
    pub(crate) async fn _read_entry_dbc(
        &self,
        string_before_dbc: &str,
        string_after_dbc: &str,
    ) -> Result<Vec<u8>, Error> {
        let url_key = format!("{}:url", string_before_dbc);
        let url = self.read_entry(url_key.as_str()).await?;
        let url_string = String::from_utf8(url)?;
        let query_string = self
            ._search_and_generate_query_string(string_before_dbc, string_after_dbc)
            .await?;
        let mut database = rdbc2::dbc::Database::new(url_string.as_str())?;
        println!("query_string: {}", query_string);
        let result = database.execute_query_and_serialize_raw(query_string.as_str())?;
        Ok(result)
    }
}
