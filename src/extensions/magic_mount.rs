mod chunk;

use crate::colink_proto::*;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn _mm_create_entry(&self, key_name: &str, payload: &[u8]) -> Result<String, Error> {
        let token = key_name.split("$").last().unwrap();
        if token.eq("chunk") {
            let response = self._create_entry_chunk(key_name, payload).await?;
            return Ok(response);
        }
        return Err("invalid storage option".into());
    }

    pub async fn _mm_read_entry(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let token = key_name.split("$").last().unwrap();
        if token.eq("chunk") {
            let response = self._read_entry_chunk(key_name).await?;
            return Ok(response);
        }
        return Err("invalid storage option".into());
    }

    pub async fn _mm_update_entry(&self, key_name: &str, payload: &[u8]) -> Result<(), Error> {
        let token = key_name.split("$").last().unwrap();
        if token.eq("chunk") {
            let response = self._update_entry_chunk(key_name, payload).await?;
            return Ok(response);
        }
        return Err("invalid storage option".into());
    }

    pub async fn _mm_delete_entry(&self, key_name: &str) -> Result<(), Error> {
        let token = key_name.split("$").last().unwrap();
        if token.eq("chunk") {
            let response = self._delete_entry_chunk(key_name).await?;
            return Ok(response);
        }
        return Err("invalid storage option".into());
    }
}
