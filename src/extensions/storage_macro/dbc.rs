use async_recursion::async_recursion;
use rdbc2;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[async_recursion]
    async fn _search_and_generate_query(
        &self,
        string_before_dbc: &str,
        string_after_dbc: &str,
    ) -> Result<(String, Vec<String>), Error> {
        let split_key_path: Vec<&str> = string_after_dbc.split(':').collect();
        for i in (0..split_key_path.len()).rev() {
            let current_key_path =
                format!("{}:{}", string_before_dbc, split_key_path[0..=i].join(":"));
            let payload = self.read_entry(current_key_path.as_str()).await;
            if let Ok(payload) = payload {
                let query_string = String::from_utf8(payload)?;
                let count = query_string.matches('?').count();
                if count != split_key_path.len() - (i + 1) {
                    return Err("Number of parameters does not match specified query string")?;
                }
                let params = split_key_path[(i + 1)..]
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>();
                return Ok((query_string, params));
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
        let (query_string, params) = self
            ._search_and_generate_query(string_before_dbc, string_after_dbc)
            .await?;
        let params: Vec<&str> = params.iter().map(AsRef::as_ref).collect();
        let mut database = rdbc2::dbc::Database::new(url_string.as_str())?;
        let result = database.execute_query_with_params(query_string.as_str(), &params)?;
        let seralized_result = serde_json::to_vec(&result)?;
        Ok(seralized_result)
    }
}
