use crate::application::{generate_request, CoLinkClient};
use crate::colink_proto::*;

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // use 1MB chunks

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    async fn _create_entry_chunk(&self, key_name: &str, payload: &[u8]) -> Result<String, Error> {
        let metadata_key = format!("{}::metadata", key_name);

        // lock the metadata entry to prevent simultaneous writes
        self.create_entry(
            metadata_key.clone(),
            "creation-in-progress-locked".into_bytes(),
        )
        .await?;

        // create the chunks and store them
        let mut offset = 0;
        let mut chunk_id = 0;
        let mut chunk_paths = Vec::new();
        while offset < payload.len() {
            let chunk_size = if offset + CHUNK_SIZE as usize > payload.len() {
                payload.len() - offset
            } else {
                CHUNK_SIZE as usize
            };
            let chunk = payload[offset..offset + chunk_size].to_vec();
            let response = self
                .create_entry(format!("{}::{}", key_name, chunk_id), chunk)
                .await?;
            chunk_paths.push(response.get_ref().key_path.clone());
            offset += chunk_size;
            chunk_id += 1;
        }

        // store the chunk paths in the metadata entry and update metadata
        let response = self
            .update_entry(metadata_key.clone(), chunk_paths.join(";").into_bytes())
            .await?;
        Ok(response.get_ref().key_path.clone())
    }

    async fn _read_entry_chunk(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let metadata_key = format!("{}::metadata", key_name);
        let metadata_response = self
            .read_entries(vec![StorageEntry {
                key_path: metadata_key,
                ..Default::default()
            }])
            .await?;

        // check if the metadata is locked
        let payload_string = String::from_utf8(response.get_ref().entries[0].payload.clone())?;
        if payload_string.ends_with("locked") {
            return Err("Creation in progress".into());
        }

        // read the chunks into a single vector
        let chunks = payload_string.split(';').collect::<Vec<&str>>();
        let mut payload = Vec::new();
        for chunk in chunks {
            let response = self
                .read_entries(vec![StorageEntry {
                    key_path: chunk.to_string(),
                    ..Default::default()
                }])
                .await?;
            payload.extend(response.get_ref().entries[0].payload.clone());
        }

        // parse payload into storage entry
        let storage_entry = StorageEntries {
            entries: vec![StorageEntry {
                key_path: metadata_response.get_ref().entries[0].key_path.clone(),
                payload,
                ..Default::default()
            }],
        };
        Ok(storage_entry)
    }

    async fn _update_entry_chunk(&self, key_name: &str, payload: &[u8]) -> Result<String, Error> {
        let metadata_key = format!("{}::metadata", key_name);

        // check if the metadata is locked
        let response = self.read_entry(metadata_key.clone()).await?;
        let payload_string = String::from_utf8(response.get_ref().payload.clone())?;
        if payload_string.ends_with("locked") {
            return Err("Creation in progress".into());
        }

        // split payload into chunks and update the chunks
        let mut offset = 0;
        let mut chunk_id = 0;
        let mut chunk_paths = Vec::new();
        while offset < payload.len() {
            let chunk_size = if offset + CHUNK_SIZE as usize > payload.len() {
                payload.len() - offset
            } else {
                CHUNK_SIZE as usize
            };
            let response = self
                .update_entry(
                    format!("{}::{}", key_name, chunk_id),
                    payload[offset..offset + chunk_size].to_vec(),
                )
                .await?;

            chunk_paths.push(response.get_ref().key_path.clone());

            offset += chunk_size;
            chunk_id += 1;
        }

        // update metadata with new chunk paths
        let response = self
            .update_entry(metadata_key.clone(), chunk_paths.join(";").into_bytes())
            .await?;
        Ok(response.get_ref().key_path.clone())
    }

    async fn _delete_entry_chunk(&self, key_name: &str) -> Result<(), Error> {
        let metadata_key = format!("{}::metadata", key_name);
        let response = self.delete_entry(metadata_key.clone()).await?;
        return Ok(response.get_ref().key_path.clone());
    }
}
