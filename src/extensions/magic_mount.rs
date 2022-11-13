mod chunk;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub(crate) async fn _mm_create_entry(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let split_key = key_name.split("$").collect::<Vec<&str>>();
        let token = split_key[split_key.len() - 1];
        let key_name = split_key[0];
        if token.eq("chunk") {
            return self._create_entry_chunk(key_name, payload).await;
        }
        return Err("invalid storage option".into());
    }

    pub(crate) async fn _mm_read_entry(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let split_key = key_name.split("$").collect::<Vec<&str>>();
        let token = split_key[split_key.len() - 1];
        let key_name = split_key[0];
        if token.eq("chunk") {
            return self._read_entry_chunk(key_name).await;
        }
        return Err("invalid storage option".into());
    }

    pub(crate) async fn _mm_update_entry(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let split_key = key_name.split("$").collect::<Vec<&str>>();
        let token = split_key[split_key.len() - 1];
        let key_name = split_key[0];
        if token.eq("chunk") {
            return self._update_entry_chunk(key_name, payload).await;
        }
        return Err("invalid storage option".into());
    }

    pub(crate) async fn _mm_delete_entry(&self, key_name: &str) -> Result<String, Error> {
        let split_key = key_name.split("$").collect::<Vec<&str>>();
        let token = split_key[split_key.len() - 1];
        let key_name = split_key[0];
        if token.eq("chunk") {
            return self._delete_entry_chunk(key_name).await;
        }
        return Err("invalid storage option".into());
    }
}
