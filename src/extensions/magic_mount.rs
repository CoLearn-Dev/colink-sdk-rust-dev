use tonic::transport::{Channel, ClientTlsConfig};
use crate::application::{CoLinkClient, generate_request};
use crate::colink_proto::*;

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // use 1MB chunks

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn _create_entry_bulk(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let metadata_key = format!("{}::metadata", key_name);
        let metadata_key_copy = metadata_key.clone();
        let result = metadata_key.clone();
        let num_chunks = payload.len() / CHUNK_SIZE + 1;
        let create_request = generate_request(
            &self.jwt,
            StorageEntry {
                key_name: metadata_key,
                payload: (num_chunks.to_string() + "-locked").into_bytes(), // simply store the number of chunks in the metadata
                ..Default::default()
            },
        );
        client.create_entry(create_request).await?;

        let mut offset = 0;
        let mut chunk_id = 0;
        while offset < payload.len() {
            let chunk_size = if offset + CHUNK_SIZE as usize > payload.len() {
                payload.len() - offset
            } else {
                CHUNK_SIZE as usize
            };
            let chunk = payload[offset..offset + chunk_size].to_vec();
            let request = generate_request(
                &self.jwt,
                StorageEntry {
                    key_name: format!("{}::{}", key_name, chunk_id),
                    payload: chunk,
                    ..Default::default()
                },
            );
            client.create_entry(request).await?;
            offset += chunk_size;
            chunk_id += 1;
        }

        let update_request = generate_request(
            &self.jwt,
            StorageEntry {
                key_name: metadata_key_copy,
                payload: (num_chunks.to_string() + "-unlocked").into_bytes(),
                ..Default::default()
            },
        );
        client.update_entry(update_request).await?;
        Ok(result)
    }

    pub async fn _read_entry_bulk(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let metadata_key = format!("{}::metadata", key_name);
        let request = generate_request(&self.jwt, StorageEntries {
            entries: vec![StorageEntry {
                key_path: metadata_key,
                ..Default::default()
            }],
        });
        let response = client.read_entries(request).await?;
        let payload_string = String::from_utf8(response.get_ref().entries[0].payload.clone())?;
        let num_chunks = payload_string.split("-").next().unwrap().parse::<usize>()?;
        let locked = payload_string.split("-").last().unwrap().eq("locked");
        if locked {
            return Err("entry is locked".into());
        }

        let mut payload = Vec::new();
        for chunk_id in 0..num_chunks {
            let request = generate_request(&self.jwt, StorageEntries {
                entries: vec![StorageEntry {
                    key_path: format!("{}::{}", key_name, chunk_id),
                    ..Default::default()
                }],
            });
            let response = client.read_entries(request).await?;
            payload.extend(response.get_ref().entries[0].payload.clone());
        }
        Ok(payload)
    }

    pub async fn _update_entry_bulk(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let metadata_key = format!("{}::metadata", key_name);
        let metadata_key_copy = metadata_key.clone();
        let result = metadata_key.clone();
        let num_chunks = payload.len() / CHUNK_SIZE + 1;
        let request = generate_request(
            &self.jwt,
            StorageEntry {
                key_name: metadata_key,
                // store number of chunks and lock in metadata
                payload: (num_chunks.to_string() + "-locked").into_bytes(),
                ..Default::default()
            },
        );
        client.update_entry(request).await?;

        let mut offset = 0;
        let mut chunk_id = 0;
        while offset < payload.len() {
            let chunk_size = if offset + CHUNK_SIZE as usize > payload.len() {
                payload.len() - offset
            } else {
                CHUNK_SIZE as usize
            };
            let chunk = payload[offset..offset + chunk_size].to_vec();
            let request = generate_request(
                &self.jwt,
                StorageEntry {
                    key_name: format!("{}::{}", key_name, chunk_id),
                    payload: chunk,
                    ..Default::default()
                },
            );
            client.update_entry(request).await?;
            offset += chunk_size;
            chunk_id += 1;
        }

        let update_request = generate_request(
            &self.jwt,
            StorageEntry {
                key_name: metadata_key_copy,
                payload: (num_chunks.to_string() + "-unlocked").into_bytes(),
                ..Default::default()
            },
        );
        client.update_entry(update_request).await?;
        Ok(result)
    }
}