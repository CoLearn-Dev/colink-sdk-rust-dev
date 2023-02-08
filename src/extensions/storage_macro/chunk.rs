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
                .update_entry(
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

    #[async_recursion]
    async fn _append_chunks(
        &self,
        chunk_paths: &str,
        payload: &[u8],
        key_name: &str,
    ) -> Result<Vec<String>, Error> {
        let mut chunk_paths = chunk_paths
            .split(';')
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        let last_chunk_id = chunk_paths.len() - 1;
        let last_chunk_timestamp = chunk_paths[last_chunk_id].clone();
        let mut last_chunk = self
            .read_entry(&format!(
                "{}::{}:{}@{}",
                self.get_user_id()?,
                key_name,
                last_chunk_id,
                last_chunk_timestamp
            ))
            .await?;
        let mut offset = 0;
        let mut chunk_id = chunk_paths.len();
        if last_chunk.len() < CHUNK_SIZE {
            let chunk_size = if payload.len() < CHUNK_SIZE - last_chunk.len() {
                payload.len()
            } else {
                CHUNK_SIZE - last_chunk.len()
            };
            last_chunk.append(&mut payload[..chunk_size].to_vec());
            let response = self
                .update_entry(&format!("{}:{}", key_name, last_chunk_id), &last_chunk)
                .await?;
            chunk_paths[last_chunk_id] = response.split('@').last().unwrap().to_string();
            offset = chunk_size;
        }
        while offset < payload.len() {
            let chunk_size = if offset + CHUNK_SIZE > payload.len() {
                payload.len() - offset
            } else {
                CHUNK_SIZE
            };
            let response = self
                .update_entry(
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
        let lock_token = self.lock(&metadata_key).await?;
        // use a closure to prevent locking forever caused by errors
        let res = async {
            // create the chunks and store them
            let chunk_paths = self._store_chunks(payload, key_name).await?;
            // make sure that the chunk paths are smaller than the maximum entry size
            let chunk_paths_string = self._check_chunk_paths_size(chunk_paths)?;
            // store the chunk paths in the metadata entry and update metadata
            let response = self
                .create_entry(&metadata_key, &chunk_paths_string.into_bytes())
                .await?;
            Ok::<String, Error>(response)
        }
        .await;
        self.unlock(lock_token).await?;
        res
    }

    #[async_recursion]
    pub(crate) async fn _read_entry_chunk(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);
        let metadata_response = self.read_entry(&metadata_key).await?;
        let payload_string = String::from_utf8(metadata_response)?;
        let user_id = self.get_user_id()?;

        // read the chunks into a single vector
        let chunks_paths = payload_string.split(';').collect::<Vec<&str>>();
        let mut payload = Vec::new();
        for (i, timestamp) in chunks_paths.iter().enumerate() {
            let mut response = self
                .read_entry(&format!("{}::{}:{}@{}", user_id, key_name, i, timestamp))
                .await?;
            payload.append(&mut response);
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
        let lock_token = self.lock(&metadata_key).await?;
        // use a closure to prevent locking forever caused by errors
        let res = async {
            // split payload into chunks and update the chunks
            let chunk_paths = self._store_chunks(payload, key_name).await?;
            // make sure that the chunk paths are smaller than the maximum entry size
            let chunk_paths_string = self._check_chunk_paths_size(chunk_paths)?;
            // update the metadata entry
            let response = self
                .update_entry(&metadata_key, &chunk_paths_string.into_bytes())
                .await?;
            Ok::<String, Error>(response)
        }
        .await;
        self.unlock(lock_token).await?;
        res
    }

    #[async_recursion]
    pub(crate) async fn _append_entry_chunk(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);
        // lock the metadata entry to prevent simultaneous writes
        let lock_token = self.lock(&metadata_key).await?;
        // use a closure to prevent locking forever caused by errors
        let res = async {
            // split payload into chunks and update the chunks
            let metadata_response = self.read_entry(&metadata_key).await?;
            let payload_string = String::from_utf8(metadata_response)?;
            let chunk_paths = self
                ._append_chunks(&payload_string, payload, key_name)
                .await?;
            // make sure that the chunk paths are smaller than the maximum entry size
            let chunk_paths_string = self._check_chunk_paths_size(chunk_paths)?;
            // update the metadata entry
            let response = self
                .update_entry(&metadata_key, &chunk_paths_string.into_bytes())
                .await?;
            Ok::<String, Error>(response)
        }
        .await;
        self.unlock(lock_token).await?;
        res
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_chunk(&self, key_name: &str) -> Result<String, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);
        let lock_token = self.lock(&metadata_key).await?;
        let res = self.delete_entry(&metadata_key).await;
        self.unlock(lock_token).await?;
        res
    }
}
