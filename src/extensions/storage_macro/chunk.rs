use async_recursion::async_recursion;

const CHUNK_SIZE: usize = 1024 * 1024; // use 1MB chunks

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[async_recursion]
    async fn _store_chunks(&self, payload: &[u8], key_name: &str) -> Result<Vec<String>, Error> {
        let mut offset = 0;
        let mut chunk_id = 0;
        let mut chunk_paths = Vec::new();
        while offset < payload.len() {
            let chunk_size = if offset + CHUNK_SIZE > payload.len() {
                payload.len() - offset
            } else {
                CHUNK_SIZE
            };
            let response = self
                .create_entry(
                    &format!("{}:{}", key_name, chunk_id),
                    &payload[offset..offset + chunk_size],
                )
                .await?;
            chunk_paths.push(response.split('@').last().unwrap().to_string()); // only store the timestamps
            offset += chunk_size;
            chunk_id += 1;
        }
        Ok(chunk_paths)
    }

    fn _check_chunk_paths_size(&self, chunk_paths: Vec<String>) -> Result<String, Error> {
        let chunk_paths_string = chunk_paths.join(";");
        if chunk_paths_string.len() > CHUNK_SIZE {
            return Err(format!(
                "File too large: failed to store {} chunks references in metadata",
                chunk_paths.len()
            )
            .into());
        }
        Ok(chunk_paths_string)
    }

    #[async_recursion]
    pub(crate) async fn _create_entry_chunk(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);

        // lock the metadata entry to prevent simultaneous writes
        let lock_token = self.lock(&metadata_key.clone()).await?;

        // create the chunks and store them
        let chunk_paths = self._store_chunks(payload, key_name).await?;

        // make sure that the chunk paths are smaller than the maximum entry size
        let chunk_paths_string = self._check_chunk_paths_size(chunk_paths)?;

        // store the chunk paths in the metadata entry and update metadata
        let response = self
            .update_entry(&metadata_key.clone(), &chunk_paths_string.into_bytes())
            .await?;
        self.unlock(lock_token).await?;
        Ok(response)
    }

    #[async_recursion]
    pub(crate) async fn _read_entry_chunk(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);
        let metadata_response = self.read_entry(&metadata_key.clone()).await?;

        // check if the metadata is locked
        let payload_string = String::from_utf8(metadata_response.clone())?;
        if payload_string == "creation-in-progress-locked" {
            return Err("Creation in progress".into());
        }

        // read the chunks into a single vector
        let chunks = payload_string.split(';').collect::<Vec<&str>>();
        let mut payload = Vec::new();
        for chunk in chunks {
            let response = self.read_entry(&format!("{}:{}", key_name, chunk)).await?;
            payload.append(&mut response.clone());
        }
        Ok(payload)
    }

    #[async_recursion]
    pub(crate) async fn _update_entry_chunk(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);

        // lock the metadata entry to prevent simultaneous writes
        let lock_token = self.lock(&metadata_key.clone()).await?;

        // split payload into chunks and update the chunks
        let chunk_paths = self._store_chunks(payload, key_name).await?;

        // make sure that the chunk paths are smaller than the maximum entry size
        let chunk_paths_string = self._check_chunk_paths_size(chunk_paths)?;

        // update the metadata entry
        let response = self
            .update_entry(&metadata_key.clone(), &chunk_paths_string.into_bytes())
            .await?;
        self.unlock(lock_token).await?;
        Ok(response)
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_chunk(&self, key_name: &str) -> Result<String, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);
        let lock_token = self.lock(&metadata_key.clone()).await?;
        let response = self.delete_entry(&metadata_key.clone()).await?;
        self.unlock(lock_token).await?;
        Ok(response)
    }
}
