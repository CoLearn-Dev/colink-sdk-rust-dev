use crate::colink_proto::*;
use async_recursion::async_recursion;

const CHUNK_SIZE: usize = 1024 * 1024; // use 1MB chunks

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[async_recursion]
    pub(crate) async fn _create_entry_chunk(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);

        // lock the metadata entry to prevent simultaneous writes
        self.create_entry(
            &metadata_key.clone(),
            &("creation-in-progress-locked".to_string().into_bytes()),
        )
        .await?;

        // create the chunks and store them
        let mut offset = 0;
        let mut chunk_id = 0;
        let mut chunk_paths = Vec::new();
        while offset < payload.len() {
            let chunk_size = if offset + CHUNK_SIZE > payload.len() {
                payload.len() - offset
            } else {
                CHUNK_SIZE
            };
            let chunk = payload[offset..offset + chunk_size].to_vec();
            let response = self
                .create_entry(&format!("{}{}", key_name, chunk_id), &chunk)
                .await?;
            chunk_paths.push(response);
            offset += chunk_size;
            chunk_id += 1;
        }

        // store the chunk paths in the metadata entry and update metadata
        self.update_entry(&metadata_key.clone(), &chunk_paths.join(";").into_bytes())
            .await
    }

    pub(crate) async fn _read_entry_chunk(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);
        let metadata_response = self
            .read_entries(&[StorageEntry {
                key_name: metadata_key.clone(),
                ..Default::default()
            }])
            .await?;

        // check if the metadata is locked
        let payload_string = String::from_utf8(metadata_response[0].payload.clone())?;
        if payload_string.ends_with("locked") {
            return Err("Creation in progress".into());
        }

        // read the chunks into a single vector
        let chunks = payload_string.split(';').collect::<Vec<&str>>();
        let mut payload = Vec::new();
        for chunk in chunks {
            let response = self
                .read_entries(&[StorageEntry {
                    key_path: chunk.to_string(),
                    ..Default::default()
                }])
                .await?;
            payload.push(response[0].key_path.clone());
        }
        Ok(payload.join(";").into_bytes())
    }

    #[async_recursion]
    pub(crate) async fn _update_entry_chunk(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);

        // check if the metadata is locked
        let response = self.read_entry(&metadata_key.clone()).await?;
        let payload_string = String::from_utf8(response.clone())?;
        if payload_string.ends_with("locked") {
            return Err("Creation in progress".into());
        }

        // split payload into chunks and update the chunks
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
            chunk_paths.push(response);
            offset += chunk_size;
            chunk_id += 1;
        }

        // update metadata with new chunk paths
        self.update_entry(&metadata_key.clone(), &chunk_paths.join(";").into_bytes())
            .await
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_chunk(&self, key_name: &str) -> Result<String, Error> {
        let metadata_key = format!("{}:chunk_metadata", key_name);
        self.delete_entry(&metadata_key.clone()).await
    }
}
